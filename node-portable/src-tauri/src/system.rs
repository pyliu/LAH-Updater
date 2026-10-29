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

pub fn detect_default_root_dir() -> PathBuf {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. 優先由已註冊之 PM2_Portable_Service ImagePath 反推根目錄 (<root>\node\service\pm2-service.exe)
    if let Ok(svc_key) = hklm.open_subkey_with_flags(r"SYSTEM\CurrentControlSet\Services\PM2_Portable_Service", KEY_READ) {
        if let Ok(img_path) = svc_key.get_value::<String, _>("ImagePath") {
            let trimmed = img_path.trim().trim_matches('"');
            let exe_p = PathBuf::from(trimmed);
            if let Some(root) = exe_p.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
                if root.join("node").join("node.exe").is_file() || root.is_dir() {
                    return root.to_path_buf();
                }
            }
        }
    }

    // 2. 由系統或使用者環境變數 PM2_HOME (<root>\.pm2) 反推根目錄
    let pm2_home_candidates = [
        hklm.open_subkey_with_flags(HKLM_ENV_PATH, KEY_READ).ok().and_then(|k| k.get_value::<String, _>("PM2_HOME").ok()),
        hkcu.open_subkey_with_flags("Environment", KEY_READ).ok().and_then(|k| k.get_value::<String, _>("PM2_HOME").ok()),
    ];
    for cand in pm2_home_candidates.into_iter().flatten() {
        let p = PathBuf::from(cand.trim().trim_end_matches(['\\', '/']));
        if p.file_name().and_then(|n| n.to_str()).map(|s| s.eq_ignore_ascii_case(".pm2")).unwrap_or(false) {
            if let Some(root) = p.parent() {
                if root.join("node").join("node.exe").is_file() {
                    return root.to_path_buf();
                }
            }
        }
    }

    // 3. 檢查執行檔所在目錄或其父目錄是否已安裝 node\node.exe
    let app_dir = get_app_dir();
    if app_dir.join("node").join("node.exe").is_file() {
        return app_dir;
    }
    if let Some(parent) = app_dir.parent() {
        if parent.join("node").join("node.exe").is_file() {
            return parent.to_path_buf();
        }
    }

    // 4. 檢查預設 C:\AppServ\node\node.exe
    let appserv = PathBuf::from(r"C:\AppServ");
    if appserv.join("node").join("node.exe").is_file() {
        return appserv;
    }

    // 5. 若工具放於 <某目錄>\node-portable 下且父目錄非磁碟機根目錄，預設指向該父目錄
    if app_dir.file_name().and_then(|n| n.to_str()).map(|s| s.eq_ignore_ascii_case("node-portable")).unwrap_or(false) {
        if let Some(parent) = app_dir.parent() {
            if parent.parent().is_some() {
                return parent.to_path_buf();
            }
        }
    }

    appserv
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

/// 將目標目錄前置於 PATH 字串（用於沙盒內部，如 dump.pm2）
pub fn clean_and_prepend_path_string(raw_path: &str, target_norm: &str) -> (String, Vec<String>) {
    let lower_target = target_norm.trim_end_matches(['\\', '/']).to_lowercase();
    let mut removed = Vec::new();
    let mut kept = vec![target_norm.to_string()];

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
        if !kept.iter().any(|k| k.trim_end_matches(['\\', '/']).eq_ignore_ascii_case(&norm_lower)) {
            kept.push(trimmed.to_string());
        }
    }

    (kept.join(";"), removed)
}

