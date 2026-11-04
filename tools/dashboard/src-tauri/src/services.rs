use crate::types::SystemServiceStatus;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::mpsc;
use std::thread::{self, sleep};
use std::time::Duration;

pub const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn run_cmd_hidden(cmd: &str, args: &[&str]) -> (bool, String, String) {
    run_cmd_hidden_with_timeout(cmd, args, Duration::from_secs(8))
}

pub fn run_cmd_hidden_with_timeout(cmd: &str, args: &[&str], timeout: Duration) -> (bool, String, String) {
    let (tx, rx) = mpsc::channel();
    let cmd_str = cmd.to_string();
    let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    let _handle = thread::spawn(move || {
        let mut command = Command::new(&cmd_str);
        command.args(&args_vec);
        command.creation_flags(CREATE_NO_WINDOW);
        let res = command.output();
        let _ = tx.send(res);
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            (output.status.success(), stdout, stderr)
        }
        Ok(Err(e)) => (false, String::new(), e.to_string()),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            (
                false,
                String::new(),
                format!("指令 [{} {:?}] 執行逾時 (未在 {} 秒內完成回應)", cmd, args, timeout.as_secs()),
            )
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            (false, String::new(), "執行緒異常中斷".to_string())
        }
    }
}

pub fn get_service_status(svc_name: &str) -> (bool, bool, String) {
    let (ok, stdout, _) = run_cmd_hidden("sc.exe", &["query", svc_name]);
    if !ok {
        return (false, false, "NOT_INSTALLED".to_string());
    }

    let upper = stdout.to_uppercase();
    if upper.contains("RUNNING") || upper.contains("正在執行") {
        (true, true, "RUNNING".to_string())
    } else if upper.contains("STOPPED") || upper.contains("已停止") {
        (true, false, "STOPPED".to_string())
    } else if upper.contains("START_PENDING") || upper.contains("啟動中") {
        (true, false, "STARTING".to_string())
    } else if upper.contains("STOP_PENDING") || upper.contains("停止中") {
        (true, false, "STOPPING".to_string())
    } else {
        (true, false, "UNKNOWN".to_string())
    }
}

pub fn get_service_binary_path(svc_name: &str) -> String {
    let (ok, stdout, _) = run_cmd_hidden("sc.exe", &["qc", svc_name]);
    if !ok {
        return String::new();
    }
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("BINARY_PATH_NAME") {
            if let Some((_, val)) = trimmed.split_once(':') {
                return val.trim().trim_matches('"').to_string();
            }
        }
    }
    String::new()
}

pub fn query_all_system_services() -> Vec<SystemServiceStatus> {
    let defs = [
        (
            "LAH_Web_Service",
            "LAH 智慧管控：應用系統入口與 API 閘道服務 (Apache 2.4)",
        ),
        (
            "LAH_PM2_Service",
            "LAH 智慧管控：背景服務總線與健康管理系統 (PM2 Daemon)",
        ),
    ];

    defs.iter()
        .map(|(name, display)| {
            let (installed, running, status) = get_service_status(name);
            SystemServiceStatus {
                name: name.to_string(),
                display_name: display.to_string(),
                status,
                is_running: running,
                is_installed: installed,
                binary_path: if installed {
                    get_service_binary_path(name)
                } else {
                    String::new()
                },
            }
        })
        .collect()
}

pub fn start_service(svc_name: &str) -> Result<String, String> {
    let (installed, running, status) = get_service_status(svc_name);
    if !installed {
        return Err(format!("服務 [{}] 尚未在系統中註冊！", svc_name));
    }
    if running {
        return Ok(format!("服務 [{}] 已處於運行中狀態。", svc_name));
    }

    // 若服務正處於 STOP_PENDING (STOPPING)，先等待其完全停止，避免 net start 因狀態衝突而直接失敗
    if status == "STOPPING" {
        for _ in 0..40 {
            sleep(Duration::from_millis(500));
            let (_, r, st) = get_service_status(svc_name);
            if r {
                return Ok(format!("服務 [{}] 已處於運行中狀態。", svc_name));
            }
            if st != "STOPPING" {
                break;
            }
        }
        sleep(Duration::from_millis(600));
    }

    let mut last_err = String::new();
    let mut last_ok = false;

    for attempt in 0..3 {
        let (ok, _, stderr) =
            run_cmd_hidden_with_timeout("net.exe", &["start", svc_name], Duration::from_secs(25));
        last_ok = ok;
        if !stderr.trim().is_empty() {
            last_err = stderr;
        }

        for _ in 0..24 {
            let (_, r, st) = get_service_status(svc_name);
            if r {
                return Ok(format!("服務 [{}] 已成功啟動！", svc_name));
            }
            if st == "STOPPED" && !ok {
                break;
            }
            sleep(Duration::from_millis(500));
        }

        let (_, r, _) = get_service_status(svc_name);
        if r {
            return Ok(format!("服務 [{}] 已成功啟動！", svc_name));
        }

        if attempt < 2 {
            sleep(Duration::from_millis(1200));
        }
    }

    if last_ok {
        Ok(format!("啟動指令已送出，服務 [{}] 正在啟動中...", svc_name))
    } else {
        Err(format!("啟動服務 [{}] 失敗：{}", svc_name, last_err))
    }
}

pub fn stop_service(svc_name: &str) -> Result<String, String> {
    let (installed, running, status) = get_service_status(svc_name);
    if !installed {
        return Err(format!("服務 [{}] 尚未在系統中註冊！", svc_name));
    }
    if !running && status == "STOPPED" {
        return Ok(format!("服務 [{}] 已經停止。", svc_name));
    }

    let (ok, _, stderr) = if status == "STOPPING" {
        (true, String::new(), String::new())
    } else {
        run_cmd_hidden_with_timeout("net.exe", &["stop", svc_name], Duration::from_secs(25))
    };

    // 必須確認狀態真正進入 STOPPED（不可僅檢查 !r，否則 STOP_PENDING 會被誤判為已停止）
    for _ in 0..40 {
        let (_, r, st) = get_service_status(svc_name);
        if !r && st == "STOPPED" {
            return Ok(format!("服務 [{}] 已成功停止。", svc_name));
        }
        sleep(Duration::from_millis(500));
    }

    let (_, r, st) = get_service_status(svc_name);
    if !r && st == "STOPPED" {
        return Ok(format!("服務 [{}] 已成功停止。", svc_name));
    }

    if ok || st == "STOPPING" {
        Ok(format!("停止指令已送出，服務 [{}] 正在停止中...", svc_name))
    } else {
        Err(format!("停止服務 [{}] 失敗：{}", svc_name, stderr))
    }
}

pub fn restart_service(svc_name: &str) -> Result<String, String> {
    let (installed, _, _) = get_service_status(svc_name);
    if !installed {
        return Err(format!("服務 [{}] 尚未在系統中註冊！", svc_name));
    }

    let _ = stop_service(svc_name);

    // 確保服務已完全脫離 STOPPING / RUNNING 狀態後再執行啟動
    for _ in 0..30 {
        let (_, r, st) = get_service_status(svc_name);
        if !r && st == "STOPPED" {
            break;
        }
        sleep(Duration::from_millis(500));
    }

    sleep(Duration::from_millis(800));
    start_service(svc_name)
}
