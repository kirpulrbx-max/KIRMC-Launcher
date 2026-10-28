import { openPath } from "@tauri-apps/plugin-opener";
import { useEffect, useState, type ReactNode } from "react";
import AccountModal from "../components/AccountModal";
import Icon, { type IconName } from "../components/Icon";
import Mascot from "../components/Mascot";
import { LANGS, useT, type Key } from "../i18n";
import PlayerHead from "../components/PlayerHead";
import { api, isTauri, skinUrl, type AfterLaunch, type JavaInstall, type Lang } from "../lib/api";
import { DEFAULT_CONFIG, useApp } from "../store/app";

type Section = "accounts" | "general" | "game" | "java" | "downloads" | "about";
const SECTIONS: { id: Section; icon: IconName }[] = [
  { id: "accounts", icon: "user" },
  { id: "general", icon: "settings" },
  { id: "game", icon: "monitor" },
  { id: "java", icon: "coffee" },
  { id: "downloads", icon: "download" },
  { id: "about", icon: "info" },
];

const RESOLUTIONS: [number, number][] = [[854, 480], [1280, 720], [1600, 900], [1920, 1080], [2560, 1440]];

/** Fila de ajuste estilo app: título + descripción a la izquierda, control a la derecha */
function Row({ title, hint, children }: { title: string; hint?: string; children: ReactNode }) {
  return (
    <div className="setting-row">
      <div className="setting-row__text"><strong>{title}</strong>{hint && <small>{hint}</small>}</div>
      <div className="setting-row__control">{children}</div>
    </div>
  );
}

function Switch({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="switch">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      <span />
    </label>
  );
}

export default function Settings() {
  const t = useT();
  const [section, setSection] = useState<Section>("accounts");
  return (
    <div className="page settings">
      <header className="page__head"><h1 className="display">{t("set.title")}</h1></header>
      <div className="settings__layout">
        <nav className="settings__nav card">
          {SECTIONS.map((s) => (
            <button key={s.id} className={section === s.id ? "is-active" : ""} onClick={() => setSection(s.id)}>
              <Icon name={s.icon} size={18} /> {t(`set.section.${s.id}` as Key)}
            </button>
          ))}
        </nav>
        <div key={section} className="settings__body card tab-enter">
          {section === "accounts" && <Accounts />}
          {section === "general" && <General />}
          {section === "game" && <Game />}
          {section === "java" && <Java />}
          {section === "downloads" && <Downloads />}
          {section === "about" && <About />}
        </div>
      </div>
    </div>
  );
}

function useConfig() {
  const config = useApp((s) => s.config) ?? DEFAULT_CONFIG;
  const update = useApp((s) => s.updateConfig);
  return [config, update] as const;
}

function Accounts() {
  const t = useT();
  const accounts = useApp((s) => s.accounts);
  const active = useApp((s) => s.activeAccount());
  const updateConfig = useApp((s) => s.updateConfig);
  const refreshAccounts = useApp((s) => s.refreshAccounts);
  const [config] = useConfig();
  const [adding, setAdding] = useState(false);

  return (
    <div className="stack">
      <h2 className="settings__title">{t("set.section.accounts")}</h2>
      <ul className="account-list stagger">
        {accounts.map((a) => (
          <li key={a.uuid} className={active?.uuid === a.uuid ? "is-active" : ""}>
            <span className="account-head head-clip"><PlayerHead skin={skinUrl(a)} size={22} /></span>
            <span className="grow">
              <strong>{a.username}</strong>
              <small className="muted"> {a.kind === "microsoft" ? "Microsoft · Premium" : "No-Premium"}</small>
            </span>
            {active?.uuid === a.uuid ? (
              <span className="badge badge--yellow">{t("set.active")}</span>
            ) : (
              <button className="btn btn-ghost btn-sm" onClick={() => updateConfig({ activeAccount: a.uuid })}>{t("set.use")}</button>
            )}
            <button className="icon-btn" onClick={async () => { await api.removeAccount(a.uuid); await refreshAccounts(); }} aria-label={t("set.remove")}>
              <Icon name="trash" size={16} />
            </button>
          </li>
        ))}
      </ul>
      <button className="btn btn-primary file-btn" onClick={() => setAdding(true)}><Icon name="plus" /> {t("set.add")}</button>
      <Row title={t("set.msClientId")} hint={t("set.msClientIdHint")}>
        <input value={config.msClientId ?? ""} placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx" spellCheck={false}
          onChange={(e) => updateConfig({ msClientId: e.target.value.trim() || null })} />
      </Row>
      {adding && <AccountModal onClose={() => setAdding(false)} />}
    </div>
  );
}

