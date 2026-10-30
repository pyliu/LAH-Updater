use std::env;
use std::path::{Path, PathBuf};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
use winreg::RegKey;

const HKLM_ENV_PATH: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";

pub fn get_app_dir() -> PathBuf {
    if let Ok(mut exe) = env::current_exe() {
        exe.pop();
        return exe;
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn is_valid_lah_or_node_root(dir: &Path) -> bool {
    dir.join("node").join("node.exe").is_file()
        || dir.join("Apache24").is_dir()
        || dir.join("www").is_dir()
        || dir.join("LAH-API").is_dir()
        || dir.join("LAH-NUXTJS").is_dir()
        || dir.join("lah-messenger-server").is_dir()
}

pub fn extract_exe_from_image_path(img_path: &str) -> Option<PathBuf> {
    let trimmed = img_path.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(PathBuf::from(&rest[..end]));
    }
    let lower = trimmed.to_lowercase();
    if let Some(idx) = lower.find(".exe") {
        return Some(PathBuf::from(&trimmed[..idx + 4]));
    }
    Some(PathBuf::from(trimmed.split_whitespace().next()?))
}

pub fn detect_default_root_dir() -> PathBuf {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. 優先由已註冊之系統服務 ImagePath 反推根目錄 (涵蓋 LAH_Web_Service, LAH_PM2_Service, PM2_Portable_Service, Apache2.4)
    for svc_name in ["LAH_Web_Service", "LAH_PM2_Service", "PM2_Portable_Service", "Apache2.4", "Apache24"] {
        let subkey = format!(r"SYSTEM\CurrentControlSet\Services\{svc_name}");
        if let Ok(svc_key) = hklm.open_subkey_with_flags(&subkey, KEY_READ) {
            if let Ok(img_path) = svc_key.get_value::<String, _>("ImagePath") {
                if let Some(exe_p) = extract_exe_from_image_path(&img_path) {
                    let mut cur = exe_p.parent();
                    for _ in 0..4 {
                        if let Some(p) = cur {
                            if is_valid_lah_or_node_root(p) {
                                return p.to_path_buf();
                            }
                            cur = p.parent();
                        }
                    }
                }
            }
        }
    }

    // 2. 由系統或使用者環境變數 PM2_HOME (<root>\.pm2) 反推根目錄
    let pm2_home_candidates = [
        hklm.open_subkey_with_flags(HKLM_ENV_PATH, KEY_READ)
            .ok()
            .and_then(|k| k.get_value::<String, _>("PM2_HOME").ok()),
        hkcu.open_subkey_with_flags("Environment", KEY_READ)
            .ok()
            .and_then(|k| k.get_value::<String, _>("PM2_HOME").ok()),
    ];
    for cand in pm2_home_candidates.into_iter().flatten() {
        let p = PathBuf::from(cand.trim().trim_end_matches(['\\', '/']));
        if p.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.eq_ignore_ascii_case(".pm2"))
            .unwrap_or(false)
        {
            if let Some(root) = p.parent() {
                if is_valid_lah_or_node_root(root) {
                    return root.to_path_buf();
                }
            }
        }
    }

    // 3. 檢查執行檔所在目錄或其父目錄是否已安裝系統元件
    let app_dir = get_app_dir();
    if is_valid_lah_or_node_root(&app_dir) {
        return app_dir;
    }
    if let Some(parent) = app_dir.parent() {
        if parent.parent().is_some() && is_valid_lah_or_node_root(parent) {
            return parent.to_path_buf();
        }
    }

    // 4. 優先偵測本機存在的 AppServ 路徑 (D:\AppServ 或 C:\AppServ)
    if Path::new(r"D:\AppServ").exists() && is_valid_lah_or_node_root(Path::new(r"D:\AppServ")) {
        return PathBuf::from(r"D:\AppServ");
    }
    if Path::new(r"C:\AppServ").exists() && is_valid_lah_or_node_root(Path::new(r"C:\AppServ")) {
        return PathBuf::from(r"C:\AppServ");
    }

    // 5. 預設回退至 C:\AppServ
    PathBuf::from(r"C:\AppServ")
}
