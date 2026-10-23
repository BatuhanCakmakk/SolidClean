//! Seçilen önerileri uygular: öğeleri Geri Dönüşüm Kutusu'na taşır veya kalıcı olarak siler.
//!
//! Güvenlik kuralları:
//! - Hedefler temizlikten hemen önce yeniden çözülür (eski tarama verisine güvenilmez).
//! - Silinen her öğe `safety::deletable`'dan geçer.
//! - Junction/symlink'lerin içine girilmez; yalnızca bağlantının kendisi kaldırılır.
//! - İndirilenler gibi kullanıcı dosyaları (`files` kuralları) her zaman Geri Dönüşüm Kutusu'na gider.

use std::collections::HashSet;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::MetadataExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use windows_sys::Win32::UI::Shell::{SHEmptyRecycleBinW, SHFileOperationW, SHFILEOPSTRUCTW};

use crate::cleanup::{running_processes, stale_entries, Analysis, Finding, Kind, Lang, Rule};
use crate::scanner::{self, flags, Progress};
use crate::{drives, locale, safety};

const FO_DELETE: u32 = 3;
const FOF_SILENT: u16 = 0x4;
const FOF_NOCONFIRMATION: u16 = 0x10;
const FOF_ALLOWUNDO: u16 = 0x40;
const FOF_NOERRORUI: u16 = 0x400;
/// Dosya Geri Dönüşüm Kutusu'na sığmıyorsa kalıcı silmeden önce Windows sorar.
const FOF_WANTNUKEWARNING: u16 = 0x4000;
const SHERB_QUIET: u32 = 0x1 | 0x2 | 0x4;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const ATTR_DIRECTORY: u32 = 0x10;
const ATTR_REPARSE_POINT: u32 = 0x400;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Geri Dönüşüm Kutusu'na taşı: geri alınabilir, alan kutu boşaltılınca açılır.
    Recycle,
    /// Kalıcı sil: alan hemen açılır.
    Permanent,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Done,
    /// Bir kısmı temizlendi, bazı öğeler kullanımda olduğu için atlandı.
    Partial,
    Skipped,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub id: String,
    pub name: String,
    pub status: Status,
    /// Temizlenen bayt.
    pub freed: u64,
    /// true: `freed` Geri Dönüşüm Kutusu'na taşındı, diskte henüz yer açılmadı.
    pub recycled: bool,
    /// Kullanımda olduğu ya da izin olmadığı için silinemeyen öğe sayısı.
    pub failed_items: u64,
    pub message: Option<String>,
}

impl Outcome {
    pub fn skipped(finding: &Finding, message: String) -> Self {
        Outcome {
            id: finding.id.clone(),
            name: finding.name.clone(),
            status: Status::Skipped,
            freed: 0,
            recycled: false,
            failed_items: 0,
            message: Some(message),
        }
    }
}

/// `include` true döndüren önerileri sırayla uygular. `on_start` her öneri başlarken çağrılır.
pub fn execute(
    analysis: &Analysis,
    mode: Mode,
    lang: Lang,
    include: impl Fn(&Finding) -> bool,
    on_start: impl Fn(&str),
) -> Vec<Outcome> {
    let running = running_processes();
    analysis
        .findings
        .iter()
        .filter(|f| include(f))
        .filter_map(|finding| {
            let rule = analysis.rule(&finding.id)?;
            on_start(&finding.id);
            Some(execute_one(rule, finding, mode, lang, &running))
        })
        .collect()
}

