use crate::health::is_port_listening;
use crate::services::{is_pm2_service_installed, stop_service};
use crate::system::{clean_and_prepend_path_string, run_cmd_hidden, run_powershell};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

const LEGACY_PM2_SERVICES: &[&str] = &["pm2.exe", "PM2"];
const LAH_SERVICE_EXE: &[u8] = include_bytes!("../../service/lah-service.exe");
const LAH_SERVICE_XML: &[u8] = include_bytes!("../../service/lah-service.xml");
const SERVICE_RUNNER_JS: &[u8] = include_bytes!("../../service/service_runner.js");

pub fn resolve_pm2_command(root_dir: &Path) -> Option<PathBuf> {
    let cand1 = root_dir.join("node").join("pm2.cmd");
    if cand1.is_file() {
        return Some(cand1);
    }
    let cand2 = root_dir
        .join("node")
        .join("node_modules")
        .join("pm2")
        .join("bin")
        .join("pm2");
    if cand2.is_file() {
        return Some(cand2);
    }
    None
}

pub fn get_node_version(root_dir: &Path) -> Option<String> {
    let node_exe = root_dir.join("node").join("node.exe");
    if !node_exe.is_file() {
        return None;
    }
    let (ok, out, _) = run_cmd_hidden(&node_exe.to_string_lossy(), &["-v"]);
    let v = out.trim();
    (ok && !v.is_empty()).then(|| v.to_string())
}

pub fn get_pm2_version(root_dir: &Path) -> Option<String> {
    let pkg = root_dir
        .join("node")
        .join("node_modules")
        .join("pm2")
        .join("package.json");
    let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(pkg).ok()?).ok()?;
    json.get("version")?.as_str().map(|s| s.to_string())
}

