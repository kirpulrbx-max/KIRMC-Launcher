import { useEffect, useState } from "react";
import Icon from "../components/Icon";
import Mascot from "../components/Mascot";
import { useT } from "../i18n";
import PlayerHead from "../components/PlayerHead";
import { api, isTauri, skinUrl, type NewsItem } from "../lib/api";
import { useSocial } from "../lib/social";
import { useApp } from "../store/app";

export const LOADER_LABEL = { vanilla: "Vanilla", fabric: "Fabric", quilt: "Quilt", forge: "Forge" } as const;

export default function Play() {
  const t = useT();
  const instances = useApp((s) => s.instances);
  const selectedId = useApp((s) => s.selectedId);
  const mood = useApp((s) => s.mood);
  const running = useApp((s) => s.running);
  const selected = instances.find((i) => i.id === selectedId);
  const runningCount = Object.keys(running).length;

  return (
    <div className="play">
      <section className="stage">
        <div className="stage__info stagger">
          <span className="eyebrow">{t("play.selected")}</span>
          <h1 className="display stage__title">{selected?.name ?? "KIRMC"}</h1>
          {selected ? (
            <div className="chips">
              <span className="chip"><Icon name="box" size={14} /> {selected.mcVersion}</span>
              <span className={`chip loader--${selected.loader}`}>{LOADER_LABEL[selected.loader]} {selected.loaderVersion ?? ""}</span>
              <span className="chip chip--ghost">
                {selected.lastPlayed ? t("play.lastPlayed", { date: new Date(selected.lastPlayed * 1000).toLocaleDateString() }) : t("play.neverPlayed")}
              </span>
            </div>
          ) : (
            <p className="muted">{t("play.noInstance")}</p>
          )}
          <div className={`status status--${mood}`}>
            <span className="status__dot" />
            {t(`play.mood.${mood}`)}
            {runningCount > 0 && <span className="badge badge--yellow">{t("play.running", { n: runningCount })}</span>}
          </div>
        </div>
        <div className="stage__mascot">
          <span key={mood} className="speech">{t(`play.speech.${mood}`)}</span>
          <Mascot mood={mood} size={250} />
        </div>
      </section>

      <div className="play__grid">
        <div className="play__main stagger">
          <QuickNews />
          <QuickPlay />
        </div>
        <FriendsPanel />
      </div>

      <PlayBar />
    </div>
  );
}

/** Barra inferior fija estilo launcher: cuenta · instancia · progreso · JUGAR */
function PlayBar() {
  const t = useT();
  const instances = useApp((s) => s.instances);
  const selectedId = useApp((s) => s.selectedId);
  const select = useApp((s) => s.select);
  const mood = useApp((s) => s.mood);
  const progress = useApp((s) => (s.selectedId ? s.progress[s.selectedId] : undefined));
  const busy = useApp((s) => !!(s.selectedId && s.busy[s.selectedId]));
  const running = useApp((s) => !!(s.selectedId && s.running[s.selectedId]));
  const launch = useApp((s) => s.launch);
  const stop = useApp((s) => s.stop);
  const setPage = useApp((s) => s.setPage);
  const account = useApp((s) => s.activeAccount());

  const onPlay = () => {
    if (!selectedId) return setPage("instances");
    if (!account) return setPage("settings");
    if (running) stop(selectedId);
    else launch();
  };
  const pct = progress ? progress.done / Math.max(progress.total, 1) : 0;

  return (
    <footer className="playbar">
      <button className="playbar__account head-host" onClick={() => setPage("settings")}>
        <span className="playbar__head head-clip"><PlayerHead skin={skinUrl(account)} size={24} /></span>
        <span className="playbar__who">
          <strong>{account?.username ?? t("play.noAccount")}</strong>
          <small>{account ? (account.kind === "microsoft" ? "Premium" : "No-Premium") : t("play.accountHint")}</small>
        </span>
      </button>

      <div className="playbar__center">
        <select value={selectedId ?? ""} onChange={(e) => select(e.target.value)} disabled={!instances.length}>
          {!instances.length && <option value="">{t("play.noInstance")}</option>}
          {instances.map((i) => (
            <option key={i.id} value={i.id}>
              {i.name} · {i.mcVersion}{i.loader !== "vanilla" ? ` · ${LOADER_LABEL[i.loader]}` : ""}
            </option>
          ))}
        </select>
        {progress ? (
          <div className="progress is-active">
            <div className="progress__bar" style={{ transform: `scaleX(${pct})` }} />
            <span>{t(`play.stage.${progress.stage}`, { done: progress.done, total: progress.total })}</span>
          </div>
        ) : (
          <span className={`playbar__status status--${mood}`}><span className="status__dot" /> {t(`play.mood.${mood}`)}</span>
        )}
      </div>

      <button className={`play-btn ${running ? "is-running" : ""} ${busy ? "is-busy" : ""}`} onClick={onPlay} disabled={busy}>
        <span className="play-btn__shine" aria-hidden="true" />
        <Icon name={running ? "stop" : "play"} size={24} />
        {busy ? t("play.preparing") : running ? t("play.stop") : t("play.launch")}
      </button>
    </footer>
  );
}