fn execute_one(rule: &Rule, finding: &Finding, mode: Mode, lang: Lang, running: &HashSet<String>) -> Outcome {
    let m = locale::messages(lang);
    if !rule.deletes_targets() && !rule.command_only() && rule.kind != Kind::RecycleBin {
        // Elle yapılan kurallar arayüzde seçilemez; yine de gelirse dokunmayız.
        return Outcome::skipped(finding, String::new());
    }
    let apps = rule.running_apps(running);
    if !apps.is_empty() {
        return Outcome::skipped(finding, format!("{}: {}", m.close_apps, apps.join(", ")));
    }
    if let Some(reason) = rule.blocked(lang) {
        return Outcome::skipped(finding, reason);
    }

    let mut outcome = Outcome {
        id: finding.id.clone(),
        name: finding.name.clone(),
        status: Status::Done,
        freed: 0,
        recycled: false,
        failed_items: 0,
        message: None,
    };

    if rule.kind == Kind::RecycleBin {
        let hr = unsafe { SHEmptyRecycleBinW(std::ptr::null_mut(), std::ptr::null(), SHERB_QUIET) };
        if hr < 0 {
            outcome.status = Status::Failed;
        } else {
            outcome.freed = finding.bytes;
        }
        return outcome;
    }

    if rule.command_only() {
        if run_command(rule.command.as_deref().unwrap_or_default()) {
            outcome.freed = finding.bytes;
        } else {
            outcome.status = Status::Failed;
            outcome.message = Some(m.command_failed.clone());
        }
        return outcome;
    }

    // Kullanıcı dosyaları ve yapay zekâ araçlarının dosyaları her durumda geri alınabilir kalır.
    let mode = if rule.always_recycles() { Mode::Recycle } else { mode };
    outcome.recycled = mode == Mode::Recycle;

    for service in &rule.services {
        run_hidden("net", &["stop", service]);
    }
    match rule.kind {
        Kind::Contents => {
            for target in &finding.targets {
                let target = Path::new(target);
                let before = measure(target);
                // Yaş ve dışlama burada diskten taze okunur, tarama sonucuna güvenilmez.
                let Some(entries) = stale_entries(rule, target, None) else {
                    outcome.failed_items += 1;
                    continue;
                };
                for entry in entries {
                    if !remove(&entry, mode) {
                        outcome.failed_items += 1;
                    }
                }
                outcome.freed += before.saturating_sub(measure(target));
            }
        }
        Kind::File | Kind::Files => {
            for target in &finding.targets {
                let target = Path::new(target);
                let size = fs::symlink_metadata(target).map_or(0, |m| m.len());
                if remove(target, mode) {
                    outcome.freed += size;
                } else {
                    outcome.failed_items += 1;
                }
            }
        }
        Kind::Dir | Kind::RecycleBin => {}
    }
    for service in &rule.services {
        run_hidden("net", &["start", service]);
    }

    if outcome.failed_items > 0 {
        if outcome.freed == 0 {
            outcome.status = Status::Failed;
            outcome.message = Some(m.nothing_removed.clone());
        } else {
            outcome.status = Status::Partial;
            outcome.message = Some(m.in_use.replace("{count}", &outcome.failed_items.to_string()));
        }
    }
    outcome
}

fn measure(dir: &Path) -> u64 {
    let text = dir.to_string_lossy();
    let result = scanner::scan(&text, drives::cluster_size(&text), &Progress::default());
    if result.root.flags & flags::DENIED != 0 {
        return 0;
    }
    result.root.alloc
}

/// Tek bir öğeyi kaldırır. Koruma listesine takılan hiçbir şeye dokunulmaz.
fn remove(path: &Path, mode: Mode) -> bool {
    if !safety::deletable(&path.to_string_lossy()) {
        return false;
    }
    match mode {
        Mode::Recycle => recycle(path),
        Mode::Permanent => delete_permanently(&extended(path)) == 0,
    }
}

/// Geri Dönüşüm Kutusu'na taşır. Bir klasörü taşımak onu olduğu gibi yeniden adlandırır;
/// içindeki junction'lar bağlantı olarak taşınır, hedeflerine dokunulmaz.
fn recycle(path: &Path) -> bool {
    let mut from: Vec<u16> = path.as_os_str().encode_wide().collect();
    from.extend([0, 0]);
    let mut op: SHFILEOPSTRUCTW = unsafe { std::mem::zeroed() };
    op.wFunc = FO_DELETE;
    op.pFrom = from.as_ptr();
    op.fFlags = FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI | FOF_WANTNUKEWARNING;
    let result = unsafe { SHFileOperationW(&mut op) };
    result == 0 && op.fAnyOperationsAborted == 0 && fs::symlink_metadata(path).is_err()
}

