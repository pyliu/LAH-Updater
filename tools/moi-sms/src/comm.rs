use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::{MailConfig, SmsApiConfig};
use crate::encoding::encode_base64_utf8;

/// 原生 `SmsMainFrame` 使用 `HttpClient.setTimeout(4000)`，預設逾時為 4,000 毫秒。
pub const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_millis(4000);

/// HTTP 傳輸抽象層，便於單元測試與替換底層實作。
pub trait HttpTransport {
    fn get(&self, url: &str, timeout: Duration) -> Result<String>;

    fn post_form(
        &self,
        url: &str,
        params: &[(&str, &str)],
        timeout: Duration,
    ) -> Result<String>;

    fn post_json(
        &self,
        url: &str,
        body: &serde_json::Value,
        timeout: Duration,
    ) -> Result<String>;
}

/// 基於 `ureq` 的同步 HTTP 傳輸實作（重用 `ureq::Agent` 連線池）。
#[derive(Debug, Clone)]
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self {
            agent: ureq::Agent::new(),
        }
    }
}

impl HttpTransport for UreqTransport {
    fn get(&self, url: &str, timeout: Duration) -> Result<String> {
        let resp = self
            .agent
            .get(url)
            .timeout(timeout)
            .call()
            .with_context(|| format!("HTTP GET 請求失敗: {url}"))?;
        let status = resp.status();
        let text = resp
            .into_string()
            .with_context(|| format!("讀取 HTTP 回應內容失敗 (status={status})"))?;
        Ok(text)
    }

    fn post_form(
        &self,
        url: &str,
        params: &[(&str, &str)],
        timeout: Duration,
    ) -> Result<String> {
        let resp = self
            .agent
            .post(url)
            .timeout(timeout)
            .send_form(params)
            .with_context(|| format!("HTTP Form POST 請求失敗: {url}"))?;
        let status = resp.status();
        let text = resp
            .into_string()
            .with_context(|| format!("讀取 HTTP 回應內容失敗 (status={status})"))?;
        Ok(text)
    }

    fn post_json(
        &self,
        url: &str,
        body: &serde_json::Value,
        timeout: Duration,
    ) -> Result<String> {
        let resp = self
            .agent
            .post(url)
            .timeout(timeout)
            .send_json(body)
            .with_context(|| format!("HTTP JSON POST 請求失敗: {url}"))?;
        let status = resp.status();
        let text = resp
            .into_string()
            .with_context(|| format!("讀取 HTTP 回應內容失敗 (status={status})"))?;
        Ok(text)
    }
}

/// 對應 `SmsMainFrame.SendNewMsg` 的舊版相容 JSON 請求結構。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NewSmsPayload<'a> {
    #[serde(rename = "siteID")]
    pub site_id: &'a str,
    pub phone: &'a str,
    pub message: &'a str,
}

/// 對應 `LAH_API_SPEC.md §4.4.3` 簡訊派送 JSON 請求結構（`method = "m"`）。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GatewaySmsPayload<'a> {
    #[serde(rename = "siteID")]
    pub site_id: &'a str,
    pub method: &'a str,
    pub to: &'a str,
    pub msg: &'a str,
    pub sms_userid: &'a str,
    pub sms_pwd: &'a str,
}

/// 對應 `LAH_API_SPEC.md §4.4.3` 郵件派送 JSON 請求結構（`method = "e"`）。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GatewayEmailPayload<'a> {
    #[serde(rename = "siteID")]
    pub site_id: &'a str,
    pub method: &'a str,
    pub subject: &'a str,
    pub to: &'a str,
    pub msg: &'a str,
    pub mail_acct: &'a str,
    pub mail_pwd: &'a str,
    pub mail_from: &'a str,
    pub username: &'a str,
}

/// IMSP `ONEServiceServlet` 閘道 JSON 回應。
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GatewayDispatchResult {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub desc: String,
}

/// 簡訊閘道通訊用戶端（對應 `SmsMainFrame.SendMsg` 與 `SmsMainFrame.SendNewMsg`）。
#[derive(Debug, Clone)]
pub struct SmsGatewayClient<T: HttpTransport = UreqTransport> {
    config: SmsApiConfig,
    timeout: Duration,
    transport: T,
}

impl SmsGatewayClient<UreqTransport> {
    pub fn new(config: SmsApiConfig) -> Result<Self> {
        Self::with_transport(config, UreqTransport::default(), DEFAULT_HTTP_TIMEOUT)
    }
}

