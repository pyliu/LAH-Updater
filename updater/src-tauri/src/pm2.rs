use crate::system::{run_cmd_hidden, run_powershell};
use std::fs;
use std::path::{Path, PathBuf};

const LAH_SERVICE_EXE: &[u8] = include_bytes!("../../service/lah-service.exe");
const LAH_SERVICE_XML: &[u8] = include_bytes!("../../service/lah-service.xml");
const SERVICE_RUNNER_JS: &[u8] = include_bytes!("../../service/service_runner.js");

pub fn resolve_pm2_command(root_dir: &Path) -> Option<PathBuf> {
    let cand1 = root_dir.join("node").join("pm2.cmd");
    if cand1.is_file() {
        return Some(cand1);
    }
    let cand2 = root_dir.join("node").join("node_modules").join("pm2").join("bin").join("pm2");
    if cand2.is_file() {
        return Some(cand2);
    }
    None
}

pub fn ensure_portable_pm2_patched(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    let node_dir = root_dir.join("node");
    if !node_dir.is_dir() {
        return false;
    }

    log_fn("🟢 正在配置綠色版 Node.js 與 PM2 執行期環境...", "info");

    // 確保 node/node_modules/pm2 存在
    let pm2_cmd = node_dir.join("pm2.cmd");
    if !pm2_cmd.is_file() {
        // 若缺少 pm2.cmd，建立一個轉發至 node.exe + pm2 bin 的批次檔
        let pm2_js = node_dir.join("node_modules").join("pm2").join("bin").join("pm2");
        if pm2_js.is_file() {
            let cmd_content = format!(
                "@ECHO OFF\r\nSETLOCAL\r\nSET PATH=\"{}\";%PATH%\r\n\"{}\" \"{}\" %*\r\n",
                node_dir.display(),
                node_dir.join("node.exe").display(),
                pm2_js.display()
            );
            let _ = fs::write(&pm2_cmd, cmd_content);
            log_fn("  ✓ 已建立可攜式 pm2.cmd 執行捷徑", "success");
        }
    }

    true
}

pub fn kill_pm2_daemon(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    log_fn("🛑 正在執行 pm2 kill 並釋放 Node.js 行程鎖定...", "info");
    if let Some(pm2) = resolve_pm2_command(root_dir) {
        let (ok, stdout, stderr) = run_cmd_hidden(&pm2.to_string_lossy(), &["kill"]);
        if ok || stdout.contains("daemon stopped") {
            log_fn("  ✓ PM2 守護進程已終止", "success");
        } else {
            log_fn(&format!("  ℹ️ PM2 kill 輸出: {} {}", stdout, stderr), "info");
        }
    }

    // 檢查是否有殘留以該 node 運行的行程
    let node_dir_str = root_dir.join("node").to_string_lossy().to_lowercase();
    let ps_kill = format!(
        "Get-CimInstance Win32_Process | Where-Object {{ ($_.Name -like 'node*.exe' -or $_.Name -like 'php-cgi*.exe') -and $_.ExecutablePath -and $_.ExecutablePath.ToLower().StartsWith('{}') }} | ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }}",
        node_dir_str.replace('\\', "\\\\")
    );
    let (ok_ps, _, _) = run_powershell(&ps_kill);
    if ok_ps {
        log_fn("  ✓ 本機相關 Node 與 FastCGI 殘留進程已清理完畢", "success");
    }

    (true, "PM2 與背景守護行程已終止".to_string())
}

pub fn resurrect_pm2_services(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    if let Some(pm2) = resolve_pm2_command(root_dir) {
        log_fn("🚀 正在執行 pm2 resurrect 喚醒已註冊之受管服務...", "info");
        let (ok, stdout, stderr) = run_cmd_hidden(&pm2.to_string_lossy(), &["resurrect"]);
        if ok || stdout.contains("Restoring") {
            log_fn("  ✓ PM2 服務已成功喚醒！", "success");
            return (true, "PM2 已喚醒".to_string());
        } else {
            log_fn(&format!("  ⚠️ pm2 resurrect 回傳: {} {}", stdout, stderr), "warning");
        }
    }
    (false, "無法執行 pm2 resurrect".to_string())
}

