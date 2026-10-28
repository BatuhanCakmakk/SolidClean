//! Temizlik önerileri: `rules.toml`'daki kuralları bu bilgisayara uygular, boyutları
//! ölçer ve seçilenler için incelenebilir bir PowerShell script'i üretir.
//! Bu modül hiçbir şeyi silmez.

use std::collections::{HashMap, HashSet};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{FindClose, FindFirstFileW, WIN32_FIND_DATAW};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::UI::Shell::{SHQueryRecycleBinW, SHQUERYRBINFO};

use crate::category::Category;
use crate::scanner::{self, flags, Kind as NodeKind, Node, Progress, ScanResult};
use crate::{drives, locale, safety};
pub use crate::locale::Lang;

const RULES: &str = include_str!("../rules.toml");

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Safety {
    Safe,
    Review,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Kind {
    Contents,
    Dir,
    File,
    Files,
    RecycleBin,
}

#[derive(Clone, Deserialize)]
struct Text {
    tr: String,
    en: String,
}

impl Text {
    /// Türkçe ve İngilizce kuralın yanında; diğer diller locales/ altında, eksikse İngilizce.
    fn get(&self, lang: Lang) -> &str {
        if lang == Lang::Tr { &self.tr } else { &self.en }
    }
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Rule {
    pub(crate) id: String,
    safety: Safety,
    category: Category,
    pub(crate) kind: Kind,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    extensions: Vec<String>,
    #[serde(default)]
    pub(crate) older_than_days: u32,
    /// `contents` kurallarında dokunulmayacak alt öğe adları (ör. başka kuralı olan klasörler).
    #[serde(default)]
    pub(crate) exclude: Vec<String>,
    /// Bir güncelleme yeniden başlatma bekliyorsa temizlenmez.
    #[serde(default)]
    pub(crate) wait_for_restart: bool,
    /// Hedeflerde bu kadar günden yeni bir değişiklik varsa öneri "gözden geçir"e düşer
    /// (ör. yakın tarihli bir mavi ekranın dökümü hâlâ incelenebilir).
    #[serde(default)]
    review_if_recent_days: u32,
    #[serde(default)]
    pub(crate) admin: bool,
    #[serde(default)]
    regenerates: bool,
    #[serde(default)]
    pub(crate) close_apps: Vec<String>,
    #[serde(default)]
    pub(crate) services: Vec<String>,
    pub(crate) command: Option<String>,
    command_check: Option<String>,
    name: Text,
    about: Text,
    effect: Text,
    manual: Option<Text>,
}

impl Rule {
    /// Hedefler doğrudan silinir mi? Yalnızca komut çalıştıran
    /// ve elle yapılan kurallar hiçbir şey silmez.
    pub(crate) fn deletes_targets(&self) -> bool {
        !self.is_manual() && !self.command_only() && matches!(self.kind, Kind::Contents | Kind::File | Kind::Files)
    }

    /// Temizliği kullanıcı Windows'un kendi araçlarıyla yapar; uygulama hiçbir şey çalıştırmaz.
    pub(crate) fn is_manual(&self) -> bool {
        self.manual.is_some()
    }

    /// Silme yerine yalnızca bir komut çalıştıran kural (yedek temizliği yok).
    pub(crate) fn command_only(&self) -> bool {
        self.command.is_some() && self.command_check.is_none()
    }

    fn excludes(&self, name: &std::ffi::OsStr) -> bool {
        let name = name.to_string_lossy();
        self.exclude.iter().any(|x| x.eq_ignore_ascii_case(&name))
    }

    /// Kalıcı silme seçilse de Geri Dönüşüm Kutusu'na taşınır: kullanıcı dosyaları ve
    /// yapay zekâ araçlarının oturum/önbellek dosyaları her zaman geri alınabilir kalır.
    pub(crate) fn always_recycles(&self) -> bool {
        self.kind == Kind::Files || self.category == Category::Ai
    }

    /// Kapatılması gereken programlardan şu an açık olanlar (`chrome.exe` biçiminde).
    /// Boşluk içeren adlar (`Gradle daemon`) exe değil, `running_processes`'in eklediği adlardır.
    pub(crate) fn running_apps(&self, running: &HashSet<String>) -> Vec<String> {
        self.close_apps
            .iter()
            .map(|app| if app.contains(' ') { app.clone() } else { format!("{app}.exe") })
            .filter(|exe| running.contains(&exe.to_lowercase()))
            .collect()
    }

    /// Temizliği şu an engelleyen bir durum varsa açıklaması (uygulama açık olması hariç).
    pub(crate) fn blocked(&self, lang: Lang) -> Option<String> {
        (self.wait_for_restart && restart_pending()).then(|| locale::messages(lang).restart_pending.clone())
    }
}

#[derive(Deserialize)]
struct RuleFile {
    rule: Vec<Rule>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub id: String,
    pub name: String,
    pub about: String,
    pub effect: String,
    /// Doluysa temizlik elle yapılır ve script'e girmez.
    pub manual: Option<String>,
    pub category: Category,
    pub safety: Safety,
    pub regenerates: bool,
    /// Listede gösterilecek kısa konum.
    pub location: String,
    pub targets: Vec<String>,
    pub bytes: u64,
    pub items: u64,
    /// false: en az bir hedef okunamadı (çoğunlukla yönetici izni gerekir).
    pub measured: bool,
    pub running_apps: Vec<String>,
    /// Doluysa temizlik şu an yapılamaz (ör. yeniden başlatma bekleniyor); neden bu metindir.
    pub blocked: Option<String>,
}

pub struct Analysis {
    pub findings: Vec<Finding>,
    rules: HashMap<String, Rule>,
    /// Treemap'te taralı gösterim için: küçük harfli tam yol → güvenlik seviyesi.
    pub marks: HashMap<String, Safety>,
}

impl Analysis {
    pub(crate) fn rule(&self, id: &str) -> Option<&Rule> {
        self.rules.get(id)
    }

}

fn load_rules() -> Vec<Rule> {
    toml::from_str::<RuleFile>(RULES).expect("rules.toml geçersiz").rule
}

/// `only` verilirse yalnızca o kurallar değerlendirilir (temizlikten hemen önce
/// hedefleri taze çözmek için).
pub fn analyze(scan: Option<&ScanResult>, lang: Lang, only: Option<&[String]>) -> Analysis {
    let running = running_processes();
    let mut findings = Vec::new();
    let mut rules = HashMap::new();
    let mut marks = HashMap::new();
    for rule in load_rules() {
        if only.is_some_and(|ids| !ids.contains(&rule.id)) {
            continue;
        }
        // Yapay zekâ araçlarının klasörleri, silinecek bir şey olmasa da haritada
        // "gözden geçirilmeli" görünür; genel temizlik (ör. geçici dosyalar) bunlara dokunmaz.
        if rule.category == Category::Ai {
            for path in rule.paths.iter().flat_map(|p| expand(p)).filter(|p| p.exists()) {
                marks.insert(long_path(&path).to_string_lossy().to_lowercase(), Safety::Review);
            }
        }
        let Some(finding) = evaluate(&rule, scan, &running, lang) else { continue };
        for target in &finding.targets {
            marks.insert(target.to_lowercase(), finding.safety);
        }
        findings.push(finding);
        rules.insert(rule.id.clone(), rule);
    }
    findings.sort_by(|a, b| (a.safety as u8).cmp(&(b.safety as u8)).then(b.bytes.cmp(&a.bytes)));
    Analysis { findings, rules, marks }
}

fn evaluate(rule: &Rule, scan: Option<&ScanResult>, running: &HashSet<String>, lang: Lang) -> Option<Finding> {
    let mut targets: Vec<PathBuf> = rule.paths.iter().flat_map(|p| expand(p)).map(|p| long_path(&p)).collect();
    targets.sort();
    targets.dedup();
    let source_dir = targets.first().cloned();
    let (mut bytes, mut items, mut measured) = (0u64, 0u64, true);

    match rule.kind {
        Kind::RecycleBin => (bytes, items) = recycle_bin(),
        Kind::Contents | Kind::Dir => {
            targets.retain(|t| t.is_dir());
            for target in &targets {
                // Yaş sınırı ya da dışlama varsa yalnızca gerçekten silinecek öğeler sayılır.
                let size = if rule.kind == Kind::Contents && (rule.older_than_days > 0 || !rule.exclude.is_empty()) {
                    stale_entries(rule, target, scan).map(|entries| {
                        entries.iter().fold((0, 0), |(b, n), entry| {
                            let (eb, en) = measure_entry(entry, scan);
                            (b + eb, n + en)
                        })
                    })
                } else {
                    measure_dir(target, scan)
                };
                match size {
                    Some((b, n)) => {
                        bytes += b;
                        items += n;
                    }
                    None => measured = false,
                }
            }
        }
        Kind::File => {
            targets.retain(|t| match file_size(t) {
                Some(size) => {
                    bytes += size;
                    items += 1;
                    true
                }
                None => false,
            });
        }
        Kind::Files => {
            let cutoff = SystemTime::now() - Duration::from_secs(u64::from(rule.older_than_days) * 86_400);
            let mut files = Vec::new();
            for dir in &targets {
                let Ok(entries) = std::fs::read_dir(dir) else { continue };
                for entry in entries.flatten() {
                    let Ok(meta) = entry.metadata() else { continue };
                    let path = entry.path();
                    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                    let old = meta.modified().map_or(false, |m| m < cutoff);
                    if meta.is_file() && old && rule.extensions.iter().any(|x| *x == ext) {
                        bytes += meta.len();
                        files.push(path);
                    }
                }
            }
            items = files.len() as u64;
            targets = files;
        }
    }

    if rule.deletes_targets() {
        targets.retain(|t| safety::deletable(&t.to_string_lossy()));
    }
    let nothing_found = if rule.kind == Kind::RecycleBin { bytes == 0 } else { targets.is_empty() };
    if nothing_found || (measured && bytes == 0) {
        return None;
    }
    let recent = SystemTime::now() - Duration::from_secs(u64::from(rule.review_if_recent_days) * 86_400);
    let safety = if rule.review_if_recent_days > 0 && targets.iter().any(|t| changed_since(t, recent, scan)) {
        Safety::Review
    } else {
        rule.safety
    };

    let targets: Vec<String> = targets.iter().map(|t| t.to_string_lossy().into_owned()).collect();
    let location = match rule.kind {
        Kind::RecycleBin => locale::messages(lang).all_drives.clone(),
        Kind::Files => source_dir.map(|d| d.to_string_lossy().into_owned()).unwrap_or_default(),
        _ if targets.len() > 1 => format!("{} +{}", targets[0], targets.len() - 1),
        _ => targets[0].clone(),
    };
    let running_apps = rule.running_apps(running);
    let translated = locale::rule_text(lang, &rule.id);

    Some(Finding {
        id: rule.id.clone(),
        name: translated.and_then(|t| t.name.clone()).unwrap_or_else(|| rule.name.get(lang).to_string()),
        about: translated.and_then(|t| t.about.clone()).unwrap_or_else(|| rule.about.get(lang).to_string()),
        effect: translated.and_then(|t| t.effect.clone()).unwrap_or_else(|| rule.effect.get(lang).to_string()),
        manual: rule.manual.as_ref().map(|m| translated.and_then(|t| t.manual.clone()).unwrap_or_else(|| m.get(lang).to_string())),
        category: rule.category,
        safety,
        regenerates: rule.regenerates,
        location,
        targets,
        bytes,
        items,
        measured,
        running_apps,
        blocked: rule.blocked(lang),
    })
}

/// `%LOCALAPPDATA%\Google\Chrome\User Data\*\Cache` gibi kalıpları gerçek yollara açar.
/// Tanımsız bir ortam değişkeni varsa kalıp hiçbir yola açılmaz.
fn expand(pattern: &str) -> Vec<PathBuf> {
    let mut text = String::new();
    let mut rest = pattern;
    while let Some(start) = rest.find('%') {
        text.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else { return Vec::new() };
        match std::env::var(&after[..end]) {
            Ok(value) if !value.is_empty() => text.push_str(&value),
            _ => return Vec::new(),
        }
        rest = &after[end + 1..];
    }
    text.push_str(rest);

    let mut current = vec![PathBuf::new()];
    for component in text.split('\\').filter(|c| !c.is_empty()) {
        if component.contains('*') {
            let pattern = component.to_lowercase();
            current = current
                .iter()
                .flat_map(|base| {
                    std::fs::read_dir(base)
                        .into_iter()
                        .flatten()
                        .flatten()
                        .filter(|e| e.file_type().map_or(false, |t| t.is_dir()))
                        .filter(|e| wildcard_match(&e.file_name().to_string_lossy().to_lowercase(), &pattern))
                        .map(|e| e.path())
                        .collect::<Vec<_>>()
                })
                .collect();
        } else if component.ends_with(':') {
            for path in &mut current {
                *path = PathBuf::from(format!("{component}\\"));
            }
        } else {
            for path in &mut current {
                path.push(component);
            }
        }
    }
    current
}

/// Yalnızca `*` destekleyen basit eşleştirme.
fn wildcard_match(name: &str, pattern: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return name == pattern;
    }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !name.starts_with(first) {
        return false;
    }
    let mut pos = first.len();
    for part in &parts[1..parts.len() - 1] {
        match name[pos..].find(part) {
            Some(i) => pos += i + part.len(),
            None => return false,
        }
    }
    name.len() >= pos + last.len() && name.ends_with(last)
}

/// 8.3 kısa adları (`CAKAR~1`) uzun adlara çevirir; treemap'teki yollarla eşleşmesi için.
fn long_path(path: &Path) -> PathBuf {
    match std::fs::canonicalize(path) {
        Ok(full) => PathBuf::from(scanner::display_path(&full.to_string_lossy())),
        Err(_) => path.to_path_buf(),
    }
}

fn find_node<'a>(scan: &'a ScanResult, path: &Path) -> Option<&'a Node> {
    let root = scanner::display_path(&scan.root_path).trim_end_matches('\\').to_lowercase();
    let target = path.to_string_lossy().trim_end_matches('\\').to_lowercase();
    let rest = if target == root { "" } else { target.strip_prefix(&root)?.strip_prefix('\\')? };
    let mut node = &scan.root;
    for component in rest.split('\\').filter(|c| !c.is_empty()) {
        node = node.children.iter().find(|c| c.kind == NodeKind::Dir && c.name.to_lowercase() == component)?;
    }
    Some(node)
}

