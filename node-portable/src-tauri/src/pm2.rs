use crate::system::{clean_and_prepend_path_string, run_cmd_hidden, run_powershell};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

pub const SERVICE_ID: &str = "PM2_Portable_Service";
const LEGACY_PM2_SERVICES: &[&str] = &["pm2.exe", "PM2"];
const SERVICE_EXE: &[u8] = include_bytes!("../../service/pm2-service.exe");
const SERVICE_RUNNER_JS: &[u8] = include_bytes!("../../service/pm2_service_runner.js");

pub fn resolve_pm2_command(root_dir: &Path) -> Option<PathBuf> {
    let cand1 = root_dir.join("node").join("pm2.cmd");
    if cand1.is_file() {
        return Some(cand1);
    }
    let cand2 = root_dir.join("node").join("node_modules").join("pm2").join("bin").join("pm2");
    if cand2.is_file() {
        return Some(cand2);
    }
    None
}

pub fn logrotate_bundled_version(root_dir: &Path) -> Option<String> {
    let pkg = root_dir
        .join("node")
        .join("node_modules")
        .join("pm2-logrotate")
        .join("package.json");
    let raw = fs::read_to_string(pkg).ok()?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    json.get("version")?.as_str().map(|s| s.to_string())
}

pub fn is_logrotate_configured(root_dir: &Path) -> bool {
    let mod_dir = root_dir
        .join(".pm2")
        .join("modules")
        .join("pm2-logrotate")
        .join("node_modules")
        .join("pm2-logrotate");
    mod_dir.join("package.json").is_file() && mod_dir.join("app.js").is_file()
}

pub fn is_logrotate_installed(root_dir: &Path) -> bool {
    logrotate_bundled_version(root_dir).is_some() || is_logrotate_configured(root_dir)
}

pub fn wire_bundled_pm2_logrotate(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    let bundled_dir = root_dir.join("node").join("node_modules").join("pm2-logrotate");
    let src_pkg = bundled_dir.join("package.json");
    let src_app = bundled_dir.join("app.js");
    if !src_pkg.is_file() || !src_app.is_file() {
        return false;
    }

    let pm2_home = root_dir.join(".pm2");
    let target_mod_dir = pm2_home
        .join("modules")
        .join("pm2-logrotate")
        .join("node_modules")
        .join("pm2-logrotate");
    if fs::create_dir_all(&target_mod_dir).is_err() {
        return false;
    }

    let dst_pkg = target_mod_dir.join("package.json");
    let _ = fs::copy(&src_pkg, &dst_pkg);

    let dst_app = target_mod_dir.join("app.js");
    let src_app_json = serde_json::to_string(&src_app.to_string_lossy().to_string())
        .unwrap_or_else(|_| format!("\"{}\"", src_app.to_string_lossy().replace('\\', "\\\\")));
    let wrapper_js = format!("require({});\n", src_app_json);
    let mut changed = false;
    if fs::read_to_string(&dst_app).ok().as_deref() != Some(wrapper_js.as_str()) {
        if fs::write(&dst_app, &wrapper_js).is_ok() {
            changed = true;
        }
    }

    let conf_path = pm2_home.join("module_conf.json");
    let mut conf_val = fs::read_to_string(&conf_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}));

    if let Some(root_obj) = conf_val.as_object_mut() {
        let db_entry = root_obj
            .entry("module-db-v2".to_string())
            .or_insert_with(|| serde_json::json!({}));
        if !db_entry.is_object() {
            *db_entry = serde_json::json!({});
        }
        if let Some(db_obj) = db_entry.as_object_mut() {
            if !db_obj.contains_key("pm2-logrotate") {
                db_obj.insert(
                    "pm2-logrotate".to_string(),
                    serde_json::json!({ "uid": null, "gid": null }),
                );
                changed = true;
            }
        }

        let lr_entry = root_obj
            .entry("pm2-logrotate".to_string())
            .or_insert_with(|| serde_json::json!({}));
        if !lr_entry.is_object() {
            *lr_entry = serde_json::json!({});
        }
        if let Some(lr_obj) = lr_entry.as_object_mut() {
            let defaults = [
                ("max_size", serde_json::json!("10M")),
                ("retain", serde_json::json!("30")),
                ("dateFormat", serde_json::json!("YYYY-MM-DD_HH-mm-ss")),
                ("workerInterval", serde_json::json!("30")),
                ("rotateInterval", serde_json::json!("0 0 * * *")),
                ("rotateModule", serde_json::json!(true)),
            ];
            for (k, v) in defaults {
                if !lr_obj.contains_key(k) {
                    lr_obj.insert(k.to_string(), v);
                    changed = true;
                }
            }
            if lr_obj.get("compress") != Some(&serde_json::json!(true)) {
                lr_obj.insert("compress".to_string(), serde_json::json!(true));
                changed = true;
            }
        }
    }

    if changed {
        if let Ok(pretty) = serde_json::to_string_pretty(&conf_val) {
            if fs::write(&conf_path, pretty).is_ok() {
                log_fn(
                    &format!("  ✓ 已掛載便攜版內建模組 ({})", bundled_dir.display()),
                    "success",
                );
            }
        }
    }
    true
}

