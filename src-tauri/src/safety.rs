//! Silme koruması. Kurallarda bir hata olsa bile hiçbir temizlik hedefi Windows'a,
//! Program Files'a, kullanıcı profilinin kendisine veya kişisel klasörlere dokunamaz.
//! Script'e giren her silme hedefi önce buradan geçer.

use crate::scanner::display_path;

fn normalize(path: &str) -> String {
    display_path(path).trim_end_matches('\\').to_lowercase()
}

fn env(var: &str) -> Option<String> {
    std::env::var(var).ok().filter(|v| !v.is_empty()).map(|v| normalize(&v))
}

/// Kendisi asla silinmeyecek klasörler (içleri kurala göre temizlenebilir).
fn protected_exact() -> Vec<String> {
    let mut list = Vec::new();
    for var in ["SystemRoot", "ProgramData", "APPDATA", "LOCALAPPDATA", "PUBLIC", "USERPROFILE"] {
        list.extend(env(var));
    }
    if let Some(drive) = env("SystemDrive") {
        list.push(format!("{drive}\\users"));
    }
    if let Some(profile) = env("USERPROFILE") {
        for sub in ["appdata", "appdata\\local", "appdata\\locallow", "appdata\\roaming", "downloads", "favorites", "saved games", "links", "contacts"] {
            list.push(format!("{profile}\\{sub}"));
        }
    }
    list
}

/// İçine hiçbir şekilde dokunulmayacak klasör ağaçları.
fn protected_trees() -> Vec<String> {
    let mut list = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
        list.extend(env(var));
    }
    if let Some(windows) = env("SystemRoot") {
        for sub in ["system32", "syswow64", "winsxs", "installer", "servicing", "boot", "fonts", "assembly", "microsoft.net", "systemapps"] {
            list.push(format!("{windows}\\{sub}"));
        }
    }
    if let Some(drive) = env("SystemDrive") {
        for sub in ["$recycle.bin", "system volume information", "recovery", "boot"] {
            list.push(format!("{drive}\\{sub}"));
        }
    }
    if let Some(profile) = env("USERPROFILE") {
        for sub in ["desktop", "documents", "pictures", "videos", "music", "onedrive", "3d objects"] {
            list.push(format!("{profile}\\{sub}"));
        }
    }
    list
}

/// Bu yol bir temizlik tarafından silinebilir mi (veya içi boşaltılabilir mi)?
pub fn deletable(path: &str) -> bool {
    let p = normalize(path);
    // Sürücü kökü ve hemen altındaki her şey (C:\Windows, C:\Users, C:\hiberfil.sys...) dışarıda.
    if p.split('\\').filter(|c| !c.is_empty()).count() < 3 || p.contains("..") {
        return false;
    }
    if protected_exact().iter().any(|x| *x == p) {
        return false;
    }
    !protected_trees().iter().any(|tree| p == *tree || p.starts_with(&format!("{tree}\\")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str) -> String {
        std::env::var(name).unwrap()
    }

    #[test]
    fn blocks_system_and_personal_folders() {
        let windows = var("SystemRoot");
        let profile = var("USERPROFILE");
        for path in [
            r"C:\".to_string(),
            r"C:\Windows".to_string(),
            r"C:\hiberfil.sys".to_string(),
            windows.clone(),
            format!(r"{windows}\System32\drivers"),
            format!(r"{windows}\WinSxS\x"),
            format!(r"{windows}\Installer\abc.msi"),
            var("ProgramFiles") + r"\App\cache",
            profile.clone(),
            format!(r"{profile}\Documents\tez.docx"),
            format!(r"{profile}\DESKTOP\proje\"),
            format!(r"{profile}\Downloads"),
            var("LOCALAPPDATA"),
            format!(r"{}\..\Windows", var("TEMP")),
        ] {
            assert!(!deletable(&path), "korunmalıydı: {path}");
        }
    }

    #[test]
    fn allows_known_cache_locations() {
        let windows = var("SystemRoot");
        let profile = var("USERPROFILE");
        for path in [
            var("TEMP"),
            format!(r"{windows}\Temp"),
            format!(r"{windows}\SoftwareDistribution\Download"),
            format!(r"{windows}\MEMORY.DMP"),
            format!(r"{}\npm-cache", var("LOCALAPPDATA")),
            format!(r"{profile}\Downloads\setup.exe"),
            format!(r"\\?\{}\npm-cache", var("LOCALAPPDATA")),
        ] {
            assert!(deletable(&path), "izin verilmeliydi: {path}");
        }
    }
}
