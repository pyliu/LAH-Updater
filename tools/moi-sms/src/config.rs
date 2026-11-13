use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use encoding_rs::BIG5;

use crate::encoding::decode_base64_legacy;

/// 主資料庫連線設定（對應 `util.ODatabase` 讀取 `env.properties`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbConfig {
    pub driver_class: String,
    pub url: String,
    pub user: String,
    pub password: String,
}

/// 備用／舊版資料庫連線設定（對應 `util.Database` 讀取 `TPCEx.properties` 明文）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyDbConfig {
    pub driver_class: String,
    pub url: String,
    pub user: String,
    pub password: String,
}

/// 簡訊閘道 HTTP API 設定（對應 `SmsMainFrame` 自 `env.properties` 讀取之 `ap_*`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmsApiConfig {
    pub ap_url: String,
    pub ap_userid: String,
    pub ap_pwd: String,
}

/// SMTP 郵件伺服器設定（對應 `SmsMainFrame` 讀取之 `mail_*` 欄位）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailConfig {
    pub smtp_host: String,
    pub smtp_port: Option<u16>,
    pub account: String,
    pub password: String,
    pub from_addr: String,
    pub username: String,
}

/// 排程時間設定（對應 `SmsMainFrame` 讀取之 Timer 參數）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleConfig {
    pub search_time: Option<u64>,
    pub resend_time: Option<u64>,
    pub start_hours: Option<u8>,
    pub start_min: Option<u8>,
    pub end_hours: Option<u8>,
    pub end_min: Option<u8>,
}

/// 完整系統組態（對應 `env.properties`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvConfig {
    pub db: DbConfig,
    pub sms_api: Option<SmsApiConfig>,
    pub mail: Option<MailConfig>,
    pub schedule: ScheduleConfig,
    pub raw_properties: HashMap<String, String>,
}

/// 解析以 Big5（或相容 ASCII/MS950）編碼的 Java `.properties` 檔案位元組。
///
/// 對應 `SmsMainFrame.readenv()` 使用 `InputStreamReader(..., "BIG5")` 載入 `Properties`。
pub fn parse_properties_bytes(bytes: &[u8]) -> Result<HashMap<String, String>> {
    let (decoded, had_errors) = BIG5.decode_without_bom_handling(bytes);
    if had_errors {
        bail!("properties 檔案包含無法以 Big5 解碼的位元組序列");
    }

    let mut map = HashMap::new();
    for raw_line in decoded.lines() {
        let trimmed = raw_line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
            continue;
        }

        let Some((key, val)) = split_property_line(trimmed) else {
            continue;
        };

        let clean_key = unescape_property_text(key.trim());
        let clean_val = unescape_property_text(val.trim_start());
        if !clean_key.is_empty() {
            map.insert(clean_key, clean_val);
        }
    }

    Ok(map)
}

fn split_property_line(line: &str) -> Option<(&str, &str)> {
    let mut escaped = false;
    for (idx, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '=' || ch == ':' {
            let (k, rest) = line.split_at(idx);
            return Some((k, &rest[ch.len_utf8()..]));
        }
    }
    None
}

fn unescape_property_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

fn get_required(map: &HashMap<String, String>, key: &str) -> Result<String> {
    map.get(key)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .with_context(|| format!("缺少必要設定欄位或欄位為空: {key}"))
}

fn get_required_b64(map: &HashMap<String, String>, key: &str) -> Result<String> {
    let raw = get_required(map, key)?;
    decode_base64_legacy(&raw).with_context(|| format!("設定欄位 {key} Base64 解碼失敗"))
}

fn get_optional_b64(map: &HashMap<String, String>, key: &str) -> Result<Option<String>> {
    match map.get(key) {
        Some(v) if !v.trim().is_empty() => {
            let decoded = decode_base64_legacy(v.trim())
                .with_context(|| format!("設定欄位 {key} Base64 解碼失敗"))?;
            Ok(Some(decoded))
        }
        _ => Ok(None),
    }
}

fn get_first_optional_b64(map: &HashMap<String, String>, keys: &[&str]) -> Result<Option<String>> {
    for key in keys {
        if let Some(val) = get_optional_b64(map, key)? {
            return Ok(Some(val));
        }
    }
    Ok(None)
}

