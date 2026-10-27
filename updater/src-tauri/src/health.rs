use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use crate::pm2::resolve_pm2_command;
use crate::system::run_cmd_hidden;

pub fn is_port_listening(port: u16, host: &str, timeout_ms: u64) -> bool {
    let addr_str = format!("{}:{}", host, port);
    if let Ok(addr) = addr_str.parse::<SocketAddr>() {
        return TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)).is_ok();
    }
    false
}

pub fn wait_for_port(port: u16, max_secs: f32, host: &str) -> bool {
    let interval = 400;
    let iterations = (max_secs * 1000.0 / interval as f32) as u32;
    for _ in 0..iterations {
        if is_port_listening(port, host, 350) {
            return true;
        }
        sleep(Duration::from_millis(interval));
    }
    false
}

pub fn verify_all_services_health(root_dir: &Path, apache_dir: Option<&Path>, log_fn: &dyn Fn(&str, &str)) -> (bool, Vec<String>) {
    log_fn("🔍 正在進行全方位系統服務與通訊連接埠健康檢查...", "info");
    let mut reports = Vec::new();
    let mut all_ok = true;

    // 1. Apache Port 80
    let has_apache = apache_dir.map_or(false, |p| p.is_dir()) || root_dir.join("Apache24").is_dir();
    if has_apache {
        if wait_for_port(80, 5.0, "127.0.0.1") {
            reports.push("  • Apache 網頁伺服器 (Port 80): 🟢 正常監聽 (Online)".to_string());
            log_fn("  ✓ Apache 網頁伺服器 (Port 80) 連線正常！", "success");
        } else {
            all_ok = false;
            reports.push("  • Apache 網頁伺服器 (Port 80): 🔴 連線逾時 (Offline / 待啟動)".to_string());
            log_fn("  ⚠️ Apache (Port 80) 連線失敗，請檢查服務是否已啟動或端口被占用。", "warning");
        }
    }

    // 2. PHP 8.5 FastCGI Port 8085 (lah-api-v85)
    let has_php85 = root_dir.join("php85").join("php-cgi.exe").is_file() && root_dir.join("LAH-API").is_dir();
    if has_php85 {
        let mut port8085_ok = wait_for_port(8085, 3.0, "127.0.0.1");
        if !port8085_ok {
            let eco_path = root_dir.join("LAH-API").join("ecosystem.config.js");
            if eco_path.is_file() {
                if let Some(pm2) = resolve_pm2_command(root_dir) {
                    log_fn("  🔄 偵測到 FastCGI (Port 8085) 尚未監聽，正在透過 PM2 嘗試拉起 lah-api-v85...", "warning");
                    let _ = run_cmd_hidden(&pm2.to_string_lossy(), &["start", &eco_path.to_string_lossy(), "--no-color"]);
                    port8085_ok = wait_for_port(8085, 4.0, "127.0.0.1");
                }
            }
        }

        if port8085_ok {
            reports.push("  • PHP 8.5 FastCGI 核心 (Port 8085): 🟢 正常監聽 (Online)".to_string());
            log_fn("  ✓ PHP 8.5 FastCGI 核心 (Port 8085) 連線正常！", "success");
        } else {
            all_ok = false;
            reports.push("  • PHP 8.5 FastCGI 核心 (Port 8085): 🔴 連線逾時 (Offline / 待啟動)".to_string());
            log_fn("  ⚠️ PHP 8.5 FastCGI (Port 8085) 連線失敗，請檢查 PM2 lah-api-v85 是否正常運作。", "warning");
        }
    }

    // 3. 前端網站 Port 8080 (LAH-NUXTJS)
    let has_nuxt = root_dir.join("LAH-NUXTJS").is_dir();
    if has_nuxt {
        if wait_for_port(8080, 5.0, "127.0.0.1") {
            reports.push("  • 智慧管控前端網站 (Port 8080): 🟢 正常監聽 (Online)".to_string());
            log_fn("  ✓ 前端網站 (Port 8080) 連線正常！", "success");
        } else {
            all_ok = false;
            reports.push("  • 智慧管控前端網站 (Port 8080): 🟡 啟動中或離線 (Port 8080 未就緒)".to_string());
            log_fn("  ℹ️ 前端網站 (Port 8080) 仍在啟動中或離線。", "info");
        }
    }

    // 4. 即時通訊 WebSocket Port 8081
    let has_messenger = root_dir.join("lah-messenger-server").is_dir();
    if has_messenger {
        if wait_for_port(8081, 4.0, "127.0.0.1") {
            reports.push("  • 即時通訊 WebSocket (Port 8081): 🟢 正常監聽 (Online)".to_string());
            log_fn("  ✓ 即時通訊 WebSocket (Port 8081) 連線正常！", "success");
        } else {
            reports.push("  • 即時通訊 WebSocket (Port 8081): 🟡 未就緒或離線".to_string());
        }

        // 5. 即時通訊 HTTP 附件 Port 8082
        if wait_for_port(8082, 3.0, "127.0.0.1") {
            reports.push("  • 即時通訊 HTTP 附件服務 (Port 8082): 🟢 正常監聽 (Online)".to_string());
            log_fn("  ✓ 即時通訊 HTTP 附件服務 (Port 8082) 連線正常！", "success");
        } else {
            reports.push("  • 即時通訊 HTTP 附件服務 (Port 8082): 🟡 未就緒或離線".to_string());
        }
    }

    (all_ok, reports)
}
