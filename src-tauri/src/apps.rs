//! Yüklü uygulamalar. Yalnızca okunur: klasörün hangi uygulamaya ait olduğu kayıt
//! defterindeki kaldırma kayıtlarından bulunur.
//!
//! Uygulama klasörleri hiçbir zaman doğrudan silinmez ve kaldırıcılar buradan
//! çalıştırılmaz: kaldırma Windows'un "Yüklü uygulamalar" sayfasında yapılır. Klasörü
//! elle silmek hizmetleri, kayıt defteri girdilerini ve kısayolları geride bırakır;
//! MSIX paketinin içinden çalıştırılan bir kaldırıcının kayıt defteri değişiklikleri de
//! paketin sanal katmanında kalabilir.

use serde::Serialize;
use windows_sys::Win32::Foundation::{ERROR_MORE_DATA, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ,
    KEY_WOW64_32KEY, KEY_WOW64_64KEY, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
};
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW};

const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
const SW_SHOWNORMAL: i32 = 1;

#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct AppInfo {
    pub name: String,
    pub publisher: Option<String>,
}

struct App {
    info: AppInfo,
    /// Küçük harfli, sonunda `\` olmayan kurulum klasörü.
    location: String,
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Program Files klasörleri: "uygulamaları da tara" seçildiğinde taramaya eklenir.
pub fn program_files() -> Vec<String> {
    let mut roots: Vec<String> = Vec::new();
    for var in ["ProgramW6432", "ProgramFiles", "ProgramFiles(x86)"] {
        if let Ok(value) = std::env::var(var) {
            let value = value.trim_end_matches('\\').to_string();
            if !value.is_empty() && !roots.iter().any(|r| r.eq_ignore_ascii_case(&value)) {
                roots.push(value);
            }
        }
    }
    roots
}

/// Uygulamaların kurulduğu kök klasörler. Bunların altındaki bir klasör için arayüz
/// "Kaldır" düğmesi gösterir.
pub fn roots() -> Vec<String> {
    let mut roots = program_files();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        roots.push(format!(r"{}\Programs", local.trim_end_matches('\\')));
    }
    roots
}

/// Klasörün ait olduğu uygulama; klasör bir üretici klasörüyse içindeki uygulamalar.
pub fn for_folder(path: &str) -> Vec<AppInfo> {
    matches(&installed(), path)
}

fn matches(apps: &[App], path: &str) -> Vec<AppInfo> {
    let target = normalize(path);
    let inside = |outer: &str, inner: &str| inner == outer || inner.starts_with(&format!("{outer}\\"));
    // Klasör bir uygulamanın kendisi ya da içi: en özel kurulum klasörü kazanır.
    if let Some(app) = apps.iter().filter(|a| inside(&a.location, &target)).max_by_key(|a| a.location.len()) {
        return vec![app.info.clone()];
    }
    // Klasörün içinde kurulu uygulamalar (ör. birden çok ürünü olan bir üretici klasörü).
    let mut found: Vec<AppInfo> = Vec::new();
    for app in apps.iter().filter(|a| inside(&target, &a.location)) {
        if !found.iter().any(|f| f.name == app.info.name) {
            found.push(app.info.clone());
        }
    }
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    found.truncate(8);
    found
}

fn normalize(path: &str) -> String {
    path.trim().trim_matches('"').trim_end_matches('\\').to_lowercase()
}

/// `"C:\Program Files\App\app.exe",0` → `C:\Program Files\App`
fn icon_dir(icon: &str) -> Option<String> {
    let mut text = icon.trim().trim_matches('"');
    if let Some((path, index)) = text.rsplit_once(',') {
        if index.trim().parse::<i32>().is_ok() {
            text = path.trim().trim_matches('"');
        }
    }
    let lower = text.to_lowercase();
    if !(lower.ends_with(".exe") || lower.ends_with(".ico")) || lower.contains(r"\windows\") {
        return None;
    }
    std::path::Path::new(text).parent().map(|p| p.to_string_lossy().into_owned())
}

struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        unsafe { RegCloseKey(self.0) };
    }
}

fn open(parent: HKEY, path: &str, view: u32) -> Option<Key> {
    let mut key: HKEY = std::ptr::null_mut();
    let status = unsafe { RegOpenKeyExW(parent, wide(path).as_ptr(), 0, KEY_READ | view, &mut key) };
    (status == ERROR_SUCCESS).then_some(Key(key))
}

/// HKLM'de (64 bit görünüm) bu anahtar var mı? Temizlik koşulları için de kullanılır.
pub(crate) fn machine_key_exists(path: &str) -> bool {
    open(HKEY_LOCAL_MACHINE, path, KEY_WOW64_64KEY).is_some()
}