/// `contents` kuralında `dir` içinden silinecek öğeler: dışlananlar ve yaş sınırını geçmeyenler
/// hariç. Klasör okunamazsa None. `scan` verilirse klasör yaşları taranmış ağaçtan okunur;
/// temizlik sırasında taze bilgi için `None` verilir.
pub(crate) fn stale_entries(rule: &Rule, dir: &Path, scan: Option<&ScanResult>) -> Option<Vec<PathBuf>> {
    let cutoff = SystemTime::now() - Duration::from_secs(u64::from(rule.older_than_days) * 86_400);
    let entries = std::fs::read_dir(dir).ok()?;
    Some(
        entries
            .flatten()
            .filter(|e| !rule.excludes(&e.file_name()))
            .map(|e| e.path())
            .filter(|p| rule.older_than_days == 0 || !changed_since(p, cutoff, scan))
            .collect(),
    )
}

/// Öğe `cutoff`tan sonra değişti mi? Klasörün kendi tarihi yalnızca doğrudan içindekiler
/// değişince güncellenir; bu yüzden klasörlerde içlerindeki en yeni değişikliğe bakılır.
/// Yoksa derinde hâlâ kullanılan dosyaları olan bir klasör (ör. açık bir oturumun) eski sanılırdı.
fn changed_since(path: &Path, cutoff: SystemTime, scan: Option<&ScanResult>) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return false };
    if meta.modified().is_ok_and(|t| t >= cutoff) {
        return true;
    }
    if !meta.is_dir() || meta.file_attributes() & 0x400 != 0 {
        return false;
    }
    let newest = match scan.and_then(|s| find_node(s, path)) {
        Some(node) => node.modified,
        None => scanner::scan(&path.to_string_lossy(), 4096, &Progress::default()).root.modified,
    };
    newest >= filetime(cutoff)
}

