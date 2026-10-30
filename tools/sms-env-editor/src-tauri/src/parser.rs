use crate::types::ConfigData;
use base64::prelude::*;
use std::collections::HashMap;

/// 將 Base64 字串解碼為純文字；若失敗則回退為原字串
pub fn decode_b64_or_raw(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    match BASE64_STANDARD.decode(trimmed) {
        Ok(bytes) => {
            // 優先以 UTF-8 解碼，若失敗嘗試 Big-5，最後回退為 lossy
            if let Ok(s) = String::from_utf8(bytes.clone()) {
                s
            } else {
                let (cow, _, _) = encoding_rs::BIG5.decode(&bytes);
                cow.into_owned()
            }
        }
        Err(_) => trimmed.to_string(),
    }
}

/// 將字串以標準 Base64 進行編碼
pub fn encode_b64(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    BASE64_STANDARD.encode(trimmed.as_bytes())
}

/// 解析 Oracle JDBC Thin URL (例如 jdbc:oracle:thin:@220.1.34.2:1521:HAWEB)
pub fn parse_oracle_url(url: &str) -> (String, String, String) {
    let trimmed = url.trim();
    if let Some(at_idx) = trimmed.find('@') {
        let after_at = &trimmed[at_idx + 1..];
        let parts: Vec<&str> = after_at.split(':').collect();
        match parts.len() {
            3 => (parts[0].to_string(), parts[1].to_string(), parts[2].to_string()),
            2 => (parts[0].to_string(), "1521".to_string(), parts[1].to_string()),
            1 => (parts[0].to_string(), "1521".to_string(), String::new()),
            _ => (after_at.to_string(), "1521".to_string(), String::new()),
        }
    } else {
        (trimmed.to_string(), "1521".to_string(), String::new())
    }
}

/// 組裝 Oracle JDBC Thin URL
pub fn format_oracle_url(ip: &str, port: &str, sid: &str) -> String {
    let p = if port.trim().is_empty() { "1521" } else { port.trim() };
    format!("jdbc:oracle:thin:@{}:{}:{}", ip.trim(), p, sid.trim())
}

/// 從二進位位元組解析 env.properties 內容（自動相容 Big-5 與 UTF-8）
pub fn parse_env_properties(bytes: &[u8]) -> ConfigData {
    // 使用 Big-5 解碼
    let (text, _, _) = encoding_rs::BIG5.decode(bytes);
    let mut config = ConfigData {
        section_header: "[LAND]".to_string(),
        odriver_class: "oracle.jdbc.driver.OracleDriver".to_string(),
        d_user: ": ;%4>".to_string(),
        d_pwd: ": ;%4>".to_string(),
        ap_user: "GM".to_string(),
        ap_pwd: "GM".to_string(),
        db_port: "1521".to_string(),
        search_time: "1".to_string(),
        resend_time: "3".to_string(),
        start_hours: "08".to_string(),
        start_min: "30".to_string(),
        end_hours: "18".to_string(),
        end_min: "30".to_string(),
        ..Default::default()
    };

    let mut raw_map: HashMap<String, String> = HashMap::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            config.section_header = trimmed.to_string();
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim().to_string();
            let val = v.trim().to_string();
            raw_map.insert(key, val);
        } else {
            config.extra_lines.push(trimmed.to_string());
        }
    }

    if let Some(v) = raw_map.remove("odriver_class") {
        config.odriver_class = v;
    }
    if let Some(v) = raw_map.remove("oUrl") {
        let raw_o_url = decode_b64_or_raw(&v);
        let (ip, port, sid) = parse_oracle_url(&raw_o_url);
        config.db_ip = ip;
        config.db_port = port;
        config.db_sid = sid;
    }
    if let Some(v) = raw_map.remove("oUser") {
        config.db_user = decode_b64_or_raw(&v);
    }
    if let Some(v) = raw_map.remove("oPwd") {
        config.db_pwd = decode_b64_or_raw(&v);
    }
    if let Some(v) = raw_map.remove("dUser") {
        config.d_user = v;
    }
    if let Some(v) = raw_map.remove("dPwd") {
        config.d_pwd = v;
    }
    if let Some(v) = raw_map.remove("apUser") {
        config.ap_user = decode_b64_or_raw(&v);
    }
    if let Some(v) = raw_map.remove("apPwd") {
        config.ap_pwd = decode_b64_or_raw(&v);
    }
    if let Some(v) = raw_map.remove("ap_url") {
        config.ap_url = decode_b64_or_raw(&v);
    }
    if let Some(v) = raw_map.remove("smsUser") {
        config.sms_user = decode_b64_or_raw(&v);
    }
    if let Some(v) = raw_map.remove("smsPwd") {
        config.sms_pwd = decode_b64_or_raw(&v);
    }
    if let Some(v) = raw_map.remove("user") {
        config.mail_user = decode_b64_or_raw(&v);
    }
    if let Some(v) = raw_map.remove("pwd") {
        config.mail_pwd = decode_b64_or_raw(&v);
    }
    if let Some(v) = raw_map.remove("username") {
        config.mail_sender_name = v;
    }
    if let Some(v) = raw_map.remove("mailaddr") {
        config.mail_sender_addr = v;
    }
    if let Some(v) = raw_map.remove("search_time") {
        config.search_time = v;
    }
    if let Some(v) = raw_map.remove("resend_time") {
        config.resend_time = v;
    }
    if let Some(v) = raw_map.remove("start_hours") {
        config.start_hours = v;
    }
    if let Some(v) = raw_map.remove("start_min") {
        config.start_min = v;
    }
    if let Some(v) = raw_map.remove("end_hours") {
        config.end_hours = v;
    }
    if let Some(v) = raw_map.remove("end_min") {
        config.end_min = v;
    }

    // 保留其餘未列出的鍵值
    for (k, v) in raw_map {
        config.extra_lines.push(format!("{}={}", k, v));
    }

    config
}

