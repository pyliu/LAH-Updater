use crate::system::{run_cmd_hidden, run_powershell};
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

pub fn uninstall_apache_service(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    let svc_name = query_apache_service().unwrap_or_else(|| "Apache2.4".to_string());
    log_fn(&format!("⏳ 正在停止並解除安裝 Apache 服務 [{}]...", svc_name), "info");

    // 1. 嘗試停止服務
    let _ = run_cmd_hidden("sc.exe", &["stop", &svc_name]);
    sleep(Duration::from_millis(800));

    let mut uninstalled = false;

    // 2. 嘗試透過 httpd.exe -k uninstall 卸載
    let httpd_exe = root_dir.join("Apache24").join("bin").join("httpd.exe");
    if httpd_exe.is_file() {
        let httpd_str = httpd_exe.to_string_lossy();
        let (ok, _, _) = run_cmd_hidden(&httpd_str, &["-k", "uninstall", "-n", &svc_name]);
        if ok {
            uninstalled = true;
            log_fn(&format!("  ✓ 已透過 httpd.exe 解除安裝 Apache 服務 [{}]", svc_name), "success");
        }
    }

    // 3. 備援透過 sc.exe delete 徹底清除 SCM 註冊
    let (ok_del, _, _) = run_cmd_hidden("sc.exe", &["delete", &svc_name]);
    if ok_del {
        uninstalled = true;
        log_fn(&format!("  ✓ 已自 Windows SCM 刪除 Apache 服務 [{}]", svc_name), "success");
    }

    if uninstalled {
        log_fn(&format!("  ✓ Apache 服務 [{}] 已成功解除安裝", svc_name), "success");
    } else {
        log_fn("  ℹ️ 未偵測到運作中之 Apache 服務或先前已卸載", "info");
    }

    true
}

pub fn uninstall_lah_pm2_service(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    log_fn("⏳ 正在停止並解除安裝 Windows 原生系統服務 (LAH_PM2_Service)...", "info");
    let exe_path = root_dir.join("node").join("service").join("lah-service.exe");

    // 1. 嘗試 WinSW 卸載
    if exe_path.is_file() {
        let exe_str = exe_path.to_string_lossy();
        let _ = run_cmd_hidden(&exe_str, &["stop"]);
        let (ok_un, stdout, _) = run_cmd_hidden(&exe_str, &["uninstall"]);
        if ok_un || stdout.to_lowercase().contains("successfully") {
            log_fn("  ✓ 已透過 WinSW 成功卸載系統服務: LAH_PM2_Service", "success");
        }
    }

    // 2. 備援透過 sc.exe 刪除
    let _ = run_cmd_hidden("sc.exe", &["stop", "LAH_PM2_Service"]);
    sleep(Duration::from_millis(800));
    let (ok_sc, _, _) = run_cmd_hidden("sc.exe", &["delete", "LAH_PM2_Service"]);
    if ok_sc {
        log_fn("  ✓ 已自 Windows SCM 刪除系統服務: LAH_PM2_Service", "success");
    }

    true
}
