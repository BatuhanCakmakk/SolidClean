import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import logoUrl from "./assets/logo.svg";
import {
  api,
  FLAG_CLOUD,
  FLAG_DENIED,
  FLAG_LINK,
  type Category,
  type FileEntry,
  type Finding,
  type HistoryEntry,
  type Locations,
  type NodeKind,
  type Outcome,
  type Safety,
  type ScanProgress,
  type ScanSummary,
  type TreeResponse,
  type Volume,
} from "./api";
import { categoryColor } from "./categories";
import { AboutDialog } from "./components/AboutDialog";
import { CleanupDialog } from "./components/CleanupDialog";
import { MoreMenu } from "./components/MoreMenu";
import { Recommendations, selectedBytes } from "./components/Recommendations";
import { HATCH, tileLabel, Treemap, type Tile } from "./components/Treemap";
import { UninstallDialog } from "./components/UninstallDialog";
import { formatBytes, formatCount, formatDate, formatPercent, formatSeconds, joinPath } from "./format";
import { LANGUAGES, lang, locale, setLanguage, t, type LanguageCode } from "./i18n";

type Phase = "idle" | "scanning" | "done";
type Tab = "recommendations" | "largest" | "history";

interface Crumb {
  index: number;
  name: string;
}

/** Gezgin altındaki bilgi şeridinde gösterilen öğe. */
interface Focus {
  label: string;
  kind: NodeKind;
  category: Category;
  flags: number;
  size: number;
  alloc: number;
  files: number;
  dirs: number;
  modified: number;
  cleanup: Safety | null;
  path: string;
}

const LIST_LIMIT = 40;

function volumeFor(locations: Locations | null, path: string): Volume | undefined {
  const lower = path.toLowerCase();
  return locations?.volumes.find((v) => lower.startsWith(v.root.toLowerCase()));
}

const defaultSelection = (findings: Finding[]) =>
  new Set(findings.filter((f) => f.safety === "safe" && !f.manual).map((f) => f.id));