/// Windows bir güncellemeyi bitirmek için yeniden başlatma bekliyor mu?
fn restart_pending() -> bool {
    [
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired",
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending",
    ]
    .iter()
    .any(|key| crate::apps::machine_key_exists(key))
}

/// Klasör ya da dosya; junction/symlink'in hedefi sayılmaz (silinirken de yalnızca bağlantı gider).
fn measure_entry(path: &Path, scan: Option<&ScanResult>) -> (u64, u64) {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_attributes() & 0x400 != 0 => (0, 1),
        Ok(meta) if meta.is_dir() => measure_dir(path, scan).unwrap_or((0, 0)),
        Ok(meta) => (meta.len(), 1),
        Err(_) => (0, 0),
    }
}

/// FILETIME: 1601'den beri 100 ns aralıklar.
fn filetime(time: SystemTime) -> u64 {
    let unix = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    (unix.as_nanos() / 100) as u64 + 116_444_736_000_000_000
}

/// (diskte kapladığı, dosya sayısı); okunamazsa None. Taranmış ağaçta varsa diske tekrar gidilmez.
fn measure_dir(path: &Path, scan: Option<&ScanResult>) -> Option<(u64, u64)> {
    if let Some(node) = scan.and_then(|s| find_node(s, path)) {
        if node.flags & flags::LINK == 0 {
            return (node.flags & flags::DENIED == 0).then_some((node.alloc, node.files));
        }
    }
    let text = path.to_string_lossy();
    let result = scanner::scan(&text, drives::cluster_size(&text), &Progress::default());
    (result.root.flags & flags::DENIED == 0).then_some((result.root.alloc, result.root.files))
}

