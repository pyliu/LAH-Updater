use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};

use crate::comm::{HttpTransport, SmsGatewayClient};
use crate::config::MailConfig;
use crate::lah_client::{
    CaseSyncResponse, LahApiClient, QueueEnqueueRequest, QueueEnqueueResponse, QueueReportRequest,
};

#[derive(Debug, Clone)]
pub struct DispatchConfig {
    pub site_id: Option<String>,
    pub max_retries: u32,
    pub mail: MailConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchItemLog {
    pub level: String,
    pub title: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct DispatchCycleSummary {
    pub sync: CaseSyncResponse,
    pub enqueue: QueueEnqueueResponse,
    pub pending_count: usize,
    pub sent_count: usize,
    pub failed_count: usize,
    pub report_error_count: usize,
    pub stopped_early: bool,
    pub item_logs: Vec<DispatchItemLog>,
}

/// 解析 `HH:MM` 格式為自 00:00 起的分鐘數。
pub fn parse_hhmm_minutes(value: &str) -> Option<u32> {
    let bytes = value.trim().as_bytes();
    if bytes.len() != 5 || bytes[2] != b':' {
        return None;
    }
    if ![bytes[0], bytes[1], bytes[3], bytes[4]]
        .iter()
        .all(u8::is_ascii_digit)
    {
        return None;
    }
    let hour: u32 = value.trim()[..2].parse().ok()?;
    let minute: u32 = value.trim()[3..].parse().ok()?;
    (hour < 24 && minute < 60).then_some(hour * 60 + minute)
}

/// 判斷目前分鐘數是否位於單日執行時段 `[start, end]` 內。
pub fn is_within_schedule_window(now_minutes: u32, start_minutes: u32, end_minutes: u32) -> bool {
    start_minutes <= end_minutes && now_minutes >= start_minutes && now_minutes <= end_minutes
}

/// 取得台灣時間（UTC+8）當日自 00:00 起的分鐘數。
pub fn current_tw_minutes() -> u32 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let tw_secs_of_day = (secs + 8 * 3600) % 86_400;
    (tw_secs_of_day / 60) as u32
}

/// 執行單輪排程作業：案件同步 -> 搬移待傳佇列 -> 查詢待傳 -> 閘道派送 -> 回報結果。
pub fn run_dispatch_cycle<T: HttpTransport, G: HttpTransport>(
    lah: &LahApiClient<T>,
    gateway: &SmsGatewayClient<G>,
    cfg: &DispatchConfig,
    should_stop: &AtomicBool,
) -> Result<DispatchCycleSummary> {
    if should_stop.load(Ordering::Relaxed) {
        bail!("排程已收到停止請求，取消本輪派送");
    }

    let mut step_logs: Vec<DispatchItemLog> = Vec::new();

    let sync = lah.sync_cases(None).context("排程掃描同步案件失敗")?;
    step_logs.push(DispatchItemLog {
        level: "info".into(),
        title: "排程步驟 1/4：案件掃描同步".into(),
        message: format!(
            "日期 {}：掃描一般 {} 筆（新增 {} 筆）、機關 {} 筆（新增 {} 筆）。",
            sync.target_date,
            sync.user_cases_scanned,
            sync.user_notifications_inserted,
            sync.gov_cases_scanned,
            sync.gov_notifications_inserted
        ),
    });

    let enqueue = lah
        .enqueue_queue(&QueueEnqueueRequest::default())
        .context("排程搬移待傳佇列失敗")?;
    step_logs.push(DispatchItemLog {
        level: "info".into(),
        title: "排程步驟 2/4：搬移待傳佇列".into(),
        message: format!(
            "已從 SSMSMAIN 搬移 {} 筆至 SSMSWAIT（刪除 {} 筆）。",
            enqueue.moved_to_wait_count, enqueue.deleted_from_main_count
        ),
    });

    let pending = lah.get_pending_queue().context("排程查詢待傳清單失敗")?;
    step_logs.push(DispatchItemLog {
        level: "info".into(),
        title: "排程步驟 3/4：查詢待傳清單".into(),
        message: format!(
            "取得 {} 筆待傳項目（site_id={}）。",
            pending.items.len(),
            if pending.site_id.trim().is_empty() {
                "未指定"
            } else {
                pending.site_id.trim()
            }
        ),
    });

    let mut summary = DispatchCycleSummary {
        sync,
        enqueue,
        pending_count: pending.items.len(),
        sent_count: 0,
        failed_count: 0,
        report_error_count: 0,
        stopped_early: false,
        item_logs: step_logs,
    };

    if pending.items.is_empty() {
        return Ok(summary);
    }

    let site_id = if !pending.site_id.trim().is_empty() {
        pending.site_id.trim().to_string()
    } else if let Some(sid) = cfg.site_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        sid.to_string()
    } else {
        lah.get_metadata()
            .context("取得站台代碼 site_id 失敗")?
            .site_id
            .trim()
            .to_string()
    };

