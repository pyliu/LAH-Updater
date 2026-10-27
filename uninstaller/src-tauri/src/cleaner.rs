use crate::system::robust_rmtree;
use crate::types::UninstallOptions;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub fn clean_pm2_dumps(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    log_fn("⏳ 正在清除 PM2 快照狀態 (dump.pm2)...", "info");

    let mut candidates = Vec::new();
    let pm2_files = ["dump.pm2", "dump.pm2.bak", "module_conf.json", "pm2.pid", "pm2.log"];

    for f in &pm2_files {
        candidates.push(root_dir.join(".pm2").join(f));
        candidates.push(PathBuf::from(r"C:\Windows\System32\config\systemprofile\.pm2").join(f));
        if let Ok(u) = env::var("USERPROFILE") {
            candidates.push(Path::new(&u).join(".pm2").join(f));
        }
    }

    for p in candidates {
        if p.is_file() {
            let _ = crate::system::remove_readonly(&p);
            if fs::remove_file(&p).is_ok() {
                log_fn(&format!("  ✓ 已清除 PM2 狀態存檔: {}", p.display()), "info");
            }
        }
    }

    true
}

pub fn clean_files_and_directories(
    root_dir: &Path,
    options: &UninstallOptions,
    log_fn: &dyn Fn(&str, &str),
    progress_fn: &dyn Fn(f64, &str),
) {
    let mut dir_tasks = Vec::new();

    if options.do_remove_fe {
        dir_tasks.push((root_dir.join("LAH-NUXTJS"), "前端網站 (LAH-NUXTJS)"));
    }
    if options.do_remove_wss {
        dir_tasks.push((root_dir.join("lah-messenger-server"), "即時通訊伺服器 (lah-messenger-server)"));
    }
    if options.do_remove_api {
        dir_tasks.push((root_dir.join("LAH-API"), "現代化後端 API (LAH-API)"));
    }
    if options.do_remove_www {
        dir_tasks.push((root_dir.join("www"), "舊版後端生產庫 (www)"));
    }
    if options.do_remove_apache {
        dir_tasks.push((root_dir.join("Apache24"), "Apache 網頁伺服器 (Apache24)"));
    }
    if options.do_remove_node {
        dir_tasks.push((root_dir.join("node"), "Node.js 綠色環境 (node)"));
    }
    if options.do_remove_php85 {
        dir_tasks.push((root_dir.join("php85"), "PHP 8.5 手動安裝目錄 (php85)"));
    }
    if options.do_remove_php7 {
        dir_tasks.push((root_dir.join("php7"), "PHP 7 手動安裝目錄 (php7)"));
    }
    if options.do_remove_instantclient {
        dir_tasks.push((root_dir.join("instantclient_11_2"), "Oracle Instant Client (instantclient_11_2)"));
    }
    if options.do_remove_logs {
        dir_tasks.push((root_dir.join("logs"), "系統日誌目錄 (logs)"));
    }

    let total = dir_tasks.len();
    for (idx, (dpath, dname)) in dir_tasks.into_iter().enumerate() {
        if dpath.exists() {
            log_fn(&format!("  🗑️ 正在刪除 {dname} [{}]...", dpath.display()), "info");
            let (ok, _) = robust_rmtree(&dpath, log_fn);
            if ok {
                log_fn(&format!("  ✓ 已成功刪除 {dname}"), "success");
            }
        }
        let curr_pct = if total > 0 {
            75.0 + ((idx + 1) as f64 / total as f64) * 15.0
        } else {
            90.0
        };
        progress_fn(curr_pct, &format!("正在刪除: {dname}"));
    }
}

