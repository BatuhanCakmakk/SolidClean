import { useState } from "react";
import type { Finding, Safety } from "../api";
import { categoryColor } from "../categories";
import { formatBytes, formatCount } from "../format";
import { t } from "../i18n";

interface Props {
  /** Gösterilen öneriler (gizlenenler hariç). */
  findings: Finding[] | null;
  /** Kullanıcının "bunu bir daha önerme" dediği öneriler. */
  hidden: Finding[];
  selected: Set<string>;
  onToggle: (id: string) => void;
  onSelectSafe: () => void;
  onClean: () => void;
  onReveal: (path: string) => void;
  onIgnore: (id: string) => void;
  onRestore: (id: string) => void;
}

const GROUPS: { safety: Safety; label: string }[] = [
  { safety: "safe", label: t.groupSafe },
  { safety: "review", label: t.groupReview },
];

export function selectedBytes(findings: Finding[] | null, selected: Set<string>): number {
  return (findings ?? []).filter((f) => selected.has(f.id)).reduce((sum, f) => sum + f.bytes, 0);
}

export function Recommendations(props: Props) {
  const { findings, hidden, selected, onToggle, onSelectSafe, onClean, onReveal, onIgnore, onRestore } = props;
  const [open, setOpen] = useState<string | null>(null);
  const [showHidden, setShowHidden] = useState(false);

  if (!findings) {
    return (
      <div className="rec-empty">
        <div className="spinner small" />
        {t.analyzing}
      </div>
    );
  }
  if (findings.length === 0 && hidden.length === 0) return <div className="rec-empty">{t.noFindings}</div>;

  return (
    <div className="rec">
      <div className="rec-list">
        {findings.length === 0 && <div className="rec-empty">{t.allHidden}</div>}
        {GROUPS.map(({ safety, label }) => {
          const items = findings.filter((f) => f.safety === safety);
          if (items.length === 0) return null;
          return (
            <div key={safety} className="rec-group">
              <div className="rec-group-head">
                <span className={`pill ${safety}`}>{label}</span>
                <span>{formatBytes(items.reduce((sum, f) => sum + f.bytes, 0))}</span>
              </div>
              {items.map((f) => (
                <Row
                  key={f.id}
                  finding={f}
                  checked={selected.has(f.id)}
                  open={open === f.id}
                  onToggle={() => onToggle(f.id)}
                  onOpen={() => setOpen(open === f.id ? null : f.id)}
                  onReveal={onReveal}
                  onIgnore={() => onIgnore(f.id)}
                />
              ))}
            </div>
          );
        })}
        {hidden.length > 0 && (
          <div className="rec-hidden">
            <button className="rec-hidden-head" onClick={() => setShowHidden(!showHidden)} aria-expanded={showHidden}>
              <span className="chev-inline">›</span>
              {t.hiddenTitle(formatCount(hidden.length))}
            </button>
            {showHidden &&
              hidden.map((f) => (
                <div key={f.id} className="rec-hidden-row">
                  <span className="rec-hidden-name">{f.name}</span>
                  <span className="rec-size">{f.measured ? formatBytes(f.bytes) : "?"}</span>
                  <button className="btn small" onClick={() => onRestore(f.id)}>
                    {t.restore}
                  </button>
                </div>
              ))}
          </div>
        )}
      </div>
      <div className="rec-foot">
        <span className="rec-total">{t.selected(formatBytes(selectedBytes(findings, selected)), formatCount(selected.size))}</span>
        <div className="rec-actions">
          <button className="btn" onClick={onSelectSafe}>
            {t.selectSafe}
          </button>
          <button className="btn primary" onClick={onClean} disabled={selected.size === 0}>
            {t.clean}
          </button>
        </div>
      </div>
    </div>
  );
}

interface RowProps {
  finding: Finding;
  checked: boolean;
  open: boolean;
  onToggle: () => void;
  onOpen: () => void;
  onReveal: (path: string) => void;
  onIgnore: () => void;
}

function Row({ finding: f, checked, open, onToggle, onOpen, onReveal, onIgnore }: RowProps) {
  const manual = f.manual !== null;
  const hidden = f.targets.length - 4;
  return (
    <div className={`rec-row ${open ? "open" : ""}`}>
      <div className="rec-main">
        <input
          type="checkbox"
          checked={checked}
          disabled={manual}
          title={manual ? t.manualOnly : undefined}
          onChange={onToggle}
        />
        <button className="rec-text" onClick={onOpen} aria-expanded={open}>
          <span className="rec-name">
            {f.name}
            {(f.runningApps.length > 0 || f.blocked) && (
              <span className="dot-warn" title={f.blocked ?? t.appsRunning(f.runningApps.join(", "))} />
            )}
          </span>
          <span className="rec-path" title={f.location}>
            {f.location}
          </span>
        </button>
        <span className="rec-size" title={f.measured ? undefined : t.unknownSize}>
          {f.measured ? formatBytes(f.bytes) : f.bytes > 0 ? `≥ ${formatBytes(f.bytes)}` : "?"}
        </span>
        <button className="chev" onClick={onOpen} aria-label={f.name}>
          ›
        </button>
      </div>
      {open && (
        <div className="rec-detail">
          <p>
            <b>{t.whatIsIt}</b> {f.about}
          </p>
          <p>
            <b>{t.ifDeleted}</b> {f.effect}
          </p>
          {f.manual && (
            <p>
              <b>{t.howTo}</b> {f.manual}
            </p>
          )}
          <div className="tags">
            <span className="tag">
              <i style={{ background: categoryColor(f.category) }} />
              {t.categories[f.category]}
            </span>
            {f.regenerates && <span className="tag">{t.regenerates}</span>}
            {f.admin && <span className="tag">{t.needsAdmin}</span>}
            {manual && <span className="tag">{t.manualOnly}</span>}
          </div>
          {f.runningApps.length > 0 && <p className="warn">{t.appsRunning(f.runningApps.join(", "))}</p>}
          {f.blocked && <p className="warn">{f.blocked}</p>}
          {!f.measured && <p className="note">{t.unknownSize}</p>}
          {f.targets.length > 0 && (
            <ul className="targets">
              {f.targets.slice(0, 4).map((p) => (
                <li key={p} title={p}>
                  {p}
                </li>
              ))}
              {hidden > 0 && <li className="more">{t.moreTargets(formatCount(hidden))}</li>}
            </ul>
          )}
          <div className="rec-detail-actions">
            {f.targets.length > 0 && (
              <button className="btn small" onClick={() => onReveal(f.targets[0])}>
                {t.openFolder}
              </button>
            )}
            <button className="btn small quiet" onClick={onIgnore} title={t.ignoreHint} aria-label={t.ignore}>
              {t.ignore}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
