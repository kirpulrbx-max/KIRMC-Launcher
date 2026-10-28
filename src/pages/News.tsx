import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import Mascot from "../components/Mascot";
import { useT } from "../i18n";
import { api, isTauri, type NewsItem } from "../lib/api";

export default function News() {
  const t = useT();
  const [items, setItems] = useState<NewsItem[] | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (!isTauri) return setFailed(true);
    api.fetchNews().then(setItems).catch(() => setFailed(true));
  }, []);

  return (
    <div className="page">
      <header className="page__head"><h1 className="display">{t("news.title")}</h1></header>
      {failed && <div className="empty"><Mascot mood="error" size={120} /><p>{t("news.error")}</p></div>}
      {!items && !failed && <div className="empty"><Mascot mood="loading" size={120} /></div>}
      <div className="news-grid">
        {items?.map((n) => (
          <article key={n.id} className="news-card">
            {n.image && <img src={n.image} alt="" loading="lazy" decoding="async" />}
            <div className="news-card__body">
              <div className="row">
                <span className={`badge ${n.tag === "community" ? "badge--yellow" : "badge--red"}`}>{n.tag}</span>
                <small className="muted">{n.date ? new Date(n.date).toLocaleDateString() : ""}</small>
              </div>
              <h3>{n.title}</h3>
              <p className="muted">{n.summary}</p>
              {n.url && <button className="btn btn-ghost btn-sm" onClick={() => openUrl(n.url!)}>{t("news.open")}</button>}
            </div>
          </article>
        ))}
      </div>
    </div>
  );
}
