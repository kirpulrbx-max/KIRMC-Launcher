// Capa tipada sobre los comandos Rust. Todo el acceso al backend pasa por aquí.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Loader = "vanilla" | "fabric" | "quilt" | "forge";
export type Lang = "es" | "en" | "ja";
export type AfterLaunch = "keep" | "hide" | "close";
export type ProjectType = "mod" | "modpack" | "resourcepack" | "shader";
export type SkinVariant = "classic" | "slim";

export interface Cape { id: string; alias: string; url: string; active: boolean }

export interface Account {
  uuid: string;
  username: string;
  kind: "microsoft" | "offline";
  skinUrl?: string | null;
  skinVariant?: string | null;
  capes: Cape[];
  /** Solo no-premium: capa servida por el servidor de skins local */
  capeUrl?: string | null;
}

export interface ServerEntry { name: string; address: string }

export interface LauncherConfig {
  language: Lang;
  javaPath?: string | null;
  minRamMb: number;
  maxRamMb: number;
  afterLaunch: AfterLaunch;
  allowMultipleInstances: boolean;
  notifications: boolean;
  animations: boolean;
  gameWidth: number;
  gameHeight: number;
  fullscreen: boolean;
  concurrentDownloads: number;
  verifyFiles: boolean;
  activeAccount?: string | null;
  skinServer?: string | null;
  favoriteServers: ServerEntry[];
  msClientId?: string | null;
}

export interface Instance {
  id: string;
  name: string;
  mcVersion: string;
  loader: Loader;
  loaderVersion?: string | null;
  versionId?: string | null;
  minRamMb?: number | null;
  maxRamMb?: number | null;
  javaPath?: string | null;
  jvmArgs: string[];
  lastPlayed?: number | null;
  created: number;
}

export type VersionType = "release" | "snapshot" | "old_beta" | "old_alpha";
export interface VersionEntry { id: string; type: VersionType; url: string; releaseTime: string }
export interface Manifest { latest: { release: string; snapshot: string }; versions: VersionEntry[] }

export interface NewsItem { id: string; title: string; summary: string; image?: string | null; date: string; tag: string; url?: string | null }
export interface ModFile { file: string; enabled: boolean; size: number }
export interface JavaInstall { path: string; version: string; major: number }

export interface MrHit {
  projectId: string;
  slug: string;
  title: string;
  description: string;
  author: string;
  downloads: number;
  follows: number;
  iconUrl?: string | null;
  projectType: ProjectType;
  categories: string[];
}
export interface MrSearch { hits: MrHit[]; offset: number; totalHits: number }

export interface PlayerTextures { uuid: string; name: string; skinUrl?: string | null; slim: boolean; capeUrl?: string | null }
export interface SavedSkin { id: string; name: string; variant: SkinVariant; kind: "skin" | "cape"; added: number }
export interface GallerySkin { id: string; name?: string | null; textureUrl: string }
export interface GalleryPage { skins: GallerySkin[]; next?: string | null }
export interface CapeEntry { provider: string; imageUrl: string }

export type Stage = "libraries" | "assets" | "mods" | "modpack" | "java";
export interface Progress { tag: string; stage: Stage; done: number; total: number }
export interface LogBatch { instanceId: string; err: boolean; lines: string[] }
export interface ExitEvent { instanceId: string; code: number | null }

export const isTauri = "__TAURI_INTERNALS__" in window;

const NO_BACKEND =
  "Esta función necesita la app de escritorio. La vista previa del navegador no tiene backend: ejecuta `npm run tauri dev`.";

/** invoke() seguro: en la vista previa del navegador falla con un mensaje claro en vez de un TypeError */
const call = <T,>(cmd: string, args?: Record<string, unknown>): Promise<T> =>
  isTauri ? invoke<T>(cmd, args) : Promise.reject(new Error(NO_BACKEND));

