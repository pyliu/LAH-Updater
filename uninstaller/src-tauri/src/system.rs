use std::env;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
use winreg::RegKey;

pub const CREATE_NO_WINDOW: u32 = 0x08000000;
const HKLM_ENV_PATH: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";

pub fn is_admin() -> bool {
    unsafe {
        windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0
    }
}

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

    // 1. 優先由已註冊之系統服務 ImagePath 反推根目錄 (涵蓋 LAH_PM2_Service, PM2_Portable_Service, Apache2.4)
    for svc_name in ["LAH_PM2_Service", "PM2_Portable_Service", "Apache2.4", "Apache24"] {
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

pub fn run_cmd_hidden(cmd: &str, args: &[&str]) -> (bool, String, String) {
    let output = Command::new(cmd)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(out) => {
            let stdout = decode_console_bytes(&out.stdout);
            let stderr = decode_console_bytes(&out.stderr);
            (out.status.success(), stdout, stderr)
        }
        Err(e) => (false, String::new(), e.to_string()),
    }
}

pub fn run_powershell(script: &str) -> (bool, String, String) {
    run_cmd_hidden("powershell", &["-NoProfile", "-Command", script])
}

pub fn decode_console_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    String::from_utf8_lossy(bytes).to_string()
}

/// 強健檔案與目錄刪除常式：自動清除唯讀屬性並支援重試
pub fn robust_rmtree(target_path: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    if !target_path.exists() {
        return (true, "路徑不存在，無需刪除".to_string());
    }

    let remove_readonly_recursive = |p: &Path| {
        let _ = remove_readonly(p);
        if p.is_dir() {
            if let Ok(entries) = fs::read_dir(p) {
                for entry in entries.flatten() {
                    let _ = remove_readonly(&entry.path());
                }
            }
        }
    };

    for attempt in 0..3 {
        remove_readonly_recursive(target_path);

        let res = if target_path.is_dir() {
            fs::remove_dir_all(target_path)
        } else {
            fs::remove_file(target_path)
        };

        match res {
            Ok(_) => return (true, "已成功刪除".to_string()),
            Err(e) => {
                if attempt == 2 {
                    let msg = format!("刪除失敗 [{}]: {}", target_path.display(), e);
                    log_fn(&format!("  ❌ {}", msg), "error");
                    return (false, msg);
                }
                sleep(Duration::from_millis(800));
            }
        }
    }

    (false, "重試 3 次後仍刪除失敗".to_string())
}

pub fn remove_readonly(path: &Path) -> std::io::Result<()> {
    let metadata = fs::metadata(path)?;
    let mut permissions = metadata.permissions();
    if permissions.readonly() {
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_exe_from_image_path() {
        let quoted = r#""C:\AppServ\Apache24\bin\httpd.exe" -k runservice"#;
        assert_eq!(
            extract_exe_from_image_path(quoted),
            Some(PathBuf::from(r"C:\AppServ\Apache24\bin\httpd.exe"))
        );

        let unquoted = r#"D:\AppServ\node\service\pm2-service.exe"#;
        assert_eq!(
            extract_exe_from_image_path(unquoted),
            Some(PathBuf::from(r"D:\AppServ\node\service\pm2-service.exe"))
        );

        let empty = "   ";
        assert_eq!(extract_exe_from_image_path(empty), None);
    }

    #[test]
    fn test_is_valid_lah_or_node_root() {
        let temp = std::env::temp_dir().join("test_lah_uninstaller_root");
        let _ = fs::create_dir_all(&temp);

        assert!(!is_valid_lah_or_node_root(&temp));

        let apache = temp.join("Apache24");
        let _ = fs::create_dir_all(&apache);
        assert!(is_valid_lah_or_node_root(&temp));

        let _ = fs::remove_dir_all(&temp);
    }
}
