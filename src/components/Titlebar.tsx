import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauri } from "../lib/api";
import Icon from "./Icon";

// Barra de título propia (ventana sin decoraciones). data-tauri-drag-region permite arrastrar.
export default function Titlebar() {
  const win = isTauri ? getCurrentWindow() : null;
  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="titlebar__brand" data-tauri-drag-region>
        <span className="titlebar__logo" data-tauri-drag-region>KIRMC<small>キルエム</small></span>
        <span className="badge badge--red">LAUNCHER</span>
      </div>
      <div className="titlebar__controls">
        <button aria-label="Minimizar" onClick={() => win?.minimize()}><Icon name="minus" size={16} /></button>
        <button aria-label="Maximizar" onClick={() => win?.toggleMaximize()}><Icon name="square" size={14} /></button>
        <button aria-label="Cerrar" className="is-close" onClick={() => win?.close()}><Icon name="close" size={16} /></button>
      </div>
    </header>
  );
}
