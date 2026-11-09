pub mod audit;
pub mod console;
pub mod header_protection;
pub mod install;
pub mod pm2;
pub mod system;
pub mod types;
pub mod uninstall;

use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use types::{
    CommandActionResult, ConsoleOverview, InstallOptions, InstallReport, LogContentResult,
    LogFileInfo, LogPayload, ProgressPayload, ScanSummary, SecurityAuditReport,
};

#[tauri::command]
async fn preview_uninstall(root_dir: String) -> Result<uninstall::RemovalPlan, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = uninstall::preview_operation_guard()?;
        uninstall::preview(&root_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn run_uninstall(app: AppHandle, plan: uninstall::RemovalPlan) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = uninstall::operation_guard()?;
        let result = uninstall::execute(
            plan,
            &|message, level| {
                let _ = app.emit(
                    "log",
                    LogPayload {
                        message: message.into(),
                        level: level.into(),
                    },
                );
            },
            &|percent, message| {
                let _ = app.emit(
                    "progress",
                    ProgressPayload {
                        percent,
                        message: message.into(),
                    },
                );
            },
        );
        if let Err(error) = &result {
            let _ = app.emit(
                "log",
                LogPayload {
                    level: "error".into(),
                    message: error.clone(),
                },
            );
        }
        result
    })
    .await
    .map_err(|e| e.to_string())?
}

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
fn has_adjacent_node_zip() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("base").join("node.zip")))
        .is_some_and(|path| path.is_file())
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
        let _guard = uninstall::operation_guard()?;
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
        Ok(install::execute_install(options, &log_fn, &progress_fn))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_header_protection(
    root_dir: String,
    pm_id: u32,
    enabled: bool,
    expected_exec_path: String,
) -> Result<CommandActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = uninstall::operation_guard()?;
        header_protection::set(
            &resolve_root(&root_dir),
            pm_id,
            enabled,
            &expected_exec_path,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_console_overview(root_dir: String) -> Result<ConsoleOverview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = uninstall::operation_guard()?;
        let root = resolve_root(&root_dir);
        Ok(console::get_console_overview(&root))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn pm2_process_action(
    root_dir: String,
    action: String,
    target: String,
) -> Result<CommandActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = uninstall::operation_guard()?;
        let root = resolve_root(&root_dir);
        Ok(console::pm2_process_action(&root, &action, &target))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn pm2_service_action(
    root_dir: String,
    action: String,
) -> Result<CommandActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = uninstall::operation_guard()?;
        let root = resolve_root(&root_dir);
        Ok(console::pm2_service_action(&root, &action))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn repair_logrotate(root_dir: String) -> Result<CommandActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = uninstall::operation_guard()?;
        let root = resolve_root(&root_dir);
        Ok(console::repair_or_restart_logrotate(&root))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn trigger_logrotate_now(root_dir: String) -> Result<CommandActionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = uninstall::operation_guard()?;
        let root = resolve_root(&root_dir);
        Ok(console::trigger_logrotate_now(&root))
    })
    .await
    .map_err(|e| e.to_string())?
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
    let Ok(_guard) = uninstall::operation_guard() else {
        return CommandActionResult {
            success: false,
            message: "另一項沙盒作業正在執行。".into(),
        };
    };
    let root = resolve_root(&root_dir);
    console::open_lah_cmd(&root)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            check_admin,
            scan_environment,
            has_adjacent_node_zip,
            check_root_dir,
            run_security_audit,
            run_install,
            preview_uninstall,
            run_uninstall,
            get_console_overview,
            set_header_protection,
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