/// Kilitli dosyalarda da (hiberfil.sys) çalışır: dosyayı açmadan dizin girdisinden okur.
fn file_size(path: &Path) -> Option<u64> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
    let handle = unsafe { FindFirstFileW(wide.as_ptr(), &mut data) };
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }
    unsafe { FindClose(handle) };
    if data.dwFileAttributes & 0x10 != 0 {
        return None;
    }
    Some(((data.nFileSizeHigh as u64) << 32) | data.nFileSizeLow as u64)
}

fn recycle_bin() -> (u64, u64) {
    let mut info: SHQUERYRBINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHQUERYRBINFO>() as u32;
    let hr = unsafe { SHQueryRecycleBinW(std::ptr::null(), &mut info) };
    if hr < 0 {
        return (0, 0);
    }
    (info.i64Size.max(0) as u64, info.i64NumItems.max(0) as u64)
}

/// Çalışan süreçlerin küçük harfli exe adları.
/// Çalışan programların küçük harfli exe adları. Arka planda çalışan bir Gradle daemon'ı
/// varsa `gradle daemon` adı da eklenir (exe'si sıradan bir `java.exe`).
pub(crate) fn running_processes() -> HashSet<String> {
    let mut names = HashSet::new();
    let mut java_pids = HashSet::new();
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return names;
    }
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    let mut ok = unsafe { Process32FirstW(snapshot, &mut entry) };
    while ok != 0 {
        let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
        let name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();
        if name == "java.exe" || name == "javaw.exe" {
            java_pids.insert(entry.th32ProcessID);
        }
        names.insert(name);
        ok = unsafe { Process32NextW(snapshot, &mut entry) };
    }
    unsafe { CloseHandle(snapshot) };
    let daemons = expand(r"%USERPROFILE%\.gradle\daemon");
    if daemons.iter().any(|dir| gradle_daemon_running(dir, &java_pids)) {
        names.insert("gradle daemon".into());
    }
    names
}

