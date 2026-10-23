// Tarayıcıda (Tauri dışında) arayüz geliştirmek için sahte veri. Yalnızca `npm run dev` ile yüklenir.
// Mağaza ekran görüntüleri de bu veriyle çekilir (store/screenshots), bu yüzden metinler iki dillidir.
import type {
  api as realApi,
  Category,
  FileEntry,
  Finding,
  HistoryEntry,
  Outcome,
  Safety,
  ScanProgress,
  ScanSummary,
  TreeNode,
} from "./api";
import { lang } from "./i18n";

type Api = typeof realApi;

const GB = 1024 ** 3;
const MB = 1024 ** 2;

const L = (tr: string, en: string) => (lang === "tr" ? tr : en);

interface Spec {
  name: string;
  cat: Category;
  size?: number;
  kids?: Spec[];
}

const file = (name: string, cat: Category, size: number): Spec => ({ name, cat, size });
const dir = (name: string, cat: Category, kids: Spec[]): Spec => ({ name, cat, kids });

const HOME_KIDS: Spec[] = [
  dir("Desktop", "other", [
    dir(L("okul-projesi", "school-project"), "other", [
      file(L("sunum-kaydi.mp4", "presentation.mp4"), "media", 6.2 * GB),
      file(L("veri.zip", "data.zip"), "archive", 2.1 * GB),
    ]),
    dir(L("fotograflar-2025", "photos-2025"), "media", [
      file("IMG_0001.heic", "media", 4 * MB),
      dir(L("tatil", "vacation"), "media", [file("clip.mov", "media", 3.4 * GB)]),
    ]),
    file(L("eski-yedek.iso", "old-backup.iso"), "archive", 4.6 * GB),
  ]),
  dir("AppData", "apps", [
    dir("Local", "apps", [
      dir("Temp", "cache", [file("setup_tmp.msi", "cache", 1.2 * GB), file("log.dmp", "cache", 900 * MB)]),
      dir("Google", "apps", [dir("Chrome", "apps", [dir("Cache", "cache", [file("data_3", "cache", 1.8 * GB)]), file("History", "apps", 300 * MB)])]),
      dir("Docker", "dev", [file("docker_data.vhdx", "dev", 14.4 * GB)]),
      dir("Programs", "apps", [dir("Discord", "apps", [file("app.asar", "apps", 600 * MB)]), dir("Cursor", "apps", [file("bin", "apps", 900 * MB)])]),
    ]),
    dir("Roaming", "apps", [dir("Code", "apps", [dir("CachedData", "cache", [file("bundle", "cache", 1.1 * GB)])])]),
  ]),
  dir("Videos", "media", [
    file(L("OBS kaydı 01.mkv", "OBS recording 01.mkv"), "media", 8.3 * GB),
    file(L("OBS kaydı 02.mkv", "OBS recording 02.mkv"), "media", 5.1 * GB),
    dir("Captures", "media", [file(L("oyun.mp4", "gameplay.mp4"), "media", 2.2 * GB)]),
  ]),
  dir("Downloads", "other", [
    file("Windows11.iso", "archive", 6.1 * GB),
    file("VSCodeSetup.exe", "archive", 98 * MB),
    file("film.mkv", "media", 2.4 * GB),
    file(L("rapor.pdf", "report.pdf"), "media", 12 * MB),
  ]),
  dir(".ollama", "ai", [dir("models", "ai", [file("llama3.1-8b", "ai", 4.6 * GB), file("mistral-7b", "ai", 3.8 * GB)])]),
  dir("Documents", "media", [
    dir(L("Tez", "Thesis"), "media", [file(L("veriler.xlsx", "data.xlsx"), "media", 220 * MB)]),
    file(L("taramalar.pdf", "scans.pdf"), "media", 640 * MB),
  ]),
  dir(".gradle", "dev", [dir("caches", "dev", [file("modules-2", "dev", 6.1 * GB)])]),
  dir("Pictures", "media", [
    file(L("Kamera Rulosu.zip", "Camera Roll.zip"), "archive", 1.6 * GB),
    dir("Screenshots", "media", [file(L("ekran.png", "screen.png"), "media", 3 * MB)]),
  ]),
  dir("OneDrive", "other", [file(L("buluttaki-video.mp4", "cloud-video.mp4"), "media", 0)]),
];

