use crate::system::{run_cmd_hidden, run_powershell};
use std::fs::OpenOptions;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

pub fn query_apache_service() -> Option<String> {
    let candidates = ["Apache2.4", "Apache24", "Apache2.2", "Apache2", "Apache", "httpd"];
    for name in candidates {
        let (ok, _, _) = run_cmd_hidden("sc.exe", &["query", name]);
        if ok {
            return Some(name.to_string());
        }
    }

    let ps_script = "Get-Service *apache* -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name -First 1";
    let (ok, stdout, _) = run_powershell(ps_script);
    let trimmed = stdout.trim();
    if ok && !trimmed.is_empty() {
        return Some(trimmed.to_string());
    }

    None
}

pub fn is_service_running(svc_name: &str) -> bool {
    let (ok, stdout, _) = run_cmd_hidden("sc.exe", &["query", svc_name]);
    if ok && (stdout.contains("RUNNING") || stdout.contains("正在執行")) {
        return true;
    }
    let ps = format!("(Get-Service '{}' -ErrorAction SilentlyContinue).Status", svc_name);
    let (ok_ps, stdout_ps, _) = run_powershell(&ps);
    ok_ps && stdout_ps.trim().eq_ignore_ascii_case("running")
}

pub fn stop_service(svc_name: &str, log_fn: &dyn Fn(&str, &str)) -> bool {
    log_fn(&format!("⏳ 正在停止服務 [{}]...", svc_name), "info");
    let _ = run_cmd_hidden("net.exe", &["stop", svc_name]);

    for _ in 0..15 {
        if !is_service_running(svc_name) {
            log_fn(&format!("  ✓ 服務 [{}] 已成功停止", svc_name), "success");
            return true;
        }
        sleep(Duration::from_millis(500));
    }

    let (ok_sc, _, _) = run_cmd_hidden("sc.exe", &["stop", svc_name]);
    if ok_sc {
        sleep(Duration::from_millis(800));
    }
    !is_service_running(svc_name)
}

pub fn start_service(svc_name: &str, log_fn: &dyn Fn(&str, &str)) -> bool {
    log_fn(&format!("🚀 正在啟動服務 [{}]...", svc_name), "info");
    let (_ok, _, stderr) = run_cmd_hidden("net.exe", &["start", svc_name]);

    for _ in 0..15 {
        if is_service_running(svc_name) {
            log_fn(&format!("  ✓ 服務 [{}] 已成功啟動運行！", svc_name), "success");
            return true;
        }
        sleep(Duration::from_millis(500));
    }

    log_fn(&format!("⚠️ 啟動服務 [{}] 逾時或回傳警告: {}", svc_name, stderr), "warning");
    false
}

pub fn install_apache_service(apache_dir: &Path, svc_name: &str, log_fn: &dyn Fn(&str, &str)) -> bool {
    let httpd_exe = apache_dir.join("bin").join("httpd.exe");
    if !httpd_exe.is_file() {
        log_fn(&format!("⚠️ 找不到 httpd.exe: {}", httpd_exe.display()), "warning");
        return false;
    }

    if let Some(existing) = query_apache_service() {
        log_fn(&format!("ℹ️ 系統已註冊 Apache 服務 [{existing}]，無需重複安裝。"), "info");
        return true;
    }

    log_fn(&format!("🔧 正在將 Apache 註冊為 Windows 系統服務 [{svc_name}]..."), "info");
    let httpd_str = httpd_exe.to_string_lossy();
    let (ok, stdout, stderr) = run_cmd_hidden(&httpd_str, &["-k", "install", "-n", svc_name]);

    if ok || stdout.contains("successfully installed") {
        log_fn(&format!("  ✓ Apache 系統服務 [{svc_name}] 註冊成功！"), "success");
        true
    } else {
        log_fn(&format!("  ⚠️ 註冊 Apache 服務回傳: {} {}", stdout, stderr), "warning");
        false
    }
}

pub fn is_file_locked(filepath: &Path) -> bool {
    if !filepath.exists() {
        return false;
    }
    match OpenOptions::new().write(true).open(filepath) {
        Ok(_) => false,
        Err(e) => {
            // PermissionDenied 或 Windows OS Error 32 (Sharing violation)
            matches!(e.kind(), std::io::ErrorKind::PermissionDenied)
                || e.raw_os_error() == Some(32)
        }
    }
}
