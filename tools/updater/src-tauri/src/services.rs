use crate::system::{run_cmd_hidden, run_powershell};
use std::fs::{self, OpenOptions};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

pub const LAH_WEB_SERVICE_NAME: &str = "LAH_Web_Service";
const LAH_SERVICE_EXE: &[u8] = include_bytes!("../../service/lah-service.exe");
const LAH_WEB_SERVICE_XML: &[u8] = include_bytes!("../../service/lah-web-service.xml");

pub fn query_apache_service() -> Option<String> {
    let candidates = [
        LAH_WEB_SERVICE_NAME,
        "Apache2.4",
        "Apache24",
        "Apache2.2",
        "Apache2",
        "Apache",
        "httpd",
    ];
    for name in candidates {
        let (ok, _, _) = run_cmd_hidden("sc.exe", &["query", name]);
        if ok {
            return Some(name.to_string());
        }
    }

    let ps_script = "Get-Service LAH_Web_Service, *web*, *apache* -ErrorAction SilentlyContinue | Where-Object { $_.Name -match '(?i)web|apache' } | Select-Object -ExpandProperty Name -First 1";
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

pub fn is_service_installed(svc_name: &str) -> bool {
    let (ok, _, _) = run_cmd_hidden("sc.exe", &["query", svc_name]);
    if ok {
        return true;
    }
    let ps = format!("(Get-Service '{}' -ErrorAction SilentlyContinue) -ne $null", svc_name);
    let (ok_ps, stdout_ps, _) = run_powershell(&ps);
    ok_ps && stdout_ps.trim().eq_ignore_ascii_case("true")
}

pub fn is_pm2_service_installed() -> bool {
    is_service_installed("LAH_PM2_Service")
}

pub fn is_pm2_service_running() -> bool {
    is_service_running("LAH_PM2_Service")
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

    let root_dir = apache_dir.parent().unwrap_or(apache_dir);

    // 1. 若本機已註冊舊版服務 (如 Apache2.4、Apache24、Apache2 等)，先予以停止並自 SCM 卸載以無縫升級為沙盒服務
    let legacy_candidates = ["Apache2.4", "Apache24", "Apache2.2", "Apache2", "Apache", "httpd"];
    for legacy in legacy_candidates {
        if legacy != svc_name && is_service_installed(legacy) {
            log_fn(
                &format!("⏳ 偵測到舊版 Apache 服務 [{legacy}]，正在停止並解除安裝以無縫升級為 [{svc_name}]..."),
                "info",
            );
            let _ = stop_service(legacy, log_fn);
            let httpd_str = httpd_exe.to_string_lossy();
            let _ = run_cmd_hidden(&httpd_str, &["-k", "uninstall", "-n", legacy]);
            let _ = run_cmd_hidden("sc.exe", &["delete", legacy]);
            log_fn(&format!("  ✓ 已自 Windows SCM 清理舊版 Apache 服務 [{legacy}]"), "success");
        }
    }

    // 2. 釋出專屬 WinSW 服務檔案至 <apache_dir>\service\
    let service_dir = apache_dir.join("service");
    if let Err(e) = fs::create_dir_all(&service_dir) {
        log_fn(&format!("⚠️ 建立 service 目錄失敗: {e}"), "warning");
        return false;
    }

    let winsw_exe = service_dir.join("lah-web-service.exe");
    if !winsw_exe.is_file() || fs::metadata(&winsw_exe).map_or(0, |m| m.len()) == 0 {
        if let Err(e) = fs::write(&winsw_exe, LAH_SERVICE_EXE) {
            log_fn(&format!("⚠️ 釋出 lah-web-service.exe 失敗: {e}"), "warning");
            return false;
        }
        log_fn("  ✓ 已釋出專屬 WinSW 服務核心 (lah-web-service.exe)", "success");
    }

    let xml_path = service_dir.join("lah-web-service.xml");
    let xml_raw = String::from_utf8_lossy(LAH_WEB_SERVICE_XML).to_string();
    let root_str = root_dir.to_string_lossy().to_string();
    let apache_str = apache_dir.to_string_lossy().to_string();
    let customized_xml = xml_raw
        .replace("C:\\AppServ\\Apache24", &apache_str)
        .replace("C:\\AppServ", &root_str);
    if let Err(e) = fs::write(&xml_path, customized_xml) {
        log_fn(&format!("⚠️ 釋出 lah-web-service.xml 失敗: {e}"), "warning");
        return false;
    }

    let winsw_str = winsw_exe.to_string_lossy().to_string();

    // 3. 若目標服務已存在，先停止並重新註冊以更新設定
    if is_service_installed(svc_name) {
        let _ = stop_service(svc_name, log_fn);
        let _ = run_cmd_hidden(&winsw_str, &["uninstall"]);
        let _ = run_cmd_hidden("sc.exe", &["delete", svc_name]);
        sleep(Duration::from_millis(500));
    }

    log_fn(&format!("🔧 正在將 Web 伺服器註冊為 Windows 沙盒服務 [{svc_name}]..."), "info");
    let (ok_inst, stdout_inst, stderr_inst) = run_cmd_hidden(&winsw_str, &["install"]);

    if ok_inst || stdout_inst.contains("successfully") || stdout_inst.contains("already exists") {
        log_fn(&format!("  ✓ Web 系統服務 [{svc_name}] 註冊成功 (WinSW 沙盒守護)！"), "success");
    } else {
        log_fn(&format!("  ⚠️ 註冊 Web 服務回傳警告: {stdout_inst} {stderr_inst}"), "warning");
    }

    // 4. 啟動服務
    let (ok_start, stdout_start, stderr_start) = run_cmd_hidden(&winsw_str, &["start"]);
    if ok_start || stdout_start.contains("Started") {
        log_fn(&format!("  ✓ Web 系統服務 [{svc_name}] 已成功啟動運行！"), "success");
        true
    } else {
        log_fn(&format!("  ⚠️ 啟動 Web 服務回傳: {stdout_start} {stderr_start}"), "warning");
        start_service(svc_name, log_fn)
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
