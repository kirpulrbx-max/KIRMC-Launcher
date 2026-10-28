// Estado global con zustand: suscripciones por selector -> solo re-renderiza lo que cambia.
import { create } from "zustand";
import { api, events, isTauri, type Account, type Instance, type LauncherConfig, type Progress } from "../lib/api";

export type Page = "play" | "instances" | "chat" | "news" | "characters" | "settings";
/** Estado de ánimo de la mascota */
export type Mood = "idle" | "loading" | "playing" | "error";
export interface LogLine { text: string; err: boolean }

const MAX_LOG_LINES = 5000;

export const DEFAULT_CONFIG: LauncherConfig = {
  language: "es",
  minRamMb: 1024,
  maxRamMb: 4096,
  afterLaunch: "hide",
  allowMultipleInstances: true,
  notifications: true,
  animations: true,
  gameWidth: 1280,
  gameHeight: 720,
  fullscreen: false,
  concurrentDownloads: 24,
  verifyFiles: false,
  favoriteServers: [{ name: "Hypixel", address: "mc.hypixel.net" }],
};

/** Ajuste de animaciones -> atributo en <html> que el CSS usa para pausar todo */
const applyMotion = (cfg: LauncherConfig | null) => {
  document.documentElement.dataset.motion = cfg?.animations === false ? "off" : "on";
};

interface AppState {
  ready: boolean;
  page: Page;
  mood: Mood;
  error: string | null;
  config: LauncherConfig | null;
  accounts: Account[];
  instances: Instance[];
  selectedId: string | null;
  running: Record<string, true>;
  /** Instancias con una instalación/preparación en curso */
  busy: Record<string, true>;
  /** Progreso de descarga por instancia (tag) */
  progress: Record<string, Progress>;
  logs: Record<string, LogLine[]>;

  init: () => Promise<void>;
  setPage: (p: Page) => void;
  updateConfig: (patch: Partial<LauncherConfig>) => Promise<void>;
  refreshAccounts: () => Promise<void>;
  refreshInstances: () => Promise<void>;
  select: (id: string) => void;
  activeAccount: () => Account | undefined;
  launch: (server?: string) => Promise<void>;
  install: (id: string) => Promise<void>;
  stop: (id: string) => Promise<void>;
  setError: (e: string | null) => void;
}

let initPromise: Promise<void> | null = null;

const without = <T,>(rec: Record<string, T>, key: string) => {
  const next = { ...rec };
  delete next[key];
  return next;
};

export const useApp = create<AppState>((set, get) => ({
  ready: false,
  page: "play",
  mood: "idle",
  error: null,
  config: null,
  accounts: [],
  instances: [],
  selectedId: null,
  running: {},
  busy: {},
  progress: {},
  logs: {},

  init() {
    // Guard contra el doble efecto de StrictMode
    initPromise ??= (async () => {
      if (!isTauri) {
        applyMotion(DEFAULT_CONFIG);
        // Vista previa con datos de ejemplo: ?demo (opcional &page=instances|characters|settings|chat|news)
        const params = new URLSearchParams(location.search);
        if (params.has("demo")) {
          const { demoState } = await import("../lib/demo");
          set({ ...demoState(DEFAULT_CONFIG), page: (params.get("page") as Page | null) ?? "play", ready: true });
          return;
        }
        set({ config: DEFAULT_CONFIG, ready: true, error: "Ejecuta con `npm run tauri dev` para usar el backend" });
        return;
      }
      try {
        const [config, accounts, instances, running] = await Promise.all([
          api.getConfig(),
          api.listAccounts(),
          api.listInstances(),
          api.runningInstances(),
        ]);
        applyMotion(config);
        set({
          config,
          accounts,
          instances,
          selectedId: instances[0]?.id ?? null,
          running: Object.fromEntries(running.map((id) => [id, true as const])),
        });
      } catch (e) {
        set({ config: DEFAULT_CONFIG, error: String(e) });
      }
      set({ ready: true });

      events.onProgress((p) =>
        set((s) => ({ progress: p.done >= p.total ? without(s.progress, p.tag) : { ...s.progress, [p.tag]: p } })),
      );
      events.onLog((b) =>
        set((s) => {
          const next = (s.logs[b.instanceId] ?? []).concat(b.lines.map((text) => ({ text, err: b.err })));
          return { logs: { ...s.logs, [b.instanceId]: next.length > MAX_LOG_LINES ? next.slice(-MAX_LOG_LINES) : next } };
        }),
      );
      events.onStarted((id) =>
        set((s) => ({ running: { ...s.running, [id]: true }, busy: without(s.busy, id), mood: "playing", progress: without(s.progress, id) })),
      );
      events.onExit((e) =>
        set((s) => {
          const running = without(s.running, e.instanceId);
          const crashed = e.code !== null && e.code !== 0;
          return { running, mood: crashed ? "error" : Object.keys(running).length ? "playing" : "idle" };
        }),
      );
    })();
    return initPromise;
  },

  setPage: (page) => set({ page }),

  async updateConfig(patch) {
    const config = { ...(get().config ?? DEFAULT_CONFIG), ...patch };
    set({ config });
    applyMotion(config);
    if (isTauri) await api.saveConfig(config);
  },

  async refreshAccounts() {
    set({ accounts: await api.listAccounts() });
  },

  async refreshInstances() {
    const instances = await api.listInstances();
    set((s) => ({ instances, selectedId: instances.some((i) => i.id === s.selectedId) ? s.selectedId : instances[0]?.id ?? null }));
  },

  select: (selectedId) => set({ selectedId }),

  activeAccount() {
    const { accounts, config } = get();
    return accounts.find((a) => a.uuid === config?.activeAccount) ?? accounts[0];
  },

  async launch(server) {
    const { selectedId } = get();
    const account = get().activeAccount();
    if (!selectedId || !account) return;
    set((s) => ({ mood: "loading", error: null, busy: { ...s.busy, [selectedId]: true }, logs: { ...s.logs, [selectedId]: [] } }));
    try {
      await api.launchInstance(selectedId, account.uuid, server);
    } catch (e) {
      set((s) => ({ mood: "error", error: String(e), busy: without(s.busy, selectedId), progress: without(s.progress, selectedId) }));
    }
  },

  async install(id) {
    set((s) => ({ mood: "loading", error: null, busy: { ...s.busy, [id]: true } }));
    try {
      await api.installInstance(id);
      set((s) => ({ mood: "idle", busy: without(s.busy, id), progress: without(s.progress, id) }));
    } catch (e) {
      set((s) => ({ mood: "error", error: String(e), busy: without(s.busy, id), progress: without(s.progress, id) }));
    }
  },

  stop: (id) => api.killInstance(id),

  setError: (error) => set({ error }),
}));
