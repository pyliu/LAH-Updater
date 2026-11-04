use crate::install::{node_version, pm2_version};
use crate::pm2::{
    ensure_pm2_logrotate, is_logrotate_configured, is_logrotate_installed, is_pm2_service_installed,
    is_pm2_service_running, logrotate_bundled_version, resolve_pm2_command, setup_pm2_service,
    SERVICE_ID,
};
use crate::system::{ensure_portable_cmd_helper, run_cmd_hidden, CREATE_NO_WINDOW};
use crate::types::{
    CommandActionResult, ConsoleOverview, LogContentResult, LogFileInfo, Pm2ProcessInfo,
};
use chrono::{DateTime, Local};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{copy, BufRead, BufReader, BufWriter, Seek, SeekFrom};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

static PM2_JLIST_CACHE: OnceLock<Mutex<Option<(Instant, Vec<Pm2ProcessInfo>)>>> = OnceLock::new();

pub fn invalidate_pm2_cache() {
    if let Some(lock) = PM2_JLIST_CACHE.get() {
        if let Ok(mut guard) = lock.lock() {
            *guard = None;
        }
    }
}

pub fn run_pm2_sandbox_cmd(
    root_dir: &Path,
    args: &[&str],
    timeout: Duration,
) -> (bool, String, String) {
    let Some(pm2_cmd) = resolve_pm2_command(root_dir) else {
        return (
            false,
            String::new(),
            format!(
                "找不到沙盒 PM2 執行檔：{}\\node\\pm2.cmd",
                root_dir.display()
            ),
        );
    };

    let pm2_home = root_dir.join(".pm2");
    let node_dir = root_dir.join("node");
    let current_path = std::env::var("PATH").unwrap_or_default();
    let sandbox_path = format!("{};{}", node_dir.display(), current_path);

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
        cmd.env("PATH", &sandbox_path);
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
        Err(mpsc::RecvTimeoutError::Timeout) => (
            false,
            String::new(),
            format!(
                "PM2 指令 [pm2 {:?}] 執行逾時 (超過 {} 秒)",
                args,
                timeout.as_secs()
            ),
        ),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            (false, String::new(), "執行緒異常中斷".to_string())
        }
    }
}

pub fn query_pm2_processes(root_dir: &Path) -> Result<Vec<Pm2ProcessInfo>, String> {
    let cache_mutex = PM2_JLIST_CACHE.get_or_init(|| Mutex::new(None));
    let mut guard = cache_mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some((ts, ref cached_list)) = *guard {
        if ts.elapsed() < Duration::from_millis(900) {
            return Ok(cached_list.clone());
        }
    }

    let result = query_pm2_processes_uncached(root_dir);
    if let Ok(ref list) = result {
        *guard = Some((Instant::now(), list.clone()));
    }
    result
}

