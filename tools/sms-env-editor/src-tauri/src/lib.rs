pub mod parser;
pub mod types;

use chrono::Local;
use std::fs;
use std::path::Path;
use tauri_plugin_dialog::DialogExt;
use types::{ConfigData, DiffItem, SaveResult};

const DEFAULT_PATH: &str = r"\\220.1.34.221\C$\SMSWEB_MOI\env.properties";

#[tauri::command]
fn get_default_path() -> String {
    DEFAULT_PATH.to_string()
}

#[tauri::command]
fn load_config(path: String) -> Result<ConfigData, String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("找不到檔案：{}", path));
    }

    let bytes = fs::read(p).map_err(|e| format!("讀取檔案失敗 ({}): {}", path, e))?;
    let config = parser::parse_env_properties(&bytes);
    Ok(config)
}

#[tauri::command]
fn preview_diff(path: String, new_data: ConfigData) -> Result<Vec<DiffItem>, String> {
    let p = Path::new(&path);
    let old_data = if p.exists() {
        let bytes = fs::read(p).map_err(|e| format!("無法讀取比對原檔: {}", e))?;
        parser::parse_env_properties(&bytes)
    } else {
        ConfigData::default()
    };

    let mut diffs = Vec::new();

    let mut check_diff = |key: &str, label: &str, old_val: &str, new_val: &str, is_secret: bool| {
        if old_val != new_val {
            diffs.push(DiffItem {
                key: key.to_string(),
                label: label.to_string(),
                old_val: old_val.to_string(),
                new_val: new_val.to_string(),
                is_secret,
            });
        }
    };

    check_diff("ap_url", "訊息平臺網址", &old_data.ap_url, &new_data.ap_url, false);
    check_diff("sms_user", "簡訊使用帳號", &old_data.sms_user, &new_data.sms_user, false);
    check_diff("sms_pwd", "簡訊使用密碼", &old_data.sms_pwd, &new_data.sms_pwd, true);
    check_diff("mail_user", "寄件伺服器帳號", &old_data.mail_user, &new_data.mail_user, false);
    check_diff("mail_pwd", "寄件伺服器密碼", &old_data.mail_pwd, &new_data.mail_pwd, true);
    check_diff("mail_sender_name", "寄件者名稱", &old_data.mail_sender_name, &new_data.mail_sender_name, false);
    check_diff("mail_sender_addr", "寄件者郵件位置", &old_data.mail_sender_addr, &new_data.mail_sender_addr, false);
    check_diff("db_ip", "資料庫 IP", &old_data.db_ip, &new_data.db_ip, false);
    check_diff("db_port", "資料庫 Port", &old_data.db_port, &new_data.db_port, false);
    check_diff("db_sid", "資料庫 SID", &old_data.db_sid, &new_data.db_sid, false);
    check_diff("db_user", "資料庫帳號", &old_data.db_user, &new_data.db_user, false);
    check_diff("db_pwd", "資料庫密碼", &old_data.db_pwd, &new_data.db_pwd, true);

    check_diff("search_time", "輪詢間隔 (分鐘)", &old_data.search_time, &new_data.search_time, false);
    check_diff("resend_time", "重送間隔 (分鐘)", &old_data.resend_time, &new_data.resend_time, false);
    check_diff("start_hours", "排程啟動時", &old_data.start_hours, &new_data.start_hours, false);
    check_diff("start_min", "排程啟動分", &old_data.start_min, &new_data.start_min, false);
    check_diff("end_hours", "排程結束時", &old_data.end_hours, &new_data.end_hours, false);
    check_diff("end_min", "排程結束分", &old_data.end_min, &new_data.end_min, false);

    Ok(diffs)
}

#[tauri::command]
fn save_config(path: String, data: ConfigData) -> Result<SaveResult, String> {
    let p = Path::new(&path);

    // 建立時間戳備份
    let mut backup_created = None;
    if p.exists() {
        let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_path = format!("{}.{}.bak", path, ts);
        fs::copy(p, &backup_path).map_err(|e| format!("建立備份失敗 ({}): {}", backup_path, e))?;
        backup_created = Some(backup_path);
    } else if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("建立目錄失敗: {}", e))?;
    }

    let bytes = parser::serialize_env_properties(&data);
    fs::write(p, bytes).map_err(|e| format!("寫入檔案失敗 ({}): {}", path, e))?;

    Ok(SaveResult {
        success: true,
        message: format!("設定已成功寫入 {}", path),
        backup_path: backup_created,
    })
}

#[tauri::command]
fn open_target_folder(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if p.exists() {
        let _ = std::process::Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| format!("無法開啟檔案總管: {}", e))?;
    } else if let Some(parent) = p.parent() {
        let _ = std::process::Command::new("explorer")
            .arg(parent.to_string_lossy().as_ref())
            .spawn()
            .map_err(|e| format!("無法開啟目錄: {}", e))?;
    } else {
        return Err("無效的路徑".to_string());
    }
    Ok(())
}

#[tauri::command]
fn pick_properties_file(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let file = app
        .dialog()
        .file()
        .add_filter("Properties 設定檔 (*.properties)", &["properties"])
        .add_filter("所有檔案 (*.*)", &["*"])
        .blocking_pick_file();

    Ok(file.map(|f| f.to_string()))
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_default_path,
            load_config,
            save_config,
            preview_diff,
            open_target_folder,
            pick_properties_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running sms-env-editor");
}