function General() {
  const t = useT();
  const [c, update] = useConfig();
  return (
    <div className="stack">
      <h2 className="settings__title">{t("set.section.general")}</h2>
      <Row title={t("set.language")}>
        <div className="segmented">
          {LANGS.map((l) => (
            <button key={l.id} className={c.language === l.id ? "is-active" : ""} onClick={() => update({ language: l.id as Lang })}>{l.label}</button>
          ))}
        </div>
      </Row>
      <Row title={t("set.afterLaunch")}>
        <div className="segmented">
          {(["keep", "hide", "close"] as AfterLaunch[]).map((v) => (
            <button key={v} className={c.afterLaunch === v ? "is-active" : ""} onClick={() => update({ afterLaunch: v })}>{t(`set.after.${v}`)}</button>
          ))}
        </div>
      </Row>
      <Row title={t("set.notifications")} hint={t("set.notificationsHint")}>
        <Switch checked={c.notifications} onChange={(v) => update({ notifications: v })} />
      </Row>
      <Row title={t("set.multiInstances")} hint={t("set.multiInstancesHint")}>
        <Switch checked={c.allowMultipleInstances} onChange={(v) => update({ allowMultipleInstances: v })} />
      </Row>
      <Row title={t("set.animations")} hint={t("set.animationsHint")}>
        <Switch checked={c.animations} onChange={(v) => update({ animations: v })} />
      </Row>
    </div>
  );
}

function Game() {
  const t = useT();
  const [c, update] = useConfig();
  return (
    <div className="stack">
      <h2 className="settings__title">{t("set.section.game")}</h2>
      <Row title={t("set.memory")} hint={`${c.minRamMb} – ${c.maxRamMb} MB`}>
        <div className="stack ram">
          <input type="range" min={512} max={16384} step={256} value={c.minRamMb} onChange={(e) => update({ minRamMb: Number(e.target.value) })} />
          <input type="range" min={1024} max={32768} step={512} value={c.maxRamMb} onChange={(e) => update({ maxRamMb: Number(e.target.value) })} />
        </div>
      </Row>
      <Row title={t("set.resolution")} hint={`${c.gameWidth} × ${c.gameHeight}`}>
        <select
          value={`${c.gameWidth}x${c.gameHeight}`}
          disabled={c.fullscreen}
          onChange={(e) => { const [w, h] = e.target.value.split("x").map(Number); update({ gameWidth: w, gameHeight: h }); }}
        >
          {RESOLUTIONS.map(([w, h]) => <option key={w} value={`${w}x${h}`}>{w} × {h}</option>)}
        </select>
      </Row>
      <Row title={t("set.fullscreen")}>
        <Switch checked={c.fullscreen} onChange={(v) => update({ fullscreen: v })} />
      </Row>
      <Row title={t("set.skinServer")}>
        <input value={c.skinServer ?? ""} placeholder="https://skins.kirmc.example/api/yggdrasil"
          onChange={(e) => update({ skinServer: e.target.value || null })} />
      </Row>
    </div>
  );
}

function Java() {
  const t = useT();
  const [c, update] = useConfig();
  const [javas, setJavas] = useState<JavaInstall[] | null>(null);
  const [scanning, setScanning] = useState(false);
  const detect = async () => {
    setScanning(true);
    try { setJavas(await api.detectJava()); } finally { setScanning(false); }
  };
  return (
    <div className="stack">
      <h2 className="settings__title">{t("set.section.java")}</h2>
      <div className="row">
        <input className="grow" value={c.javaPath ?? ""} placeholder="auto" onChange={(e) => update({ javaPath: e.target.value || null })} />
        <button className="btn btn-ghost" onClick={detect} disabled={scanning}>{scanning ? <span className="spinner" /> : <Icon name="search" size={16} />} {t("set.detect")}</button>
      </div>
      {javas && (
        <ul className="java-list stagger">
          {javas.map((j) => (
            <li key={j.path}>
              <button className={c.javaPath === j.path ? "is-active" : ""} onClick={() => update({ javaPath: j.path })}>
                <span className="badge badge--yellow">Java {j.major}</span> <code>{j.path}</code>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function Downloads() {
  const t = useT();
  const [c, update] = useConfig();
  return (
    <div className="stack">
      <h2 className="settings__title">{t("set.section.downloads")}</h2>
      <Row title={t("set.downloads")} hint={String(c.concurrentDownloads)}>
        <input type="range" min={4} max={64} step={4} value={c.concurrentDownloads} onChange={(e) => update({ concurrentDownloads: Number(e.target.value) })} />
      </Row>
      <Row title={t("set.verifyFiles")} hint={t("set.verifyFilesHint")}>
        <Switch checked={c.verifyFiles} onChange={(v) => update({ verifyFiles: v })} />
      </Row>
    </div>
  );
}

function About() {
  const t = useT();
  const [dir, setDir] = useState("");
  useEffect(() => { if (isTauri) api.dataDir().then(setDir).catch(() => {}); }, []);
  return (
    <div className="stack about">
      <Mascot size={140} />
      <h2 className="display">KIRMC Launcher</h2>
      <p className="muted">{t("set.version")} 0.1.0 · Tauri 2 · React</p>
      <Row title={t("set.dataFolder")} hint={dir}>
        <button className="btn btn-ghost btn-sm" disabled={!dir} onClick={() => openPath(dir)}><Icon name="folder" size={14} /> {t("set.open")}</button>
      </Row>
    </div>
  );
}