fn subkeys(key: &Key) -> Vec<String> {
    let mut names = Vec::new();
    let mut buf = [0u16; 512];
    for index in 0.. {
        let mut len = buf.len() as u32;
        let status = unsafe {
            RegEnumKeyExW(
                key.0,
                index,
                buf.as_mut_ptr(),
                &mut len,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if status == ERROR_MORE_DATA {
            continue;
        }
        if status != ERROR_SUCCESS {
            break;
        }
        names.push(String::from_utf16_lossy(&buf[..len as usize]));
    }
    names
}

/// REG_SZ ya da REG_EXPAND_SZ (ortam değişkenleri açılmış olarak).
fn string(key: &Key, name: &str) -> Option<String> {
    let name = wide(name);
    let mut size = 0u32;
    let flags = RRF_RT_REG_SZ;
    for _ in 0..3 {
        let mut buf = vec![0u16; (size as usize).div_ceil(2).max(1)];
        let mut bytes = (buf.len() * 2) as u32;
        let status = unsafe {
            RegGetValueW(key.0, std::ptr::null(), name.as_ptr(), flags, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut bytes)
        };
        if status == ERROR_MORE_DATA {
            size = bytes;
            continue;
        }
        if status != ERROR_SUCCESS {
            return None;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let text = String::from_utf16_lossy(&buf[..len]).trim().to_string();
        return (!text.is_empty()).then_some(text);
    }
    None
}

fn dword(key: &Key, name: &str) -> Option<u32> {
    let mut value = 0u32;
    let mut bytes = 4u32;
    let status = unsafe {
        RegGetValueW(
            key.0,
            std::ptr::null(),
            wide(name).as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut value as *mut u32).cast(),
            &mut bytes,
        )
    };
    (status == ERROR_SUCCESS).then_some(value)
}

fn installed() -> Vec<App> {
    let mut apps = Vec::new();
    let hives = [(HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY), (HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY), (HKEY_CURRENT_USER, 0)];
    for (hive, view) in hives {
        let Some(list) = open(hive, UNINSTALL, view) else { continue };
        for id in subkeys(&list) {
            let Some(entry) = open(list.0, &id, view) else { continue };
            // Sistem bileşenleri ve güncellemeler Ayarlar'daki listede de görünmez.
            if dword(&entry, "SystemComponent") == Some(1) || string(&entry, "ParentKeyName").is_some() {
                continue;
            }
            let Some(name) = string(&entry, "DisplayName") else { continue };
            let location = string(&entry, "InstallLocation").or_else(|| string(&entry, "DisplayIcon").and_then(|i| icon_dir(&i)));
            let Some(location) = location.map(|l| normalize(&l)) else { continue };
            // Sürücü kökü gibi çok genel konumlar her klasörle eşleşirdi.
            if location.split('\\').filter(|c| !c.is_empty()).count() < 2 {
                continue;
            }
            apps.push(App { info: AppInfo { name, publisher: string(&entry, "Publisher") }, location });
        }
    }
    apps
}

/// Windows Ayarlar > Uygulamalar > Yüklü uygulamalar. Ayarlar ayrı bir süreç olarak açılır;
/// kaldırma işlemi de onun içinde, bu uygulamanın (ve MSIX paketinin) dışında yürür.
pub fn open_installed_apps() -> Result<(), String> {
    shell_open("ms-settings:appsfeatures")
}

/// Bir adresi (ms-settings:, https:) Windows'un varsayılan programıyla açar.
pub(crate) fn shell_open(target: &str) -> Result<(), String> {
    let verb = wide("open");
    let uri = wide(target);
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.lpVerb = verb.as_ptr();
    info.lpFile = uri.as_ptr();
    info.nShow = SW_SHOWNORMAL;
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        return Err(format!("{target}: {}", unsafe { GetLastError() }));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, location: &str) -> App {
        App { info: AppInfo { name: name.into(), publisher: None }, location: normalize(location) }
    }

    #[test]
    fn icon_paths_give_install_folder() {
        assert_eq!(icon_dir(r#""C:\Program Files\App\app.exe",0"#).as_deref(), Some(r"C:\Program Files\App"));
        assert_eq!(icon_dir(r"C:\Program Files\App\app.ico").as_deref(), Some(r"C:\Program Files\App"));
        assert_eq!(icon_dir(r"C:\Windows\Installer\{GUID}\icon.exe"), None);
        assert_eq!(icon_dir(r"C:\Program Files\App\icon.dll,2"), None);
    }

    #[test]
    fn folders_match_their_apps() {
        let apps = [
            app("Docker Desktop", r"C:\Program Files\Docker\Docker"),
            app("Docker Compose", r"C:\Program Files\Docker\Compose\"),
            app("Visual Studio Code", r"C:\Users\İsmail\AppData\Local\Programs\Microsoft VS Code"),
        ];
        let names = |path: &str| matches(&apps, path).into_iter().map(|a| a.name).collect::<Vec<_>>();
        // Uygulamanın kendisi ve içindeki bir klasör.
        assert_eq!(names(r"C:\Program Files\Docker\Docker"), ["Docker Desktop"]);
        assert_eq!(names(r"c:\program files\docker\docker\resources"), ["Docker Desktop"]);
        // Üretici klasörü: içindeki uygulamalar.
        assert_eq!(names(r"C:\Program Files\Docker"), ["Docker Compose", "Docker Desktop"]);
        // Benzer adlı ama farklı bir klasör eşleşmez.
        assert!(names(r"C:\Program Files\Docker\DockerTools").is_empty());
        assert_eq!(names(r"C:\Users\İsmail\AppData\Local\Programs\Microsoft VS Code"), ["Visual Studio Code"]);
    }

    #[test]
    fn reads_real_registry_without_panicking() {
        // Bu makinedeki kayıtlar değişken; yalnızca okumanın çalıştığını doğrular.
        let apps = installed();
        assert!(apps.iter().all(|a| !a.info.name.is_empty() && !a.location.ends_with('\\')));
    }
}
