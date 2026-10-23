//! Paralel dosya sistemi tarayıcısı.
//!
//! `FindFirstFileExW` ile her klasörü tek sistem çağrısı dizisiyle okur (boyut ve
//! öznitelikler dizin girdisinden gelir, dosya başına ek çağrı yapılmaz) ve alt
//! klasörleri rayon ile paralel tarar.
//!
//! Güvenlik ve doğruluk kuralları:
//! - Junction/symlink klasörlerinin içine girilmez: döngü ve çift sayım olmaz.
//! - Buluttaki (OneDrive vb.) yer tutucu dosyalar okunmaz, diskte 0 bayt sayılır.
//! - Yollar `\\?\` önekiyle açılır, 260 karakter sınırına takılmaz.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::Mutex;
use std::time::Instant;

use rayon::prelude::*;
use windows_sys::Win32::Foundation::{GetLastError, FILETIME, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    FindClose, FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW,
    GetCompressedFileSizeW, FIND_FIRST_EX_LARGE_FETCH, WIN32_FIND_DATAW,
};

const ATTR_DIRECTORY: u32 = 0x10;
const ATTR_SPARSE: u32 = 0x200;
const ATTR_REPARSE_POINT: u32 = 0x400;
const ATTR_COMPRESSED: u32 = 0x800;
const ATTR_OFFLINE: u32 = 0x1000;
const ATTR_RECALL_ON_OPEN: u32 = 0x4_0000;
const ATTR_RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;

const SEP: u16 = b'\\' as u16;

/// Bu boyutun altındaki dosyalar tek tek tutulmaz, klasör başına tek düğümde toplanır.
/// Milyonlarca küçük dosyada belleği makul tutar.
const SMALL_FILE_LIMIT: u64 = 1 << 20;

pub mod flags {
    /// Dosya bulutta, diskte yer kaplamıyor.
    pub const CLOUD: u8 = 1;
    /// Junction/symlink; hedefi taranmadı.
    pub const LINK: u8 = 2;
    /// Klasör okunamadı (izin yok vb.).
    pub const DENIED: u8 = 4;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Dir,
    File,
    /// Bir klasördeki küçük dosyaların toplamı.
    SmallFiles,
}

pub struct Node {
    pub name: Box<str>,
    pub kind: Kind,
    pub flags: u8,
    /// Mantıksal boyut (dosya içeriği).
    pub size: u64,
    /// Diskte kapladığı alan (küme boyutuna yuvarlanmış, bulut dosyaları 0).
    pub alloc: u64,
    pub files: u64,
    pub dirs: u64,
    /// Alt ağaçtaki en yeni değişiklik zamanı (FILETIME).
    pub modified: u64,
    /// Diskte kapladığı alana göre büyükten küçüğe sıralı.
    pub children: Vec<Node>,
}

impl Node {
    fn new(name: Box<str>, kind: Kind) -> Self {
        Node { name, kind, flags: 0, size: 0, alloc: 0, files: 0, dirs: 0, modified: 0, children: Vec::new() }
    }
}

/// Taramanın canlı durumu; arayüze düzenli aralıklarla gönderilir.
#[derive(Default)]
pub struct Progress {
    pub cancel: AtomicBool,
    pub files: AtomicU64,
    pub dirs: AtomicU64,
    pub alloc: AtomicU64,
    pub errors: AtomicU64,
    pub current: Mutex<String>,
}

pub struct ScanResult {
    pub root_path: String,
    pub root: Node,
    pub elapsed_ms: u64,
    pub errors: u64,
    pub cancelled: bool,
}

struct Ctx<'a> {
    progress: &'a Progress,
    cluster: u64,
}

pub fn scan(root_path: &str, cluster: u64, progress: &Progress) -> ScanResult {
    let start = Instant::now();
    let wide = extended_path(root_path);
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8).clamp(4, 32);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .stack_size(16 << 20)
        .build()
        .expect("tarama iş parçacıkları oluşturulamadı");
    let ctx = Ctx { progress, cluster: cluster.max(512) };
    let root = pool.install(|| scan_dir(&ctx, &wide, root_path.into()));
    finish_result(root_path, root, start, progress)
}

