use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::Serialize;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// 單一 Log 檔防爆上限：50 MB
pub const DEFAULT_MAX_LOG_BYTES: u64 = 50 * 1024 * 1024;
/// 超過此天數即壓縮打包
pub const LOG_COMPRESS_AFTER_DAYS: i64 = 3;
/// 超過此天數即刪除（含壓縮檔與未壓縮檔）
pub const LOG_DELETE_AFTER_DAYS: i64 = 30;
/// 保留最近 N 份 `env.properties.bak.*` 備份
pub const DEFAULT_MAX_ENV_BACKUPS: usize = 10;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LogCleanupReport {
    pub compressed_count: usize,
    pub deleted_logs_count: usize,
    pub deleted_zips_count: usize,
    pub deleted_backups_count: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq)]
pub struct DailyLogTail {
    pub file_path: String,
    pub file_size: u64,
    pub total_lines: usize,
    pub lines: Vec<String>,
}

/// 取得台灣時間（UTC+8）的 `(year, month, day, hour, min, sec, ms)`。
pub fn tw_datetime_parts(now_ms: u64) -> (i64, u32, u32, i64, i64, i64, u64) {
    let total_secs = (now_ms / 1000) as i64 + 8 * 3600;
    let ms = now_ms % 1000;
    let days = total_secs.div_euclid(86_400);
    let secs_of_day = total_secs.rem_euclid(86_400);
    let hour = secs_of_day / 3600;
    let min = (secs_of_day % 3600) / 60;
    let sec = secs_of_day % 60;

    let (year, month, day) = civil_from_days(days);
    (year, month, day, hour, min, sec, ms)
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year, m as u32, d as u32)
}

/// Howard Hinnant's days-from-civil 演算法，將 `(year, month, day)` 轉為自 1970-01-01 起算的天數。
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = y - if m <= 2 { 1 } else { 0 };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400) as u64;
    let m = m as u64;
    let d = d as u64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + (doe as i64) - 719_468
}

pub fn tw_date_string(now_ms: u64) -> String {
    let (y, m, d, _, _, _, _) = tw_datetime_parts(now_ms);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn format_tw_timestamp(now_ms: u64) -> String {
    let (y, m, d, hour, min, sec, ms) = tw_datetime_parts(now_ms);
    format!("{y:04}-{m:02}-{d:02} {hour:02}:{min:02}:{sec:02}.{ms:03}")
}

/// 依日期與 `max_bytes` 決定實際寫入的每日 Log 檔路徑（若超過上限則自動切 `.part1.log`, `.part2.log`...）。
pub fn resolve_writable_log_file(log_dir: &Path, date_str: &str, max_bytes: u64) -> PathBuf {
    let base = log_dir.join(format!("moi-sms-{date_str}.log"));
    let base_len = fs::metadata(&base).map(|m| m.len()).unwrap_or(0);
    if base_len < max_bytes {
        return base;
    }

    for part in 1..=999 {
        let candidate = log_dir.join(format!("moi-sms-{date_str}.part{part}.log"));
        let len = fs::metadata(&candidate).map(|m| m.len()).unwrap_or(0);
        if len < max_bytes {
            return candidate;
        }
    }
    base
}

/// 寫入單行日誌至當日 Log 檔（自動建立目錄、依台灣日期切換及超過上限切分）。
pub fn append_daily_log_line(
    log_dir: &Path,
    now_ms: u64,
    level: &str,
    title: &str,
    message: &str,
    max_bytes: u64,
) -> Result<PathBuf> {
    fs::create_dir_all(log_dir)
        .with_context(|| format!("無法建立日誌目錄: {}", log_dir.display()))?;
    let date_str = tw_date_string(now_ms);
    let target = resolve_writable_log_file(log_dir, &date_str, max_bytes);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&target)
        .with_context(|| format!("無法開啟日誌檔: {}", target.display()))?;
    let ts = format_tw_timestamp(now_ms);
    let clean_msg = message.replace(['\r', '\n'], " ");
    writeln!(file, "[{ts}] [{}] {title} - {clean_msg}", level.to_uppercase())
        .with_context(|| format!("寫入日誌檔失敗: {}", target.display()))?;
    Ok(target)
}

