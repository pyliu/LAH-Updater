use crate::apache_conf::*;
use crate::extractor::{
    auto_flatten_nested_dirs, detect_strip_prefix, ensure_default_env_files, extract_zip,
    restore_from_backup_zip, ALL_PACKAGE_SPECS,
};
use crate::health::verify_all_services_health;
use crate::pm2::*;
use crate::services::*;
use crate::system::*;
use crate::types::{ProgressEvent, SystemConfigOptions, UpdateOptions, UpdateSummary};
use std::path::PathBuf;
use std::time::Instant;

pub fn execute_update_pipeline<F>(options: UpdateOptions, emit_fn: F) -> UpdateSummary
where
    F: Fn(ProgressEvent) + Send + Sync,
{
    let start_time = Instant::now();
    let root_path = PathBuf::from(&options.root_dir);

    let log_event = |msg: &str, level: &str| {
        emit_fn(ProgressEvent {
            stage: "log".to_string(),
            percent: 0,
            message: msg.to_string(),
            level: level.to_string(),
        });
    };

    let set_prog = |percent: u32, msg: &str| {
        emit_fn(ProgressEvent {
            stage: "progress".to_string(),
            percent,
            message: msg.to_string(),
            level: "info".to_string(),
        });
    };

    log_event("============================================================", "info");
    log_event("🚀 開始執行自動更新作業...", "info");
    set_prog(0, "正在準備更新作業...");

    let mut selected_pkgs = options.packages.clone();
    // 基礎環境優先處理
    selected_pkgs.sort_by_key(|p| if p.is_base_env { 0 } else { 1 });

    let svc_name = query_apache_service().unwrap_or_else(|| "Apache2.4".to_string());
    let has_base_env_update = selected_pkgs.iter().any(|p| p.is_base_env);
    let has_apache_app_update = selected_pkgs.iter().any(|p| p.id == "www" || p.id == "lah_api");
    let should_stop_apache = options.restart_apache || has_base_env_update || has_apache_app_update;
    let mut apache_was_stopped = false;

    // 步驟 1：停止 Apache 防止檔案鎖定
    if should_stop_apache {
        if is_service_running(&svc_name) {
            set_prog(2, &format!("正在停止 Apache 服務 [{svc_name}]..."));
            apache_was_stopped = stop_service(&svc_name, &log_event);
        } else {
            log_event("ℹ️ 系統未偵測到運行中的 Apache 服務，跳過停止步驟。", "info");
        }
    }

    // 步驟 1.5：若有更新 node 執行期或 node_modules，進行防護釋放
    let has_node_runtime = selected_pkgs.iter().any(|p| p.id == "node");
    let has_node_modules = selected_pkgs.iter().any(|p| p.is_sub_package);
    if has_node_runtime || has_node_modules {
        set_prog(3, "正在檢查 Node.js 執行狀態並釋放檔案鎖定...");
        if is_pm2_service_running() {
            stop_service("LAH_PM2_Service", &log_event);
        }
        let node_exe = root_path.join("node").join("node.exe");
        let _ = kill_pm2_daemon(&root_path, &log_event);
        if is_file_locked(&node_exe) {
            log_event(&format!("⚠️ 警告：{} 仍處於鎖定狀態，若解壓失敗請確認行程已結束。", node_exe.display()), "warning");
        } else {
            log_event("✅ node.exe 檔案未受鎖定，可安全覆蓋解壓。", "info");
        }
    }

    // 步驟 2：逐一解壓更新包
    set_prog(5, "正在解壓縮安裝包檔案...");
    let total_pkgs = selected_pkgs.len();
    let mut total_files_extracted = 0;
    let mut zips_to_clean = Vec::new();

    for (idx, pkg) in selected_pkgs.iter().enumerate() {
        let zip_p = PathBuf::from(&pkg.zip_path);
        let tgt_p = PathBuf::from(&pkg.target_path);

        if !zip_p.is_file() {
            log_event(&format!("⚠️ 找不到安裝包檔案: {}", zip_p.display()), "warning");
            continue;
        }

        zips_to_clean.push(pkg.zip_path.clone());

        let pkg_num = idx + 1;
        log_event("--------------------------------------------------", "info");
        log_event(&format!("[{}/{}] 正在解壓縮：{} ➔ {}", pkg_num, total_pkgs, pkg.name, tgt_p.display()), "info");

        // 依據 PackageSpec 取得權威 expected_folder 與 alt_folders
        let spec_opt = ALL_PACKAGE_SPECS.iter().find(|s| s.id == pkg.id);
        let expected = spec_opt.map(|s| s.expected_folder).unwrap_or(&pkg.expected_folder);
        let alt_vec: Vec<&str> = spec_opt
            .map(|s| s.alt_folders.to_vec())
            .unwrap_or_else(|| pkg.alt_folders.iter().map(|s| s.as_str()).collect());

        let strip_prefix = detect_strip_prefix(&zip_p, expected, &alt_vec);
        if let Some(ref pfx) = strip_prefix {
            log_event(&format!("🛡️ [防呆觸發] 偵測到頂層包含 '{pfx}' 單一目錄，已自動對齊解壓至目標路徑！"), "warning");
        } else {
            log_event("ℹ️ 壓縮包頂層為直接檔案結構，依照目標目錄直接解壓縮覆蓋。", "info");
        }

        let base_percent = 5 + (idx as u32 * 75 / total_pkgs.max(1) as u32);
        let on_file = |rel_file: &str, current: usize, total: usize| {
            if current % 30 == 0 || current == total {
                let pkg_prog = (current as f32 / total.max(1) as f32 * (75.0 / total_pkgs.max(1) as f32)) as u32;
                set_prog(base_percent + pkg_prog, &format!("[{}/{}] {} ({}/{})", pkg_num, total_pkgs, rel_file, current, total));
            }
        };

        match extract_zip(&zip_p, &tgt_p, strip_prefix.as_deref(), &on_file) {
            Ok(count) => {
                total_files_extracted += count;
                log_event(&format!("✅ {} 解壓縮覆蓋完成 (處理 {} 個項目)。", pkg.name, count), "success");
            }
            Err(e) => {
                let err_msg = format!("❌ 解壓縮 {} 失敗: {}", pkg.name, e);
                log_event(&err_msg, "error");
                return UpdateSummary {
                    success: false,
                    elapsed_secs: start_time.elapsed().as_secs_f64(),
                    processed_packages: idx,
                    total_files: total_files_extracted,
                    health_report: Vec::new(),
                    zips_to_clean,
                    error_message: Some(err_msg),
                };
            }
        }
    }

    // 解壓後立即進行巢狀目錄防護檢查與自癒攤平
    auto_flatten_nested_dirs(&root_path, &log_event);

    // 確保各子專案 .env 基礎設定檔齊備
    ensure_default_env_files(&root_path, &log_event);

    // 步驟 3：備份檔還原
    if options.restore_backup {
        let b_path = PathBuf::from(&options.backup_zip_path);
        if b_path.is_file() {
            set_prog(85, "正在從備份包還原原始自訂設定...");
            let _ = restore_from_backup_zip(&b_path, &root_path, &log_event);
        }
    }

    let apache_tgt = root_path.join("Apache24");
    let php7_tgt = root_path.join("php7");
    let php85_tgt = root_path.join("php85");
    let node_tgt = root_path.join("node");

    // =========================================================================
    // 【階段 1/5】配置 PHP 7.4 與 Apache mod_php
    // =========================================================================
    if php7_tgt.is_dir() {
        set_prog(88, "[階段 1/5] 正在配置 PHP 7.4 設定檔 (php.ini)...");
        ensure_php_ini_configured(&php7_tgt, false, &log_event);
    }
    if apache_tgt.is_dir() {
        set_prog(89, "[階段 1/5] 正在配置 Apache 2.4 + PHP 7.4 與註冊服務...");
        configure_apache_mod_php7(&apache_tgt, &root_path, if php7_tgt.is_dir() { Some(&php7_tgt) } else { None }, &log_event);
        if options.auto_register_apache || has_base_env_update || query_apache_service().is_none() {
            install_apache_service(&apache_tgt, "Apache2.4", &log_event);
        }
    }

    // =========================================================================
    // 【階段 2/5】配置 PHP 8.5 與 FCGI Proxy
    // =========================================================================
    if php85_tgt.is_dir() || root_path.join("LAH-API").is_dir() {
        set_prog(91, "[階段 2/5] 正在配置 PHP 8.5 與 FCGI API Proxy (lah-api-v85)...");
        if php85_tgt.is_dir() {
            ensure_php_ini_configured(&php85_tgt, true, &log_event);
        }
        if apache_tgt.is_dir() {
            configure_apache_fcgi_proxy(&apache_tgt, &root_path, if php85_tgt.is_dir() { Some(&php85_tgt) } else { None }, &log_event);
        }
    }

    // =========================================================================
    // 【階段 3/5】資安弱點防護與 PHP2Shell 緩解
    // =========================================================================
    if options.auto_security_hardening {
        set_prog(93, "[階段 3/5] 正在套用資安防護與 PHP2Shell 緩解設定...");
        if apache_tgt.is_dir() {
            let conf_f = apache_tgt.join("conf").join("httpd.conf");
            if conf_f.is_file() {
                apply_apache_security_hardening(&conf_f, &root_path, &log_event);
            }
        }
        if php7_tgt.is_dir() {
            let ini7 = php7_tgt.join("php.ini");
            if ini7.is_file() {
                apply_php_security_hardening(&ini7, &log_event);
            }
        }
        if php85_tgt.is_dir() {
            let ini85 = php85_tgt.join("php.ini");
            if ini85.is_file() {
                apply_php_security_hardening(&ini85, &log_event);
            }
        }
    }
    if apache_tgt.is_dir() {
        check_apache_syntax(&apache_tgt, &log_event);
    }

    // =========================================================================
    // 【階段 4/5】綠色版 Node.js 設定與啟動 PM2
    // =========================================================================
    if node_tgt.is_dir() {
        set_prog(95, "[階段 4/5] 正在設定綠色版 Node 環境與 PM2 自動化服務...");
        let oci_dir = root_path.join("instantclient_11_2");
        if oci_dir.is_dir() {
            register_path_to_user_environment(&oci_dir, &log_event);
        }
        register_path_to_user_environment(&node_tgt, &log_event);
        register_pm2_home_and_clean_legacy_env(&root_path.join(".pm2"), &log_event);
        ensure_portable_pm2_patched(&root_path, &log_event);

        // 喚醒或重啟 PM2
        if options.restart_pm2 {
            restart_pm2_services(&root_path, &log_event);
        } else {
            let (res_ok, _) = resurrect_pm2_services(&root_path, &log_event);
            if !res_ok {
                restart_pm2_services(&root_path, &log_event);
            }
        }

        // 自動確認並配置 WinSW 服務 (LAH_PM2_Service)
        if options.auto_register_pm2 {
            setup_pm2_autostart(&root_path, &log_event);
        } else {
            log_event("  ℹ️ 未勾選註冊 PM2 系統服務，跳過 LAH_PM2_Service 註冊與自啟配置", "info");
        }

        // 全方位巡檢與補齊 PM2 核心服務 (LAH-WSS, LAH-FE, LAH-SCHEDULER, lah-api-v85, pm2-logrotate)
        audit_and_ensure_pm2_services(&root_path, &log_event);
    }

    // =========================================================================
    // 【階段 5/5】啟動 Apache 與健康巡檢
    // =========================================================================
    let target_svc = query_apache_service().unwrap_or_else(|| "Apache2.4".to_string());
    if is_service_running(&target_svc) {
        if should_stop_apache || apache_was_stopped {
            set_prog(98, &format!("[階段 5/5] 正在重新啟動 Apache 服務 [{target_svc}]..."));
            stop_service(&target_svc, &log_event);
            start_service(&target_svc, &log_event);
        }
    } else if options.restart_apache || has_base_env_update || has_apache_app_update {
        set_prog(98, &format!("[階段 5/5] 正在啟動 Apache 服務 [{target_svc}]..."));
        start_service(&target_svc, &log_event);
    }

    // 全方位服務健康確認
    set_prog(99, "正在進行全系統服務與通訊埠健康巡檢...");
    let (_, health_report) = verify_all_services_health(&root_path, if apache_tgt.is_dir() { Some(&apache_tgt) } else { None }, &log_event);

    let elapsed = start_time.elapsed().as_secs_f64();
    set_prog(100, &format!("✅ 全部安裝與服務啟動確認完成！(耗時 {:.1} 秒)", elapsed));
    log_event("============================================================", "success");
    log_event(&format!("🎉 全部選取的安裝包與 5 階段系統自動化配置已順利完成！(總耗時: {:.1} 秒)", elapsed), "success");

    UpdateSummary {
        success: true,
        elapsed_secs: elapsed,
        processed_packages: total_pkgs,
        total_files: total_files_extracted,
        health_report,
        zips_to_clean,
        error_message: None,
    }
}

