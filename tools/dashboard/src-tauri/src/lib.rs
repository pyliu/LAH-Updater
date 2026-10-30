pub mod logs;
pub mod pm2;
pub mod services;
pub mod sysinfo;
pub mod types;

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use types::{
    AppInfo, CommandActionResult, HostMetrics, LogContentResult, LogFileInfo, Pm2ProcessInfo,
    SystemServiceStatus,
};

pub fn resolve_root_dir() -> PathBuf {
    // 1. 優先檢查目前執行檔所在目錄
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            if parent.join("lah-cmd.bat").is_file()
                || parent.join("node").join("pm2.cmd").is_file()
                || parent.join("Apache24").is_dir()
            {
                return parent.to_path_buf();
            }
        }
    }

    // 2. 探測標準安裝目錄 C:\AppServ
    let c_cand = PathBuf::from(r"C:\AppServ");
    if c_cand.join("lah-cmd.bat").is_file() || c_cand.join("node").is_dir() {
        return c_cand;
    }

    // 3. 探測次要磁碟 D:\AppServ
    let d_cand = PathBuf::from(r"D:\AppServ");
    if d_cand.join("lah-cmd.bat").is_file() || d_cand.join("node").is_dir() {
        return d_cand;
    }

    // 預設退回目前執行路徑父目錄
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from(r"C:\AppServ"))
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    let root = resolve_root_dir();
    let has_apache = root.join("Apache24").is_dir();
    let has_pm2 = root.join("node").join("pm2.cmd").is_file();
    let is_valid = root.join("lah-cmd.bat").is_file() || (has_apache && has_pm2);

    AppInfo {
        root_dir: root.to_string_lossy().to_string(),
        is_valid_root: is_valid,
        has_apache,
        has_pm2,
    }
}

#[tauri::command]
fn get_system_services() -> Vec<SystemServiceStatus> {
    services::query_all_system_services()
}

