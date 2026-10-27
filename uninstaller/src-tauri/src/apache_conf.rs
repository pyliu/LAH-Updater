use crate::system::run_cmd_hidden;
use chrono::Local;
use regex::Regex;
use std::fs;
use std::path::Path;

pub fn clean_apache_php_configuration(
    root_dir: &Path,
    remove_php7: bool,
    remove_php85: bool,
    log_fn: &dyn Fn(&str, &str),
) -> bool {
    let conf_path = root_dir.join("Apache24").join("conf").join("httpd.conf");
    if !conf_path.is_file() {
        return true;
    }

    let orig_content = match fs::read_to_string(&conf_path) {
        Ok(c) => c,
        Err(e) => {
            log_fn(&format!("  ⚠️ 讀取 httpd.conf 失敗: {e}"), "warning");
            return false;
        }
    };

    let mut content = orig_content.clone();

    // 1. 清理 PHP 8.5 FastCGI 與 Alias 轉發區塊
    if remove_php85 {
        if let Ok(re1) = Regex::new(r"(?s)# =====================================================================\s*# LAH-API \(PHP 8\.5 FastCGI.*?</Proxy>") {
            content = re1.replace_all(&content, "").to_string();
        }
        if let Ok(re2) = Regex::new(r"(?s)# --- LAH-API PHP 8\.5 智慧無感分流規則 ---.*?</IfModule>") {
            content = re2.replace_all(&content, "").to_string();
        }
        log_fn("  ✓ 已自 Apache httpd.conf 清理 LAH-API (PHP 8.5) FastCGI 轉發規則", "info");
    }

    // 2. 清理 PHP 7 模組載入規則（註解 LoadModule 與 PHPIniDir）
    if remove_php7 {
        if let Ok(re_load) = Regex::new(r"(?m)^(\s*LoadModule\s+php7_module\s+.*)$") {
            content = re_load.replace_all(&content, "# [自動移除工具已註銷] $1").to_string();
        }
        if let Ok(re_ini) = Regex::new(r"(?m)^(\s*PHPIniDir\s+.*)$") {
            content = re_ini.replace_all(&content, "# [自動移除工具已註銷] $1").to_string();
        }
        log_fn("  ✓ 已自 Apache httpd.conf 註銷 PHP 7 模組 (LoadModule php7_module)", "info");
    }

    if content != orig_content {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let bak_path = conf_path.with_extension(format!("conf.bak_{timestamp}"));
        let _ = fs::copy(&conf_path, &bak_path);

        if let Err(e) = fs::write(&conf_path, &content) {
            log_fn(&format!("  ⚠️ 寫入更新 httpd.conf 失敗: {e}"), "warning");
            return false;
        }
        log_fn("  ✓ Apache httpd.conf 設定檔已安全更新與備份！", "success");

        // 檢驗 Apache 語法
        let httpd_exe = root_dir.join("Apache24").join("bin").join("httpd.exe");
        if httpd_exe.is_file() {
            let httpd_str = httpd_exe.to_string_lossy();
            let (ok, stdout, stderr) = run_cmd_hidden(&httpd_str, &["-t"]);
            let merged = format!("{stdout} {stderr}");
            if ok || merged.contains("Syntax OK") {
                log_fn("  ✓ Apache 設定檔語法檢驗通過 (Syntax OK)！", "success");
            } else {
                log_fn(&format!("  ⚠️ Apache 語法檢驗回傳警示: {}", merged.trim()), "warning");
            }
        }
    }

    true
}
