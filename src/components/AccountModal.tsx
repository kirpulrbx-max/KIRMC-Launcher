import { useState } from "react";
import { useT } from "../i18n";
import { api } from "../lib/api";
import { useApp } from "../store/app";
import Mascot from "./Mascot";
import Modal from "./Modal";

type Tab = "microsoft" | "offline";

export default function AccountModal({ onClose }: { onClose: () => void }) {
  const t = useT();
  const refreshAccounts = useApp((s) => s.refreshAccounts);
  const updateConfig = useApp((s) => s.updateConfig);
  const [tab, setTab] = useState<Tab>("microsoft");
  const [username, setUsername] = useState("");
  const [waiting, setWaiting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const done = async (uuid: string) => {
    await refreshAccounts();
    await updateConfig({ activeAccount: uuid });
    onClose();
  };

  // Se abre la página oficial de Microsoft en su propia ventana; el launcher nunca ve tu contraseña
  const loginMicrosoft = async () => {
    setWaiting(true);
    setError(null);
    try {
      const acc = await api.msLoginWindow();
      await done(acc.uuid);
    } catch (e) {
      setError(String(e));
    } finally {
      setWaiting(false);
    }
  };

  const addOffline = async () => {
    setError(null);
    try {
      const acc = await api.addOfflineAccount(username);
      await done(acc.uuid);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <Modal title={t("set.accounts")} onClose={onClose}>
      <div className="segmented">
        <button className={tab === "microsoft" ? "is-active" : ""} onClick={() => setTab("microsoft")}>Premium</button>
        <button className={tab === "offline" ? "is-active" : ""} onClick={() => setTab("offline")}>No-Premium</button>
      </div>

      {tab === "microsoft" ? (
        <div className="stack center">
          {waiting ? (
            <div className="row muted"><Mascot mood="loading" size={56} /> {t("set.msWaiting")}</div>
          ) : (
            <button className="btn btn-primary btn-lg" onClick={loginMicrosoft}>{t("set.addMicrosoft")}</button>
          )}
          <p className="small muted center-text">{t("set.msPrivacy")}</p>
        </div>
      ) : (
        <form className="stack" onSubmit={(e) => { e.preventDefault(); addOffline(); }}>
          <label className="field">
            <span>{t("set.username")}</span>
            <input value={username} maxLength={16} onChange={(e) => setUsername(e.target.value)} placeholder="Steve" autoFocus />
          </label>
          <button className="btn btn-primary" disabled={username.trim().length < 3}>{t("set.add")}</button>
        </form>
      )}

      {error && <p className="error-text">{error}</p>}
    </Modal>
  );
}
