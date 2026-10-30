use std::env;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};
use winreg::RegKey;

pub const CREATE_NO_WINDOW: u32 = 0x08000000;
const HKLM_ENV_PATH: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";

pub fn is_admin() -> bool {
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
}

pub fn get_app_dir() -> PathBuf {
    if let Ok(mut exe) = env::current_exe() {
        exe.pop();
        return exe;
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn is_valid_lah_or_node_root(dir: &Path) -> bool {
    dir.join("node").join("node.exe").is_file()
        || dir.join("Apache24").is_dir()
        || dir.join("www").is_dir()
        || dir.join("LAH-API").is_dir()
        || dir.join("LAH-NUXTJS").is_dir()
}

pub fn extract_exe_from_image_path(img_path: &str) -> Option<PathBuf> {
    let trimmed = img_path.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(PathBuf::from(&rest[..end]));
    }
    let lower = trimmed.to_lowercase();
    if let Some(idx) = lower.find(".exe") {
        return Some(PathBuf::from(&trimmed[..idx + 4]));
    }
    Some(PathBuf::from(trimmed.split_whitespace().next()?))
}

pub fn detect_default_root_dir() -> PathBuf {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. 優先由已註冊之 LAH_Web_Service / LAH_PM2_Service / PM2_Portable_Service / Apache2.4 ImagePath 反推根目錄
    for svc_name in ["LAH_Web_Service", "LAH_PM2_Service", "PM2_Portable_Service", "Apache2.4", "Apache24"] {
        let subkey = format!(r"SYSTEM\CurrentControlSet\Services\{svc_name}");
        if let Ok(svc_key) = hklm.open_subkey_with_flags(&subkey, KEY_READ) {
            if let Ok(img_path) = svc_key.get_value::<String, _>("ImagePath") {
                if let Some(exe_p) = extract_exe_from_image_path(&img_path) {
                    if let Some(root) = exe_p.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
                        if is_valid_lah_or_node_root(root) {
                            return root.to_path_buf();
                        }
                    }
                }
            }
        }
    }

    // 2. 由系統或使用者環境變數 PM2_HOME (<root>\.pm2) 反推根目錄
    let pm2_home_candidates = [
        hklm.open_subkey_with_flags(HKLM_ENV_PATH, KEY_READ)
            .ok()
            .and_then(|k| k.get_value::<String, _>("PM2_HOME").ok()),
        hkcu.open_subkey_with_flags("Environment", KEY_READ)
            .ok()
            .and_then(|k| k.get_value::<String, _>("PM2_HOME").ok()),
    ];
    for cand in pm2_home_candidates.into_iter().flatten() {
        let p = PathBuf::from(cand.trim().trim_end_matches(['\\', '/']));
        if p.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.eq_ignore_ascii_case(".pm2"))
            .unwrap_or(false)
        {
            if let Some(root) = p.parent() {
                if is_valid_lah_or_node_root(root) {
                    return root.to_path_buf();
                }
            }
        }
    }

    // 3. 檢查執行檔所在目錄或其父目錄是否已安裝系統元件
    let app_dir = get_app_dir();
    if is_valid_lah_or_node_root(&app_dir) {
        return app_dir;
    }
    if let Some(parent) = app_dir.parent() {
        if parent.parent().is_some() && is_valid_lah_or_node_root(parent) {
            return parent.to_path_buf();
        }
    }

    // 4. 預設回退至 C:\AppServ
    PathBuf::from(r"C:\AppServ")
}

pub fn run_cmd_hidden(cmd: &str, args: &[&str]) -> (bool, String, String) {
    let output = Command::new(cmd)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(out) => {
            let stdout = decode_console_bytes(&out.stdout);
            let stderr = decode_console_bytes(&out.stderr);
            (out.status.success(), stdout, stderr)
        }
        Err(e) => (false, String::new(), e.to_string()),
    }
}

pub fn run_powershell(script: &str) -> (bool, String, String) {
    run_cmd_hidden("powershell", &["-NoProfile", "-Command", script])
}

pub fn decode_console_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    String::from_utf8_lossy(bytes).to_string()
}