pub fn apply_system_configurations<F>(options: SystemConfigOptions, emit_fn: F) -> UpdateSummary
where
    F: Fn(ProgressEvent) + Send + Sync,
{
    let start_time = Instant::now();
    let root_path = PathBuf::from(&options.root_dir);

    let log_event = |msg: &str, level: &str| {
        emit_fn(ProgressEvent {
            stage: "log".to_string(),
            percent: 0,
            message: msg.to_string(),
            level: level.to_string(),
        });
    };

    let set_prog = |percent: u32, msg: &str| {
        emit_fn(ProgressEvent {
            stage: "progress".to_string(),
            percent,
            message: msg.to_string(),
            level: "info".to_string(),
        });
    };

    log_event("============================================================", "info");
    log_event("⚡ 開始即時套用系統服務設定與資安強化配置...", "info");
    set_prog(0, "正在檢查系統環境...");

    let apache_tgt = root_path.join("Apache24");
    let php7_tgt = root_path.join("php7");
    let php85_tgt = root_path.join("php85");
    let node_tgt = root_path.join("node");

    // 1. 雙層巢狀目錄自癒攤平
    set_prog(10, "正在檢查目錄結構...");
    auto_flatten_nested_dirs(&root_path, &log_event);

    // 2. 確保環境檔存在
    ensure_default_env_files(&root_path, &log_event);

    // 3. 配置 PHP 7.4 與 Apache mod_php
    if php7_tgt.is_dir() {
        set_prog(25, "正在配置 PHP 7.4 (php.ini)...");
        ensure_php_ini_configured(&php7_tgt, false, &log_event);
    }
    if apache_tgt.is_dir() {
        set_prog(35, "正在配置 Apache 2.4 (httpd.conf) 與 mod_php7 模組...");
        configure_apache_mod_php7(&apache_tgt, &root_path, if php7_tgt.is_dir() { Some(&php7_tgt) } else { None }, &log_event);
        if options.auto_register_apache || query_apache_service().is_none() {
            set_prog(45, "正在註冊 Apache 系統服務...");
            install_apache_service(&apache_tgt, "Apache2.4", &log_event);
        }
    }

    // 4. 配置 PHP 8.5 與 FCGI Proxy
    if php85_tgt.is_dir() || root_path.join("LAH-API").is_dir() {
        set_prog(55, "正在配置 PHP 8.5 與 FCGI Proxy...");
        if php85_tgt.is_dir() {
            ensure_php_ini_configured(&php85_tgt, true, &log_event);
        }
        if apache_tgt.is_dir() {
            configure_apache_fcgi_proxy(&apache_tgt, &root_path, if php85_tgt.is_dir() { Some(&php85_tgt) } else { None }, &log_event);
        }
    }

    // 5. 套用資安強化
    if options.auto_security_hardening {
        set_prog(65, "正在套用資安強化 (隱藏版本標頭、CVE-2024-4577 緩解防護)...");
        if apache_tgt.is_dir() {
            let conf_f = apache_tgt.join("conf").join("httpd.conf");
            if conf_f.is_file() {
                apply_apache_security_hardening(&conf_f, &root_path, &log_event);
            }
        }
        if php7_tgt.is_dir() {
            let ini7 = php7_tgt.join("php.ini");
            if ini7.is_file() {
                apply_php_security_hardening(&ini7, &log_event);
            }
        }
        if php85_tgt.is_dir() {
            let ini85 = php85_tgt.join("php.ini");
            if ini85.is_file() {
                apply_php_security_hardening(&ini85, &log_event);
            }
        }
    }

    if apache_tgt.is_dir() {
        check_apache_syntax(&apache_tgt, &log_event);
    }

    // 6. Node.js 與 PM2 核心巡檢
    if node_tgt.is_dir() {
        set_prog(75, "正在巡檢與配置 PM2 核心服務...");
        let oci_dir = root_path.join("instantclient_11_2");
        if oci_dir.is_dir() {
            register_path_to_user_environment(&oci_dir, &log_event);
        }
        register_path_to_user_environment(&node_tgt, &log_event);
        register_pm2_home_and_clean_legacy_env(&root_path.join(".pm2"), &log_event);
        ensure_portable_pm2_patched(&root_path, &log_event);
        if options.auto_register_pm2 {
            setup_pm2_autostart(&root_path, &log_event);
        } else {
            log_event("  ℹ️ 未勾選註冊 PM2 系統服務，跳過 LAH_PM2_Service 註冊與自啟配置", "info");
        }
        audit_and_ensure_pm2_services(&root_path, &log_event);
        if options.restart_pm2 {
            restart_pm2_services(&root_path, &log_event);
        }
    }

    // 7. 重啟 Apache 服務 (若勾選)
    let target_svc = query_apache_service().unwrap_or_else(|| "Apache2.4".to_string());
    if options.restart_apache {
        set_prog(88, &format!("正在重啟 Apache 服務 [{target_svc}]..."));
        if is_service_running(&target_svc) {
            stop_service(&target_svc, &log_event);
        }
        start_service(&target_svc, &log_event);
    }

    // 8. 全方位服務健康確認
    set_prog(95, "正在進行系統服務健康確認...");
    let (_, health_report) = verify_all_services_health(
        &root_path,
        if apache_tgt.is_dir() { Some(&apache_tgt) } else { None },
        &log_event,
    );

    let elapsed = start_time.elapsed().as_secs_f64();
    set_prog(100, &format!("✅ 系統服務與資安配置套用完成！(耗時 {:.1} 秒)", elapsed));
    log_event("============================================================", "success");
    log_event(&format!("🎉 系統服務與資安強化配置已即時生效！(耗時 {:.1} 秒)", elapsed), "success");

    UpdateSummary {
        success: true,
        elapsed_secs: elapsed,
        processed_packages: 0,
        total_files: 0,
        health_report,
        zips_to_clean: Vec::new(),
        error_message: None,
    }
}
