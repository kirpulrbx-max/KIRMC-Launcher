import { openPath } from "@tauri-apps/plugin-opener";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useT } from "../../i18n";
import { api, type Instance, type ModFile } from "../../lib/api";
import { useApp } from "../../store/app";
import Icon from "../Icon";
import ModrinthBrowser from "./ModrinthBrowser";

type Tab = "general" | "content" | "logs";
const VISIBLE_LOG_LINES = 400; // el resto queda en memoria, no en el DOM

export default function InstanceDetail({ instance, onClose }: { instance: Instance; onClose: () => void }) {
  const t = useT();
  const [tab, setTab] = useState<Tab>("general");
  return (
    <aside className="drawer">
      <header className="drawer__head">
        <div>
          <h2 className="display">{instance.name}</h2>
          <small className="muted">{instance.mcVersion} · {instance.loader} {instance.loaderVersion ?? ""}</small>
        </div>
        <button className="icon-btn" onClick={onClose} aria-label="Cerrar"><Icon name="close" /></button>
      </header>
      <div className="segmented">
        {(["general", "content", "logs"] as Tab[]).map((x) => (
          <button key={x} className={tab === x ? "is-active" : ""} onClick={() => setTab(x)}>{t(`inst.tab.${x}`)}</button>
        ))}
      </div>
      <div className="drawer__body">
        {tab === "general" && <General instance={instance} onDeleted={onClose} />}
        {tab === "content" && <Content instance={instance} />}
        {tab === "logs" && <Logs id={instance.id} />}
      </div>
    </aside>
  );
}

function General({ instance, onDeleted }: { instance: Instance; onDeleted: () => void }) {
  const t = useT();
  const config = useApp((s) => s.config);
  const refreshInstances = useApp((s) => s.refreshInstances);
  const install = useApp((s) => s.install);
  const [draft, setDraft] = useState(instance);
  const [confirm, setConfirm] = useState(false);
  const min = draft.minRamMb ?? config?.minRamMb ?? 1024;
  const max = draft.maxRamMb ?? config?.maxRamMb ?? 4096;

  const save = async () => {
    await api.updateInstance(draft);
    await refreshInstances();
  };
  const remove = async () => {
    if (!confirm) return setConfirm(true);
    await api.deleteInstance(instance.id);
    await refreshInstances();
    onDeleted();
  };

  return (
    <div className="stack">
      <label className="field">
        <span>{t("inst.name")}</span>
        <input value={draft.name} onChange={(e) => setDraft({ ...draft, name: e.target.value })} />
      </label>

      <div className="field">
        <span>{t("inst.ram")}: {min} – {max} MB</span>
        <label className="range"><small>{t("inst.ramMin")}</small>
          <input type="range" min={512} max={16384} step={256} value={min}
            onChange={(e) => setDraft({ ...draft, minRamMb: Number(e.target.value) })} />
        </label>
        <label className="range"><small>{t("inst.ramMax")}</small>
          <input type="range" min={1024} max={32768} step={512} value={max}
            onChange={(e) => setDraft({ ...draft, maxRamMb: Number(e.target.value) })} />
        </label>
      </div>

      <label className="field">
        <span>{t("inst.java")}</span>
        <input value={draft.javaPath ?? ""} placeholder="auto"
          onChange={(e) => setDraft({ ...draft, javaPath: e.target.value || null })} />
      </label>
      <label className="field">
        <span>{t("inst.jvmArgs")}</span>
        <input value={draft.jvmArgs.join(" ")} placeholder="-Dfml.ignorePatchDiscrepancies=true"
          onChange={(e) => setDraft({ ...draft, jvmArgs: e.target.value.split(/\s+/).filter(Boolean) })} />
      </label>

      <div className="row wrap">
        <button className="btn btn-primary" onClick={save}>{t("inst.save")}</button>
        <button className="btn btn-ghost" onClick={() => install(instance.id)}><Icon name="refresh" size={16} /> {t("inst.install")}</button>
        <button className="btn btn-ghost" onClick={async () => openPath(await api.instanceDir(instance.id))}>
          <Icon name="folder" size={16} /> {t("inst.openFolder")}
        </button>
        <button className="btn btn-danger" onClick={remove}><Icon name="trash" size={16} /> {confirm ? t("inst.confirmDelete") : t("inst.delete")}</button>
      </div>
    </div>
  );
}

function Content({ instance }: { instance: Instance }) {
  const t = useT();
  const [reload, setReload] = useState(0);
  return (
    <div className="stack">
      <details className="installed" open>
        <summary className="card__title">{t("inst.installed")}</summary>
        <Mods id={instance.id} reload={reload} />
      </details>
      <ModrinthBrowser types={["mod", "resourcepack", "shader"]} instance={instance} onInstalled={() => setReload((r) => r + 1)} />
    </div>
  );
}

function Mods({ id, reload }: { id: string; reload: number }) {
  const t = useT();
  const [mods, setMods] = useState<ModFile[]>([]);
  const load = () => api.listMods(id).then(setMods).catch(() => setMods([]));
  useEffect(() => { load(); }, [id, reload]); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="stack">
      <div className="row">
        <button className="btn btn-ghost btn-sm" onClick={async () => openPath(await api.instanceDir(id, "mods"))}>
          <Icon name="folder" size={14} /> {t("inst.openMods")}
        </button>
        <button className="icon-btn" onClick={load} aria-label="Recargar"><Icon name="refresh" size={16} /></button>
      </div>
      {mods.length === 0 && <p className="muted">{t("inst.noMods")}</p>}
      <ul className="mod-list">
        {mods.map((m) => (
          <li key={m.file} className={m.enabled ? "" : "is-disabled"}>
            <span>{m.file.replace(/\.jar(\.disabled)?$/, "")}</span>
            <small>{(m.size / 1024 / 1024).toFixed(1)} MB</small>
            <label className="switch">
              <input type="checkbox" checked={m.enabled} onChange={async () => { await api.toggleMod(id, m.file); load(); }} />
              <span />
            </label>
          </li>
        ))}
      </ul>
    </div>
  );
}

function Logs({ id }: { id: string }) {
  const t = useT();
  const lines = useApp((s) => s.logs[id]);
  const ref = useRef<HTMLPreElement>(null);
  const visible = lines?.slice(-VISIBLE_LOG_LINES) ?? [];

  // Auto-scroll solo si el usuario ya estaba abajo
  useLayoutEffect(() => {
    const el = ref.current;
    if (el && el.scrollHeight - el.scrollTop - el.clientHeight < 80) el.scrollTop = el.scrollHeight;
  }, [lines]);

  if (!visible.length) return <p className="muted">{t("inst.noLogs")}</p>;
  return (
    <pre className="logs" ref={ref}>
      {visible.map((l, i) => (
        <div key={i} className={l.err || /ERROR|Exception/.test(l.text) ? "log-err" : /WARN/.test(l.text) ? "log-warn" : ""}>{l.text}</div>
      ))}
    </pre>
  );
}
