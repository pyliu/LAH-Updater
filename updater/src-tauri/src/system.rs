use std::env;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};
use winreg::RegKey;

pub const CREATE_NO_WINDOW: u32 = 0x08000000;
const HKLM_ENV_PATH: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";

pub fn is_admin() -> bool {
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
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

    // 1. 優先由已註冊之 LAH_PM2_Service / PM2_Portable_Service / Apache2.4 ImagePath 反推根目錄
    for svc_name in ["LAH_PM2_Service", "PM2_Portable_Service", "Apache2.4", "Apache24"] {
        let subkey = format!(r"SYSTEM\CurrentControlSet\Services\{svc_name}");
        if let Ok(svc_key) = hklm.open_subkey_with_flags(&subkey, KEY_READ) {
            if let Ok(img_path) = svc_key.get_value::<String, _>("ImagePath") {
                if let Some(exe_p) = extract_exe_from_image_path(&img_path) {
                    if let Some(root) = exe_p.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
                        if is_valid_lah_or_node_root(root) {
                            return root.to_path_buf();
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

    // 4. 預設回退至 C:\AppServ
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

pub fn ensure_root_dir(path: &Path) -> Result<(), String> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(|e| format!("建立目錄失敗: {}", e))?;
    }
    let test_file = path.join(".lah_write_test");
    fs::write(&test_file, b"test").map_err(|e| format!("目錄無法寫入: {}", e))?;
    let _ = fs::remove_file(&test_file);
    Ok(())
}

pub fn is_legacy_global_node_or_pm2_path(p_norm_lower: &str) -> bool {
    matches!(
        p_norm_lower,
        r"c:\program files\nodejs"
            | r"c:\program files (x86)\nodejs"
            | r"c:\programdata\npm\npm"
            | r"c:\programdata\npm"
            | r"c:\programdata\pm2"
    ) || p_norm_lower.ends_with(r"\appdata\roaming\npm")
}

pub fn clean_and_prepend_path_string(raw_path: &str, target_norm: &str) -> (String, Vec<String>) {
    let lower_target = target_norm.trim_end_matches(['\\', '/']).to_lowercase();
    let mut removed = Vec::new();
    let mut kept = vec![target_norm.trim_end_matches(['\\', '/']).to_string()];

    for part in raw_path.split(';') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let norm_lower = trimmed.trim_end_matches(['\\', '/']).to_lowercase();
        if norm_lower == lower_target {
            continue;
        }
        if is_legacy_global_node_or_pm2_path(&norm_lower) {
            removed.push(trimmed.to_string());
            continue;
        }
        if !kept
            .iter()
            .any(|k| k.trim_end_matches(['\\', '/']).eq_ignore_ascii_case(&norm_lower))
        {
            kept.push(trimmed.to_string());
        }
    }

    (kept.join(";"), removed)
}

pub fn register_path_to_user_environment(target_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    if !target_dir.is_dir() {
        log_fn(
            &format!("⚠️ 目錄不存在，略過註冊 PATH: {}", target_dir.display()),
            "warning",
        );
        return false;
    }

    let target_norm = target_dir
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string();

    // 1. 優先更新系統環境變數 PATH (HKLM)，因為 Windows 系統 PATH 優先序高於使用者 PATH
    if is_admin() {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(sys_key) = hklm.open_subkey_with_flags(HKLM_ENV_PATH, KEY_READ | KEY_WRITE) {
            let cur_sys = sys_key.get_value::<String, _>("Path").unwrap_or_default();
            let (new_sys, removed_sys) = clean_and_prepend_path_string(&cur_sys, &target_norm);
            for r in &removed_sys {
                log_fn(
                    &format!("  ✓ 已自系統 PATH (HKLM) 移除舊全域 Node/PM2 路徑: [{r}]"),
                    "info",
                );
            }
            if let Err(e) = sys_key.set_value("Path", &new_sys) {
                log_fn(&format!("⚠️ 寫入系統 PATH (HKLM) 失敗: {}", e), "warning");
            } else {
                log_fn(
                    &format!("✅ 已將 [{target_norm}] 置於系統環境變數 PATH (HKLM) 最前端"),
                    "success",
                );
            }
        }
    }

    // 2. 同步更新使用者環境變數 PATH (HKCU)
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(usr_key) = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE) {
        let cur_usr = usr_key.get_value::<String, _>("Path").unwrap_or_default();
        let (new_usr, removed_usr) = clean_and_prepend_path_string(&cur_usr, &target_norm);
        for r in &removed_usr {
            log_fn(
                &format!("  ✓ 已自使用者 PATH (HKCU) 移除舊全域 Node/PM2 路徑: [{r}]"),
                "info",
            );
        }
        if let Err(e) = usr_key.set_value("Path", &new_usr) {
            log_fn(&format!("⚠️ 寫入使用者 PATH (HKCU) 失敗: {}", e), "warning");
        } else {
            log_fn(
                &format!("✅ 已將 [{target_norm}] 置於使用者環境變數 PATH (HKCU) 最前端"),
                "success",
            );
        }
    }

    // 3. 同步更新當前行程 PATH
    let cur_proc = env::var("PATH").unwrap_or_default();
    let (new_proc, _) = clean_and_prepend_path_string(&cur_proc, &target_norm);
    env::set_var("PATH", new_proc);

    broadcast_setting_change();
    true
}

pub fn register_pm2_home_and_clean_legacy_env(pm2_home: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    let val = pm2_home.to_string_lossy().to_string();
    let legacy_vars = ["PM2_INSTALL_DIRECTORY", "PM2_SERVICE_DIRECTORY"];

    if is_admin() {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(sys_key) = hklm.open_subkey_with_flags(HKLM_ENV_PATH, KEY_READ | KEY_WRITE) {
            let _ = sys_key.set_value("PM2_HOME", &val);
            for lv in &legacy_vars {
                if sys_key.get_value::<String, _>(lv).is_ok() && sys_key.delete_value(lv).is_ok() {
                    log_fn(
                        &format!("  ✓ 已清除系統環境變數 (HKLM) 舊殘留設定: {lv}"),
                        "info",
                    );
                }
            }
            log_fn(
                &format!("✅ 已設定系統環境變數 (HKLM) PM2_HOME={val}"),
                "success",
            );
        }
    }

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(usr_key) = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE) {
        let _ = usr_key.set_value("PM2_HOME", &val);
        for lv in &legacy_vars {
            if usr_key.get_value::<String, _>(lv).is_ok() && usr_key.delete_value(lv).is_ok() {
                log_fn(
                    &format!("  ✓ 已清除使用者環境變數 (HKCU) 舊殘留設定: {lv}"),
                    "info",
                );
            }
        }
        log_fn(
            &format!("✅ 已設定使用者環境變數 (HKCU) PM2_HOME={val}"),
            "success",
        );
    }

    for lv in &legacy_vars {
        env::remove_var(lv);
    }
    env::set_var("PM2_HOME", &val);
    broadcast_setting_change();
    true
}

fn broadcast_setting_change() {
    unsafe {
        let env_wide: Vec<u16> = std::ffi::OsStr::new("Environment")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut result: usize = 0;
        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageTimeoutW(
            0xFFFF as _, // HWND_BROADCAST
            0x001A,      // WM_SETTINGCHANGE
            0,
            env_wide.as_ptr() as isize,
            0x0002, // SMTO_ABORTIFHUNG
            3000,
            &mut result as *mut usize as _,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_and_prepend_path_replaces_global_node_and_pm2() {
        let raw = r"C:\Windows\system32;C:\ProgramData\npm\npm;C:\Program Files\nodejs\;D:\DEV\node\;C:\Users\administrator.HA\AppData\Roaming\npm";
        let (cleaned, removed) = clean_and_prepend_path_string(raw, r"D:\DEV\node");
        assert_eq!(cleaned, r"D:\DEV\node;C:\Windows\system32");
        assert_eq!(removed.len(), 3);
    }

    #[test]
    fn test_extract_exe_from_image_path() {
        assert_eq!(
            extract_exe_from_image_path(r#""D:\DEV\Apache24\bin\httpd.exe" -k runservice"#),
            Some(PathBuf::from(r"D:\DEV\Apache24\bin\httpd.exe"))
        );
        assert_eq!(
            extract_exe_from_image_path(r"D:\DEV\node\service\lah-service.exe"),
            Some(PathBuf::from(r"D:\DEV\node\service\lah-service.exe"))
        );
    }
}
