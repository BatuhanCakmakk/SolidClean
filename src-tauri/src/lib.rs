mod apps;
mod category;
mod clean;
pub mod cleanup;
pub mod drives;
pub mod elevate;
mod history;
mod locale;
mod safety;
pub mod scanner;
mod view;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use scanner::{Progress, ScanResult};

#[derive(Default)]
struct AppState {
    active: Mutex<Option<Arc<Progress>>>,
    result: Mutex<Option<Arc<ScanResult>>>,
    analysis: Mutex<Option<Arc<cleanup::Analysis>>>,
    /// Temizlikten sonra taranmış ağaç artık diski yansıtmaz; öneriler diskten ölçülür.
    tree_stale: AtomicBool,
    cleaning: AtomicBool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    files: u64,
    dirs: u64,
    alloc: u64,
    errors: u64,
    current: String,
    elapsed_ms: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanSummary {
    root_path: String,
    elapsed_ms: u64,
    files: u64,
    dirs: u64,
    size: u64,
    alloc: u64,
    errors: u64,
    cancelled: bool,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TreeResponse {
    full_path: String,
    node: view::NodeView,
}

fn progress_event(progress: &Progress, start: Instant) -> ProgressEvent {
    let current = progress.current.lock().map(|c| scanner::display_path(&c).to_string()).unwrap_or_default();
    ProgressEvent {
        files: progress.files.load(Ordering::Relaxed),
        dirs: progress.dirs.load(Ordering::Relaxed),
        alloc: progress.alloc.load(Ordering::Relaxed),
        errors: progress.errors.load(Ordering::Relaxed),
        current,
        elapsed_ms: start.elapsed().as_millis() as u64,
    }
}

fn current_result(state: &AppState) -> Result<Arc<ScanResult>, String> {
    state.result.lock().unwrap().clone().ok_or_else(|| "Henüz tarama yapılmadı".to_string())
}

#[tauri::command]
fn get_locations() -> drives::Locations {
    drives::list()
}

#[tauri::command]
fn start_scan(app: AppHandle, state: State<'_, AppState>, root: String) -> Result<(), String> {
    let mut active = state.active.lock().unwrap();
    if active.is_some() {
        return Err("Bir tarama zaten sürüyor".into());
    }
    if !std::path::Path::new(&root).is_dir() {
        return Err(format!("Klasör bulunamadı: {root}"));
    }
    let progress = Arc::new(Progress::default());
    *active = Some(progress.clone());
    drop(active);
    // Önceki ağacı hemen bırak; büyük taramalarda yüzlerce MB olabilir.
    *state.result.lock().unwrap() = None;
    *state.analysis.lock().unwrap() = None;
    state.tree_stale.store(false, Ordering::Relaxed);

    let done = Arc::new(AtomicBool::new(false));
    {
        let (app, progress, done) = (app.clone(), progress.clone(), done.clone());
        std::thread::spawn(move || {
            let start = Instant::now();
            while !done.load(Ordering::Acquire) {
                let _ = app.emit("scan-progress", progress_event(&progress, start));
                std::thread::sleep(Duration::from_millis(120));
            }
        });
    }

    std::thread::spawn(move || {
        let cluster = drives::cluster_size(&root);
        let outcome = catch_unwind(AssertUnwindSafe(|| scanner::scan(&root, cluster, &progress)));
        done.store(true, Ordering::Release);
        let state = app.state::<AppState>();
        let summary = match outcome {
            Ok(result) => {
                let summary = ScanSummary {
                    root_path: result.root_path.clone(),
                    elapsed_ms: result.elapsed_ms,
                    files: result.root.files,
                    dirs: result.root.dirs,
                    size: result.root.size,
                    alloc: result.root.alloc,
                    errors: result.errors,
                    cancelled: result.cancelled,
                    error: None,
                };
                *state.result.lock().unwrap() = Some(Arc::new(result));
                summary
            }
            Err(_) => ScanSummary {
                root_path: root,
                elapsed_ms: 0,
                files: 0,
                dirs: 0,
                size: 0,
                alloc: 0,
                errors: 0,
                cancelled: true,
                error: Some("Tarama beklenmedik bir hatayla durdu".into()),
            },
        };
        *state.active.lock().unwrap() = None;
        let _ = app.emit("scan-done", summary);
    });
    Ok(())
}

#[tauri::command]
fn cancel_scan(state: State<'_, AppState>) {
    if let Some(progress) = state.active.lock().unwrap().as_ref() {
        progress.cancel.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
fn get_tree(state: State<'_, AppState>, path: Vec<u32>, depth: u8, limit: usize) -> Result<TreeResponse, String> {
    let result = current_result(&state)?;
    let analysis = state.analysis.lock().unwrap().clone();
    let no_marks = view::Marks::new();
    let marks = analysis.as_ref().map_or(&no_marks, |a| &a.marks);
    let (full_path, node) =
        view::subtree(&result, &path, depth.min(3), limit.clamp(1, 500), marks).ok_or("Klasör bulunamadı")?;
    Ok(TreeResponse { full_path, node })
}

/// Temizlik önerilerini hesaplar. Taranmış ağaçta olan klasörler için diske tekrar gidilmez.
#[tauri::command]
async fn analyze_cleanup(state: State<'_, AppState>, lang: String) -> Result<Vec<cleanup::Finding>, String> {
    let scan = state.result.lock().unwrap().clone().filter(|_| !state.tree_stale.load(Ordering::Relaxed));
    let analysis =
        tauri::async_runtime::spawn_blocking(move || cleanup::analyze(scan.as_deref(), cleanup::Lang::parse(&lang), None))
            .await
            .map_err(|e| e.to_string())?;
    let findings = analysis.findings.clone();
    *state.analysis.lock().unwrap() = Some(Arc::new(analysis));
    Ok(findings)
}

/// Seçilen önerileri uygular. Hedefler burada diskten yeniden çözülür; yönetici gerektirenler
/// tek bir UAC onayıyla ayrı bir süreçte çalışır. Her öneri başlarken `cleanup-progress` yayılır.
#[tauri::command]
async fn run_cleanup(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
    mode: clean::Mode,
    lang: String,
) -> Result<Vec<clean::Outcome>, String> {
    if state.cleaning.swap(true, Ordering::AcqRel) {
        return Err("Bir temizlik zaten sürüyor".into());
    }
    let emitter = app.clone();
    let outcomes = tauri::async_runtime::spawn_blocking(move || {
        let language = cleanup::Lang::parse(&lang);
        let analysis = cleanup::analyze(None, language, Some(&ids));
        let elevated = elevate::is_elevated();
        let needs_helper = |f: &cleanup::Finding| analysis.needs_admin(&f.id) && !elevated;
        let emit = |id: &str| {
            let _ = emitter.emit("cleanup-progress", id.to_string());
        };

        let mut outcomes = clean::execute(&analysis, mode, language, |f| !needs_helper(f), emit);
        let admin: Vec<&cleanup::Finding> = analysis.findings.iter().filter(|f| needs_helper(f)).collect();
        if !admin.is_empty() {
            admin.iter().for_each(|f| emit(&f.id));
            let admin_ids: Vec<String> = admin.iter().map(|f| f.id.clone()).collect();
            match elevate::run_as_admin(&admin_ids, mode, &lang) {
                Ok(result) => outcomes.extend(result),
                Err(e) => outcomes.extend(admin.iter().map(|f| clean::Outcome::skipped(f, e.message(language)))),
            }
        }
        history::append(mode, &outcomes);
        outcomes
    })
    .await;
    state.cleaning.store(false, Ordering::Release);
    let outcomes = outcomes.map_err(|e| e.to_string())?;
    state.tree_stale.store(true, Ordering::Relaxed);
    *state.analysis.lock().unwrap() = None;
    Ok(outcomes)
}

#[tauri::command]
fn get_history() -> Vec<history::Entry> {
    history::load()
}

/// Klasörün ait olduğu yüklü uygulama(lar). Yalnızca kayıt defterini okur.
#[tauri::command]
async fn apps_for_folder(path: String) -> Result<Vec<apps::AppInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || apps::for_folder(&path)).await.map_err(|e| e.to_string())
}

/// Windows Ayarlar'daki "Yüklü uygulamalar" sayfasını açar; kaldırma orada yapılır.
#[tauri::command]
fn open_installed_apps() -> Result<(), String> {
    apps::open_installed_apps()
}

/// Projeyi destekleme sayfası. Bağış hiçbir özelliği açmaz (Store politikası 10.8.2).
const SUPPORT_URL: &str = "https://buymeacoffee.com/batuhancakmak";

/// Destek sayfasını varsayılan tarayıcıda açar. Adres sabittir, arayüzden gelmez.
#[tauri::command]
fn open_support_page() -> Result<(), String> {
    apps::shell_open(SUPPORT_URL)
}

#[tauri::command]
fn largest_files(state: State<'_, AppState>, limit: usize) -> Result<Vec<view::FileEntry>, String> {
    let result = current_result(&state)?;
    Ok(view::largest_files(&result, limit.clamp(1, 200)))
}

/// Dosyayı Explorer'da seçili olarak açar.
#[tauri::command]
fn reveal(path: String) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    if !std::path::Path::new(&path).exists() {
        return Err(format!("Bulunamadı: {path}"));
    }
    std::process::Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{path}\""))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_locations,
            start_scan,
            cancel_scan,
            get_tree,
            largest_files,
            reveal,
            analyze_cleanup,
            run_cleanup,
            get_history,
            apps_for_folder,
            open_installed_apps,
            open_support_page
        ])
        .run(tauri::generate_context!())
        .expect("SolidClean başlatılamadı");
}