export default function App() {
  const [locations, setLocations] = useState<Locations | null>(null);
  const [root, setRoot] = useState("");
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [summary, setSummary] = useState<ScanSummary | null>(null);
  const [nav, setNav] = useState<Crumb[]>([]);
  const [tree, setTree] = useState<TreeResponse | null>(null);
  const [largest, setLargest] = useState<FileEntry[]>([]);
  const [focus, setFocus] = useState<Focus | null>(null);
  const [findings, setFindings] = useState<Finding[] | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [cleaning, setCleaning] = useState(false);
  /** Temizlikten sonra öneriler yeniden hesaplansın diye artırılır. */
  const [analysisRun, setAnalysisRun] = useState(0);
  const [treeStale, setTreeStale] = useState(false);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [tab, setTab] = useState<Tab>("recommendations");
  const [error, setError] = useState<string | null>(null);
  /** Doluysa bir sonraki öneri hesabında varsayılan seçim yerine bu seçim kullanılır. */
  const keepSelection = useRef<Set<string> | null>(null);
  const [uninstall, setUninstall] = useState<Focus | null>(null);
  const [about, setAbout] = useState(false);
  /** "Bunu bir daha önerme" denen önerilerin kimlikleri; bu bilgisayarda saklanır. */
  const [ignored, setIgnored] = useState<string[]>(readIgnored);

  useEffect(() => writeIgnored(ignored), [ignored]);

  useEffect(() => {
    api
      .locations()
      .then((l) => {
        setLocations(l);
        setRoot(volumeFor(l, l.systemDrive)?.root ?? l.home);
      })
      .catch((e) => setError(String(e)));
    api.history().then(setHistory).catch(() => {});
  }, []);

  useEffect(() => {
    const unlisten = [
      api.onProgress(setProgress),
      api.onDone((s) => {
        setSummary(s);
        setNav([]);
        setPhase(s.error ? "idle" : "done");
        if (s.error) setError(s.error);
      }),
    ];
    return () => unlisten.forEach((p) => p.then((off) => off()));
  }, []);

  // Tarama bitince öneriler hesaplanır; taranmış ağaç sayesinde çoğu klasör tekrar okunmaz.
  useEffect(() => {
    if (phase !== "done") return;
    let alive = true;
    setFindings(null);
    api
      .analyze(lang)
      .then((f) => {
        if (!alive) return;
        // Yeni taramada güvenliler seçili gelir; temizlikten sonra kullanıcının seçimi korunur.
        const keep = keepSelection.current;
        keepSelection.current = null;
        setFindings(f);
        setSelected(keep ? new Set(f.filter((x) => keep.has(x.id)).map((x) => x.id)) : defaultSelection(f));
      })
      .catch((e) => setError(String(e)));
    return () => {
      alive = false;
    };
  }, [phase, summary, analysisRun]);

  // Öneriler gelince ağaç yeniden istenir: taralı işaretler onlarla birlikte gelir.
  useEffect(() => {
    if (phase !== "done") return;
    let alive = true;
    api
      .tree(nav.map((c) => c.index), 2, LIST_LIMIT)
      .then((r) => {
        if (!alive) return;
        setTree(r);
        setFocus(null);
      })
      .catch((e) => setError(String(e)));
    return () => {
      alive = false;
    };
  }, [phase, nav, summary, findings]);

  useEffect(() => {
    if (phase === "done") api.largestFiles(25).then(setLargest).catch((e) => setError(String(e)));
  }, [phase, summary]);

  const startScan = async () => {
    setError(null);
    setTree(null);
    setLargest([]);
    setSummary(null);
    setProgress(null);
    setFocus(null);
    setFindings(null);
    setSelected(new Set());
    setTreeStale(false);
    setPhase("scanning");
    try {
      await api.startScan(root);
    } catch (e) {
      setError(String(e));
      setPhase("idle");
    }
  };

  const reveal = useCallback((path: string) => api.reveal(path).catch((e) => setError(String(e))), []);

  const toggle = (id: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const finishCleanup = (outcomes: Outcome[] | null) => {
    setCleaning(false);
    if (!outcomes || outcomes.every((o) => o.status === "skipped")) return;
    // Kullanıcının seçimi korunur; yalnızca temizlenenlerin işareti kalkar. Atlananlar ve
    // yapılamayanlar seçili kalır, tekrar denenebilir.
    const cleaned = new Set(outcomes.filter((o) => o.status === "done" || o.status === "partial").map((o) => o.id));
    keepSelection.current = new Set([...selected].filter((id) => !cleaned.has(id)));
    // Disk değişti: boş alanı, önerileri ve geçmişi tazele; ağaç eski kalır.
    setTreeStale(true);
    setAnalysisRun((n) => n + 1);
    api.locations().then(setLocations).catch(() => {});
    api.history().then(setHistory).catch(() => {});
  };

  const volume = volumeFor(locations, root);
  const used = volume ? volume.total - volume.free : 0;
  const isVolumeRoot = !!volume && root.toLowerCase() === volume.root.toLowerCase();
  const systemVolume = locations ? volumeFor(locations, locations.systemDrive) : undefined;

  // "Bunu bir daha önerme" denenler listede, toplamlarda ve temizlikte yer almaz.
  const visible = useMemo(() => findings?.filter((f) => !ignored.includes(f.id)) ?? null, [findings, ignored]);
  const hiddenFindings = useMemo(() => (findings ?? []).filter((f) => ignored.includes(f.id)), [findings, ignored]);
  const chosen = useMemo(() => new Set([...selected].filter((id) => !ignored.includes(id))), [selected, ignored]);

  const totals = useMemo(() => {
    const sum = (safety: Safety) => {
      const items = (visible ?? []).filter((f) => f.safety === safety);
      return { bytes: items.reduce((s, f) => s + f.bytes, 0), count: items.length };
    };
    return { safe: sum("safe"), review: sum("review"), selected: selectedBytes(visible, chosen) };
  }, [visible, chosen]);

  const openTile = (tile: Tile) => {
    if (tile.node.kind === "dir" && tile.path.length > 0 && tile.node.childCount > 0) {
      setNav((prev) => [...prev, ...tile.path.map((index, i) => ({ index, name: tile.names[i] }))]);
    }
  };

  const hoverTile = (tile: Tile | null) => {
    if (!tile || !tree) return;
    const { node } = tile;
    // Toplu öğelerin kendi yolu yok; bulundukları klasörü göster.
    const names = node.kind === "small" || node.kind === "rest" ? tile.names.slice(0, -1) : tile.names;
    setFocus({ ...node, label: tileLabel(node), path: joinPath(tree.fullPath, names) });
  };

  const current: Focus | null = tree
    ? { ...tree.node, label: nav.length ? tree.node.name : tree.fullPath, path: tree.fullPath }
    : null;

  // Treemap'in gösterdiği iki seviyeye göre kategori payları.
  const breakdown = useMemo(() => {
    const bytes = new Map<Category, number>();
    const add = (category: Category, n: number) => {
      if (n > 0) bytes.set(category, (bytes.get(category) ?? 0) + n);
    };
    for (const child of tree?.node.children ?? []) {
      let inner = 0;
      for (const grand of child.children) {
        add(grand.category, grand.alloc);
        inner += grand.alloc;
      }
      add(child.category, child.alloc - inner);
    }
    return [...bytes].map(([category, n]) => ({ category, bytes: n })).sort((a, b) => b.bytes - a.bytes);
  }, [tree]);

  const analyzed = phase === "done" && findings !== null;

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <Logo />
          <div>
            <div className="brand-name">SolidClean</div>
            <div className="brand-sub">{t.tagline}</div>
          </div>
        </div>
        <div className="topbar-actions">
          <label className="lang-picker" title={t.language}>
            <span aria-hidden="true">&#xE774;</span>
            <select
              aria-label={t.language}
              value={lang}
              disabled={phase === "scanning" || cleaning}
              onChange={(e) => setLanguage(e.target.value as LanguageCode)}
            >
              {LANGUAGES.map((l) => (
                <option key={l.code} value={l.code}>
                  {l.name}
                </option>
              ))}
            </select>
          </label>
          <select value={root} disabled={phase === "scanning" || !locations} onChange={(e) => setRoot(e.target.value)}>
            {locations?.volumes.map((v) => (
              <option key={v.root} value={v.root}>
                {`${v.label || t.localDisk} (${v.root.slice(0, 2)}) · ${t.freeOf(formatBytes(v.free))}`}
              </option>
            ))}
            {locations && <option value={locations.home}>{`${t.home} (${locations.home})`}</option>}
          </select>
          {phase === "scanning" ? (
            <button className="btn" onClick={() => api.cancelScan()}>
              {t.stop}
            </button>
          ) : (
            <button className="btn primary" onClick={startScan} disabled={!root}>
              {phase === "done" ? t.rescan : t.scan}
            </button>
          )}
          <MoreMenu onAbout={() => setAbout(true)} onSupport={() => api.openSupportPage().catch((e) => setError(String(e)))} />
        </div>
      </header>

      {error && (
        <div className="notice error" onClick={() => setError(null)}>
          {error}
        </div>
      )}
      {summary?.cancelled && !summary.error && <div className="notice">{t.cancelled}</div>}

      <section className="stats">
        <Stat
          label={systemVolume ? t.freeAfter(systemVolume.root.slice(0, 2)) : t.freeSpace}
          value={systemVolume ? formatBytes(systemVolume.free + totals.selected) : "—"}
          sub={
            totals.selected > 0 ? (
              <span className="gain">{t.plusSelected(formatBytes(totals.selected))}</span>
            ) : systemVolume ? (
              t.ofTotal(formatBytes(systemVolume.total))
            ) : (
              ""
            )
          }
        >
          {systemVolume && <Meter value={(systemVolume.total - systemVolume.free - totals.selected) / systemVolume.total} />}
        </Stat>
        <Stat
          dot="safe"
          label={t.safeToRemove}
          value={analyzed ? formatBytes(totals.safe.bytes) : "—"}
          sub={analyzed ? t.itemCount(formatCount(totals.safe.count)) : ""}
        />
        <Stat
          dot="review"
          label={t.needsReview}
          value={analyzed ? formatBytes(totals.review.bytes) : "—"}
          sub={analyzed ? t.itemCount(formatCount(totals.review.count)) : ""}
        />
        <Stat
          label={t.notScanned}
          value={summary && volume ? formatBytes(Math.max(0, used - summary.alloc)) : "—"}
          sub={summary ? (/^[a-z]:\\?$/i.test(summary.rootPath) ? t.notScannedSubDrive : t.notScannedSub) : ""}
        />
      </section>

      <main className="main">
        <section className="card explorer">
          <div className="card-head">
            <div>
              <h2>{t.explorer}</h2>
              <p>{t.explorerSub}</p>
            </div>
            {summary && (
              <div className="scan-info" title={summary.errors ? t.unreadable(formatCount(summary.errors)) : t.allRead}>
                {t.scanInfo(formatCount(summary.files), formatSeconds(summary.elapsedMs))}
                {summary.errors > 0 && ` · ${t.unreadable(formatCount(summary.errors))}`}
              </div>
            )}
          </div>
          {phase === "done" && treeStale && (
            <div className="notice stale">
              {t.staleTree}
              <button className="btn small" onClick={startScan}>
                {t.rescan}
              </button>
            </div>
          )}
          {phase === "done" && tree && (
            <nav className="crumbs">
              <button onClick={() => setNav([])} disabled={nav.length === 0}>
                {summary?.rootPath}
              </button>
              {nav.map((c, i) => (
                <span key={i}>
                  <span className="sep">›</span>
                  <button onClick={() => setNav(nav.slice(0, i + 1))} disabled={i === nav.length - 1}>
                    {c.name}
                  </button>
                </span>
              ))}
              {analyzed && (
                <span className="marks">
                  <span>
                    <i className="hatch-swatch safe" />
                    {t.safeToRemove}
                  </span>
                  <span>
                    <i className="hatch-swatch review" />
                    {t.needsReview}
                  </span>
                </span>
              )}
              <span className="crumbs-size">{formatBytes(tree.node.alloc)}</span>
            </nav>
          )}
          <div className="treemap-area">
            {phase === "idle" && (
              <div className="empty">
                <Logo size={44} />
                <h3>{t.emptyTitle}</h3>
                <p>{t.emptyBody}</p>
              </div>
            )}
            {phase === "scanning" && <ScanningView progress={progress} expected={isVolumeRoot ? used : 0} />}
            {phase === "done" && tree && <Treemap root={tree.node} onOpen={openTile} onHover={hoverTile} />}
          </div>
          {phase === "done" && (focus ?? current) && (
            <FocusBar
              item={(focus ?? current)!}
              total={summary?.alloc ?? 0}
              appRoots={locations?.appRoots ?? []}
              onReveal={reveal}
              onUninstall={setUninstall}
            />
          )}
          {phase === "done" && <Composition parts={breakdown} />}
        </section>

        <aside className="card side">
          <div className="tabs" role="tablist">
            <button role="tab" aria-selected={tab === "recommendations"} onClick={() => setTab("recommendations")}>
              {t.tabRecommendations}
              {visible && visible.length > 0 && <span className="count">{visible.length}</span>}
            </button>
            <button role="tab" aria-selected={tab === "largest"} onClick={() => setTab("largest")}>
              {t.tabLargest}
            </button>
            <button role="tab" aria-selected={tab === "history"} onClick={() => setTab("history")}>
              {t.tabHistory}
            </button>
          </div>
          {tab === "history" ? (
            <HistoryList entries={history} />
          ) : phase !== "done" ? (
            <p className="side-empty muted">{t.idleHint}</p>
          ) : tab === "recommendations" ? (
            <Recommendations
              findings={visible}
              hidden={hiddenFindings}
              selected={chosen}
              onToggle={toggle}
              onIgnore={(id) => setIgnored((prev) => (prev.includes(id) ? prev : [...prev, id]))}
              onRestore={(id) => setIgnored((prev) => prev.filter((x) => x !== id))}
              onSelectSafe={() => visible && setSelected(defaultSelection(visible))}
              onClean={() => setCleaning(true)}
              onReveal={reveal}
            />
          ) : (
            <ul className="largest">
              {largest.map((f) => (
                <li key={f.path}>
                  <button
                    onClick={() =>
                      setFocus({ ...f, label: f.name, kind: "file", flags: 0, files: 1, dirs: 0, cleanup: null })
                    }
                    onDoubleClick={() => reveal(f.path)}
                    title={f.path}
                  >
                    <i style={{ background: categoryColor(f.category) }} />
                    <span className="file-text">
                      <span className="file-name">{f.name}</span>
                      <span className="file-path">{f.path.slice(0, f.path.length - f.name.length - 1)}</span>
                    </span>
                    <span className="file-size">{formatBytes(f.alloc)}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </aside>
      </main>

      {cleaning && visible && (
        <CleanupDialog findings={visible.filter((f) => chosen.has(f.id))} onClose={finishCleanup} />
      )}
      {about && <AboutDialog onClose={() => setAbout(false)} />}
      {uninstall && <UninstallDialog path={uninstall.path} label={uninstall.label} onClose={() => setUninstall(null)} />}
    </div>
  );
}

function HistoryList({ entries }: { entries: HistoryEntry[] }) {
  if (entries.length === 0) return <p className="side-empty muted">{t.noHistory}</p>;
  return (
    <ul className="history">
      {entries.map((e) => (
        <li key={e.time}>
          <div className="history-head">
            <span>{new Date(e.time).toLocaleString(locale, { dateStyle: "medium", timeStyle: "short" })}</span>
            <span className="history-total">
              {e.freed > 0 && <span className="gain">{t.freedNow(formatBytes(e.freed))}</span>}
              {e.recycled > 0 && <span>{t.movedToBin(formatBytes(e.recycled))}</span>}
            </span>
          </div>
          <div className="history-items">{e.items
              .filter((o) => o.status !== "skipped")
              .map((o) => o.name)
              .join(", ")}</div>
        </li>
      ))}
    </ul>
  );
}

function Stat({
  label,
  value,
  sub,
  dot,
  children,
}: {
  label: string;
  value: string;
  sub: React.ReactNode;
  dot?: Safety;
  children?: React.ReactNode;
}) {
  return (
    <div className="stat">
      <div className="stat-label">
        {dot && <i className="stat-dot" style={{ background: HATCH[dot].border }} />}
        {label}
      </div>
      <div className="stat-value">{value}</div>
      <div className="stat-sub">{sub || " "}</div>
      {children}
    </div>
  );
}

function Meter({ value }: { value: number }) {
  const pct = Math.min(100, Math.max(0, value * 100));
  return (
    <div className={`meter ${pct > 90 ? "danger" : ""}`}>
      <div style={{ width: `${pct}%` }} />
    </div>
  );
}

function ScanningView({ progress, expected }: { progress: ScanProgress | null; expected: number }) {
  const pct = expected > 0 && progress ? Math.min(99, (progress.alloc / expected) * 100) : null;
  return (
    <div className="scanning">
      <div className="spinner" />
      <div className="scan-title">{t.scanning}…</div>
      <div className="scan-count">
        {formatCount(progress?.files ?? 0)} <span>{t.files}</span>
      </div>
      <div className="scan-size">{formatBytes(progress?.alloc ?? 0)}</div>
      <div className={`bar ${pct === null ? "indeterminate" : ""}`}>
        <div style={pct === null ? undefined : { width: `${pct}%` }} />
      </div>
      <div className="scan-path" title={progress?.current}>
        {progress?.current || " "}
      </div>
    </div>
  );
}

/** Uygulama kökünün (ör. `C:\Program Files`) altındaki bir klasör mü? Kökün kendisi değil. */
function inAppRoot(path: string, roots: string[]): boolean {
  const lower = path.toLowerCase();
  return roots.some((r) => lower.startsWith(r.toLowerCase().replace(/\\$/, "") + "\\"));
}

function FocusBar({
  item,
  total,
  appRoots,
  onReveal,
  onUninstall,
}: {
  item: Focus;
  total: number;
  appRoots: string[];
  onReveal: (path: string) => void;
  onUninstall: (item: Focus) => void;
}) {
  const notes = [
    item.flags & FLAG_CLOUD ? t.cloudNote : null,
    item.flags & FLAG_LINK ? t.linkNote : null,
    item.flags & FLAG_DENIED ? t.deniedNote : null,
  ].filter(Boolean);
  const app = item.kind === "dir" && inAppRoot(item.path, appRoots);
  const share = total > 0 ? Math.min(1, item.alloc / total) : 0;
  return (
    <div className="focus" style={{ "--cat": categoryColor(item.category) } as React.CSSProperties}>
      <span className="focus-icon">
        <KindIcon kind={item.kind} />
      </span>
      <div className="focus-text">
        <div className="focus-title">
          <span className="focus-name" title={item.label}>
            {item.label}
          </span>
          {item.cleanup && (
            <span className={`pill ${item.cleanup}`}>{item.cleanup === "safe" ? t.safeToRemove : t.needsReview}</span>
          )}
        </div>
        <div className="focus-meta" title={item.path}>
          {[
            t.categories[item.category],
            ...notes,
            item.kind !== "file" ? `${formatCount(item.files)} ${t.files}` : null,
            `${t.modified}: ${formatDate(item.modified)}`,
          ]
            .filter(Boolean)
            .join(" · ")}
        </div>
      </div>
      <div className="focus-share">
        <span>
          {t.shareOfScan} <b>{formatPercent(share)}</b>
        </span>
        <div className="focus-meter">
          <div style={{ width: `${share * 100}%` }} />
        </div>
      </div>
      <div className="focus-size">
        <strong>{formatBytes(item.alloc)}</strong>
        {item.size !== item.alloc && (
          <span title={t.logicalSize}>
            {t.logicalSize}: {formatBytes(item.size)}
          </span>
        )}
      </div>
      <button className="btn small with-icon" onClick={() => onReveal(item.path)}>
        <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M9 2.5h4.5V7M13.5 2.5 7.5 8.5M6 3.5H4A1.5 1.5 0 0 0 2.5 5v7A1.5 1.5 0 0 0 4 13.5h7a1.5 1.5 0 0 0 1.5-1.5v-2" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
        {t.showInExplorer}
      </button>
      {app && (
        <button className="btn small" onClick={() => onUninstall(item)}>
          {t.uninstall}
        </button>
      )}
    </div>
  );
}

const IGNORED_KEY = "ignoredFindings";

function readIgnored(): string[] {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(IGNORED_KEY) ?? "[]");
    return Array.isArray(value) ? value.filter((x): x is string => typeof x === "string") : [];
  } catch {
    return [];
  }
}

function writeIgnored(ids: string[]) {
  try {
    localStorage.setItem(IGNORED_KEY, JSON.stringify(ids));
  } catch {
    // Depolama kapalıysa liste yalnızca bu oturumda geçerli.
  }
}

function KindIcon({ kind }: { kind: NodeKind }) {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" aria-hidden="true">
      {kind === "file" ? (
        <>
          <path d="M5.5 2h6l4 4v10.5a1.5 1.5 0 0 1-1.5 1.5H5.5A1.5 1.5 0 0 1 4 16.5v-13A1.5 1.5 0 0 1 5.5 2Z" fill="currentColor" />
          <path d="M11.5 2v2.5A1.5 1.5 0 0 0 13 6h2.5" fill="none" stroke="var(--card)" strokeWidth="1.2" opacity=".7" />
        </>
      ) : kind === "dir" ? (
        <>
          <path d="M2 5.5A1.5 1.5 0 0 1 3.5 4h3.8l1.7 1.8h7.5A1.5 1.5 0 0 1 18 7.3v8.2a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 2 15.5Z" fill="currentColor" />
          <path d="M2 8.5h16" stroke="var(--card)" strokeWidth="1.2" opacity=".55" />
        </>
      ) : (
        <>
          <rect x="5" y="3" width="11" height="11" rx="2" fill="currentColor" opacity=".45" />
          <rect x="3" y="6" width="11" height="11" rx="2" fill="currentColor" />
        </>
      )}
    </svg>
  );
}

function Composition({ parts }: { parts: { category: Category; bytes: number }[] }) {
  const total = parts.reduce((sum, p) => sum + p.bytes, 0);
  if (total === 0) return null;
  return (
    <div className="composition">
      <div className="composition-bar">
        {parts.map((p) => (
          <i
            key={p.category}
            style={{ flexGrow: p.bytes, background: categoryColor(p.category) }}
            title={`${t.categories[p.category]} · ${formatBytes(p.bytes)}`}
          />
        ))}
      </div>
      <div className="composition-legend">
        {parts.map((p) => (
          <span key={p.category}>
            <i style={{ background: categoryColor(p.category) }} />
            {t.categories[p.category]}
            <b>{formatPercent(p.bytes / total)}</b>
          </span>
        ))}
      </div>
    </div>
  );
}

function Logo({ size = 30 }: { size?: number }) {
  return <img src={logoUrl} width={size} height={size} alt="" />;
}
