use crate::pm2::{
    ensure_pm2_logrotate, ensure_portable_pm2_patched, is_logrotate_configured,
    is_logrotate_installed, kill_pm2_daemon, logrotate_bundled_version, resolve_pm2_command,
    resurrect_pm2_services, setup_pm2_service, stop_pm2_service,
};
use crate::system::{
    detect_default_root_dir, ensure_root_dir, get_app_dir, register_path_to_user_environment,
    register_pm2_home_and_clean_legacy_env, run_cmd_hidden,
};
use crate::types::{InstallOptions, InstallReport, ScanSummary};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

const ZIP_NAME: &str = "node.zip";

pub fn find_node_zip(root_dir: &Path) -> Option<PathBuf> {
    let app_dir = get_app_dir();
    let temp = PathBuf::from(r"C:\Temp");
    let dirs = [
        app_dir.join("base"),
        app_dir,
        temp.join("base"),
        temp,
        root_dir.join("base"),
        root_dir.to_path_buf(),
    ];
    dirs.iter().map(|d| d.join(ZIP_NAME)).find(|p| p.is_file())
}

pub fn node_version(root_dir: &Path) -> Option<String> {
    let node_exe = root_dir.join("node").join("node.exe");
    if !node_exe.is_file() {
        return None;
    }
    let (ok, out, _) = run_cmd_hidden(&node_exe.to_string_lossy(), &["-v"]);
    let v = out.trim();
    (ok && !v.is_empty()).then(|| v.to_string())
}

pub fn pm2_version(root_dir: &Path) -> Option<String> {
    let pkg = root_dir.join("node").join("node_modules").join("pm2").join("package.json");
    let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(pkg).ok()?).ok()?;
    json.get("version")?.as_str().map(|s| s.to_string())
}

pub fn scan(root_dir: &str) -> ScanSummary {
    let root = if root_dir.trim().is_empty() {
        detect_default_root_dir()
    } else {
        PathBuf::from(root_dir.trim())
    };
    ScanSummary {
        root_dir: root.to_string_lossy().to_string(),
        zip_found: find_node_zip(&root).map(|p| p.to_string_lossy().to_string()),
        node_version: node_version(&root),
        pm2_version: pm2_version(&root),
        logrotate_version: logrotate_bundled_version(&root),
        logrotate_installed: is_logrotate_installed(&root),
        logrotate_configured: is_logrotate_configured(&root),
        service_installed: crate::pm2::is_pm2_service_installed(),
        service_running: crate::pm2::is_pm2_service_running(),
    }
}

/// zip 內若所有項目皆位於頂層 `node/` 之下，則剝除該前綴，避免產生 node\node 雙層目錄。
fn detect_strip_node_prefix(archive: &mut zip::ZipArchive<fs::File>) -> bool {
    let mut any = false;
    for i in 0..archive.len() {
        let Ok(f) = archive.by_index_raw(i) else { return false };
        let name = f.name().replace('\\', "/");
        if name.is_empty() {
            continue;
        }
        any = true;
        if name != "node" && !name.starts_with("node/") {
            return false;
        }
    }
    any
}

fn extract_node_zip(
    zip_path: &Path,
    node_dir: &Path,
    progress_fn: &dyn Fn(f64, &str),
) -> Result<usize, String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("無法開啟 {}: {}", zip_path.display(), e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("壓縮檔損毀或格式不正確: {}", e))?;
    let strip = detect_strip_node_prefix(&mut archive);
    fs::create_dir_all(node_dir).map_err(|e| format!("建立 node 目錄失敗: {}", e))?;

    let total = archive.len();
    let mut written = 0usize;
    for i in 0..total {
        let mut entry = archive.by_index(i).map_err(|e| format!("讀取壓縮項目失敗: {}", e))?;
        let Some(rel) = entry.enclosed_name() else { continue };
        let mut comps = rel.components().peekable();
        if strip && matches!(comps.peek(), Some(Component::Normal(c)) if *c == "node") {
            comps.next();
        }
        let rel_path: PathBuf = comps.collect();
        if rel_path.as_os_str().is_empty() {
            continue;
        }
        let dest = node_dir.join(&rel_path);
        if entry.is_dir() {
            fs::create_dir_all(&dest).map_err(|e| format!("建立目錄 {} 失敗: {}", dest.display(), e))?;
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("建立目錄 {} 失敗: {}", parent.display(), e))?;
            }
            let mut out = fs::File::create(&dest)
                .map_err(|e| format!("寫入 {} 失敗 (可能被其他程序鎖定): {}", dest.display(), e))?;
            io::copy(&mut entry, &mut out).map_err(|e| format!("解壓 {} 失敗: {}", dest.display(), e))?;
            written += 1;
        }
        if i % 200 == 0 {
            progress_fn(15.0 + 55.0 * (i as f64 / total as f64), &format!("正在解壓 ({}/{})...", i, total));
        }
    }
    Ok(written)
}