fn find_best_legacy_pm2_dir(unified_pm2_home: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(env_home) = std::env::var("PM2_HOME") {
        let p = PathBuf::from(env_home.trim());
        if p != unified_pm2_home {
            candidates.push(p);
        }
    }
    candidates.push(PathBuf::from(r"C:\ProgramData\pm2\home"));
    candidates.push(PathBuf::from(r"C:\ProgramData\pm2"));
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        candidates.push(PathBuf::from(user_profile).join(".pm2"));
    }
    if let Ok(entries) = fs::read_dir(r"C:\Users") {
        for entry in entries.flatten() {
            candidates.push(entry.path().join(".pm2"));
        }
    }

    candidates
        .into_iter()
        .filter(|d| d != unified_pm2_home && d.join("dump.pm2").is_file())
        .max_by_key(|d| {
            fs::metadata(d.join("dump.pm2"))
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        })
}

fn sanitize_env_map(
    map: &mut serde_json::Map<String, serde_json::Value>,
    node_dir_str: &str,
    pm2_home_str: &str,
) -> bool {
    let mut changed = false;
    for key in ["PM2_INSTALL_DIRECTORY", "PM2_SERVICE_DIRECTORY"] {
        if map.remove(key).is_some() {
            changed = true;
        }
    }
    if map.contains_key("PM2_HOME") {
        let cur = map.get("PM2_HOME").and_then(|v| v.as_str()).unwrap_or("");
        if !cur.eq_ignore_ascii_case(pm2_home_str) {
            map.insert("PM2_HOME".to_string(), serde_json::Value::String(pm2_home_str.to_string()));
            changed = true;
        }
    }
    for path_key in ["Path", "PATH"] {
        if let Some(val) = map.get(path_key).and_then(|v| v.as_str()).map(|s| s.to_string()) {
            let (cleaned, removed) = clean_and_prepend_path_string(&val, node_dir_str);
            if !removed.is_empty() || cleaned != val {
                map.insert(path_key.to_string(), serde_json::Value::String(cleaned));
                changed = true;
            }
        }
    }
    changed
}

pub fn sanitize_pm2_dump(dump_path: &Path, root_dir: &Path, log_fn: &dyn Fn(&str, &str)) {
    let Ok(raw) = fs::read_to_string(dump_path) else { return };
    let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&raw) else { return };
    let Some(arr) = parsed.as_array_mut() else { return };

    let node_dir_str = root_dir.join("node").to_string_lossy().to_string();
    let pm2_home_str = root_dir.join(".pm2").to_string_lossy().to_string();
    let pids_dir = root_dir.join(".pm2").join("pids");
    let mut modified = false;

    for item in arr.iter_mut() {
        let Some(obj) = item.as_object_mut() else { continue };
        if sanitize_env_map(obj, &node_dir_str, &pm2_home_str) {
            modified = true;
        }
        if let Some(env_obj) = obj.get_mut("env").and_then(|v| v.as_object_mut()) {
            if sanitize_env_map(env_obj, &node_dir_str, &pm2_home_str) {
                modified = true;
            }
        }
        if let Some(interp) = obj.get("exec_interpreter").and_then(|v| v.as_str()).map(|s| s.to_string()) {
            let lower = interp.to_lowercase();
            if lower.ends_with("node.exe") && !lower.starts_with(&node_dir_str.to_lowercase()) {
                obj.insert("exec_interpreter".to_string(), serde_json::Value::String("node".to_string()));
                modified = true;
            }
        }
        if let Some(pid_p) = obj.get("pm_pid_path").and_then(|v| v.as_str()).map(|s| s.to_string()) {
            if let Some(fname) = Path::new(&pid_p).file_name() {
                let new_pid_p = pids_dir.join(fname).to_string_lossy().to_string();
                if !pid_p.eq_ignore_ascii_case(&new_pid_p) {
                    obj.insert("pm_pid_path".to_string(), serde_json::Value::String(new_pid_p));
                    modified = true;
                }
            }
        }
    }

    if modified {
        if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
            if fs::write(dump_path, pretty).is_ok() {
                log_fn("  ✓ 已清理 dump.pm2 內殘留之舊全域 Node/PM2 路徑設定", "success");
            }
        }
    }
}

