//! Arayüz olmadan ölçüm:
//!   `cargo run --release --bin scanbench -- C:\Users\ad`      tarama hızı
//!   `cargo run --release --bin scanbench -- --cleanup [out.ps1]`  temizlik önerileri ve script

use solidclean_lib::{cleanup, drives, scanner};

const GIB: f64 = (1u64 << 30) as f64;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--cleanup") {
        return report_cleanup(args.get(1));
    }
    let root = args.first().cloned().unwrap_or_else(drives::home);
    let progress = scanner::Progress::default();
    let result = scanner::scan(&root, drives::cluster_size(&root), &progress);
    let r = &result.root;
    println!("root:         {root}");
    println!("elapsed:      {} ms", result.elapsed_ms);
    println!("files:        {}", r.files);
    println!("dirs:         {}", r.dirs);
    println!("size:         {:.2} GiB", r.size as f64 / GIB);
    println!("size on disk: {:.2} GiB", r.alloc as f64 / GIB);
    println!("unreadable:   {}", result.errors);
}

fn report_cleanup(script_path: Option<&String>) {
    let start = std::time::Instant::now();
    let analysis = cleanup::analyze(None, cleanup::Lang::En, None);
    println!("analyzed in {} ms\n", start.elapsed().as_millis());
    for f in &analysis.findings {
        let size = if f.measured { format!("{:.2} GiB", f.bytes as f64 / GIB) } else { "?".into() };
        let flags = [
            (f.manual.is_some(), "manual"),
            (!f.running_apps.is_empty(), "app-running"),
        ]
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join(",");
        println!("{:<7} {:>10}  {:<24} {:<14} {}", format!("{:?}", f.safety), size, f.id, flags, f.location);
    }
    if let Some(path) = script_path {
        let ids: Vec<String> = analysis.findings.iter().map(|f| f.id.clone()).collect();
        std::fs::write(path, cleanup::script(&analysis, &ids, cleanup::Lang::Tr)).unwrap();
        println!("\nscript written to {path}");
    }
}
