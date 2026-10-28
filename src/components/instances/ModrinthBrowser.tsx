import { useEffect, useRef, useState } from "react";
import { useT } from "../../i18n";
import { api, formatCount, isTauri, type Instance, type MrHit, type ProjectType } from "../../lib/api";
import { useApp } from "../../store/app";
import Icon from "../Icon";

type InstallState = "installing" | "installed" | "error";

/**
 * Buscador de Modrinth reutilizable.
 * - Con `instance`: filtra por versión/cargador e instala dentro de esa instancia.
 * - Con tipo "modpack": instala el modpack como instancia nueva.
 */
export default function ModrinthBrowser({ types, instance, onInstalled }: {
  types: ProjectType[];
  instance?: Instance;
  onInstalled?: () => void;
}) {
  const t = useT();
  const setError = useApp((s) => s.setError);
  const refreshInstances = useApp((s) => s.refreshInstances);
  const [type, setType] = useState<ProjectType>(types[0]);
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<MrHit[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [state, setState] = useState<Record<string, InstallState>>({});
  const reqId = useRef(0);

  const vanillaBlocked = !!instance && instance.loader === "vanilla" && type === "mod";

  const search = async (offset: number) => {
    if (!isTauri || vanillaBlocked) return;
    const id = ++reqId.current;
    setLoading(true);
    try {
      const res = await api.modrinthSearch({
        query,
        projectType: type,
        gameVersion: instance?.mcVersion ?? null,
        loader: instance?.loader ?? null,
        offset,
      });
      if (id !== reqId.current) return; // respuesta obsoleta
      setHits((prev) => (offset === 0 ? res.hits : [...prev, ...res.hits]));
      setTotal(res.totalHits);
    } catch (e) {
      setError(String(e));
    } finally {
      if (id === reqId.current) setLoading(false);
    }
  };

  // Búsqueda con debounce
  useEffect(() => {
    const h = setTimeout(() => search(0), 320);
    return () => clearTimeout(h);
  }, [query, type, instance?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const install = async (hit: MrHit) => {
    setState((s) => ({ ...s, [hit.projectId]: "installing" }));
    try {
      if (hit.projectType === "modpack") {
        const inst = await api.modrinthInstallModpack(hit.projectId);
        await refreshInstances();
        setError(t("mr.modpackDone", { name: inst.name }));
      } else if (instance) {
        await api.modrinthInstall(instance.id, hit.projectId, hit.projectType);
      }
      setState((s) => ({ ...s, [hit.projectId]: "installed" }));
      onInstalled?.();
    } catch (e) {
      setState((s) => ({ ...s, [hit.projectId]: "error" }));
      setError(String(e));
    }
  };

  return (
    <div className="mr">
      <div className="toolbar">
        <label className="search-field">
          <Icon name="search" size={16} />
          <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t("mr.search")} />
        </label>
        {types.length > 1 && (
          <div className="segmented">
            {types.map((ty) => (
              <button key={ty} className={type === ty ? "is-active" : ""} onClick={() => setType(ty)}>
                {t(`mr.type.${ty as "mod" | "resourcepack" | "shader"}`)}
              </button>
            ))}
          </div>
        )}
      </div>
      {instance && !vanillaBlocked && (
        <p className="small muted">{t("mr.filtered", { version: instance.mcVersion, loader: instance.loader })}</p>
      )}
      {vanillaBlocked && <p className="banner">{t("mr.vanillaNote")}</p>}

      <div className={`mr-grid ${instance ? "mr-grid--compact" : ""}`}>
        {loading && hits.length === 0 && Array.from({ length: 6 }, (_, i) => <div key={i} className="skeleton skeleton--card" />)}
        {hits.map((h, i) => {
          const st = state[h.projectId];
          return (
            <article key={h.projectId} className="mr-card" style={{ "--i": i % 12 } as React.CSSProperties}>
              {h.iconUrl ? <img src={h.iconUrl} alt="" width={56} height={56} loading="lazy" /> : <div className="mr-card__noicon"><Icon name="box" /></div>}
              <div className="mr-card__body">
                <strong>{h.title}</strong>
                <small className="muted">{t("mr.by", { author: h.author })}</small>
                <p>{h.description}</p>
                <div className="mr-card__meta">
                  <span><Icon name="download" size={13} /> {formatCount(h.downloads)}</span>
                  <span>♥ {formatCount(h.follows)}</span>
                </div>
              </div>
              <button
                className={`btn btn-sm ${st === "installed" ? "btn-ghost" : "btn-primary"}`}
                disabled={st === "installing" || st === "installed" || (!instance && h.projectType !== "modpack")}
                onClick={() => install(h)}
              >
                {st === "installing" ? <span className="spinner" /> : <Icon name={st === "installed" ? "check" : "download"} size={14} />}
                {st === "installing" ? t("mr.installing") : st === "installed" ? t("mr.installed") : t("mr.install")}
              </button>
            </article>
          );
        })}
      </div>

      {!loading && hits.length === 0 && !vanillaBlocked && <div className="empty">{t("mr.empty")}</div>}
      {hits.length > 0 && hits.length < total && (
        <button className="btn btn-ghost load-more" disabled={loading} onClick={() => search(hits.length)}>
          {loading ? <span className="spinner" /> : null} {t("mr.more")}
        </button>
      )}
    </div>
  );
}
