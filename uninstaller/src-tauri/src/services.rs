use crate::system::{run_cmd_hidden, run_powershell};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

pub fn query_apache_service() -> Option<String> {
    let candidates = [
        "LAH_Web_Service",
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

pub fn uninstall_apache_service(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    let svc_name = query_apache_service().unwrap_or_else(|| "LAH_Web_Service".to_string());
    log_fn(&format!("⏳ 正在停止並解除安裝 Web / Apache 服務 [{}]...", svc_name), "info");

    let mut uninstalled = false;

    // 1. 若存在專屬 WinSW 服務 (lah-web-service.exe)，優先透過 WinSW 原生命令 stop & uninstall
    let winsw_exe = root_dir.join("Apache24").join("service").join("lah-web-service.exe");
    if winsw_exe.is_file() {
        let winsw_str = winsw_exe.to_string_lossy();
        let _ = run_cmd_hidden(&winsw_str, &["stop"]);
        let (ok_un, stdout, _) = run_cmd_hidden(&winsw_str, &["uninstall"]);
        if ok_un || stdout.to_lowercase().contains("successfully") {
            uninstalled = true;
            log_fn("  ✓ 已透過 WinSW 成功卸載 Web 沙盒服務 (LAH_Web_Service)", "success");
        }
    }

    // 2. 嘗試停止服務
    let _ = run_cmd_hidden("sc.exe", &["stop", &svc_name]);
    sleep(Duration::from_millis(500));

    // 3. 嘗試透過 httpd.exe -k uninstall 卸載 (相容舊版直接註冊之 Apache2.4)
    let httpd_exe = root_dir.join("Apache24").join("bin").join("httpd.exe");
    if httpd_exe.is_file() {
        let httpd_str = httpd_exe.to_string_lossy();
        let (ok, _, _) = run_cmd_hidden(&httpd_str, &["-k", "uninstall", "-n", &svc_name]);
        if ok {
            uninstalled = true;
            log_fn(&format!("  ✓ 已透過 httpd.exe 解除安裝 Apache 服務 [{}]", svc_name), "success");
        }
    }

    // 4. 備援透過 sc.exe delete 徹底清除 SCM 註冊 (涵蓋 LAH_Web_Service 與舊版候選名)
    for s in &[svc_name.as_str(), "LAH_Web_Service", "Apache2.4", "Apache24"] {
        let (ok_del, _, _) = run_cmd_hidden("sc.exe", &["delete", s]);
        if ok_del {
            uninstalled = true;
            log_fn(&format!("  ✓ 已自 Windows SCM 刪除系統服務 [{s}]"), "success");
        }
    }

    if uninstalled {
        log_fn(&format!("  ✓ Web / Apache 服務 [{}] 已成功解除安裝", svc_name), "success");
    } else {
        log_fn("  ℹ️ 未偵測到運作中之 Web / Apache 服務或先前已卸載", "info");
    }

    true
}

pub fn uninstall_lah_pm2_service(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    log_fn("⏳ 正在停止並解除安裝 PM2 Windows 系統服務 (含 LAH_PM2_Service 與 PM2_Portable_Service)...", "info");

    let service_pairs = [
        ("LAH_PM2_Service", "lah-service.exe"),
        ("PM2_Portable_Service", "pm2-service.exe"),
    ];

    for (svc_name, exe_name) in service_pairs {
        let exe_path = root_dir.join("node").join("service").join(exe_name);

        // 1. 嘗試透過 WinSW 原生命令 stop & uninstall
        if exe_path.is_file() {
            let exe_str = exe_path.to_string_lossy();
            let _ = run_cmd_hidden(&exe_str, &["stop"]);
            let (ok_un, stdout, _) = run_cmd_hidden(&exe_str, &["uninstall"]);
            if ok_un || stdout.to_lowercase().contains("successfully") {
                log_fn(&format!("  ✓ 已透過 WinSW 成功卸載系統服務: {svc_name}"), "success");
            }
        }

        // 2. 備援透過 sc.exe 強制停止並自 SCM 刪除註冊
        let (query_ok, out_query, _) = run_cmd_hidden("sc.exe", &["query", svc_name]);
        if query_ok && !out_query.contains("1060") { // 1060: 指定的服務未安裝
            let _ = run_cmd_hidden("sc.exe", &["stop", svc_name]);
            sleep(Duration::from_millis(500));
            let (ok_sc, _, _) = run_cmd_hidden("sc.exe", &["delete", svc_name]);
            if ok_sc {
                log_fn(&format!("  ✓ 已自 Windows SCM 刪除系統服務: {svc_name}"), "success");
            }
        }
    }

    true
}
