pub mod apache_conf;
pub mod autostart;
pub mod cleaner;
pub mod process;
pub mod services;
pub mod system;
pub mod types;
pub mod uninstall;

use tauri::{AppHandle, Emitter};
use types::{LogPayload, ProgressPayload, UninstallOptions, UninstallReport};

#[tauri::command]
fn fit_window_to_content(window: tauri::Window, client_height: f64) -> Result<(), String> {
    use tauri::{LogicalSize, Size};
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let inner = window.inner_size().unwrap_or_default();

    let target_inner_height = client_height.max(480.0).min(900.0);
    let cur_width = (inner.width as f64 / scale_factor).max(860.0);

    let _ = window.set_size(Size::Logical(LogicalSize {
        width: cur_width,
        height: target_inner_height,
    }));
    Ok(())
}

#[tauri::command]
fn check_admin() -> bool {
    system::is_admin()
}

#[tauri::command]
fn detect_root_dir() -> String {
    system::detect_default_root_dir().to_string_lossy().to_string()
}

#[tauri::command]
fn exit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
async fn run_uninstall(app: AppHandle, options: UninstallOptions) -> Result<UninstallReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_handle = app.clone();
        let log_fn = move |msg: &str, level: &str| {
            let _ = app_handle.emit(
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

        uninstall::execute_uninstall_workflow(options, log_fn, progress_fn)
    })
    .await
    .map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![fit_window_to_content, check_admin, detect_root_dir, run_uninstall, exit_app])
        .run(tauri::generate_context!())
        .expect("啟動智慧控管與監控系統 - 自動移除工具失敗");
}