pub fn is_logrotate_installed(root_dir: &Path) -> bool {
    let mod_dir = root_dir.join(".pm2").join("modules").join("pm2-logrotate");
    let bundled_dir = root_dir
        .join("node")
        .join("node_modules")
        .join("pm2-logrotate");
    mod_dir
        .join("node_modules")
        .join("pm2-logrotate")
        .join("package.json")
        .is_file()
        || mod_dir.join("package.json").is_file()
        || bundled_dir.join("package.json").is_file()
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
            map.insert(
                "PM2_HOME".to_string(),
                serde_json::Value::String(pm2_home_str.to_string()),
            );
            changed = true;
        }
    }
    for path_key in ["Path", "PATH"] {
        if let Some(val) = map
            .get(path_key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        {
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
    let Ok(raw) = fs::read_to_string(dump_path) else {
        return;
    };
    let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
    let Some(arr) = parsed.as_array_mut() else {
        return;
    };

    let node_dir_str = root_dir.join("node").to_string_lossy().to_string();
    let pm2_home_str = root_dir.join(".pm2").to_string_lossy().to_string();
    let pids_dir = root_dir.join(".pm2").join("pids");
    let mut modified = false;

    for item in arr.iter_mut() {
        let Some(obj) = item.as_object_mut() else {
            continue;
        };
        if sanitize_env_map(obj, &node_dir_str, &pm2_home_str) {
            modified = true;
        }
        if let Some(env_obj) = obj.get_mut("env").and_then(|v| v.as_object_mut()) {
            if sanitize_env_map(env_obj, &node_dir_str, &pm2_home_str) {
                modified = true;
            }
        }
        if let Some(interp) = obj
            .get("exec_interpreter")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        {
            let lower = interp.to_lowercase();
            if lower.ends_with("node.exe") && !lower.starts_with(&node_dir_str.to_lowercase()) {
                obj.insert(
                    "exec_interpreter".to_string(),
                    serde_json::Value::String("node".to_string()),
                );
                modified = true;
            }
        }
        if let Some(pid_p) = obj
            .get("pm_pid_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        {
            if let Some(fname) = Path::new(&pid_p).file_name() {
                let new_pid_p = pids_dir.join(fname).to_string_lossy().to_string();
                if !pid_p.eq_ignore_ascii_case(&new_pid_p) {
                    obj.insert(
                        "pm_pid_path".to_string(),
                        serde_json::Value::String(new_pid_p),
                    );
                    modified = true;
                }
            }
        }
    }

    if modified {
        if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
            if fs::write(dump_path, pretty).is_ok() {
                log_fn(
                    "  ✓ 已清理 dump.pm2 內殘留之舊全域 Node/PM2 路徑設定",
                    "success",
                );
            }
        }
    }
}

/// 將便攜版 node\node_modules\pm2-logrotate 預先掛載至 <root_dir>\.pm2\modules\pm2-logrotate 與 module_conf.json
pub fn seed_bundled_logrotate_module(
    root_dir: &Path,
    unified_pm2_home: &Path,
    log_fn: &dyn Fn(&str, &str),
) -> bool {
    let bundled_lr = root_dir
        .join("node")
        .join("node_modules")
        .join("pm2-logrotate");
    let src_pkg = bundled_lr.join("package.json");
    let src_app = bundled_lr.join("app.js");
    if !src_pkg.is_file() || !src_app.is_file() {
        return false;
    }

    let target_mod_dir = unified_pm2_home
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
    let wrapper_code = "const path = require('path');\n\
const fs = require('fs');\n\
const execDir = path.dirname(process.execPath || '');\n\
const cand1 = path.resolve(execDir, 'node_modules', 'pm2-logrotate', 'app.js');\n\
const cand2 = path.resolve(__dirname, '../../../../../node/node_modules/pm2-logrotate/app.js');\n\
require(fs.existsSync(cand1) ? cand1 : cand2);\n";
    let _ = fs::write(&dst_app, wrapper_code);

    let conf_path = unified_pm2_home.join("module_conf.json");
    let mut conf_val: serde_json::Value = fs::read_to_string(&conf_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    if let Some(root_obj) = conf_val.as_object_mut() {
        let mut changed = false;
        let db_entry = root_obj
            .entry("module-db-v2".to_string())
            .or_insert_with(|| serde_json::json!({}));
        if let Some(db_obj) = db_entry.as_object_mut() {
            if !db_obj.contains_key("pm2-logrotate") {
                db_obj.insert("pm2-logrotate".to_string(), serde_json::json!({}));
                changed = true;
            }
        }
        let lr_entry = root_obj
            .entry("pm2-logrotate".to_string())
            .or_insert_with(|| serde_json::json!({}));
        if let Some(lr_obj) = lr_entry.as_object_mut() {
            if !lr_obj.contains_key("compress") {
                lr_obj.insert(
                    "compress".to_string(),
                    serde_json::Value::String("true".to_string()),
                );
                changed = true;
            }
        }
        if changed || !conf_path.is_file() {
            if let Ok(pretty) = serde_json::to_string_pretty(&conf_val) {
                if fs::write(&conf_path, pretty).is_ok() {
                    log_fn(
                        "  ✓ 已預先掛載便攜版內建 pm2-logrotate 模組至 .pm2/modules",
                        "success",
                    );
                }
            }
        }
    }

    true
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
                    log_fn(
                        &format!("  ✓ 已自 {} 遷移 {fname}", legacy_dir.display()),
                        "success",
                    );
                }
            }
        }
    }
    std::env::set_var("PM2_HOME", &unified_pm2_home);

    if target_dump.is_file() {
        sanitize_pm2_dump(&target_dump, root_dir, log_fn);
    }

    // 0.5 預先掛載便攜版內建之 pm2-logrotate
    seed_bundled_logrotate_module(root_dir, &unified_pm2_home, log_fn);

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
        (
            node_dir.join("node_modules").join("pm2").join("paths.js"),
            "../..",
        ),
        (
            node_dir
                .join("node_modules")
                .join("pm2-logrotate")
                .join("node_modules")
                .join("pm2")
                .join("paths.js"),
            "../../../..",
        ),
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
                let re_home =
                    Regex::new(r"if\s*\(process\.env\.PM2_HOME\)\s*return\s+process\.env\.PM2_HOME;")
                        .unwrap();
                p_txt = re_home.replace(&p_txt, &home_hook).to_string();
            }

            if p_txt != orig {
                let _ = fs::write(p_file, p_txt);
                patched_any = true;
            }
        }
    }

    // 3. 修補 Daemon.js (避免在 Windows 對 tcp:// 或 pipe 執行 fs.chmod 報錯)
    let daemon_file = node_dir
        .join("node_modules")
        .join("pm2")
        .join("lib")
        .join("Daemon.js");
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

    // 4. 修補 Utility.js (支援 Windows 本機路徑解析 getCanonicModuleName)
    let util_file = node_dir
        .join("node_modules")
        .join("pm2")
        .join("lib")
        .join("Utility.js");
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
    let npm_file = node_dir
        .join("node_modules")
        .join("pm2")
        .join("lib")
        .join("API")
        .join("Modules")
        .join("NPM.js");
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

            if n_txt != orig {
                let _ = fs::write(&npm_file, n_txt);
                patched_any = true;
            }
        }
    }

    // 6. 修補 pm2-logrotate/app.js 確保預設指向 <root_dir>\.pm2
    let logrotate_app = node_dir
        .join("node_modules")
        .join("pm2-logrotate")
        .join("app.js");
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

    if patched_any {
        log_fn(
            "  ✓ PM2 綠色環境、Loopback TCP IPC (Port 42681) 與離線日誌輪替模組修補完成",
            "success",
        );
    }

    true
}

