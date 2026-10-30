use crate::types::SystemServiceStatus;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

pub const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn run_cmd_hidden(cmd: &str, args: &[&str]) -> (bool, String, String) {
    let mut command = Command::new(cmd);
    command.args(args);
    command.creation_flags(CREATE_NO_WINDOW);

    match command.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            (output.status.success(), stdout, stderr)
        }
        Err(e) => (false, String::new(), e.to_string()),
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
            }
        })
        .collect()
}

pub fn start_service(svc_name: &str) -> Result<String, String> {
    let (installed, running, _) = get_service_status(svc_name);
    if !installed {
        return Err(format!("服務 [{}] 尚未在系統中註冊！", svc_name));
    }
    if running {
        return Ok(format!("服務 [{}] 已處於運行中狀態。", svc_name));
    }

    let (ok, _, stderr) = run_cmd_hidden("net.exe", &["start", svc_name]);
    for _ in 0..12 {
        let (_, r, _) = get_service_status(svc_name);
        if r {
            return Ok(format!("服務 [{}] 已成功啟動！", svc_name));
        }
        sleep(Duration::from_millis(500));
    }

    if ok {
        Ok(format!("啟動指令已送出，服務 [{}] 正在啟動中...", svc_name))
    } else {
        Err(format!("啟動服務 [{}] 失敗：{}", svc_name, stderr))
    }
}

pub fn stop_service(svc_name: &str) -> Result<String, String> {
    let (installed, running, _) = get_service_status(svc_name);
    if !installed {
        return Err(format!("服務 [{}] 尚未在系統中註冊！", svc_name));
    }
    if !running {
        return Ok(format!("服務 [{}] 已經停止。", svc_name));
    }

    let (ok, _, stderr) = run_cmd_hidden("net.exe", &["stop", svc_name]);
    for _ in 0..12 {
        let (_, r, _) = get_service_status(svc_name);
        if !r {
            return Ok(format!("服務 [{}] 已成功停止。", svc_name));
        }
        sleep(Duration::from_millis(500));
    }

    if ok {
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
    sleep(Duration::from_millis(800));
    start_service(svc_name)
}