pub fn ensure_root_dir(path: &Path) -> Result<(), String> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(|e| format!("建立目錄失敗: {}", e))?;
    }
    let test_file = path.join(".lah_write_test");
    fs::write(&test_file, b"test").map_err(|e| format!("目錄無法寫入: {}", e))?;
    let _ = fs::remove_file(&test_file);
    Ok(())
}

pub fn is_legacy_global_node_or_pm2_path(p_norm_lower: &str) -> bool {
    matches!(
        p_norm_lower,
        r"c:\program files\nodejs"
            | r"c:\program files (x86)\nodejs"
            | r"c:\programdata\npm\npm"
            | r"c:\programdata\npm"
            | r"c:\programdata\pm2"
    ) || p_norm_lower.ends_with(r"\appdata\roaming\npm")
}

/// 將目標目錄前置於 PATH 字串，並在沙盒內部淨化全域干擾（用於 dump.pm2 內部環境或當前進程內部）
pub fn clean_and_prepend_path_string(raw_path: &str, target_norm: &str) -> (String, Vec<String>) {
    let lower_target = target_norm.trim_end_matches(['\\', '/']).to_lowercase();
    let mut removed = Vec::new();
    let mut kept = vec![target_norm.trim_end_matches(['\\', '/']).to_string()];

    for part in raw_path.split(';') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let norm_lower = trimmed.trim_end_matches(['\\', '/']).to_lowercase();
        if norm_lower == lower_target {
            continue;
        }
        if is_legacy_global_node_or_pm2_path(&norm_lower) {
            removed.push(trimmed.to_string());
            continue;
        }
        if !kept
            .iter()
            .any(|k| k.trim_end_matches(['\\', '/']).eq_ignore_ascii_case(&norm_lower))
        {
            kept.push(trimmed.to_string());
        }
    }

    (kept.join(";"), removed)
}

/// 自系統/使用者 PATH 字串中拔除指定目標路徑（用於自癒清除過往寫入之綠色路徑）
pub fn purge_target_from_path_string(raw_path: &str, target_norm: &str) -> (String, Vec<String>) {
    let lower_target = target_norm.trim_end_matches(['\\', '/']).to_lowercase();
    let mut removed = Vec::new();
    let mut kept: Vec<String> = Vec::new();

    for part in raw_path.split(';') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let norm_lower = trimmed.trim_end_matches(['\\', '/']).to_lowercase();
        if norm_lower == lower_target {
            removed.push(trimmed.to_string());
            continue;
        }
        if !kept
            .iter()
            .any(|k| k.trim_end_matches(['\\', '/']).eq_ignore_ascii_case(&norm_lower))
        {
            kept.push(trimmed.to_string());
        }
    }

    (kept.join(";"), removed)
}

/// 自動產生專屬沙盒終端批次檔 lah-cmd.bat（零全域污染，隨點即用）
pub fn ensure_portable_cmd_helper(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) {
    let bat_path = root_dir.join("lah-cmd.bat");
    let content = "@echo off\r\n\
chcp 65001 >nul\r\n\
setlocal\r\n\
set \"ROOT_DIR=%~dp0\"\r\n\
set \"ROOT_DIR=%ROOT_DIR:~0,-1%\"\r\n\
set \"NODE_DIR=%ROOT_DIR%\\node\"\r\n\
set \"APACHE_DIR=%ROOT_DIR%\\Apache24\"\r\n\
set \"OCI_DIR=%ROOT_DIR%\\instantclient_11_2\"\r\n\
set \"PM2_HOME=%ROOT_DIR%\\.pm2\"\r\n\
set \"PATH=%NODE_DIR%;%APACHE_DIR%\\bin;%OCI_DIR%;%PATH%\"\r\n\
title LAH 智慧控管專屬終端 (Node 16 / PM2 / Apache 2.4 沙盒環境)\r\n\
echo ============================================================\r\n\
echo   LAH 智慧控管專屬維運終端 (沙盒環境 - 零全域污染)\r\n\
echo   - 隔離 Node 目錄  : %NODE_DIR%\r\n\
echo   - 隔離 Apache 目錄: %APACHE_DIR%\r\n\
echo   - 隔離 PM2 目錄   : %PM2_HOME%\r\n\
echo   - 全域環境不受任何干擾 (關閉視窗即自動釋放)\r\n\
echo ============================================================\r\n\
call node -v\r\n\
call \"%NODE_DIR%\\pm2.cmd\" -v\r\n\
echo ------------------------------------------------------------\r\n\
echo 提示:\r\n\
echo   - Node/PM2: pm2 status / pm2 logs / pm2 restart all\r\n\
echo   - Apache   : httpd -t (語法檢查) / sc query LAH_Web_Service\r\n\
echo ============================================================\r\n\
cmd /k\r\n";

    if let Err(e) = fs::write(&bat_path, content) {
        log_fn(&format!("⚠️ 建立 lah-cmd.bat 失敗: {e}"), "warning");
    } else {
        log_fn("  ✓ 已就緒專屬沙盒維運終端批次檔 (lah-cmd.bat)", "success");
    }
}