pub fn stop_and_remove_legacy_pm2_services(log_fn: &dyn Fn(&str, &str)) {
    for svc in LEGACY_PM2_SERVICES {
        if run_cmd_hidden("sc.exe", &["query", svc]).0 {
            log_fn(
                &format!("🛑 偵測到舊全域 PM2 服務 [{svc}]，正在停止並移除..."),
                "info",
            );
            let _ = run_cmd_hidden("net.exe", &["stop", svc]);
            let _ = run_cmd_hidden("sc.exe", &["stop", svc]);
            std::thread::sleep(std::time::Duration::from_millis(800));
            let (ok_del, _, _) = run_cmd_hidden("sc.exe", &["delete", svc]);
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
        log_fn(
            "  ✓ 本機相關 Node、舊全域 PM2 與 FastCGI 殘留進程已清理完畢",
            "success",
        );
    }

    (true, "PM2 與背景守護行程已終止".to_string())
}

pub fn resurrect_pm2_services(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    if let Some(pm2) = resolve_pm2_command(root_dir) {
        log_fn("🚀 正在執行 pm2 resurrect 喚醒已註冊之受管服務...", "info");
        let (ok, stdout, stderr) =
            run_cmd_hidden(&pm2.to_string_lossy(), &["resurrect", "--no-color"]);
        if ok || stdout.contains("Restoring") {
            log_fn("  ✓ PM2 服務已成功喚醒！", "success");
            return (true, "PM2 已喚醒".to_string());
        } else {
            log_fn(
                &format!("  ⚠️ pm2 resurrect 回傳: {} {}", stdout, stderr),
                "warning",
            );
        }
    }
    (false, "無法執行 pm2 resurrect".to_string())
}

pub fn restart_pm2_services(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    if let Some(pm2) = resolve_pm2_command(root_dir) {
        log_fn("🔄 正在執行 pm2 restart all 重啟所有受管服務...", "info");
        let (ok, stdout, stderr) =
            run_cmd_hidden(&pm2.to_string_lossy(), &["restart", "all", "--no-color"]);
        if ok {
            log_fn("  ✓ PM2 服務已全部重新啟動！", "success");
            return (true, "已重啟全部 PM2 服務".to_string());
        } else {
            log_fn(
                &format!("  ⚠️ pm2 restart all 警示: {} {}", stdout, stderr),
                "warning",
            );
        }
    }
    (false, "無法執行 pm2 restart all".to_string())
}

fn is_logrotate_online(pm2_cmd: &str) -> bool {
    let (ok, out, _) = run_cmd_hidden(pm2_cmd, &["jlist"]);
    if !ok {
        return false;
    }
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&out) else {
        return false;
    };
    let Some(arr) = parsed.as_array() else {
        return false;
    };
    arr.iter().any(|item| {
        item.get("name").and_then(|v| v.as_str()) == Some("pm2-logrotate")
            && item
                .get("pm2_env")
                .and_then(|e| e.get("status"))
                .and_then(|v| v.as_str())
                == Some("online")
    })
}

