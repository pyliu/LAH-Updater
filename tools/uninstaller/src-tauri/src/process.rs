use crate::system::{run_cmd_hidden, run_powershell};
use std::path::Path;

pub fn kill_all_pm2_and_node(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    log_fn("⏳ 正在終止 PM2 核心守護進程、FastCGI 與殘留背景程序...", "info");

    // 1. 嘗試呼叫 pm2 kill
    let pm2_cmd = root_dir.join("node").join("pm2.cmd");
    if pm2_cmd.is_file() {
        let pm2_str = pm2_cmd.to_string_lossy();
        let (ok, _, _) = run_cmd_hidden(&pm2_str, &["kill"]);
        if ok {
            log_fn("  ✓ 已執行 pm2 kill 停止所有受控 PM2 背景進程", "info");
        }
    }

    // 2. 透過 PowerShell 強健終止屬於 root_dir 或 PM2 守護相關之進程 (含 node, php-cgi, httpd, pm2)
    let norm_root = root_dir.to_string_lossy().replace('\'', "''").to_lowercase();
    let ps_cmd = format!(
        "Get-CimInstance Win32_Process | Where-Object {{ \
            (($_.Name -like 'node*.exe' -or $_.Name -like 'php-cgi*.exe' -or $_.Name -like 'httpd*.exe') -and $_.ExecutablePath -and $_.ExecutablePath.ToLower().StartsWith('{norm_root}')) \
            -or ($_.Name -like 'node*.exe' -and ($_.CommandLine -like '*Daemon.js*' -or $_.CommandLine -like '*God.js*' -or $_.CommandLine -like '*pm2*service*index.js*' -or $_.CommandLine -like '*pm2_service_runner.js*')) \
            -or ($_.Name -eq 'pm2.exe') \
        }} | ForEach-Object {{ \
            Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue; \
            Write-Output \"Terminated $($_.Name) ($($_.ProcessId))\" \
        }}"
    );

    let (ok, stdout, _) = run_powershell(&ps_cmd);
    if ok && !stdout.trim().is_empty() {
        for line in stdout.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                log_fn(&format!("  ✓ 強制終止殘留進程: {trimmed}"), "info");
            }
        }
    }

    true
}