/// 自系統 (HKLM) 與使用者 (HKCU) 登錄檔中還原乾淨環境，移除可攜版路徑污染
pub fn purge_portable_from_registry(target_dir: &Path, log_fn: &dyn Fn(&str, &str)) {
    let target_norm = target_dir
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string();

    let mut changed_any = false;

    // 1. 清理 HKLM PATH 與 PM2_HOME
    if is_admin() {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(sys_key) = hklm.open_subkey_with_flags(HKLM_ENV_PATH, KEY_READ | KEY_WRITE) {
            if let Ok(cur_sys) = sys_key.get_value::<String, _>("Path") {
                let (new_sys, removed_sys) = purge_target_from_path_string(&cur_sys, &target_norm);
                if !removed_sys.is_empty() {
                    let _ = sys_key.set_value("Path", &new_sys);
                    changed_any = true;
                    log_fn(
                        &format!("  ✓ 已自系統 PATH (HKLM) 移除可攜路徑 [{target_norm}]，確保全域 Node 獨立性"),
                        "info",
                    );
                }
            }
            if sys_key.get_value::<String, _>("PM2_HOME").is_ok() {
                let _ = sys_key.delete_value("PM2_HOME");
                changed_any = true;
                log_fn("  ✓ 已自系統環境變數 (HKLM) 清理 PM2_HOME，避免污染一般終端", "info");
            }
        }
    }

    // 2. 清理 HKCU PATH 與 PM2_HOME
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(usr_key) = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE) {
        if let Ok(cur_usr) = usr_key.get_value::<String, _>("Path") {
            let (new_usr, removed_usr) = purge_target_from_path_string(&cur_usr, &target_norm);
            if !removed_usr.is_empty() {
                let _ = usr_key.set_value("Path", &new_usr);
                changed_any = true;
                log_fn(
                    &format!("  ✓ 已自使用者 PATH (HKCU) 移除可攜路徑 [{target_norm}]，確保全域 Node 獨立性"),
                    "info",
                );
            }
        }
        if usr_key.get_value::<String, _>("PM2_HOME").is_ok() {
            let _ = usr_key.delete_value("PM2_HOME");
            changed_any = true;
            log_fn("  ✓ 已自使用者環境變數 (HKCU) 清理 PM2_HOME，避免污染一般終端", "info");
        }
    }

    if changed_any {
        broadcast_setting_change();
    }
}