pub fn ensure_pm2_logrotate(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    let Some(pm2) = resolve_pm2_command(root_dir) else {
        log_fn("⚠️ 找不到 pm2.cmd，略過 pm2-logrotate 安裝", "warning");
        return false;
    };
    let pm2_str = pm2.to_string_lossy().to_string();

    if is_logrotate_online(&pm2_str) {
        log_fn("  ✓ 日誌輪替模組 [pm2-logrotate] 已在線運作", "success");
        return true;
    }

    log_fn("📦 正在自便攜版內建模組啟動 [pm2-logrotate]...", "info");
    let (restart_ok, _, _) =
        run_cmd_hidden(&pm2_str, &["restart", "pm2-logrotate", "--no-color"]);
    if restart_ok && is_logrotate_online(&pm2_str) {
        log_fn("  ✓ 已喚醒 [pm2-logrotate] 模組", "success");
        let _ = run_cmd_hidden(&pm2_str, &["save", "--force", "--no-color"]);
        return true;
    }

    let local_mod = root_dir
        .join("node")
        .join("node_modules")
        .join("pm2-logrotate");
    let target_pkg = if local_mod.is_dir() {
        local_mod.to_string_lossy().to_string()
    } else {
        "pm2-logrotate".to_string()
    };

    let (inst_ok, stdout, stderr) =
        run_cmd_hidden(&pm2_str, &["install", &target_pkg, "--no-color"]);
    if inst_ok || is_logrotate_online(&pm2_str) {
        let _ = run_cmd_hidden(
            &pm2_str,
            &["set", "pm2-logrotate:compress", "true", "--no-color"],
        );
        let _ = run_cmd_hidden(&pm2_str, &["save", "--force", "--no-color"]);
        log_fn(
            "  ✓ 成功啟動便攜版內建 [pm2-logrotate] 模組（已啟用日誌壓縮）",
            "success",
        );
        true
    } else {
        log_fn(
            &format!("  ⚠️ 安裝 [pm2-logrotate] 回傳警示: {} {}", stdout, stderr),
            "warning",
        );
        false
    }
}

pub fn query_pm2_daemon_info(root_dir: &Path) -> (bool, String) {
    let nv = get_node_version(root_dir);
    let pv = get_pm2_version(root_dir);
    let lr = is_logrotate_installed(root_dir);
    let tcp_alive = is_port_listening(42681, "127.0.0.1", 300);

    match (nv, pv) {
        (Some(n_ver), Some(p_ver)) => {
            let lr_tag = if lr { " + logrotate" } else { "" };
            if tcp_alive {
                (
                    true,
                    format!("Node {n_ver} / PM2 v{p_ver}{lr_tag} 運行中"),
                )
            } else {
                (
                    false,
                    format!("Node {n_ver} / PM2 v{p_ver}{lr_tag} 已就緒 (待啟動)"),
                )
            }
        }
        _ => (false, "PM2 未安裝或離線".to_string()),
    }
}

