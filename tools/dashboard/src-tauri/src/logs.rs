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
            if size_bytes == 0 {
                if let Ok(real_size) = File::open(&path_str).and_then(|mut f| f.seek(SeekFrom::End(0))) {
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
        path: path_str,
        exists,
        size_bytes,
        modified,
    }
}

pub fn list_available_logs(root_dir: &Path) -> Vec<LogFileInfo> {
    let logs_dir = root_dir.join("logs");
    let pm2_logs_dir = root_dir.join(".pm2").join("logs");
    let unified_apache_dir = logs_dir.join("apache");
    let legacy_apache_dir = root_dir.join("Apache24").join("logs");
    let apache_logs_dir = if unified_apache_dir.is_dir() {
        unified_apache_dir
    } else {
        legacy_apache_dir.clone()
    };
    let api_logs_dir = logs_dir.join("api");

    let before_api_targets = [
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
            "error",
            ".log",
            Some(("error_log", "")),
        ),
        (
            "apache_access",
            "Apache 2.4 存取紀錄",
            &apache_logs_dir,
            "access",
            ".log",
            Some(("access_log", "")),
        ),
        (
            "service_pm2",
            "LAH_PM2_Service 包裝日誌",
            &logs_dir.join("service"),
            "lah-service",
            ".log",
            Some(("wrapper", ".log")),
        ),
        (
            "service_web",
            "LAH_Web_Service 包裝日誌",
            &logs_dir.join("web_service"),
            "lah-web-service",
            ".log",
            Some(("wrapper", ".log")),
        ),
        (
            "pm2_logrotate",
            "PM2 Logrotate 運作日誌",
            &pm2_logs_dir,
            "pm2-logrotate-out",
            ".log",
            None,
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
        // 主下拉選單回歸單一對稱項目，預設指向最新一筆日誌檔案
        let latest_file = &api_log_files[0];
        let file_name = latest_file.file_name().and_then(|n| n.to_str()).unwrap_or("log.log");
        let path_str = latest_file.to_string_lossy().to_string();
        result.push(build_log_info(
            "api_www_log".to_string(),
            format!("LAH-API 核心業務日誌 (logs\\api: 最新 {})", file_name),
            path_str,
            true,
        ));
    }

    // 3. Apache 2.4、Windows 系統服務與 PM2 日誌輪替
    for (key, label, dir, prefix, suffix, fallback) in after_api_targets {
        let mut file_path = if suffix.is_empty() {
            let p = dir.join(prefix);
            if p.is_file() {
                Some(p)
            } else {
                None
            }
        } else {
            find_latest_matching_file(dir, prefix, suffix)
        };

        if file_path.is_none() {
            if let Some((fb_prefix, fb_suffix)) = fallback {
                file_path = if fb_suffix.is_empty() {
                    let p = dir.join(fb_prefix);
                    if p.is_file() {
                        Some(p)
                    } else {
                        None
                    }
                } else {
                    find_latest_matching_file(dir, fb_prefix, fb_suffix)
                };
            }
            if file_path.is_none() && (key == "apache_error" || key == "apache_access") {
                file_path = find_latest_matching_file(&legacy_apache_dir, prefix, suffix);
                if file_path.is_none() {
                    if let Some((fb_prefix, fb_suffix)) = fallback {
                        file_path = if fb_suffix.is_empty() {
                            let p = legacy_apache_dir.join(fb_prefix);
                            if p.is_file() {
                                Some(p)
                            } else {
                                None
                            }
                        } else {
                            find_latest_matching_file(&legacy_apache_dir, fb_prefix, fb_suffix)
                        };
                    }
                }
            }
        }

        let exists = file_path.is_some();
        let path_str = file_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| dir.join(format!("{}{}", prefix, suffix)).to_string_lossy().to_string());

        result.push(build_log_info(key.to_string(), label.to_string(), path_str, exists));
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
            key: String::new(),
            path: path_str.to_string(),
            lines: vec![format!("【提示】日誌檔案尚未建立：{}", path_str)],
            file_size: 0,
            exists: false,
        });
    }

    let mut file = File::open(p).map_err(|e| format!("開啟檔案失敗：{}", e))?;
    let file_size = file
        .seek(SeekFrom::End(0))
        .or_else(|_| fs::metadata(p).map(|m| m.len()))
        .map_err(|e| format!("無法取得日誌屬性：{}", e))?;
    if file_size == 0 {
        return Ok(LogContentResult {
            key: String::new(),
            path: path_str.to_string(),
            lines: vec!["【提示】日誌檔案內容為空 (0 bytes)".to_string()],
            file_size: 0,
            exists: true,
        });
    }

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

pub fn list_api_log_files(root_dir: &Path) -> Vec<LogFileInfo> {
    let api_logs_dir = root_dir.join("logs").join("api");
    let files = find_all_matching_files(&api_logs_dir, "", ".log");
    let mut result = Vec::new();

    for (idx, p) in files.into_iter().enumerate() {
        let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("log.log");
        let date_str = file_name
            .trim_start_matches("log-")
            .trim_end_matches(".log");
        let label = if idx == 0 {
            format!("{} (最新)", date_str)
        } else {
            date_str.to_string()
        };
        let path_str = p.to_string_lossy().to_string();
        result.push(build_log_info(
            format!("api_log_{}", date_str),
            label,
            path_str,
            true,
        ));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_available_logs_falls_back_to_legacy_apache_plain_logs_when_unified_dir_is_empty() {
        let temp_root = std::env::temp_dir().join(format!(
            "lah_test_dashboard_logs_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let unified_apache_dir = temp_root.join("logs").join("apache");
        let legacy_apache_dir = temp_root.join("Apache24").join("logs");
        fs::create_dir_all(&unified_apache_dir).unwrap();
        fs::create_dir_all(&legacy_apache_dir).unwrap();

        fs::write(legacy_apache_dir.join("error_log"), b"legacy error\n").unwrap();
        fs::write(legacy_apache_dir.join("access_log"), b"legacy access\n").unwrap();

        let logs = list_available_logs(&temp_root);
        let apache_err = logs.iter().find(|l| l.key == "apache_error").unwrap();
        let apache_acc = logs.iter().find(|l| l.key == "apache_access").unwrap();
        assert!(apache_err.exists);
        assert!(apache_err.path.ends_with("error_log"));
        assert!(apache_acc.exists);
        assert!(apache_acc.path.ends_with("access_log"));

        fs::write(unified_apache_dir.join("error.2026.11.05.log"), b"unified error\n").unwrap();
        fs::write(unified_apache_dir.join("access.2026.11.05.log"), b"unified access\n").unwrap();
        let logs2 = list_available_logs(&temp_root);
        let apache_err2 = logs2.iter().find(|l| l.key == "apache_error").unwrap();
        let apache_acc2 = logs2.iter().find(|l| l.key == "apache_access").unwrap();
        assert!(apache_err2.exists);
        assert!(apache_err2.path.ends_with("error.2026.11.05.log"));
        assert!(apache_acc2.exists);
        assert!(apache_acc2.path.ends_with("access.2026.11.05.log"));

        let _ = fs::remove_dir_all(&temp_root);
    }
}