const HOME: Spec = dir("C:\\Users\\demo", "other", HOME_KIDS);

const DRIVE: Spec = dir("C:\\", "other", [
  dir("Users", "other", [dir("demo", "other", HOME_KIDS), dir("Public", "other", [file(L("ortak.zip", "shared.zip"), "archive", 300 * MB)])]),
  dir("Windows", "system", [
    dir("WinSxS", "system", [file(L("bileşenler", "components"), "system", 9.8 * GB)]),
    dir("System32", "system", [file(L("sürücüler", "drivers"), "system", 6.2 * GB)]),
  ]),
  file("pagefile.sys", "system", 8 * GB),
  file("hiberfil.sys", "system", 12.7 * GB),
  dir("Program Files", "apps", [
    dir("Docker", "dev", [dir("Docker", "dev", [file("resources", "dev", 3.1 * GB)])]),
    dir("Microsoft Office", "apps", [dir("root", "apps", [file("Office16", "apps", 4.2 * GB)])]),
    dir("NVIDIA Corporation", "apps", [file("Installer2", "apps", 1.9 * GB)]),
    dir("Adobe", "apps", [dir("Adobe Photoshop 2025", "apps", [file("Photoshop.exe", "apps", 2.7 * GB)])]),
    dir("WindowsApps", "apps", [file(L("paketler", "packages"), "apps", 6.4 * GB)]),
  ]),
  dir("Program Files (x86)", "apps", [
    dir("Steam", "games", [dir("steamapps", "games", [dir("common", "games", [file("Counter-Strike 2", "games", 34 * GB), file("Stardew Valley", "games", 650 * MB)])])]),
    dir("Microsoft", "apps", [dir("Edge", "apps", [file("Application", "apps", 820 * MB)])]),
  ]),
]);

const MARKS: Record<string, Safety> = {
  Temp: "safe",
  Cache: "safe",
  CachedData: "safe",
  ".gradle": "safe",
  ".ollama": "review",
  Docker: "review",
  "Windows11.iso": "review",
};

let seq = 0;
interface MockNode extends TreeNode {
  all: MockNode[];
}

function build(spec: Spec, index: number, inherited: Safety | null = null): MockNode {
  const cleanup = MARKS[spec.name] ?? inherited;
  const all = (spec.kids ?? []).map((k, i) => build(k, i, cleanup)).sort((a, b) => b.alloc - a.alloc);
  all.forEach((c, i) => (c.index = i));
  const own = spec.size ?? 0;
  const alloc = own + all.reduce((s, c) => s + c.alloc, 0);
  const isFile = !spec.kids;
  return {
    name: spec.name,
    kind: isFile ? "file" : "dir",
    category: spec.cat,
    flags: isFile && own === 0 ? 1 : 0,
    size: alloc || 1.3 * GB,
    alloc,
    files: isFile ? 1 : all.reduce((s, c) => s + c.files, 0) + 40,
    dirs: isFile ? 0 : all.reduce((s, c) => s + c.dirs + (c.kind === "dir" ? 1 : 0), 0),
    modified: Date.now() - (seq++ % 40) * 86_400_000,
    index,
    childCount: all.length,
    cleanup,
    children: [],
    all,
  };
}

const finding = (f: Partial<Finding> & Pick<Finding, "id" | "name" | "safety" | "bytes" | "location">): Finding => ({
  about: L("Programların kurulum ve çalışma sırasında bıraktığı geçici dosyalar.", "Leftovers that programs create while installing or running."),
  effect: L("Bir etkisi yok. Son 7 günde kullanılan ve şu an açık olan dosyalar atlanır.", "No effect. Files used in the last 7 days and files in use are skipped."),
  manual: null,
  category: "cache",
  admin: false,
  regenerates: true,
  targets: [f.location],
  items: 120,
  measured: true,
  runningApps: [],
  blocked: null,
  ...f,
});

const BROWSER_ABOUT = L("Ziyaret ettiğiniz sitelerden indirilen resim, betik ve dosya kopyaları.", "Copies of images, scripts and files from websites you visited.");
const BROWSER_EFFECT = L(
  "Siteler ilk açılışta biraz yavaş yüklenebilir. Şifreler, geçmiş ve oturumlar silinmez.",
  "Sites may load a bit slower the first time. Passwords, history and logins are kept.",
);

