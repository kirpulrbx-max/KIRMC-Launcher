import { useEffect, useRef, useState } from "react";
import Icon from "../components/Icon";
import Mascot from "../components/Mascot";
import { useT } from "../i18n";
import { useSocial } from "../lib/social";
import { useApp } from "../store/app";

export default function Chat() {
  const t = useT();
  const { demo, friends, messages, activeChat, openChat, send, connect } = useSocial();
  const account = useApp((s) => s.activeAccount());
  const [text, setText] = useState("");
  const endRef = useRef<HTMLDivElement>(null);
  const friend = friends.find((f) => f.uuid === activeChat);
  const thread = activeChat ? messages[activeChat] ?? [] : [];

  useEffect(() => {
    if (account) connect({ uuid: account.uuid, username: account.username });
  }, [account, connect]);
  useEffect(() => endRef.current?.scrollIntoView({ behavior: "smooth" }), [thread.length]);

  return (
    <div className="page chat">
      <aside className="chat__friends card">
        <h3 className="card__title">{t("chat.title")}</h3>
        {friends.map((f) => (
          <button key={f.uuid} className={`friend ${activeChat === f.uuid ? "is-active" : ""}`} onClick={() => openChat(f.uuid)}>
            <img src={`https://mc-heads.net/avatar/${f.name}/32`} alt="" width={32} height={32} loading="lazy" />
            <span className="friend__name">{f.name}<small>{f.activity ?? f.presence}</small></span>
            <span className={`dot dot--${f.presence}`} />
          </button>
        ))}
      </aside>

      <section className="chat__thread card">
        {demo && <p className="banner">{t("chat.demo")}</p>}
        {friend ? (
          <>
            <header className="chat__head"><strong>{friend.name}</strong><span className={`dot dot--${friend.presence}`} /></header>
            <div className="chat__messages">
              {thread.map((m) => (
                <div key={m.id} className={`bubble ${m.mine ? "bubble--mine" : ""}`}>
                  {m.body}
                  <time>{new Date(m.ts).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</time>
                </div>
              ))}
              <div ref={endRef} />
            </div>
            <form className="chat__input" onSubmit={(e) => { e.preventDefault(); send(friend.uuid, text); setText(""); }}>
              <input value={text} onChange={(e) => setText(e.target.value)} placeholder={t("chat.placeholder")} maxLength={500} />
              <button className="btn btn-primary" aria-label={t("chat.send")}><Icon name="send" size={18} /></button>
            </form>
          </>
        ) : (
          <div className="empty"><Mascot size={120} /><p>{t("chat.pick")}</p></div>
        )}
      </section>
    </div>
  );
}
