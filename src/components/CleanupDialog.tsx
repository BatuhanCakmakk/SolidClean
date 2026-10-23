import { useEffect, useState } from "react";
import { api, type CleanupMode, type Finding, type Outcome } from "../api";
import { formatBytes } from "../format";
import { lang, t } from "../i18n";

type Stage =
  | { kind: "confirm" }
  | { kind: "running"; started: string[] }
  | { kind: "done"; outcomes: Outcome[] };

interface Props {
  findings: Finding[];
  /** Temizlik yapıldıysa sonuçlarıyla, vazgeçildiyse `null` ile çağrılır. */
  onClose: (outcomes: Outcome[] | null) => void;
}

export function CleanupDialog({ findings, onClose }: Props) {
  const [stage, setStage] = useState<Stage>({ kind: "confirm" });
  const [mode, setMode] = useState<CleanupMode>("recycle");
  const total = findings.reduce((sum, f) => sum + f.bytes, 0);
  const busy = stage.kind === "running";

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && stage.kind === "confirm") onClose(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [stage, onClose]);

  const start = async () => {
    setStage({ kind: "running", started: [] });
    const unlisten = await api.onCleanupProgress((id) =>
      setStage((s) => (s.kind === "running" ? { ...s, started: [...s.started, id] } : s)),
    );
    try {
      const outcomes = await api.runCleanup(
        findings.map((f) => f.id),
        mode,
        lang,
      );
      setStage({ kind: "done", outcomes });
    } catch (e) {
      setStage({
        kind: "done",
        outcomes: findings.map((f) => ({ id: f.id, name: f.name, status: "failed", freed: 0, recycled: false, failedItems: 0, message: String(e) })),
      });
    } finally {
      unlisten();
    }
  };

  return (
    <div className="modal-backdrop" onClick={() => stage.kind === "confirm" && onClose(null)}>
      <div className="modal cleanup" role="dialog" aria-modal="true" onClick={(e) => e.stopPropagation()}>
        {stage.kind === "confirm" && (
          <Confirm findings={findings} total={total} mode={mode} setMode={setMode} onCancel={() => onClose(null)} onStart={start} />
        )}
        {busy && <Running findings={findings} started={stage.started} />}
        {stage.kind === "done" && <Done outcomes={stage.outcomes} onClose={() => onClose(stage.outcomes)} />}
      </div>
    </div>
  );
}

function Confirm(props: {
  findings: Finding[];
  total: number;
  mode: CleanupMode;
  setMode: (mode: CleanupMode) => void;
  onCancel: () => void;
  onStart: () => void;
}) {
  const { findings, total, mode, setMode, onCancel, onStart } = props;
  const has = (id: string) => findings.some((f) => f.id === id);
  const notes = [
    findings.some((f) => f.admin) ? t.adminNote : null,
    has("recycle-bin") ? t.recycleBinNote : null,
    has("old-downloads") && mode === "permanent" ? t.downloadsNote : null,
    findings.some((f) => f.category === "ai") && mode === "permanent" ? t.aiNote : null,
  ].filter(Boolean);

  return (
    <>
      <h2>{t.confirmTitle}</h2>
      <ul className="cleanup-list">
        {findings.map((f) => (
          <li key={f.id}>
            <span className="cleanup-name">
              {f.name}
              {f.admin && <span className="tag">{t.needsAdmin}</span>}
            </span>
            <span className="cleanup-size">{f.measured ? formatBytes(f.bytes) : "?"}</span>
            {f.runningApps.length > 0 && <span className="cleanup-warn">{t.appsRunning(f.runningApps.join(", "))}</span>}
            {f.blocked && <span className="cleanup-warn">{f.blocked}</span>}
          </li>
        ))}
      </ul>
      <div className="cleanup-total">{t.total(formatBytes(total))}</div>
      <div className="modes" role="radiogroup">
        {(
          [
            ["recycle", t.modeRecycle, t.modeRecycleHint],
            ["permanent", t.modePermanent, t.modePermanentHint],
          ] as const
        ).map(([value, label, hint]) => (
          <label key={value} className={`mode ${mode === value ? "active" : ""}`}>
            <input type="radio" name="mode" checked={mode === value} onChange={() => setMode(value)} />
            <span>
              <strong>{label}</strong>
              <small>{hint}</small>
            </span>
          </label>
        ))}
      </div>
      {notes.map((n) => (
        <p key={n} className="note">
          {n}
        </p>
      ))}
      <div className="modal-foot">
        <button className="btn" onClick={onCancel}>
          {t.cancel}
        </button>
        <button className={`btn primary ${mode === "permanent" ? "danger" : ""}`} onClick={onStart}>
          {mode === "permanent" ? t.modePermanent : t.modeRecycle}
        </button>
      </div>
    </>
  );
}

function Running({ findings, started }: { findings: Finding[]; started: string[] }) {
  const current = started[started.length - 1];
  return (
    <>
      <h2>{t.running}</h2>
      <p className="muted">{t.runningHint}</p>
      <ul className="cleanup-list">
        {findings.map((f) => {
          const state = f.id === current ? "active" : started.includes(f.id) ? "done" : "pending";
          return (
            <li key={f.id} className={state}>
              <span className="cleanup-name">
                {state === "active" ? <span className="spinner small" /> : <span className={`status-dot ${state}`} />}
                {f.name}
              </span>
              <span className="cleanup-size">{f.measured ? formatBytes(f.bytes) : "?"}</span>
            </li>
          );
        })}
      </ul>
    </>
  );
}

function Done({ outcomes, onClose }: { outcomes: Outcome[]; onClose: () => void }) {
  const freed = outcomes.filter((o) => !o.recycled).reduce((sum, o) => sum + o.freed, 0);
  const moved = outcomes.filter((o) => o.recycled).reduce((sum, o) => sum + o.freed, 0);
  return (
    <>
      <h2>{t.doneTitle}</h2>
      <div className="cleanup-result">
        {freed > 0 && <strong className="gain">{t.freedNow(formatBytes(freed))}</strong>}
        {moved > 0 && <strong>{t.movedToBin(formatBytes(moved))}</strong>}
        {freed === 0 && moved === 0 && <strong>{t.nothingDone}</strong>}
        {moved > 0 && <p className="muted">{t.movedHint}</p>}
      </div>
      <ul className="cleanup-list">
        {outcomes.map((o) => (
          <li key={o.id} className={o.status}>
            <span className="cleanup-name">
              <span className={`status-dot ${o.status}`} />
              {o.name}
            </span>
            <span className="cleanup-size">{o.freed > 0 ? formatBytes(o.freed) : t.status[o.status]}</span>
            {o.message && <span className="cleanup-warn">{o.message}</span>}
          </li>
        ))}
      </ul>
      <div className="modal-foot">
        <button className="btn primary" onClick={onClose}>
          {t.ok}
        </button>
      </div>
    </>
  );
}