pub fn execute_install(
    options: InstallOptions,
    log_fn: &dyn Fn(&str, &str),
    progress_fn: &dyn Fn(f64, &str),
) -> InstallReport {
    let started = Instant::now();
    let fail = |msg: String| -> InstallReport {
        log_fn(&format!("❌ {}", msg), "error");
        InstallReport { success: false, message: msg, elapsed_secs: started.elapsed().as_secs_f64() }
    };

    let root = PathBuf::from(options.root_dir.trim());
    if options.root_dir.trim().is_empty() || root.parent().is_none() {
        return fail("安裝目錄不可為空或磁碟機根目錄，請指定例如 C:\\AppServ 或 D:\\DEV".to_string());
    }

    progress_fn(2.0, "檢查安裝目錄與 node.zip...");
    if let Err(e) = ensure_root_dir(&root) {
        return fail(e);
    }
    let Some(zip_path) = find_node_zip(&root) else {
        return fail("找不到 node.zip，請放置於工具旁的 base 資料夾、工具同層或 C:\\Temp。".to_string());
    };
    log_fn(&format!("📦 使用安裝包: {}", zip_path.display()), "info");

    let node_dir = root.join("node");
    progress_fn(6.0, "停止既有 PM2 與舊全域服務...");
    stop_pm2_service(log_fn);
    let _ = kill_pm2_daemon(&root, log_fn);

    progress_fn(15.0, "正在解壓 Node.js 與 PM2...");
    match extract_node_zip(&zip_path, &node_dir, progress_fn) {
        Ok(n) => log_fn(&format!("  ✓ 已解壓 {} 個檔案至 {}", n, node_dir.display()), "success"),
        Err(e) => return fail(e),
    }

    progress_fn(72.0, "修補 PM2 綠色環境...");
    if !ensure_portable_pm2_patched(&root, log_fn) {
        return fail("PM2 修補失敗，node 目錄不存在".to_string());
    }

    progress_fn(80.0, "接管系統與使用者環境變數...");
    if options.register_path {
        register_path_to_user_environment(&node_dir, log_fn);
    }
    if options.set_pm2_home {
        register_pm2_home_and_clean_legacy_env(&root.join(".pm2"), log_fn);
    }

    progress_fn(86.0, "驗證安裝結果...");
    let Some(nv) = node_version(&root) else {
        return fail("node.exe 無法執行，請確認 node.zip 內容正確".to_string());
    };
    log_fn(&format!("  ✓ Node.js 版本 {}", nv), "success");
    if let Some(pm2) = resolve_pm2_command(&root) {
        let (ok, out, err) = run_cmd_hidden(&pm2.to_string_lossy(), &["-v"]);
        if ok {
            log_fn(&format!("  ✓ PM2 版本 {}", out.lines().last().unwrap_or("").trim()), "success");
        } else {
            return fail(format!("pm2 -v 執行失敗: {} {}", out, err));
        }
    } else {
        return fail("找不到 pm2.cmd，node.zip 可能未包含 PM2".to_string());
    }

    progress_fn(90.0, "還原既有 PM2 程序與啟用 pm2-logrotate...");
    if options.resurrect || options.register_service {
        resurrect_pm2_services(&root, log_fn);
    }
    if options.install_logrotate {
        ensure_pm2_logrotate(&root, log_fn);
    }

    progress_fn(96.0, "設定開機自啟服務...");
    if options.register_service && !setup_pm2_service(&root, log_fn) {
        return fail("開機自啟服務註冊失敗，請檢視上方日誌".to_string());
    }

    progress_fn(100.0, "安裝完成");
    log_fn("🎉 可攜 Node.js 與 PM2 安裝完成！新開啟的命令提示字元即可直接使用 node / pm2。", "success");
    InstallReport {
        success: true,
        message: format!("Node.js {} 安裝完成", nv),
        elapsed_secs: started.elapsed().as_secs_f64(),
    }
}