fn parse_optional_u64(map: &HashMap<String, String>, key: &str) -> Result<Option<u64>> {
    match map.get(key) {
        Some(v) if !v.trim().is_empty() => {
            let parsed = v
                .trim()
                .parse::<u64>()
                .with_context(|| format!("設定欄位 {key} 無法解析為正整數: '{v}'"))?;
            Ok(Some(parsed))
        }
        _ => Ok(None),
    }
}

fn parse_optional_u8(map: &HashMap<String, String>, key: &str) -> Result<Option<u8>> {
    match map.get(key) {
        Some(v) if !v.trim().is_empty() => {
            let parsed = v
                .trim()
                .parse::<u8>()
                .with_context(|| format!("設定欄位 {key} 無法解析為 0..255 整數: '{v}'"))?;
            Ok(Some(parsed))
        }
        _ => Ok(None),
    }
}

/// 桌面端讀寫 `env.properties` 所需之設定集合（不要求包含舊版 DB 連線欄位）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvSettingsData {
    pub lah_api_url: Option<String>,
    pub ap_url: Option<String>,
    pub sms_user: String,
    pub sms_pwd: String,
    pub mail_account: String,
    pub mail_pwd: String,
    pub mail_from: String,
    pub username: String,
    pub search_minutes: Option<u64>,
    pub max_retries: Option<u64>,
    pub start_hours: Option<u8>,
    pub start_min: Option<u8>,
    pub end_hours: Option<u8>,
    pub end_min: Option<u8>,
    pub auto_start_schedule: Option<bool>,
}

/// 寫回 `env.properties` 的更新資料。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvSavePatch {
    pub lah_api_url: String,
    pub ap_url: String,
    pub sms_user: String,
    pub sms_pwd: String,
    pub mail_account: String,
    pub mail_pwd: String,
    pub mail_from: String,
    pub username: String,
    pub search_minutes: u32,
    pub max_retries: u32,
    pub start_hours: u8,
    pub start_min: u8,
    pub end_hours: u8,
    pub end_min: u8,
    pub auto_start_schedule: bool,
}

fn parse_optional_bool(map: &HashMap<String, String>, key: &str) -> Option<bool> {
    map.get(key).and_then(|v| match v.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    })
}

/// 以寬鬆模式讀取 `env.properties`（無須包含 `odriver_class` / `oUrl` / `oUser` / `oPwd`）。
pub fn read_env_settings_file(path: &Path) -> Result<EnvSettingsData> {
    let bytes = fs::read(path)
        .with_context(|| format!("無法讀取設定檔: {}", path.display()))?;
    read_env_settings_bytes(&bytes)
        .with_context(|| format!("解析設定檔失敗: {}", path.display()))
}

/// 以寬鬆模式從位元組解析 `env.properties`。
pub fn read_env_settings_bytes(bytes: &[u8]) -> Result<EnvSettingsData> {
    let map = parse_properties_bytes(bytes)?;
    Ok(EnvSettingsData {
        lah_api_url: map
            .get("lah_api_url")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        ap_url: get_optional_b64(&map, "ap_url")?,
        sms_user: get_first_optional_b64(&map, &["smsUser", "ap_userid", "apUser"])?
            .unwrap_or_default(),
        sms_pwd: get_first_optional_b64(&map, &["smsPwd", "ap_pwd", "apPwd"])?
            .unwrap_or_default(),
        mail_account: get_first_optional_b64(&map, &["user", "mail_acct"])?.unwrap_or_default(),
        mail_pwd: get_first_optional_b64(&map, &["pwd", "mail_pwd"])?.unwrap_or_default(),
        mail_from: map.get("mailaddr").map(|s| s.trim().to_string()).unwrap_or_default(),
        username: map.get("username").map(|s| s.trim().to_string()).unwrap_or_default(),
        search_minutes: parse_optional_u64(&map, "search_time")?,
        max_retries: parse_optional_u64(&map, "resend_time")?,
        start_hours: parse_optional_u8(&map, "start_hours")?,
        start_min: parse_optional_u8(&map, "start_min")?,
        end_hours: parse_optional_u8(&map, "end_hours")?,
        end_min: parse_optional_u8(&map, "end_min")?,
        auto_start_schedule: parse_optional_bool(&map, "auto_start_schedule"),
    })
}

