use crate::services::CREATE_NO_WINDOW;
use crate::types::Pm2ProcessInfo;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::{copy, BufReader, BufWriter};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub fn resolve_pm2_cmd(root_dir: &Path) -> Option<PathBuf> {
    let cand = root_dir.join("node").join("pm2.cmd");
    if cand.is_file() {
        return Some(cand);
    }
    None
}

pub fn run_pm2_command(root_dir: &Path, args: &[&str]) -> (bool, String, String) {
    run_pm2_command_with_timeout(root_dir, args, Duration::from_secs(8))
}

pub fn run_pm2_command_with_timeout(root_dir: &Path, args: &[&str], timeout: Duration) -> (bool, String, String) {
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

    let (tx, rx) = mpsc::channel();
    let pm2_cmd_str = pm2_cmd.to_string_lossy().to_string();
    let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    let _handle = thread::spawn(move || {
        let mut cmd = Command::new("cmd.exe");
        cmd.arg("/C").arg(&pm2_cmd_str);
        for arg in &args_vec {
            cmd.arg(arg);
        }
        cmd.env("PM2_HOME", &pm2_home);
        cmd.env("PATH", &unified_path);
        cmd.creation_flags(CREATE_NO_WINDOW);

        let res = cmd.output();
        let _ = tx.send(res);
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            (output.status.success(), stdout, stderr)
        }
        Ok(Err(e)) => (false, String::new(), e.to_string()),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            (
                false,
                String::new(),
                format!("PM2 指令 [pm2 {:?}] 執行逾時 (未在 {} 秒內完成回應)", args, timeout.as_secs()),
            )
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            (false, String::new(), "執行緒異常中斷".to_string())
        }
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
    let timeout = if target == "all" { Duration::from_secs(15) } else { Duration::from_secs(8) };
    let (ok, stdout, stderr) = run_pm2_command_with_timeout(root_dir, &["restart", target], timeout);
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
    let now = chrono::Local::now();
    let timestamp = now.format("%Y-%m-%d_%H-%M-%S").to_string();

    let search_dirs = [
        root_dir.join("logs"),
        root_dir.join(".pm2").join("logs"),
        root_dir.join("Apache24").join("logs"),
    ];

    let min_size_bytes: u64 = 100 * 1024; // 100 KB 閥值
    let mut rotated_count = 0;
    let mut total_freed_bytes: u64 = 0;

    for dir in &search_dirs {
        if !dir.is_dir() {
            continue;
        }

        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };

            // 僅處理以 .log 結尾，且非歷史歸檔檔（不包含 '__' 且不以 .gz 結尾）
            if !file_name.ends_with(".log") || file_name.contains("__") || file_name.ends_with(".gz") {
                continue;
            }

            let metadata = match fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let file_size = metadata.len();
            if file_size < min_size_bytes {
                continue;
            }

            // 準備壓縮目標檔名: <stem>__<timestamp>.log.gz
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("log");
            let gz_name = format!("{}__{}.log.gz", stem, timestamp);
            let gz_path = dir.join(gz_name);

            // 執行原生 Gzip 串流壓縮
            let compress_success = (|| -> Result<(), std::io::Error> {
                let src_file = File::open(&path)?;
                let mut reader = BufReader::new(src_file);

                let dst_file = File::create(&gz_path)?;
                let writer = BufWriter::new(dst_file);
                let mut encoder = GzEncoder::new(writer, Compression::best());

                copy(&mut reader, &mut encoder)?;
                encoder.finish()?;
                Ok(())
            })();

            if compress_success.is_ok() {
                // 壓縮完成後截斷清空原日誌檔案
                if let Ok(file) = OpenOptions::new().write(true).truncate(true).open(&path) {
                    drop(file);
                    rotated_count += 1;
                    total_freed_bytes += file_size;
                } else {
                    let _ = fs::write(&path, b"");
                    rotated_count += 1;
                    total_freed_bytes += file_size;
                }
            }
        }
    }

    // 呼叫 pm2 reloadLogs 通知所有 PM2 進程安全釋放舊檔案句柄
    let _ = run_pm2_command(root_dir, &["reloadLogs"]);

    if rotated_count > 0 {
        let freed_mb = (total_freed_bytes as f64) / (1024.0 * 1024.0);
        Ok(format!(
            "已成功壓縮輪替 {} 個日誌檔案，釋放 {:.2} MB 磁碟空間，並已重載日誌句柄！",
            rotated_count, freed_mb
        ))
    } else {
        Ok("目前活躍日誌大小皆在 100 KB 以內，已為您執行 pm2 reloadLogs 保持日誌流暢。".to_string())
    }
}
