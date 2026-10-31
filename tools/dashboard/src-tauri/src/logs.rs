use crate::types::{LogContentResult, LogFileInfo};
use chrono::{DateTime, Local};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;

fn find_latest_matching_file(dir: &Path, prefix: &str, suffix: &str) -> Option<PathBuf> {
    find_all_matching_files(dir, prefix, suffix).into_iter().next()
}

fn find_all_matching_files(dir: &Path, prefix: &str, suffix: &str) -> Vec<PathBuf> {
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if (prefix.is_empty() || name.starts_with(prefix))
                        && (suffix.is_empty() || name.ends_with(suffix))
                    {
                        files.push(path);
                    }
                }
            }
        }
    }

    files.sort_by(|a, b| {
        let meta_b = fs::metadata(b)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let meta_a = fs::metadata(a)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        meta_b.cmp(&meta_a)
    });

    files
}

fn build_log_info(key: String, label: String, path_str: String, exists: bool) -> LogFileInfo {
    let mut size_bytes = 0;
    let mut modified = "-".to_string();

    if exists {
        if let Ok(meta) = fs::metadata(&path_str) {
            size_bytes = meta.len();
            if let Ok(mtime) = meta.modified() {
                let dt: DateTime<Local> = mtime.into();
                modified = dt.format("%Y-%m-%d %H:%M:%S").to_string();
            }
        }
    }

    LogFileInfo {
        key,
        label,
        path: path_str,
        exists,
        size_bytes,
        modified,
    }
}

pub fn list_available_logs(root_dir: &Path) -> Vec<LogFileInfo> {
    let logs_dir = root_dir.join("logs");
    let pm2_logs_dir = root_dir.join(".pm2").join("logs");
    let apache_logs_dir = root_dir.join("Apache24").join("logs");
    let api_logs_dir = logs_dir.join("api");

    let before_api_targets = [
        ("fe_out", "LAH-FE 前端標準輸出", &logs_dir, "fe_out", ".log"),
        ("fe_err", "LAH-FE 前端錯誤日誌", &logs_dir, "fe_err", ".log"),
        (
            "wss_out",
            "LAH-WSS 即時通標準輸出",
            &logs_dir,
            "wss_out",
            ".log",
        ),
        (
            "wss_err",
            "LAH-WSS 即時通錯誤日誌",
            &logs_dir,
            "wss_err",
            ".log",
        ),
        (
            "scheduler_out",
            "排程服務標準輸出",
            &logs_dir,
            "scheduler_out",
            ".log",
        ),
        (
            "scheduler_err",
            "排程服務錯誤日誌",
            &logs_dir,
            "scheduler_err",
            ".log",
        ),
        (
            "api_out",
            "PHP 8.5 FastCGI 標準輸出",
            &pm2_logs_dir,
            "lah-api-v85-out",
            ".log",
        ),
        (
            "api_err",
            "PHP 8.5 FastCGI 錯誤日誌",
            &pm2_logs_dir,
            "lah-api-v85-error",
            ".log",
        ),
    ];

    let after_api_targets = [
        (
            "apache_error",
            "Apache 2.4 錯誤日誌",
            &apache_logs_dir,
            "error_log",
            "",
        ),
        (
            "apache_access",
            "Apache 2.4 存取紀錄",
            &apache_logs_dir,
            "access_log",
            "",
        ),
        (
            "service_pm2",
            "LAH_PM2_Service 包裝日誌",
            &logs_dir.join("service"),
            "wrapper",
            ".log",
        ),
        (
            "service_web",
            "LAH_Web_Service 包裝日誌",
            &logs_dir.join("web_service"),
            "wrapper",
            ".log",
        ),
        (
            "pm2_logrotate",
            "PM2 Logrotate 運作日誌",
            &pm2_logs_dir,
            "pm2-logrotate-out",
            ".log",
        ),
    ];

    let mut result = Vec::new();

    // 1. 前端、即時通、排程與 PM2 FastCGI
    for (key, label, dir, prefix, suffix) in before_api_targets {
        let file_path = if suffix.is_empty() {
            let p = dir.join(prefix);
            if p.is_file() {
                Some(p)
            } else {
                None
            }
        } else {
            find_latest_matching_file(dir, prefix, suffix)
        };

        let exists = file_path.is_some();
        let path_str = file_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| dir.join(format!("{}{}", prefix, suffix)).to_string_lossy().to_string());

        result.push(build_log_info(key.to_string(), label.to_string(), path_str, exists));
    }

    // 2. LAH-API 核心業務日誌 (logs\api)
    let api_log_files = find_all_matching_files(&api_logs_dir, "", ".log");
    if api_log_files.is_empty() {
        let fallback_path = api_logs_dir.join(format!("log-{}.log", Local::now().format("%Y-%m-%d")));
        result.push(build_log_info(
            "api_www_log".to_string(),
            "LAH-API 核心業務日誌 (logs\\api)".to_string(),
            fallback_path.to_string_lossy().to_string(),
            false,
        ));
    } else {
        // 最多納入最新 5 筆日誌檔案（通常為今天與最近數日）
        for (idx, p) in api_log_files.into_iter().take(5).enumerate() {
            let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("log.log");
            let label = if idx == 0 {
                format!("LAH-API 核心業務日誌 (logs\\api: 最新 {})", file_name)
            } else {
                format!("LAH-API 歷史業務日誌 (logs\\api: {})", file_name)
            };

            let path_str = p.to_string_lossy().to_string();
            result.push(build_log_info(
                format!("api_www_log_{}", idx),
                label,
                path_str,
                true,
            ));
        }
    }

    // 3. Apache 2.4、Windows 系統服務與 PM2 日誌輪替
    for (key, label, dir, prefix, suffix) in after_api_targets {
        let file_path = if suffix.is_empty() {
            let p = dir.join(prefix);
            if p.is_file() {
                Some(p)
            } else {
                None
            }
        } else {
            find_latest_matching_file(dir, prefix, suffix)
        };

        let exists = file_path.is_some();
        let path_str = file_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| dir.join(format!("{}{}", prefix, suffix)).to_string_lossy().to_string());

        result.push(build_log_info(key.to_string(), label.to_string(), path_str, exists));
    }

    result
}

