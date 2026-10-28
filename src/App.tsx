import { lazy, Suspense, useEffect, type ComponentType, type LazyExoticComponent } from "react";
import ErrorBoundary from "./components/ErrorBoundary";
import Sidebar from "./components/Sidebar";
import SplashScreen from "./components/SplashScreen";
import Titlebar from "./components/Titlebar";
import { useApp, type Page } from "./store/app";

// Code-splitting por sección: solo la página activa está montada y en memoria
const LOADERS: Record<Page, () => Promise<{ default: ComponentType }>> = {
  play: () => import("./pages/Play"),
  instances: () => import("./pages/Instances"),
  chat: () => import("./pages/Chat"),
  news: () => import("./pages/News"),
  characters: () => import("./pages/Characters"),
  settings: () => import("./pages/Settings"),
};
const PAGES = Object.fromEntries(
  Object.entries(LOADERS).map(([k, load]) => [k, lazy(load)]),
) as Record<Page, LazyExoticComponent<ComponentType>>;

// Precarga en reposo: los chunks de página pesan pocos KB (three.js sigue diferido)
const prefetchPages = () => Object.values(LOADERS).forEach((load) => load());

export default function App() {
  const ready = useApp((s) => s.ready);
  const page = useApp((s) => s.page);
  const init = useApp((s) => s.init);
  const error = useApp((s) => s.error);
  const setError = useApp((s) => s.setError);

  useEffect(() => {
    init();
  }, [init]);
  useEffect(() => {
    if (ready) setTimeout(prefetchPages, 1500);
  }, [ready]);

  const Current = PAGES[page];
  return (
    <>
      <SplashScreen visible={!ready} />
      <div className="shell">
        <Titlebar />
        <Sidebar />
        <div className="main">
          {/* Patrón animado: una sola capa desplazada con transform (GPU), se pausa si se desactivan animaciones */}
          <div className="bg-pattern" aria-hidden="true" />
          <div className="bg-glow" aria-hidden="true" />
          <main className="content">
            {ready && (
              <ErrorBoundary key={page}>
                <Suspense fallback={null}>
                  <div className="page-enter">
                    <Current />
                  </div>
                </Suspense>
              </ErrorBoundary>
            )}
          </main>
        </div>
      </div>
      {error && (
        <div className="toast" role="alert" onClick={() => setError(null)}>
          {error}
        </div>
      )}
    </>
  );
}
