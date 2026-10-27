use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

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

    // 2. 前端網站 Port 8080 (LAH-NUXTJS)
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

    // 3. 即時通訊 WebSocket Port 8081
    let has_messenger = root_dir.join("lah-messenger-server").is_dir();
    if has_messenger {
        if wait_for_port(8081, 4.0, "127.0.0.1") {
            reports.push("  • 即時通訊 WebSocket (Port 8081): 🟢 正常監聽 (Online)".to_string());
            log_fn("  ✓ 即時通訊 WebSocket (Port 8081) 連線正常！", "success");
        } else {
            reports.push("  • 即時通訊 WebSocket (Port 8081): 🟡 未就緒或離線".to_string());
        }

        // 4. 即時通訊 HTTP 附件 Port 8082
        if wait_for_port(8082, 3.0, "127.0.0.1") {
            reports.push("  • 即時通訊 HTTP 附件服務 (Port 8082): 🟢 正常監聽 (Online)".to_string());
            log_fn("  ✓ 即時通訊 HTTP 附件服務 (Port 8082) 連線正常！", "success");
        } else {
            reports.push("  • 即時通訊 HTTP 附件服務 (Port 8082): 🟡 未就緒或離線".to_string());
        }
    }

    (all_ok, reports)
}
