import { useEffect } from "react";
import logoUrl from "../assets/logo.svg";
import { t } from "../i18n";

export function AboutDialog({ onClose }: { onClose: () => void }) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal about" role="dialog" aria-modal="true" aria-labelledby="about-title" onClick={(e) => e.stopPropagation()}>
        <div className="about-head">
          <img src={logoUrl} width={48} height={48} alt="" />
          <div>
            <h2 id="about-title">SolidClean</h2>
            <p className="muted">
              {t.tagline} · {t.version(__APP_VERSION__)}
            </p>
          </div>
        </div>
        <p>{t.aboutFree}</p>
        <p className="muted">{t.aboutPrivacy}</p>
        <div className="modal-foot">
          <button className="btn primary" onClick={onClose} autoFocus>
            {t.ok}
          </button>
        </div>
      </div>
    </div>
  );
}
