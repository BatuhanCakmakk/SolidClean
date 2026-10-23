// Sürüm derlemesinde arkada konsol penceresi açılmasın.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Yönetici gerektiren temizlikler için UAC ile başlatılan arayüzsüz yardımcı mod.
    if args.first().map(String::as_str) == Some(solidclean_lib::elevate::FLAG) {
        std::process::exit(solidclean_lib::elevate::helper_main(&args[1..]));
    }
    solidclean_lib::run()
}