pub fn clean_root_directory(
    root_dir: &Path,
    options: &UninstallOptions,
    log_fn: &dyn Fn(&str, &str),
) {
    if !options.do_clean_root || !root_dir.is_dir() {
        return;
    }

    log_fn("⏳ 正在清理安裝根目錄其餘殘留檔案 (僅保留刪除前備份壓縮檔)...", "info");

    // 建立使用者未勾選需保留的目錄集合
    let mut keep_paths = std::collections::HashSet::new();
    if !options.do_remove_fe {
        keep_paths.insert(root_dir.join("LAH-NUXTJS").to_string_lossy().to_lowercase());
    }
    if !options.do_remove_wss {
        keep_paths.insert(root_dir.join("lah-messenger-server").to_string_lossy().to_lowercase());
    }
    if !options.do_remove_api {
        keep_paths.insert(root_dir.join("LAH-API").to_string_lossy().to_lowercase());
    }
    if !options.do_remove_www {
        keep_paths.insert(root_dir.join("www").to_string_lossy().to_lowercase());
    }
    if !options.do_remove_apache {
        keep_paths.insert(root_dir.join("Apache24").to_string_lossy().to_lowercase());
    }
    if !options.do_remove_node {
        keep_paths.insert(root_dir.join("node").to_string_lossy().to_lowercase());
    }
    if !options.do_remove_php85 {
        keep_paths.insert(root_dir.join("php85").to_string_lossy().to_lowercase());
    }
    if !options.do_remove_php7 {
        keep_paths.insert(root_dir.join("php7").to_string_lossy().to_lowercase());
    }
    if !options.do_remove_instantclient {
        keep_paths.insert(root_dir.join("instantclient_11_2").to_string_lossy().to_lowercase());
    }
    if !options.do_remove_logs {
        keep_paths.insert(root_dir.join("logs").to_string_lossy().to_lowercase());
    }

    // 磁碟機根目錄防呆保護 (例如 C:\ 或 D:\)
    let root_str = root_dir.to_string_lossy();
    if root_str.len() <= 3 && root_str.contains(':') {
        log_fn("⚠️ 偵測到安裝根目錄為磁碟機根路徑，為保護作業系統，略過整層目錄清理！", "warning");
        return;
    }

    if let Ok(entries) = fs::read_dir(root_dir) {
        for entry in entries.flatten() {
            let item_path = entry.path();
            let fname = entry.file_name().to_string_lossy().to_string();
            let norm_item = item_path.to_string_lossy().to_lowercase();

            // 1. 嚴格保護 backuper 產出之備份壓縮檔
            if fname.starts_with("_LAH_Backup_") && fname.ends_with(".zip") {
                log_fn(&format!("  📦 保留刪除前關鍵備份壓縮檔: [{fname}]"), "info");
                continue;
            }

            // 2. 保留使用者未選取移除的目錄
            if keep_paths.contains(&norm_item) {
                log_fn(&format!("  ℹ️ 保留未勾選移除項目: [{fname}]"), "info");
                continue;
            }

            // 3. 清理殘留檔案或資料夾
            if item_path.is_file() {
                let _ = crate::system::remove_readonly(&item_path);
                if fs::remove_file(&item_path).is_ok() {
                    log_fn(&format!("  🗑️ 已清理根目錄殘留檔案: [{fname}]"), "info");
                }
            } else if item_path.is_dir() {
                let (ok, msg) = robust_rmtree(&item_path, log_fn);
                if ok {
                    log_fn(&format!("  🗑️ 已清理根目錄殘留資料夾: [{fname}]"), "info");
                } else {
                    log_fn(&format!("  ⚠️ 清理資料夾 [{fname}] 失敗: {msg}"), "warning");
                }
            }
        }
    }

    // 檢查清理後的剩餘檔案
    if let Ok(entries) = fs::read_dir(root_dir) {
        let remaining: Vec<String> = entries
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        if remaining.is_empty() {
            let _ = fs::remove_dir(root_dir);
            log_fn(&format!("  ✓ 安裝根目錄已完全清空，已安全移除資料夾 [{}]", root_dir.display()), "success");
        } else if remaining.iter().all(|f| f.starts_with("_LAH_Backup_") && f.ends_with(".zip")) {
            log_fn(&format!("  🎉 安裝根目錄其餘項目已全數清除，僅保留必要備份檔: [{}] 於 [{}]", remaining.join(", "), root_dir.display()), "success");
        } else {
            log_fn(&format!("  ℹ️ 安裝根目錄已完成清理，保留之項目 ({} 個): {}", remaining.len(), remaining.join(", ")), "info");
        }
    }
}