pub fn ensure_portable_pm2_patched(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    let node_dir = root_dir.join("node");
    if !node_dir.is_dir() {
        return false;
    }

    log_fn("🟢 正在配置綠色版 Node.js 與 PM2 執行期環境...", "info");
    let mut patched_any = false;

    // 0. 確保統一的 <root_dir>\.pm2 目錄存在，若尚無 dump.pm2 則自動自既有全域/使用者 .pm2 遷移
    let unified_pm2_home = root_dir.join(".pm2");
    let _ = fs::create_dir_all(&unified_pm2_home);

    let target_dump = unified_pm2_home.join("dump.pm2");
    if !target_dump.is_file() {
        if let Some(legacy_dir) = find_best_legacy_pm2_dir(&unified_pm2_home) {
            for fname in &["dump.pm2", "module_conf.json"] {
                let src_f = legacy_dir.join(fname);
                let dst_f = unified_pm2_home.join(fname);
                if src_f.is_file() && !dst_f.is_file() && fs::copy(&src_f, &dst_f).is_ok() {
                    log_fn(&format!("  ✓ 已自 {} 遷移 {fname}", legacy_dir.display()), "success");
                }
            }
        }
    }
    std::env::set_var("PM2_HOME", &unified_pm2_home);

    if target_dump.is_file() {
        sanitize_pm2_dump(&target_dump, root_dir, log_fn);
    }

    // 1. 確保 pm2.cmd 自動將 %dp0% 注入該子程序 PATH 最前端
    let pm2_cmd_path = node_dir.join("pm2.cmd");
    let pm2_cmd_content = "@ECHO off\r\n\
GOTO start\r\n\
:find_dp0\r\n\
SET dp0=%~dp0\r\n\
EXIT /b\r\n\
:start\r\n\
SETLOCAL\r\n\
CALL :find_dp0\r\n\r\n\
IF EXIST \"%dp0%\\node.exe\" (\r\n\
  SET \"_prog=%dp0%\\node.exe\"\r\n\
) ELSE (\r\n\
  SET \"_prog=node\"\r\n\
  SET PATHEXT=%PATHEXT:;.JS;=;%\r\n\
)\r\n\r\n\
SET \"PATH=%dp0%;%PATH%\"\r\n\
endLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & SET \"PATH=%dp0%;%PATH%\" & \"%_prog%\"  \"%dp0%\\node_modules\\pm2\\bin\\pm2\" %*\r\n";

    let need_write_cmd = match fs::read_to_string(&pm2_cmd_path) {
        Ok(s) => !s.contains("PATH=%dp0%;%PATH%"),
        Err(_) => true,
    };
    if need_write_cmd && fs::write(&pm2_cmd_path, pm2_cmd_content).is_ok() {
        patched_any = true;
        log_fn("  ✓ 已建立可攜式 pm2.cmd 執行捷徑", "success");
    }

    // 2. 修補 paths.js (主 pm2 與 pm2-logrotate 內嵌之 pm2)
    let paths_candidates = [
        (node_dir.join("node_modules").join("pm2").join("paths.js"), "../.."),
        (node_dir.join("node_modules").join("pm2-logrotate").join("node_modules").join("pm2").join("paths.js"), "../../../.."),
    ];

    for (p_file, rel_up) in &paths_candidates {
        if !p_file.is_file() {
            continue;
        }
        if let Ok(mut p_txt) = fs::read_to_string(p_file) {
            let orig = p_txt.clone();
            if !p_txt.contains("tcp://127.0.0.1:42681") {
                let re_pipe = Regex::new(r"pm2_file_stucture\.DAEMON_RPC_PORT\s*=\s*'\\\\\\\\\.\\\\pipe\\\\rpc\.sock';\s*pm2_file_stucture\.DAEMON_PUB_PORT\s*=\s*'\\\\\\\\\.\\\\pipe\\\\pub\.sock';\s*pm2_file_stucture\.INTERACTOR_RPC_PORT\s*=\s*'\\\\\\\\\.\\\\pipe\\\\interactor\.sock';").unwrap();
                let replacement_ports = "pm2_file_stucture.DAEMON_RPC_PORT = process.env.PM2_DAEMON_RPC_PORT || 'tcp://127.0.0.1:42681';\n    pm2_file_stucture.DAEMON_PUB_PORT = process.env.PM2_DAEMON_PUB_PORT || 'tcp://127.0.0.1:42682';\n    pm2_file_stucture.INTERACTOR_RPC_PORT = process.env.PM2_INTERACTOR_RPC_PORT || 'tcp://127.0.0.1:42683';";
                p_txt = re_pipe.replace(&p_txt, replacement_ports).to_string();
            }

            if !p_txt.contains("p.basename(execDir).toLowerCase() === 'node'") {
                let home_hook = format!(
                    "  if (process.env.PM2_HOME)\n    return process.env.PM2_HOME;\n\n  try {{\n    var execDir = p.dirname(process.execPath || '');\n    if (p.basename(execDir).toLowerCase() === 'node') {{\n      return p.resolve(execDir, '..', '.pm2');\n    }}\n    var modNodeDir = p.resolve(__dirname, '{}');\n    if (p.basename(modNodeDir).toLowerCase() === 'node') {{\n      return p.resolve(modNodeDir, '..', '.pm2');\n    }}\n  }} catch (e) {{}}",
                    rel_up
                );
                let re_home = Regex::new(r"if\s*\(process\.env\.PM2_HOME\)\s*return\s+process\.env\.PM2_HOME;").unwrap();
                p_txt = re_home.replace(&p_txt, &home_hook).to_string();
            }

            if p_txt != orig {
                let _ = fs::write(p_file, p_txt);
                patched_any = true;
            }
        }
    }

    // 3. 修補 Daemon.js
    let daemon_file = node_dir.join("node_modules").join("pm2").join("lib").join("Daemon.js");
    if daemon_file.is_file() {
        if let Ok(mut d_txt) = fs::read_to_string(&daemon_file) {
            let orig = d_txt.clone();
            if !d_txt.contains("!/^(tcp:|") {
                d_txt = d_txt
                    .replace(
                        "  this.pub_socket.once('bind', function() {\n    fs.chmod(that.pub_socket_file, '775', function(e) {",
                        "  this.pub_socket.once('bind', function() {\n    if (!/^(tcp:|\\\\\\\\\\.\\\\pipe\\\\)/i.test(String(that.pub_socket_file))) fs.chmod(that.pub_socket_file, '775', function(e) {"
                    )
                    .replace(
                        "  this.rpc_socket.once('bind', function() {\n    fs.chmod(that.rpc_socket_file, '775', function(e) {",
                        "  this.rpc_socket.once('bind', function() {\n    if (!/^(tcp:|\\\\\\\\\\.\\\\pipe\\\\)/i.test(String(that.rpc_socket_file))) fs.chmod(that.rpc_socket_file, '775', function(e) {"
                    );
            }
            if d_txt != orig {
                let _ = fs::write(&daemon_file, d_txt);
                patched_any = true;
            }
        }
    }

    // 4. 修補 Utility.js
    let util_file = node_dir.join("node_modules").join("pm2").join("lib").join("Utility.js");
    if util_file.is_file() {
        if let Ok(mut u_txt) = fs::read_to_string(&util_file) {
            let orig = u_txt.clone();
            if !u_txt.contains("canonic_module_name.indexOf('\\\\') !== -1") {
                let target_snip = "    else if(canonic_module_name.indexOf('file://') === 0) {\n      canonic_module_name = canonic_module_name.replace(/\\/$/, '').split('/').pop();\n    }";
                let replacement_snip = "    else if(canonic_module_name.indexOf('file://') === 0) {\n      canonic_module_name = canonic_module_name.replace(/\\/$/, '').split('/').pop();\n    }\n\n    else if(canonic_module_name.indexOf('\\\\') !== -1 || /^[a-zA-Z]:/.test(canonic_module_name)) {\n      canonic_module_name = canonic_module_name.replace(/[\\\\\\/]+$/, '').split(/[\\\\\\/]/).pop();\n    }";
                u_txt = u_txt.replace(target_snip, replacement_snip);
            }
            if u_txt != orig {
                let _ = fs::write(&util_file, u_txt);
                patched_any = true;
            }
        }
    }

    // 5. 修補 NPM.js (支援直接自 node\node_modules\<module> 離線複製安裝模組)
    let npm_file = node_dir.join("node_modules").join("pm2").join("lib").join("API").join("Modules").join("NPM.js");
    if npm_file.is_file() {
        if let Ok(mut n_txt) = fs::read_to_string(&npm_file) {
            let orig = n_txt.clone();
            if !n_txt.contains("var local_src = null;") {
                let target_npm = "  var canonic_module_name = Utility.getCanonicModuleName(module_name);\n  var install_path = path.join(cst.DEFAULT_MODULE_PATH, canonic_module_name);\n\n  fs.promises.mkdir(install_path, { recursive: true })";
                let replacement_npm = "  var canonic_module_name = Utility.getCanonicModuleName(module_name);\n  var install_path = path.join(cst.DEFAULT_MODULE_PATH, canonic_module_name);\n\n  var local_src = null;\n  try {\n    if (fs.existsSync(module_name) && fs.statSync(module_name).isDirectory() && fs.existsSync(path.join(module_name, 'package.json'))) {\n      local_src = path.resolve(module_name);\n    } else {\n      var bundled_cand = path.resolve(__dirname, '../../../../', canonic_module_name);\n      if (fs.existsSync(path.join(bundled_cand, 'package.json'))) {\n        local_src = bundled_cand;\n      }\n    }\n  } catch (e) {}\n\n  if (local_src) {\n    var target_mod_dir = path.join(install_path, 'node_modules', canonic_module_name);\n    fs.promises.mkdir(target_mod_dir, { recursive: true })\n      .then(function() {\n        Common.printOut(cst.PREFIX_MSG_MOD + 'Installing bundled local module from ' + local_src + ' ...');\n        var src_pkg_path = path.join(local_src, 'package.json');\n        var dst_pkg_path = path.join(target_mod_dir, 'package.json');\n        fs.copyFileSync(src_pkg_path, dst_pkg_path);\n        var pkg_obj = JSON.parse(fs.readFileSync(src_pkg_path, 'utf8'));\n        var entry_rel = (pkg_obj.apps && pkg_obj.apps[0] && pkg_obj.apps[0].script) || pkg_obj.main || 'app.js';\n        var src_entry = path.join(local_src, entry_rel);\n        var dst_entry = path.join(target_mod_dir, entry_rel);\n        fs.mkdirSync(path.dirname(dst_entry), { recursive: true });\n        fs.writeFileSync(dst_entry, 'require(' + JSON.stringify(src_entry) + ');\\n', 'utf8');\n        finalizeInstall(0);\n      })\n      .catch(function(err) {\n        console.error(err.stack || err);\n        finalizeInstall(1);\n      });\n    return;\n  }\n\n  fs.promises.mkdir(install_path, { recursive: true })";
                n_txt = n_txt.replace(target_npm, replacement_npm);
            }

            let old_backup = "    var module_path = path.join(cst.DEFAULT_MODULE_PATH, canonic_module_name);\n    copydirSync(module_path, path.join(tmpdir, canonic_module_name));";
            let new_backup = "    var module_path = path.join(cst.DEFAULT_MODULE_PATH, canonic_module_name);\n    try { if (fs.existsSync(module_path)) copydirSync(module_path, path.join(tmpdir, canonic_module_name)); } catch (e) {}";
            n_txt = n_txt.replace(old_backup, new_backup);

            let old_start = "  var install_path = path.join(cst.DEFAULT_MODULE_PATH, module_name);\n  var proc_path = path.join(install_path, 'node_modules', module_name);\n  var package_json_path = path.join(proc_path, 'package.json');";
            let new_start = "  var install_path = path.join(cst.DEFAULT_MODULE_PATH, module_name);\n  var proc_path = path.join(install_path, 'node_modules', module_name);\n  var package_json_path = path.join(proc_path, 'package.json');\n  if (!fs.existsSync(package_json_path)) {\n    var bundled_proc = path.resolve(__dirname, '../../../../', module_name);\n    if (fs.existsSync(path.join(bundled_proc, 'package.json'))) {\n      proc_path = bundled_proc;\n      package_json_path = path.join(bundled_proc, 'package.json');\n    }\n  }";
            if !n_txt.contains("var bundled_proc = path.resolve(__dirname") {
                n_txt = n_txt.replace(old_start, new_start);
            }

            if n_txt != orig {
                let _ = fs::write(&npm_file, n_txt);
                patched_any = true;
            }
        }
    }

    // 6. 修補 pm2-logrotate/app.js 確保預設指向 <root_dir>\.pm2，並直接掛載便攜版內建 pm2-logrotate 至 .pm2
    let logrotate_app = node_dir.join("node_modules").join("pm2-logrotate").join("app.js");
    if logrotate_app.is_file() {
        if let Ok(mut lr_txt) = fs::read_to_string(&logrotate_app) {
            let orig = lr_txt.clone();
            if !lr_txt.contains("path.basename(path.dirname(process.execPath") {
                let target_lr = "if (process.env.PM2_HOME)\n  PM2_ROOT_PATH = process.env.PM2_HOME;";
                let replacement_lr = "if (process.env.PM2_HOME)\n  PM2_ROOT_PATH = process.env.PM2_HOME;\nelse if (path.basename(path.dirname(process.execPath || '')).toLowerCase() === 'node')\n  PM2_ROOT_PATH = path.resolve(path.dirname(process.execPath), '..', '.pm2');";
                lr_txt = lr_txt.replace(target_lr, replacement_lr);
            }
            if lr_txt != orig {
                let _ = fs::write(&logrotate_app, lr_txt);
                patched_any = true;
            }
        }
    }
    wire_bundled_pm2_logrotate(root_dir, log_fn);

    if patched_any {
        log_fn("  ✓ PM2 綠色環境、Loopback TCP IPC (Port 42681) 與離線日誌輪替模組修補完成", "success");
    }

    true
}

