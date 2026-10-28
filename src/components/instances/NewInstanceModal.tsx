import { useEffect, useMemo, useState } from "react";
import { useT } from "../../i18n";
import { api, type Loader, type VersionEntry } from "../../lib/api";
import { useApp } from "../../store/app";
import Modal from "../Modal";

const LOADERS: Loader[] = ["vanilla", "fabric", "quilt", "forge"];

export default function NewInstanceModal({ initialVersion, onClose, onCreated }: {
  initialVersion?: string;
  onClose: () => void;
  onCreated: (id: string) => void;
}) {
  const t = useT();
  const refreshInstances = useApp((s) => s.refreshInstances);
  const select = useApp((s) => s.select);
  const install = useApp((s) => s.install);

  const [versions, setVersions] = useState<VersionEntry[]>([]);
  const [snapshots, setSnapshots] = useState(false);
  const [mcVersion, setMcVersion] = useState("");
  const [loader, setLoader] = useState<Loader>("vanilla");
  const [loaderVersions, setLoaderVersions] = useState<string[]>([]);
  const [loaderVersion, setLoaderVersion] = useState("");
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.listVersions().then((m) => {
      setVersions(m.versions);
      const initial = m.versions.find((v) => v.id === initialVersion);
      if (initial && initial.type !== "release") setSnapshots(true);
      setMcVersion(initial?.id ?? m.latest.release);
    }).catch((e) => setError(String(e)));
  }, [initialVersion]);

  const [loadingLoader, setLoadingLoader] = useState(false);
  useEffect(() => {
    setLoaderVersions([]);
    setLoaderVersion("");
    setError(null);
    if (loader === "vanilla" || !mcVersion) return;
    let alive = true;
    setLoadingLoader(true);
    api.listLoaderVersions(loader, mcVersion)
      .then((list) => { if (alive) { setLoaderVersions(list); setLoaderVersion(list[0] ?? ""); } })
      .catch((e) => alive && setError(String(e)))
      .finally(() => alive && setLoadingLoader(false));
    return () => { alive = false; };
  }, [loader, mcVersion]);
  const loaderUnsupported = loader !== "vanilla" && !loadingLoader && !error && loaderVersions.length === 0;

  const visible = useMemo(
    () => versions.filter((v) => v.type === "release" || snapshots || v.id === mcVersion),
    [versions, snapshots, mcVersion],
  );
  const canCreate = !!mcVersion && (loader === "vanilla" || !!loaderVersion);

  const create = async () => {
    try {
      const inst = await api.createInstance({
        name: name.trim() || `${mcVersion} ${loader === "vanilla" ? "" : loader}`.trim(),
        mcVersion,
        loader,
        loaderVersion: loader === "vanilla" ? null : loaderVersion,
      });
      await refreshInstances();
      select(inst.id);
      onCreated(inst.id);
      onClose();
      install(inst.id); // descarga en segundo plano; el progreso se ve en la barra
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <Modal title={t("inst.new")} onClose={onClose}>
      <form className="stack" onSubmit={(e) => { e.preventDefault(); create(); }}>
        <label className="field">
          <span>{t("inst.name")}</span>
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder={mcVersion} />
        </label>

        <label className="field">
          <span>{t("inst.version")}</span>
          <select value={mcVersion} onChange={(e) => setMcVersion(e.target.value)}>
            {visible.map((v) => <option key={v.id} value={v.id}>{v.id}{v.type === "snapshot" ? " (snapshot)" : ""}</option>)}
          </select>
        </label>
        <label className="check">
          <input type="checkbox" checked={snapshots} onChange={(e) => setSnapshots(e.target.checked)} /> {t("inst.snapshots")}
        </label>

        <div className="field">
          <span>{t("inst.loader")}</span>
          <div className="segmented">
            {LOADERS.map((l) => (
              <button type="button" key={l} className={loader === l ? "is-active" : ""} onClick={() => setLoader(l)}>{l}</button>
            ))}
          </div>
        </div>

        {loader !== "vanilla" && !loaderUnsupported && (
          <label className="field">
            <span>{t("inst.loaderVersion")}</span>
            <select value={loaderVersion} onChange={(e) => setLoaderVersion(e.target.value)} disabled={!loaderVersions.length}>
              {loadingLoader && <option>…</option>}
              {loaderVersions.map((v) => <option key={v} value={v}>{v}</option>)}
            </select>
          </label>
        )}
        {loaderUnsupported && (
          <p className="banner">{t("inst.loaderUnsupported", { loader: loader[0].toUpperCase() + loader.slice(1), version: mcVersion })}</p>
        )}

        {error && <p className="error-text">{error}</p>}
        <div className="row end">
          <button type="button" className="btn btn-ghost" onClick={onClose}>{t("common.cancel")}</button>
          <button className="btn btn-primary" disabled={!canCreate}>{t("inst.create")}</button>
        </div>
      </form>
    </Modal>
  );
}