fn encode_b64_or_empty(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        STANDARD.encode(trimmed.as_bytes())
    }
}

/// 無損更新 `env.properties` 內容（保留未管理之欄位與註解，並轉回 Big5 位元組）。
pub fn update_env_properties_bytes(existing: Option<&[u8]>, patch: &EnvSavePatch) -> Result<Vec<u8>> {
    let updates: Vec<(&str, String)> = vec![
        ("lah_api_url", patch.lah_api_url.trim().to_string()),
        ("ap_url", encode_b64_or_empty(&patch.ap_url)),
        ("smsUser", encode_b64_or_empty(&patch.sms_user)),
        ("smsPwd", encode_b64_or_empty(&patch.sms_pwd)),
        ("user", encode_b64_or_empty(&patch.mail_account)),
        ("pwd", encode_b64_or_empty(&patch.mail_pwd)),
        ("mailaddr", patch.mail_from.trim().to_string()),
        ("username", patch.username.trim().to_string()),
        ("search_time", patch.search_minutes.to_string()),
        ("resend_time", patch.max_retries.to_string()),
        ("start_hours", patch.start_hours.to_string()),
        ("start_min", patch.start_min.to_string()),
        ("end_hours", patch.end_hours.to_string()),
        ("end_min", patch.end_min.to_string()),
        (
            "auto_start_schedule",
            patch.auto_start_schedule.to_string(),
        ),
    ];

    let update_map: HashMap<&str, &str> = updates
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    let mut written_keys: HashSet<&str> = HashSet::new();
    let mut out_lines: Vec<String> = Vec::new();

    if let Some(bytes) = existing {
        let (decoded, had_errors) = BIG5.decode_without_bom_handling(bytes);
        if had_errors {
            bail!("既有 env.properties 包含無法以 Big5 解碼的位元組序列");
        }
        for raw_line in decoded.lines() {
            let trimmed = raw_line.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                out_lines.push(raw_line.to_string());
                continue;
            }
            if let Some((raw_key, _)) = split_property_line(trimmed) {
                let clean_key = unescape_property_text(raw_key.trim());
                if let Some((&canonical_key, &new_val)) = update_map.get_key_value(clean_key.as_str()) {
                    out_lines.push(format!("{canonical_key}={new_val}"));
                    written_keys.insert(canonical_key);
                    continue;
                }
            }
            out_lines.push(raw_line.to_string());
        }
    }

    for (key, val) in &updates {
        if !written_keys.contains(key) {
            out_lines.push(format!("{key}={val}"));
        }
    }

    let joined = format!("{}\r\n", out_lines.join("\r\n"));
    let (encoded, _, had_unmappable) = BIG5.encode(&joined);
    if had_unmappable {
        bail!("設定內容包含無法以 Big5 編碼的字元，請檢查輸入內容");
    }
    Ok(encoded.into_owned())
}

