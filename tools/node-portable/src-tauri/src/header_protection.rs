use crate::system::CREATE_NO_WINDOW;
use crate::types::CommandActionResult;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

pub const GUARD_NAME: &str = "lah-http-header-guard.cjs";
const CONTROL_NAME: &str = "lah-pm2-header-control.cjs";
const GUARD: &[u8] = include_bytes!("../../service/lah-http-header-guard.cjs");
const CONTROL: &[u8] = include_bytes!("../../service/lah-pm2-header-control.cjs");

pub fn install(root: &Path) -> Result<(), String> {
    let node = root.join("node");
    if !node.is_dir() {
        return Err("找不到沙盒 Node 目錄".to_string());
    }
    for (name, content) in [(GUARD_NAME, GUARD), (CONTROL_NAME, CONTROL)] {
        let target = node.join(name);
        if fs::read(&target).ok().as_deref() != Some(content) {
            let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
            let temporary = node.join(format!(".{name}.{}.{}.tmp", std::process::id(), stamp));
            let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temporary)
                .map_err(|e| format!("無法建立標頭防護暫存檔 {name}：{e}"))?;
            let written = file.write_all(content).and_then(|_| file.sync_all());
            drop(file);
            if let Err(error) = written.and_then(|_| fs::rename(&temporary, &target)) {
                let _ = fs::remove_file(&temporary);
                return Err(format!("無法部署標頭防護檔案 {name}：{error}"));
            }
        }
    }
    Ok(())
}

fn normalized(value: &str) -> String {
    value.replace('\\', "/").to_ascii_lowercase()
}

fn is_own_path(value: &str) -> bool {
    normalized(value).rsplit('/').next() == Some(GUARD_NAME)
}

pub fn rebase_node_args(value: &mut Value, root: &Path) -> bool {
    let Some(args) = value.as_array_mut() else { return false };
    let target = root.join("node").join(GUARD_NAME).to_string_lossy().to_string();
    let mut changed = false;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str().unwrap_or("");
        if matches!(arg, "--require" | "-r") && index + 1 < args.len() {
            if let Some(previous) = args[index + 1].as_str() {
                if is_own_path(previous) && previous != target {
                    args[index + 1] = Value::String(target.clone());
                    changed = true;
                }
            }
            index += 2;
        } else {
            if let Some(previous) = arg.strip_prefix("--require=") {
                if is_own_path(previous) && previous != target {
                    args[index] = Value::String(format!("--require={target}"));
                    changed = true;
                }
            }
            index += 1;
        }
    }
    changed
}

pub fn process_state(root: &Path, env: Option<&Value>, is_module: bool) -> (bool, bool) {
    let interpreter = env.and_then(|v| v.get("exec_interpreter")).and_then(Value::as_str).unwrap_or("node");
    let interpreter = normalized(interpreter);
    let interpreter = interpreter.rsplit('/').next().unwrap_or("");
    let supported = !is_module && matches!(interpreter, "node" | "node.exe");
    let target = normalized(&root.join("node").join(GUARD_NAME).to_string_lossy());
    let enabled = env.and_then(|v| v.get("node_args")).and_then(Value::as_array).is_some_and(|args| {
        args.iter().enumerate().any(|(index, arg)| {
            let Some(arg) = arg.as_str() else { return false };
            if matches!(arg, "--require" | "-r") {
                args.get(index + 1).and_then(Value::as_str).is_some_and(|value| normalized(value) == target)
            } else {
                arg.strip_prefix("--require=").is_some_and(|value| normalized(value) == target)
            }
        })
    });
    (enabled, supported)
}

pub fn set(root: &Path, pm_id: u32, enabled: bool, expected_exec_path: &str) -> Result<CommandActionResult, String> {
    if expected_exec_path.trim().is_empty() {
        return Err("程序路徑不可為空，請重新整理後再操作".to_string());
    }
    install(root)?;
    let node = root.join("node");
    let mut child = Command::new(node.join("node.exe"))
        .arg(node.join(CONTROL_NAME))
        .arg(root)
        .arg(pm_id.to_string())
        .arg(if enabled { "enable" } else { "disable" })
        .arg(expected_exec_path)
        .env("PM2_HOME", root.join(".pm2"))
        .env("PATH", format!("{};{}", node.display(), std::env::var("PATH").unwrap_or_default()))
        .env_remove("PM2_INSTALL_DIRECTORY")
        .env_remove("PM2_SERVICE_DIRECTORY")
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("無法執行標頭防護操作：{e}"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < Duration::from_secs(30) => thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                crate::console::invalidate_pm2_cache();
                return Err("標頭防護操作逾時，已終止控制程序；服務設定可能已變更，請重新整理確認，勿直接重複操作".to_string());
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("無法取得標頭防護操作結果：{e}"));
            }
        }
    }
    let output = child.wait_with_output().map_err(|e| format!("無法讀取標頭防護操作結果：{e}"))?;
    crate::console::invalidate_pm2_cache();
    String::from_utf8_lossy(&output.stdout).lines().rev()
        .find_map(|line| serde_json::from_str::<CommandActionResult>(line).ok())
        .ok_or_else(|| "標頭防護操作未回傳有效結果，請檢查服務日誌".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migrates_only_own_preload_and_is_idempotent() {
        let root = Path::new(r"D:\New Root");
        let mut args = serde_json::json!(["--trace-warnings", "--require", r"C:\Old\node\lah-http-header-guard.cjs", "--require=C:/Old/node/lah-http-header-guard.cjs", "--require", "other.cjs"]);
        assert!(rebase_node_args(&mut args, root));
        let expected = root.join("node").join(GUARD_NAME).to_string_lossy().to_string();
        assert_eq!(args[2], expected);
        assert_eq!(args[3], format!("--require={expected}"));
        assert_eq!(args[5], "other.cjs");
        assert!(!rebase_node_args(&mut args, root));
    }
}