/// 自系統/使用者 PATH 字串中拔除指定目標路徑（用於自癒清除過往寫入之綠色路徑）
pub fn purge_target_from_path_string(raw_path: &str, target_norm: &str) -> (String, Vec<String>) {
    let lower_target = target_norm.trim_end_matches(['\\', '/']).to_lowercase();
    let mut removed = Vec::new();
    let mut kept: Vec<String> = Vec::new();

    for part in raw_path.split(';') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let norm_lower = trimmed.trim_end_matches(['\\', '/']).to_lowercase();
        if norm_lower == lower_target {
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

/// 自動產生專屬沙盒終端批次檔 lah-cmd.bat（零全域污染，隨點即用）
pub fn ensure_portable_cmd_helper(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) {
    let bat_path = root_dir.join("lah-cmd.bat");
    let content = "@echo off\r\n\
chcp 65001 >nul\r\n\
setlocal\r\n\
set \"ROOT_DIR=%~dp0\"\r\n\
set \"ROOT_DIR=%ROOT_DIR:~0,-1%\"\r\n\
set \"NODE_DIR=%ROOT_DIR%\\node\"\r\n\
set \"PM2_HOME=%ROOT_DIR%\\.pm2\"\r\n\
set \"PATH=%NODE_DIR%;%PATH%\"\r\n\
title LAH 智慧控管專屬終端 (Node 16 / PM2 沙盒環境)\r\n\
echo ============================================================\r\n\
echo   LAH 智慧控管專屬維運終端 (沙盒環境 - 零全域污染)\r\n\
echo   - 隔離 Node 目錄: %NODE_DIR%\r\n\
echo   - 隔離 PM2 目錄 : %PM2_HOME%\r\n\
echo   - 全域環境不受任何干擾 (關閉視窗即自動釋放)\r\n\
echo ============================================================\r\n\
call node -v\r\n\
call \"%NODE_DIR%\\pm2.cmd\" -v\r\n\
echo ------------------------------------------------------------\r\n\
echo 提示: 您可在此直接執行 pm2 status / pm2 logs / pm2 restart all\r\n\
echo ============================================================\r\n\
cmd /k\r\n";

    if let Err(e) = fs::write(&bat_path, content) {
        log_fn(&format!("⚠️ 建立 lah-cmd.bat 失敗: {e}"), "warning");
    } else {
        log_fn("  ✓ 已就緒專屬沙盒維運終端批次檔 (lah-cmd.bat)", "success");
    }
}

/// 自系統 (HKLM) 與使用者 (HKCU) 登錄檔中還原乾淨環境，移除可攜版路徑污染
pub fn purge_portable_from_registry(target_dir: &Path, log_fn: &dyn Fn(&str, &str)) {
    let target_norm = target_dir
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string();

    let mut changed_any = false;

    // 1. 清理 HKLM PATH 與 PM2_HOME
    if is_admin() {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(sys_key) = hklm.open_subkey_with_flags(HKLM_ENV_PATH, KEY_READ | KEY_WRITE) {
            if let Ok(cur_sys) = sys_key.get_value::<String, _>("Path") {
                let (new_sys, removed_sys) = purge_target_from_path_string(&cur_sys, &target_norm);
                if !removed_sys.is_empty() {
                    let _ = sys_key.set_value("Path", &new_sys);
                    changed_any = true;
                    log_fn(
                        &format!("  ✓ 已自系統 PATH (HKLM) 移除可攜路徑 [{target_norm}]，確保全域 Node 獨立性"),
                        "info",
                    );
                }
            }
            if sys_key.get_value::<String, _>("PM2_HOME").is_ok() {
                let _ = sys_key.delete_value("PM2_HOME");
                changed_any = true;
                log_fn("  ✓ 已自系統環境變數 (HKLM) 清理 PM2_HOME，避免污染一般終端", "info");
            }
        }
    }

    // 2. 清理 HKCU PATH 與 PM2_HOME
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(usr_key) = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE) {
        if let Ok(cur_usr) = usr_key.get_value::<String, _>("Path") {
            let (new_usr, removed_usr) = purge_target_from_path_string(&cur_usr, &target_norm);
            if !removed_usr.is_empty() {
                let _ = usr_key.set_value("Path", &new_usr);
                changed_any = true;
                log_fn(
                    &format!("  ✓ 已自使用者 PATH (HKCU) 移除可攜路徑 [{target_norm}]，確保全域 Node 獨立性"),
                    "info",
                );
            }
        }
        if usr_key.get_value::<String, _>("PM2_HOME").is_ok() {
            let _ = usr_key.delete_value("PM2_HOME");
            changed_any = true;
            log_fn("  ✓ 已自使用者環境變數 (HKCU) 清理 PM2_HOME，避免污染一般終端", "info");
        }
    }

    if changed_any {
        broadcast_setting_change();
    }
}

/// 綠色可攜環境設定（零全域污染模式）：
/// 1. 主動自登錄檔清理可能殘留的綠色路徑，恢復全域 Node 獨立性。
/// 2. 僅於當前行程 (Process) PATH 注入 target_dir，供本次安裝程序使用。
/// 3. 自動生成專屬沙盒維運終端 lah-cmd.bat。
pub fn register_path_to_user_environment(target_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    if !target_dir.is_dir() {
        log_fn(&format!("⚠️ 目錄不存在，略過設定 PATH: {}", target_dir.display()), "warning");
        return false;
    }

    let target_norm = target_dir.to_string_lossy().trim_end_matches(['\\', '/']).to_string();

    // 1. 自癒：自登錄檔中還原全域環境（移除過往版本可能寫入之綠色版路徑）
    purge_portable_from_registry(target_dir, log_fn);

    // 2. 僅於當前行程內更新 PATH，供本次安裝過程呼叫使用，作業系統全域零污染
    let cur_proc = env::var("PATH").unwrap_or_default();
    let (new_proc, _) = clean_and_prepend_path_string(&cur_proc, &target_norm);
    env::set_var("PATH", new_proc);
    log_fn(
        &format!("✅ [零全域污染] 已將可攜環境限縮於當前行程與專屬沙盒 ({target_norm})"),
        "success",
    );

    // 3. 自動建立/更新專屬沙盒終端批次檔
    if let Some(parent) = target_dir.parent() {
        ensure_portable_cmd_helper(parent, log_fn);
    }

    true
}

/// 綠色可攜 PM2_HOME 設定（零全域污染模式）：
/// 僅作用於當前行程，不污染 HKLM/HKCU 登錄檔。
pub fn register_pm2_home_and_clean_legacy_env(pm2_home: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    let val = pm2_home.to_string_lossy().to_string();

    let legacy_vars = ["PM2_INSTALL_DIRECTORY", "PM2_SERVICE_DIRECTORY"];
    for lv in &legacy_vars {
        env::remove_var(lv);
    }
    env::set_var("PM2_HOME", &val);
    log_fn(
        &format!("✅ [零全域污染] 已設定當前行程 PM2_HOME={val}"),
        "success",
    );
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
            0xFFFF as _,
            0x001A,
            0,
            env_wide.as_ptr() as isize,
            0x0002,
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
    fn test_purge_target_from_path_preserves_global_node() {
        let raw = r"C:\Windows\system32;C:\Program Files\nodejs\;D:\DEV\node\;C:\Users\administrator.HA\AppData\Roaming\npm";
        let (cleaned, removed) = purge_target_from_path_string(raw, r"D:\DEV\node");
        assert_eq!(
            cleaned,
            r"C:\Windows\system32;C:\Program Files\nodejs\;C:\Users\administrator.HA\AppData\Roaming\npm"
        );
        assert_eq!(removed, vec![r"D:\DEV\node\"]);
    }
}