impl<T: HttpTransport> SmsGatewayClient<T> {
    pub fn with_transport(
        config: SmsApiConfig,
        transport: T,
        timeout: Duration,
    ) -> Result<Self> {
        validate_http_url(&config.ap_url)?;
        Ok(Self {
            config,
            timeout,
            transport,
        })
    }

    /// 對應 `SmsMainFrame.SendMsg(...)`：
    /// 以表單 POST 傳送 `ap_userid`、`ap_pwd`、目標門號與 UTF-8 Base64 編碼的簡訊內容。
    pub fn send_legacy_msg(&self, phone: &str, message: &str) -> Result<String> {
        if phone.trim().is_empty() {
            bail!("收件門號不可為空");
        }
        if message.is_empty() {
            bail!("簡訊內容不可為空");
        }

        let encoded_msg = encode_base64_utf8(message);
        let params = [
            ("userid", self.config.ap_userid.as_str()),
            ("pwd", self.config.ap_pwd.as_str()),
            ("phone", phone.trim()),
            ("msg", encoded_msg.as_str()),
        ];

        self.transport
            .post_form(&self.config.ap_url, &params, self.timeout)
            .with_context(|| format!("呼叫舊版簡訊 API 失敗 (phone={phone})"))
    }

    /// 對應 `SmsMainFrame.SendNewMsg(...)`：
    /// 帶入 `siteID`（來自 `srkeyn` `kcde_1='55' and kcde_2='01'`）並以 JSON POST 呼叫 `ap_url`。
    pub fn send_new_msg(&self, site_id: &str, phone: &str, message: &str) -> Result<String> {
        if site_id.trim().is_empty() {
            bail!("siteID 不可為空");
        }
        if phone.trim().is_empty() {
            bail!("收件門號不可為空");
        }
        if message.is_empty() {
            bail!("簡訊內容不可為空");
        }

        let payload = NewSmsPayload {
            site_id: site_id.trim(),
            phone: phone.trim(),
            message,
        };
        let json_body = serde_json::to_value(&payload)
            .context("序列化新版簡訊 JSON Payload 失敗")?;

        self.transport
            .post_json(&self.config.ap_url, &json_body, self.timeout)
            .with_context(|| format!("呼叫新版簡訊 JSON API 失敗 (siteID={site_id}, phone={phone})"))
    }

    /// 依 `LAH_API_SPEC.md §4.4.3` 規格發送簡訊（`method = "m"`），並驗證 `code == "1"`。
    pub fn send_sms_gateway(
        &self,
        site_id: &str,
        to: &str,
        msg: &str,
    ) -> Result<GatewayDispatchResult> {
        let site_id = site_id.trim();
        let to = to.trim();
        if site_id.is_empty() {
            bail!("siteID 不可為空");
        }
        if to.is_empty() {
            bail!("收件門號不可為空");
        }
        if msg.is_empty() {
            bail!("簡訊內容不可為空");
        }

        let payload = GatewaySmsPayload {
            site_id,
            method: "m",
            to,
            msg,
            sms_userid: self.config.ap_userid.trim(),
            sms_pwd: self.config.ap_pwd.as_str(),
        };
        let json_body = serde_json::to_value(&payload)
            .context("序列化簡訊閘道 JSON Payload 失敗")?;
        let raw = self
            .transport
            .post_json(&self.config.ap_url, &json_body, self.timeout)
            .with_context(|| format!("呼叫簡訊閘道失敗 (siteID={site_id}, to={to})"))?;
        parse_gateway_response(&raw)
    }

    /// 依 `LAH_API_SPEC.md §4.4.3` 規格發送郵件（`method = "e"`），並驗證 `code == "1"`。
    pub fn send_email_gateway(
        &self,
        site_id: &str,
        mail: &MailConfig,
        subject: &str,
        to: &str,
        msg: &str,
    ) -> Result<GatewayDispatchResult> {
        let site_id = site_id.trim();
        let to = to.trim();
        let subject = subject.trim();
        if site_id.is_empty() {
            bail!("siteID 不可為空");
        }
        if to.is_empty() {
            bail!("收件信箱不可為空");
        }
        if subject.is_empty() {
            bail!("郵件主旨不可為空");
        }
        if msg.is_empty() {
            bail!("郵件內容不可為空");
        }

        let payload = GatewayEmailPayload {
            site_id,
            method: "e",
            subject,
            to,
            msg,
            mail_acct: mail.account.trim(),
            mail_pwd: mail.password.as_str(),
            mail_from: mail.from_addr.trim(),
            username: mail.username.trim(),
        };
        let json_body = serde_json::to_value(&payload)
            .context("序列化郵件閘道 JSON Payload 失敗")?;
        let raw = self
            .transport
            .post_json(&self.config.ap_url, &json_body, self.timeout)
            .with_context(|| format!("呼叫郵件閘道失敗 (siteID={site_id}, to={to})"))?;
        parse_gateway_response(&raw)
    }
}

