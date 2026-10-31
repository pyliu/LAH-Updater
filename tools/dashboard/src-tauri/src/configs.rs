use crate::services::CREATE_NO_WINDOW;
use crate::types::{CommandActionResult, ConfigBackupInfo, ConfigFileInfo, ConfigSaveResult};
use chrono::{DateTime, Local};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn find_example_file(config_path: &Path) -> Option<PathBuf> {
    let parent = config_path.parent()?;
    let file_name = config_path.file_name()?.to_string_lossy();

    // 1. 同名加上 .example (例如 .env -> .env.example, php.ini -> php.ini.example)
    let candidate1 = parent.join(format!("{}.example", file_name));
    if candidate1.is_file() {
        return Some(candidate1);
    }

    // 2. 若為 .env，檢查 env.example
    if file_name == ".env" {
        let candidate2 = parent.join("env.example");
        if candidate2.is_file() {
            return Some(candidate2);
        }
    }

    None
}

pub fn list_core_configs(root_dir: &Path) -> Vec<ConfigFileInfo> {
    // 智慧路徑定位：支援 AppServ 與開發工作區目錄結構
    let www_env = {
        let p1 = root_dir.join("www").join(".env");
        if p1.is_file() {
            p1
        } else {
            root_dir.join("LAH-API").join("www").join(".env")
        }
    };

    let nuxt_env = {
        let p1 = root_dir.join("LAH-NUXTJS").join(".env");
        if p1.is_file() {
            p1
        } else {
            root_dir.join("LAH-FE").join(".env")
        }
    };

    let targets = [
        (
            "apache_httpd",
            "Apache 核心組態 (httpd.conf)",
            root_dir.join("Apache24").join("conf").join("httpd.conf"),
            "apache",
        ),
        (
            "apache_vhosts",
            "Apache 虛擬主機 (httpd-vhosts.conf)",
            root_dir
                .join("Apache24")
                .join("conf")
                .join("extra")
                .join("httpd-vhosts.conf"),
            "apache",
        ),
        (
            "php7_ini",
            "PHP 7.3 組態 (php.ini)",
            root_dir.join("php7").join("php.ini"),
            "apache",
        ),
        (
            "php85_ini",
            "PHP 8.5 組態 (php.ini)",
            root_dir.join("php85").join("php.ini"),
            "pm2",
        ),
        (
            "www_env",
            "後端 API 環境變數 (www/.env)",
            www_env,
            "pm2",
        ),
        (
            "nuxt_env",
            "前端網站環境變數 (LAH-NUXTJS/.env)",
            nuxt_env,
            "pm2",
        ),
        (
            "wss_env",
            "即時通訊伺服器環境變數 (.env)",
            root_dir.join("lah-messenger-server").join(".env"),
            "pm2",
        ),
        (
            "api_core_env",
            "LAH-API 核心環境變數 (.env)",
            root_dir.join("LAH-API").join(".env"),
            "pm2",
        ),
        (
            "apache_winsw",
            "Apache 系統服務組態 (lah-web-service.xml)",
            root_dir
                .join("Apache24")
                .join("bin")
                .join("lah-web-service.xml"),
            "apache",
        ),
        (
            "pm2_winsw",
            "PM2 系統服務組態 (lah-pm2-service.xml)",
            root_dir
                .join("node")
                .join("service")
                .join("lah-pm2-service.xml"),
            "pm2",
        ),
    ];

    let mut result = Vec::new();
    for (key, label, path, related_svc) in targets {
        let exists = path.is_file();
        let path_str = path.to_string_lossy().to_string();
        let mut size_bytes = 0;
        let mut modified = "-".to_string();

        if exists {
            if let Ok(meta) = fs::metadata(&path) {
                size_bytes = meta.len();
                if let Ok(mtime) = meta.modified() {
                    let dt: DateTime<Local> = mtime.into();
                    modified = dt.format("%Y-%m-%d %H:%M:%S").to_string();
                }
            }
        }

        let example_path = find_example_file(&path);
        let has_example = example_path.is_some();
        let example_path_str = example_path.map(|p| p.to_string_lossy().to_string());

        result.push(ConfigFileInfo {
            key: key.to_string(),
            label: label.to_string(),
            path: path_str,
            exists,
            size_bytes,
            modified,
            related_service: related_svc.to_string(),
            has_example,
            example_path: example_path_str,
        });
    }

    result
}

