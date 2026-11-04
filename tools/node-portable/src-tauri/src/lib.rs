pub mod audit;
pub mod console;
pub mod install;
pub mod pm2;
pub mod system;
pub mod types;

use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use types::{
    CommandActionResult, ConsoleOverview, InstallOptions, InstallReport, LogContentResult,
    LogFileInfo, LogPayload, ProgressPayload, ScanSummary, SecurityAuditReport,
};

fn resolve_root(root_dir: &str) -> PathBuf {
    if root_dir.trim().is_empty() {
        system::detect_default_root_dir()
    } else {
        PathBuf::from(root_dir.trim())
    }
}

#[tauri::command]
fn check_admin() -> bool {
    system::is_admin()
}

#[tauri::command]
fn exit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn scan_environment(root_dir: String) -> ScanSummary {
    install::scan(&root_dir)
}

#[tauri::command]
fn check_root_dir(path: String) -> bool {
    let p = PathBuf::from(path.trim());
    let probe = p.join(".lah_write_test");
    p.is_dir() && fs::write(&probe, b"t").is_ok() && fs::remove_file(&probe).is_ok()
}

#[tauri::command]
async fn run_security_audit(root_dir: String) -> Result<SecurityAuditReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = resolve_root(&root_dir);
        audit::execute_security_audit(&root, None)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn run_install(app: AppHandle, options: InstallOptions) -> Result<InstallReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_log = app.clone();
        let log_fn = move |msg: &str, level: &str| {
            let _ = app_log.emit(
                "log",
                LogPayload {
                    level: level.to_string(),
                    message: msg.to_string(),
                },
            );
        };
        let app_prog = app.clone();
        let progress_fn = move |percent: f64, message: &str| {
            let _ = app_prog.emit(
                "progress",
                ProgressPayload {
                    percent,
                    message: message.to_string(),
                },
            );
        };
        install::execute_install(options, &log_fn, &progress_fn)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_console_overview(root_dir: String) -> Result<ConsoleOverview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = resolve_root(&root_dir);
        console::get_console_overview(&root)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn pm2_process_action(
    root_dir: String,
    action: String,
    target: String,
) -> Result<CommandActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = resolve_root(&root_dir);
        console::pm2_process_action(&root, &action, &target)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn pm2_service_action(
    root_dir: String,
    action: String,
) -> Result<CommandActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = resolve_root(&root_dir);
        console::pm2_service_action(&root, &action)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn repair_logrotate(root_dir: String) -> Result<CommandActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = resolve_root(&root_dir);
        console::repair_or_restart_logrotate(&root)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn trigger_logrotate_now(root_dir: String) -> Result<CommandActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = resolve_root(&root_dir);
        console::trigger_logrotate_now(&root)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_sandbox_logs(root_dir: String) -> Result<Vec<LogFileInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = resolve_root(&root_dir);
        console::list_sandbox_logs(&root)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_log_tail(path: String, max_lines: usize) -> Result<LogContentResult, String> {
    console::read_log_tail(&path, max_lines)
}

#[tauri::command]
fn open_log_in_notepad(path: String) -> CommandActionResult {
    console::open_in_notepad(&path)
}

#[tauri::command]
fn open_logs_folder(root_dir: String) -> CommandActionResult {
    let root = resolve_root(&root_dir);
    console::open_logs_folder(&root)
}

#[tauri::command]
fn open_lah_cmd(root_dir: String) -> CommandActionResult {
    let root = resolve_root(&root_dir);
    console::open_lah_cmd(&root)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            check_admin,
            scan_environment,
            check_root_dir,
            run_security_audit,
            run_install,
            get_console_overview,
            pm2_process_action,
            pm2_service_action,
            repair_logrotate,
            trigger_logrotate_now,
            get_sandbox_logs,
            get_log_tail,
            open_log_in_notepad,
            open_logs_folder,
            open_lah_cmd,
            exit_app
        ])
        .run(tauri::generate_context!())
        .expect("啟動智慧控管與監控系統 - Node 沙盒監管工具失敗");
}