/// Her daemon `~/.gradle/daemon/<sürüm>/daemon-<pid>.out.log` yazar; o pid hâlâ bir Java
/// süreciyse daemon açıktır. Açıkken önbelleği silmek kilitli, yarım dosyalar bırakabilir.
fn gradle_daemon_running(daemon_dir: &Path, java_pids: &HashSet<u32>) -> bool {
    if java_pids.is_empty() {
        return false;
    }
    let Ok(versions) = std::fs::read_dir(daemon_dir) else { return false };
    versions
        .flatten()
        .filter_map(|version| std::fs::read_dir(version.path()).ok())
        .flat_map(|entries| entries.flatten())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_prefix("daemon-")?.strip_suffix(".out.log")?.parse::<u32>().ok()
        })
        .any(|pid| java_pids.contains(&pid))
}

// ------------------------------------------------------------------ script

/// Script'in başındaki yardımcılar. Windows PowerShell 5.1 ile uyumludur.
///
/// `Remove-Item -Recurse` kullanılmaz: Windows PowerShell 5.1'de junction'ların
/// içine girip hedef klasördeki dosyaları silebiliyor. Bunun yerine her öğe tek tek
/// silinir, junction/symlink ise yalnızca bağlantı olarak kaldırılır.
const PRELUDE: &str = r#"$ErrorActionPreference = 'SilentlyContinue'
$IsAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
$script:Freed = 0
$script:Skipped = 0

# Deletes one file or folder. Junctions and symlinks are removed as links only;
# their targets are never entered.
function Remove-Entry([IO.FileSystemInfo]$Item) {
    try {
        if ($Item.Attributes -band [IO.FileAttributes]::ReparsePoint) { $Item.Delete(); return }
        if ($Item -is [IO.DirectoryInfo]) {
            foreach ($child in $Item.GetFileSystemInfos()) { Remove-Entry $child }
            $Item.Attributes = 'Directory'
            $Item.Delete()
        } else {
            $size = $Item.Length
            $Item.Attributes = 'Normal'
            $Item.Delete()
            $script:Freed += $size
        }
    } catch { $script:Skipped++ }
}

# Newest write time in an entry. A folder's own date changes only when its direct
# children change, so its whole contents are checked. Links are not entered.
function Get-Newest([IO.FileSystemInfo]$Item) {
    $newest = $Item.LastWriteTime
    if ($Item -is [IO.DirectoryInfo] -and -not ($Item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        try {
            foreach ($child in $Item.GetFileSystemInfos()) {
                $time = Get-Newest $child
                if ($time -gt $newest) { $newest = $time }
            }
        } catch {}
    }
    return $newest
}

# Empties a folder but keeps the folder itself and the entries named in -Exclude.
function Clear-Folder([string]$Path, [int]$OlderThanDays = 0, [string[]]$Exclude = @()) {
    $dir = New-Object IO.DirectoryInfo $Path
    if (-not $dir.Exists) { return }
    $cutoff = (Get-Date).AddDays(-$OlderThanDays)
    try { $children = $dir.GetFileSystemInfos() } catch { $script:Skipped++; return }
    foreach ($child in $children) {
        if ($Exclude -contains $child.Name) { continue }
        if ($OlderThanDays -eq 0 -or (Get-Newest $child) -lt $cutoff) { Remove-Entry $child }
    }
}

function Remove-File([string]$Path) {
    $file = New-Object IO.FileInfo $Path
    if ($file.Exists) { Remove-Entry $file }
}

function Test-Running([string[]]$Names) {
    return @(Get-Process -Name $Names -ErrorAction SilentlyContinue).Count -gt 0
}
"#;

struct Messages {
    title: &'static str,
    read_first: &'static str,
    permanent: &'static str,
    admin_hint: &'static str,
    admin_needed: &'static str,
    close_first: &'static str,
    freed: &'static str,
    skipped: &'static str,
}

const TR: Messages = Messages {
    title: "SolidClean temizlik script'i",
    read_first: "Çalıştırmadan önce okuyun. Kullanımdaki dosyalar atlanır.",
    permanent: "Silinen dosyalar Geri Dönüşüm Kutusu'na gitmez.",
    admin_hint: "Yönetici gerektiren adımlar için PowerShell'i 'Yönetici olarak çalıştır' ile açın.",
    admin_needed: "yönetici izni gerekiyor, atlandı",
    close_first: "önce şunları kapatın",
    freed: "Boşaltılan alan",
    skipped: "Atlanan öğe (kullanımda veya izin yok)",
};

const EN: Messages = Messages {
    title: "SolidClean cleanup script",
    read_first: "Read it before running. Files in use are skipped.",
    permanent: "Deleted files do not go to the Recycle Bin.",
    admin_hint: "For steps that need an administrator, open PowerShell with 'Run as administrator'.",
    admin_needed: "needs administrator rights, skipped",
    close_first: "close these first",
    freed: "Space freed",
    skipped: "Skipped items (in use or no permission)",
};

/// PowerShell tek tırnaklı dizgisi. PowerShell akıllı tırnakları da tırnak sayar.
fn ps(text: &str) -> String {
    let mut out = String::from("'");
    for c in text.chars() {
        if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}') {
            out.push(c);
        }
        out.push(c);
    }
    out.push('\'');
    out
}

