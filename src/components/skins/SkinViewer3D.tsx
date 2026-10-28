import { useEffect, useRef } from "react";
import type { SkinViewer } from "skinview3d";

/**
 * Visor 3D (three.js vía skinview3d). Se importa bajo demanda y se destruye al desmontar,
 * así la GPU y la RAM vuelven a reposo al salir de PERSONAJES.
 */
export default function SkinViewer3D({ skin, cape, slim, walk, rotate }: {
  skin: string;
  cape?: string | null;
  slim: boolean;
  walk: boolean;
  rotate: boolean;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const viewerRef = useRef<SkinViewer | null>(null);
  const modRef = useRef<typeof import("skinview3d") | null>(null);
  const latest = useRef({ skin, cape, slim, walk, rotate });
  latest.current = { skin, cape, slim, walk, rotate };

  const apply = () => {
    const v = viewerRef.current;
    const mod = modRef.current;
    if (!v || !mod) return;
    const s = latest.current;
    v.loadSkin(s.skin, { model: s.slim ? "slim" : "default" }).catch(() => {});
    if (s.cape) v.loadCape(s.cape).catch(() => {});
    else v.resetCape();
    v.autoRotate = s.rotate;
    v.animation = s.walk ? new mod.WalkingAnimation() : new mod.IdleAnimation();
  };

  useEffect(() => {
    let disposed = false;
    import("skinview3d").then((mod) => {
      if (disposed || !canvasRef.current) return;
      modRef.current = mod;
      const v = new mod.SkinViewer({ canvas: canvasRef.current, width: 320, height: 430 });
      v.zoom = 0.82;
      v.autoRotateSpeed = 0.6;
      viewerRef.current = v;
      apply();
    });
    return () => {
      disposed = true;
      viewerRef.current?.dispose();
      viewerRef.current = null;
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(apply, [skin, cape, slim, walk, rotate]); // eslint-disable-line react-hooks/exhaustive-deps

  return <canvas ref={canvasRef} className="skin-canvas" />;
}
