// Cliente del ecosistema social de KIRMC (amigos + chat) sobre WebSocket.
// Sin VITE_SOCIAL_URL funciona en modo demo para poder diseñar la UI sin backend.
import { create } from "zustand";

export type Presence = "online" | "playing" | "away" | "offline";
export interface Friend { uuid: string; name: string; presence: Presence; activity?: string }
export interface Message { id: string; with: string; body: string; ts: number; mine: boolean }

type ServerPacket =
  | { t: "friends"; friends: Friend[] }
  | { t: "presence"; uuid: string; presence: Presence; activity?: string }
  | { t: "msg"; id: string; from: string; body: string; ts: number };

const SOCIAL_URL = import.meta.env.VITE_SOCIAL_URL as string | undefined;

const DEMO_FRIENDS: Friend[] = [
  { uuid: "demo-1", name: "Yuki_Neko", presence: "playing", activity: "Survival 1.21" },
  { uuid: "demo-2", name: "RamenKing", presence: "online" },
  { uuid: "demo-3", name: "Sakura_mc", presence: "playing", activity: "mc.hypixel.net" },
  { uuid: "demo-4", name: "Kitsune77", presence: "away" },
  { uuid: "demo-5", name: "Oni_Dev", presence: "offline" },
];
const DEMO_REPLIES = ["¡Ramen time! 🍜", "¿Jugamos una partida?", "nya~ ya me conecto", "¡Ese build está genial!"];

interface SocialState {
  demo: boolean;
  connected: boolean;
  friends: Friend[];
  messages: Record<string, Message[]>;
  activeChat: string | null;
  connect: (me: { uuid: string; username: string }) => void;
  openChat: (uuid: string) => void;
  send: (to: string, body: string) => void;
}

let ws: WebSocket | null = null;

export const useSocial = create<SocialState>((set, get) => {
  const push = (withUuid: string, msg: Message) =>
    set((s) => ({ messages: { ...s.messages, [withUuid]: [...(s.messages[withUuid] ?? []), msg] } }));

  return {
    demo: !SOCIAL_URL,
    connected: false,
    friends: SOCIAL_URL ? [] : DEMO_FRIENDS,
    messages: {},
    activeChat: null,

    connect(me) {
      if (!SOCIAL_URL || ws) return;
      ws = new WebSocket(SOCIAL_URL);
      // El servidor debe verificar identidad (p. ej. join/hasJoined de Mojang para Premium)
      ws.onopen = () => {
        set({ connected: true });
        ws?.send(JSON.stringify({ t: "hello", uuid: me.uuid, name: me.username }));
      };
      ws.onmessage = (ev) => {
        const p = JSON.parse(ev.data) as ServerPacket;
        if (p.t === "friends") set({ friends: p.friends });
        else if (p.t === "presence")
          set((s) => ({ friends: s.friends.map((f) => (f.uuid === p.uuid ? { ...f, presence: p.presence, activity: p.activity } : f)) }));
        else if (p.t === "msg") push(p.from, { id: p.id, with: p.from, body: p.body, ts: p.ts, mine: false });
      };
      ws.onclose = () => {
        ws = null;
        set({ connected: false });
        setTimeout(() => get().connect(me), 5000); // reconexión
      };
    },

    openChat: (uuid) => set({ activeChat: uuid }),

    send(to, body) {
      const text = body.trim();
      if (!text) return;
      push(to, { id: crypto.randomUUID(), with: to, body: text, ts: Date.now(), mine: true });
      if (ws?.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ t: "msg", to, body: text }));
      } else if (get().demo) {
        const reply = DEMO_REPLIES[Math.floor(Math.random() * DEMO_REPLIES.length)];
        setTimeout(() => push(to, { id: crypto.randomUUID(), with: to, body: reply, ts: Date.now(), mine: false }), 1100);
      }
    },
  };
});
