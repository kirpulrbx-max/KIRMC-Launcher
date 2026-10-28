import { useEffect, useState } from "react";
import Icon from "../components/Icon";
import SkinFront, { CapeThumb } from "../components/skins/SkinFront";
import SkinViewer3D from "../components/skins/SkinViewer3D";
import { useT } from "../i18n";
import {
  api,
  isTauri,
  pngUrl,
  skinUrl,
  type CapeEntry,
  type GallerySkin,
  type PlayerTextures,
  type SavedSkin,
  type SkinVariant,
} from "../lib/api";
import { useApp } from "../store/app";

type Tab = "skin" | "gallery" | "capes" | "library" | "search";

/** Skin en edición: bytes (para subir/guardar) + URL de previsualización */
interface Draft { bytes: Uint8Array; url: string; name: string }

const PROVIDER_LABEL: Record<string, string> = {
  minecraft: "Minecraft",
  optifine: "OptiFine",
  minecraftcapes: "MinecraftCapes",
  labymod: "LabyMod",
  "5zig": "5zig",
  tlauncher: "TLauncher",
  skinmc: "SkinMC",
};
const CAPE_SUGGESTIONS = ["Notch", "jeb_", "Dinnerbone", "Technoblade", "Dream", "Grumm"];

export default function Characters() {
  const t = useT();
  const account = useApp((s) => s.activeAccount());
  const premium = account?.kind === "microsoft";
  const [tab, setTab] = useState<Tab>("skin");
  const [draft, setDraft] = useState<Draft | null>(null);
  const [variant, setVariant] = useState<SkinVariant>(account?.skinVariant === "slim" ? "slim" : "classic");
  const [capePreview, setCapePreview] = useState<string | null>(null);
  const [walk, setWalk] = useState(false);
  const [rotate, setRotate] = useState(true);

  // Capa equipada: se descarga vía backend (textures.minecraft.net no siempre envía CORS para WebGL)
  const activeCape = account?.capes.find((c) => c.active);
  const [activeCapeUrl, setActiveCapeUrl] = useState<string | null>(null);
  useEffect(() => {
    // No-premium: la capa la sirve el servidor local (con CORS), se usa tal cual
    if (account?.kind === "offline") return setActiveCapeUrl(account.capeUrl ?? null);
    if (!activeCape || !isTauri) return setActiveCapeUrl(null);
    let url: string | null = null;
    api.fetchTexture(activeCape.url).then((b) => setActiveCapeUrl((url = pngUrl(b)))).catch(() => setActiveCapeUrl(null));
    return () => { if (url) URL.revokeObjectURL(url); };
  }, [activeCape?.url, account?.capeUrl]); // eslint-disable-line react-hooks/exhaustive-deps

  const takeDraft = (bytes: Uint8Array, name: string, v?: SkinVariant) => {
    setDraft((old) => {
      if (old) URL.revokeObjectURL(old.url);
      return { bytes, url: pngUrl(bytes), name };
    });
    if (v) setVariant(v);
    setTab("skin");
  };

  return (
    <div className="page characters">
      <section className="characters__stage card">
        <SkinViewer3D
          skin={draft?.url ?? skinUrl(account)}
          cape={capePreview ?? activeCapeUrl}
          slim={variant === "slim"}
          walk={walk}
          rotate={rotate}
        />
        <div className="row center wrap">
          <div className="segmented">
            <button className={variant === "classic" ? "is-active" : ""} onClick={() => setVariant("classic")}>{t("chars.classic")}</button>
            <button className={variant === "slim" ? "is-active" : ""} onClick={() => setVariant("slim")}>{t("chars.slim")}</button>
          </div>
          <label className="check"><input type="checkbox" checked={walk} onChange={(e) => setWalk(e.target.checked)} /> {t("chars.walk")}</label>
          <label className="check"><input type="checkbox" checked={rotate} onChange={(e) => setRotate(e.target.checked)} /> {t("chars.rotate")}</label>
        </div>
      </section>

      <section className="characters__panel">
        <header className="page__head">
          <h1 className="display">{account?.username ?? t("chars.title")}</h1>
          <div className="tabs">
            {(["skin", "gallery", "capes", "library", "search"] as Tab[]).map((x) => (
              <button key={x} className={tab === x ? "is-active" : ""} onClick={() => setTab(x)}>{t(`chars.tab.${x}`)}</button>
            ))}
          </div>
        </header>
        <div key={tab} className="tab-enter">
          {tab === "skin" && <SkinTab draft={draft} variant={variant} premium={premium} onDraft={takeDraft} onGallery={() => setTab("gallery")} />}
          {tab === "gallery" && <GalleryTab onUse={takeDraft} />}
          {tab === "capes" && <CapesTab premium={premium} onPreview={setCapePreview} />}
          {tab === "library" && <LibraryTab onUse={takeDraft} onPreviewCape={setCapePreview} />}
          {tab === "search" && <SearchTab onUse={takeDraft} onPreviewCape={setCapePreview} />}
        </div>
      </section>
    </div>
  );
}

