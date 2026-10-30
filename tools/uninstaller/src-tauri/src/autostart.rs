use crate::system::{is_admin, run_cmd_hidden};
use std::env;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};
use winreg::RegKey;

pub fn clean_autostart_and_scheduler(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    log_fn("⏳ 正在清除開機自啟動腳本、工作排程器與註冊表...", "info");

    // 1. 清除工作排程 (涵蓋 LAH_PM2_Autostart, PM2_Autostart, LAH_Autostart)
    for tn in &["LAH_PM2_Autostart", "PM2_Autostart", "LAH_Autostart"] {
        let (ok, _, _) = run_cmd_hidden("schtasks.exe", &["/Delete", "/TN", tn, "/F"]);
        if ok {
            log_fn(&format!("  ✓ 已自 Windows 工作排程器刪除: {tn}"), "success");
        }
    }

    // 2. 清除啟動資料夾 (APPDATA, ProgramData, ALLUSERSPROFILE)
    let mut startup_dirs: Vec<PathBuf> = Vec::new();
    if let Ok(appdata) = env::var("APPDATA") {
        startup_dirs.push(Path::new(&appdata).join(r"Microsoft\Windows\Start Menu\Programs\Startup"));
    }
    if let Ok(programdata) = env::var("ProgramData") {
        startup_dirs.push(Path::new(&programdata).join(r"Microsoft\Windows\Start Menu\Programs\Startup"));
    }
    if let Ok(allusers) = env::var("ALLUSERSPROFILE") {
        startup_dirs.push(Path::new(&allusers).join(r"Microsoft\Windows\Start Menu\Programs\Startup"));
    }

    let mut seen = std::collections::HashSet::new();
    for sdir in startup_dirs {
        let norm = sdir.to_string_lossy().to_lowercase();
        if !seen.insert(norm) || !sdir.is_dir() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(&sdir) {
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_lowercase();
                let is_target = fname.contains("pm2") || fname.contains("lah") || fname.contains("appserv");
                let is_script = fname.ends_with(".vbs")
                    || fname.ends_with(".bat")
                    || fname.ends_with(".cmd")
                    || fname.ends_with(".lnk")
                    || fname.ends_with(".ps1");

                if is_target && is_script {
                    let path = entry.path();
                    let _ = crate::system::remove_readonly(&path);
                    if fs::remove_file(&path).is_ok() {
                        log_fn(&format!("  ✓ 已清理啟動資料夾腳本/捷徑: {}", path.display()), "success");
                    }
                }
            }
        }
    }

    // 3. 清理各處舊版 pm2-autostart.vbs 獨立檔案
    let mut standalone_scripts = Vec::new();
    if let Ok(userprofile) = env::var("USERPROFILE") {
        standalone_scripts.push(Path::new(&userprofile).join(r".pm2\pm2-autostart.vbs"));
    }
    standalone_scripts.push(PathBuf::from(r"C:\Windows\System32\config\systemprofile\.pm2\pm2-autostart.vbs"));
    standalone_scripts.push(root_dir.join("node").join("pm2-autostart.vbs"));

    for sp in standalone_scripts {
        if sp.is_file() {
            let _ = crate::system::remove_readonly(&sp);
            if fs::remove_file(&sp).is_ok() {
                log_fn(&format!("  ✓ 已清理獨立喚醒腳本: {}", sp.display()), "success");
            }
        }
    }

    // 4. 清理註冊表 Run 自啟動鍵值
    clean_registry_run(HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Run", root_dir, log_fn);
    if is_admin() {
        clean_registry_run(HKEY_LOCAL_MACHINE, r"Software\Microsoft\Windows\CurrentVersion\Run", root_dir, log_fn);
    }

    true
}

fn clean_registry_run(hkey: winreg::HKEY, subkey: &str, root_dir: &Path, log_fn: &dyn Fn(&str, &str)) {
    let root_key = RegKey::predef(hkey);
    if let Ok(run_key) = root_key.open_subkey_with_flags(subkey, KEY_READ | KEY_WRITE) {
        let norm_root = root_dir.to_string_lossy().to_lowercase();
        let mut to_delete = Vec::new();

        for item in run_key.enum_values().flatten() {
            let (name, value) = item;
            let name_lower = name.to_lowercase();
            let val_str = value.to_string().to_lowercase();

            if name_lower.contains("pm2") || name_lower.contains("lah") || (!norm_root.is_empty() && val_str.contains(&norm_root)) {
                to_delete.push(name);
            }
        }

        for name in to_delete {
            if run_key.delete_value(&name).is_ok() {
                log_fn(&format!("  ✓ 已清理註冊表 Run 自啟動項目: [{name}]"), "success");
            }
        }
    }
}

