use moi_sms::log_maintenance::{append_daily_log_line, DEFAULT_MAX_LOG_BYTES};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub lah_api_url: String,
    pub gateway_url: String,
    pub sms_user: String,
    pub sms_password: String,
    pub mail_account: String,
    pub mail_password: String,
    pub mail_from: String,
    pub username: String,
    pub search_minutes: u32,
    pub max_retries: u32,
    pub start_time: String,
    pub end_time: String,
    pub auto_start_schedule: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            lah_api_url: "http://127.0.0.1/api/sms_v1_api.php".into(),
            gateway_url: "https://gateway.example.com/send".into(),
            sms_user: String::new(),
            sms_password: String::new(),
            mail_account: String::new(),
            mail_password: String::new(),
            mail_from: "demo@example.com".into(),
            username: "桃園地政事務所".into(),
            search_minutes: 1,
            max_retries: 3,
            start_time: "08:00".into(),
            end_time: "18:30".into(),
            auto_start_schedule: false,
        }
    }
}

fn clock_minutes(value: &str) -> Option<u32> {
    let bytes = value.as_bytes();
    if bytes.len() != 5 || bytes[2] != b':' {
        return None;
    }
    if ![bytes[0], bytes[1], bytes[3], bytes[4]]
        .iter()
        .all(u8::is_ascii_digit)
    {
        return None;
    }
    let hour: u32 = value[..2].parse().ok()?;
    let minute: u32 = value[3..].parse().ok()?;
    (hour < 24 && minute < 60).then_some(hour * 60 + minute)
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        for (name, url) in [
            ("LAH-API URL", &self.lah_api_url),
            ("派送閘道 URL", &self.gateway_url),
        ] {
            if !http_url_valid(url) {
                return Err(format!("{name} 須為具有主機名稱的 HTTP 或 HTTPS 網址"));
            }
        }
        if !(1..=1440).contains(&self.search_minutes) {
            return Err("搜尋間隔須介於 1～1440 分鐘".into());
        }
        if self.max_retries > 99 {
            return Err("最大重試次數須介於 0～99 次".into());
        }
        let start = clock_minutes(&self.start_time).ok_or("開始時間格式須為 HH:MM")?;
        let end = clock_minutes(&self.end_time).ok_or("結束時間格式須為 HH:MM")?;
        if start >= end {
            return Err("結束時間須晚於開始時間；此版本使用單日執行時段".into());
        }
        Ok(())
    }
}

fn http_url_valid(value: &str) -> bool {
    url::Url::parse(value).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && !value.chars().any(char::is_whitespace)
            && url.username().is_empty()
            && url.password().is_none()
    })
}

