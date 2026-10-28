// Datos de ejemplo para la vista previa en navegador (`npm run dev` + `?demo`).
// Sirven para capturas y para diseñar la UI sin backend; la app de escritorio nunca los usa.
import type { Account, Instance, LauncherConfig, Progress } from "./api";

const DAY = 86_400;
const now = Math.floor(Date.now() / 1000);

export function demoState(config: LauncherConfig) {
  const instances: Instance[] = [
    { id: "demo-rlcraft", name: "RLCraft", mcVersion: "1.12.2", loader: "forge", loaderVersion: "14.23.5.2860", jvmArgs: [], lastPlayed: now - DAY, created: now - 30 * DAY },
    { id: "demo-survival", name: "Survival 26.3", mcVersion: "26.3", loader: "vanilla", jvmArgs: [], lastPlayed: now - 3 * DAY, created: now - 20 * DAY },
    { id: "demo-fabric", name: "Fabric Optimizado", mcVersion: "1.21.1", loader: "fabric", loaderVersion: "0.16.9", jvmArgs: [], created: now - 9 * DAY },
    { id: "demo-quilt", name: "Cozy Quilt", mcVersion: "1.20.1", loader: "quilt", loaderVersion: "0.27.1", jvmArgs: [], created: now - 2 * DAY },
  ];
  const accounts: Account[] = [
    { uuid: "demo", username: "KirPul", kind: "offline", skinUrl: "https://mc-heads.net/skin/MHF_Alex", skinVariant: "slim", capes: [] },
  ];
  const progress: Record<string, Progress> = {
    "demo-fabric": { tag: "demo-fabric", stage: "assets", done: 2140, total: 3870 },
  };
  return {
    config: {
      ...config,
      activeAccount: "demo",
      favoriteServers: [
        { name: "Hypixel", address: "mc.hypixel.net" },
        { name: "KIRMC Survival", address: "play.kirmc.example" },
      ],
    },
    accounts,
    instances,
    selectedId: instances[0].id,
    progress,
  };
}
