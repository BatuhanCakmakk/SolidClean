import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Volume {
  root: string;
  label: string;
  fileSystem: string;
  total: number;
  free: number;
  removable: boolean;
}

export interface Locations {
  home: string;
  systemDrive: string;
  volumes: Volume[];
  /** Uygulamaların kurulduğu kökler (Program Files, %LOCALAPPDATA%\Programs). */
  appRoots: string[];
}

export interface AppInfo {
  name: string;
  publisher: string | null;
}

export interface ScanProgress {
  files: number;
  dirs: number;
  alloc: number;
  errors: number;
  current: string;
  elapsedMs: number;
}

export interface ScanSummary {
  rootPath: string;
  elapsedMs: number;
  files: number;
  dirs: number;
  size: number;
  alloc: number;
  errors: number;
  cancelled: boolean;
  error: string | null;
}

export type Category = "media" | "apps" | "games" | "system" | "cache" | "archive" | "dev" | "ai" | "other";

/** "small": bir klasördeki küçük dosyaların toplamı, "rest": listeye sığmayan öğelerin toplamı. */
export type NodeKind = "dir" | "file" | "small" | "rest";

export type Safety = "safe" | "review";

export interface Finding {
  id: string;
  name: string;
  about: string;
  effect: string;
  /** Doluysa temizlik elle yapılır ve script'e girmez. */
  manual: string | null;
  category: Category;
  safety: Safety;
  regenerates: boolean;
  location: string;
  targets: string[];
  bytes: number;
  items: number;
  /** false: bir hedef okunamadı, boyut eksik (çoğunlukla yönetici izni gerekir). */
  measured: boolean;
  runningApps: string[];
  /** Doluysa temizlik şu an yapılamaz (ör. yeniden başlatma bekleniyor); neden bu metindir. */
  blocked: string | null;
}

/** recycle: Geri Dönüşüm Kutusu'na taşı (geri alınabilir), permanent: kalıcı sil. */
export type CleanupMode = "recycle" | "permanent";

export interface Outcome {
  id: string;
  name: string;
  status: "done" | "partial" | "skipped" | "failed";
  freed: number;
  /** true: `freed` Geri Dönüşüm Kutusu'na taşındı, diskte henüz yer açılmadı. */
  recycled: boolean;
  failedItems: number;
  message: string | null;
}

export interface HistoryEntry {
  time: number;
  mode: CleanupMode;
  freed: number;
  recycled: number;
  items: Outcome[];
}

export const FLAG_CLOUD = 1;
export const FLAG_LINK = 2;
export const FLAG_DENIED = 4;

export interface TreeNode {
  name: string;
  kind: NodeKind;
  category: Category;
  flags: number;
  size: number;
  alloc: number;
  files: number;
  dirs: number;
  modified: number;
  index: number;
  childCount: number;
  /** Bir temizlik önerisinin kapsamındaysa seviyesi. */
  cleanup: Safety | null;
  children: TreeNode[];
}

export interface TreeResponse {
  fullPath: string;
  node: TreeNode;
}

export interface FileEntry {
  name: string;
  path: string;
  size: number;
  alloc: number;
  category: Category;
  modified: number;
}

const tauriApi = {
  locations: () => invoke<Locations>("get_locations"),
  startScan: (root: string) => invoke<void>("start_scan", { root }),
  cancelScan: () => invoke<void>("cancel_scan"),
  tree: (path: number[], depth: number, limit: number) => invoke<TreeResponse>("get_tree", { path, depth, limit }),
  largestFiles: (limit: number) => invoke<FileEntry[]>("largest_files", { limit }),
  reveal: (path: string) => invoke<void>("reveal", { path }),
  analyze: (lang: string) => invoke<Finding[]>("analyze_cleanup", { lang }),
  runCleanup: (ids: string[], mode: CleanupMode, lang: string) =>
    invoke<Outcome[]>("run_cleanup", { ids, mode, lang }),
  history: () => invoke<HistoryEntry[]>("get_history"),
  appsForFolder: (path: string) => invoke<AppInfo[]>("apps_for_folder", { path }),
  /** Windows Ayarlar > Yüklü uygulamalar; kaldırma orada yapılır. */
  openInstalledApps: () => invoke<void>("open_installed_apps"),
  /** Buy Me a Coffee sayfasını tarayıcıda açar. */
  openLink: (link: "support" | "source") => invoke<void>("open_link", { link }),
  /** Her öneri temizlenmeye başlarken kimliğiyle çağrılır. */
  onCleanupProgress: (cb: (id: string) => void): Promise<UnlistenFn> =>
    listen<string>("cleanup-progress", (e) => cb(e.payload)),
  onProgress: (cb: (p: ScanProgress) => void): Promise<UnlistenFn> =>
    listen<ScanProgress>("scan-progress", (e) => cb(e.payload)),
  onDone: (cb: (s: ScanSummary) => void): Promise<UnlistenFn> =>
    listen<ScanSummary>("scan-done", (e) => cb(e.payload)),
};

// Tauri dışında, düz tarayıcıda açıldığında arayüz sahte veriyle çalışır (yalnızca geliştirmede).
const inTauri = "__TAURI_INTERNALS__" in window;

export const api: typeof tauriApi =
  !inTauri && import.meta.env.DEV ? (await import("./mock")).mockApi : tauriApi;
