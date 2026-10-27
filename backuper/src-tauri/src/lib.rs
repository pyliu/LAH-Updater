pub mod backup;
pub mod restore;

use backup::{BackupReport, Progress};
use restore::RestoreReport;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

#[tauri::command]
fn check_services(root: String) -> Vec<String> {
    restore::running_services(&PathBuf::from(root))
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
        .invoke_handler(tauri::generate_handler![run_backup, run_restore, check_services])
        .run(tauri::generate_context!())
        .expect("啟動備份工具失敗");
}
