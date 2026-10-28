use crate::system::{run_cmd_hidden, run_powershell};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

pub const SERVICE_ID: &str = "PM2_Portable_Service";
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

pub fn ensure_portable_pm2_patched(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    let node_dir = root_dir.join("node");
    if !node_dir.is_dir() {
        return false;
    }

    log_fn("🟢 正在配置綠色版 Node.js 與 PM2 執行期環境...", "info");
    let mut patched_any = false;

    // 0. 確保統一的 <root_dir>\.pm2 目錄存在，若尚無 dump.pm2 則自動自現有使用者 .pm2 遷移
    let unified_pm2_home = root_dir.join(".pm2");
    let _ = fs::create_dir_all(&unified_pm2_home);
    std::env::set_var("PM2_HOME", &unified_pm2_home);

    let target_dump = unified_pm2_home.join("dump.pm2");
    if !target_dump.is_file() {
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            let user_pm2 = PathBuf::from(user_profile).join(".pm2");
            for fname in &["dump.pm2", "module_conf.json"] {
                let src_f = user_pm2.join(fname);
                let dst_f = unified_pm2_home.join(fname);
                if src_f.is_file() && !dst_f.is_file() {
                    let _ = fs::copy(&src_f, &dst_f);
                }
            }
        }
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
    if need_write_cmd {
        if fs::write(&pm2_cmd_path, pm2_cmd_content).is_ok() {
            patched_any = true;
            log_fn("  ✓ 已建立可攜式 pm2.cmd 執行捷徑", "success");
        }
    }

    // 2. 修補 paths.js (主 pm2 與 pm2-logrotate 內嵌之 pm2)
    //    將 Windows IPC 通訊由 Named Pipe 改為本機 Loopback TCP (127.0.0.1:42681/42682)，
    //    根除未提權 CMD 視窗執行 pm2 時之 connect EPERM 權限錯誤
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

    // 3. 修補 Daemon.js (避免在 Windows 對 tcp:// 或 pipe 執行 fs.chmod 報錯)
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

    // 4. 修補 Utility.js (支援 Windows 本機路徑解析 getCanonicModuleName)
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

            if n_txt != orig {
                let _ = fs::write(&npm_file, n_txt);
                patched_any = true;
            }
        }
    }

    // 6. 修補 pm2-logrotate/app.js 確保預設指向 <root_dir>\.pm2
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

    if patched_any {
        log_fn("  ✓ PM2 綠色環境、Loopback TCP IPC (Port 42681) 與離線日誌輪替模組修補完成", "success");
    }

    true
}
pub fn kill_pm2_daemon(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    log_fn("🛑 正在執行 pm2 kill 並釋放 Node.js 行程鎖定...", "info");
    if let Some(pm2) = resolve_pm2_command(root_dir) {
        let (ok, stdout, stderr) = run_cmd_hidden(&pm2.to_string_lossy(), &["kill"]);
        if ok || stdout.contains("daemon stopped") {
            log_fn("  ✓ PM2 守護進程已終止", "success");
        } else {
            log_fn(&format!("  ℹ️ PM2 kill 輸出: {} {}", stdout, stderr), "info");
        }
    }

    // 檢查是否有殘留以該 node 運行的行程
    let node_dir_str = root_dir.join("node").to_string_lossy().to_lowercase();
    let ps_kill = format!(
        "Get-CimInstance Win32_Process | Where-Object {{ ($_.Name -like 'node*.exe' -or $_.Name -like 'php-cgi*.exe') -and $_.ExecutablePath -and $_.ExecutablePath.ToLower().StartsWith('{}') }} | ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }}",
        node_dir_str.replace('\\', "\\\\")
    );
    let (ok_ps, _, _) = run_powershell(&ps_kill);
    if ok_ps {
        log_fn("  ✓ 本機相關 Node 與 FastCGI 殘留進程已清理完畢", "success");
    }

    (true, "PM2 與背景守護行程已終止".to_string())
}

pub fn resurrect_pm2_services(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    if let Some(pm2) = resolve_pm2_command(root_dir) {
        log_fn("🚀 正在執行 pm2 resurrect 喚醒已註冊之受管服務...", "info");
        let (ok, stdout, stderr) = run_cmd_hidden(&pm2.to_string_lossy(), &["resurrect"]);
        if ok || stdout.contains("Restoring") {
            log_fn("  ✓ PM2 服務已成功喚醒！", "success");
            return (true, "PM2 已喚醒".to_string());
        } else {
            log_fn(&format!("  ⚠️ pm2 resurrect 回傳: {} {}", stdout, stderr), "warning");
        }
    }
    (false, "無法執行 pm2 resurrect".to_string())
}

pub fn restart_pm2_services(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    if let Some(pm2) = resolve_pm2_command(root_dir) {
        log_fn("🔄 正在執行 pm2 restart all 重啟所有受管服務...", "info");
        let (ok, stdout, stderr) = run_cmd_hidden(&pm2.to_string_lossy(), &["restart", "all"]);
        if ok {
            log_fn("  ✓ PM2 服務已全部重新啟動！", "success");
            return (true, "已重啟全部 PM2 服務".to_string());
        } else {
            log_fn(&format!("  ⚠️ pm2 restart all 警示: {} {}", stdout, stderr), "warning");
        }
    }
    (false, "無法執行 pm2 restart all".to_string())
}

pub fn query_pm2_daemon_info(root_dir: &Path) -> (bool, String) {
    if let Some(pm2) = resolve_pm2_command(root_dir) {
        let (ok, stdout, _) = run_cmd_hidden(&pm2.to_string_lossy(), &["ping"]);
        if ok && (stdout.contains("pong") || stdout.contains("PM2 Successfully")) {
            return (true, "PM2 守護程序運行中 (Active)".to_string());
        }
    }
    (false, "PM2 未運行或離線".to_string())
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