pub fn remove_paths_from_environment(paths_to_remove: &[PathBuf], log_fn: &dyn Fn(&str, &str)) -> bool {
    let norm_targets: Vec<String> = paths_to_remove
        .iter()
        .map(|p| p.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase())
        .collect();

    let mut removed_any = false;

    // 1. 清理使用者 PATH (HKCU\Environment) 與 PM2 相關環境變數
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(env_key) = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE) {
        if !norm_targets.is_empty() {
            if let Ok(cur_val) = env_key.get_value::<String, _>("Path") {
                let parts: Vec<&str> = cur_val.split(';').filter(|s| !s.trim().is_empty()).collect();
                let mut new_parts = Vec::new();

                for p in parts {
                    let p_norm = p.trim().trim_end_matches(['\\', '/']).to_lowercase();
                    if norm_targets.contains(&p_norm) {
                        removed_any = true;
                        log_fn(&format!("  ✓ 自使用者 PATH 環境變數剔除: [{p}]"), "info");
                    } else {
                        new_parts.push(p.trim());
                    }
                }

                if removed_any {
                    let new_val = new_parts.join(";");
                    let _ = env_key.set_value("Path", &new_val);
                }
            }
        }

        // 徹底清理使用者層級 PM2_HOME 與歷史變數
        for var_name in &["PM2_HOME", "PM2_INSTALL_DIRECTORY", "PM2_SERVICE_DIRECTORY"] {
            if env_key.get_value::<String, _>(var_name).is_ok() {
                let _ = env_key.delete_value(var_name);
                removed_any = true;
                log_fn(&format!("  ✓ 自使用者環境變數 (HKCU) 清理殘留變數: [{var_name}]"), "info");
            }
        }
    }

    // 2. 若為系統管理員，清理系統 PATH (HKLM\...\Environment) 與 PM2 相關環境變數
    if is_admin() {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let hklm_sub = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
        if let Ok(env_key) = hklm.open_subkey_with_flags(hklm_sub, KEY_READ | KEY_WRITE) {
            if !norm_targets.is_empty() {
                if let Ok(cur_val) = env_key.get_value::<String, _>("Path") {
                    let parts: Vec<&str> = cur_val.split(';').filter(|s| !s.trim().is_empty()).collect();
                    let mut new_parts = Vec::new();
                    let mut sys_removed = false;

                    for p in parts {
                        let p_norm = p.trim().trim_end_matches(['\\', '/']).to_lowercase();
                        if norm_targets.contains(&p_norm) {
                            sys_removed = true;
                            removed_any = true;
                            log_fn(&format!("  ✓ 自系統 PATH 環境變數剔除: [{p}]"), "info");
                        } else {
                            new_parts.push(p.trim());
                        }
                    }

                    if sys_removed {
                        let new_val = new_parts.join(";");
                        let _ = env_key.set_value("Path", &new_val);
                    }
                }
            }

            // 徹底清理系統層級 PM2_HOME 與歷史變數
            for var_name in &["PM2_HOME", "PM2_INSTALL_DIRECTORY", "PM2_SERVICE_DIRECTORY"] {
                if env_key.get_value::<String, _>(var_name).is_ok() {
                    let _ = env_key.delete_value(var_name);
                    removed_any = true;
                    log_fn(&format!("  ✓ 自系統環境變數 (HKLM) 清理殘留變數: [{var_name}]"), "info");
                }
            }
        }
    }

    // 3. 廣播系統環境變數更新通知
    if removed_any {
        broadcast_setting_change();
        log_fn("  ✅ 已廣播通知 Windows 系統環境變數更新完成！", "success");
    }

    true
}

fn broadcast_setting_change() {
    unsafe {
        let wide_str: Vec<u16> = std::ffi::OsStr::new("Environment")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut res: usize = 0;
        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageTimeoutW(
            -1 as isize as _, // HWND_BROADCAST
            windows_sys::Win32::UI::WindowsAndMessaging::WM_SETTINGCHANGE,
            0,
            wide_str.as_ptr() as isize,
            windows_sys::Win32::UI::WindowsAndMessaging::SMTO_ABORTIFHUNG,
            5000,
            &mut res,
        );
    }
}