pub fn stop_and_remove_legacy_pm2_services(log_fn: &dyn Fn(&str, &str)) {
    for svc in LEGACY_PM2_SERVICES {
        if run_sc(&["query", svc]).0 {
            log_fn(&format!("🛑 偵測到舊全域 PM2 服務 [{svc}]，正在停止並移除..."), "info");
            let _ = run_cmd_hidden("net.exe", &["stop", svc]);
            let _ = run_sc(&["stop", svc]);
            std::thread::sleep(std::time::Duration::from_millis(800));
            let (ok_del, _, _) = run_sc(&["delete", svc]);
            if ok_del {
                log_fn(&format!("  ✓ 已移除舊全域服務 [{svc}]"), "success");
            }
        }
    }
}

pub fn kill_pm2_daemon(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    stop_and_remove_legacy_pm2_services(log_fn);

    log_fn("🛑 正在執行 pm2 kill 並釋放 Node.js / PM2 行程鎖定...", "info");
    if let Some(pm2) = resolve_pm2_command(root_dir) {
        let (ok, stdout, stderr) = run_cmd_hidden(&pm2.to_string_lossy(), &["kill"]);
        if ok || stdout.contains("daemon stopped") {
            log_fn("  ✓ PM2 守護進程已終止", "success");
        } else {
            log_fn(&format!("  ℹ️ PM2 kill 輸出: {} {}", stdout, stderr), "info");
        }
    }

    let node_dir_str = root_dir.join("node").to_string_lossy().to_lowercase();
    let ps_kill = format!(
        "Get-CimInstance Win32_Process | Where-Object {{ \
            (($_.Name -like 'node*.exe' -or $_.Name -like 'php-cgi*.exe') -and $_.ExecutablePath -and $_.ExecutablePath.ToLower().StartsWith('{}')) \
            -or ($_.Name -like 'node*.exe' -and ($_.CommandLine -like '*Daemon.js*' -or $_.CommandLine -like '*God.js*' -or $_.CommandLine -like '*pm2*service*index.js*')) \
            -or ($_.Name -eq 'pm2.exe') \
        }} | ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }}",
        node_dir_str.replace('\\', "\\\\")
    );
    let (ok_ps, _, _) = run_powershell(&ps_kill);
    if ok_ps {
        log_fn("  ✓ 本機相關 Node 與舊全域 PM2 殘留進程已清理完畢", "success");
    }

    (true, "PM2 與背景守護行程已終止".to_string())
}