pub fn ensure_winsw_service_files(
    root_dir: &Path,
    log_fn: &dyn Fn(&str, &str),
) -> Result<PathBuf, String> {
    let service_dir = root_dir.join("node").join("service");
    fs::create_dir_all(&service_dir).map_err(|e| format!("建立 service 目錄失敗: {}", e))?;

    let exe_path = service_dir.join("lah-service.exe");
    if !exe_path.is_file() || fs::metadata(&exe_path).map_or(0, |m| m.len()) == 0 {
        fs::write(&exe_path, LAH_SERVICE_EXE)
            .map_err(|e| format!("釋出 lah-service.exe 失敗: {}", e))?;
        log_fn("  ✓ 已釋出自包含 WinSW 服務核心 (lah-service.exe)", "success");
    }

    let runner_path = service_dir.join("service_runner.js");
    fs::write(&runner_path, SERVICE_RUNNER_JS)
        .map_err(|e| format!("釋出 service_runner.js 失敗: {}", e))?;

    let xml_path = service_dir.join("lah-service.xml");
    let xml_str = String::from_utf8_lossy(LAH_SERVICE_XML).to_string();
    let root_str = root_dir.to_string_lossy().to_string();
    let customized = xml_str.replace("C:\\AppServ", &root_str);
    fs::write(&xml_path, customized).map_err(|e| format!("釋出 lah-service.xml 失敗: {}", e))?;

    Ok(exe_path)
}

pub fn setup_pm2_autostart(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    stop_and_remove_legacy_pm2_services(log_fn);

    log_fn("⚙️ 正在設定 PM2 開機自啟動服務 (LAH_PM2_Service)...", "info");

    let exe_path = match ensure_winsw_service_files(root_dir, log_fn) {
        Ok(p) => p,
        Err(e) => return (false, e),
    };

    let exe_str = exe_path.to_string_lossy().to_string();

    if is_pm2_service_installed() {
        let _ = stop_service("LAH_PM2_Service", log_fn);
        let _ = run_cmd_hidden(&exe_str, &["uninstall"]);
    }

    // 1. 安裝服務
    let (ok_inst, out_inst, _) = run_cmd_hidden(&exe_str, &["install"]);
    if ok_inst || out_inst.contains("already exists") {
        log_fn("  ✓ LAH_PM2_Service 系統服務註冊完成", "success");
    }

    // 2. 啟動服務
    let (ok_start, out_start, _) = run_cmd_hidden(&exe_str, &["start"]);
    if ok_start || out_start.contains("Started") {
        log_fn("  ✓ LAH_PM2_Service 系統服務已成功啟動", "success");
        return (true, "LAH_PM2_Service 已註冊並啟動".to_string());
    }

    (true, "LAH_PM2_Service 已註冊".to_string())
}

