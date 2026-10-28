import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    // WebView2 (Chromium) en Windows, WebKit en macOS/Linux
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari15",
    minify: "esbuild",
    cssCodeSplit: true,
    chunkSizeWarningLimit: 600, // el chunk de three.js es grande pero diferido
    rollupOptions: {
      output: {
        // three.js (visor 3D) en su propio chunk: solo se carga al abrir PERSONAJES
        manualChunks: { skinview: ["skinview3d"] },
      },
    },
  },
});