#[derive(Clone, Debug, Serialize)]
pub struct Record {
    pub id: String,
    pub table: String,
    pub kind: String,
    pub year: String,
    pub word: String,
    pub number: String,
    pub date: String,
    pub time: String,
    pub channel: String,
    pub recipient: String,
    pub status: String,
    pub retry_count: u32,
    pub subject: String,
    pub message: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Activity {
    pub id: u64,
    pub timestamp_ms: u64,
    pub title: String,
    pub message: String,
    pub level: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct RecordQuery {
    pub table: String,
    pub keyword: String,
    pub channel: String,
    pub status: String,
    pub date: String,
    pub sort_by: String,
    pub ascending: bool,
}

#[derive(Serialize)]
pub struct RecordList {
    pub items: Vec<Record>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct Snapshot {
    pub settings: Settings,
    pub running: bool,
    pub schedule_phase: String,
    pub main_count: usize,
    pub wait_count: usize,
    pub log_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub activities: Vec<Activity>,
    pub config_path: String,
    pub log_path: String,
    pub startup_warning: Option<String>,
    pub site_label: Option<String>,
    pub os_autostart_enabled: bool,
}

pub struct DesktopModel {
    pub settings: Settings,
    pub running: bool,
    pub schedule_phase: String,
    pub last_counts: (usize, usize, usize, usize, usize),
    pub activities: Vec<Activity>,
    pub next_id: u64,
    pub log_path: Option<PathBuf>,
}

impl DesktopModel {
    #[cfg(test)]
    pub fn new(settings: Settings) -> Self {
        Self::with_log_path(settings, None)
    }

    pub fn with_log_path(settings: Settings, log_path: Option<PathBuf>) -> Self {
        let mut model = Self {
            settings,
            running: false,
            schedule_phase: "已暫停".into(),
            last_counts: (0, 0, 0, 0, 0),
            activities: Vec::new(),
            next_id: 1,
            log_path,
        };
        model.log(
            "info",
            "桌面程式已啟動",
            "已啟用 LAH-API 整合介面。",
        );
        model
    }

    pub fn log(&mut self, level: &str, title: &str, message: &str) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if let Some(log_dir) = &self.log_path {
            let _ = append_daily_log_line(
                log_dir,
                now_ms,
                level,
                title,
                message,
                DEFAULT_MAX_LOG_BYTES,
            );
        }
        self.activities.insert(
            0,
            Activity {
                id: self.next_id,
                timestamp_ms: now_ms,
                title: title.into(),
                message: message.into(),
                level: level.into(),
            },
        );
        self.next_id += 1;
        self.activities.truncate(200);
    }

    pub fn set_running(&mut self, running: bool) {
        self.running = running;
        self.schedule_phase = if running {
            "執行中".into()
        } else {
            "已暫停".into()
        };
        self.log(
            "info",
            if running {
                "排程已啟動"
            } else {
                "排程已暫停"
            },
            "已更新排程執行狀態。",
        );
    }

    pub fn validate_delivery_input(
        &self,
        channel: &str,
        recipient: &str,
        message: &str,
    ) -> Result<(), String> {
        let recipient = recipient.trim();
        let valid = match channel {
            "sms" => {
                recipient.len() == 10
                    && recipient.starts_with("09")
                    && recipient.bytes().all(|b| b.is_ascii_digit())
            }
            "email" => recipient.split_once('@').is_some_and(|(local, host)| {
                !local.is_empty()
                    && host.contains('.')
                    && !host.starts_with('.')
                    && !host.ends_with('.')
                    && !host.contains('@')
                    && !recipient.chars().any(char::is_whitespace)
            }),
            _ => return Err("未知的傳送管道".into()),
        };
        if !valid {
            return Err("請輸入有效的手機號碼或 Email".into());
        }
        if message.trim().is_empty() {
            return Err("測試內容不可為空".into());
        }
        Ok(())
    }
}

pub fn filter_records(records: &[Record], query: &RecordQuery) -> Result<RecordList, String> {
    if !["main", "wait", "log"].contains(&query.table.as_str()) {
        return Err("未知的案件資料表".into());
    }
    let keyword = query.keyword.trim().to_lowercase();
    let total = records.iter().filter(|r| r.table == query.table).count();
    let mut items: Vec<_> = records
        .iter()
        .filter(|r| {
            r.table == query.table
                && (query.channel.is_empty() || r.channel == query.channel)
                && (query.status.is_empty() || r.status == query.status)
                && (query.date.is_empty() || r.date == query.date.replace('/', ""))
                && (keyword.is_empty()
                    || format!("{} {} {}", r.word, r.number, r.recipient)
                        .to_lowercase()
                        .contains(&keyword))
        })
        .cloned()
        .collect();
    items.sort_by(|a, b| match query.sort_by.as_str() {
        "number" => a
            .number
            .cmp(&b.number)
            .then_with(|| a.date.cmp(&b.date))
            .then_with(|| a.time.cmp(&b.time)),
        _ => a
            .date
            .cmp(&b.date)
            .then_with(|| a.time.cmp(&b.time))
            .then_with(|| a.number.cmp(&b.number)),
    });
    if !query.ascending {
        items.reverse();
    }
    Ok(RecordList { items, total })
}

fn format_hms(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() == 6 && trimmed.bytes().all(|b| b.is_ascii_digit()) {
        format!("{}:{}:{}", &trimmed[0..2], &trimmed[2..4], &trimmed[4..6])
    } else {
        trimmed.to_string()
    }
}

pub fn record_from_api(
    table: &str,
    idx: usize,
    item: &moi_sms::lah_client::SmsTableRecord,
) -> Record {
    let has_mail = !item.ms_mail.trim().is_empty();
    let channel = if has_mail { "email" } else { "sms" }.to_string();
    let recipient = if has_mail {
        item.ms_mail.trim().to_string()
    } else {
        item.ms14.trim().to_string()
    };
    let retry_count = item.ms32.trim().parse::<u32>().unwrap_or(0);
    let status = match table {
        "log" => {
            if !item.ms31_label.trim().is_empty() {
                item.ms31_label.trim().to_string()
            } else if item.ms31.trim().eq_ignore_ascii_case("S") {
                "成功".to_string()
            } else {
                "失敗".to_string()
            }
        }
        "wait" => {
            if retry_count > 0 {
                "等待重試".to_string()
            } else {
                "待傳送".to_string()
            }
        }
        _ => "待處理".to_string(),
    };
    let kind = if !item.ms_type_label.trim().is_empty() {
        item.ms_type_label.trim().to_string()
    } else {
        item.ms_type.trim().to_string()
    };
    let subject = if !item.ms30_label.trim().is_empty() {
        item.ms30_label.trim().to_string()
    } else {
        "地籍異動通知".to_string()
    };
    let message = if !item.ms_note.trim().is_empty() {
        item.ms_note.trim().to_string()
    } else {
        format!(
            "{}年 {} {}號（{}）",
            item.ms03.trim(),
            item.ms04_1.trim(),
            item.ms04_2.trim(),
            subject
        )
    };
    let error = if status == "失敗" && !item.ms_note.trim().is_empty() {
        Some(item.ms_note.trim().to_string())
    } else {
        None
    };

    Record {
        id: format!(
            "{table}-{idx}-{}-{}-{}",
            item.ms03.trim(),
            item.ms04_1.trim(),
            item.ms04_2.trim()
        ),
        table: table.to_string(),
        kind,
        year: item.ms03.trim().to_string(),
        word: item.ms04_1.trim().to_string(),
        number: item.ms04_2.trim().to_string(),
        date: item.ms07_1.trim().to_string(),
        time: format_hms(&item.ms07_2),
        channel,
        recipient,
        status,
        retry_count,
        subject,
        message,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_combines_filters_and_does_not_modify_other_tables() {
        let records = vec![
            Record {
                id: "1".into(),
                table: "log".into(),
                kind: "產權通知".into(),
                year: "115".into(),
                word: "HA81".into(),
                number: "197000".into(),
                date: "1151113".into(),
                time: "11:00:00".into(),
                channel: "email".into(),
                recipient: "demo.land@example.com".into(),
                status: "成功".into(),
                retry_count: 0,
                subject: "主旨".into(),
                message: "內容".into(),
                error: None,
            },
            Record {
                id: "2".into(),
                table: "log".into(),
                kind: "產權通知".into(),
                year: "115".into(),
                word: "HA81".into(),
                number: "197020".into(),
                date: "1151113".into(),
                time: "11:01:00".into(),
                channel: "sms".into(),
                recipient: "0912345678".into(),
                status: "成功".into(),
                retry_count: 0,
                subject: "主旨".into(),
                message: "內容".into(),
                error: None,
            },
        ];
        let query = RecordQuery {
            table: "log".into(),
            channel: "email".into(),
            status: "成功".into(),
            keyword: "197000".into(),
            date: "115/11/13".into(),
            ..Default::default()
        };
        let result = filter_records(&records, &query).unwrap();
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].recipient, "demo.land@example.com");
        assert_eq!(result.total, 2);

        // 預設依日期與時間 DESC 排序（最新在最前），切換 ascending=true 則為 ASC
        let desc = filter_records(
            &records,
            &RecordQuery {
                table: "log".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(desc.items.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), vec!["2", "1"]);

        let asc = filter_records(
            &records,
            &RecordQuery {
                table: "log".into(),
                sort_by: "time".into(),
                ascending: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(asc.items.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), vec!["1", "2"]);
        assert!(filter_records(&records, &RecordQuery::default()).is_err());
    }

    #[test]
    fn set_running_updates_schedule_phase_and_logs() {
        let mut model = DesktopModel::new(Settings::default());
        assert!(!model.running);
        assert_eq!(model.schedule_phase, "已暫停");
        model.set_running(true);
        assert!(model.running);
        assert_eq!(model.schedule_phase, "執行中");
        model.set_running(false);
        assert!(!model.running);
        assert_eq!(model.schedule_phase, "已暫停");
    }

    #[test]
    fn settings_and_delivery_reject_invalid_inputs() {
        let mut settings = Settings::default();
        assert!(settings.validate().is_ok());
        settings.start_time = "25:00".into();
        assert!(settings.validate().is_err());
        settings.start_time = "18:30".into();
        assert!(settings.validate().is_err());
        let model = DesktopModel::new(Settings::default());
        assert!(model
            .validate_delivery_input("sms", "0900000000", "測試")
            .is_ok());
        assert!(model
            .validate_delivery_input("sms", "123", "測試")
            .is_err());
        assert!(model
            .validate_delivery_input("email", "a@example.com", "")
            .is_err());
        assert!(model
            .validate_delivery_input("email", "a@@example.com", "測試")
            .is_err());
    }

    #[test]
    fn record_from_api_maps_sms_email_and_log_statuses() {
        let api_sms = moi_sms::lah_client::SmsTableRecord {
            ms03: "115".into(),
            ms04_1: "HA81".into(),
            ms04_2: "197021".into(),
            ms07_1: "1151113".into(),
            ms07_2: "093005".into(),
            ms14: "0912345678".into(),
            ms_mail: "".into(),
            ms30: "Y".into(),
            ms31: "".into(),
            ms32: "1".into(),
            ms_type: "M".into(),
            ms_note: "測試通知".into(),
            ms_type_label: "產權通知".into(),
            ms30_label: "產權收件通知".into(),
            ms31_label: "".into(),
        };
        let wait_rec = record_from_api("wait", 0, &api_sms);
        assert_eq!(wait_rec.channel, "sms");
        assert_eq!(wait_rec.recipient, "0912345678");
        assert_eq!(wait_rec.time, "09:30:05");
        assert_eq!(wait_rec.status, "等待重試");
        assert_eq!(wait_rec.retry_count, 1);

        let api_log_fail = moi_sms::lah_client::SmsTableRecord {
            ms14: "".into(),
            ms_mail: "owner@example.com".into(),
            ms31: "F".into(),
            ms31_label: "失敗".into(),
            ms_note: "閘道逾時".into(),
            ..api_sms
        };
        let log_rec = record_from_api("log", 1, &api_log_fail);
        assert_eq!(log_rec.channel, "email");
        assert_eq!(log_rec.recipient, "owner@example.com");
        assert_eq!(log_rec.status, "失敗");
        assert_eq!(log_rec.error.as_deref(), Some("閘道逾時"));
    }

    #[test]
    fn desktop_model_writes_timestamped_log_lines_to_file() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("moi_log_test_{unique}"));
        let log_dir = dir.join("logs");

        let mut model = DesktopModel::with_log_path(Settings::default(), Some(log_dir.clone()));
        model.log("error", "API 錯誤", "連線失敗\n詳細原因");

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let expected_file = log_dir.join(format!(
            "moi-sms-{}.log",
            moi_sms::log_maintenance::tw_date_string(now_ms)
        ));
        let content = std::fs::read_to_string(&expected_file).unwrap();
        assert!(content.contains("[INFO] 桌面程式已啟動"));
        assert!(content.contains("[ERROR] API 錯誤 - 連線失敗 詳細原因"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