/// 備份既有 `env.properties`（若有）並寫回更新後內容，回傳備份檔路徑（若有備份）。
pub fn save_env_properties_file(
    path: &Path,
    patch: &EnvSavePatch,
    backup_tag: &str,
) -> Result<Option<PathBuf>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("無法建立設定目錄: {}", parent.display()))?;
    }

    let (existing_bytes, backup_path) = if path.exists() {
        let raw = fs::read(path)
            .with_context(|| format!("讀取既有設定檔失敗: {}", path.display()))?;
        let bak_name = format!(
            "{}.bak.{}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("env.properties"),
            backup_tag
        );
        let bak_path = path.with_file_name(bak_name);
        fs::copy(path, &bak_path)
            .with_context(|| format!("建立設定備份失敗: {}", bak_path.display()))?;
        (Some(raw), Some(bak_path))
    } else {
        (None, None)
    };

    let updated = update_env_properties_bytes(existing_bytes.as_deref(), patch)?;
    let tmp_path = path.with_extension("properties.tmp");
    fs::write(&tmp_path, updated)
        .with_context(|| format!("寫入暫存設定檔失敗: {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .with_context(|| format!("更新設定檔失敗: {}", path.display()))?;

    Ok(backup_path)
}

impl EnvConfig {
    /// 從檔案路徑載入 `env.properties`。
    pub fn from_file(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)
            .with_context(|| format!("無法讀取設定檔: {}", path.display()))?;
        Self::from_bytes(&bytes)
            .with_context(|| format!("解析設定檔失敗: {}", path.display()))
    }

    /// 從原始位元組解析 `env.properties`。
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let map = parse_properties_bytes(bytes)?;

        let db = DbConfig {
            driver_class: get_required(&map, "odriver_class")?,
            url: get_required_b64(&map, "oUrl")?,
            user: get_required_b64(&map, "oUser")?,
            password: get_required_b64(&map, "oPwd")?,
        };

        let sms_api = match get_optional_b64(&map, "ap_url")? {
            Some(ap_url) => Some(SmsApiConfig {
                ap_url,
                ap_userid: get_first_optional_b64(&map, &["smsUser", "ap_userid", "apUser"])?
                    .unwrap_or_default(),
                ap_pwd: get_first_optional_b64(&map, &["smsPwd", "ap_pwd", "apPwd"])?
                    .unwrap_or_default(),
            }),
            None => None,
        };

        // 原生 SmsMainFrame 使用 "mail_stmp"（拼字如此），亦相容 "mail_smtp"
        let smtp_host = get_first_optional_b64(&map, &["mail_stmp", "mail_smtp"])?;
        let mail = if smtp_host.is_some()
            || map.contains_key("user")
            || map.contains_key("mail_acct")
            || map.contains_key("mailaddr")
        {
            let smtp_port = match map.get("mail_port") {
                Some(v) if !v.trim().is_empty() => Some(
                    v.trim()
                        .parse::<u16>()
                        .with_context(|| format!("mail_port 格式錯誤: '{v}'"))?,
                ),
                _ => None,
            };
            Some(MailConfig {
                smtp_host: smtp_host.unwrap_or_default(),
                smtp_port,
                account: get_first_optional_b64(&map, &["user", "mail_acct"])?.unwrap_or_default(),
                password: get_first_optional_b64(&map, &["pwd", "mail_pwd"])?.unwrap_or_default(),
                from_addr: map.get("mailaddr").cloned().unwrap_or_default(),
                username: map.get("username").cloned().unwrap_or_default(),
            })
        } else {
            None
        };

        let schedule = ScheduleConfig {
            search_time: parse_optional_u64(&map, "search_time")?,
            resend_time: parse_optional_u64(&map, "resend_time")?,
            start_hours: parse_optional_u8(&map, "start_hours")?,
            start_min: parse_optional_u8(&map, "start_min")?,
            end_hours: parse_optional_u8(&map, "end_hours")?,
            end_min: parse_optional_u8(&map, "end_min")?,
        };

        Ok(Self {
            db,
            sms_api,
            mail,
            schedule,
            raw_properties: map,
        })
    }
}