#[tauri::command]
fn start_system_service(name: String) -> CommandActionResult {
    match services::start_service(&name) {
        Ok(msg) => CommandActionResult {
            success: true,
            message: msg,
        },
        Err(e) => CommandActionResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
fn stop_system_service(name: String) -> CommandActionResult {
    match services::stop_service(&name) {
        Ok(msg) => CommandActionResult {
            success: true,
            message: msg,
        },
        Err(e) => CommandActionResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
fn restart_system_service(name: String) -> CommandActionResult {
    match services::restart_service(&name) {
        Ok(msg) => CommandActionResult {
            success: true,
            message: msg,
        },
        Err(e) => CommandActionResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
fn get_pm2_processes() -> Result<Vec<Pm2ProcessInfo>, String> {
    let root = resolve_root_dir();
    pm2::query_pm2_processes(&root)
}

#[tauri::command]
fn restart_pm2_process(target: String) -> CommandActionResult {
    let root = resolve_root_dir();
    match pm2::pm2_restart(&root, &target) {
        Ok(msg) => CommandActionResult {
            success: true,
            message: msg,
        },
        Err(e) => CommandActionResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
fn stop_pm2_process(target: String) -> CommandActionResult {
    let root = resolve_root_dir();
    match pm2::pm2_stop(&root, &target) {
        Ok(msg) => CommandActionResult {
            success: true,
            message: msg,
        },
        Err(e) => CommandActionResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
fn start_pm2_process(target: String) -> CommandActionResult {
    let root = resolve_root_dir();
    match pm2::pm2_start(&root, &target) {
        Ok(msg) => CommandActionResult {
            success: true,
            message: msg,
        },
        Err(e) => CommandActionResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
fn restart_all_services() -> CommandActionResult {
    let root = resolve_root_dir();
    let mut messages = Vec::new();

    // 1. 重啟 PM2 所有進程
    match pm2::pm2_restart(&root, "all") {
        Ok(msg) => messages.push(msg),
        Err(e) => messages.push(format!("PM2 重啟失敗：{}", e)),
    }

    // 2. 重啟 Apache 系統服務 (若已安裝)
    let (apache_installed, _, _) = services::get_service_status("LAH_Web_Service");
    if apache_installed {
        match services::restart_service("LAH_Web_Service") {
            Ok(msg) => messages.push(msg),
            Err(e) => messages.push(format!("Apache 服務重啟失敗：{}", e)),
        }
    }

    CommandActionResult {
        success: true,
        message: messages.join("\n"),
    }
}

#[tauri::command]
fn get_host_metrics() -> HostMetrics {
    let root = resolve_root_dir();
    sysinfo::query_host_metrics(&root)
}

#[tauri::command]
fn get_available_logs() -> Vec<LogFileInfo> {
    let root = resolve_root_dir();
    logs::list_available_logs(&root)
}

#[tauri::command]
fn get_log_tail(path: String, max_lines: usize) -> Result<LogContentResult, String> {
    logs::read_log_tail(&path, max_lines)
}

#[tauri::command]
fn open_log_in_notepad(path: String) -> CommandActionResult {
    match logs::open_in_notepad(&path) {
        Ok(_) => CommandActionResult {
            success: true,
            message: "已啟動記事本開啟日誌檔。".to_string(),
        },
        Err(e) => CommandActionResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
fn open_logs_folder() -> CommandActionResult {
    let root = resolve_root_dir();
    let logs_dir = root.join("logs");
    match logs::open_folder(&logs_dir.to_string_lossy()) {
        Ok(_) => CommandActionResult {
            success: true,
            message: "已開啟日誌資料夾。".to_string(),
        },
        Err(e) => CommandActionResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
fn open_lah_cmd() -> CommandActionResult {
    let root = resolve_root_dir();
    let bat = root.join("lah-cmd.bat");
    if !bat.is_file() {
        return CommandActionResult {
            success: false,
            message: format!("找不到沙盒終端批次檔：{}", bat.display()),
        };
    }

    let res = Command::new("cmd.exe")
        .args(&["/C", "start", "LAH 智慧控管專屬終端", bat.to_string_lossy().as_ref()])
        .current_dir(&root)
        .spawn();

    match res {
        Ok(_) => CommandActionResult {
            success: true,
            message: "已開啟 LAH 沙盒維運終端！".to_string(),
        },
        Err(e) => CommandActionResult {
            success: false,
            message: format!("啟動終端失敗：{}", e),
        },
    }
}

#[tauri::command]
fn open_web_frontend() -> CommandActionResult {
    let res = Command::new("explorer.exe")
        .arg("http://127.0.0.1:8080/")
        .spawn();

    match res {
        Ok(_) => CommandActionResult {
            success: true,
            message: "已開啟預設瀏覽器導向前端入口。".to_string(),
        },
        Err(e) => CommandActionResult {
            success: false,
            message: format!("開啟瀏覽器失敗：{}", e),
        },
    }
}

#[tauri::command]
fn check_apache_syntax() -> CommandActionResult {
    let root = resolve_root_dir();
    let httpd = root.join("Apache24").join("bin").join("httpd.exe");
    if !httpd.is_file() {
        return CommandActionResult {
            success: false,
            message: format!("找不到 Apache 執行檔：{}", httpd.display()),
        };
    }

    let mut cmd = Command::new(&httpd);
    cmd.arg("-t");
    cmd.creation_flags(services::CREATE_NO_WINDOW);

    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let combined = if stderr.is_empty() {
                stdout
            } else if stdout.is_empty() {
                stderr
            } else {
                format!("{}\n{}", stdout, stderr)
            };

            let ok = combined.contains("Syntax OK") || output.status.success();
            CommandActionResult {
                success: ok,
                message: if combined.is_empty() {
                    "Apache 語法校驗正常 (Syntax OK)".to_string()
                } else {
                    combined
                },
            }
        }
        Err(e) => CommandActionResult {
            success: false,
            message: format!("執行 Apache 語法檢查失敗：{}", e),
        },
    }
}

#[tauri::command]
fn trigger_logrotate() -> CommandActionResult {
    let root = resolve_root_dir();
    match pm2::pm2_trigger_logrotate(&root) {
        Ok(msg) => CommandActionResult {
            success: true,
            message: msg,
        },
        Err(e) => CommandActionResult {
            success: false,
            message: e,
        },
    }
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.maximize();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            get_system_services,
            start_system_service,
            stop_system_service,
            restart_system_service,
            get_pm2_processes,
            restart_pm2_process,
            stop_pm2_process,
            start_pm2_process,
            restart_all_services,
            get_host_metrics,
            get_available_logs,
            get_log_tail,
            open_log_in_notepad,
            open_logs_folder,
            open_lah_cmd,
            open_web_frontend,
            check_apache_syntax,
            trigger_logrotate,
        ])
        .run(tauri::generate_context!())
        .expect("error while running lah-dashboard");
}