fn query_pm2_processes_uncached(root_dir: &Path) -> Result<Vec<Pm2ProcessInfo>, String> {
    let (ok, stdout, stderr) = run_pm2_sandbox_cmd(root_dir, &["jlist"], Duration::from_secs(12));
    if !ok && stdout.trim().is_empty() {
        return Err(format!("PM2 查詢失敗：{}", stderr.trim()));
    }

    let start_idx = stdout.find('[').ok_or_else(|| {
        if stderr.trim().is_empty() {
            "PM2 尚未回傳有效進程陣列（Daemon 可能尚未啟動）".to_string()
        } else {
            stderr.trim().to_string()
        }
    })?;
    let end_idx = stdout
        .rfind(']')
        .ok_or_else(|| "PM2 JSON 結構不完整".to_string())?;

    let json_str = &stdout[start_idx..=end_idx];
    let parsed: Value = serde_json::from_str(json_str)
        .map_err(|e| format!("解析 PM2 JSON 失敗：{}", e))?;

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

        let is_mod_flag = env
            .and_then(|e| e.get("pmx_module"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || env
                .and_then(|e| e.get("axm_options"))
                .and_then(|a| a.get("isModule"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        let is_module = is_mod_flag || name == "pm2-logrotate";

        let version = env
            .and_then(|e| e.get("version"))
            .and_then(|v| v.as_str())
            .or_else(|| {
                env.and_then(|e| e.get("axm_options"))
                    .and_then(|a| a.get("module_version"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("-")
            .to_string();

        let exec_path = env
            .and_then(|e| e.get("pm_exec_path"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let out_log_path = env
            .and_then(|e| e.get("pm_out_log_path"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let err_log_path = env
            .and_then(|e| e.get("pm_err_log_path"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

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
            is_module,
            version,
            exec_path,
            out_log_path,
            err_log_path,
        });
    }

    Ok(list)
}

fn is_logrotate_compress_enabled(root_dir: &Path) -> bool {
    let conf_path = root_dir.join(".pm2").join("module_conf.json");
    let Ok(raw) = fs::read_to_string(conf_path) else {
        return false;
    };
    let Ok(val) = serde_json::from_str::<Value>(&raw) else {
        return false;
    };
    val.get("pm2-logrotate")
        .and_then(|lr| lr.get("compress"))
        .and_then(|c| c.as_bool())
        .unwrap_or(false)
}

pub fn get_console_overview(root_dir: &Path) -> ConsoleOverview {
    let nv = node_version(root_dir);
    let pv = pm2_version(root_dir);
    let installed = nv.is_some() && pv.is_some();

    let (processes, pm2_error) = if installed {
        match query_pm2_processes(root_dir) {
            Ok(list) => (list, None),
            Err(e) => (Vec::new(), Some(e)),
        }
    } else {
        (Vec::new(), None)
    };

    ConsoleOverview {
        root_dir: root_dir.to_string_lossy().to_string(),
        installed,
        node_version: nv,
        pm2_version: pv,
        service_installed: is_pm2_service_installed(),
        service_running: is_pm2_service_running(),
        logrotate_installed: is_logrotate_installed(root_dir),
        logrotate_configured: is_logrotate_configured(root_dir),
        logrotate_version: logrotate_bundled_version(root_dir),
        logrotate_compress: is_logrotate_compress_enabled(root_dir),
        processes,
        pm2_error,
    }
}

pub fn pm2_process_action(root_dir: &Path, action: &str, target: &str) -> CommandActionResult {
    let timeout = if target == "all" || action == "resurrect" {
        Duration::from_secs(30)
    } else {
        Duration::from_secs(15)
    };

    let (ok, stdout, stderr) = match action {
        "start" => run_pm2_sandbox_cmd(root_dir, &["start", target, "--no-color"], timeout),
        "stop" => run_pm2_sandbox_cmd(root_dir, &["stop", target, "--no-color"], timeout),
        "restart" => run_pm2_sandbox_cmd(root_dir, &["restart", target, "--no-color"], timeout),
        "reload" => run_pm2_sandbox_cmd(root_dir, &["reload", target, "--no-color"], timeout),
        "save" => run_pm2_sandbox_cmd(root_dir, &["save", "--force", "--no-color"], timeout),
        "resurrect" => run_pm2_sandbox_cmd(root_dir, &["resurrect", "--no-color"], timeout),
        _ => {
            return CommandActionResult {
                success: false,
                message: format!("不支援的 PM2 動作：{}", action),
            }
        }
    };

    invalidate_pm2_cache();

    let action_label = match action {
        "start" => format!("啟動 [{}]", target),
        "stop" => format!("停止 [{}]", target),
        "restart" => format!("重啟 [{}]", target),
        "reload" => format!("平滑重載 [{}]", target),
        "save" => "儲存 PM2 進程快照 (dump.pm2)".to_string(),
        "resurrect" => "還原 PM2 進程快照 (resurrect)".to_string(),
        _ => action.to_string(),
    };

    if ok {
        CommandActionResult {
            success: true,
            message: format!("✅ 已完成：{}", action_label),
        }
    } else {
        CommandActionResult {
            success: false,
            message: format!(
                "⚠️ {} 執行失敗：{}\n{}",
                action_label,
                stderr.trim(),
                stdout.trim()
            ),
        }
    }
}

pub fn pm2_service_action(root_dir: &Path, action: &str) -> CommandActionResult {
    let svc_exe = root_dir.join("node").join("service").join("pm2-service.exe");

    match action {
        "register" => {
            let logs = RefCell::new(Vec::new());
            let ok = setup_pm2_service(root_dir, &|m, _| logs.borrow_mut().push(m.to_string()));
            invalidate_pm2_cache();
            CommandActionResult {
                success: ok,
                message: if ok {
                    format!("✅ {} 服務已註冊並啟動", SERVICE_ID)
                } else {
                    format!("⚠️ 註冊服務失敗：\n{}", logs.into_inner().join("\n"))
                },
            }
        }
        "start" => {
            if !is_pm2_service_installed() {
                let logs = RefCell::new(Vec::new());
                let ok = setup_pm2_service(root_dir, &|m, _| logs.borrow_mut().push(m.to_string()));
                invalidate_pm2_cache();
                return CommandActionResult {
                    success: ok,
                    message: if ok {
                        format!("✅ 已自動註冊並啟動服務 {}", SERVICE_ID)
                    } else {
                        format!(
                            "⚠️ 啟動服務失敗（請確認是否以系統管理員身分執行）：\n{}",
                            logs.into_inner().join("\n")
                        )
                    },
                };
            }
            let (ok, out, err) = if svc_exe.is_file() {
                run_cmd_hidden(&svc_exe.to_string_lossy(), &["start"])
            } else {
                run_cmd_hidden("net.exe", &["start", SERVICE_ID])
            };
            invalidate_pm2_cache();
            let running = is_pm2_service_running();
            CommandActionResult {
                success: ok || running,
                message: if ok || running {
                    format!("✅ 服務 {} 已啟動", SERVICE_ID)
                } else {
                    format!("⚠️ 啟動服務失敗：{} {}", err.trim(), out.trim())
                },
            }
        }
        "stop" => {
            let (ok, out, err) = if svc_exe.is_file() {
                run_cmd_hidden(&svc_exe.to_string_lossy(), &["stop"])
            } else {
                run_cmd_hidden("net.exe", &["stop", SERVICE_ID])
            };
            for _ in 0..10 {
                if !is_pm2_service_running() {
                    break;
                }
                thread::sleep(Duration::from_millis(300));
            }
            invalidate_pm2_cache();
            let stopped = !is_pm2_service_running();
            CommandActionResult {
                success: ok || stopped,
                message: if ok || stopped {
                    format!("✅ 服務 {} 已停止", SERVICE_ID)
                } else {
                    format!("⚠️ 停止服務失敗：{} {}", err.trim(), out.trim())
                },
            }
        }
        "restart" => {
            let _ = pm2_service_action(root_dir, "stop");
            thread::sleep(Duration::from_millis(500));
            let res = pm2_service_action(root_dir, "start");
            CommandActionResult {
                success: res.success,
                message: if res.success {
                    format!("✅ 服務 {} 已重新啟動", SERVICE_ID)
                } else {
                    res.message
                },
            }
        }
        _ => CommandActionResult {
            success: false,
            message: format!("不支援的服務動作：{}", action),
        },
    }
}

pub fn repair_or_restart_logrotate(root_dir: &Path) -> CommandActionResult {
    let logs = RefCell::new(Vec::new());
    let ok = ensure_pm2_logrotate(root_dir, &|m, _| logs.borrow_mut().push(m.to_string()));
    invalidate_pm2_cache();
    CommandActionResult {
        success: ok,
        message: if ok {
            "✅ 已完成沙盒內建 pm2-logrotate 模組掛載與喚醒（已啟用 compress: true）".to_string()
        } else {
            format!("⚠️ 啟動 pm2-logrotate 異常：\n{}", logs.into_inner().join("\n"))
        },
    }
}

pub fn trigger_logrotate_now(root_dir: &Path) -> CommandActionResult {
    let now = Local::now();
    let timestamp = now.format("%Y-%m-%d_%H-%M-%S").to_string();

    let search_dirs = [root_dir.join("logs"), root_dir.join(".pm2").join("logs")];

    let min_size_bytes: u64 = 100 * 1024; // 100 KB 閥值
    let mut rotated_count = 0;
    let mut total_freed_bytes: u64 = 0;

    for dir in &search_dirs {
        if !dir.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !file_name.ends_with(".log")
                || file_name.contains("__")
                || file_name.ends_with(".gz")
            {
                continue;
            }
            let Ok(metadata) = fs::metadata(&path) else {
                continue;
            };
            let file_size = metadata.len();
            if file_size < min_size_bytes {
                continue;
            }

            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("log");
            let gz_name = format!("{}__{}.log.gz", stem, timestamp);
            let gz_path = dir.join(gz_name);

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

    let _ = run_pm2_sandbox_cmd(root_dir, &["reloadLogs"], Duration::from_secs(10));

    if rotated_count > 0 {
        let freed_mb = (total_freed_bytes as f64) / (1024.0 * 1024.0);
        CommandActionResult {
            success: true,
            message: format!(
                "✅ 已成功 Gzip 壓縮輪替 {} 個日誌檔案（釋放 {:.2} MB），並已執行 pm2 reloadLogs！",
                rotated_count, freed_mb
            ),
        }
    } else {
        CommandActionResult {
            success: true,
            message: "✅ 目前活躍日誌皆小於 100 KB，已執行 pm2 reloadLogs 重新整理日誌句柄。"
                .to_string(),
        }
    }
}

fn build_log_info(key: String, label: String, path: &Path) -> LogFileInfo {
    let exists = path.is_file();
    let mut size_bytes = 0;
    let mut modified = "-".to_string();

    if exists {
        if let Ok(meta) = fs::metadata(path) {
            size_bytes = meta.len();
            if size_bytes == 0 {
                if let Ok(real_size) =
                    File::open(path).and_then(|mut f| f.seek(SeekFrom::End(0)))
                {
                    size_bytes = real_size;
                }
            }
            if let Ok(mtime) = meta.modified() {
                let dt: DateTime<Local> = mtime.into();
                modified = dt.format("%Y-%m-%d %H:%M:%S").to_string();
            }
        }
    }

    LogFileInfo {
        key,
        label,
        path: path.to_string_lossy().to_string(),
        exists,
        size_bytes,
        modified,
    }
}

pub fn list_sandbox_logs(root_dir: &Path) -> Vec<LogFileInfo> {
    let mut result = Vec::new();
    let mut seen_paths = HashSet::new();

    let mut add_entry = |key: String, label: String, p: PathBuf| {
        let norm = p.to_string_lossy().to_lowercase();
        if seen_paths.insert(norm) {
            result.push(build_log_info(key, label, &p));
        }
    };

    // 1. 優先納入目前 PM2 受管進程的標準輸出與錯誤日誌
    if let Ok(procs) = query_pm2_processes(root_dir) {
        for proc in procs {
            if !proc.out_log_path.is_empty() {
                add_entry(
                    format!("pm2_{}_out", proc.name),
                    format!("[PM2] {} - 標準輸出 (out.log)", proc.name),
                    PathBuf::from(&proc.out_log_path),
                );
            }
            if !proc.err_log_path.is_empty() {
                add_entry(
                    format!("pm2_{}_err", proc.name),
                    format!("[PM2] {} - 錯誤日誌 (err.log)", proc.name),
                    PathBuf::from(&proc.err_log_path),
                );
            }
        }
    }

    // 2. 納入 .pm2\logs 下的日誌（如 pm2.log、pm2-logrotate 等）
    let pm2_logs_dir = root_dir.join(".pm2").join("logs");
    let pm2_daemon_log = root_dir.join(".pm2").join("pm2.log");
    if pm2_daemon_log.is_file() {
        add_entry(
            "pm2_daemon".to_string(),
            "[Daemon] PM2 核心守護行程日誌 (pm2.log)".to_string(),
            pm2_daemon_log,
        );
    }

    let scan_dirs = [
        ("PM2 日誌", pm2_logs_dir),
        ("服務包裝日誌", root_dir.join("logs").join("pm2-service")),
        ("系統日誌", root_dir.join("logs")),
    ];

    for (prefix_label, dir) in scan_dirs {
        if !dir.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.ends_with(".log") && !n.contains("__"))
                        .unwrap_or(false)
            })
            .collect();

        files.sort_by(|a, b| {
            let mb = fs::metadata(b)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let ma = fs::metadata(a)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            mb.cmp(&ma)
        });

        for f in files {
            let fname = f
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("log.log")
                .to_string();
            add_entry(
                format!("file_{}", fname),
                format!("[{}] {}", prefix_label, fname),
                f,
            );
        }
    }

    result
}

#[cfg(target_os = "windows")]
fn decode_cp950(bytes: &[u8]) -> Option<String> {
    extern "system" {
        fn MultiByteToWideChar(
            CodePage: u32,
            dwFlags: u32,
            lpMultiByteStr: *const u8,
            cbMultiByte: i32,
            lpWideCharStr: *mut u16,
            cchWideChar: i32,
        ) -> i32;
    }
    const CP_BIG5: u32 = 950;
    const MB_ERR_INVALID_CHARS: u32 = 0x00000008;

    if bytes.is_empty() {
        return Some(String::new());
    }
    unsafe {
        let len = MultiByteToWideChar(
            CP_BIG5,
            MB_ERR_INVALID_CHARS,
            bytes.as_ptr(),
            bytes.len() as i32,
            std::ptr::null_mut(),
            0,
        );
        if len <= 0 {
            return None;
        }
        let mut wide = vec![0u16; len as usize];
        let written = MultiByteToWideChar(
            CP_BIG5,
            MB_ERR_INVALID_CHARS,
            bytes.as_ptr(),
            bytes.len() as i32,
            wide.as_mut_ptr(),
            len,
        );
        if written <= 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&wide[..written as usize]))
    }
}

fn decode_log_line(mut bytes: Vec<u8>) -> String {
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    if let Ok(s) = std::str::from_utf8(&bytes) {
        return s.to_string();
    }
    #[cfg(target_os = "windows")]
    if let Some(s) = decode_cp950(&bytes) {
        return s;
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

pub fn read_log_tail(path_str: &str, max_lines: usize) -> Result<LogContentResult, String> {
    let p = Path::new(path_str);
    if !p.is_file() {
        return Ok(LogContentResult {
            path: path_str.to_string(),
            lines: vec![format!("【提示】日誌檔案尚未產生：{}", path_str)],
            file_size: 0,
            exists: false,
        });
    }

    let mut file = File::open(p).map_err(|e| format!("開啟日誌檔案失敗：{}", e))?;
    let file_size = file
        .seek(SeekFrom::End(0))
        .or_else(|_| fs::metadata(p).map(|m| m.len()))
        .map_err(|e| format!("無法讀取日誌大小：{}", e))?;

    if file_size == 0 {
        return Ok(LogContentResult {
            path: path_str.to_string(),
            lines: vec!["【提示】日誌檔案目前為空 (0 bytes)".to_string()],
            file_size: 0,
            exists: true,
        });
    }

    let read_start = if file_size > 524_288 {
        file_size - 524_288
    } else {
        0
    };
    file.seek(SeekFrom::Start(read_start))
        .map_err(|e| format!("位移失敗：{}", e))?;

    let reader = BufReader::new(file);
    let mut all_lines: Vec<String> = Vec::new();
    for byte_res in reader.split(b'\n') {
        if let Ok(bytes) = byte_res {
            all_lines.push(decode_log_line(bytes));
        }
    }

    let total = all_lines.len();
    let lines = if total > max_lines {
        all_lines[total - max_lines..].to_vec()
    } else {
        all_lines
    };

    Ok(LogContentResult {
        path: path_str.to_string(),
        lines,
        file_size,
        exists: true,
    })
}

pub fn open_in_notepad(path_str: &str) -> CommandActionResult {
    let p = Path::new(path_str);
    if !p.is_file() {
        return CommandActionResult {
            success: false,
            message: format!("找不到日誌檔案：{}", path_str),
        };
    }

    match Command::new("notepad.exe").arg(path_str).spawn() {
        Ok(_) => CommandActionResult {
            success: true,
            message: "已啟動記事本開啟日誌檔。".to_string(),
        },
        Err(e) => CommandActionResult {
            success: false,
            message: format!("啟動記事本失敗：{}", e),
        },
    }
}

pub fn open_logs_folder(root_dir: &Path) -> CommandActionResult {
    let pm2_logs = root_dir.join(".pm2").join("logs");
    let root_logs = root_dir.join("logs");
    let target = if pm2_logs.is_dir() {
        pm2_logs
    } else if root_logs.is_dir() {
        root_logs
    } else {
        root_dir.to_path_buf()
    };

    match Command::new("explorer.exe")
        .arg(target.to_string_lossy().as_ref())
        .spawn()
    {
        Ok(_) => CommandActionResult {
            success: true,
            message: format!("已開啟日誌資料夾：{}", target.display()),
        },
        Err(e) => CommandActionResult {
            success: false,
            message: format!("開啟日誌資料夾失敗：{}", e),
        },
    }
}

pub fn open_lah_cmd(root_dir: &Path) -> CommandActionResult {
    let bat = root_dir.join("lah-cmd.bat");
    if !bat.is_file() {
        ensure_portable_cmd_helper(root_dir, &|_, _| {});
    }
    if !bat.is_file() {
        return CommandActionResult {
            success: false,
            message: format!("找不到沙盒終端批次檔：{}", bat.display()),
        };
    }

    match Command::new("cmd.exe")
        .args([
            "/C",
            "start",
            "LAH 可攜 Node 沙盒終端",
            bat.to_string_lossy().as_ref(),
        ])
        .current_dir(root_dir)
        .spawn()
    {
        Ok(_) => CommandActionResult {
            success: true,
            message: "已開啟專屬沙盒維運終端 (lah-cmd.bat)".to_string(),
        },
        Err(e) => CommandActionResult {
            success: false,
            message: format!("開啟沙盒終端失敗：{}", e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_log_tail_and_logrotate_compression() {
        let tmp = std::env::temp_dir().join(format!("lah_test_con_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let logs_dir = tmp.join(".pm2").join("logs");
        fs::create_dir_all(&logs_dir).unwrap();

        let log_file = logs_dir.join("app-out.log");
        let mut content = String::new();
        for i in 1..=2500 {
            content.push_str(&format!("Line {:04}: 測試沙盒日誌輸出內容與壓縮輪替驗證\r\n", i));
        }
        fs::write(&log_file, content.as_bytes()).unwrap();

        // 1. 驗證 read_log_tail 只取最後 N 行且內容正確
        let tail = read_log_tail(&log_file.to_string_lossy(), 50).unwrap();
        assert!(tail.exists);
        assert_eq!(tail.lines.len(), 50);
        assert!(tail.lines.last().unwrap().contains("Line 2500"));

        // 2. 驗證 trigger_logrotate_now 會將超過 100KB 的日誌壓縮為 .log.gz 並清空原檔
        let res = trigger_logrotate_now(&tmp);
        assert!(res.success);
        assert!(res.message.contains("Gzip 壓縮輪替"));

        let after_size = fs::metadata(&log_file).unwrap().len();
        assert_eq!(after_size, 0);

        let gz_exists = fs::read_dir(&logs_dir)
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().ends_with(".log.gz"));
        assert!(gz_exists);

        let _ = fs::remove_dir_all(&tmp);
    }
}