impl LegacyDbConfig {
    /// 解析舊版 `TPCEx.properties`（明文儲存，對應 `util.Database.readProperty()`）。
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let map = parse_properties_bytes(bytes)?;
        Ok(Self {
            driver_class: get_required(&map, "sdriver_class")?,
            url: get_required(&map, "sUrl")?,
            user: get_required(&map, "sUser")?,
            password: get_required(&map, "sPwd")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_config_with_big5_and_base64() {
        let o_url = STANDARD.encode("jdbc:db2://10.0.0.1:50000/MOIDB");
        let o_user = STANDARD.encode("dbuser");
        let o_pwd = STANDARD.encode("secret!123");
        let ap_url = STANDARD.encode("https://sms.example.gov.tw/api/send");
        let ap_userid = STANDARD.encode("moi_sms");
        let ap_pwd = STANDARD.encode("sms_pass");
        let mail_stmp = STANDARD.encode("smtp.example.gov.tw");

        // 含 Big5 中文註解與 username 欄位（測試 BIG5 解碼）
        let mut raw_bytes = Vec::new();
        raw_bytes.extend_from_slice(b"# ");
        raw_bytes.extend_from_slice(&[0xB4, 0xFA, 0xB8, 0xD5]); // Big5「測試」
        raw_bytes.extend_from_slice(
            format!(
                "\nodriver_class=com.ibm.db2.jcc.DB2Driver\n\
                 oUrl={o_url}\n\
                 oUser={o_user}\n\
                 oPwd={o_pwd}\n\
                 ap_url={ap_url}\n\
                 ap_userid={ap_userid}\n\
                 ap_pwd={ap_pwd}\n\
                 mail_stmp={mail_stmp}\n\
                 mail_port=465\n\
                 mailaddr=noreply@example.gov.tw\n\
                 search_time=60\n\
                 start_hours=8\n\
                 username="
            )
            .as_bytes(),
        );
        raw_bytes.extend_from_slice(&[0xB4, 0xFA, 0xB8, 0xD5]); // username=測試

        let cfg = EnvConfig::from_bytes(&raw_bytes).expect("應成功解析含 Big5 與 Base64 之 env.properties");
        assert_eq!(cfg.db.driver_class, "com.ibm.db2.jcc.DB2Driver");
        assert_eq!(cfg.db.url, "jdbc:db2://10.0.0.1:50000/MOIDB");
        assert_eq!(cfg.db.user, "dbuser");
        assert_eq!(cfg.db.password, "secret!123");

        let sms = cfg.sms_api.expect("應解析出 SmsApiConfig");
        assert_eq!(sms.ap_url, "https://sms.example.gov.tw/api/send");
        assert_eq!(sms.ap_userid, "moi_sms");
        assert_eq!(sms.ap_pwd, "sms_pass");

        let mail = cfg.mail.expect("應解析出 MailConfig");
        assert_eq!(mail.smtp_host, "smtp.example.gov.tw");
        assert_eq!(mail.smtp_port, Some(465));
        assert_eq!(mail.from_addr, "noreply@example.gov.tw");
        assert_eq!(mail.username, "測試");

        assert_eq!(cfg.schedule.search_time, Some(60));
        assert_eq!(cfg.schedule.start_hours, Some(8));
    }

    #[test]
    fn test_env_config_missing_required_or_invalid_base64_fails() {
        let missing = b"odriver_class=com.ibm.db2.jcc.DB2Driver\noUrl=aHR0cDovL2E=\n";
        let err = EnvConfig::from_bytes(missing).unwrap_err();
        assert!(err.to_string().contains("oUser"));

        let bad_b64 = b"odriver_class=D\noUrl=!!!not_base64!!!\noUser=dQ==\noPwd=cA==\n";
        let err2 = EnvConfig::from_bytes(bad_b64).unwrap_err();
        assert!(err2.to_string().contains("oUrl"));
    }

    #[test]
    fn test_legacy_db_config_plaintext() {
        let raw = b"sdriver_class=oracle.jdbc.driver.OracleDriver\nsUrl=jdbc\\:oracle\\:thin\\:@localhost\\:1521\\:orcl\nsUser=scott\nsPwd=tiger\n";
        let cfg = LegacyDbConfig::from_bytes(raw).unwrap();
        assert_eq!(cfg.driver_class, "oracle.jdbc.driver.OracleDriver");
        assert_eq!(cfg.url, "jdbc:oracle:thin:@localhost:1521:orcl");
        assert_eq!(cfg.user, "scott");
        assert_eq!(cfg.password, "tiger");
    }

    #[test]
    fn test_parse_env_config_supports_smsuser_and_user_keys() {
        let mut raw = format!(
            "odriver_class=oracle.jdbc.driver.OracleDriver\n\
             oUrl={}\n\
             oUser={}\n\
             oPwd={}\n\
             ap_url={}\n\
             smsUser={}\n\
             smsPwd={}\n\
             user={}\n\
             pwd={}\n\
             mailaddr=ha-landnotify@mail.tycg.gov.tw\n\
             username=",
            STANDARD.encode("jdbc:oracle:thin:@127.0.0.1:1521:orcl"),
            STANDARD.encode("ora_u"),
            STANDARD.encode("ora_p"),
            STANDARD.encode("http://220.1.33.122:8080/IMSP/ONEServiceServlet"),
            STANDARD.encode("sms_acct_01"),
            STANDARD.encode("sms_secret_01"),
            STANDARD.encode("mail_acct_01"),
            STANDARD.encode("mail_secret_01"),
        )
        .into_bytes();
        let (big5_name, _, _) = BIG5.encode("桃園地政事務所");
        raw.extend_from_slice(&big5_name);
        raw.push(b'\n');

        let cfg = EnvConfig::from_bytes(&raw).unwrap();
        let sms = cfg.sms_api.expect("應解析出 SmsApiConfig");
        assert_eq!(sms.ap_userid, "sms_acct_01");
        assert_eq!(sms.ap_pwd, "sms_secret_01");

        let mail = cfg.mail.expect("應解析出 MailConfig");
        assert_eq!(mail.account, "mail_acct_01");
        assert_eq!(mail.password, "mail_secret_01");
        assert_eq!(mail.from_addr, "ha-landnotify@mail.tycg.gov.tw");
        assert_eq!(mail.username, "桃園地政事務所");
    }

    #[test]
    fn test_update_and_save_env_properties_preserves_unknown_keys_and_creates_backup() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("moi_env_write_test_{unique}"));
        fs::create_dir_all(&dir).unwrap();
        let env_path = dir.join("env.properties");

        let (initial_big5, _, _) = BIG5.encode(
            "# 舊版設定保留\r\nodriver_class=oracle.jdbc.driver.OracleDriver\r\noUrl=b3Jh\r\noUser=dQ==\r\noPwd=cA==\r\napUser=R00=\r\nap_url=b2xk\r\nsmsUser=b2xkX3U=\r\n",
        );
        fs::write(&env_path, initial_big5.as_ref()).unwrap();

        let patch = EnvSavePatch {
            lah_api_url: "http://127.0.0.1/LAH-API/v1/sms_v1_api.php".into(),
            ap_url: "http://220.1.33.122:8080/IMSP/ONEServiceServlet".into(),
            sms_user: "new_sms_u".into(),
            sms_pwd: "new_sms_p".into(),
            mail_account: "new_mail_u".into(),
            mail_pwd: "new_mail_p".into(),
            mail_from: "notify@example.gov.tw".into(),
            username: "桃園地政事務所".into(),
            search_minutes: 3,
            max_retries: 5,
            start_hours: 8,
            start_min: 30,
            end_hours: 17,
            end_min: 45,
            auto_start_schedule: true,
        };

        let bak = save_env_properties_file(&env_path, &patch, "20261113_163000")
            .unwrap()
            .expect("應建立時間戳備份檔");
        assert!(bak.exists());

        // 確認未管理欄位與註解皆完整保留
        let saved_bytes = fs::read(&env_path).unwrap();
        let (decoded, had_err) = BIG5.decode_without_bom_handling(&saved_bytes);
        assert!(!had_err);
        assert!(decoded.contains("# 舊版設定保留"));
        assert!(decoded.contains("oUrl=b3Jh"));
        assert!(decoded.contains("apUser=R00="));

        // 確認寬鬆模式可完整讀回
        let reloaded = read_env_settings_file(&env_path).unwrap();
        assert_eq!(reloaded.lah_api_url.as_deref(), Some("http://127.0.0.1/LAH-API/v1/sms_v1_api.php"));
        assert_eq!(reloaded.ap_url.as_deref(), Some("http://220.1.33.122:8080/IMSP/ONEServiceServlet"));
        assert_eq!(reloaded.sms_user, "new_sms_u");
        assert_eq!(reloaded.sms_pwd, "new_sms_p");
        assert_eq!(reloaded.mail_account, "new_mail_u");
        assert_eq!(reloaded.mail_pwd, "new_mail_p");
        assert_eq!(reloaded.mail_from, "notify@example.gov.tw");
        assert_eq!(reloaded.username, "桃園地政事務所");
        assert_eq!(reloaded.search_minutes, Some(3));
        assert_eq!(reloaded.max_retries, Some(5));
        assert_eq!(reloaded.start_hours, Some(8));
        assert_eq!(reloaded.start_min, Some(30));
        assert_eq!(reloaded.end_hours, Some(17));
        assert_eq!(reloaded.end_min, Some(45));
        assert_eq!(reloaded.auto_start_schedule, Some(true));

        let _ = fs::remove_dir_all(&dir);
    }
}
