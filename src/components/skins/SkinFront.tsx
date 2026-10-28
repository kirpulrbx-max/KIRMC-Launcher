import { useEffect, useRef, useState } from "react";

/** Render 2D frontal de una skin (miniatura de biblioteca) dibujado en un canvas de 16×32 */
export default function SkinFront({ src, height = 96 }: { src: string; height?: number }) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const img = new Image();
    img.onload = () => {
      const ctx = ref.current?.getContext("2d");
      if (!ctx) return;
      ctx.imageSmoothingEnabled = false;
      ctx.clearRect(0, 0, 16, 32);
      const legacy = img.height === 32; // skins antiguas 64×32: brazo/pierna izquierdos = espejo
      const d = (sx: number, sy: number, w: number, h: number, dx: number, dy: number) =>
        ctx.drawImage(img, sx, sy, w, h, dx, dy, w, h);
      d(8, 8, 8, 8, 4, 0); // cabeza
      d(40, 8, 8, 8, 4, 0); // capa del sombrero
      d(20, 20, 8, 12, 4, 8); // torso
      d(44, 20, 4, 12, 0, 8); // brazo derecho
      legacy ? d(44, 20, 4, 12, 12, 8) : d(36, 52, 4, 12, 12, 8); // brazo izquierdo
      d(4, 20, 4, 12, 4, 20); // pierna derecha
      legacy ? d(4, 20, 4, 12, 8, 20) : d(20, 52, 4, 12, 8, 20); // pierna izquierda
    };
    img.src = src;
  }, [src]);

  return <canvas ref={ref} width={16} height={32} className="skin-front" style={{ height }} />;
}

/**
 * Miniatura frontal de una capa (región 10×16 en (1,1)).
 * Soporta texturas Mojang 64×32 y OptiFine 46×22, incluidas sus versiones HD (múltiplos).
 */
export function CapeThumb({ url }: { url: string }) {
  const [size, setSize] = useState<[number, number] | null>(null);
  useEffect(() => {
    const img = new Image();
    img.onload = () => setSize([img.naturalWidth, img.naturalHeight]);
    img.src = url;
  }, [url]);
  if (!size) return <span className="cape-thumb" />;
  const [w, h] = size;
  const unit = w % 46 === 0 && h % 22 === 0 ? h / 22 : h / 32; // píxeles de textura por "píxel lógico"
  const k = 4 / unit; // 4 px de pantalla por píxel lógico
  return (
    <span
      className="cape-thumb"
      style={{ backgroundImage: `url(${url})`, backgroundSize: `${w * k}px ${h * k}px`, backgroundPosition: "-4px -4px" }}
    />
  );
}
