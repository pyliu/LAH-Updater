use std::fs;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

pub const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn is_admin() -> bool {
    unsafe {
        windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0
    }
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