/// 260 karakterden uzun yollar için `\\?\` öneki.
fn extended(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if text.starts_with(r"\\?\") {
        path.to_path_buf()
    } else {
        PathBuf::from(format!(r"\\?\{text}"))
    }
}

/// Kalıcı siler ve silinemeyen öğe sayısını döndürür. Kullanımdaki bir dosya işi
/// durdurmaz, yalnızca atlanır.
fn delete_permanently(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else { return 1 };
    let attrs = meta.file_attributes();
    let is_dir = attrs & ATTR_DIRECTORY != 0;
    if attrs & ATTR_REPARSE_POINT != 0 {
        // Junction/symlink: yalnızca bağlantıyı kaldır.
        let removed = if is_dir { fs::remove_dir(path) } else { fs::remove_file(path) };
        return removed.is_err() as u64;
    }
    if is_dir {
        let Ok(entries) = fs::read_dir(path) else { return 1 };
        let mut failed: u64 = entries.flatten().map(|e| delete_permanently(&e.path())).sum();
        if failed == 0 && fs::remove_dir(path).is_err() {
            failed += 1;
        }
        return failed;
    }
    if meta.permissions().readonly() {
        let mut permissions = meta.permissions();
        permissions.set_readonly(false);
        let _ = fs::set_permissions(path, permissions);
    }
    fs::remove_file(path).is_err() as u64
}

fn run_hidden(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .is_ok_and(|o| o.status.success())
}

fn run_command(command: &str) -> bool {
    let mut parts = command.split_whitespace();
    let Some(program) = parts.next() else { return false };
    let args: Vec<&str> = parts.collect();
    run_hidden(program, &args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn junction(link: &Path, target: &Path) {
        let out = Command::new("cmd").args(["/C", "mklink", "/J"]).arg(link).arg(target).output().unwrap();
        assert!(out.status.success());
    }

    #[test]
    fn permanent_delete_never_follows_junctions() {
        let root = std::env::temp_dir().join(format!("solidclean-delete-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let doomed = root.join("cache");
        let keep = root.join("keep");
        fs::create_dir_all(doomed.join("deep").join("deeper")).unwrap();
        fs::create_dir_all(&keep).unwrap();
        fs::write(doomed.join("deep").join("deeper").join("a.bin"), b"x").unwrap();
        fs::write(doomed.join("ro.txt"), b"y").unwrap();
        let mut ro = fs::metadata(doomed.join("ro.txt")).unwrap().permissions();
        ro.set_readonly(true);
        fs::set_permissions(doomed.join("ro.txt"), ro).unwrap();
        fs::write(keep.join("onemli.txt"), b"degerli").unwrap();
        junction(&doomed.join("deep").join("link"), &keep);

        let failed = delete_permanently(&extended(&doomed));
        let kept = keep.join("onemli.txt").exists();
        let gone = !doomed.exists();
        let _ = fs::remove_dir_all(&root);

        assert_eq!(failed, 0);
        assert!(kept, "junction hedefindeki dosya silindi!");
        assert!(gone);
    }

    /// Gerçek Geri Dönüşüm Kutusu'nu kullanır (test klasörü kutuda kalır), bu yüzden
    /// varsayılan olarak çalışmaz: `cargo test -- --ignored recycle`
    #[test]
    #[ignore]
    fn recycle_moves_folder_without_touching_junction_target() {
        let root = std::env::temp_dir().join(format!("solidclean-recycle-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let doomed = root.join("solidclean-recycle-test");
        let keep = root.join("keep");
        fs::create_dir_all(&doomed).unwrap();
        fs::create_dir_all(&keep).unwrap();
        fs::write(doomed.join("a.tmp"), b"x").unwrap();
        fs::write(keep.join("onemli.txt"), b"degerli").unwrap();
        junction(&doomed.join("link"), &keep);

        let ok = recycle(&doomed);
        let kept = keep.join("onemli.txt").exists();
        let gone = !doomed.exists();
        let _ = fs::remove_dir_all(&root);

        assert!(ok && gone, "Geri Dönüşüm Kutusu'na taşınamadı");
        assert!(kept, "junction hedefindeki dosya silindi!");
    }
}
