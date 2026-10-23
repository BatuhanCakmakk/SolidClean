//! Sürücü listesi ve birim bilgileri.

use serde::Serialize;
use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDiskFreeSpaceW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
};

const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub root: String,
    pub label: String,
    pub file_system: String,
    pub total: u64,
    pub free: u64,
    pub removable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Locations {
    pub home: String,
    /// Windows'un kurulu olduğu sürücü, ör. `C:\`.
    pub system_drive: String,
    pub volumes: Vec<Volume>,
    /// Uygulamaların kurulduğu kökler (Program Files, %LOCALAPPDATA%\Programs).
    pub app_roots: Vec<String>,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn from_wide(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

pub fn home() -> String {
    std::env::var("USERPROFILE").unwrap_or_else(|_| r"C:\".into())
}

pub fn list() -> Locations {
    let mask = unsafe { GetLogicalDrives() };
    let mut volumes = Vec::new();
    for i in 0..26u8 {
        if mask & (1 << i) == 0 {
            continue;
        }
        let root = format!("{}:\\", (b'A' + i) as char);
        let w = wide(&root);
        let drive_type = unsafe { GetDriveTypeW(w.as_ptr()) };
        if drive_type != DRIVE_FIXED && drive_type != DRIVE_REMOVABLE {
            continue;
        }
        let (mut available, mut total, mut free) = (0u64, 0u64, 0u64);
        if unsafe { GetDiskFreeSpaceExW(w.as_ptr(), &mut available, &mut total, &mut free) } == 0 {
            continue;
        }
        let mut label = [0u16; 261];
        let mut file_system = [0u16; 261];
        unsafe {
            GetVolumeInformationW(
                w.as_ptr(),
                label.as_mut_ptr(),
                label.len() as u32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                file_system.as_mut_ptr(),
                file_system.len() as u32,
            )
        };
        volumes.push(Volume {
            root,
            label: from_wide(&label),
            file_system: from_wide(&file_system),
            total,
            free,
            removable: drive_type == DRIVE_REMOVABLE,
        });
    }
    let system_drive = format!("{}\\", std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into()));
    Locations { home: home(), system_drive, volumes, app_roots: crate::apps::roots() }
}

/// Yolun bulunduğu birimin küme (cluster) boyutu; bilinmiyorsa 4 KiB.
pub fn cluster_size(path: &str) -> u64 {
    let path = crate::scanner::display_path(path);
    let bytes = path.as_bytes();
    if bytes.len() < 2 || bytes[1] != b':' {
        return 4096;
    }
    let root = wide(&format!("{}\\", &path[..2]));
    let (mut sectors, mut bytes_per_sector, mut free, mut total) = (0u32, 0u32, 0u32, 0u32);
    let ok = unsafe { GetDiskFreeSpaceW(root.as_ptr(), &mut sectors, &mut bytes_per_sector, &mut free, &mut total) };
    if ok == 0 || sectors == 0 || bytes_per_sector == 0 {
        return 4096;
    }
    sectors as u64 * bytes_per_sector as u64
}
