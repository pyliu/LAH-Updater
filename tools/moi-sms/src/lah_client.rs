use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::comm::{validate_http_url, HttpTransport, UreqTransport};

/// 一般 `LAH-API` 查詢／回報請求逾時（30 秒）。
pub const DEFAULT_LAH_TIMEOUT: Duration = Duration::from_secs(30);
/// 案件掃描同步與佇列搬移逾時（240 秒，涵蓋 Oracle 9i 多表查詢與批次寫入耗時）。
pub const SYNC_LAH_TIMEOUT: Duration = Duration::from_secs(240);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct MetadataResponse {
    #[serde(default)]
    pub site_id: String,
    #[serde(default)]
    pub office_name: String,
    #[serde(default)]
    pub county_name: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct CaseSyncRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date_yyy_mm_dd: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CaseSyncResponse {
    #[serde(default)]
    pub target_date: String,
    #[serde(default)]
    pub user_cases_scanned: usize,
    #[serde(default)]
    pub user_notifications_inserted: usize,
    #[serde(default)]
    pub gov_cases_scanned: usize,
    #[serde(default)]
    pub gov_notifications_inserted: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct QueueEnqueueRequest {
    pub check_register_a: bool,
    pub check_register_b: bool,
    pub check_register_c: bool,
    pub check_register_d: bool,
}

impl Default for QueueEnqueueRequest {
    fn default() -> Self {
        Self {
            check_register_a: true,
            check_register_b: true,
            check_register_c: true,
            check_register_d: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct QueueEnqueueResponse {
    #[serde(default)]
    pub moved_to_wait_count: usize,
    #[serde(default)]
    pub deleted_from_main_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GovExt {
    #[serde(default)]
    pub app_seq: String,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub gov_id: String,
    #[serde(default)]
    pub cc00: String,
    #[serde(default)]
    pub cc48: String,
    #[serde(default)]
    pub cc49: String,
    #[serde(default)]
    pub gov_oid: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct RecordIdentity {
    #[serde(default)]
    pub ms03: String,
    #[serde(default)]
    pub ms04_1: String,
    #[serde(default)]
    pub ms04_2: String,
    #[serde(default)]
    pub ms30: String,
    #[serde(default)]
    pub ms_type: String,
    #[serde(default)]
    pub ms14: String,
    #[serde(default)]
    pub ms_mail: String,
    #[serde(default)]
    pub gov_ext: Option<GovExt>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PendingQueueItem {
    pub identity: RecordIdentity,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub recipient: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub msg: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PendingQueueResponse {
    #[serde(default)]
    pub site_id: String,
    #[serde(default)]
    pub items: Vec<PendingQueueItem>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct QueueReportRequest {
    pub identity: RecordIdentity,
    pub success: bool,
    pub msg: String,
    pub max_retries: u32,
    pub error_desc: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct QueueReportResponse {
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub retry_count: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct SmsTableRecord {
    #[serde(default)]
    pub ms03: String,
    #[serde(default)]
    pub ms04_1: String,
    #[serde(default)]
    pub ms04_2: String,
    #[serde(default)]
    pub ms07_1: String,
    #[serde(default)]
    pub ms07_2: String,
    #[serde(default)]
    pub ms14: String,
    #[serde(default)]
    pub ms_mail: String,
    #[serde(default)]
    pub ms30: String,
    #[serde(default)]
    pub ms31: String,
    #[serde(default)]
    pub ms32: String,
    #[serde(default)]
    pub ms_type: String,
    #[serde(default)]
    pub ms_note: String,
    #[serde(default)]
    pub ms_type_label: String,
    #[serde(default)]
    pub ms30_label: String,
    #[serde(default)]
    pub ms31_label: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct RecordsResponse {
    #[serde(default)]
    pub table: String,
    #[serde(default)]
    pub limit: usize,
    #[serde(default)]
    pub count: usize,
    #[serde(default)]
    pub records: Vec<SmsTableRecord>,
}

/// 封裝對 `LAH-API` REST 端點（`/metadata`、`/cases/sync`、`/queue/*`、`/records`）的 HTTP 用戶端。
#[derive(Debug, Clone)]
pub struct LahApiClient<T: HttpTransport = UreqTransport> {
    base_url: String,
    timeout: Duration,
    sync_timeout: Duration,
    transport: T,
}

impl LahApiClient<UreqTransport> {
    pub fn new(base_url: &str) -> Result<Self> {
        Self::with_timeouts(
            base_url,
            UreqTransport::default(),
            DEFAULT_LAH_TIMEOUT,
            SYNC_LAH_TIMEOUT,
        )
    }
}

impl<T: HttpTransport> LahApiClient<T> {
    pub fn with_transport(base_url: &str, transport: T, timeout: Duration) -> Result<Self> {
        Self::with_timeouts(base_url, transport, timeout, timeout.max(SYNC_LAH_TIMEOUT))
    }

    pub fn with_timeouts(
        base_url: &str,
        transport: T,
        timeout: Duration,
        sync_timeout: Duration,
    ) -> Result<Self> {
        validate_http_url(base_url).context("LAH-API URL 格式無效")?;
        Ok(Self {
            base_url: base_url.trim().trim_end_matches('/').to_string(),
            timeout,
            sync_timeout,
            transport,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn get_json<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let url = self.endpoint(path);
        let raw = self
            .transport
            .get(&url, self.timeout)
            .with_context(|| format!("呼叫 LAH-API GET {path} 失敗"))?;
        serde_json::from_str::<R>(&raw)
            .with_context(|| format!("解析 LAH-API GET {path} JSON 回應失敗"))
    }

    fn post_json_with_timeout<Req: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        req: &Req,
        timeout: Duration,
    ) -> Result<R> {
        let url = self.endpoint(path);
        let body = serde_json::to_value(req).context("序列化 LAH-API 請求失敗")?;
        let raw = self
            .transport
            .post_json(&url, &body, timeout)
            .with_context(|| format!("呼叫 LAH-API POST {path} 失敗"))?;
        serde_json::from_str::<R>(&raw)
            .with_context(|| format!("解析 LAH-API POST {path} JSON 回應失敗"))
    }

    pub fn get_metadata(&self) -> Result<MetadataResponse> {
        self.get_json("/metadata")
    }

    pub fn sync_cases(&self, target_date_yyy_mm_dd: Option<&str>) -> Result<CaseSyncResponse> {
        let req = CaseSyncRequest {
            target_date_yyy_mm_dd: target_date_yyy_mm_dd.map(str::to_string),
        };
        self.post_json_with_timeout("/cases/sync", &req, self.sync_timeout)
    }

    pub fn enqueue_queue(&self, req: &QueueEnqueueRequest) -> Result<QueueEnqueueResponse> {
        self.post_json_with_timeout("/queue/enqueue", req, self.sync_timeout)
    }

    pub fn get_pending_queue(&self) -> Result<PendingQueueResponse> {
        self.get_json("/queue/pending")
    }

    pub fn report_queue_item(&self, req: &QueueReportRequest) -> Result<QueueReportResponse> {
        self.post_json_with_timeout("/queue/report", req, self.timeout)
    }

    pub fn get_records(&self, table: &str, limit: usize) -> Result<RecordsResponse> {
        if !matches!(table, "main" | "wait" | "log") {
            bail!("不支援的資料表名稱: '{table}'（僅允許 main、wait、log）");
        }
        if !(1..=5000).contains(&limit) {
            bail!("limit 須介於 1～5000，收到: {limit}");
        }
        self.get_json(&format!("/records?table={table}&limit={limit}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct StubTransport {
        last_get: RefCell<Option<(String, Duration)>>,
        last_post: RefCell<Option<(String, serde_json::Value, Duration)>>,
        next_response: RefCell<String>,
        fail_with: Option<String>,
    }

    impl StubTransport {
        fn with_json(json: &str) -> Self {
            Self {
                last_get: RefCell::new(None),
                last_post: RefCell::new(None),
                next_response: RefCell::new(json.to_string()),
                fail_with: None,
            }
        }
    }

    impl HttpTransport for StubTransport {
        fn get(&self, url: &str, timeout: Duration) -> Result<String> {
            if let Some(err) = &self.fail_with {
                bail!("{err}");
            }
            *self.last_get.borrow_mut() = Some((url.to_string(), timeout));
            Ok(self.next_response.borrow().clone())
        }

        fn post_form(
            &self,
            _url: &str,
            _params: &[(&str, &str)],
            _timeout: Duration,
        ) -> Result<String> {
            unreachable!("LahApiClient 不使用 form POST")
        }

        fn post_json(
            &self,
            url: &str,
            body: &serde_json::Value,
            timeout: Duration,
        ) -> Result<String> {
            if let Some(err) = &self.fail_with {
                bail!("{err}");
            }
            *self.last_post.borrow_mut() = Some((url.to_string(), body.clone(), timeout));
            Ok(self.next_response.borrow().clone())
        }
    }

    #[test]
    fn test_client_rejects_invalid_url_and_normalizes_trailing_slash() {
        assert!(LahApiClient::with_transport(
            "ftp://127.0.0.1/api",
            StubTransport::with_json("{}"),
            DEFAULT_LAH_TIMEOUT
        )
        .is_err());

        let client = LahApiClient::with_transport(
            "http://127.0.0.1/api/sms_v1_api.php/",
            StubTransport::with_json(
                r#"{"site_id":"HA","office_name":"桃園地政事務所","county_name":"桃園市"}"#,
            ),
            DEFAULT_LAH_TIMEOUT,
        )
        .unwrap();
        assert_eq!(client.base_url(), "http://127.0.0.1/api/sms_v1_api.php");

        let meta = client.get_metadata().unwrap();
        assert_eq!(meta.site_id, "HA");
        assert_eq!(meta.office_name, "桃園地政事務所");
        assert_eq!(
            client.transport.last_get.borrow().as_ref().map(|(u, d)| (u.as_str(), *d)),
            Some(("http://127.0.0.1/api/sms_v1_api.php/metadata", DEFAULT_LAH_TIMEOUT))
        );
    }

    #[test]
    fn test_sync_enqueue_pending_report_and_records() {
        let client = LahApiClient::with_transport(
            "http://127.0.0.1/api/sms_v1_api.php",
            StubTransport::with_json(
                r#"{"target_date":"1151113","user_cases_scanned":5,"user_notifications_inserted":2,"gov_cases_scanned":1,"gov_notifications_inserted":1}"#,
            ),
            DEFAULT_LAH_TIMEOUT,
        )
        .unwrap();

        let sync = client.sync_cases(Some("1151113")).unwrap();
        assert_eq!(sync.user_notifications_inserted, 2);
        let (post_url, post_body, sync_timeout) = client.transport.last_post.borrow().clone().unwrap();
        assert_eq!(post_url, "http://127.0.0.1/api/sms_v1_api.php/cases/sync");
        assert_eq!(post_body["target_date_yyy_mm_dd"], "1151113");
        assert_eq!(sync_timeout, SYNC_LAH_TIMEOUT);

        *client.transport.next_response.borrow_mut() =
            r#"{"moved_to_wait_count":3,"deleted_from_main_count":3}"#.to_string();
        let enq = client
            .enqueue_queue(&QueueEnqueueRequest::default())
            .unwrap();
        assert_eq!(enq.moved_to_wait_count, 3);
        assert_eq!(
            client.transport.last_post.borrow().as_ref().map(|(_, _, d)| *d),
            Some(SYNC_LAH_TIMEOUT)
        );

        *client.transport.next_response.borrow_mut() =
            r#"{"site_id":"HA","items":[]}"#.to_string();
        let pending = client.get_pending_queue().unwrap();
        assert_eq!(pending.site_id, "HA");
        assert!(pending.items.is_empty());
        assert_eq!(
            client.transport.last_get.borrow().as_ref().map(|(_, d)| *d),
            Some(DEFAULT_LAH_TIMEOUT)
        );

        *client.transport.next_response.borrow_mut() =
            r#"{"action":"logged_success_and_removed"}"#.to_string();
        let report = client
            .report_queue_item(&QueueReportRequest {
                identity: RecordIdentity {
                    ms03: "115".into(),
                    ms04_1: "HA01".into(),
                    ms04_2: "001230".into(),
                    ms30: "Y".into(),
                    ms_type: "M".into(),
                    ms14: "0912345678".into(),
                    ms_mail: "".into(),
                    gov_ext: None,
                },
                success: true,
                msg: "ok".into(),
                max_retries: 3,
                error_desc: None,
            })
            .unwrap();
        assert_eq!(report.action, "logged_success_and_removed");
        assert_eq!(
            client.transport.last_post.borrow().as_ref().map(|(_, _, d)| *d),
            Some(DEFAULT_LAH_TIMEOUT)
        );

        *client.transport.next_response.borrow_mut() =
            r#"{"table":"wait","limit":100,"count":1,"records":[{"ms03":"115","ms04_1":"HA81","ms04_2":"197021","ms07_1":"1151113","ms07_2":"093000","ms14":"0912345678","ms30":"Y","ms32":"1","ms_type":"M","ms_type_label":"產權通知","ms30_label":"產權收件通知"}]}"#.to_string();
        let recs = client.get_records("wait", 100).unwrap();
        assert_eq!(recs.count, 1);
        assert_eq!(recs.records[0].ms04_2, "197021");
        assert!(client.get_records("unknown", 10).is_err());
        assert!(client.get_records("main", 0).is_err());
    }
}