/// 綠色可攜環境設定（零全域污染模式）：
/// 1. 主動自登錄檔清理可能殘留的綠色路徑，恢復全域 Node 獨立性。
/// 2. 僅於當前行程 (Process) PATH 注入 target_dir，供本次安裝/更新程序使用。
/// 3. 自動生成專屬沙盒維運終端 lah-cmd.bat。
pub fn register_path_to_user_environment(target_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    if !target_dir.is_dir() {
        log_fn(
            &format!("⚠️ 目錄不存在，略過設定 PATH: {}", target_dir.display()),
            "warning",
        );
        return false;
    }

    let target_norm = target_dir
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string();

    // 1. 自癒：自登錄檔中還原全域環境（移除過往版本可能寫入之綠色版路徑）
    purge_portable_from_registry(target_dir, log_fn);

    // 2. 僅於當前更新行程內更新 PATH，供本執行檔執行 node/pm2 命令使用，作業系統全域零污染
    let cur_proc = env::var("PATH").unwrap_or_default();
    let (new_proc, _) = clean_and_prepend_path_string(&cur_proc, &target_norm);
    env::set_var("PATH", new_proc);
    log_fn(
        &format!("✅ [零全域污染] 已將可攜環境限縮於當前行程與專屬沙盒 ({target_norm})"),
        "success",
    );

    // 3. 自動建立/更新專屬沙盒終端批次檔
    if let Some(parent) = target_dir.parent() {
        ensure_portable_cmd_helper(parent, log_fn);
    }

    true
}

/// 綠色可攜 PM2_HOME 設定（零全域污染模式）：
/// 僅作用於當前行程，不污染 HKLM/HKCU 登錄檔。
pub fn register_pm2_home_and_clean_legacy_env(pm2_home: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    let val = pm2_home.to_string_lossy().to_string();

    let legacy_vars = ["PM2_INSTALL_DIRECTORY", "PM2_SERVICE_DIRECTORY"];
    for lv in &legacy_vars {
        env::remove_var(lv);
    }
    env::set_var("PM2_HOME", &val);
    log_fn(
        &format!("✅ [零全域污染] 已設定當前行程 PM2_HOME={val}"),
        "success",
    );
    true
}

fn broadcast_setting_change() {
    unsafe {
        let env_wide: Vec<u16> = std::ffi::OsStr::new("Environment")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut result: usize = 0;
        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageTimeoutW(
            0xFFFF as _,
            0x001A,
            0,
            env_wide.as_ptr() as isize,
            0x0002,
            3000,
            &mut result as *mut usize as _,
        );
    }
}

fn find_dashboard_exe_direct(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    // 1. 精準優先候選檔名
    let primary_candidates = [
        "智慧控管與監控系統 - 管理儀表板.exe",
        "LAH服務監控儀表板.exe",
        "lah-dashboard.exe",
    ];
    for name in &primary_candidates {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }

    // 2. 廣泛匹配包含「儀表板」或「dashboard」之執行檔 (排除 updater/uninstaller/backuper/node)
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                    let lower = name.to_lowercase();
                    if lower.ends_with(".exe")
                        && (lower.contains("儀表板") || lower.contains("dashboard"))
                        && !lower.contains("updater")
                        && !lower.contains("更新")
                        && !lower.contains("uninstall")
                        && !lower.contains("移除")
                        && !lower.contains("backup")
                        && !lower.contains("備份")
                        && !lower.contains("node")
                    {
                        return Some(p);
                    }
                }
            }
        }
    }

    None
}

/// 尋找目錄下之管理儀表板執行檔 (優先同目錄，若開發環境則相容探測 dist 子目錄)
pub fn find_dashboard_exe_in_dir(dir: &Path) -> Option<PathBuf> {
    if let Some(p) = find_dashboard_exe_direct(dir) {
        return Some(p);
    }
    // 開發或排程測試容錯：若位於 src-tauri 或 target 目錄，回溯至 dist/ 探測
    if let Some(parent) = dir.parent() {
        let dist = parent.join("dist");
        if let Some(p) = find_dashboard_exe_direct(&dist) {
            return Some(p);
        }
        if let Some(grandparent) = parent.parent() {
            let dist2 = grandparent.join("dist");
            if let Some(p) = find_dashboard_exe_direct(&dist2) {
                return Some(p);
            }
        }
    }
    None
}