pub fn resurrect_pm2_services(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    if let Some(pm2) = resolve_pm2_command(root_dir) {
        log_fn("🚀 正在執行 pm2 resurrect 喚醒已註冊之受管服務...", "info");
        let (ok, stdout, stderr) = run_cmd_hidden(&pm2.to_string_lossy(), &["resurrect", "--no-color"]);
        if ok || stdout.contains("Restoring") {
            log_fn("  ✓ PM2 服務已成功喚醒！", "success");
            return (true, "PM2 已喚醒".to_string());
        } else {
            log_fn(&format!("  ⚠️ pm2 resurrect 回傳: {} {}", stdout, stderr), "warning");
        }
    }
    (false, "無法執行 pm2 resurrect".to_string())
}

fn is_logrotate_online(pm2_cmd: &str) -> bool {
    let (ok, out, _) = run_cmd_hidden(pm2_cmd, &["jlist"]);
    if !ok {
        return false;
    }
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&out) else { return false };
    let Some(arr) = parsed.as_array() else { return false };
    arr.iter().any(|item| {
        item.get("name").and_then(|v| v.as_str()) == Some("pm2-logrotate")
            && item.get("pm2_env").and_then(|e| e.get("status")).and_then(|v| v.as_str()) == Some("online")
    })
}