export const api = {
  getConfig: () => call<LauncherConfig>("get_config"),
  saveConfig: (cfg: LauncherConfig) => call<void>("save_config", { cfg }),
  dataDir: () => call<string>("data_dir"),

  listAccounts: () => call<Account[]>("list_accounts"),
  addOfflineAccount: (username: string) => call<Account>("add_offline_account", { username }),
  removeAccount: (uuid: string) => call<void>("remove_account", { uuid }),
  /** Abre la ventana oficial de Microsoft y resuelve con la cuenta cuando termina el login */
  msLoginWindow: () => call<Account>("ms_login_window"),
  uploadSkin: (uuid: string, bytes: Uint8Array, variant: SkinVariant) =>
    call<Account>("upload_skin", { uuid, bytes: Array.from(bytes), variant }),
  setCape: (uuid: string, capeId: string | null) => call<Account>("set_cape", { uuid, capeId }),
  /** No-premium: skin/capa que el servidor de skins local sirve al juego */
  setOfflineSkin: (uuid: string, bytes: Uint8Array, variant: SkinVariant) =>
    call<void>("set_offline_skin", { uuid, bytes: Array.from(bytes), variant }),
  setOfflineCape: (uuid: string, bytes: Uint8Array | null) =>
    call<void>("set_offline_cape", { uuid, bytes: bytes ? Array.from(bytes) : null }),

  lookupPlayer: (name: string) => call<PlayerTextures>("lookup_player", { name }),
  fetchTexture: (url: string) => call<number[]>("fetch_texture", { url }).then((b) => new Uint8Array(b)),
  listSkins: () => call<SavedSkin[]>("list_skins"),
  saveSkin: (name: string, bytes: Uint8Array, variant: SkinVariant, kind: "skin" | "cape" = "skin") =>
    call<SavedSkin>("save_skin", { name, bytes: Array.from(bytes), variant, kind }),
  mineskinGallery: (after?: string | null) => call<GalleryPage>("mineskin_gallery", { after: after ?? null }),
  lookupCapes: (name: string) => call<CapeEntry[]>("lookup_capes", { name }),
  elybySkin: (name: string) => call<number[]>("elyby_skin", { name }).then((b) => new Uint8Array(b)),
  readSkin: (id: string) => call<number[]>("read_skin", { id }).then((b) => new Uint8Array(b)),
  deleteSkin: (id: string) => call<void>("delete_skin", { id }),

  listVersions: () => call<Manifest>("list_versions"),
  listLoaderVersions: (loader: Loader, mcVersion: string) => call<string[]>("list_loader_versions", { loader, mcVersion }),

  listInstances: () => call<Instance[]>("list_instances"),
  createInstance: (p: { name: string; mcVersion: string; loader: Loader; loaderVersion: string | null }) =>
    call<Instance>("create_instance", p),
  updateInstance: (instance: Instance) => call<Instance>("update_instance", { instance }),
  deleteInstance: (id: string) => call<void>("delete_instance", { id }),
  instanceDir: (id: string, sub?: "mods" | "resourcepacks" | "shaderpacks" | "config" | "saves") =>
    call<string>("instance_dir", { id, sub: sub ?? null }),
  listMods: (id: string) => call<ModFile[]>("list_mods", { id }),
  toggleMod: (id: string, file: string) => call<void>("toggle_mod", { id, file }),

  modrinthSearch: (p: { query: string; projectType: ProjectType; gameVersion?: string | null; loader?: Loader | null; offset?: number }) =>
    call<MrSearch>("modrinth_search", { gameVersion: null, loader: null, offset: 0, ...p }),
  modrinthInstall: (instanceId: string, projectId: string, projectType: ProjectType, versionId?: string) =>
    call<string[]>("modrinth_install", { instanceId, projectId, projectType, versionId: versionId ?? null }),
  modrinthInstallModpack: (projectId: string, versionId?: string) =>
    call<Instance>("modrinth_install_modpack", { projectId, versionId: versionId ?? null }),

  installInstance: (id: string) => call<void>("install_instance", { id }),
  launchInstance: (id: string, accountUuid: string, server?: string) =>
    call<void>("launch_instance", { id, accountUuid, server: server ?? null }),
  killInstance: (id: string) => call<void>("kill_instance", { id }),
  runningInstances: () => call<string[]>("running_instances"),

  detectJava: () => call<JavaInstall[]>("detect_java"),
  fetchNews: () => call<NewsItem[]>("fetch_news"),
};

export const events = {
  onProgress: (cb: (p: Progress) => void) => listen<Progress>("download://progress", (e) => cb(e.payload)),
  onLog: (cb: (b: LogBatch) => void) => listen<LogBatch>("game://log", (e) => cb(e.payload)),
  onStarted: (cb: (id: string) => void) => listen<string>("game://started", (e) => cb(e.payload)),
  onExit: (cb: (e: ExitEvent) => void) => listen<ExitEvent>("game://exit", (e) => cb(e.payload)),
};

/** Avatares/skins vía mc-heads (con CORS, apto para el visor 3D) */
export const avatarUrl = (a?: Account | null, size = 64) =>
  `https://mc-heads.net/avatar/${a?.kind === "microsoft" ? a.uuid : "MHF_Steve"}/${size}`;
/** Textura de skin 64×64 de la cuenta (para la cabeza 3D y el visor) */
export const skinUrl = (a?: Account | null) =>
  a?.kind === "offline" && a.skinUrl
    ? a.skinUrl // servidor de skins local (127.0.0.1)
    : `https://mc-heads.net/skin/${a?.kind === "microsoft" ? a.uuid : "MHF_Steve"}`;

/** PNG en memoria -> URL utilizable en <img> / visor 3D (recuerda revocarla) */
export const pngUrl = (bytes: Uint8Array) => URL.createObjectURL(new Blob([bytes as BlobPart], { type: "image/png" }));

export const formatCount = (n: number) =>
  n >= 1e6 ? `${(n / 1e6).toFixed(1)}M` : n >= 1e3 ? `${(n / 1e3).toFixed(1)}k` : String(n);