const FINDINGS: Finding[] = [
  finding({ id: "user-temp", name: L("Geçici dosyalar", "Temporary files"), safety: "safe", bytes: 2.1 * GB, location: "C:\\Users\\demo\\AppData\\Local\\Temp" }),
  finding({
    id: "browser-chrome",
    name: L("Chrome önbelleği", "Chrome cache"),
    safety: "safe",
    bytes: 1.8 * GB,
    location: "C:\\Users\\demo\\AppData\\Local\\Google\\Chrome\\User Data\\Default\\Cache +2",
    runningApps: ["chrome.exe"],
    about: BROWSER_ABOUT,
    effect: BROWSER_EFFECT,
  }),
  finding({
    id: "gradle-cache",
    name: L("Gradle önbelleği", "Gradle caches"),
    safety: "safe",
    bytes: 6.1 * GB,
    location: "C:\\Users\\demo\\.gradle\\caches",
    category: "dev",
    about: L("Android ve Java projeleri için indirilmiş kütüphaneler ve derleme önbelleği.", "Libraries and build caches downloaded for Android and Java projects."),
    effect: L("Bir sonraki derleme bağımlılıkları yeniden indirir ve daha uzun sürer.", "The next build downloads dependencies again and takes longer."),
  }),
  finding({
    id: "vscode-cache",
    name: L("VS Code önbelleği", "VS Code cache"),
    safety: "safe",
    bytes: 1.1 * GB,
    location: "C:\\Users\\demo\\AppData\\Roaming\\Code\\CachedData",
    category: "dev",
    about: L("Eski sürümlerden kalan dosyalar ve indirilmiş eklenti paketleri.", "Files left from older versions and downloaded extension packages."),
    effect: L("Bir etkisi yok. Ayarlarınız ve eklentileriniz kalır.", "No effect. Your settings and extensions stay."),
  }),
  finding({
    id: "update-cache",
    name: L("Windows Update indirmeleri", "Windows Update downloads"),
    safety: "safe",
    bytes: 1.3 * GB,
    admin: true,
    location: "C:\\Windows\\SoftwareDistribution\\Download",
    about: L("Kurulmuş güncellemelerin indirilen kurulum dosyaları.", "Downloaded setup files of updates that are already installed."),
    blocked: L(
      "Bir güncelleme yeniden başlatma bekliyor. Önce bilgisayarı yeniden başlatın, yoksa bu adım atlanır.",
      "An update is waiting for a restart. Restart your PC first, or this step is skipped.",
    ),
  }),
  finding({ id: "windows-temp", name: L("Windows geçici dosyaları", "Windows temporary files"), safety: "safe", bytes: 0, measured: false, admin: true, location: "C:\\Windows\\Temp" }),
  finding({
    id: "hibernation",
    name: L("Hazırda bekletme dosyası", "Hibernation file"),
    safety: "review",
    bytes: 12.7 * GB,
    admin: true,
    regenerates: false,
    category: "system",
    location: "C:\\hiberfil.sys",
    about: L("Hazırda bekletme ve Hızlı Başlangıç için belleğin diske yazıldığı dosya.", "Where memory is saved for hibernation and Fast Startup."),
    effect: L("Hazırda bekletme ve Hızlı Başlangıç kapanır.", "Hibernation and Fast Startup are turned off."),
  }),
  finding({
    id: "ollama-models",
    name: L("Ollama modelleri", "Ollama models"),
    safety: "review",
    bytes: 8.4 * GB,
    regenerates: false,
    category: "ai",
    location: "C:\\Users\\demo\\.ollama\\models",
    about: L("Bilgisayarınızda çalışan yapay zekâ modelleri. Her biri birkaç GB olabilir.", "AI models that run on your computer. Each can be several GB."),
    effect: L("Silinen model tekrar kullanılmak istenirse yeniden indirilir.", "A removed model is downloaded again if you use it later."),
    manual: L(
      "Terminalde \"ollama list\" ile modelleri görün, kullanmadıklarınızı \"ollama rm <model>\" ile kaldırın.",
      "Run \"ollama list\" in a terminal, then remove unused ones with \"ollama rm <model>\".",
    ),
  }),
  finding({
    id: "claude-temp",
    name: L("Claude oturum dosyaları", "Claude session files"),
    safety: "review",
    bytes: 1.4 * GB,
    regenerates: false,
    category: "ai",
    location: "C:\\Users\\demo\\AppData\\Local\\Temp\\claude",
    runningApps: ["claude.exe"],
    about: L(
      "Claude Code oturumlarının komut çıktıları, yapıştırılan görseller ve çalışma dosyaları. Açık oturumlar bu dosyaları kullanır.",
      "Command output, pasted images and working files from Claude Code sessions. Open sessions use these files.",
    ),
    effect: L(
      "Claude açıkken temizlenmez. Yalnızca 30 günden uzun süredir kullanılmayan oturum klasörleri Geri Dönüşüm Kutusu'na taşınır; o oturumlara dönerseniz eski çıktılar ve görseller bulunamaz.",
      "Not cleaned while Claude is open. Only session folders unused for more than 30 days go to the Recycle Bin; if you go back to those sessions, their old output and images will be missing.",
    ),
  }),
  finding({
    id: "codex-sessions",
    name: L("Codex oturum kayıtları", "Codex session records"),
    safety: "review",
    bytes: 640 * MB,
    regenerates: false,
    category: "ai",
    location: "C:\\Users\\demo\\.codex\\sessions +1",
    about: L("OpenAI Codex sohbetlerinin kayıtları. Eski sohbetlere dönmek ve devam etmek bunlarla mümkün.", "Records of OpenAI Codex chats. They are what lets you return to and continue old chats."),
    effect: L("Silinen sohbetlere geri dönülemez.", "Deleted chats can't be resumed."),
    manual: L(
      "Kullanmadığınız sohbetleri Codex'in kendisinden silin (terminalde \"codex delete <sohbet kimliği>\"). Dosyaları elle silmek sohbet listesini bozabilir; arşivlemek yer açmaz.",
      "Delete chats you no longer need from Codex itself (\"codex delete <thread id>\" in a terminal). Deleting the files by hand can break the chat list; archiving does not free space.",
    ),
  }),
  finding({
    id: "old-downloads",
    name: L("Eski kurulum dosyaları ve arşivler", "Old installers and archives"),
    safety: "review",
    bytes: 6.2 * GB,
    regenerates: false,
    category: "archive",
    location: "C:\\Users\\demo\\Downloads",
    targets: ["C:\\Users\\demo\\Downloads\\Windows11.iso", "C:\\Users\\demo\\Downloads\\VSCodeSetup.exe"],
    about: L("İndirilenler klasöründe 30 günden eski kurulum dosyaları ve sıkıştırılmış arşivler.", "Installers and compressed archives in Downloads that are older than 30 days."),
    effect: L(
      "Program zaten kuruluysa bir etkisi yok. Arşivlerin içinde ihtiyacınız olan bir şey olmadığından emin olun.",
      "No effect if the program is already installed. Make sure no archive holds something you need.",
    ),
  }),
].sort((a, b) => (a.safety === b.safety ? b.bytes - a.bytes : a.safety === "safe" ? -1 : 1));