pub fn ensure_pm2_logrotate(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    wire_bundled_pm2_logrotate(root_dir, log_fn);

    let Some(pm2) = resolve_pm2_command(root_dir) else {
        log_fn("⚠️ 找不到 pm2.cmd，略過 pm2-logrotate 啟動", "warning");
        return false;
    };
    let pm2_str = pm2.to_string_lossy().to_string();
    let local_mod = root_dir.join("node").join("node_modules").join("pm2-logrotate");

    if is_logrotate_online(&pm2_str) {
        let _ = run_cmd_hidden(&pm2_str, &["set", "pm2-logrotate:compress", "true", "--no-color"]);
        log_fn(
            &format!("  ✓ 便攜版內建 [pm2-logrotate] 已在線運作 ({})", local_mod.display()),
            "success",
        );
        return true;
    }

    log_fn(
        &format!("📦 正在由便攜版目錄啟動日誌輪替模組 ({})...", local_mod.display()),
        "info",
    );
    let (restart_ok, _, _) = run_cmd_hidden(&pm2_str, &["restart", "pm2-logrotate", "--no-color"]);
    if restart_ok && is_logrotate_online(&pm2_str) {
        let _ = run_cmd_hidden(&pm2_str, &["set", "pm2-logrotate:compress", "true", "--no-color"]);
        let _ = run_cmd_hidden(&pm2_str, &["save", "--force", "--no-color"]);
        log_fn("  ✓ 已喚醒便攜版內建 [pm2-logrotate] 模組（已啟用日誌壓縮）", "success");
        return true;
    }

    let target_pkg = if local_mod.is_dir() {
        local_mod.to_string_lossy().to_string()
    } else {
        "pm2-logrotate".to_string()
    };

    let (inst_ok, stdout, stderr) = run_cmd_hidden(&pm2_str, &["install", &target_pkg, "--no-color"]);
    if inst_ok || is_logrotate_online(&pm2_str) {
        let _ = run_cmd_hidden(&pm2_str, &["set", "pm2-logrotate:compress", "true", "--no-color"]);
        let _ = run_cmd_hidden(&pm2_str, &["save", "--force", "--no-color"]);
        log_fn("  ✓ 成功由便攜版目錄掛載並啟動 [pm2-logrotate] 模組（已啟用日誌壓縮）", "success");
        true
    } else {
        log_fn(&format!("  ⚠️ 啟動 [pm2-logrotate] 回傳警示: {} {}", stdout, stderr), "warning");
        false
    }
}

