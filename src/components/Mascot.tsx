import { useState } from "react";
import type { Mood } from "../store/app";

/**
 * Mascota oficial: el pequeño demonio rojo con su tazón de ramen.
 * Coloca la ilustración en /public/mascot.png (PNG con transparencia).
 * Su animación refleja el estado del launcher: idle, loading (vapor), playing (salta), error (tiembla).
 */
export default function Mascot({ mood = "idle", size = 180 }: { mood?: Mood; size?: number }) {
  const [missing, setMissing] = useState(false);
  return (
    <div className={`mascot mascot--${mood}`} style={{ width: size, height: size }}>
      <div className="mascot__steam" aria-hidden="true">
        <span /><span /><span />
      </div>
      {missing ? (
        <FallbackOni />
      ) : (
        <img src="/mascot.png" alt="KIRMC" draggable={false} onError={() => setMissing(true)} />
      )}
    </div>
  );
}

// Placeholder vectorial mientras no exista /public/mascot.png
function FallbackOni() {
  return (
    <svg viewBox="0 0 120 120" role="img" aria-label="KIRMC">
      <path d="M38 30 30 8l20 16M82 30l8-22-20 16" fill="#FFCD3C" stroke="#fff" strokeWidth="3" strokeLinejoin="round" />
      <circle cx="60" cy="52" r="30" fill="#E51E1E" stroke="#fff" strokeWidth="4" />
      <path d="M46 50q4-5 8 0M66 50q4-5 8 0" stroke="#3A1A10" strokeWidth="3.5" fill="none" strokeLinecap="round" />
      <ellipse cx="42" cy="60" rx="5" ry="3" fill="#ff8a8a" />
      <ellipse cx="78" cy="60" rx="5" ry="3" fill="#ff8a8a" />
      <path d="M54 63q6 6 12 0" stroke="#3A1A10" strokeWidth="3" fill="none" strokeLinecap="round" />
      <path d="M22 80h76q-4 30-38 30T22 80z" fill="#fff" stroke="#3A1A10" strokeWidth="3" />
      <path d="M26 86h68" stroke="#E51E1E" strokeWidth="4" />
      <path d="M70 52 96 86M76 50l24 34" stroke="#c98a4b" strokeWidth="3" strokeLinecap="round" />
    </svg>
  );
}