const homeTree = build(HOME, 0);
const driveTree = build(DRIVE, 0);
let root = homeTree;

const join = (base: string, name: string) => (base.endsWith("\\") ? base + name : `${base}\\${name}`);

function view(node: MockNode, depth: number): TreeNode {
  const { all, ...rest } = node;
  return { ...rest, children: depth > 0 ? all.map((c) => view(c, depth - 1)) : [] };
}

function filesOf(node: MockNode, path: string, out: FileEntry[]) {
  for (const c of node.all) {
    const p = join(path, c.name);
    if (c.kind === "file") out.push({ name: c.name, path: p, size: c.size, alloc: c.alloc, category: c.category, modified: c.modified });
    else filesOf(c, p, out);
  }
}

let onCleanupProgress: ((id: string) => void) | null = null;
const history: HistoryEntry[] = [];
let onProgress: ((p: ScanProgress) => void) | null = null;
let onDone: ((s: ScanSummary) => void) | null = null;

export const mockApi: Api = {
  locations: async () => ({
    home: "C:\\Users\\demo",
    systemDrive: "C:\\",
    volumes: [
      // Kullanılan alan sahte sürücü ağacıyla aşağı yukarı aynı; "taranmayan" küçük kalsın.
      { root: "C:\\", label: "", fileSystem: "NTFS", total: 256 * GB, free: 74 * GB, removable: false },
      { root: "D:\\", label: L("Oyunlar", "Games"), fileSystem: "NTFS", total: 931 * GB, free: 410 * GB, removable: false },
    ],
    appRoots: ["C:\\Program Files", "C:\\Program Files (x86)", "C:\\Users\\demo\\AppData\\Local\\Programs"],
  }),
  startScan: async (rootPath) => {
    root = /^[a-z]:\\?$/i.test(rootPath) ? driveTree : homeTree;
    const start = Date.now();
    const timer = setInterval(() => {
      const k = Math.min(1, (Date.now() - start) / 1600);
      onProgress?.({ files: Math.round(k * 412_000), dirs: 0, alloc: k * root.alloc, errors: 0, current: join(rootPath, `AppData\\Local\\Temp\\${Math.round(k * 999)}`), elapsedMs: Date.now() - start });
      if (k >= 1) {
        clearInterval(timer);
        onDone?.({ rootPath, elapsedMs: 1600, files: 412_318, dirs: 61_402, size: root.size, alloc: root.alloc, errors: 3, cancelled: false, error: null });
      }
    }, 120);
  },
  cancelScan: async () => {},
  tree: async (path, depth) => {
    let node = root;
    const names: string[] = [];
    for (const i of path) {
      node = node.all[i];
      names.push(node.name);
    }
    return { fullPath: names.reduce(join, root.name), node: view(node, depth) };
  },
  largestFiles: async (limit) => {
    const out: FileEntry[] = [];
    filesOf(root, root.name, out);
    return out.sort((a, b) => b.alloc - a.alloc).slice(0, limit);
  },
  reveal: async () => {},
  analyze: () => new Promise((resolve) => setTimeout(() => resolve(FINDINGS), 700)),
  runCleanup: async (ids, mode) => {
    const outcomes: Outcome[] = [];
    for (const f of FINDINGS.filter((f) => ids.includes(f.id))) {
      onCleanupProgress?.(f.id);
      await new Promise((r) => setTimeout(r, 450));
      outcomes.push(
        f.runningApps.length
          ? { id: f.id, name: f.name, status: "skipped", freed: 0, recycled: false, failedItems: 0, message: `${L("Önce şunları kapatın", "Close these first")}: ${f.runningApps.join(", ")}` }
          : {
              id: f.id,
              name: f.name,
              status: f.admin ? "partial" : "done",
              freed: f.bytes * 0.9,
              recycled: mode === "recycle",
              failedItems: f.admin ? 3 : 0,
              message: f.admin ? L("3 öğe kullanımda olduğu için atlandı", "3 items were skipped because they are in use") : null,
            },
      );
    }
    history.unshift({ time: Date.now(), mode, freed: 0, recycled: outcomes.reduce((s, o) => s + o.freed, 0), items: outcomes });
    return outcomes;
  },
  history: async () => [...history],
  appsForFolder: async (path) => {
    await new Promise((r) => setTimeout(r, 300));
    if (/steam/i.test(path)) return [{ name: "Steam", publisher: "Valve Corporation" }];
    if (/adobe/i.test(path)) return [{ name: "Adobe Photoshop 2025", publisher: "Adobe Inc." }];
    if (/microsoft$/i.test(path))
      return [
        { name: "Microsoft Edge", publisher: "Microsoft Corporation" },
        { name: "Microsoft Edge WebView2 Runtime", publisher: "Microsoft Corporation" },
      ];
    return [];
  },
  openInstalledApps: async () => {},
  openSupportPage: async () => {
    window.open("https://buymeacoffee.com/batuhancakmak", "_blank", "noopener");
  },
  onCleanupProgress: async (cb) => {
    onCleanupProgress = cb;
    return () => {
      if (onCleanupProgress === cb) onCleanupProgress = null;
    };
  },
  // Dinleyici yalnızca hâlâ kendisi kayıtlıysa kaldırılır (StrictMode çift kaydı).
  onProgress: async (cb) => {
    onProgress = cb;
    return () => {
      if (onProgress === cb) onProgress = null;
    };
  },
  onDone: async (cb) => {
    onDone = cb;
    return () => {
      if (onDone === cb) onDone = null;
    };
  },
};
