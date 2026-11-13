use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use encoding_rs::BIG5;

/// 將資料庫讀取的 ISO-8859-1（實際存放 Big5 位元組）字串轉換為 UTF-8 字串。
///
/// 對應原 Java `util.Common.isoToBig5(String)`。
pub fn iso_to_big5(input: &str) -> Result<String> {
    if input.is_empty() {
        return Ok(String::new());
    }

    let mut raw_bytes = Vec::with_capacity(input.len());
    for ch in input.chars() {
        let code = u32::from(ch);
        if code > 0xFF {
            bail!("輸入字串包含非 ISO-8859-1 字元: U+{code:04X} ('{ch}')");
        }
        raw_bytes.push(code as u8);
    }

    let (decoded, had_errors) = BIG5.decode_without_bom_handling(&raw_bytes);
    if had_errors {
        bail!("ISO-8859-1 原始位元組無法完整解碼為有效的 Big5 字串");
    }

    Ok(decoded.into_owned())
}

/// 將 UTF-8 字串編碼為 Big5 位元組後，再逐位元組對應為 ISO-8859-1 字串以寫回資料庫。
///
/// 對應原 Java `util.Common.big5ToIso(String)`。
pub fn big5_to_iso(input: &str) -> Result<String> {
    if input.is_empty() {
        return Ok(String::new());
    }

    let (encoded, _, had_errors) = BIG5.encode(input);
    if had_errors {
        bail!("輸入字串包含無法以 Big5 編碼的字元");
    }

    Ok(encoded.iter().map(|&b| char::from(b)).collect())
}

/// 模擬原 Java `util.Common.getFromBASE64(String)`：
/// 忽略空白與換行字元後進行 Base64 解碼，並依序嘗試以 UTF-8 或 Big5 解析為字串。
pub fn decode_base64_legacy(encoded: &str) -> Result<String> {
    let cleaned: String = encoded.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if cleaned.is_empty() {
        return Ok(String::new());
    }

    let bytes = BASE64_STANDARD
        .decode(&cleaned)
        .with_context(|| "Base64 解碼失敗")?;

    match String::from_utf8(bytes) {
        Ok(utf8) => Ok(utf8),
        Err(err) => {
            let raw = err.into_bytes();
            let (decoded, had_errors) = BIG5.decode_without_bom_handling(&raw);
            if had_errors {
                bail!("Base64 解碼後的位元組既非有效 UTF-8 亦非有效 Big5");
            }
            Ok(decoded.into_owned())
        }
    }
}

/// 將 UTF-8 字串進行 Base64 編碼（對應 `SmsMainFrame.SendMsg` 的 `BASE64Encoder().encode(msg.getBytes("UTF-8"))`）。
pub fn encode_base64_utf8(input: &str) -> String {
    BASE64_STANDARD.encode(input.as_bytes())
}

/// 將西元年月日格式化為民國年月日 `YYYMMDD`（對應 `util.Datetime.getYYYMMDD()`，用於 Log 檔名前綴）。
pub fn format_roc_date(year: i32, month: u32, day: u32) -> Result<String> {
    if year <= 1911 {
        bail!("西元年份必須大於 1911，收到: {year}");
    }
    if !(1..=12).contains(&month) {
        bail!("月份必須介於 1..=12，收到: {month}");
    }
    if !(1..=31).contains(&day) {
        bail!("日期必須介於 1..=31，收到: {day}");
    }

    let roc_year = year - 1911;
    Ok(format!("{roc_year:03}{month:02}{day:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iso_to_big5_and_roundtrip() {
        // 「測試」的 Big5 位元組為 B4 FA B8 D5
        let iso_str: String = [0xB4u8, 0xFA, 0xB8, 0xD5]
            .iter()
            .map(|&b| char::from(b))
            .collect();

        let zh = iso_to_big5(&iso_str).expect("應成功將 ISO-8859-1 包裝的 Big5 轉回中文");
        assert_eq!(zh, "測試");

        let back_to_iso = big5_to_iso(&zh).expect("應成功將中文轉回 ISO-8859-1 字串");
        assert_eq!(back_to_iso, iso_str);
    }

    #[test]
    fn test_encoding_empty_strings() {
        assert_eq!(iso_to_big5("").unwrap(), "");
        assert_eq!(big5_to_iso("").unwrap(), "");
        assert_eq!(decode_base64_legacy("").unwrap(), "");
        assert_eq!(decode_base64_legacy("   \r\n\t ").unwrap(), "");
    }

    #[test]
    fn test_iso_to_big5_rejects_non_latin1_and_invalid_big5() {
        // 包含 U+4E2D ('中')，超出 ISO-8859-1 範圍
        let err = iso_to_big5("中文").unwrap_err();
        assert!(err.to_string().contains("非 ISO-8859-1"));

        // 孤立的 Big5 前導位元組 0xB4 沒有後續位元組
        let bad_bytes = String::from(char::from(0xB4u8));
        assert!(iso_to_big5(&bad_bytes).is_err());
    }

    #[test]
    fn test_big5_to_iso_rejects_unmappable_characters() {
        // Emoji 無法以 Big5 表示
        let err = big5_to_iso("測試🚀").unwrap_err();
        assert!(err.to_string().contains("Big5"));
    }

    #[test]
    fn test_decode_base64_legacy_handles_newlines_and_big5() {
        // "jdbc:db2://127.0.0.1:50000/MOI" 含換行（模擬 sun.misc.BASE64Decoder）
        let decoded = decode_base64_legacy("amRiYzpkYjI6Ly8x\r\nMjcuMC4wLjE6NTAwMDAvTU9J\n").unwrap();
        assert_eq!(decoded, "jdbc:db2://127.0.0.1:50000/MOI");

        // Big5 位元組 [0xB4, 0xFA, 0xB8, 0xD5] ("測試") 的 Base64 為 "tPq41Q=="
        let decoded_big5 = decode_base64_legacy("tPq41Q==").unwrap();
        assert_eq!(decoded_big5, "測試");

        // 無效 Base64
        assert!(decode_base64_legacy("@@@invalid@@@").is_err());
    }

    #[test]
    fn test_format_roc_date_and_boundaries() {
        assert_eq!(format_roc_date(2026, 11, 13).unwrap(), "1151113");
        assert_eq!(format_roc_date(1912, 1, 1).unwrap(), "0010101");
        assert!(format_roc_date(1911, 12, 31).is_err());
        assert!(format_roc_date(2026, 0, 10).is_err());
        assert!(format_roc_date(2026, 13, 10).is_err());
        assert!(format_roc_date(2026, 11, 32).is_err());
    }
}