/// 將 ConfigData 序列化回 env.properties 格式，並以 Big-5 編碼產出位元組陣列
pub fn serialize_env_properties(data: &ConfigData) -> Vec<u8> {
    let mut lines = Vec::new();

    // 區段名稱
    lines.push(if data.section_header.is_empty() {
        "[LAND]".to_string()
    } else {
        data.section_header.clone()
    });

    // Oracle 設定
    lines.push(format!("odriver_class={}", data.odriver_class));
    let raw_oracle_url = format_oracle_url(&data.db_ip, &data.db_port, &data.db_sid);
    lines.push(format!("oUrl={}", encode_b64(&raw_oracle_url)));
    lines.push(format!("oUser={}", encode_b64(&data.db_user)));
    lines.push(format!("oPwd={}", encode_b64(&data.db_pwd)));

    // dUser / dPwd
    lines.push(format!("dUser={}", data.d_user));
    lines.push(format!("dPwd={}", data.d_pwd));

    // apUser / apPwd
    lines.push(format!("apUser={}", encode_b64(&data.ap_user)));
    lines.push(format!("apPwd={}", encode_b64(&data.ap_pwd)));

    // 簡訊平臺設定
    lines.push(format!("ap_url={}", encode_b64(&data.ap_url)));
    lines.push(format!("smsUser={}", encode_b64(&data.sms_user)));
    lines.push(format!("smsPwd={}", encode_b64(&data.sms_pwd)));

    // 郵件寄件設定
    lines.push(format!("user={}", encode_b64(&data.mail_user)));
    lines.push(format!("pwd={}", encode_b64(&data.mail_pwd)));
    lines.push(format!("username={}", data.mail_sender_name));
    lines.push(format!("mailaddr={}", data.mail_sender_addr));

    // 排程參數
    lines.push(format!("search_time={}", data.search_time));
    lines.push(format!("resend_time={}", data.resend_time));
    lines.push(format!("start_hours={}", data.start_hours));
    lines.push(format!("start_min={}", data.start_min));
    lines.push(format!("end_hours={}", data.end_hours));
    lines.push(format!("end_min={}", data.end_min));

    // 其他額外行
    for extra in &data.extra_lines {
        lines.push(extra.clone());
    }

    let full_content = lines.join("\r\n") + "\r\n";
    let (encoded_bytes, _, _) = encoding_rs::BIG5.encode(&full_content);
    encoded_bytes.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip_parse_and_serialize() {
        let sample = r#"[LAND]
odriver_class=oracle.jdbc.driver.OracleDriver
oUrl=amRiYzpvcmFjbGU6dGhpbjpAMjIwLjEuMzQuMjoxNTIxOkhBV0VC
oUser=TU9JQURN
oPwd=TU9JQURN
dUser=: ;%4>
dPwd=: ;%4>
apUser=R00=
apPwd=R00=
ap_url=aHR0cDovLzIyMC4xLjMzLjEyMjo4MDgwL0lNU1AvT05FU2VydmljZVNlcnZsZXQ=
smsUser=dGFsYW5kQVBJVTAx
smsPwd=dGFsYW5kUFdVMDE=
user=aGFpbmZv
pwd=ZXhsLS0yMDI2MDA0
username=桃園地政事務所
mailaddr=hainfo@mail.tyland.gov.tw
search_time=1
resend_time=3
start_hours=08
start_min=30
end_hours=18
end_min=30
"#;
        let (bytes, _, _) = encoding_rs::BIG5.encode(sample);
        let parsed = parse_env_properties(&bytes);

        assert_eq!(parsed.ap_url, "http://220.1.33.122:8080/IMSP/ONEServiceServlet");
        assert_eq!(parsed.sms_user, "talandAPIU01");
        assert_eq!(parsed.sms_pwd, "talandPWU01");
        assert_eq!(parsed.mail_user, "hainfo");
        assert_eq!(parsed.mail_pwd, "exl--2026004");
        assert_eq!(parsed.mail_sender_name, "桃園地政事務所");
        assert_eq!(parsed.mail_sender_addr, "hainfo@mail.tyland.gov.tw");
        assert_eq!(parsed.db_ip, "220.1.34.2");
        assert_eq!(parsed.db_port, "1521");
        assert_eq!(parsed.db_sid, "HAWEB");
        assert_eq!(parsed.db_user, "MOIADM");
        assert_eq!(parsed.db_pwd, "MOIADM");

        let serialized = serialize_env_properties(&parsed);
        let re_parsed = parse_env_properties(&serialized);
        assert_eq!(re_parsed.mail_sender_name, "桃園地政事務所");
        assert_eq!(re_parsed.ap_url, parsed.ap_url);
    }
}