fn run_sc(args: &[&str]) -> (bool, String, String) {
    run_cmd_hidden("sc.exe", args)
}

pub fn is_pm2_service_installed() -> bool {
    run_sc(&["query", SERVICE_ID]).0
}

pub fn is_pm2_service_running() -> bool {
    let (_, out, _) = run_sc(&["query", SERVICE_ID]);
    out.contains("RUNNING")
}

pub fn stop_pm2_service(log_fn: &dyn Fn(&str, &str)) {
    if !is_pm2_service_running() {
        return;
    }
    log_fn(&format!("⏳ 正在停止服務 [{}] 以釋放 node.exe 鎖定...", SERVICE_ID), "info");
    let _ = run_cmd_hidden("net.exe", &["stop", SERVICE_ID]);
    for _ in 0..20 {
        if !is_pm2_service_running() {
            log_fn("  ✓ 服務已停止", "success");
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    log_fn("  ⚠️ 服務停止逾時，將繼續嘗試清理行程", "warning");
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn build_service_xml(root_dir: &Path) -> String {
    let root = xml_escape(&root_dir.to_string_lossy());
    format!(
        "<service>\r\n\
  <id>{id}</id>\r\n\
  <name>PM2 可攜版背景服務</name>\r\n\
  <description>開機自動以可攜 Node.js 喚醒 PM2 並還原 dump.pm2，Daemon 掉線時自動 resurrect。</description>\r\n\
  <executable>{root}\\node\\node.exe</executable>\r\n\
  <arguments>\"{root}\\node\\service\\pm2_service_runner.js\"</arguments>\r\n\
  <workingdirectory>{root}</workingdirectory>\r\n\
  <env name=\"PATH\" value=\"{root}\\node;%PATH%\" />\r\n\
  <env name=\"PM2_HOME\" value=\"{root}\\.pm2\" />\r\n\
  <logmode>roll-by-size</logmode>\r\n\
  <logpath>{root}\\logs\\pm2-service</logpath>\r\n\
  <sizeThreshold>10240</sizeThreshold>\r\n\
  <keepFiles>10</keepFiles>\r\n\
  <onfailure action=\"restart\" delay=\"10 sec\"/>\r\n\
  <onfailure action=\"restart\" delay=\"30 sec\"/>\r\n\
  <resetfailure>1 hour</resetfailure>\r\n\
</service>\r\n",
        id = SERVICE_ID,
        root = root
    )
}

pub fn setup_pm2_service(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    log_fn(&format!("⚙️ 正在註冊開機自啟動服務 ({})...", SERVICE_ID), "info");
    let service_dir = root_dir.join("node").join("service");
    if let Err(e) = fs::create_dir_all(&service_dir) {
        log_fn(&format!("  ❌ 建立 service 目錄失敗: {}", e), "error");
        return false;
    }
    let exe_path = service_dir.join("pm2-service.exe");
    let writes = [
        (&exe_path, SERVICE_EXE.to_vec()),
        (&service_dir.join("pm2_service_runner.js"), SERVICE_RUNNER_JS.to_vec()),
        (&service_dir.join("pm2-service.xml"), build_service_xml(root_dir).into_bytes()),
    ];
    for (p, bytes) in &writes {
        if let Err(e) = fs::write(p, bytes) {
            log_fn(&format!("  ❌ 釋出 {} 失敗: {}", p.display(), e), "error");
            return false;
        }
    }

    let exe_str = exe_path.to_string_lossy().to_string();
    if is_pm2_service_installed() {
        let _ = run_cmd_hidden(&exe_str, &["uninstall"]);
    }
    let (ok_inst, out_inst, err_inst) = run_cmd_hidden(&exe_str, &["install"]);
    if !ok_inst {
        log_fn(&format!("  ❌ 服務註冊失敗: {} {}", out_inst, err_inst), "error");
        return false;
    }
    log_fn(&format!("  ✓ {} 系統服務註冊完成", SERVICE_ID), "success");

    let (ok_start, out_start, err_start) = run_cmd_hidden(&exe_str, &["start"]);
    if ok_start || is_pm2_service_running() {
        log_fn("  ✓ 服務已啟動", "success");
    } else {
        log_fn(&format!("  ⚠️ 服務啟動回傳: {} {}", out_start, err_start), "warning");
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wire_bundled_pm2_logrotate_idempotent() {
        let tmp = std::env::temp_dir().join(format!("lah_test_lr_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let bundled = tmp.join("node").join("node_modules").join("pm2-logrotate");
        fs::create_dir_all(&bundled).unwrap();
        fs::write(
            bundled.join("package.json"),
            r#"{"name":"pm2-logrotate","version":"3.0.0","main":"app.js"}"#,
        )
        .unwrap();
        fs::write(bundled.join("app.js"), "// mock logrotate").unwrap();

        assert_eq!(logrotate_bundled_version(&tmp).as_deref(), Some("3.0.0"));
        assert!(is_logrotate_installed(&tmp));
        assert!(!is_logrotate_configured(&tmp));

        assert!(wire_bundled_pm2_logrotate(&tmp, &|_, _| {}));
        assert!(is_logrotate_configured(&tmp));

        let wrapper = fs::read_to_string(
            tmp.join(".pm2")
                .join("modules")
                .join("pm2-logrotate")
                .join("node_modules")
                .join("pm2-logrotate")
                .join("app.js"),
        )
        .unwrap();
        assert!(wrapper.contains("pm2-logrotate"));
        assert!(wrapper.contains("app.js"));

        let conf_str = fs::read_to_string(tmp.join(".pm2").join("module_conf.json")).unwrap();
        let conf: serde_json::Value = serde_json::from_str(&conf_str).unwrap();
        assert!(conf["module-db-v2"]["pm2-logrotate"].is_object());
        assert_eq!(conf["pm2-logrotate"]["compress"], serde_json::json!(true));

        let _ = fs::remove_dir_all(&tmp);
    }
}
