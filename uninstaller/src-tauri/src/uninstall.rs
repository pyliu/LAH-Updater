use crate::apache_conf::clean_apache_php_configuration;
use crate::autostart::{clean_autostart_and_scheduler, remove_paths_from_environment};
use crate::cleaner::{clean_files_and_directories, clean_pm2_dumps, clean_root_directory};
use crate::process::kill_all_pm2_and_node;
use crate::services::{uninstall_apache_service, uninstall_lah_pm2_service};
use crate::types::{UninstallOptions, UninstallReport};
use std::path::PathBuf;
use std::time::Instant;

pub fn execute_uninstall_workflow(
    options: UninstallOptions,
    log_fn: impl Fn(&str, &str),
    progress_fn: impl Fn(f64, &str),
) -> UninstallReport {
    let start_time = Instant::now();
    let root_path = PathBuf::from(options.root_dir.trim());

    if !root_path.is_dir() {
        let msg = format!("指定的安裝根目錄不存在：{}", root_path.display());
        log_fn(&format!("❌ {msg}"), "error");
        return UninstallReport {
            success: false,
            message: msg,
            elapsed_secs: 0.0,
        };
    }

    log_fn(&"=".repeat(60), "info");
    log_fn(&format!("🚨 開始執行系統自動移除清理作業... (目標: {})", root_path.display()), "info");
    progress_fn(5.0, "正在準備移除作業...");

    // 階段 2：停止並解除安裝 Windows 服務
    if options.do_services {
        progress_fn(25.0, "正在停止並解除安裝 Windows 服務...");
        uninstall_lah_pm2_service(&root_path, &log_fn);
        if options.do_remove_apache {
            uninstall_apache_service(&root_path, &log_fn);
        }
    }

    // 若未移除 Apache，但移除了 PHP 7 或 PHP 8.5，自動清理 httpd.conf 中的轉發或載入設定
    if !options.do_remove_apache && (options.do_remove_php7 || options.do_remove_php85) {
        clean_apache_php_configuration(&root_path, options.do_remove_php7, options.do_remove_php85, &log_fn);
    }

    // 階段 3：終止 PM2 與背景殘留進程
    if options.do_kill_procs {
        progress_fn(40.0, "正在終止受控守護行程與殘留進程...");
        kill_all_pm2_and_node(&root_path, &log_fn);
    }

    // 階段 4：清除自啟動與環境變數
    if options.do_autostart {
        progress_fn(55.0, "正在清除開機自啟動與工作排程...");
        clean_autostart_and_scheduler(&root_path, &log_fn);

        let mut paths_to_remove = Vec::new();
        if options.do_remove_node {
            paths_to_remove.push(root_path.join("node"));
        }
        if options.do_remove_instantclient {
            paths_to_remove.push(root_path.join("instantclient_11_2"));
        }
        if options.do_remove_php85 {
            paths_to_remove.push(root_path.join("php85"));
        }
        if options.do_remove_php7 {
            paths_to_remove.push(root_path.join("php7"));
        }
        if options.do_remove_apache {
            paths_to_remove.push(root_path.join("Apache24").join("bin"));
        }

        if !paths_to_remove.is_empty() {
            remove_paths_from_environment(&paths_to_remove, &log_fn);
        }
    }

    // 階段 5：重置 PM2 快照
    if options.do_pm2_dumps {
        progress_fn(65.0, "正在清除 PM2 快照狀態檔...");
        clean_pm2_dumps(&root_path, &log_fn);
    }

    // 階段 6：刪除檔案與目錄
    progress_fn(75.0, "正在清理檔案與程式碼目錄...");
    clean_files_and_directories(&root_path, &options, &log_fn, &progress_fn);

    // 階段 6.5：清理安裝根目錄中的其餘殘留檔案與目錄（保留備份）
    if options.do_clean_root {
        progress_fn(92.0, "正在清理安裝根目錄其餘殘留檔案...");
        clean_root_directory(&root_path, &options, &log_fn);
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    progress_fn(100.0, &format!("✅ 全部選取項目已成功移除！(耗時 {elapsed:.1} 秒)"));
    log_fn(&"=".repeat(60), "success");
    log_fn(&format!("🎉 智慧控管與監控系統 - 自動移除作業順利完成！(耗時 {elapsed:.1} 秒)"), "success");

    UninstallReport {
        success: true,
        message: "系統移除清理作業已順利完成！".to_string(),
        elapsed_secs: elapsed,
    }
}