fn finish_result(root_path: &str, root: Node, start: Instant, progress: &Progress) -> ScanResult {
    ScanResult {
        root_path: root_path.to_string(),
        root,
        elapsed_ms: start.elapsed().as_millis() as u64,
        errors: progress.errors.load(Relaxed),
        cancelled: progress.cancel.load(Relaxed),
    }
}

/// `C:\Users` → `\\?\C:\Users`, `\\sunucu\pay` → `\\?\UNC\sunucu\pay` (sondaki `\` olmadan).
fn extended_path(path: &str) -> Vec<u16> {
    let trimmed = path.trim_end_matches('\\');
    let full = if trimmed.starts_with(r"\\?\") {
        trimmed.to_string()
    } else if let Some(unc) = trimmed.strip_prefix(r"\\") {
        format!(r"\\?\UNC\{unc}")
    } else {
        format!(r"\\?\{trimmed}")
    };
    full.encode_utf16().collect()
}

pub fn display_path(path: &str) -> &str {
    path.strip_prefix(r"\\?\").unwrap_or(path)
}

fn scan_dir(ctx: &Ctx, path: &[u16], name: Box<str>) -> Node {
    let mut node = Node::new(name, Kind::Dir);
    if ctx.progress.cancel.load(Relaxed) {
        return node;
    }

    let mut pattern = Vec::with_capacity(path.len() + 3);
    pattern.extend_from_slice(path);
    pattern.extend_from_slice(&[SEP, b'*' as u16, 0]);

    let mut data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
    let handle = unsafe {
        FindFirstFileExW(
            pattern.as_ptr(),
            FindExInfoBasic,
            &mut data as *mut WIN32_FIND_DATAW as *mut _,
            FindExSearchNameMatch,
            std::ptr::null(),
            FIND_FIRST_EX_LARGE_FETCH,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        node.flags |= flags::DENIED;
        ctx.progress.errors.fetch_add(1, Relaxed);
        return node;
    }

    let mut subdirs: Vec<Vec<u16>> = Vec::new();
    let mut small = Node::new("".into(), Kind::SmallFiles);
    let (mut local_files, mut local_alloc) = (0u64, 0u64);

    loop {
        let len = data.cFileName.iter().position(|&c| c == 0).unwrap_or(data.cFileName.len());
        let wname = &data.cFileName[..len];
        if !is_dot_entry(wname) {
            let attrs = data.dwFileAttributes;
            let modified = filetime(&data.ftLastWriteTime);
            if attrs & ATTR_DIRECTORY != 0 {
                if attrs & ATTR_REPARSE_POINT != 0 && !is_cloud_tag(data.dwReserved0) {
                    let mut link = Node::new(String::from_utf16_lossy(wname).into(), Kind::Dir);
                    link.flags = flags::LINK;
                    link.modified = modified;
                    node.children.push(link);
                } else {
                    subdirs.push(wname.to_vec());
                }
            } else {
                let size = ((data.nFileSizeHigh as u64) << 32) | data.nFileSizeLow as u64;
                let mut file_flags = 0;
                let alloc = if attrs & (ATTR_OFFLINE | ATTR_RECALL_ON_DATA_ACCESS | ATTR_RECALL_ON_OPEN) != 0 {
                    file_flags |= flags::CLOUD;
                    0
                } else if attrs & (ATTR_COMPRESSED | ATTR_SPARSE) != 0 {
                    compressed_size(path, wname).unwrap_or_else(|| round_up(size, ctx.cluster))
                } else {
                    round_up(size, ctx.cluster)
                };
                local_files += 1;
                local_alloc += alloc;
                if size >= SMALL_FILE_LIMIT || alloc >= SMALL_FILE_LIMIT {
                    let mut file = Node::new(String::from_utf16_lossy(wname).into(), Kind::File);
                    file.flags = file_flags;
                    file.size = size;
                    file.alloc = alloc;
                    file.files = 1;
                    file.modified = modified;
                    node.children.push(file);
                } else {
                    small.size += size;
                    small.alloc += alloc;
                    small.files += 1;
                    small.flags |= file_flags;
                    small.modified = small.modified.max(modified);
                }
            }
        }
        if unsafe { FindNextFileW(handle, &mut data) } == 0 {
            break;
        }
    }
    unsafe { FindClose(handle) };

    ctx.progress.files.fetch_add(local_files, Relaxed);
    ctx.progress.alloc.fetch_add(local_alloc, Relaxed);
    if ctx.progress.dirs.fetch_add(1, Relaxed) % 64 == 0 {
        if let Ok(mut current) = ctx.progress.current.try_lock() {
            *current = String::from_utf16_lossy(path);
        }
    }

    let scanned: Vec<Node> = subdirs
        .into_par_iter()
        .map(|wname| {
            let mut child = Vec::with_capacity(path.len() + 1 + wname.len());
            child.extend_from_slice(path);
            child.push(SEP);
            child.extend_from_slice(&wname);
            scan_dir(ctx, &child, String::from_utf16_lossy(&wname).into())
        })
        .collect();
    node.children.extend(scanned);
    if small.files > 0 {
        node.children.push(small);
    }
    summarize(&mut node);
    node
}

/// Alt öğelerin toplamlarını düğüme yazar ve alt öğeleri büyükten küçüğe sıralar.
fn summarize(node: &mut Node) {
    for child in &node.children {
        node.size += child.size;
        node.alloc += child.alloc;
        node.files += child.files;
        node.dirs += child.dirs + (child.kind == Kind::Dir) as u64;
        node.modified = node.modified.max(child.modified);
    }
    node.children.sort_unstable_by(|a, b| b.alloc.cmp(&a.alloc).then(b.size.cmp(&a.size)));
    node.children.shrink_to_fit();
}

fn is_dot_entry(name: &[u16]) -> bool {
    const DOT: u16 = b'.' as u16;
    name == [DOT] || name == [DOT, DOT]
}

/// Bulut sağlayıcılarının (OneDrive, iCloud, Dropbox) yer tutucu klasörleri.
/// Bunlar gerçek klasördür, içine girilmesi güvenlidir.
fn is_cloud_tag(tag: u32) -> bool {
    tag & 0xFFFF_0FFF == 0x9000_001A
}

fn filetime(ft: &FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64
}

fn round_up(size: u64, cluster: u64) -> u64 {
    size.div_ceil(cluster) * cluster
}

fn compressed_size(dir: &[u16], name: &[u16]) -> Option<u64> {
    let mut full = Vec::with_capacity(dir.len() + name.len() + 2);
    full.extend_from_slice(dir);
    full.push(SEP);
    full.extend_from_slice(name);
    full.push(0);
    let mut high = 0u32;
    let low = unsafe { GetCompressedFileSizeW(full.as_ptr(), &mut high) };
    if low == u32::MAX && unsafe { GetLastError() } != 0 {
        return None;
    }
    Some(((high as u64) << 32) | low as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    #[test]
    fn counts_files_and_does_not_follow_junctions() {
        let dir = std::env::temp_dir().join(format!("solidclean-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("a").join("b")).unwrap();
        fs::write(dir.join("a").join("one.bin"), vec![0u8; 3000]).unwrap();
        fs::write(dir.join("a").join("b").join("two.bin"), vec![0u8; 2 << 20]).unwrap();
        // Kendi köküne işaret eden junction: takip edilseydi sonsuz döngü olurdu.
        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(dir.join("loop"))
            .arg(&dir)
            .output()
            .unwrap()
            .status;
        assert!(status.success());

        let progress = Progress::default();
        let result = scan(dir.to_str().unwrap(), 4096, &progress);
        let root = &result.root;

        // Junction'ı silmek yalnızca bağlantıyı kaldırır, hedefe dokunmaz.
        fs::remove_dir(dir.join("loop")).unwrap();
        fs::remove_dir_all(&dir).unwrap();

        assert_eq!(root.files, 2);
        assert_eq!(root.size, 3000 + (2 << 20));
        assert_eq!(root.alloc, 4096 + (2 << 20));
        assert_eq!(result.errors, 0);
        let link = root.children.iter().find(|c| &*c.name == "loop").unwrap();
        assert_eq!(link.flags & flags::LINK, flags::LINK);
        assert_eq!(link.size, 0);
        // En büyük alt öğe başta.
        assert_eq!(&*root.children[0].name, "a");
    }
}