pub fn restart_pm2_services(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    if let Some(pm2) = resolve_pm2_command(root_dir) {
        log_fn("🔄 正在執行 pm2 restart all 重啟所有受管服務...", "info");
        let (ok, stdout, stderr) = run_cmd_hidden(&pm2.to_string_lossy(), &["restart", "all"]);
        if ok {
            log_fn("  ✓ PM2 服務已全部重新啟動！", "success");
            return (true, "已重啟全部 PM2 服務".to_string());
        } else {
            log_fn(&format!("  ⚠️ pm2 restart all 警示: {} {}", stdout, stderr), "warning");
        }
    }
    (false, "無法執行 pm2 restart all".to_string())
}

pub fn query_pm2_daemon_info(root_dir: &Path) -> (bool, String) {
    if let Some(pm2) = resolve_pm2_command(root_dir) {
        let (ok, stdout, _) = run_cmd_hidden(&pm2.to_string_lossy(), &["ping"]);
        if ok && (stdout.contains("pong") || stdout.contains("PM2 Successfully")) {
            return (true, "PM2 守護程序運行中 (Active)".to_string());
        }
    }
    (false, "PM2 未運行或離線".to_string())
}

pub fn ensure_winsw_service_files(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> Result<PathBuf, String> {
    let service_dir = root_dir.join("node").join("service");
    fs::create_dir_all(&service_dir).map_err(|e| format!("建立 service 目錄失敗: {}", e))?;

    let exe_path = service_dir.join("lah-service.exe");
    if !exe_path.is_file() || fs::metadata(&exe_path).map_or(0, |m| m.len()) == 0 {
        fs::write(&exe_path, LAH_SERVICE_EXE).map_err(|e| format!("釋出 lah-service.exe 失敗: {}", e))?;
        log_fn("  ✓ 已釋出自包含 WinSW 服務核心 (lah-service.exe)", "success");
    }

    let runner_path = service_dir.join("service_runner.js");
    if !runner_path.is_file() {
        fs::write(&runner_path, SERVICE_RUNNER_JS).map_err(|e| format!("釋出 service_runner.js 失敗: {}", e))?;
    }

    let xml_path = service_dir.join("lah-service.xml");
    if !xml_path.is_file() {
        let xml_str = String::from_utf8_lossy(LAH_SERVICE_XML).to_string();
        let root_str = root_dir.to_string_lossy().to_string();
        let customized = xml_str.replace("C:\\AppServ", &root_str);
        fs::write(&xml_path, customized).map_err(|e| format!("釋出 lah-service.xml 失敗: {}", e))?;
    }

    Ok(exe_path)
}

pub fn setup_pm2_autostart(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    log_fn("⚙️ 正在設定 PM2 開機自啟動服務 (LAH_PM2_Service)...", "info");

    let exe_path = match ensure_winsw_service_files(root_dir, log_fn) {
        Ok(p) => p,
        Err(e) => return (false, e),
    };

    let exe_str = exe_path.to_string_lossy();

    // 1. 安裝服務
    let (ok_inst, out_inst, _) = run_cmd_hidden(&exe_str, &["install"]);
    if ok_inst || out_inst.contains("already exists") {
        log_fn("  ✓ LAH_PM2_Service 系統服務註冊完成", "success");
    }

    // 2. 啟動服務
    let (ok_start, out_start, _) = run_cmd_hidden(&exe_str, &["start"]);
    if ok_start || out_start.contains("Started") {
        log_fn("  ✓ LAH_PM2_Service 系統服務已成功啟動", "success");
        return (true, "LAH_PM2_Service 已註冊並啟動".to_string());
    }

    (true, "LAH_PM2_Service 已註冊".to_string())
}