fn ps_list(items: &[String]) -> String {
    format!("@({})", items.iter().map(|i| ps(i)).collect::<Vec<_>>().join(", "))
}

fn human(bytes: u64) -> String {
    const GB: f64 = (1u64 << 30) as f64;
    const MB: f64 = (1u64 << 20) as f64;
    if bytes as f64 >= GB {
        format!("{:.1} GB", bytes as f64 / GB)
    } else {
        format!("{:.0} MB", bytes as f64 / MB)
    }
}

fn indent(open: String, body: Vec<String>) -> Vec<String> {
    let mut lines = vec![open];
    lines.extend(body.into_iter().map(|l| format!("    {l}")));
    lines.push("}".into());
    lines
}

fn block(rule: &Rule, finding: &Finding, m: &Messages) -> String {
    let action: Vec<String> = match rule.kind {
        Kind::Contents => finding
            .targets
            .iter()
            .map(|t| {
                let mut line = format!("Clear-Folder {} -OlderThanDays {}", ps(t), rule.older_than_days);
                if !rule.exclude.is_empty() {
                    line.push_str(&format!(" -Exclude {}", ps_list(&rule.exclude)));
                }
                line
            })
            .collect(),
        Kind::File | Kind::Files => finding.targets.iter().map(|t| format!("Remove-File {}", ps(t))).collect(),
        Kind::RecycleBin => vec!["Clear-RecycleBin -Force".into()],
        Kind::Dir => Vec::new(),
    };

    let mut body = Vec::new();
    if !rule.services.is_empty() {
        body.push(format!("Stop-Service -Name {} -Force", ps_list(&rule.services)));
    }
    match (&rule.command, &rule.command_check) {
        (Some(command), Some(check)) => {
            // Resmî komut varsa onu kullan, yoksa klasörü kendimiz temizleyelim.
            body.push(format!("if (Get-Command {} -ErrorAction SilentlyContinue) {{", ps(check)));
            body.push(format!("    {command}"));
            body.extend(indent("} else {".into(), action));
        }
        (Some(command), None) => body.push(command.clone()),
        (None, _) => body.extend(action),
    }
    if !rule.services.is_empty() {
        body.push(format!("Start-Service -Name {}", ps_list(&rule.services)));
    }

    let name = &finding.name;
    if !rule.close_apps.is_empty() {
        let apps: Vec<String> = rule.close_apps.iter().map(|a| format!("{a}.exe")).collect();
        let warning = format!("{name}: {} ({})", m.close_first, apps.join(", "));
        body = indent(
            format!("if (Test-Running {}) {{ Write-Warning {} }} else {{", ps_list(&rule.close_apps), ps(&warning)),
            body,
        );
    }
    if rule.admin {
        let warning = format!("{name}: {}", m.admin_needed);
        body = indent(format!("if (-not $IsAdmin) {{ Write-Warning {} }} else {{", ps(&warning)), body);
    }

    let mut out = format!("\n# --- {name} ({}) ---\nWrite-Host {}\n", human(finding.bytes), ps(&format!("> {name}")));
    for line in body {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// Seçilen önerilerden PowerShell script'i üretir. Elle yapılan kurallar atlanır.
pub fn script(analysis: &Analysis, ids: &[String], lang: Lang) -> String {
    // Script geliştirici aracı; Türkçe dışındaki dillerde İngilizce yazılır.
    let m = if lang == Lang::Tr { &TR } else { &EN };
    let mut out = format!(
        "# {}\n# {}\n# {}\n# {}\n\n{PRELUDE}",
        m.title, m.read_first, m.permanent, m.admin_hint
    );
    for finding in analysis.findings.iter().filter(|f| ids.contains(&f.id) && f.manual.is_none()) {
        if let Some(rule) = analysis.rules.get(&finding.id) {
            out.push_str(&block(rule, finding, m));
        }
    }
    out.push_str(&format!(
        "\nWrite-Host ''\nWrite-Host ({} -f ($script:Freed / 1GB))\nif ($script:Skipped -gt 0) {{ Write-Host ({} + $script:Skipped) }}\n",
        ps(&format!("{}: {{0:N2}} GB", m.freed)),
        ps(&format!("{}: ", m.skipped)),
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    #[test]
    fn every_rule_is_translated() {
        let rules = load_rules();
        for (lang, code) in locale::all().filter(|(l, _)| !matches!(l, Lang::Tr | Lang::En)) {
            for id in locale::rule_ids(lang) {
                assert!(rules.iter().any(|r| r.id == id), "{code}: bilinmeyen kural {id}");
            }
            for rule in &rules {
                let text = locale::rule_text(lang, &rule.id).unwrap_or_else(|| panic!("{code}: {} çevrilmemiş", rule.id));
                assert!(text.name.is_some() && text.about.is_some() && text.effect.is_some(), "{code}: {}", rule.id);
                assert_eq!(text.manual.is_some(), rule.manual.is_some(), "{code}: {} manual", rule.id);
            }
        }
    }

    #[test]
    fn rules_are_well_formed() {
        let rules = load_rules();
        let mut ids = HashSet::new();
        for rule in &rules {
            assert!(ids.insert(rule.id.clone()), "tekrarlanan kural: {}", rule.id);
            assert!(!rule.name.tr.is_empty() && !rule.name.en.is_empty(), "{}", rule.id);
            if rule.kind == Kind::Dir {
                assert!(rule.manual.is_some(), "{}: dir kuralları elle yapılmalı", rule.id);
            }
            // Store paketi yetki yükseltemez (allowElevation yok): yönetici gerektiren temizlik
            // kullanıcıya Windows'un kendi araçlarıyla yaptırılır.
            if rule.admin {
                assert!(rule.manual.is_some(), "{}: yönetici gerektiren kurallar elle yapılmalı", rule.id);
            }
            if !rule.exclude.is_empty() {
                assert_eq!(rule.kind, Kind::Contents, "{}: exclude yalnızca contents kurallarında", rule.id);
            }
            if rule.category == Category::Ai {
                assert_eq!(rule.safety, Safety::Review, "{}: yapay zekâ araçları her zaman gözden geçirilir", rule.id);
                if rule.deletes_targets() {
                    assert!(rule.older_than_days >= 30, "{}: en az 30 gün", rule.id);
                    assert!(rule.always_recycles(), "{}", rule.id);
                }
            }
            if rule.kind == Kind::RecycleBin {
                assert!(rule.paths.is_empty(), "{}", rule.id);
            } else {
                assert!(!rule.paths.is_empty(), "{}", rule.id);
            }
        }
    }

    /// Kural dosyasındaki bir hata koruma listesine takılmalı; ama doğru yazılmış
    /// kuralların hedefleri takılmamalı. Burada ikincisini doğruluyoruz.
    #[test]
    fn rule_targets_pass_the_safety_guard() {
        for rule in load_rules().iter().filter(|r| r.deletes_targets()) {
            for pattern in &rule.paths {
                for path in expand(pattern) {
                    let path = path.to_string_lossy().into_owned();
                    let checked = if rule.kind == Kind::Files { format!("{path}\\ornek.exe") } else { path };
                    assert!(safety::deletable(&checked), "{}: {checked}", rule.id);
                }
            }
        }
    }

    /// Genel kurallar (ör. geçici dosyalar) bir yapay zekâ aracının klasörünü kapsıyorsa onu
    /// `exclude` ile dışarıda bırakmalı; yoksa açık oturumların dosyaları "güvenli" diye silinirdi.
    #[test]
    fn generic_rules_leave_ai_folders_alone() {
        let rules = load_rules();
        let ai_paths: Vec<String> = rules
            .iter()
            .filter(|r| r.category == Category::Ai)
            .flat_map(|r| r.paths.iter().map(|p| p.to_lowercase()))
            .collect();
        for rule in rules.iter().filter(|r| r.category != Category::Ai && r.deletes_targets()) {
            for parent in rule.paths.iter().map(|p| p.to_lowercase()) {
                for ai in &ai_paths {
                    let Some(rest) = ai.strip_prefix(&format!("{parent}\\")) else { continue };
                    let first = rest.split('\\').next().unwrap();
                    assert!(
                        rule.exclude.iter().any(|x| x.eq_ignore_ascii_case(first)),
                        "{}: {ai} dışarıda bırakılmalı",
                        rule.id
                    );
                }
            }
        }
    }

    fn set_modified(path: &Path, time: SystemTime) {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_WRITE_ATTRIBUTES: u32 = 0x100;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        let file = fs::OpenOptions::new()
            .access_mode(FILE_WRITE_ATTRIBUTES)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)
            .unwrap();
        file.set_modified(time).unwrap();
    }

    fn test_rule(fields: &str) -> Rule {
        toml::from_str(&format!(
            "id = \"test\"\ncategory = \"cache\"\nname = {{ tr = \"t\", en = \"t\" }}\nabout = {{ tr = \"t\", en = \"t\" }}\neffect = {{ tr = \"t\", en = \"t\" }}\n{fields}"
        ))
        .unwrap()
    }

    /// Yakın tarihli bir döküm hâlâ incelenebilir: "safe" kural "review" önerisine döner.
    #[test]
    fn recent_dump_needs_review() {
        let dir = std::env::temp_dir().join(format!("solidclean-dump-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let dump = dir.join("MEMORY.DMP");
        fs::write(&dump, b"dump").unwrap();
        let rule = test_rule(&format!(
            "safety = \"safe\"\nkind = \"file\"\nreview_if_recent_days = 30\npaths = ['{}']",
            dump.display()
        ));
        let safety_now = evaluate(&rule, None, &HashSet::new(), Lang::En).map(|f| f.safety);
        set_modified(&dump, SystemTime::now() - Duration::from_secs(60 * 86_400));
        let safety_old = evaluate(&rule, None, &HashSet::new(), Lang::En).map(|f| f.safety);
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(safety_now, Some(Safety::Review));
        assert_eq!(safety_old, Some(Safety::Safe));
    }

    #[test]
    fn gradle_daemon_counts_as_running_app() {
        let rule = test_rule("safety = \"safe\"\nkind = \"contents\"\npaths = ['x']\nclose_apps = [\"studio64\", \"Gradle daemon\"]");
        let running: HashSet<String> = ["gradle daemon".to_string(), "explorer.exe".to_string()].into();
        assert_eq!(rule.running_apps(&running), ["Gradle daemon"]);
        let running: HashSet<String> = ["studio64.exe".to_string()].into();
        assert_eq!(rule.running_apps(&running), ["studio64.exe"]);
    }

    #[test]
    fn gradle_daemon_is_found_by_its_log_pid() {
        let dir = std::env::temp_dir().join(format!("solidclean-gradle-{}", std::process::id()));
        fs::create_dir_all(dir.join("8.13")).unwrap();
        fs::write(dir.join("8.13").join("daemon-4242.out.log"), b"").unwrap();
        fs::write(dir.join("8.13").join("registry.bin"), b"").unwrap();
        let alive = gradle_daemon_running(&dir, &[4242].into());
        let stale = gradle_daemon_running(&dir, &[1111].into());
        let _ = fs::remove_dir_all(&dir);
        assert!(alive);
        assert!(!stale);
    }

    /// Klasörün kendi tarihi eski olsa da içinde yeni bir dosya varsa klasör silinmez
    /// (ör. açık bir oturumun çalışma klasörü); dışlanan öğelere hiç dokunulmaz.
    #[test]
    fn stale_entries_skip_excluded_and_recently_used_folders() {
        let rule: Rule = toml::from_str(
            r#"
            id = "test"
            safety = "safe"
            category = "cache"
            kind = "contents"
            paths = ['%TEMP%']
            older_than_days = 1
            exclude = ['Keep']
            name = { tr = "t", en = "t" }
            about = { tr = "t", en = "t" }
            effect = { tr = "t", en = "t" }
            "#,
        )
        .unwrap();
        let root = std::env::temp_dir().join(format!("solidclean-age-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let week_ago = SystemTime::now() - Duration::from_secs(7 * 86_400);
        for name in ["active", "stale", "keep"] {
            fs::create_dir_all(root.join(name).join("deep")).unwrap();
        }
        fs::write(root.join("active").join("deep").join("fresh.txt"), b"x").unwrap();
        for name in ["stale", "keep"] {
            let file = root.join(name).join("deep").join("old.txt");
            fs::write(&file, b"y").unwrap();
            set_modified(&file, week_ago);
        }
        for name in ["active", "stale", "keep"] {
            set_modified(&root.join(name).join("deep"), week_ago);
            set_modified(&root.join(name), week_ago);
        }

        let stale = stale_entries(&rule, &root, None).unwrap();
        let _ = fs::remove_dir_all(&root);

        let names: Vec<String> = stale.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["stale"]);
    }

    #[test]
    fn wildcard() {
        assert!(wildcard_match("default", "*"));
        assert!(wildcard_match("profile 1", "profile*"));
        assert!(wildcard_match("abc.default-release", "*.default*"));
        assert!(!wildcard_match("cache", "code*"));
        assert!(wildcard_match("cache", "cache"));
    }

    #[test]
    fn quotes_powershell_strings() {
        assert_eq!(ps("C:\\a'b"), "'C:\\a''b'");
        assert_eq!(ps("x\u{2019}y"), "'x\u{2019}\u{2019}y'");
    }

    /// Asıl güvenlik testi: üretilen script, temizlenen klasördeki bir junction'ın
    /// hedefine dokunmamalı.
    #[test]
    fn script_never_follows_junctions() {
        let root = std::env::temp_dir().join(format!("solidclean-script-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let clean = root.join("clean");
        let keep = root.join("keep");
        fs::create_dir_all(clean.join("sub")).unwrap();
        fs::create_dir_all(&keep).unwrap();
        fs::write(clean.join("a.tmp"), b"x").unwrap();
        fs::write(clean.join("sub").join("b.tmp"), b"y").unwrap();
        fs::write(keep.join("onemli.txt"), b"degerli").unwrap();
        let ok = Command::new("cmd").args(["/C", "mklink", "/J"]).arg(clean.join("link")).arg(&keep).output().unwrap();
        assert!(ok.status.success());

        let script = format!("{PRELUDE}\nClear-Folder {}\n", ps(&clean.to_string_lossy()));
        let out = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .output()
            .unwrap();

        let kept = keep.join("onemli.txt").exists();
        let remaining: Vec<_> = fs::read_dir(&clean).unwrap().collect();
        let _ = fs::remove_dir_all(&root);

        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert!(kept, "junction hedefindeki dosya silindi!");
        assert!(remaining.is_empty(), "klasör boşaltılmadı");
    }
}
