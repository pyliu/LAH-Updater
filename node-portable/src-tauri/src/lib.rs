pub mod install;
pub mod pm2;
pub mod system;
pub mod types;

use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use types::{InstallOptions, InstallReport, LogPayload, ProgressPayload, ScanSummary};

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
async fn run_install(app: AppHandle, options: InstallOptions) -> Result<InstallReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_log = app.clone();
        let log_fn = move |msg: &str, level: &str| {
            let _ = app_log.emit("log", LogPayload { level: level.to_string(), message: msg.to_string() });
        };
        let app_prog = app.clone();
        let progress_fn = move |percent: f64, message: &str| {
            let _ = app_prog.emit("progress", ProgressPayload { percent, message: message.to_string() });
        };
        install::execute_install(options, &log_fn, &progress_fn)
    })
    .await
    .map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            check_admin,
            scan_environment,
            check_root_dir,
            run_install,
            exit_app
        ])
        .run(tauri::generate_context!())
        .expect("啟動智慧控管與監控系統 - 可攜 Node 安裝工具失敗");
}
