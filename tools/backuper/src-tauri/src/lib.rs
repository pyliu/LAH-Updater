pub mod backup;
pub mod restore;
pub mod system;

use backup::{BackupReport, Progress};
use restore::RestoreReport;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

#[tauri::command]
fn fit_window_to_content(window: tauri::Window, client_height: f64) -> Result<(), String> {
    use tauri::{LogicalSize, Size};
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let inner = window.inner_size().unwrap_or_default();

    let target_inner_height = client_height.max(380.0).min(800.0);
    let cur_width = (inner.width as f64 / scale_factor).max(660.0);

    let _ = window.set_size(Size::Logical(LogicalSize {
        width: cur_width,
        height: target_inner_height,
    }));
    Ok(())
}

#[tauri::command]
fn check_services(root: String) -> Vec<String> {
    restore::running_services(&PathBuf::from(root))
}

#[tauri::command]
fn detect_root_dir() -> String {
    system::detect_default_root_dir().to_string_lossy().to_string()
}

#[tauri::command]
async fn run_restore(app: AppHandle, root: String, zip: String) -> Result<RestoreReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        restore::restore(&PathBuf::from(root), &PathBuf::from(zip), |p: Progress| {
            let _ = app.emit("progress", p);
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn run_backup(app: AppHandle, root: String, dest: String) -> Result<BackupReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        backup::backup(&PathBuf::from(root), &PathBuf::from(dest), |p: Progress| {
            let _ = app.emit("progress", p);
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![fit_window_to_content, run_backup, run_restore, check_services, detect_root_dir])
        .run(tauri::generate_context!())
        .expect("啟動備份工具失敗");
}
