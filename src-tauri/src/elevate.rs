//! Yönetici gerektiren temizlikler. Uygulamanın kendisi normal yetkiyle çalışır;
//! gerektiğinde kendi exe'si UAC ile yönetici olarak, arayüzsüz bir yardımcı modda
//! yeniden başlatılır ve iş bitince kapanır.
//!
//! Yardımcıya yalnızca kural kimlikleri ve mod gönderilir; silinecek yolları kendisi
//! yeniden çözer. Böylece iş dosyası değiştirilse bile keyfi bir yol sildirilemez.

use std::fs::{self, OpenOptions};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
use windows_sys::Win32::System::Threading::{WaitForSingleObject, INFINITE};
use windows_sys::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};

use crate::clean::{self, Mode, Outcome};
use crate::cleanup::{self, Lang};

/// `solidclean.exe --elevated-cleanup <iş dosyası> <sonuç dosyası>`
pub const FLAG: &str = "--elevated-cleanup";

const ERROR_CANCELLED: u32 = 1223;

#[derive(Serialize, Deserialize)]
struct Job {
    ids: Vec<String>,
    mode: Mode,
    lang: String,
}

pub enum ElevationError {
    /// Kullanıcı UAC penceresinde "Hayır" dedi.
    Declined,
    Failed(String),
}

impl ElevationError {
    pub fn message(&self, lang: Lang) -> String {
        let m = crate::locale::messages(lang);
        match self {
            ElevationError::Declined => m.elevation_declined.clone(),
            ElevationError::Failed(e) => m.elevation_failed.replace("{error}", e),
        }
    }
}

pub fn is_elevated() -> bool {
    unsafe { IsUserAnAdmin() != 0 }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Verilen kuralları yönetici olarak çalıştırır. UAC penceresi bir kez açılır.
pub fn run_as_admin(ids: &[String], mode: Mode, lang: &str) -> Result<Vec<Outcome>, ElevationError> {
    let fail = |e: &dyn std::fmt::Display| ElevationError::Failed(e.to_string());
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let base = std::env::temp_dir().join(format!("solidclean-{}-{stamp}", std::process::id()));
    let job_path = base.with_extension("job.json");
    let result_path = base.with_extension("result.json");
    let job = Job { ids: ids.to_vec(), mode, lang: lang.to_string() };
    fs::write(&job_path, serde_json::to_vec(&job).map_err(|e| fail(&e))?).map_err(|e| fail(&e))?;

    let outcome = launch(&job_path, &result_path).and_then(|()| {
        let data = fs::read(&result_path).map_err(|e| fail(&e))?;
        serde_json::from_slice::<Vec<Outcome>>(&data).map_err(|e| fail(&e))
    });
    let _ = fs::remove_file(&job_path);
    let _ = fs::remove_file(&result_path);
    outcome
}

fn launch(job: &Path, result: &Path) -> Result<(), ElevationError> {
    let exe = std::env::current_exe().map_err(|e| ElevationError::Failed(e.to_string()))?;
    let exe: Vec<u16> = exe.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let params = wide(&format!("{FLAG} \"{}\" \"{}\"", job.display(), result.display()));
    let verb = wide("runas");

    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpVerb = verb.as_ptr();
    info.lpFile = exe.as_ptr();
    info.lpParameters = params.as_ptr();
    info.nShow = 0; // SW_HIDE

    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        let code = unsafe { GetLastError() };
        return Err(if code == ERROR_CANCELLED {
            ElevationError::Declined
        } else {
            ElevationError::Failed(format!("ShellExecuteEx {code}"))
        });
    }
    if info.hProcess.is_null() {
        return Err(ElevationError::Failed("no process handle".into()));
    }
    unsafe {
        WaitForSingleObject(info.hProcess, INFINITE);
        CloseHandle(info.hProcess);
    }
    Ok(())
}

/// Yönetici olarak başlatılan yardımcının giriş noktası; çıkış kodunu döndürür.
pub fn helper_main(args: &[String]) -> i32 {
    let [job_path, result_path] = args else { return 2 };
    let Ok(data) = fs::read(job_path) else { return 3 };
    let Ok(job) = serde_json::from_slice::<Job>(&data) else { return 4 };
    let lang = Lang::parse(&job.lang);
    let analysis = cleanup::analyze(None, lang, Some(&job.ids));
    let outcomes = clean::execute(&analysis, job.mode, lang, |_| true, |_| {});
    // `create_new`: yol önceden (ör. bir bağlantı olarak) hazırlanmışsa yazmayı reddet.
    let Ok(file) = OpenOptions::new().write(true).create_new(true).open(result_path) else { return 5 };
    if serde_json::to_writer(file, &outcomes).is_err() {
        return 6;
    }
    0
}
