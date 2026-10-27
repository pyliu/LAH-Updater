use std::env;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_EXPAND_SZ};
use winreg::RegKey;

pub const CREATE_NO_WINDOW: u32 = 0x08000000;

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
    // 驗證寫入權限
    let test_file = path.join(".lah_write_test");
    fs::write(&test_file, b"test").map_err(|e| format!("目錄無法寫入: {}", e))?;
    let _ = fs::remove_file(&test_file);
    Ok(())
}

pub fn register_path_to_user_environment(target_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    if !target_dir.is_dir() {
        log_fn(&format!("⚠️ 目錄不存在，略過註冊 PATH: {}", target_dir.display()), "warning");
        return false;
    }

    let target_norm = target_dir.to_string_lossy().trim_end_matches(['\\', '/']).to_string();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let (path_val, _val_type) = match hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE) {
        Ok(env_key) => {
            let cur_val: Result<String, _> = env_key.get_value("Path");
            match cur_val {
                Ok(v) => (v, REG_EXPAND_SZ),
                Err(_) => (String::new(), REG_EXPAND_SZ),
            }
        }
        Err(e) => {
            log_fn(&format!("⚠️ 無法開啟使用者環境變數註冊表: {}", e), "warning");
            return false;
        }
    };

    let parts: Vec<&str> = path_val.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let lower_target = target_norm.to_lowercase();
    let already_present = parts.iter().any(|p| p.to_lowercase() == lower_target);

    if already_present {
        log_fn(&format!("ℹ️ 使用者環境變數 PATH 已包含 [{target_norm}]，略過重複寫入。"), "info");
        return true;
    }

    let mut new_parts = vec![target_norm.as_str()];
    new_parts.extend(parts);
    let new_val = new_parts.join(";");

    if let Ok(env_key) = hkcu.open_subkey_with_flags("Environment", KEY_WRITE) {
        if let Err(e) = env_key.set_value("Path", &new_val) {
            log_fn(&format!("⚠️ 寫入 PATH 註冊表失敗: {}", e), "warning");
            return false;
        }
    }

    // 廣播 WM_SETTINGCHANGE
    broadcast_setting_change();

    // 同步當前行程 PATH
    if let Ok(cur_env_path) = env::var("PATH") {
        if !cur_env_path.to_lowercase().contains(&lower_target) {
            env::set_var("PATH", format!("{};{}", target_norm, cur_env_path));
        }
    }

    log_fn(&format!("✅ 已成功將 [{target_norm}] 自動註冊至使用者環境變數 PATH 最前端！"), "success");
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
            0x0002,      // SMTO_ABORTIFHUNG
            3000,
            &mut result as *mut usize as _,
        );
    }
}
