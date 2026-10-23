import { useEffect, useState } from "react";
import { api } from "../api";
import logoUrl from "../assets/logo.svg";
import { t } from "../i18n";

export function AboutDialog({ onClose }: { onClose: () => void }) {
  const [error, setError] = useState<string | null>(null);

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
        {error && <p className="cleanup-warn">{error}</p>}
        <div className="modal-foot">
          <button className="btn with-icon about-source" onClick={() => api.openLink("source").catch((e) => setError(String(e)))}>
            <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
              <path
                d="M9 2.5h4.5V7M13.5 2.5 7.5 8.5M6 3.5H4A1.5 1.5 0 0 0 2.5 5v7A1.5 1.5 0 0 0 4 13.5h7a1.5 1.5 0 0 0 1.5-1.5v-2"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
            {t.sourceCode}
          </button>
          <button className="btn primary" onClick={onClose} autoFocus>
            {t.ok}
          </button>
        </div>
      </div>
    </div>
  );
}
