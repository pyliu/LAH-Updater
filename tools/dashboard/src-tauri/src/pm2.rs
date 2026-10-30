use crate::services::CREATE_NO_WINDOW;
use crate::types::Pm2ProcessInfo;
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn resolve_pm2_cmd(root_dir: &Path) -> Option<PathBuf> {
    let cand = root_dir.join("node").join("pm2.cmd");
    if cand.is_file() {
        return Some(cand);
    }
    None
}

pub fn run_pm2_command(root_dir: &Path, args: &[&str]) -> (bool, String, String) {
    let Some(pm2_cmd) = resolve_pm2_cmd(root_dir) else {
        return (
            false,
            String::new(),
            format!("找不到可攜式 PM2 指令檔：{}\\node\\pm2.cmd", root_dir.display()),
        );
    };

    let pm2_home = root_dir.join(".pm2");
    let node_dir = root_dir.join("node");
    let apache_bin = root_dir.join("Apache24").join("bin");
    let current_path = std::env::var("PATH").unwrap_or_default();
    let unified_path = format!(
        "{};{};{}",
        node_dir.display(),
        apache_bin.display(),
        current_path
    );

    let mut cmd = Command::new("cmd.exe");
    cmd.arg("/C").arg(&pm2_cmd);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.env("PM2_HOME", &pm2_home);
    cmd.env("PATH", &unified_path);
    cmd.creation_flags(CREATE_NO_WINDOW);

    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            (output.status.success(), stdout, stderr)
        }
        Err(e) => (false, String::new(), e.to_string()),
    }
}

pub fn query_pm2_processes(root_dir: &Path) -> Result<Vec<Pm2ProcessInfo>, String> {
    let (ok, stdout, stderr) = run_pm2_command(root_dir, &["jlist"]);
    if !ok && stdout.trim().is_empty() {
        return Err(format!("PM2 查詢失敗：{}", stderr));
    }

    // PM2 可能在輸出 JSON 之前輸出一些前綴字元，搜尋第一個 '[' 和最後一個 ']'
    let start_idx = stdout.find('[').ok_or_else(|| {
        if stderr.is_empty() {
            "PM2 未回傳有效的進程陣列（可能尚未啟動守護行程）".to_string()
        } else {
            stderr.clone()
        }
    })?;
    let end_idx = stdout.rfind(']').ok_or_else(|| "PM2 JSON 結構不完整".to_string())?;

    let json_str = &stdout[start_idx..=end_idx];
    let parsed: Value = serde_json::from_str(json_str)
        .map_err(|e| format!("解析 PM2 進程清單 JSON 失敗：{}", e))?;

    let Some(arr) = parsed.as_array() else {
        return Ok(Vec::new());
    };

    let mut list = Vec::new();
    for item in arr {
        let pm_id = item.get("pm_id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let pid = item.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let env = item.get("pm2_env");
        let status = env
            .and_then(|e| e.get("status"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let uptime = env
            .and_then(|e| e.get("pm_uptime"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let restarts = env
            .and_then(|e| e.get("restart_time"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let monit = item.get("monit");
        let cpu = monit
            .and_then(|m| m.get("cpu"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let memory = monit
            .and_then(|m| m.get("memory"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        list.push(Pm2ProcessInfo {
            pm_id,
            name,
            status,
            pid,
            cpu,
            memory,
            uptime,
            restarts,
        });
    }

    Ok(list)
}

pub fn pm2_restart(root_dir: &Path, target: &str) -> Result<String, String> {
    let (ok, stdout, stderr) = run_pm2_command(root_dir, &["restart", target]);
    if ok {
        Ok(format!("PM2 重啟指令 [{}] 已成功送出！", target))
    } else {
        Err(format!("PM2 重啟失敗：{}\n{}", stderr, stdout))
    }
}

pub fn pm2_stop(root_dir: &Path, target: &str) -> Result<String, String> {
    let (ok, stdout, stderr) = run_pm2_command(root_dir, &["stop", target]);
    if ok {
        Ok(format!("PM2 停止指令 [{}] 已成功送出！", target))
    } else {
        Err(format!("PM2 停止失敗：{}\n{}", stderr, stdout))
    }
}

pub fn pm2_start(root_dir: &Path, target: &str) -> Result<String, String> {
    let (ok, stdout, stderr) = run_pm2_command(root_dir, &["start", target]);
    if ok {
        Ok(format!("PM2 啟動指令 [{}] 已成功送出！", target))
    } else {
        Err(format!("PM2 啟動失敗：{}\n{}", stderr, stdout))
    }
}

pub fn pm2_trigger_logrotate(root_dir: &Path) -> Result<String, String> {
    let (ok, stdout, stderr) = run_pm2_command(root_dir, &["trigger", "pm2-logrotate", "rotate"]);
    if ok {
        Ok("已成功觸發 pm2-logrotate 日誌輪替！".to_string())
    } else {
        Err(format!("觸發日誌輪替失敗：{}\n{}", stderr, stdout))
    }
}