function SkinTab({ draft, variant, premium, onDraft, onGallery }: {
  draft: Draft | null;
  variant: SkinVariant;
  premium: boolean;
  onDraft: (bytes: Uint8Array, name: string) => void;
  onGallery: () => void;
}) {
  const t = useT();
  const account = useApp((s) => s.activeAccount());
  const refreshAccounts = useApp((s) => s.refreshAccounts);
  const setError = useApp((s) => s.setError);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);

  const fromFile = async (file?: File) => {
    if (!file) return;
    onDraft(new Uint8Array(await file.arrayBuffer()), file.name.replace(/\.png$/i, ""));
  };
  const apply = async () => {
    if (!draft || !account) return;
    setBusy(true);
    try {
      if (premium) await api.uploadSkin(account.uuid, draft.bytes, variant);
      else await api.setOfflineSkin(account.uuid, draft.bytes, variant);
      await refreshAccounts();
      setError(t(premium ? "chars.applied" : "chars.appliedOffline"));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const save = async () => {
    if (!draft) return;
    try {
      await api.saveSkin(name || draft.name, draft.bytes, variant);
      setError(t("chars.saved"));
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="stack">
      {!premium && <p className="banner">{t("chars.offlineNote")}</p>}
      <div className="card stack">
        <h3 className="card__title"><Icon name="image" size={16} /> {t("chars.tab.skin")}</h3>
        <div className="row wrap">
          <label className="btn btn-ghost">
            <Icon name="folder" size={16} /> {t("chars.upload")}
            <input type="file" accept="image/png" hidden onChange={(e) => fromFile(e.target.files?.[0])} />
          </label>
          <button className="btn btn-primary" onClick={onGallery}><Icon name="sparkle" size={16} /> {t("chars.tab.gallery")}</button>
        </div>
        {draft && (
          <div className="draft">
            <SkinFront src={draft.url} height={80} />
            <div className="stack grow">
              <input value={name} onChange={(e) => setName(e.target.value)} placeholder={draft.name || t("chars.skinName")} />
              <div className="row wrap">
                <button className="btn btn-primary" disabled={busy || !account} onClick={apply}><Icon name="check" size={16} /> {t("chars.apply")}</button>
                <button className="btn btn-ghost" onClick={save}><Icon name="save" size={16} /> {t("chars.save")}</button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

/** Galería pública de MineSkin con paginación infinita */
function GalleryTab({ onUse }: { onUse: (bytes: Uint8Array, name: string) => void }) {
  const t = useT();
  const setError = useApp((s) => s.setError);
  const [skins, setSkins] = useState<GallerySkin[]>([]);
  const [next, setNext] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);

  const load = async (after: string | null) => {
    setLoading(true);
    try {
      const page = await api.mineskinGallery(after);
      setSkins((prev) => (after ? [...prev, ...page.skins] : page.skins));
      setNext(page.next ?? null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };
  useEffect(() => { load(null); }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const use = async (s: GallerySkin, save: boolean) => {
    setBusy(s.id);
    try {
      const bytes = await api.fetchTexture(s.textureUrl);
      const name = s.name ?? `MineSkin ${s.id.slice(0, 6)}`;
      if (save) {
        await api.saveSkin(name, bytes, "classic");
        setError(t("chars.saved"));
      } else onUse(bytes, name);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="stack">
      <p className="small muted on-red">{t("chars.galleryNote")}</p>
      <div className="library-grid">
        {loading && skins.length === 0 && Array.from({ length: 12 }, (_, i) => <div key={i} className="skeleton skeleton--card" />)}
        {skins.map((s, i) => (
          <div key={s.id} className="library-item" style={{ animationDelay: `${(i % 24) * 25}ms` }}>
            <SkinFront src={s.textureUrl} height={88} />
            <strong>{s.name ?? s.id.slice(0, 8)}</strong>
            <div className="row">
              <button className="btn btn-primary btn-sm" disabled={busy === s.id} onClick={() => use(s, false)}>
                {busy === s.id ? <span className="spinner" /> : null} {t("chars.use")}
              </button>
              <button className="icon-btn" title={t("chars.save")} disabled={busy === s.id} onClick={() => use(s, true)}>
                <Icon name="download" size={15} />
              </button>
            </div>
          </div>
        ))}
      </div>
      {next && (
        <button className="btn btn-ghost load-more" disabled={loading} onClick={() => load(next)}>
          {loading ? <span className="spinner" /> : null} {t("mr.more")}
        </button>
      )}
    </div>
  );
}

function CapesTab({ premium, onPreview }: { premium: boolean; onPreview: (url: string | null) => void }) {
  const t = useT();
  const account = useApp((s) => s.activeAccount());
  const refreshAccounts = useApp((s) => s.refreshAccounts);
  const setError = useApp((s) => s.setError);
  const [busy, setBusy] = useState<string | null>(null);
  useEffect(() => () => onPreview(null), [onPreview]); // al salir, quitar la vista previa

  const equip = async (id: string | null) => {
    if (!account) return;
    setBusy(id ?? "none");
    try {
      await api.setCape(account.uuid, id);
      await refreshAccounts();
      onPreview(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };
  const preview = async (url: string) => {
    try { onPreview(pngUrl(await api.fetchTexture(url))); } catch { onPreview(null); }
  };

  const capes = account?.capes ?? [];
  return (
    <div className="stack">
      {premium ? (
        <div className="card stack">
          <h3 className="card__title"><Icon name="cape" size={16} /> {t("chars.myCapes")}</h3>
          {capes.length === 0 && <p className="small muted">{t("chars.noCapes")}</p>}
          <div className="cape-grid">
            <button className={`cape-card ${!capes.some((c) => c.active) ? "is-active" : ""}`} onClick={() => equip(null)} onMouseEnter={() => onPreview(null)}>
              <span className="cape-thumb cape-thumb--none"><Icon name="close" /></span>
              <strong>{t("chars.noCape")}</strong>
            </button>
            {capes.map((c) => (
              <button key={c.id} className={`cape-card ${c.active ? "is-active" : ""}`} onMouseEnter={() => preview(c.url)} onClick={() => equip(c.id)} disabled={busy !== null}>
                <CapeThumb url={c.url} />
                <strong>{c.alias}</strong>
                <small>{busy === c.id ? <span className="spinner" /> : c.active ? t("chars.equipped") : t("chars.equip")}</small>
              </button>
            ))}
          </div>
        </div>
      ) : (
        <div className="card stack">
          <h3 className="card__title"><Icon name="cape" size={16} /> {t("chars.myCapes")}</h3>
          <p className="small muted">{t("chars.capesOffline")}</p>
          <div className="row">
            {account?.capeUrl ? <CapeThumb url={account.capeUrl} /> : <span className="cape-thumb cape-thumb--none"><Icon name="close" /></span>}
            <button className="btn btn-ghost btn-sm" disabled={!account?.capeUrl} onClick={async () => {
              if (!account) return;
              await api.setOfflineCape(account.uuid, null);
              await refreshAccounts();
              onPreview(null);
            }}>
              {t("chars.removeCape")}
            </button>
          </div>
        </div>
      )}
      <CapeExplorer onPreview={onPreview} />
    </div>
  );
}

/** No-premium: hace que una capa (PNG) sea la que el servidor local sirve al juego */
function useOfflineCape() {
  const t = useT();
  const account = useApp((s) => s.activeAccount());
  const refreshAccounts = useApp((s) => s.refreshAccounts);
  const setError = useApp((s) => s.setError);
  if (account?.kind !== "offline") return null;
  return async (bytes: Uint8Array) => {
    try {
      await api.setOfflineCape(account.uuid, bytes);
      await refreshAccounts();
      setError(t("chars.capeApplied"));
    } catch (e) {
      setError(String(e));
    }
  };
}

/** Explorador de capas de cualquier jugador vía capes.dev (7 servicios), con descarga directa */
function CapeExplorer({ onPreview }: { onPreview: (url: string | null) => void }) {
  const t = useT();
  const setError = useApp((s) => s.setError);
  const [name, setName] = useState("");
  const [owner, setOwner] = useState("");
  const [capes, setCapes] = useState<CapeEntry[] | null>(null);
  const [busy, setBusy] = useState(false);
  const useCape = useOfflineCape();

  const search = async (who: string) => {
    if (who.trim().length < 3) return;
    setName(who);
    setBusy(true);
    try {
      setCapes(await api.lookupCapes(who));
      setOwner(who);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const save = async (c: CapeEntry) => {
    try {
      await api.saveSkin(`${owner} · ${PROVIDER_LABEL[c.provider] ?? c.provider}`, await api.fetchTexture(c.imageUrl), "classic", "cape");
      setError(t("chars.saved"));
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="card stack">
      <h3 className="card__title"><Icon name="search" size={16} /> {t("chars.capeSearch")}</h3>
      <form className="row" onSubmit={(e) => { e.preventDefault(); search(name); }}>
        <label className="search-field grow">
          <Icon name="search" size={16} />
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder={t("chars.playerName")} maxLength={16} />
        </label>
        <button className="btn btn-primary" disabled={name.trim().length < 3 || busy}>{busy ? <span className="spinner" /> : null} {t("chars.lookup")}</button>
      </form>
      <div className="chips">
        {CAPE_SUGGESTIONS.map((s) => <button key={s} className="chip chip--btn" onClick={() => search(s)}>{s}</button>)}
      </div>
      <p className="small muted">{t("chars.capeProviders")}</p>
      {capes?.length === 0 && <p className="small">{t("chars.capeNoResults")}</p>}
      {capes && capes.length > 0 && (
        <div className="cape-grid">
          {capes.map((c) => (
            <div key={c.provider} className="cape-card" onMouseEnter={async () => {
              try { onPreview(pngUrl(await api.fetchTexture(c.imageUrl))); } catch { /* sin vista previa */ }
            }}>
              <CapeThumb url={c.imageUrl} />
              <strong>{PROVIDER_LABEL[c.provider] ?? c.provider}</strong>
              {useCape && (
                <button className="btn btn-primary btn-sm" onClick={async () => useCape(await api.fetchTexture(c.imageUrl))}>
                  {t("chars.useCape")}
                </button>
              )}
              <button className="btn btn-ghost btn-sm" onClick={() => save(c)}><Icon name="download" size={13} /> {t("chars.download")}</button>
            </div>
          ))}
        </div>
      )}
      <p className="small muted">{t("chars.capeApplyNote")}</p>
    </div>
  );
}

function LibraryTab({ onUse, onPreviewCape }: {
  onUse: (bytes: Uint8Array, name: string, v?: SkinVariant) => void;
  onPreviewCape: (url: string | null) => void;
}) {
  const t = useT();
  const [items, setItems] = useState<SavedSkin[] | null>(null);
  const load = () => (isTauri ? api.listSkins().then(setItems) : Promise.resolve(setItems([])));
  useEffect(() => { load(); }, []);
  useEffect(() => () => onPreviewCape(null), [onPreviewCape]);

  if (items?.length === 0) return <div className="empty card">{t("chars.libraryEmpty")}</div>;
  const skins = items?.filter((s) => s.kind !== "cape") ?? [];
  const capes = items?.filter((s) => s.kind === "cape") ?? [];
  return (
    <div className="stack">
      {skins.length > 0 && <h3 className="section-title">{t("chars.mySkins")}</h3>}
      <div className="library-grid stagger">
        {skins.map((s) => <LibraryItem key={s.id} item={s} onUse={onUse} onDeleted={load} />)}
      </div>
      {capes.length > 0 && <h3 className="section-title">{t("chars.myCapes")}</h3>}
      <div className="library-grid stagger">
        {capes.map((s) => <LibraryItem key={s.id} item={s} onPreviewCape={onPreviewCape} onDeleted={load} />)}
      </div>
    </div>
  );
}

function LibraryItem({ item, onUse, onPreviewCape, onDeleted }: {
  item: SavedSkin;
  onUse?: (bytes: Uint8Array, name: string, v?: SkinVariant) => void;
  onPreviewCape?: (url: string | null) => void;
  onDeleted: () => void;
}) {
  const t = useT();
  const [data, setData] = useState<{ bytes: Uint8Array; url: string } | null>(null);
  useEffect(() => {
    let url = "";
    api.readSkin(item.id).then((bytes) => setData({ bytes, url: (url = pngUrl(bytes)) })).catch(() => {});
    return () => { if (url) URL.revokeObjectURL(url); };
  }, [item.id]);
  const isCape = item.kind === "cape";
  const useCape = useOfflineCape();

  return (
    <div className="library-item">
      {!data ? <div className="skeleton skeleton--skin" /> : isCape ? <CapeThumb url={data.url} /> : <SkinFront src={data.url} height={88} />}
      <strong>{item.name}</strong>
      {!isCape && <small className="muted">{item.variant}</small>}
      <div className="row">
        <button
          className="btn btn-primary btn-sm"
          disabled={!data}
          onClick={() => data && (isCape ? onPreviewCape?.(data.url) : onUse?.(data.bytes, item.name, item.variant))}
        >
          {isCape ? t("chars.preview") : t("chars.use")}
        </button>
        {isCape && useCape && (
          <button className="btn btn-ghost btn-sm" disabled={!data} onClick={() => data && useCape(data.bytes)}>{t("chars.useCape")}</button>
        )}
        <button className="icon-btn" aria-label={t("chars.delete")} onClick={async () => { await api.deleteSkin(item.id); onDeleted(); }}>
          <Icon name="trash" size={15} />
        </button>
      </div>
    </div>
  );
}

function SearchTab({ onUse, onPreviewCape }: {
  onUse: (bytes: Uint8Array, name: string, v?: SkinVariant) => void;
  onPreviewCape: (url: string | null) => void;
}) {
  const t = useT();
  const setError = useApp((s) => s.setError);
  const [source, setSource] = useState<"mojang" | "elyby">("mojang");
  const [name, setName] = useState("");
  const [player, setPlayer] = useState<PlayerTextures | null>(null);
  const [skin, setSkin] = useState<{ bytes: Uint8Array; url: string } | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => () => onPreviewCape(null), [onPreviewCape]);

  const lookup = async () => {
    setBusy(true);
    try {
      if (source === "elyby") {
        const bytes = await api.elybySkin(name);
        setPlayer({ uuid: "", name, slim: false, skinUrl: null, capeUrl: null });
        setSkin({ bytes, url: pngUrl(bytes) });
        onPreviewCape(null);
      } else {
        const p = await api.lookupPlayer(name);
        setPlayer(p);
        if (p.skinUrl) {
          const bytes = await api.fetchTexture(p.skinUrl);
          setSkin({ bytes, url: pngUrl(bytes) });
        } else setSkin(null);
        onPreviewCape(p.capeUrl ? pngUrl(await api.fetchTexture(p.capeUrl)) : null);
      }
    } catch (e) {
      setPlayer(null);
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="stack">
      <form className="card stack" onSubmit={(e) => { e.preventDefault(); lookup(); }}>
        <div className="row wrap">
          <span className="field-label">{t("chars.source")}</span>
          <div className="segmented">
            <button type="button" className={source === "mojang" ? "is-active" : ""} onClick={() => setSource("mojang")}>Mojang (Premium)</button>
            <button type="button" className={source === "elyby" ? "is-active" : ""} onClick={() => setSource("elyby")}>Ely.by</button>
          </div>
        </div>
        <div className="row">
          <label className="search-field grow">
            <Icon name="search" size={16} />
            <input value={name} onChange={(e) => setName(e.target.value)} placeholder={t("chars.playerName")} maxLength={16} />
          </label>
          <button className="btn btn-primary" disabled={name.trim().length < 3 || busy}>{busy ? <span className="spinner" /> : null} {t("chars.lookup")}</button>
        </div>
      </form>
      {player && (
        <div className="card player-result">
          {skin ? <SkinFront src={skin.url} height={110} /> : <div className="skeleton skeleton--skin" />}
          <div className="stack grow">
            <h3 className="display">{player.name}</h3>
            <div className="chips">
              <span className="chip">{source === "elyby" ? "Ely.by" : player.slim ? "Slim" : "Classic"}</span>
              {player.capeUrl && <span className="chip chip--yellow"><Icon name="cape" size={13} /> {t("chars.playerCape", { name: player.name })}</span>}
            </div>
            <div className="row wrap">
              <button className="btn btn-primary" disabled={!skin} onClick={() => skin && onUse(skin.bytes, player.name, player.slim ? "slim" : "classic")}>
                {t("chars.use")}
              </button>
              <button className="btn btn-ghost" disabled={!skin} onClick={async () => {
                if (!skin) return;
                await api.saveSkin(player.name, skin.bytes, player.slim ? "slim" : "classic");
                setError(t("chars.saved"));
              }}>
                <Icon name="download" size={16} /> {t("chars.save")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