pub fn read_config_example_file(path_str: &str) -> Result<String, String> {
    let p = Path::new(path_str);
    let example_path = find_example_file(p)
        .ok_or_else(|| format!("此組態目錄下未偵測到 .example 範本檔 ({})", path_str))?;

    read_config_file(&example_path.to_string_lossy())
}

pub fn read_config_file(path_str: &str) -> Result<String, String> {
    let p = Path::new(path_str);
    if !p.is_file() {
        return Err(format!("組態檔案不存在：{}", path_str));
    }

    let mut file = File::open(p).map_err(|e| format!("無法開啟檔案：{}", e))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| format!("讀取檔案失敗：{}", e))?;

    // 處理 UTF-8 BOM
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8(bytes[3..].to_vec())
            .map_err(|e| format!("檔案非標準 UTF-8 編碼：{}", e))
    } else {
        String::from_utf8(bytes).map_err(|e| format!("檔案非標準 UTF-8 編碼：{}", e))
    }
}

pub fn check_apache_syntax_with_output(root_dir: &Path) -> (bool, String) {
    let httpd_exe = root_dir.join("Apache24").join("bin").join("httpd.exe");
    if !httpd_exe.is_file() {
        return (true, "未偵測到 Apache24 執行檔，略過語法預檢".to_string());
    }

    let mut cmd = Command::new(&httpd_exe);
    cmd.arg("-t");
    cmd.current_dir(root_dir.join("Apache24"));
    cmd.creation_flags(CREATE_NO_WINDOW);

    match cmd.output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let full_out = format!("{}\n{}", stdout.trim(), stderr.trim())
                .trim()
                .to_string();
            (out.status.success(), full_out)
        }
        Err(e) => (false, format!("執行 httpd.exe -t 失敗：{}", e)),
    }
}

pub fn write_config_file(root_dir: &Path, path_str: &str, content: &str) -> ConfigSaveResult {
    let p = Path::new(path_str);
    if !p.is_file() {
        return ConfigSaveResult {
            success: false,
            message: format!("目標檔案不存在：{}", path_str),
            backup_path: None,
            syntax_checked: false,
            syntax_error: false,
            syntax_output: String::new(),
            related_service: "none".to_string(),
        };
    }

    // 判斷關聯服務
    let p_lower = path_str.to_lowercase();
    let related_svc = if p_lower.contains("apache") || p_lower.contains("httpd") || p_lower.contains("php7") {
        "apache"
    } else if p_lower.contains("lah-api") || p_lower.contains("messenger") || p_lower.contains("php85") || p_lower.contains("pm2") {
        "pm2"
    } else {
        "none"
    };

    // 1. 建立自動備份: <path>.bak_<YYYYMMDD_HHmmss>
    let now = chrono::Local::now();
    let bak_suffix = now.format("%Y%m%d_%H%M%S").to_string();
    let bak_path = PathBuf::from(format!("{}.bak_{}", path_str, bak_suffix));
    let mut backup_saved = None;

    if let Ok(_) = fs::copy(p, &bak_path) {
        backup_saved = Some(bak_path.to_string_lossy().to_string());
        // 清理多餘備份：最多保留 5 份最新的 .bak_* 檔案
        clean_old_backups(p, 5);
    }

    // 2. 寫入新內容
    let write_res = (|| -> Result<(), std::io::Error> {
        let mut file = File::create(p)?;
        file.write_all(content.as_bytes())?;
        file.flush()?;
        Ok(())
    })();

    if let Err(e) = write_res {
        return ConfigSaveResult {
            success: false,
            message: format!("寫入組態檔案失敗：{}", e),
            backup_path: backup_saved,
            syntax_checked: false,
            syntax_error: false,
            syntax_output: String::new(),
            related_service: related_svc.to_string(),
        };
    }

    // 3. 若為 Apache 組態，執行語法預檢
    if p_lower.contains("httpd") {
        let (syntax_ok, syntax_msg) = check_apache_syntax_with_output(root_dir);
        if !syntax_ok {
            return ConfigSaveResult {
                success: true,
                message: "檔案已儲存，但 Apache 語法檢查未通過！".to_string(),
                backup_path: backup_saved,
                syntax_checked: true,
                syntax_error: true,
                syntax_output: syntax_msg,
                related_service: "apache".to_string(),
            };
        }

        return ConfigSaveResult {
            success: true,
            message: "✓ 組態已成功儲存，且通過 Apache 語法檢查 (Syntax OK)！".to_string(),
            backup_path: backup_saved,
            syntax_checked: true,
            syntax_error: false,
            syntax_output: syntax_msg,
            related_service: "apache".to_string(),
        };
    }

    ConfigSaveResult {
        success: true,
        message: "✓ 組態檔案已成功儲存！".to_string(),
        backup_path: backup_saved,
        syntax_checked: false,
        syntax_error: false,
        syntax_output: String::new(),
        related_service: related_svc.to_string(),
    }
}

