use std::env;
use std::path::Path;

use anyhow::{Context, Result};
use moi_sms::config::EnvConfig;
use moi_sms::encoding::{big5_to_iso, encode_base64_utf8, format_roc_date, iso_to_big5};

fn mask_secret(s: &str) -> String {
    if s.is_empty() {
        "<empty>".to_string()
    } else {
        format!("*** (len={})", s.chars().count())
    }
}

fn main() -> Result<()> {
    println!("=== MOI_SMS Rust 核心模組執行觀察 ===");

    // 1. 展示核心資料轉換與編碼自我檢核
    let sample_zh = "地政案件簡訊通知測試";
    let iso_wrapped = big5_to_iso(sample_zh)?;
    let restored_zh = iso_to_big5(&iso_wrapped)?;
    let roc_today = format_roc_date(2026, 11, 13)?;
    println!("[編碼檢核] 原始字串      : {sample_zh}");
    println!("[編碼檢核] Big5->ISO 長度: {} chars", iso_wrapped.chars().count());
    println!("[編碼檢核] ISO->Big5 還原: {restored_zh}");
    println!("[編碼檢核] UTF8->Base64  : {}", encode_base64_utf8(sample_zh));
    println!("[日期檢核] 民國日期前綴  : {roc_today} (對應 {roc_today}moniter_sms.log / {roc_today}sms.log)");
    println!();

    // 2. 載入並檢視 env.properties（支援第一個 CLI 參數指定路徑，預設為 ./env.properties）
    let arg_path = env::args().nth(1).unwrap_or_else(|| "env.properties".to_string());
    let env_path = Path::new(&arg_path);

    if env_path.exists() {
        println!("[設定讀取] 載入檔案: {}", env_path.display());
        let cfg = EnvConfig::from_file(env_path)
            .with_context(|| format!("載入 {} 失敗", env_path.display()))?;

        println!("  ├─ DB Driver   : {}", cfg.db.driver_class);
        println!("  ├─ DB URL      : {}", cfg.db.url);
        println!("  ├─ DB User     : {}", cfg.db.user);
        println!("  ├─ DB Password : {}", mask_secret(&cfg.db.password));

        if let Some(sms) = &cfg.sms_api {
            println!("  ├─ SMS API URL : {}", sms.ap_url);
            println!("  ├─ SMS UserID  : {}", sms.ap_userid);
            println!("  ├─ SMS Pwd     : {}", mask_secret(&sms.ap_pwd));
        }

        if let Some(mail) = &cfg.mail {
            println!("  ├─ SMTP Host   : {} (port: {:?})", mail.smtp_host, mail.smtp_port);
            println!("  ├─ Mail Acct   : {}", mail.account);
            println!("  ├─ Mail From   : {} ({})", mail.from_addr, mail.username);
        }

        println!(
            "  └─ 排程設定    : search_time={:?}, resend_time={:?}, 執行時段={:?}:{:?} ~ {:?}:{:?}",
            cfg.schedule.search_time,
            cfg.schedule.resend_time,
            cfg.schedule.start_hours,
            cfg.schedule.start_min,
            cfg.schedule.end_hours,
            cfg.schedule.end_min,
        );
    } else {
        println!(
            "[設定讀取] 未找到 '{}'（可將 env.properties 放至同目錄，或執行 `moi_sms.exe <env.properties路徑>`）",
            env_path.display()
        );
    }

    Ok(())
}
