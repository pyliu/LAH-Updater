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

pub fn audit_and_ensure_pm2_services(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    let logs_dir = root_dir.join("logs");
    let _ = fs::create_dir_all(&logs_dir);

    let Some(pm2) = resolve_pm2_command(root_dir) else {
        log_fn("⚠️ 系統未偵測到有效的 pm2.cmd，跳過 PM2 服務巡檢。", "warning");
        return (false, "未偵測到 pm2.cmd".to_string());
    };
    let pm2_str = pm2.to_string_lossy().to_string();

    log_fn("--------------------------------------------------", "info");
    log_fn("🔍 正在執行 PM2 核心服務全方位狀態巡檢...", "info");

    // 1. 取得現有 PM2 程序清單
    let mut managed_apps: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let (ok_jlist, out_jlist, _) = run_cmd_hidden(&pm2_str, &["jlist"]);
    if ok_jlist {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&out_jlist) {
            if let Some(arr) = parsed.as_array() {
                for item in arr {
                    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        let status = item.get("pm2_env")
                            .and_then(|e| e.get("status"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        if !managed_apps.contains_key(name) || status == "online" {
                            managed_apps.insert(name.to_string(), status.to_string());
                        }
                    }
                }
            }
        }
    }

    // 2. 定義 3 大生態系設定檔與所屬服務
    struct EcosystemDef {
        title: &'static str,
        apps: &'static [&'static str],
        candidates: Vec<PathBuf>,
    }

    let core_ecosystems = [
        EcosystemDef {
            title: "即時通訊伺服器 (LAH-WSS)",
            apps: &["LAH-WSS"],
            candidates: vec![
                root_dir.join("lah-messenger-server").join("ecosystem.config.js"),
            ],
        },
        EcosystemDef {
            title: "智慧管控前端網站 (LAH-FE / LAH-SCHEDULER)",
            apps: &["LAH-FE", "LAH-SCHEDULER"],
            candidates: vec![
                root_dir.join("LAH-NUXTJS").join("ecosystem.config.js"),
                root_dir.join("LAH-FE").join("ecosystem.config.js"),
            ],
        },
        EcosystemDef {
            title: "現代化後端 API (lah-api-v85)",
            apps: &["lah-api-v85"],
            candidates: vec![
                root_dir.join("LAH-API").join("ecosystem.config.js"),
                root_dir.join("LAH-API").join("API").join("ecosystem.config.js"),
            ],
        },
    ];

    let mut need_save = false;

    for eco in &core_ecosystems {
        let eco_path_opt = eco.candidates.iter().find(|p| p.is_file());
        let Some(eco_path) = eco_path_opt else { continue; };

        let missing_apps: Vec<&&str> = eco.apps.iter().filter(|a| {
            managed_apps.get(**a).map_or(true, |st| st != "online")
        }).collect();

        if !missing_apps.is_empty() {
            let names = missing_apps.iter().map(|s| **s).collect::<Vec<&str>>().join(", ");
            log_fn(&format!("⚡ 偵測到 [{}] 之服務 [{}] 尚未在線，正在啟動 [{}]...", eco.title, names, eco_path.display()), "warning");

            let eco_str = eco_path.to_string_lossy().to_string();
            let (start_ok, stdout, stderr) = run_cmd_hidden(&pm2_str, &["start", &eco_str, "--no-color"]);
            if start_ok || stdout.contains("✓") {
                log_fn(&format!("  ✅ 成功啟動 [{}]！", eco.title), "success");
                need_save = true;
            } else {
                log_fn(&format!("  ⚠️ 啟動 [{}] 回傳警示: {} {}", eco_path.display(), stdout, stderr), "warning");
            }
        }
    }

    // 2.1 檢查並確保日誌輪替模組 (pm2-logrotate) 運作
    if managed_apps.get("pm2-logrotate").map_or(true, |st| st != "online") {
        log_fn("⚡ 偵測到日誌輪替模組 [pm2-logrotate] 尚未在線，正在嘗試啟動...", "warning");
        let (restart_ok, _, _) = run_cmd_hidden(&pm2_str, &["restart", "pm2-logrotate", "--no-color"]);
        if restart_ok {
            log_fn("  ✅ 成功啟動 [pm2-logrotate] 模組！", "success");
            need_save = true;
        } else {
            let local_mod = root_dir.join("node").join("node_modules").join("pm2-logrotate");
            let target_pkg = if local_mod.is_dir() {
                local_mod.to_string_lossy().to_string()
            } else {
                "pm2-logrotate".to_string()
            };
            let (inst_ok, stdout, stderr) = run_cmd_hidden(&pm2_str, &["install", &target_pkg, "--no-color"]);
            if inst_ok {
                log_fn("  ✅ 成功註冊並啟動 [pm2-logrotate] 模組！", "success");
                let _ = run_cmd_hidden(&pm2_str, &["set", "pm2-logrotate:compress", "true"]);
                need_save = true;
            } else {
                log_fn(&format!("  ⚠️ 啟動 [pm2-logrotate] 模組警示: {} {}", stdout, stderr), "warning");
            }
        }
    }

    // 3. 若有補啟動任何項目，自動持久化保存狀態
    if need_save {
        log_fn("💾 偵測到服務配置異動，正在執行 [pm2 save] 更新持久化存檔...", "info");
        let _ = run_cmd_hidden(&pm2_str, &["save", "--force"]);
    }

    // 4. 重新查詢一次最新狀態印出摘要報告
    let mut final_apps: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let (ok_final, out_final, _) = run_cmd_hidden(&pm2_str, &["jlist"]);
    if ok_final {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&out_final) {
            if let Some(arr) = parsed.as_array() {
                for item in arr {
                    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        let status = item.get("pm2_env")
                            .and_then(|e| e.get("status"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        if !final_apps.contains_key(name) || status == "online" {
                            final_apps.insert(name.to_string(), status.to_string());
                        }
                    }
                }
            }
        }
    }

    log_fn("📋 PM2 核心服務現況總覽：", "info");
    let mut all_healthy = true;
    for app_name in &["LAH-WSS", "LAH-FE", "LAH-SCHEDULER", "lah-api-v85", "pm2-logrotate"] {
        let st = final_apps.get(*app_name).map(|s| s.as_str()).unwrap_or("未註冊 / 離線");
        if st == "online" {
            log_fn(&format!("  • {:<16} : ✅ {st}", app_name), "success");
        } else {
            all_healthy = false;
            log_fn(&format!("  • {:<16} : ⚠️ {st}", app_name), "warning");
        }
    }

    if all_healthy {
        log_fn("🎉 全部 5 大核心服務與模組皆已成功在線運作！", "success");
    } else {
        log_fn("⚠️ 部分核心服務未能順利在線，請檢視上方日誌輸出或終端機 pm2 status。", "warning");
    }

    (all_healthy, "PM2 服務巡檢完成".to_string())
}
