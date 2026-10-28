import { startTransition } from "react";
import { useT, type Key } from "../i18n";
import { skinUrl } from "../lib/api";
import PlayerHead from "./PlayerHead";
import { useApp, type Page } from "../store/app";
import Icon, { type IconName } from "./Icon";

const ITEMS: { page: Page; icon: IconName; label: Key }[] = [
  { page: "play", icon: "play", label: "nav.play" },
  { page: "instances", icon: "layers", label: "nav.instances" },
  { page: "chat", icon: "chat", label: "nav.chat" },
  { page: "news", icon: "news", label: "nav.news" },
  { page: "characters", icon: "user", label: "nav.characters" },
  { page: "settings", icon: "settings", label: "nav.settings" },
];

export default function Sidebar() {
  const t = useT();
  const page = useApp((s) => s.page);
  const setPage = useApp((s) => s.setPage);
  const account = useApp((s) => s.accounts.find((a) => a.uuid === s.config?.activeAccount) ?? s.accounts[0]);
  const index = ITEMS.findIndex((i) => i.page === page);

  return (
    <nav className="sidebar">
      <div className="sidebar__items">
        {/* Indicador que se desliza con transform (GPU), sin re-layout */}
        <span className="sidebar__pill" style={{ transform: `translateY(${index * 64}px)` }} />
        {ITEMS.map((item) => (
          <button
            key={item.page}
            className={`sidebar__item ${page === item.page ? "is-active" : ""}`}
            // Transición: la página actual sigue visible mientras carga el chunk de la siguiente
            onClick={() => startTransition(() => setPage(item.page))}
            title={t(item.label)}
          >
            <Icon name={item.icon} size={22} />
            <span>{t(item.label)}</span>
          </button>
        ))}
      </div>
      <button className="sidebar__account head-host" onClick={() => startTransition(() => setPage("settings"))} title={account?.username ?? ""}>
        <span className="head-clip"><PlayerHead skin={skinUrl(account)} size={28} /></span>
        {account && <span className={`dot dot--${account.kind === "microsoft" ? "premium" : "offline"}`} />}
      </button>
    </nav>
  );
}
