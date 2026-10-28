import { useT } from "../i18n";
import Mascot from "./Mascot";

export default function SplashScreen({ visible }: { visible: boolean }) {
  const t = useT();
  return (
    <div className={`splash ${visible ? "" : "splash--hidden"}`} aria-hidden={!visible}>
      <Mascot mood="loading" size={200} />
      <h1 className="splash__title">KIRMC</h1>
      <div className="splash__bar"><span /></div>
      <p className="splash__hint">{t("splash.loading")}</p>
    </div>
  );
}