/// 透過 PowerShell COM 介面建立 Windows 桌面捷徑 (.lnk)
pub fn create_desktop_shortcut(
    target_exe: &Path,
    working_dir: &Path,
    shortcut_name: &str,
    description: &str,
) -> (bool, Vec<PathBuf>, String) {
    let target_str = target_exe.to_string_lossy().replace('\'', "''");
    let work_str = working_dir.to_string_lossy().replace('\'', "''");
    let name_str = shortcut_name.replace('\'', "''");
    let desc_str = description.replace('\'', "''");

    let ps_script = format!(
        r#"$ws = New-Object -ComObject WScript.Shell
$dirs = @(
    [Environment]::GetFolderPath('CommonDesktopDirectory'),
    [Environment]::GetFolderPath('Desktop')
) | Where-Object {{ $_ -and (Test-Path $_) }} | Select-Object -Unique

$created = @()
foreach ($d in $dirs) {{
    try {{
        $lnk = Join-Path $d '{name_str}'
        $s = $ws.CreateShortcut($lnk)
        $s.TargetPath = '{target_str}'
        $s.WorkingDirectory = '{work_str}'
        $s.Description = '{desc_str}'
        $s.IconLocation = '{target_str},0'
        $s.Save()
        $created += $lnk
    }} catch {{
        Write-Error $_
    }}
}}
$created -join ';'
"#
    );

    let (ok, stdout, stderr) = run_powershell(&ps_script);
    let mut paths = Vec::new();
    for p in stdout.trim().split(';') {
        let p_trimmed = p.trim();
        if !p_trimmed.is_empty() {
            paths.push(PathBuf::from(p_trimmed));
        }
    }
    (ok && !paths.is_empty(), paths, stderr)
}

