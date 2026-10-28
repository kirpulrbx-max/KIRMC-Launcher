import type { CSSProperties } from "react";

/**
 * Cabeza de jugador en 3D hecha con CSS (6 caras + capa del sombrero) a partir de la textura de skin.
 * Sin WebGL ni three.js: cuesta prácticamente nada en reposo.
 * En reposo mira de lado y se balancea; al pasar el cursor por el contenedor `.head-host` mira al frente.
 */
type Face = "front" | "back" | "right" | "left" | "top" | "bottom";

// Origen (x, y) de cada cara en la textura 64×64: cabeza y capa exterior (+32 en x)
const FACES: Record<Face, [number, number]> = {
  front: [8, 8],
  back: [24, 8],
  right: [0, 8], // lado derecho del jugador (a la izquierda del espectador)
  left: [16, 8],
  top: [8, 0],
  bottom: [16, 0],
};

function Cube({ skin, size, offsetX, className }: { skin: string; size: number; offsetX: number; className: string }) {
  const k = size / 8; // píxeles de pantalla por píxel de textura
  return (
    <div className={`head__cube ${className}`} style={{ "--s": `${size}px` } as CSSProperties}>
      {(Object.keys(FACES) as Face[]).map((f) => {
        const [x, y] = FACES[f];
        return (
          <span
            key={f}
            className={`head__face head__face--${f}`}
            style={{
              backgroundImage: `url(${skin})`,
              backgroundSize: `${64 * k}px auto`,
              backgroundPosition: `-${(x + offsetX) * k}px -${y * k}px`,
            }}
          />
        );
      })}
    </div>
  );
}

export default function PlayerHead({ skin, size = 30 }: { skin: string; size?: number }) {
  return (
    <div className="head" style={{ width: size, height: size }} aria-hidden="true">
      <div className="head__rig">
        <Cube skin={skin} size={size} offsetX={0} className="head__base" />
        <Cube skin={skin} size={size * 1.125} offsetX={32} className="head__hat" />
      </div>
    </div>
  );
}