fn clean_old_backups(target_file: &Path, retain_count: usize) {
    let parent = match target_file.parent() {
        Some(p) => p,
        None => return,
    };
    let file_name = match target_file.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return,
    };

    let prefix = format!("{}.bak_", file_name);
    let mut baks = Vec::new();

    if let Ok(entries) = fs::read_dir(parent) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(n) = p.file_name().and_then(|s| s.to_str()) {
                    if n.starts_with(&prefix) {
                        baks.push(p);
                    }
                }
            }
        }
    }

    baks.sort_by_key(|p| {
        fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    baks.reverse(); // 最新的在前

    if baks.len() > retain_count {
        for old in &baks[retain_count..] {
            let _ = fs::remove_file(old);
        }
    }
}

pub fn list_config_backups(path_str: &str) -> Vec<ConfigBackupInfo> {
    let target = Path::new(path_str);
    let parent = match target.parent() {
        Some(p) => p,
        None => return Vec::new(),
    };
    let file_name = match target.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return Vec::new(),
    };

    let prefix = format!("{}.bak_", file_name);
    let mut result = Vec::new();

    if let Ok(entries) = fs::read_dir(parent) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(n) = p.file_name().and_then(|s| s.to_str()) {
                    if n.starts_with(&prefix) {
                        let mut size_bytes = 0;
                        let mut modified = "-".to_string();
                        if let Ok(meta) = fs::metadata(&p) {
                            size_bytes = meta.len();
                            if let Ok(mtime) = meta.modified() {
                                let dt: DateTime<Local> = mtime.into();
                                modified = dt.format("%Y-%m-%d %H:%M:%S").to_string();
                            }
                        }
                        result.push(ConfigBackupInfo {
                            file_name: n.to_string(),
                            path: p.to_string_lossy().to_string(),
                            size_bytes,
                            modified,
                        });
                    }
                }
            }
        }
    }

    result.sort_by(|a, b| b.file_name.cmp(&a.file_name));
    result
}

pub fn restore_config_backup(target_path_str: &str, backup_path_str: &str) -> CommandActionResult {
    let target = Path::new(target_path_str);
    let backup = Path::new(backup_path_str);

    if !backup.is_file() {
        return CommandActionResult {
            success: false,
            message: format!("備份檔案不存在：{}", backup_path_str),
        };
    }

    match fs::copy(backup, target) {
        Ok(_) => CommandActionResult {
            success: true,
            message: format!("✓ 已成功從備份還原：{}", backup_path_str),
        },
        Err(e) => CommandActionResult {
            success: false,
            message: format!("還原失敗：{}", e),
        },
    }
}

pub fn pick_custom_config_file() -> Option<String> {
    let ps_script = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Title = "選取要編輯的組態檔案"
$dialog.Filter = "常用設定檔 (*.conf;*.ini;*.env;*.json;*.xml;*.txt)|*.conf;*.ini;*.env;*.json;*.xml;*.txt|所有檔案 (*.*)|*.*"
$dialog.InitialDirectory = "C:\AppServ"
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    Write-Output $dialog.FileName
}
"#;
    let mut cmd = Command::new("powershell.exe");
    cmd.arg("-NoProfile");
    cmd.arg("-NonInteractive");
    cmd.arg("-Command");
    cmd.arg(ps_script);
    cmd.creation_flags(CREATE_NO_WINDOW);

    match cmd.output() {
        Ok(out) => {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() && Path::new(&path).is_file() {
                Some(path)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}