/// 讀取目前當日最新寫入的日誌檔最後 `max_lines` 行，供介面追蹤顯示。
pub fn read_latest_daily_log_tail(
    log_dir: &Path,
    now_ms: u64,
    max_bytes: u64,
    max_lines: usize,
) -> Result<DailyLogTail> {
    let date_str = tw_date_string(now_ms);
    let target = resolve_writable_log_file(log_dir, &date_str, max_bytes);
    let file_path = target.display().to_string();
    if !target.exists() {
        return Ok(DailyLogTail {
            file_path,
            file_size: 0,
            total_lines: 0,
            lines: Vec::new(),
        });
    }
    let bytes = fs::read(&target)
        .with_context(|| format!("無法讀取日誌檔: {}", target.display()))?;
    let file_size = bytes.len() as u64;
    let text = String::from_utf8_lossy(&bytes);
    let all_lines: Vec<String> = text.lines().map(ToString::to_string).collect();
    let total_lines = all_lines.len();
    let keep = max_lines.max(1);
    let start = total_lines.saturating_sub(keep);
    Ok(DailyLogTail {
        file_path,
        file_size,
        total_lines,
        lines: all_lines[start..].to_vec(),
    })
}

/// 從檔名解析 `moi-sms-YYYY-MM-DD(.partN)?.log(.zip)?` 的日期。
pub fn parse_log_filename_date(name: &str) -> Option<(i64, u32, u32)> {
    let rest = name.strip_prefix("moi-sms-")?;
    if rest.len() < 10 {
        return None;
    }
    let date_part = &rest[..10];
    let after_date = &rest[10..];
    let is_supported_suffix = after_date == ".log"
        || after_date == ".log.zip"
        || (after_date.starts_with(".part")
            && (after_date.ends_with(".log") || after_date.ends_with(".log.zip")));
    if !is_supported_suffix {
        return None;
    }

    let mut segs = date_part.split('-');
    let y: i64 = segs.next()?.parse().ok()?;
    let m: u32 = segs.next()?.parse().ok()?;
    let d: u32 = segs.next()?.parse().ok()?;
    if segs.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

fn compress_log_to_zip(log_path: &Path, zip_path: &Path) -> Result<()> {
    let file_name = log_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("無效的 Log 檔名")?;
    let tmp_zip = zip_path.with_extension("zip.tmp");

    {
        let mut src = File::open(log_path)
            .with_context(|| format!("開啟來源 Log 失敗: {}", log_path.display()))?;
        let out = File::create(&tmp_zip)
            .with_context(|| format!("建立暫存壓縮檔失敗: {}", tmp_zip.display()))?;
        let mut zip = ZipWriter::new(out);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file(file_name, options)
            .context("寫入 ZIP 檔頭失敗")?;

        let mut buf = [0u8; 32 * 1024];
        loop {
            let n = src.read(&mut buf).context("讀取來源 Log 失敗")?;
            if n == 0 {
                break;
            }
            zip.write_all(&buf[..n]).context("寫入壓縮資料失敗")?;
        }
        zip.finish().context("完成 ZIP 壓縮失敗")?;
    }

    fs::rename(&tmp_zip, zip_path)
        .with_context(|| format!("重新命名壓縮檔失敗: {}", zip_path.display()))?;
    Ok(())
}

/// 執行每日 Log 清理與壓縮，並清理過舊的 `env.properties.bak.*` 備份檔。
pub fn cleanup_logs_and_backups(
    log_dir: &Path,
    env_dir: Option<&Path>,
    now_ms: u64,
    compress_after_days: i64,
    delete_after_days: i64,
    max_env_backups: usize,
) -> LogCleanupReport {
    let mut report = LogCleanupReport::default();
    let (cur_y, cur_m, cur_d, _, _, _, _) = tw_datetime_parts(now_ms);
    let today_days = days_from_civil(cur_y, cur_m, cur_d);

    if let Ok(entries) = fs::read_dir(log_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            let is_zip = name.ends_with(".log.zip");
            let is_log = name.ends_with(".log");
            if !is_zip && !is_log {
                continue;
            }

            let file_days = if let Some((y, m, d)) = parse_log_filename_date(name) {
                days_from_civil(y, m, d)
            } else if name == "moi-sms.log" {
                // 相容清理舊版單一檔 moi-sms.log（以修改時間推算天數）
                let mtime_ms = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(now_ms);
                let (fy, fm, fd, _, _, _, _) = tw_datetime_parts(mtime_ms);
                days_from_civil(fy, fm, fd)
            } else {
                continue;
            };

            let age_days = today_days - file_days;
            if age_days > delete_after_days {
                match fs::remove_file(&path) {
                    Ok(_) => {
                        if is_zip {
                            report.deleted_zips_count += 1;
                        } else {
                            report.deleted_logs_count += 1;
                        }
                    }
                    Err(e) => report.errors.push(format!("刪除 {} 失敗: {e}", path.display())),
                }
            } else if is_log && age_days > compress_after_days {
                let zip_path = PathBuf::from(format!("{}.zip", path.display()));
                if zip_path.exists() {
                    if let Err(e) = fs::remove_file(&path) {
                        report
                            .errors
                            .push(format!("移除已壓縮 Log {} 失敗: {e}", path.display()));
                    } else {
                        report.deleted_logs_count += 1;
                    }
                    continue;
                }
                match compress_log_to_zip(&path, &zip_path) {
                    Ok(_) => {
                        report.compressed_count += 1;
                        if let Err(e) = fs::remove_file(&path) {
                            report
                                .errors
                                .push(format!("壓縮後移除原檔 {} 失敗: {e}", path.display()));
                        }
                    }
                    Err(e) => report.errors.push(format!("壓縮 {} 失敗: {e:#}", path.display())),
                }
            }
        }
    }

    if let Some(dir) = env_dir {
        if let Ok(entries) = fs::read_dir(dir) {
            let mut backups: Vec<(SystemTime, PathBuf)> = entries
                .flatten()
                .filter_map(|entry| {
                    let path = entry.path();
                    let name = path.file_name()?.to_str()?;
                    if path.is_file() && name.starts_with("env.properties.bak.") {
                        let mtime = entry
                            .metadata()
                            .and_then(|m| m.modified())
                            .unwrap_or(UNIX_EPOCH);
                        Some((mtime, path))
                    } else {
                        None
                    }
                })
                .collect();

            // 依修改時間與檔名由新到舊排序
            backups.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));

            let now_sys = UNIX_EPOCH + Duration::from_millis(now_ms);
            let max_age = Duration::from_secs((delete_after_days.max(0) as u64) * 86_400);

            for (idx, (mtime, path)) in backups.into_iter().enumerate() {
                let too_old = now_sys
                    .duration_since(mtime)
                    .map(|age| age > max_age)
                    .unwrap_or(false);
                if idx >= max_env_backups || too_old {
                    match fs::remove_file(&path) {
                        Ok(_) => report.deleted_backups_count += 1,
                        Err(e) => report
                            .errors
                            .push(format!("刪除舊設定備份 {} 失敗: {e}", path.display())),
                    }
                }
            }
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::ZipArchive;

    fn ms_for_tw_date(y: i64, m: u32, d: u32) -> u64 {
        let days = days_from_civil(y, m, d);
        // 台灣中午 12:00:00 換算為 UTC 04:00:00
        ((days * 86_400 + 4 * 3600) as u64) * 1000
    }

    #[test]
    fn test_daily_log_rotation_and_size_split() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("moi_log_rot_{unique}"));

        let day1 = ms_for_tw_date(2026, 11, 12);
        let day2 = ms_for_tw_date(2026, 11, 13);

        // 設定極小上限 80 bytes 觸發同日 .part1 切分
        let p1 = append_daily_log_line(&dir, day1, "info", "啟動", "第一筆訊息內容較長足以超過八十位元組限制測試", 80).unwrap();
        let p2 = append_daily_log_line(&dir, day1, "info", "第二", "第二筆應切換至 part1", 80).unwrap();
        let p3 = append_daily_log_line(&dir, day2, "info", "跨日", "隔日新檔", 80).unwrap();

        assert!(p1.ends_with("moi-sms-2026-11-12.log"));
        assert!(p2.ends_with("moi-sms-2026-11-12.part1.log"));
        assert!(p3.ends_with("moi-sms-2026-11-13.log"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_cleanup_compresses_over_3_days_and_deletes_over_30_days() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("moi_log_clean_{unique}"));
        fs::create_dir_all(&dir).unwrap();

        let today_ms = ms_for_tw_date(2026, 11, 13);

        // 3 天內：保留原 .log
        let log_today = dir.join("moi-sms-2026-11-13.log");
        let log_3d = dir.join("moi-sms-2026-11-10.log");
        // 超過 3 天（4天與10天）：應壓縮成 .log.zip 並刪除原 .log
        let log_4d = dir.join("moi-sms-2026-11-09.log");
        let log_10d = dir.join("moi-sms-2026-11-03.part1.log");
        // 超過 30 天（31天）：無論 .log 或 .log.zip 皆直接刪除
        let log_31d = dir.join("moi-sms-2026-10-13.log");
        let zip_31d = dir.join("moi-sms-2026-10-12.log.zip");

        fs::write(&log_today, "today content").unwrap();
        fs::write(&log_3d, "3d content").unwrap();
        fs::write(&log_4d, "4d log content").unwrap();
        fs::write(&log_10d, "10d part1 content").unwrap();
        fs::write(&log_31d, "31d expired").unwrap();
        fs::write(&zip_31d, "31d zip expired").unwrap();

        let report = cleanup_logs_and_backups(&dir, None, today_ms, 3, 30, 10);
        assert!(report.errors.is_empty(), "unexpected errors: {:?}", report.errors);
        assert_eq!(report.compressed_count, 2);
        assert_eq!(report.deleted_logs_count, 1);
        assert_eq!(report.deleted_zips_count, 1);

        assert!(log_today.exists());
        assert!(log_3d.exists());
        assert!(!log_4d.exists());
        assert!(!log_10d.exists());
        assert!(!log_31d.exists());
        assert!(!zip_31d.exists());

        let zip_4d = dir.join("moi-sms-2026-11-09.log.zip");
        assert!(zip_4d.exists());
        let mut archive = ZipArchive::new(File::open(&zip_4d).unwrap()).unwrap();
        let mut entry = archive.by_name("moi-sms-2026-11-09.log").unwrap();
        let mut extracted = String::new();
        entry.read_to_string(&mut extracted).unwrap();
        assert_eq!(extracted, "4d log content");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_cleanup_env_backups_keeps_latest_ten() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("moi_bak_clean_{unique}"));
        let env_dir = dir.join("env");
        let log_dir = dir.join("logs");
        fs::create_dir_all(&env_dir).unwrap();
        fs::create_dir_all(&log_dir).unwrap();

        for i in 1..=13 {
            let p = env_dir.join(format!("env.properties.bak.{i:02}"));
            fs::write(&p, format!("bak {i}")).unwrap();
        }

        let today_ms = ms_for_tw_date(2026, 11, 13);
        let report = cleanup_logs_and_backups(&log_dir, Some(&env_dir), today_ms, 3, 30, 10);
        assert!(report.errors.is_empty());
        assert_eq!(report.deleted_backups_count, 3);

        let remaining = fs::read_dir(&env_dir).unwrap().count();
        assert_eq!(remaining, 10);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_read_latest_daily_log_tail_limits_lines() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("moi_log_tail_{unique}"));
        let now_ms = ms_for_tw_date(2026, 11, 13);

        // 檔案尚未建立時應回傳空結果
        let empty = read_latest_daily_log_tail(&dir, now_ms, DEFAULT_MAX_LOG_BYTES, 10).unwrap();
        assert_eq!(empty.total_lines, 0);
        assert!(empty.lines.is_empty());

        for i in 1..=5 {
            append_daily_log_line(
                &dir,
                now_ms + i * 1000,
                "info",
                "測試",
                &format!("第 {i} 行"),
                DEFAULT_MAX_LOG_BYTES,
            )
            .unwrap();
        }
        let tail = read_latest_daily_log_tail(&dir, now_ms, DEFAULT_MAX_LOG_BYTES, 3).unwrap();
        assert_eq!(tail.total_lines, 5);
        assert_eq!(tail.lines.len(), 3);
        assert!(tail.lines[0].contains("第 3 行"));
        assert!(tail.lines[2].contains("第 5 行"));

        let _ = fs::remove_dir_all(&dir);
    }
}