    if site_id.is_empty() {
        bail!("無法取得有效的 site_id，中止本輪派送");
    }

    for item in pending.items {
        if should_stop.load(Ordering::Relaxed) {
            summary.stopped_early = true;
            summary.item_logs.push(DispatchItemLog {
                level: "warn".into(),
                title: "排程步驟 4/4：已收到停止請求".into(),
                message: "已中止領取後續待傳項目，將安全暫停排程。".into(),
            });
            break;
        }

        let channel_key = item.channel.trim().to_ascii_lowercase();
        let case_ref = format!(
            "{}年 {} {}號（{} / {}）",
            item.identity.ms03.trim(),
            item.identity.ms04_1.trim(),
            item.identity.ms04_2.trim(),
            channel_key,
            item.recipient.trim()
        );
        summary.item_logs.push(DispatchItemLog {
            level: "info".into(),
            title: "排程步驟 4/4：開始派送".into(),
            message: format!("{case_ref}，目前重試次數 {}。", item.retry_count),
        });

        let dispatch_res = match channel_key.as_str() {
            "sms" => gateway
                .send_sms_gateway(&site_id, &item.recipient, &item.msg)
                .map(|r| r.desc),
            "email" => gateway
                .send_email_gateway(&site_id, &cfg.mail, &item.subject, &item.recipient, &item.msg)
                .map(|r| r.desc),
            other => Err(anyhow::anyhow!("不支援的派送管道: {other}")),
        };

        let (success, error_desc) = match dispatch_res {
            Ok(_) => (true, None),
            Err(err) => (false, Some(format!("{err:#}"))),
        };

        let report_req = QueueReportRequest {
            identity: item.identity,
            success,
            msg: item.msg,
            max_retries: cfg.max_retries,
            error_desc: error_desc.clone(),
        };

        match lah.report_queue_item(&report_req) {
            Ok(rep) => {
                if success {
                    summary.sent_count += 1;
                    summary.item_logs.push(DispatchItemLog {
                        level: "success".into(),
                        title: "通知派送成功".into(),
                        message: format!("{case_ref} 已送出（{}）。", rep.action),
                    });
                } else {
                    summary.failed_count += 1;
                    summary.item_logs.push(DispatchItemLog {
                        level: "error".into(),
                        title: "通知派送失敗".into(),
                        message: format!(
                            "{case_ref} 派送失敗（{}）：{}",
                            rep.action,
                            error_desc.unwrap_or_default()
                        ),
                    });
                }
            }
            Err(rep_err) => {
                summary.report_error_count += 1;
                let title = if success {
                    "已送出，結果回報待確認"
                } else {
                    "派送失敗且結果回報失敗"
                };
                summary.item_logs.push(DispatchItemLog {
                    level: "warn".into(),
                    title: title.into(),
                    message: format!("{case_ref}：{rep_err:#}"),
                });
            }
        }
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SmsApiConfig;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::time::Duration;

    struct ScriptedTransport {
        gets: RefCell<VecDeque<Result<String>>>,
        posts: RefCell<VecDeque<Result<String>>>,
        recorded_posts: RefCell<Vec<(String, serde_json::Value)>>,
    }

    impl ScriptedTransport {
        fn new(gets: Vec<Result<String>>, posts: Vec<Result<String>>) -> Self {
            Self {
                gets: RefCell::new(gets.into()),
                posts: RefCell::new(posts.into()),
                recorded_posts: RefCell::new(Vec::new()),
            }
        }
    }

    impl HttpTransport for ScriptedTransport {
        fn get(&self, _url: &str, _timeout: Duration) -> Result<String> {
            self.gets
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| bail!("unexpected GET"))
        }

        fn post_form(&self, _url: &str, _params: &[(&str, &str)], _timeout: Duration) -> Result<String> {
            bail!("unexpected form POST")
        }

        fn post_json(&self, url: &str, body: &serde_json::Value, _timeout: Duration) -> Result<String> {
            self.recorded_posts
                .borrow_mut()
                .push((url.to_string(), body.clone()));
            self.posts
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| bail!("unexpected JSON POST"))
        }
    }

    fn sample_cfg() -> DispatchConfig {
        DispatchConfig {
            site_id: Some("HA".into()),
            max_retries: 3,
            mail: MailConfig {
                smtp_host: String::new(),
                smtp_port: None,
                account: "mail_u".into(),
                password: "mail_p".into(),
                from_addr: "noreply@land.gov.tw".into(),
                username: "桃園地政事務所".into(),
            },
        }
    }

    fn sample_gateway_cfg() -> SmsApiConfig {
        SmsApiConfig {
            ap_url: "https://gw.example.gov.tw/ONEServiceServlet".into(),
            ap_userid: "sms_u".into(),
            ap_pwd: "sms_p".into(),
        }
    }

    #[test]
    fn test_schedule_window_boundaries() {
        assert_eq!(parse_hhmm_minutes("08:00"), Some(480));
        assert_eq!(parse_hhmm_minutes("18:30"), Some(1110));
        assert_eq!(parse_hhmm_minutes("24:00"), None);
        assert!(is_within_schedule_window(480, 480, 1110));
        assert!(is_within_schedule_window(1110, 480, 1110));
        assert!(!is_within_schedule_window(479, 480, 1110));
        assert!(!is_within_schedule_window(1111, 480, 1110));
    }

    #[test]
    fn test_run_dispatch_cycle_sends_sms_and_email_and_reports_results() {
        let pending_json = r#"{
            "site_id": "HA",
            "items": [
                {
                    "identity": {"ms03":"115","ms04_1":"HA81","ms04_2":"000100","ms30":"Y","ms_type":"M","ms14":"0912345678","ms_mail":"","gov_ext":null},
                    "retry_count": 0,
                    "channel": "sms",
                    "recipient": "0912345678",
                    "subject": "主旨1",
                    "msg": "簡訊內容"
                },
                {
                    "identity": {"ms03":"115","ms04_1":"SMHA","ms04_2":"000200","ms30":"y","ms_type":"M","ms14":"","ms_mail":"gov@example.com","gov_ext":null},
                    "retry_count": 1,
                    "channel": "email",
                    "recipient": "gov@example.com",
                    "subject": "機關通報",
                    "msg": "郵件內容"
                }
            ]
        }"#;

        let lah_transport = ScriptedTransport::new(
            vec![Ok(pending_json.into())],
            vec![
                Ok(r#"{"target_date":"1151113","user_cases_scanned":2,"user_notifications_inserted":1,"gov_cases_scanned":1,"gov_notifications_inserted":1}"#.into()),
                Ok(r#"{"moved_to_wait_count":2,"deleted_from_main_count":2}"#.into()),
                Ok(r#"{"action":"logged_success_and_removed","retry_count":0}"#.into()),
                Ok(r#"{"action":"retry_incremented","retry_count":2}"#.into()),
            ],
        );
        let gw_transport = ScriptedTransport::new(
            vec![],
            vec![
                Ok(r#"{"code":"1","desc":"OK"}"#.into()),
                Ok(r#"{"code":"0","desc":"SMTP 拒收"}"#.into()),
            ],
        );

        let lah = LahApiClient::with_transport(
            "http://127.0.0.1/api/sms_v1_api.php",
            lah_transport,
            Duration::from_secs(5),
        )
        .unwrap();
        let gw = SmsGatewayClient::with_transport(
            sample_gateway_cfg(),
            gw_transport,
            Duration::from_secs(5),
        )
        .unwrap();

        let stop = AtomicBool::new(false);
        let summary = run_dispatch_cycle(&lah, &gw, &sample_cfg(), &stop).unwrap();

        assert_eq!(summary.pending_count, 2);
        assert_eq!(summary.sent_count, 1);
        assert_eq!(summary.failed_count, 1);
        assert_eq!(summary.report_error_count, 0);
        assert!(!summary.stopped_early);
        assert!(summary
            .item_logs
            .iter()
            .any(|l| l.title == "排程步驟 1/4：案件掃描同步"));
        assert!(summary
            .item_logs
            .iter()
            .any(|l| l.title == "排程步驟 2/4：搬移待傳佇列"));
        assert!(summary
            .item_logs
            .iter()
            .any(|l| l.title == "排程步驟 3/4：查詢待傳清單"));
        assert!(summary
            .item_logs
            .iter()
            .any(|l| l.level == "success" && l.title == "通知派送成功"));
        let fail_log = summary
            .item_logs
            .iter()
            .find(|l| l.level == "error" && l.title == "通知派送失敗")
            .expect("應包含派送失敗紀錄");
        assert!(fail_log.message.contains("SMTP 拒收"));
    }

    #[test]
    fn test_run_dispatch_cycle_marks_report_failure_distinctly() {
        let pending_json = r#"{
            "site_id": "HA",
            "items": [{
                "identity": {"ms03":"115","ms04_1":"HA81","ms04_2":"000100","ms30":"Y","ms_type":"M","ms14":"0912345678","ms_mail":"","gov_ext":null},
                "retry_count": 0,
                "channel": "sms",
                "recipient": "0912345678",
                "subject": "主旨",
                "msg": "內容"
            }]
        }"#;

        let lah_transport = ScriptedTransport::new(
            vec![Ok(pending_json.into())],
            vec![
                Ok(r#"{"target_date":"1151113","user_cases_scanned":1,"user_notifications_inserted":0,"gov_cases_scanned":0,"gov_notifications_inserted":0}"#.into()),
                Ok(r#"{"moved_to_wait_count":0,"deleted_from_main_count":0}"#.into()),
                Err(anyhow::anyhow!("LAH-API 500")),
            ],
        );
        let gw_transport = ScriptedTransport::new(
            vec![],
            vec![Ok(r#"{"code":"1","desc":"OK"}"#.into())],
        );

        let lah = LahApiClient::with_transport(
            "http://127.0.0.1/api/sms_v1_api.php",
            lah_transport,
            Duration::from_secs(5),
        )
        .unwrap();
        let gw = SmsGatewayClient::with_transport(
            sample_gateway_cfg(),
            gw_transport,
            Duration::from_secs(5),
        )
        .unwrap();

        let stop = AtomicBool::new(false);
        let summary = run_dispatch_cycle(&lah, &gw, &sample_cfg(), &stop).unwrap();

        assert_eq!(summary.report_error_count, 1);
        assert_eq!(summary.sent_count, 0);
        let warn_log = summary
            .item_logs
            .iter()
            .find(|l| l.title == "已送出，結果回報待確認")
            .expect("應包含回報待確認紀錄");
        assert_eq!(warn_log.level, "warn");
    }
}