/// 若同目錄下存在管理儀表板 EXE，複製至安裝根目錄並建立桌面捷徑
pub fn deploy_dashboard_if_found(
    app_dir: &Path,
    root_path: &Path,
    log_fn: &dyn Fn(&str, &str),
) -> (bool, Option<PathBuf>) {
    let Some(src_exe) = find_dashboard_exe_in_dir(app_dir) else {
        return (false, None);
    };

    let filename = src_exe
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("智慧控管與監控系統 - 管理儀表板.exe");
    let dest_exe = root_path.join(filename);

    log_fn("--------------------------------------------------", "info");
    log_fn(&format!("📊 偵測到同目錄下存在管理儀表板：{}", src_exe.display()), "info");

    // 若來源與目標不同，進行複製覆蓋
    if src_exe != dest_exe {
        let copy_result = if dest_exe.is_file() {
            match fs::copy(&src_exe, &dest_exe) {
                Ok(_) => Ok(()),
                Err(e) => {
                    // 若目標被執行中進程鎖定，嘗試將舊檔移至 .old 後覆蓋
                    let old_path = dest_exe.with_extension("exe.old");
                    let _ = fs::remove_file(&old_path);
                    if fs::rename(&dest_exe, &old_path).is_ok() {
                        fs::copy(&src_exe, &dest_exe)
                            .map(|_| ())
                            .map_err(|e2| format!("更名後複製仍失敗: {e2}"))
                    } else {
                        Err(format!("覆蓋複製失敗 (目標可能遭鎖定): {e}"))
                    }
                }
            }
        } else {
            fs::copy(&src_exe, &dest_exe)
                .map(|_| ())
                .map_err(|e| format!("複製失敗: {e}"))
        };

        match copy_result {
            Ok(_) => {
                log_fn(
                    &format!(
                        "  ✓ 已同步複製管理儀表板至安裝根目錄：{}",
                        dest_exe.display()
                    ),
                    "success",
                );
            }
            Err(err) => {
                log_fn(&format!("  ⚠️ 同步管理儀表板失敗: {err}"), "warning");
                return (false, None);
            }
        }
    } else {
        log_fn(
            &format!(
                "  ℹ️ 管理儀表板已位於安裝根目錄：{}",
                dest_exe.display()
            ),
            "info",
        );
    }

    // 建立 Windows 桌面捷徑
    let shortcut_stem = dest_exe
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("智慧控管與監控系統 - 管理儀表板");
    let shortcut_name = format!("{shortcut_stem}.lnk");

    let (shortcut_ok, created_paths, shortcut_err) = create_desktop_shortcut(
        &dest_exe,
        root_path,
        &shortcut_name,
        "智慧控管與監控系統 - 管理儀表板",
    );

    if shortcut_ok && !created_paths.is_empty() {
        for cp in created_paths {
            log_fn(
                &format!("  ✓ 已建立管理儀表板桌面捷徑：{}", cp.display()),
                "success",
            );
        }
    } else {
        log_fn(
            &format!("  ⚠️ 建立桌面捷徑時發生警告: {shortcut_err}"),
            "warning",
        );
    }

    (true, Some(dest_exe))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_dashboard_exe_in_dir() {
        let temp_dir = env::temp_dir().join("test_lah_dashboard_search");
        let _ = fs::create_dir_all(&temp_dir);
        let target_exe = temp_dir.join("智慧控管與監控系統 - 管理儀表板.exe");
        let _ = fs::write(&target_exe, b"dummy exe content");

        let found = find_dashboard_exe_in_dir(&temp_dir);
        assert_eq!(found, Some(target_exe.clone()));

        let _ = fs::remove_file(&target_exe);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_deploy_dashboard_if_found() {
        let temp_src = env::temp_dir().join("test_lah_dash_src");
        let temp_tgt = env::temp_dir().join("test_lah_dash_tgt");
        let _ = fs::create_dir_all(&temp_src);
        let _ = fs::create_dir_all(&temp_tgt);

        let src_exe = temp_src.join("智慧控管與監控系統 - 管理儀表板.exe");
        let _ = fs::write(&src_exe, b"test dashboard binary content");

        let (ok, out_path) = deploy_dashboard_if_found(&temp_src, &temp_tgt, &|_, _| {});
        assert!(ok);
        assert_eq!(out_path, Some(temp_tgt.join("智慧控管與監控系統 - 管理儀表板.exe")));
        assert!(temp_tgt.join("智慧控管與監控系統 - 管理儀表板.exe").is_file());

        // 清理測試檔案與捷徑
        let _ = fs::remove_dir_all(&temp_src);
        let _ = fs::remove_dir_all(&temp_tgt);
        let shortcut_name = "智慧控管與監控系統 - 管理儀表板.lnk";
        if let Ok(user_profile) = env::var("USERPROFILE") {
            let u = PathBuf::from(user_profile);
            let _ = fs::remove_file(u.join("Desktop").join(shortcut_name));
            let _ = fs::remove_file(u.join("OneDrive").join("Desktop").join(shortcut_name));
        }
        let _ = fs::remove_file(PathBuf::from(r"C:\Users\Public\Desktop").join(shortcut_name));
    }

    #[test]
    fn test_clean_and_prepend_path_replaces_global_node_and_pm2() {
        let raw = r"C:\Windows\system32;C:\ProgramData\npm\npm;C:\Program Files\nodejs\;D:\DEV\node\;C:\Users\administrator.HA\AppData\Roaming\npm";
        let (cleaned, removed) = clean_and_prepend_path_string(raw, r"D:\DEV\node");
        assert_eq!(cleaned, r"D:\DEV\node;C:\Windows\system32");
        assert_eq!(removed.len(), 3);
    }

    #[test]
    fn test_purge_target_from_path_preserves_global_node() {
        let raw = r"C:\Windows\system32;C:\Program Files\nodejs\;D:\DEV\node\;C:\Users\administrator.HA\AppData\Roaming\npm";
        let (cleaned, removed) = purge_target_from_path_string(raw, r"D:\DEV\node");
        // 保留全域 Node (C:\Program Files\nodejs) 與使用者 npm，僅拔除可攜版 D:\DEV\node
        assert_eq!(
            cleaned,
            r"C:\Windows\system32;C:\Program Files\nodejs\;C:\Users\administrator.HA\AppData\Roaming\npm"
        );
        assert_eq!(removed, vec![r"D:\DEV\node\"]);
    }

    #[test]
    fn test_extract_exe_from_image_path() {
        assert_eq!(
            extract_exe_from_image_path(r#""D:\DEV\Apache24\bin\httpd.exe" -k runservice"#),
            Some(PathBuf::from(r"D:\DEV\Apache24\bin\httpd.exe"))
        );
        assert_eq!(
            extract_exe_from_image_path(r"D:\DEV\node\service\lah-service.exe"),
            Some(PathBuf::from(r"D:\DEV\node\service\lah-service.exe"))
        );
    }
}
