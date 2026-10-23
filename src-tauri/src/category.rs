//! Treemap renklendirmesi için kaba kategori tahmini.
//!
//! Bu yalnızca görsel bir ipucudur, "silinebilir" anlamına gelmez. Temizlik
//! kararları ileride ayrı ve açık kurallarla verilecek.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Media,
    Apps,
    Games,
    System,
    Cache,
    Archive,
    Dev,
    Ai,
    Other,
}

use Category::*;

const CACHE_DIRS: &[&str] = &[
    "temp", "tmp", "cache", "caches", "cache2", "cache_data", "cachestorage", "inetcache",
    "code cache", "gpucache", "dawncache", "dawngraphitecache", "dawnwebgpucache", "grshadercache",
    "shadercache", "d3dscache", "dxcache", "glcache", "nv_cache", "crashdumps", "npm-cache",
    "cacheddata", "cachedextensionvsixs", "softwaredistribution", "deliveryoptimization",
    "webcache", ".cache",
];
const AI_DIRS: &[&str] = &[".ollama", "ollama", "huggingface", ".lmstudio", "lm-studio", "lm studio"];
const DEV_DIRS: &[&str] = &[
    ".npm", ".gradle", ".m2", ".nuget", ".cargo", ".rustup", "node_modules", ".pnpm-store",
    ".yarn", ".vscode", ".vscode-insiders", ".android", "android", ".docker", "docker", ".conda",
    "anaconda3", "miniconda3", ".venv", "venv", "__pycache__", ".vs", ".next", ".turbo",
];
const GAME_DIRS: &[&str] = &[
    "steam", "steamapps", "steamlibrary", "epic games", "riot games", "xboxgames", "gog galaxy",
    "gog games", "battle.net", "ea games", "ubisoft game launcher", "rockstar games", ".minecraft",
];
const APP_DIRS: &[&str] = &["appdata", "programs", "windowsapps"];
const MEDIA_DIRS: &[&str] = &["documents", "pictures", "videos", "music", "camera roll", "saved pictures", "screenshots"];

const MEDIA_EXT: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "heic", "webp", "bmp", "tif", "tiff", "raw", "cr2", "cr3", "nef",
    "arw", "dng", "psd", "mp4", "mkv", "mov", "avi", "wmv", "m4v", "webm", "mts", "mp3", "flac",
    "wav", "m4a", "aac", "ogg", "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "epub",
];
const ARCHIVE_EXT: &[&str] = &["iso", "zip", "rar", "7z", "tar", "gz", "exe", "msi", "msix", "appx", "img"];

/// `level`: sürücü kökünden derinlik (`C:\Windows` için 1).
pub fn classify_dir(name: &str, parent: Category, level: usize) -> Category {
    let n = name.to_lowercase();
    let n = n.as_str();
    if level == 1 {
        match n {
            "windows" | "windows.old" | "system volume information" | "recovery" | "$winreagent"
            | "$windows.~bt" | "$windows.~ws" | "$sysreset" => return System,
            "program files" | "program files (x86)" | "programdata" => return Apps,
            "$recycle.bin" => return Cache,
            _ => {}
        }
    }
    // Yapay zekâ modelleri `.cache` altında dursa bile önbellek değildir: yeniden indirmek pahalı.
    if AI_DIRS.contains(&n) {
        return Ai;
    }
    if parent == Cache || CACHE_DIRS.contains(&n) {
        return Cache;
    }
    if DEV_DIRS.contains(&n) {
        return Dev;
    }
    if GAME_DIRS.contains(&n) {
        return Games;
    }
    if APP_DIRS.contains(&n) {
        return Apps;
    }
    if parent == Other && MEDIA_DIRS.contains(&n) {
        return Media;
    }
    parent
}

pub fn classify_file(name: &str, parent: Category, level: usize) -> Category {
    let n = name.to_lowercase();
    if level == 1 && matches!(n.as_str(), "pagefile.sys" | "hiberfil.sys" | "swapfile.sys" | "dumpstack.log") {
        return System;
    }
    if !matches!(parent, Other | Media) {
        return parent;
    }
    let Some((_, ext)) = n.rsplit_once('.') else { return parent };
    if MEDIA_EXT.contains(&ext) {
        Media
    } else if ARCHIVE_EXT.contains(&ext) {
        Archive
    } else if matches!(ext, "vhd" | "vhdx") {
        Dev
    } else if matches!(ext, "dmp" | "tmp") {
        Cache
    } else {
        parent
    }
}

/// Tarama kökünün kategorisini ve sürücü kökünden derinliğini yol bileşenlerinden çıkarır.
pub fn for_path(path: &str) -> (Category, usize) {
    let path = crate::scanner::display_path(path);
    let mut category = Other;
    let mut level = 0;
    // İlk bileşen sürücü harfidir (`C:`).
    for (i, component) in path.split('\\').filter(|c| !c.is_empty()).enumerate().skip(1) {
        level = i;
        category = classify_dir(component, category, level);
    }
    (category, level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_locations() {
        assert_eq!(for_path(r"C:\"), (Other, 0));
        assert_eq!(for_path(r"C:\Users\ali"), (Other, 2));
        assert_eq!(for_path(r"C:\Windows\SoftwareDistribution").0, Cache);
        assert_eq!(for_path(r"C:\Windows\Installer").0, System);
        assert_eq!(for_path(r"C:\Users\ali\AppData\Local\Temp").0, Cache);
        assert_eq!(for_path(r"C:\Users\ali\.cache\huggingface").0, Ai);
        assert_eq!(for_path(r"C:\Users\ali\Documents\proje\node_modules").0, Dev);
        assert_eq!(for_path(r"C:\Users\ali\Pictures").0, Media);
        assert_eq!(classify_file("kurulum.EXE", Other, 3), Archive);
        assert_eq!(classify_file("tatil.mp4", Other, 3), Media);
        assert_eq!(classify_file("pagefile.sys", Other, 1), System);
        assert_eq!(classify_file("oyun.exe", Games, 4), Games);
    }
}
