import { useEffect, useMemo, useState } from "react";
import { useT } from "../../i18n";
import { api, isTauri, type Manifest, type VersionType } from "../../lib/api";
import { useApp } from "../../store/app";
import Icon from "../Icon";

const TYPES: VersionType[] = ["release", "snapshot", "old_beta", "old_alpha"];
const PAGE = 60;

/** Catálogo de todas las versiones oficiales de Minecraft, con descarga directa */
export default function VersionBrowser({ onCustomize, onCreated }: {
  onCustomize: (version: string) => void;
  onCreated: (id: string) => void;
}) {
  const t = useT();
  const instances = useApp((s) => s.instances);
  const refreshInstances = useApp((s) => s.refreshInstances);
  const select = useApp((s) => s.select);
  const install = useApp((s) => s.install);
  const setError = useApp((s) => s.setError);
  const [manifest, setManifest] = useState<Manifest | null>(null);
  const [type, setType] = useState<VersionType>("release");
  const [query, setQuery] = useState("");
  const [limit, setLimit] = useState(PAGE);

  useEffect(() => {
    if (isTauri) api.listVersions().then(setManifest).catch((e) => setError(String(e)));
  }, [setError]);
  useEffect(() => setLimit(PAGE), [type, query]);

  const owned = useMemo(() => {
    const m = new Map<string, number>();
    instances.forEach((i) => m.set(i.mcVersion, (m.get(i.mcVersion) ?? 0) + 1));
    return m;
  }, [instances]);

  const list = useMemo(
    () => manifest?.versions.filter((v) => v.type === type && v.id.toLowerCase().includes(query.trim().toLowerCase())) ?? [],
    [manifest, type, query],
  );

  const quickDownload = async (version: string) => {
    try {
      const inst = await api.createInstance({ name: version, mcVersion: version, loader: "vanilla", loaderVersion: null });
      await refreshInstances();
      select(inst.id);
      onCreated(inst.id);
      install(inst.id);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="versions">
      <div className="toolbar">
        <label className="search-field">
          <Icon name="search" size={16} />
          <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t("ver.search")} />
        </label>
        <div className="segmented">
          {TYPES.map((ty) => (
            <button key={ty} className={type === ty ? "is-active" : ""} onClick={() => setType(ty)}>{t(`ver.${ty}`)}</button>
          ))}
        </div>
      </div>

      <div className="version-list card">
        {!manifest && Array.from({ length: 8 }, (_, i) => <div key={i} className="skeleton skeleton--row" />)}
        {list.slice(0, limit).map((v, i) => {
          const latest = v.id === manifest?.latest.release || v.id === manifest?.latest.snapshot;
          const have = owned.get(v.id);
          return (
            <div key={v.id} className="version-row" style={{ "--i": Math.min(i, 14) } as React.CSSProperties}>
              <span className={`version-row__icon type--${v.type}`}><Icon name="box" size={18} /></span>
              <strong>{v.id}</strong>
              {latest && <span className="badge badge--yellow">{t("ver.latest")}</span>}
              {have && <span className="badge badge--brown">{t("ver.have", { n: have })}</span>}
              <small className="muted grow">{new Date(v.releaseTime).toLocaleDateString()}</small>
              <button className="btn btn-ghost btn-sm" onClick={() => onCustomize(v.id)}>{t("ver.create")}</button>
              <button className="btn btn-primary btn-sm" onClick={() => quickDownload(v.id)}>
                <Icon name="download" size={14} /> {t("ver.download")}
              </button>
            </div>
          );
        })}
        {list.length > limit && (
          <button className="btn btn-ghost load-more" onClick={() => setLimit((l) => l + PAGE)}>+ {list.length - limit}</button>
        )}
      </div>
    </div>
  );
}
