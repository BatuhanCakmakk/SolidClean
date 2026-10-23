import { useEffect, useState } from "react";
import { api, type AppInfo } from "../api";
import { t } from "../i18n";

interface Props {
  /** Haritada seçili uygulama klasörü. */
  path: string;
  label: string;
  onClose: () => void;
}

/** Uygulama klasörünü silmez: sahibini gösterir ve Windows'un kaldırma sayfasına yönlendirir. */
export function UninstallDialog({ path, label, onClose }: Props) {
  const [apps, setApps] = useState<AppInfo[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    api.appsForFolder(path).then(
      (found) => alive && setApps(found),
      () => alive && setApps([]),
    );
    return () => {
      alive = false;
    };
  }, [path]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const open = async () => {
    try {
      await api.openInstalledApps();
      onClose();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal uninstall" role="dialog" aria-modal="true" onClick={(e) => e.stopPropagation()}>
        <h2>{t.uninstallTitle}</h2>
        <div className="uninstall-target">
          <strong>{label}</strong>
          <span className="uninstall-path">{path}</span>
        </div>
        {apps === null ? (
          <p className="muted">{t.uninstallLooking}</p>
        ) : apps.length === 0 ? (
          <p>{t.uninstallUnknown}</p>
        ) : apps.length === 1 ? (
          <p>
            {t.uninstallOwner(apps[0].name)}
            {apps[0].publisher && <span className="muted"> · {apps[0].publisher}</span>}
          </p>
        ) : (
          <div>
            <p>{t.uninstallSeveral}</p>
            <ul className="uninstall-apps">
              {apps.map((a) => (
                <li key={a.name}>
                  {a.name}
                  {a.publisher && <span className="muted"> · {a.publisher}</span>}
                </li>
              ))}
            </ul>
          </div>
        )}
        <p className="note">{t.uninstallWhy}</p>
        <p className="muted uninstall-steps">{t.uninstallSteps}</p>
        {error && <p className="cleanup-warn">{error}</p>}
        <div className="modal-foot">
          <button className="btn" onClick={onClose}>
            {t.cancel}
          </button>
          <button className="btn primary" onClick={open}>
            {t.openInstalledApps}
          </button>
        </div>
      </div>
    </div>
  );
}