function FriendsPanel() {
  const t = useT();
  const friends = useSocial((s) => s.friends);
  const openChat = useSocial((s) => s.openChat);
  const setPage = useApp((s) => s.setPage);
  const online = friends.filter((f) => f.presence !== "offline");
  return (
    <aside className="card friends">
      <h3 className="card__title"><Icon name="user" size={16} /> {t("play.friends")} <span className="badge badge--yellow">{online.length}</span></h3>
      <ul className="friend-list">
        {online.map((f) => (
          <li key={f.uuid}>
            <button className="friend" onClick={() => { openChat(f.uuid); setPage("chat"); }}>
              <img src={`https://mc-heads.net/avatar/${f.name}/32`} alt="" width={32} height={32} loading="lazy" />
              <span className="friend__name">{f.name}<small>{f.activity ?? f.presence}</small></span>
              <span className={`dot dot--${f.presence}`} />
            </button>
          </li>
        ))}
      </ul>
    </aside>
  );
}

function QuickNews() {
  const t = useT();
  const setPage = useApp((s) => s.setPage);
  const [news, setNews] = useState<NewsItem[] | null>(null);
  useEffect(() => {
    if (isTauri) api.fetchNews().then((n) => setNews(n.slice(0, 3))).catch(() => setNews([]));
    else setNews([]);
  }, []);
  if (news?.length === 0) return null; // sin noticias: no dejar una tarjeta vacía
  return (
    <section className="card quick-news">
      <h3 className="card__title"><Icon name="news" size={16} /> {t("play.news")}</h3>
      <div className="quick-news__list">
        {news === null && [0, 1, 2].map((i) => <div key={i} className="skeleton" />)}
        {news?.map((n) => (
          <button key={n.id} className="quick-news__item" onClick={() => setPage("news")}>
            {n.image && <img src={n.image} alt="" loading="lazy" />}
            <span><span className="badge badge--red">{n.tag}</span>{n.title}</span>
          </button>
        ))}
      </div>
    </section>
  );
}

function QuickPlay() {
  const t = useT();
  const servers = useApp((s) => s.config?.favoriteServers ?? []);
  const updateConfig = useApp((s) => s.updateConfig);
  const launch = useApp((s) => s.launch);
  const [adding, setAdding] = useState(false);
  const [name, setName] = useState("");
  const [address, setAddress] = useState("");

  const add = async () => {
    if (!address.trim()) return;
    await updateConfig({ favoriteServers: [...servers, { name: name.trim() || address.trim(), address: address.trim() }] });
    setName("");
    setAddress("");
    setAdding(false);
  };
  const remove = (address: string) => updateConfig({ favoriteServers: servers.filter((s) => s.address !== address) });

  return (
    <section className="quickplay">
      <h3 className="section-title"><Icon name="server" size={16} /> {t("play.quickplay")}</h3>
      <div className="quickplay__list">
        {servers.map((s) => (
          <div key={s.address} className="server-card">
            <img src={`https://mc-heads.net/head/${encodeURIComponent(s.name)}/40`} alt="" width={40} height={40} loading="lazy" />
            <div className="server-card__info"><strong>{s.name}</strong><small>{s.address}</small></div>
            <button className="icon-btn icon-btn--red" title={t("play.launch")} onClick={() => launch(s.address)}>
              <Icon name="play" size={16} />
            </button>
            <button className="icon-btn server-card__remove" onClick={() => remove(s.address)} aria-label="Quitar">
              <Icon name="close" size={14} />
            </button>
          </div>
        ))}
        {adding ? (
          <form className="server-card server-card--form" onSubmit={(e) => { e.preventDefault(); add(); }}>
            <input placeholder={t("play.serverName")} value={name} onChange={(e) => setName(e.target.value)} />
            <input placeholder={t("play.serverAddress")} value={address} onChange={(e) => setAddress(e.target.value)} autoFocus />
            <button className="icon-btn icon-btn--red"><Icon name="plus" size={16} /></button>
          </form>
        ) : (
          <button className="server-card server-card--add" onClick={() => setAdding(true)}>
            <Icon name="plus" /> {t("play.addServer")}
          </button>
        )}
      </div>
    </section>
  );
}