pub fn read_log_tail(path_str: &str, max_lines: usize) -> Result<LogContentResult, String> {
    let p = Path::new(path_str);
    if !p.is_file() {
        return Ok(LogContentResult {
            key: String::new(),
            path: path_str.to_string(),
            lines: vec![format!("【提示】日誌檔案尚未建立：{}", path_str)],
            file_size: 0,
            exists: false,
        });
    }

    let meta = fs::metadata(p).map_err(|e| format!("無法取得日誌屬性：{}", e))?;
    let file_size = meta.len();
    if file_size == 0 {
        return Ok(LogContentResult {
            key: String::new(),
            path: path_str.to_string(),
            lines: vec!["【提示】日誌檔案內容為空 (0 bytes)".to_string()],
            file_size: 0,
            exists: true,
        });
    }

    let mut file = File::open(p).map_err(|e| format!("開啟檔案失敗：{}", e))?;

    // 針對大檔案，僅從末尾 512KB 開始讀取
    let read_start = if file_size > 524_288 {
        file_size - 524_288
    } else {
        0
    };
    file.seek(SeekFrom::Start(read_start))
        .map_err(|e| format!("位移失敗：{}", e))?;

    let reader = BufReader::new(file);
    let mut all_lines: Vec<String> = Vec::new();
    for line_res in reader.lines() {
        match line_res {
            Ok(line) => all_lines.push(line),
            Err(_) => {
                // 若遇非 UTF-8，以替代符號安全處理
                all_lines.push("[Non-UTF8 line skipped]".to_string());
            }
        }
    }

    let total = all_lines.len();
    let lines = if total > max_lines {
        all_lines[total - max_lines..].to_vec()
    } else {
        all_lines
    };

    Ok(LogContentResult {
        key: String::new(),
        path: path_str.to_string(),
        lines,
        file_size,
        exists: true,
    })
}

pub fn open_in_notepad(path_str: &str) -> Result<(), String> {
    let p = Path::new(path_str);
    if !p.is_file() {
        return Err(format!("找不到檔案：{}", path_str));
    }

    Command::new("notepad.exe")
        .arg(path_str)
        .spawn()
        .map_err(|e| format!("無法啟動記事本：{}", e))?;

    Ok(())
}

pub fn open_folder(path_str: &str) -> Result<(), String> {
    let p = Path::new(path_str);
    let target = if p.is_dir() {
        p
    } else if let Some(parent) = p.parent() {
        parent
    } else {
        p
    };

    Command::new("explorer.exe")
        .arg(target.to_string_lossy().as_ref())
        .spawn()
        .map_err(|e| format!("無法開啟目錄：{}", e))?;

    Ok(())
}
