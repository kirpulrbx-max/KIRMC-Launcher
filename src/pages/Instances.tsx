import { useState } from "react";
import Icon from "../components/Icon";
import InstanceDetail from "../components/instances/InstanceDetail";
import ModrinthBrowser from "../components/instances/ModrinthBrowser";
import NewInstanceModal from "../components/instances/NewInstanceModal";
import VersionBrowser from "../components/instances/VersionBrowser";
import { useT } from "../i18n";
import { useApp } from "../store/app";
import { LOADER_LABEL } from "./Play";

type Tab = "mine" | "versions" | "modpacks";

export default function Instances() {
  const t = useT();
  const instances = useApp((s) => s.instances);
  const running = useApp((s) => s.running);
  const progress = useApp((s) => s.progress);
  const busy = useApp((s) => s.busy);
  const [tab, setTab] = useState<Tab>("mine");
  const [creating, setCreating] = useState<string | null>(null); // versión preseleccionada
  const [openId, setOpenId] = useState<string | null>(null);
  const open = instances.find((i) => i.id === openId);
  const downloads = Object.values(progress);

  const goMine = (id: string) => {
    setTab("mine");
    setOpenId(id);
  };

  return (
    <div className="page">
      <header className="page__head">
        <h1 className="display">{t("inst.title")}</h1>
        <div className="tabs">
          {(["mine", "versions", "modpacks"] as Tab[]).map((x) => (
            <button key={x} className={tab === x ? "is-active" : ""} onClick={() => setTab(x)}>
              {t(`inst.tab.${x}`)}
              {x === "mine" && <span className="tabs__count">{instances.length}</span>}
            </button>
          ))}
        </div>
        <button className="btn btn-primary" onClick={() => setCreating("")}><Icon name="plus" /> {t("inst.new")}</button>
      </header>

      {downloads.length > 0 && (
        <section className="card downloads">
          <h3 className="card__title"><Icon name="download" size={16} /> {t("ver.downloads")}</h3>
          {downloads.map((p) => (
            <div key={p.tag} className="download-row">
              <strong>{instances.find((i) => i.id === p.tag)?.name ?? p.tag}</strong>
              <div className="progress is-active grow">
                <div className="progress__bar" style={{ transform: `scaleX(${p.done / Math.max(p.total, 1)})` }} />
                <span>{t(`play.stage.${p.stage}`, { done: p.done, total: p.total })}</span>
              </div>
            </div>
          ))}
        </section>
      )}

      {tab === "mine" &&
        (instances.length === 0 ? (
          <div className="empty card"><p>{t("inst.empty")}</p>
            <button className="btn btn-primary" onClick={() => setTab("versions")}><Icon name="download" size={16} /> {t("inst.tab.versions")}</button>
          </div>
        ) : (
          <div className="instance-grid stagger">
            {instances.map((i) => {
              const p = progress[i.id];
              return (
                <button key={i.id} className={`instance-card ${openId === i.id ? "is-open" : ""}`} onClick={() => setOpenId(i.id)}>
                  <span className={`instance-card__loader loader--${i.loader}`}>{LOADER_LABEL[i.loader]}</span>
                  <strong>{i.name}</strong>
                  <small>{i.mcVersion}{i.loaderVersion ? ` · ${i.loaderVersion}` : ""}</small>
                  {running[i.id] && <span className="instance-card__state is-running">●</span>}
                  {busy[i.id] && !running[i.id] && <span className="instance-card__state"><span className="spinner" /></span>}
                  {p && <span className="instance-card__bar" style={{ transform: `scaleX(${p.done / Math.max(p.total, 1)})` }} />}
                </button>
              );
            })}
          </div>
        ))}

      {tab === "versions" && <VersionBrowser onCustomize={(v) => setCreating(v)} onCreated={goMine} />}
      {tab === "modpacks" && <ModrinthBrowser types={["modpack"]} />}

      {open && <InstanceDetail key={open.id} instance={open} onClose={() => setOpenId(null)} />}
      {creating !== null && (
        <NewInstanceModal initialVersion={creating || undefined} onClose={() => setCreating(null)} onCreated={goMine} />
      )}
    </div>
  );
}