/// 解析閘道回應 JSON，僅當 `code == "1"` 時視為成功。
pub fn parse_gateway_response(raw: &str) -> Result<GatewayDispatchResult> {
    let parsed: GatewayDispatchResult = serde_json::from_str(raw.trim())
        .with_context(|| format!("閘道回應非有效 JSON: {}", raw.trim()))?;
    if parsed.code.trim() != "1" {
        bail!(
            "閘道回傳失敗 (code={}, desc={})",
            parsed.code.trim(),
            parsed.desc.trim()
        );
    }
    Ok(parsed)
}

pub fn validate_http_url(url: &str) -> Result<()> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        bail!("URL 不可為空");
    }
    if trimmed.chars().any(char::is_whitespace) {
        bail!("URL 不可包含空白字元");
    }
    let rest = if let Some(r) = trimmed.strip_prefix("http://") {
        r
    } else if let Some(r) = trimmed.strip_prefix("https://") {
        r
    } else {
        bail!("URL 必須為 http:// 或 https:// 開頭，收到: '{trimmed}'");
    };
    let authority = rest.split('/').next().unwrap_or("");
    if authority.is_empty() || authority.contains('@') {
        bail!("URL 必須包含有效主機名稱且不可內嵌帳密");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct MockTransport {
        last_form: RefCell<Option<(String, Vec<(String, String)>, Duration)>>,
        last_json: RefCell<Option<(String, serde_json::Value, Duration)>>,
        json_response: Option<String>,
        fail_with: Option<String>,
    }

    impl HttpTransport for MockTransport {
        fn get(&self, _url: &str, _timeout: Duration) -> Result<String> {
            if let Some(msg) = &self.fail_with {
                bail!("{msg}");
            }
            Ok("{}".to_string())
        }

        fn post_form(
            &self,
            url: &str,
            params: &[(&str, &str)],
            timeout: Duration,
        ) -> Result<String> {
            if let Some(msg) = &self.fail_with {
                bail!("{msg}");
            }
            *self.last_form.borrow_mut() = Some((
                url.to_string(),
                params
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect(),
                timeout,
            ));
            Ok("OK".to_string())
        }

        fn post_json(
            &self,
            url: &str,
            body: &serde_json::Value,
            timeout: Duration,
        ) -> Result<String> {
            if let Some(msg) = &self.fail_with {
                bail!("{msg}");
            }
            *self.last_json.borrow_mut() = Some((url.to_string(), body.clone(), timeout));
            Ok(self
                .json_response
                .clone()
                .unwrap_or_else(|| r#"{"code":"1","desc":"OK"}"#.to_string()))
        }
    }

    fn sample_api_cfg() -> SmsApiConfig {
        SmsApiConfig {
            ap_url: "https://sms.example.gov.tw/send".to_string(),
            ap_userid: "moi_user".to_string(),
            ap_pwd: "moi_pass".to_string(),
        }
    }

    #[test]
    fn test_send_legacy_msg_encodes_utf8_base64_and_timeout() {
        let client = SmsGatewayClient::with_transport(
            sample_api_cfg(),
            MockTransport::default(),
            DEFAULT_HTTP_TIMEOUT,
        )
        .unwrap();

        let resp = client.send_legacy_msg("0912345678", "案件通知").unwrap();
        assert_eq!(resp, "OK");

        let recorded = client.transport.last_form.borrow();
        let (url, params, timeout) = recorded.as_ref().unwrap();
        assert_eq!(url, "https://sms.example.gov.tw/send");
        assert_eq!(*timeout, Duration::from_millis(4000));
        assert_eq!(params[0], ("userid".to_string(), "moi_user".to_string()));
        assert_eq!(params[1], ("pwd".to_string(), "moi_pass".to_string()));
        assert_eq!(params[2], ("phone".to_string(), "0912345678".to_string()));
        assert_eq!(
            params[3],
            ("msg".to_string(), encode_base64_utf8("案件通知"))
        );
    }

    #[test]
    fn test_send_new_msg_builds_json_with_site_id() {
        let client = SmsGatewayClient::with_transport(
            sample_api_cfg(),
            MockTransport::default(),
            DEFAULT_HTTP_TIMEOUT,
        )
        .unwrap();

        let resp = client
            .send_new_msg("SITE01", "0912345678", "系統測試")
            .unwrap();
        assert_eq!(resp, r#"{"code":"1","desc":"OK"}"#);

        let recorded = client.transport.last_json.borrow();
        let (url, body, _) = recorded.as_ref().unwrap();
        assert_eq!(url, "https://sms.example.gov.tw/send");
        assert_eq!(body["siteID"], "SITE01");
        assert_eq!(body["phone"], "0912345678");
        assert_eq!(body["message"], "系統測試");
    }

    #[test]
    fn test_comm_boundary_validation_and_transport_errors() {
        // 無效 URL
        let bad_cfg = SmsApiConfig {
            ap_url: "ftp://invalid".to_string(),
            ap_userid: "u".to_string(),
            ap_pwd: "p".to_string(),
        };
        assert!(SmsGatewayClient::with_transport(
            bad_cfg,
            MockTransport::default(),
            DEFAULT_HTTP_TIMEOUT
        )
        .is_err());

        // 空門號、空訊息、空 siteID
        let client = SmsGatewayClient::with_transport(
            sample_api_cfg(),
            MockTransport::default(),
            DEFAULT_HTTP_TIMEOUT,
        )
        .unwrap();
        assert!(client.send_legacy_msg("   ", "hello").is_err());
        assert!(client.send_legacy_msg("0912345678", "").is_err());
        assert!(client.send_new_msg("", "0912345678", "hello").is_err());

        // 傳輸層錯誤包裝 anyhow context
        let failing = SmsGatewayClient::with_transport(
            sample_api_cfg(),
            MockTransport {
                fail_with: Some("connection timed out".to_string()),
                ..Default::default()
            },
            DEFAULT_HTTP_TIMEOUT,
        )
        .unwrap();
        let err = failing.send_new_msg("SITE01", "0912345678", "hi").unwrap_err();
        assert!(format!("{err:#}").contains("connection timed out"));
    }

    #[test]
    fn test_send_sms_and_email_gateway_aligns_with_spec_and_checks_code() {
        let client = SmsGatewayClient::with_transport(
            sample_api_cfg(),
            MockTransport::default(),
            DEFAULT_HTTP_TIMEOUT,
        )
        .unwrap();

        let sms_res = client
            .send_sms_gateway("HA", "0912345678", "案件收件通知")
            .unwrap();
        assert_eq!(sms_res.code, "1");
        {
            let recorded = client.transport.last_json.borrow();
            let (_, body, _) = recorded.as_ref().unwrap();
            assert_eq!(body["siteID"], "HA");
            assert_eq!(body["method"], "m");
            assert_eq!(body["to"], "0912345678");
            assert_eq!(body["msg"], "案件收件通知");
            assert_eq!(body["sms_userid"], "moi_user");
            assert_eq!(body["sms_pwd"], "moi_pass");
        }

        let mail_cfg = MailConfig {
            smtp_host: String::new(),
            smtp_port: None,
            account: "mail_acct".to_string(),
            password: "mail_pw".to_string(),
            from_addr: "noreply@land.gov.tw".to_string(),
            username: "桃園地政事務所".to_string(),
        };
        let mail_res = client
            .send_email_gateway("HA", &mail_cfg, "異動通知", "owner@example.com", "內文")
            .unwrap();
        assert_eq!(mail_res.code, "1");
        {
            let recorded = client.transport.last_json.borrow();
            let (_, body, _) = recorded.as_ref().unwrap();
            assert_eq!(body["siteID"], "HA");
            assert_eq!(body["method"], "e");
            assert_eq!(body["subject"], "異動通知");
            assert_eq!(body["to"], "owner@example.com");
            assert_eq!(body["msg"], "內文");
            assert_eq!(body["mail_acct"], "mail_acct");
            assert_eq!(body["mail_pwd"], "mail_pw");
            assert_eq!(body["mail_from"], "noreply@land.gov.tw");
            assert_eq!(body["username"], "桃園地政事務所");
        }

        // code != "1" 時必須判定失敗
        let rejected = SmsGatewayClient::with_transport(
            sample_api_cfg(),
            MockTransport {
                json_response: Some(r#"{"code":"0","desc":"帳號驗證失敗"}"#.to_string()),
                ..Default::default()
            },
            DEFAULT_HTTP_TIMEOUT,
        )
        .unwrap();
        let err = rejected
            .send_sms_gateway("HA", "0912345678", "測試")
            .unwrap_err();
        assert!(format!("{err:#}").contains("帳號驗證失敗"));
    }
}