pub fn audit_and_ensure_pm2_services(
    root_dir: &Path,
    log_fn: &dyn Fn(&str, &str),
) -> (bool, String) {
    let logs_dir = root_dir.join("logs");
    let _ = fs::create_dir_all(&logs_dir);

    let Some(pm2) = resolve_pm2_command(root_dir) else {
        log_fn("⚠️ 系統未偵測到有效的 pm2.cmd，跳過 PM2 服務巡檢。", "warning");
        return (false, "未偵測到 pm2.cmd".to_string());
    };
    let pm2_str = pm2.to_string_lossy().to_string();

    log_fn("--------------------------------------------------", "info");
    log_fn("🔍 正在執行 PM2 核心服務全方位狀態巡檢...", "info");

    // 1. 取得現有 PM2 程序清單
    let mut managed_apps: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let (ok_jlist, out_jlist, _) = run_cmd_hidden(&pm2_str, &["jlist"]);
    if ok_jlist {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&out_jlist) {
            if let Some(arr) = parsed.as_array() {
                for item in arr {
                    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        let status = item
                            .get("pm2_env")
                            .and_then(|e| e.get("status"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        if !managed_apps.contains_key(name) || status == "online" {
                            managed_apps.insert(name.to_string(), status.to_string());
                        }
                    }
                }
            }
        }
    }

    // 2. 定義 3 大生態系設定檔與所屬服務
    struct EcosystemDef {
        title: &'static str,
        apps: &'static [&'static str],
        candidates: Vec<PathBuf>,
    }

    let core_ecosystems = [
        EcosystemDef {
            title: "即時通訊伺服器 (LAH-WSS)",
            apps: &["LAH-WSS"],
            candidates: vec![root_dir
                .join("lah-messenger-server")
                .join("ecosystem.config.js")],
        },
        EcosystemDef {
            title: "智慧管控前端網站 (LAH-FE / LAH-SCHEDULER)",
            apps: &["LAH-FE", "LAH-SCHEDULER"],
            candidates: vec![
                root_dir.join("LAH-NUXTJS").join("ecosystem.config.js"),
                root_dir.join("LAH-FE").join("ecosystem.config.js"),
            ],
        },
        EcosystemDef {
            title: "現代化後端 API (lah-api-v85)",
            apps: &["lah-api-v85"],
            candidates: vec![
                root_dir.join("LAH-API").join("ecosystem.config.js"),
                root_dir
                    .join("LAH-API")
                    .join("API")
                    .join("ecosystem.config.js"),
            ],
        },
    ];

    let mut need_save = false;

    for eco in &core_ecosystems {
        let eco_path_opt = eco.candidates.iter().find(|p| p.is_file());
        let Some(eco_path) = eco_path_opt else {
            continue;
        };

        let missing_apps: Vec<&&str> = eco
            .apps
            .iter()
            .filter(|a| managed_apps.get(**a).map_or(true, |st| st != "online"))
            .collect();

        if !missing_apps.is_empty() {
            let names = missing_apps
                .iter()
                .map(|s| **s)
                .collect::<Vec<&str>>()
                .join(", ");
            log_fn(
                &format!(
                    "⚡ 偵測到 [{}] 之服務 [{}] 尚未在線，正在啟動 [{}]...",
                    eco.title,
                    names,
                    eco_path.display()
                ),
                "warning",
            );

            let eco_str = eco_path.to_string_lossy().to_string();
            let (start_ok, stdout, stderr) =
                run_cmd_hidden(&pm2_str, &["start", &eco_str, "--no-color"]);
            if start_ok || stdout.contains("✓") {
                log_fn(&format!("  ✅ 成功啟動 [{}]！", eco.title), "success");
                need_save = true;
            } else {
                log_fn(
                    &format!(
                        "  ⚠️ 啟動 [{}] 回傳警示: {} {}",
                        eco_path.display(),
                        stdout,
                        stderr
                    ),
                    "warning",
                );
            }
        }
    }

    // 2.1 檢查並確保日誌輪替模組 (pm2-logrotate) 運作
    if managed_apps
        .get("pm2-logrotate")
        .map_or(true, |st| st != "online")
    {
        if ensure_pm2_logrotate(root_dir, log_fn) {
            need_save = true;
        }
    }

    // 3. 若有補啟動任何項目，自動持久化保存狀態
    if need_save {
        log_fn(
            "💾 偵測到服務配置異動，正在執行 [pm2 save] 更新持久化存檔...",
            "info",
        );
        let _ = run_cmd_hidden(&pm2_str, &["save", "--force", "--no-color"]);
    }

    // 4. 重新查詢一次最新狀態印出摘要報告
    let mut final_apps: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let (ok_final, out_final, _) = run_cmd_hidden(&pm2_str, &["jlist"]);
    if ok_final {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&out_final) {
            if let Some(arr) = parsed.as_array() {
                for item in arr {
                    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        let status = item
                            .get("pm2_env")
                            .and_then(|e| e.get("status"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        if !final_apps.contains_key(name) || status == "online" {
                            final_apps.insert(name.to_string(), status.to_string());
                        }
                    }
                }
            }
        }
    }

    log_fn("📋 PM2 核心服務現況總覽：", "info");
    let mut all_healthy = true;
    for app_name in &[
        "LAH-WSS",
        "LAH-FE",
        "LAH-SCHEDULER",
        "lah-api-v85",
        "pm2-logrotate",
    ] {
        let st = final_apps
            .get(*app_name)
            .map(|s| s.as_str())
            .unwrap_or("未註冊 / 離線");
        if st == "online" {
            log_fn(&format!("  • {:<16} : ✅ {st}", app_name), "success");
        } else {
            all_healthy = false;
            log_fn(&format!("  • {:<16} : ⚠️ {st}", app_name), "warning");
        }
    }

    if all_healthy {
        log_fn("🎉 全部 5 大核心服務與模組皆已成功在線運作！", "success");
    } else {
        log_fn(
            "⚠️ 部分核心服務未能順利在線，請檢視上方日誌輸出或終端機 pm2 status。",
            "warning",
        );
    }

    (all_healthy, "PM2 服務巡檢完成".to_string())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_pm2_dump_and_seed_logrotate() {
        let tmp = std::env::temp_dir().join(format!("lah_updater_pm2_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let node_dir = tmp.join("node");
        let bundled_lr = node_dir.join("node_modules").join("pm2-logrotate");
        let pm2_dir = tmp.join(".pm2");
        fs::create_dir_all(&bundled_lr).unwrap();
        fs::create_dir_all(&pm2_dir).unwrap();

        fs::write(
            bundled_lr.join("package.json"),
            r#"{"name":"pm2-logrotate","version":"2.7.0","main":"app.js"}"#,
        )
        .unwrap();
        fs::write(bundled_lr.join("app.js"), "console.log('ok');").unwrap();

        let dump_file = pm2_dir.join("dump.pm2");
        fs::write(
            &dump_file,
            r#"[{
                "name": "LAH-FE",
                "exec_interpreter": "C:\\Program Files\\nodejs\\node.exe",
                "pm_pid_path": "C:\\ProgramData\\pm2\\home\\pids\\LAH-FE-0.pid",
                "PM2_HOME": "C:\\ProgramData\\pm2\\home",
                "PM2_INSTALL_DIRECTORY": "C:\\ProgramData\\pm2",
                "PM2_SERVICE_DIRECTORY": "C:\\ProgramData\\pm2",
                "Path": "C:\\Program Files\\nodejs;C:\\ProgramData\\npm;C:\\Windows\\System32",
                "env": {
                    "PM2_HOME": "C:\\ProgramData\\pm2\\home",
                    "PM2_INSTALL_DIRECTORY": "C:\\ProgramData\\pm2",
                    "Path": "C:\\Program Files\\nodejs;C:\\Windows\\System32"
                }
            }]"#,
        )
        .unwrap();

        let noop = |_: &str, _: &str| {};
        sanitize_pm2_dump(&dump_file, &tmp, &noop);
        seed_bundled_logrotate_module(&tmp, &pm2_dir, &noop);

        let sanitized: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&dump_file).unwrap()).unwrap();
        let proc0 = &sanitized[0];
        assert_eq!(proc0["exec_interpreter"], "node");
        assert_eq!(proc0["PM2_HOME"], pm2_dir.to_string_lossy().as_ref());
        assert!(proc0.get("PM2_INSTALL_DIRECTORY").is_none());
        assert!(proc0.get("PM2_SERVICE_DIRECTORY").is_none());
        let cleaned_path = proc0["Path"].as_str().unwrap();
        assert!(cleaned_path.starts_with(node_dir.to_string_lossy().as_ref()));
        assert!(!cleaned_path.to_lowercase().contains("program files\\nodejs"));

        assert!(is_logrotate_installed(&tmp));
        let mod_conf: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(pm2_dir.join("module_conf.json")).unwrap())
                .unwrap();
        assert_eq!(mod_conf["pm2-logrotate"]["compress"], "true");
        assert!(mod_conf["module-db-v2"]["pm2-logrotate"].is_object());

        let _ = fs::remove_dir_all(&tmp);
    }
}
