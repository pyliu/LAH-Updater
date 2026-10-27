pub mod apache_conf;
pub mod extractor;
pub mod health;
pub mod pm2;
pub mod pipeline;
pub mod services;
pub mod system;
pub mod types;

use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use types::{PackageItem, RootDirCheck, ScanSummary, SystemConfigOptions, UpdateOptions, UpdateSummary};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct LogPayload {
    pub level: String,
    pub message: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ProgressPayload {
    pub percent: u32,
    pub message: String,
}

#[tauri::command]
fn fit_window_to_content(window: tauri::Window, client_height: f64) -> Result<(), String> {
    use tauri::{LogicalSize, Size};
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let outer = window.outer_size().unwrap_or_default();
    let inner = window.inner_size().unwrap_or_default();

    let border_height_logical = if outer.height >= inner.height {
        (outer.height - inner.height) as f64 / scale_factor
    } else {
        39.0
    };

    let target_outer_height = client_height + border_height_logical;
    let clamped_height = target_outer_height.max(500.0).min(960.0);
    let cur_width = (outer.width as f64 / scale_factor).max(960.0);

    let _ = window.set_size(Size::Logical(LogicalSize {
        width: cur_width,
        height: clamped_height,
    }));
    Ok(())
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
fn check_root_dir(path: String) -> RootDirCheck {
    let p = PathBuf::from(path.trim());
    if p.exists() && p.is_dir() {
        let test_file = p.join(".lah_write_test");
        let can_write = fs::write(&test_file, b"test").is_ok() && fs::remove_file(&test_file).is_ok();
        RootDirCheck {
            exists: true,
            path: p.to_string_lossy().to_string(),
            can_write,
            is_dir: true,
        }
    } else {
        RootDirCheck {
            exists: false,
            path: p.to_string_lossy().to_string(),
            can_write: false,
            is_dir: false,
        }
    }
}

#[tauri::command]
fn create_root_dir(path: String) -> Result<String, String> {
    let p = PathBuf::from(path.trim());
    system::ensure_root_dir(&p)?;
    Ok(p.to_string_lossy().to_string())
}

#[tauri::command]
fn read_release_notes() -> Option<String> {
    let app_dir = system::get_app_dir();
    if let Ok(entries) = fs::read_dir(&app_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                if (name.contains("更新說明") || name.contains("readme") || name.contains("release")) && name.ends_with(".txt") {
                    if let Ok(content) = fs::read_to_string(&p) {
                        return Some(content);
                    }
                }
            }
        }
    }
    None
}

#[tauri::command]
fn scan_environment(root_dir: String) -> ScanSummary {
    let app_dir = system::get_app_dir();
    let root_path = PathBuf::from(root_dir.trim());
    let root_exists = root_path.exists() && root_path.is_dir();

    let search_dirs = vec![
        app_dir.clone(),
        PathBuf::from(r"C:\Temp"),
        root_path.clone(),
    ];

    let packages: Vec<PackageItem> = extractor::scan_packages(&search_dirs, &root_path);

    let apache_service = services::query_apache_service();
    let apache_running = apache_service.as_ref().map_or(false, |s| services::is_service_running(s));

    let (pm2_running, pm2_info) = pm2::query_pm2_daemon_info(&root_path);
    let pm2_installed = pm2::resolve_pm2_command(&root_path).is_some();
    let pm2_service_installed = services::is_pm2_service_installed();
    let pm2_service_running = services::is_pm2_service_running();

    let backup_zip_found = extractor::find_latest_backup_zip(&search_dirs).map(|p| p.to_string_lossy().to_string());
    let release_notes = read_release_notes();

    ScanSummary {
        root_dir: root_path.to_string_lossy().to_string(),
        root_exists,
        packages,
        apache_service,
        apache_running,
        pm2_installed,
        pm2_running,
        pm2_daemon_info: pm2_info,
        pm2_service_installed,
        pm2_service_running,
        backup_zip_found,
        release_notes,
    }
}

#[tauri::command]
async fn run_update(app: AppHandle, options: UpdateOptions) -> Result<UpdateSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_handle = app.clone();
        let emit_fn = move |event: types::ProgressEvent| {
            if event.stage == "log" {
                let _ = app_handle.emit(
                    "log",
                    LogPayload {
                        level: event.level,
                        message: event.message,
                    },
                );
            } else if event.stage == "progress" {
                let _ = app_handle.emit(
                    "progress",
                    ProgressPayload {
                        percent: event.percent,
                        message: event.message,
                    },
                );
            }
        };

        pipeline::execute_update_pipeline(options, emit_fn)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn apply_system_config(app: AppHandle, options: SystemConfigOptions) -> Result<UpdateSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_handle = app.clone();
        let emit_fn = move |event: types::ProgressEvent| {
            if event.stage == "log" {
                let _ = app_handle.emit(
                    "log",
                    LogPayload {
                        level: event.level,
                        message: event.message,
                    },
                );
            } else if event.stage == "progress" {
                let _ = app_handle.emit(
                    "progress",
                    ProgressPayload {
                        percent: event.percent,
                        message: event.message,
                    },
                );
            }
        };

        pipeline::apply_system_configurations(options, emit_fn)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_installed_zips(zip_paths: Vec<String>) -> Result<usize, String> {
    let mut count = 0;
    for zp in zip_paths {
        let p = PathBuf::from(zp);
        if p.is_file() {
            if fs::remove_file(&p).is_ok() {
                count += 1;
            }
        }
    }
    Ok(count)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            fit_window_to_content,
            check_admin,
            check_root_dir,
            create_root_dir,
            read_release_notes,
            scan_environment,
            run_update,
            apply_system_config,
            delete_installed_zips,
            exit_app
        ])
        .run(tauri::generate_context!())
        .expect("啟動智慧控管與監控系統 - 自動更新工具失敗");
}
