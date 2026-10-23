//! Temizlik geçmişi: `%LOCALAPPDATA%\SolidClean\history.json`, en yeni başta, en çok 100 kayıt.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::clean::{Mode, Outcome, Status};

const LIMIT: usize = 100;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// Unix zamanı, milisaniye.
    pub time: u64,
    pub mode: Mode,
    /// Diskte hemen açılan alan.
    pub freed: u64,
    /// Geri Dönüşüm Kutusu'na taşınan alan.
    pub recycled: u64,
    pub items: Vec<Outcome>,
}

fn path() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(base).join("SolidClean").join("history.json"))
}

pub fn load() -> Vec<Entry> {
    path()
        .and_then(|p| fs::read(p).ok())
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default()
}

/// Hiçbir şey yapılmadıysa (hepsi atlandıysa) kayıt eklenmez.
pub fn append(mode: Mode, items: &[Outcome]) {
    if items.iter().all(|o| o.status == Status::Skipped) {
        return;
    }
    let Some(path) = path() else { return };
    let entry = Entry {
        time: SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64),
        mode,
        freed: items.iter().filter(|o| !o.recycled).map(|o| o.freed).sum(),
        recycled: items.iter().filter(|o| o.recycled).map(|o| o.freed).sum(),
        items: items.to_vec(),
    };
    let mut all = load();
    all.insert(0, entry);
    all.truncate(LIMIT);
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    // Önce geçici dosyaya yaz, sonra değiştir: yarıda kalan yazma geçmişi bozmasın.
    let tmp = path.with_extension("json.tmp");
    if serde_json::to_vec(&all).is_ok_and(|data| fs::write(&tmp, data).is_ok()) {
        let _ = fs::rename(&tmp, &path);
    }
}
