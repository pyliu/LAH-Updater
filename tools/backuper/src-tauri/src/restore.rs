use crate::backup::Progress;
use chrono::Local;
use regex::{NoExpand, Regex};
use serde::Serialize;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Serialize)]
pub struct RestoreReport {
    pub restored: u64,
    pub undo_zip: Option<String>,
    pub undo_files: u64,
}

const PS_LIST_EXE: &str = "Get-CimInstance Win32_Process -Filter \"Name='httpd.exe' or Name='node.exe' or Name='php-cgi.exe' or Name='lah-web-service.exe' or Name='lah-service.exe' or Name='pm2-service.exe'\" | ForEach-Object { $_.ExecutablePath }";
const FE_VHOST_BEGIN: &str = "# === LAH-FE Static VirtualHost (:8080) ===";
const FE_VHOST_END: &str = "# === End LAH-FE Static VirtualHost ===";

/// 回傳執行檔位於 `root` 底下、仍在執行的 httpd/node/php-cgi 路徑。僅檢查屬於該安裝目錄的行程，
/// 避免被機器上其他無關的 node/php-cgi 誤擋。UNC 根目錄無法檢查本機行程，回傳空陣列。
pub fn running_services(root: &Path) -> Vec<String> {
    let root_s = root.to_string_lossy().to_lowercase();
    if root_s.starts_with("\\\\") {
        return vec![];
    }
    let prefix = format!("{}\\", root_s.trim_end_matches('\\'));
    let out = Command::new("powershell").args(["-NoProfile", "-Command", PS_LIST_EXE]).output();
    let text = out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && l.to_lowercase().starts_with(&prefix))
        .map(String::from)
        .collect()
}

/// 掃描 ZIP，回傳 (有效條目索引, 目標路徑)。含 `..` 或絕對路徑的條目一律拒絕（Zip Slip）。
fn plan(archive: &mut ZipArchive<BufReader<File>>, root: &Path) -> Result<Vec<(usize, PathBuf)>, String> {
    let mut out = Vec::new();
    for i in 0..archive.len() {
        let f = archive.by_index(i).map_err(|e| format!("讀取 ZIP 條目失敗: {e}"))?;
        if f.is_dir() {
            continue;
        }
        let rel = f.enclosed_name().ok_or_else(|| format!("偵測到不安全的路徑，已中止: {}", f.name()))?;
        out.push((i, root.join(rel)));
    }
    Ok(out)
}

fn build_fe_vhost_block(fe_dist_fwd: &str, www_fwd: &str, api_fwd: &str) -> String {
    format!(
        "{FE_VHOST_BEGIN}\n\
Listen 8080\n\
<VirtualHost *:8080>\n\
    ServerName localhost:8080\n\
    DocumentRoot \"{fe_dist_fwd}\"\n\
\n\
    # 本機 PHP 7.4 (www) 與 PHP 8.5 (LAH-API) 路由別名映射\n\
    Alias /api \"{www_fwd}/api\"\n\
    Alias /img \"{www_fwd}\"\n\
    Alias /legacy \"{www_fwd}\"\n\
    Alias /assets \"{www_fwd}/assets\"\n\
    Alias /project \"{www_fwd}/project\"\n\
    Alias /LAH-API/api \"{api_fwd}/v1\"\n\
    Alias /lah-api/api \"{api_fwd}/v1\"\n\
    Alias /LAH-API \"{api_fwd}\"\n\
    Alias /lah-api \"{api_fwd}\"\n\
    Alias /get_user_img.php \"{api_fwd}/v1/get_user_img.php\"\n\
    Alias /get_carousel_img.php \"{api_fwd}/v1/get_carousel_img.php\"\n\
    Alias /get_blob_img.php \"{api_fwd}/v1/get_blob_img.php\"\n\
    Alias /get_adm_reserve_pdf.php \"{api_fwd}/v1/get_adm_reserve_pdf.php\"\n\
    Alias /get_reg_foreigner_pdf.php \"{api_fwd}/v1/get_reg_foreigner_pdf.php\"\n\
    Alias /get_sur_destruction_pdf.php \"{api_fwd}/v1/get_sur_destruction_pdf.php\"\n\
\n\
    <IfModule rewrite_module>\n\
        RewriteEngine On\n\
        RewriteRule ^/(l05proxy|smsproxy|srmas-weathermap)/(.*)$ /api/proxy.php?_target=$1&_path=$2 [QSA,PT,L]\n\
    </IfModule>\n\
\n\
    <Directory \"{fe_dist_fwd}\">\n\
        Header set Access-Control-Allow-Origin \"*\"\n\
        Options -Indexes +FollowSymLinks\n\
        AllowOverride None\n\
        Require all granted\n\
        DirectoryIndex index.html\n\
        <FilesMatch \"^(index\\.html|runtime-config\\.js)$\">\n\
            <IfModule headers_module>\n\
                Header set Cache-Control \"no-store, no-cache, must-revalidate, max-age=0\"\n\
                Header set Pragma \"no-cache\"\n\
            </IfModule>\n\
        </FilesMatch>\n\
        <IfModule rewrite_module>\n\
            RewriteEngine On\n\
            RewriteRule ^(l05proxy|smsproxy|srmas-weathermap)/(.*)$ /api/proxy.php?_target=$1&_path=$2 [QSA,L]\n\
            RewriteCond %{{REQUEST_FILENAME}} !-f\n\
            RewriteCond %{{REQUEST_FILENAME}} !-d\n\
            RewriteCond %{{REQUEST_URI}} !^/(api|LAH-API|lah-api|img|legacy|l05proxy|smsproxy|srmas-weathermap|get_)\n\
            RewriteRule ^ /index.html [L]\n\
        </IfModule>\n\
    </Directory>\n\
</VirtualHost>\n\
{FE_VHOST_END}"
    )
}

/// 還原後自動校準 httpd.conf 中的硬編碼路徑至當前目標根目錄，防止跨磁碟/跨目錄還原導致 Apache 啟動失敗
pub fn calibrate_httpd_conf(root: &Path) {
    let conf_path = root.join("Apache24").join("conf").join("httpd.conf");
    if !conf_path.is_file() {
        return;
    }
    let Ok(content) = fs::read_to_string(&conf_path) else {
        return;
    };
    let orig = content.clone();
    let root_fwd = root.to_string_lossy().replace('\\', "/").trim_end_matches('/').to_string();
    let apache_fwd = format!("{root_fwd}/Apache24");
    let www_fwd = format!("{root_fwd}/www");
    let logs_fwd = format!("{root_fwd}/logs/apache");
    let _ = fs::create_dir_all(root.join("logs").join("apache"));
    let api_fwd = format!("{root_fwd}/LAH-API");
    let php7_fwd = format!("{root_fwd}/php7");
    let fe_folder = if root.join("LAH-FE").is_dir() && !root.join("LAH-NUXTJS").is_dir() {
        "LAH-FE"
    } else {
        "LAH-NUXTJS"
    };
    let fe_dist_dir = root.join(fe_folder).join("dist");
    let fe_dist_fwd = format!("{root_fwd}/{fe_folder}/dist");

    let mut updated = content;

    // 1. SRVROOT
    if let Ok(re_srv) = Regex::new(r#"(?m)^\s*Define\s+SRVROOT\s+.*$"#) {
        if re_srv.is_match(&updated) {
            updated = re_srv.replace(&updated, format!(r#"Define SRVROOT "{}""#, apache_fwd)).to_string();
        } else {
            updated = format!("Define SRVROOT \"{}\"\nServerRoot \"${{SRVROOT}}\"\n{}", apache_fwd, updated);
        }
    }

    // 2. 主站台 (:80) DocumentRoot（僅替換第一個頂層 DocumentRoot，避免誤改 VirtualHost *:8080 內的 DocumentRoot）
    if let Ok(re_doc) = Regex::new(r#"(?m)^(\s*)DocumentRoot\s+"[^"]*""#) {
        updated = re_doc.replace(&updated, format!(r#"${{1}}DocumentRoot "{www_fwd}""#)).to_string();
    }

    // 3. Directory block for www
    if let Ok(re_old_www) = Regex::new(r#"(?s)<Directory\s+"[a-zA-Z]:/[^"]*?/www">"#) {
        updated = re_old_www.replace_all(&updated, format!(r#"<Directory "{}">"#, www_fwd)).to_string();
    }

    // 4. Rotatelogs 日誌路徑 (含舊版 Apache24/logs 與預設 logs/error_log 平移歸集至 logs/apache)
    if let Ok(re_errlog_rotate) = Regex::new(
        r#"(?m)^([ \t]*)ErrorLog[ \t]+"\|(?:[^"]*?/)?rotatelogs(?:\.exe)?[ \t]+-l[ \t]+[^"]*?(error\.[^"]+)"[ \t]*(\r?)$"#,
    ) {
        if re_errlog_rotate.is_match(&updated) {
            updated = re_errlog_rotate
                .replace_all(&updated, |caps: &regex::Captures| {
                    let indent = &caps[1];
                    let err_pat = &caps[2];
                    let cr = &caps[3];
                    format!(r#"{indent}ErrorLog "|bin/rotatelogs.exe -l {logs_fwd}/{err_pat}"{cr}"#)
                })
                .to_string();
        } else if let Ok(re_errlog_plain) = Regex::new(r#"(?m)^([ \t]*)ErrorLog[ \t]+"[^"]*"[ \t]*(\r?)$"#) {
            updated = re_errlog_plain
                .replace_all(&updated, |caps: &regex::Captures| {
                    let indent = &caps[1];
                    let cr = &caps[2];
                    format!(r#"{indent}ErrorLog "|bin/rotatelogs.exe -l {logs_fwd}/error.%Y.%m.%d.log 100M"{cr}"#)
                })
                .to_string();
        }
    }
    if let Ok(re_custlog_rotate) = Regex::new(
        r#"(?m)^([ \t]*)CustomLog[ \t]+"\|(?:[^"]*?/)?rotatelogs(?:\.exe)?[ \t]+-l[ \t]+[^"]*?(access\.[^"]+)"[ \t]+(\S+)[ \t]*(\r?)$"#,
    ) {
        if re_custlog_rotate.is_match(&updated) {
            updated = re_custlog_rotate
                .replace_all(&updated, |caps: &regex::Captures| {
                    let indent = &caps[1];
                    let acc_pat = &caps[2];
                    let fmt_name = &caps[3];
                    let cr = &caps[4];
                    format!(r#"{indent}CustomLog "|bin/rotatelogs.exe -l  {logs_fwd}/{acc_pat}" {fmt_name}{cr}"#)
                })
                .to_string();
        } else if let Ok(re_custlog_plain) =
            Regex::new(r#"(?m)^([ \t]*)CustomLog[ \t]+"[^"]*"[ \t]+(?:common|combined)[ \t]*(\r?)$"#)
        {
            updated = re_custlog_plain
                .replace_all(&updated, |caps: &regex::Captures| {
                    let indent = &caps[1];
                    let cr = &caps[2];
                    format!(
                        r#"{indent}CustomLog "|bin/rotatelogs.exe -l  {logs_fwd}/access.%Y.%m.%d.log 100M" combined{cr}"#
                    )
                })
                .to_string();
        }
    }

    // 5. FastCGI / LAH-API Alias 與 Directory
    if let Ok(re_api_alias) = Regex::new(r#"(?m)^(\s*)Alias\s+/((?i:lah-api))/api\s+"[a-zA-Z]:/[^"]*?""#) {
        updated = re_api_alias.replace_all(&updated, format!(r#"${{1}}Alias /$2/api "{api_fwd}/v1""#)).to_string();
    }
    if let Ok(re_api_root) = Regex::new(r#"(?m)^(\s*)Alias\s+/((?i:lah-api))\s+"[a-zA-Z]:/[^"]*?""#) {
        updated = re_api_root.replace_all(&updated, format!(r#"${{1}}Alias /$2 "{api_fwd}""#)).to_string();
    }
    if let Ok(re_stream) = Regex::new(r#"(?m)^(\s*)Alias\s+/(get_[a-zA-Z0-9_]+\.(?:php|pdf))\s+"[a-zA-Z]:/[^"]*?""#) {
        updated = re_stream.replace_all(&updated, format!(r#"${{1}}Alias /$2 "{api_fwd}/v1/$2""#)).to_string();
    }
    if let Ok(re_api_dir) = Regex::new(r#"(?s)<Directory\s+"[a-zA-Z]:/[^"]*?/(?i:lah-api)">"#) {
        updated = re_api_dir.replace_all(&updated, format!(r#"<Directory "{}">"#, api_fwd)).to_string();
    }

    // 6. PHP 7 LoadFile / LoadModule / PHPIniDir
    if let Ok(re_loadfile) = Regex::new(r#"(?m)^\s*LoadFile\s+"[a-zA-Z]:/[^"]*?/php7/([^"]+)""#) {
        updated = re_loadfile.replace_all(&updated, format!(r#"LoadFile "{php7_fwd}/$1""#)).to_string();
    }
    if let Ok(re_loadmod) = Regex::new(r#"(?m)^\s*LoadModule\s+php7_module\s+"?[a-zA-Z]:/[^"]*?/php7apache2_4\.dll"?"#) {
        updated = re_loadmod.replace_all(&updated, format!(r#"LoadModule php7_module "{php7_fwd}/php7apache2_4.dll""#)).to_string();
    }
    if let Ok(re_phpinidir) = Regex::new(r#"(?m)^\s*PHPIniDir\s+"[a-zA-Z]:/[^"]*?/php7/?""#) {
        updated = re_phpinidir.replace_all(&updated, format!(r#"PHPIniDir "{php7_fwd}/""#)).to_string();
    }

    // 7. 修正歷史重複百分比符號
    updated = updated.replace("%{%REQUEST_FILENAME}", "%{REQUEST_FILENAME}");

    // 8. LAH-FE 靜態 SPA VirtualHost (*:8080) 路徑校準與自動補齊
    let existing_vhost_re = Regex::new(&format!(
        r"(?s){}.*?{}",
        regex::escape(FE_VHOST_BEGIN),
        regex::escape(FE_VHOST_END)
    ))
    .unwrap();
    let has_static_fe_on_disk = fe_dist_dir.join("index.html").is_file();

    if existing_vhost_re.is_match(&updated) || has_static_fe_on_disk {
        for m in &["headers_module", "rewrite_module", "alias_module"] {
            let pat = format!(
                r#"(?m)^\s*#\s*(LoadModule\s+{}\s+modules/mod_{}\.so)"#,
                m,
                m.trim_end_matches("_module")
            );
            if let Ok(re) = Regex::new(&pat) {
                updated = re.replace(&updated, "$1").to_string();
            }
        }
        let vhost_block = build_fe_vhost_block(&fe_dist_fwd, &www_fwd, &api_fwd);
        if existing_vhost_re.is_match(&updated) {
            updated = existing_vhost_re.replace(&updated, NoExpand(&vhost_block)).to_string();
        } else {
            let re_stray_listen8080 = Regex::new(r#"(?m)^\s*Listen\s+(?:[0-9.]+:)?8080\s*\r?\n"#).unwrap();
            updated = re_stray_listen8080.replace_all(&updated, "").to_string();
            updated.push_str(&format!("\n{}\n", vhost_block));
        }
    } else {
        if let Ok(re_fe_doc) = Regex::new(r#"(?m)^(\s*DocumentRoot\s+)"[a-zA-Z]:/[^"]*?/(?i:lah-nuxtjs|lah-fe)/dist""#) {
            updated = re_fe_doc.replace_all(&updated, format!(r#"${{1}}"{fe_dist_fwd}""#)).to_string();
        }
        if let Ok(re_fe_dir) = Regex::new(r#"(?s)<Directory\s+"[a-zA-Z]:/[^"]*?/(?i:lah-nuxtjs|lah-fe)/dist">"#) {
            updated = re_fe_dir.replace_all(&updated, format!(r#"<Directory "{fe_dist_fwd}">"#)).to_string();
        }
    }

    if updated != orig {
        let _ = fs::write(&conf_path, updated);
    }
}

/// 依據還原後之 LAH-NUXTJS/.env 立即重新產出 dist/runtime-config.js 與 static/runtime-config.js
pub fn sync_fe_runtime_config(root: &Path) -> bool {
    let fe_dir = if root.join("LAH-NUXTJS").is_dir() {
        root.join("LAH-NUXTJS")
    } else if root.join("LAH-FE").is_dir() {
        root.join("LAH-FE")
    } else {
        return false;
    };

    if !fe_dir.join("dist").is_dir() && !fe_dir.join("static").is_dir() {
        return false;
    }

    let mut env_map = serde_json::Map::new();
    for env_candidate in &[fe_dir.join(".env"), fe_dir.join(".env.example")] {
        if let Ok(raw) = fs::read_to_string(env_candidate) {
            for line in raw.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some(eq_idx) = trimmed.find('=') {
                    let key = trimmed[..eq_idx].trim();
                    let mut val = trimmed[eq_idx + 1..].trim();
                    if let Some(hash_idx) = val.find(" #") {
                        val = val[..hash_idx].trim();
                    }
                    val = val.trim_matches(|c| c == '"' || c == '\'');
                    if !key.is_empty() {
                        env_map.insert(key.to_string(), serde_json::Value::String(val.to_string()));
                    }
                }
            }
            break;
        }
    }

    let get_str = |k: &str, default: &str| -> String {
        env_map
            .get(k)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(default)
            .to_string()
    };
    let get_bool = |k: &str| -> bool {
        env_map
            .get(k)
            .and_then(|v| v.as_str())
            .map(|s| s.eq_ignore_ascii_case("true") || s == "1")
            .unwrap_or(false)
    };
    let get_f64 = |k: &str, default: f64| -> f64 {
        env_map
            .get(k)
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(default)
    };

    let pkg_version = fs::read_to_string(fe_dir.join("package.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("version").and_then(|ver| ver.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| "1.0.0".to_string());

    let is_dev = get_str("NODE_ENV", "production") != "production";
    let runtime_cfg = serde_json::json!({
        "isDev": is_dev,
        "pkgVersion": pkg_version,
        "websocketHost": get_str("WEBSOCKET_HOST", "127.0.0.1"),
        "websocketPort": get_str("WEBSOCKET_PORT", "8081"),
        "TestR": get_bool("TEST_RESULT"),
        "monitor": {
            "capacity": {
                "threshold": get_f64("MONITOR_CAPACITY_THRESHOLD", 70.0)
            },
            "host": {
                "L05": get_str("MONITOR_HOST_L05", "http://220.1.33.2:8080")
            },
            "srmas": {
                "host": {
                    "ip": get_str("SRMAS_HOST_IP", "220.1.34.253"),
                    "port": get_str("SRMAS_HOST_PORT", "443")
                },
                "warning": {
                    "cpu": {
                        "temperature": get_f64("SRMAS_WARNING_CPU_TEMP", 70.0),
                        "utilization": get_f64("SRMAS_WARNING_CPU_USAGE", 90.0)
                    },
                    "memory": {
                        "utilization": get_f64("SRMAS_WARNING_RMA_USAGE", 95.0)
                    },
                    "disk": {
                        "utilization": get_f64("SRMAS_WARNING_DISK_USAGE", 90.0)
                    }
                }
            }
        },
        "axios": {
            "browserBaseURL": ""
        },
        "rawEnv": env_map
    });

    let js_content = format!(
        "// 自動由 .env 產出之執行期組態檔 (請勿手動修改，修改 .env 後由 LAH-SCHEDULER 或 LAH-Updater 自動同步)\nwindow.__LAH_CONFIG__ = {};\n",
        serde_json::to_string_pretty(&runtime_cfg).unwrap_or_else(|_| "{}".to_string())
    );

    let mut wrote = false;
    for sub in ["dist", "static"] {
        let dir = fe_dir.join(sub);
        if dir.is_dir() {
            let target_js = dir.join("runtime-config.js");
            if fs::write(&target_js, &js_content).is_ok() {
                wrote = true;
            }
        }
    }
    wrote
}

/// 若目標環境已部署靜態前端 (dist/index.html)，自動自還原的 .pm2/dump.pm2 剔除已退役之 LAH-FE 並正規化路徑
pub fn sanitize_restored_pm2_dump(root: &Path) -> bool {
    let has_static_fe = root.join("LAH-NUXTJS").join("dist").join("index.html").is_file()
        || root.join("LAH-FE").join("dist").join("index.html").is_file();
    let dump_path = root.join(".pm2").join("dump.pm2");
    if !dump_path.is_file() {
        return false;
    }
    let Ok(raw) = fs::read_to_string(&dump_path) else {
        return false;
    };
    let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return false;
    };
    let Some(arr) = val.as_array_mut() else {
        return false;
    };

    if has_static_fe {
        arr.retain(|item| item.get("name").and_then(|v| v.as_str()) != Some("LAH-FE"));
    }

    let pm2_home_str = root.join(".pm2").to_string_lossy().to_string();
    for item in arr.iter_mut() {
        if let Some(obj) = item.as_object_mut() {
            if let Some(interp) = obj.get("exec_interpreter").and_then(|v| v.as_str()) {
                if interp.to_lowercase().contains("node") {
                    obj.insert("exec_interpreter".to_string(), serde_json::Value::String("node".to_string()));
                }
            }
            obj.insert("PM2_HOME".to_string(), serde_json::Value::String(pm2_home_str.clone()));
            obj.remove("PM2_INSTALL_DIRECTORY");
            obj.remove("PM2_SERVICE_DIRECTORY");
            if let Some(env_obj) = obj.get_mut("env").and_then(|v| v.as_object_mut()) {
                env_obj.insert("PM2_HOME".to_string(), serde_json::Value::String(pm2_home_str.clone()));
                env_obj.remove("PM2_INSTALL_DIRECTORY");
                env_obj.remove("PM2_SERVICE_DIRECTORY");
            }
        }
    }

    if let Ok(new_json) = serde_json::to_string_pretty(&val) {
        return fs::write(&dump_path, new_json).is_ok();
    }
    false
}

pub fn restore(root: &Path, zip_path: &Path, mut on_progress: impl FnMut(Progress)) -> Result<RestoreReport, String> {
    if !root.is_dir() {
        return Err(format!("安裝根目錄不存在或無法存取: {}", root.display()));
    }
    let running = running_services(root);
    if !running.is_empty() {
        return Err(format!("請先停止服務再還原，仍在執行: {}", running.join(", ")));
    }
    let file = File::open(zip_path).map_err(|e| format!("無法開啟備份檔: {e}"))?;
    let mut archive = ZipArchive::new(BufReader::new(file)).map_err(|e| format!("備份檔格式錯誤: {e}"))?;
    let entries = plan(&mut archive, root)?;
    let total = entries.len().max(1) as f64;

    on_progress(Progress { percent: 2.0, message: "正在建立還原前復原點...".into() });
    let existing: Vec<&(usize, PathBuf)> = entries.iter().filter(|(_, p)| p.is_file()).collect();
    let mut undo_zip = None;
    if !existing.is_empty() {
        let dir = zip_path.parent().unwrap_or(Path::new("."));
        let path = dir.join(format!("restore_undo_{}.zip", Local::now().format("%Y%m%d_%H%M%S")));
        let mut zw = ZipWriter::new(File::create(&path).map_err(|e| format!("建立復原點失敗: {e}"))?);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated).large_file(true);
        for (_, target) in &existing {
            let arc = target.strip_prefix(root).unwrap_or(target).to_string_lossy().replace('\\', "/");
            zw.start_file(arc, opts).map_err(|e| e.to_string())?;
            let mut rd = BufReader::new(File::open(target).map_err(|e| format!("復原點讀取失敗 {}: {e}", target.display()))?);
            io::copy(&mut rd, &mut zw).map_err(|e| e.to_string())?;
        }
        zw.finish().map_err(|e| format!("寫入復原點失敗: {e}"))?;
        undo_zip = Some(path.to_string_lossy().into_owned());
    }

    let mut restored = 0u64;
    for (n, (idx, target)) in entries.iter().enumerate() {
        on_progress(Progress {
            percent: 5.0 + (n as f64 / total) * 90.0,
            message: format!("正在還原: {}", target.strip_prefix(root).unwrap_or(target).display()),
        });
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("建立目錄失敗 {}: {e}", dir.display()))?;
        }
        let mut entry = archive.by_index(*idx).map_err(|e| e.to_string())?;
        let mut out = File::create(target).map_err(|e| format!("寫入失敗 {}: {e}", target.display()))?;
        io::copy(&mut entry, &mut out).map_err(|e| format!("寫入失敗 {}: {e}", target.display()))?;
        restored += 1;
    }

    // 還原後自動校正 Apache 設定檔路徑、同步前端 runtime-config.js 並淨化 PM2 dump.pm2
    on_progress(Progress { percent: 96.0, message: "正在校準系統設定檔與靜態前端執行期組態...".into() });
    calibrate_httpd_conf(root);
    sync_fe_runtime_config(root);
    sanitize_restored_pm2_dump(root);

    on_progress(Progress { percent: 100.0, message: format!("還原完成，共 {restored} 個檔案") });
    Ok(RestoreReport { restored, undo_zip, undo_files: existing.len() as u64 })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibrate_httpd_conf_and_static_fe_reconciliation() {
        let temp = std::env::temp_dir().join(format!("test_backuper_calibrate_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let conf_dir = temp.join("Apache24").join("conf");
        let fe_dist = temp.join("LAH-NUXTJS").join("dist");
        let pm2_dir = temp.join(".pm2");
        fs::create_dir_all(&conf_dir).unwrap();
        fs::create_dir_all(&fe_dist).unwrap();
        fs::create_dir_all(&pm2_dir).unwrap();
        fs::write(fe_dist.join("index.html"), "<html></html>").unwrap();
        fs::write(
            temp.join("LAH-NUXTJS").join(".env"),
            "WEBSOCKET_HOST=220.1.34.75\nWEBSOCKET_PORT=8081\n",
        )
        .unwrap();

        let conf_file = conf_dir.join("httpd.conf");
        let old_conf = r#"
Define SRVROOT "C:/AppServ/Apache24"
ServerRoot "${SRVROOT}"
#LoadModule headers_module modules/mod_headers.so
#LoadModule rewrite_module modules/mod_rewrite.so
DocumentRoot "C:/AppServ/www"
<Directory "C:/AppServ/www">
    Options Indexes FollowSymLinks
</Directory>
ErrorLog "|bin/rotatelogs.exe -l C:/AppServ/Apache24/logs/error.%Y.%m.%d.log 100M"
CustomLog "|bin/rotatelogs.exe -l  C:/AppServ/Apache24/logs/access.%Y.%m.%d.log 100M" combined
Alias /LAH-API/api "C:/AppServ/LAH-API/v1"
Alias /LAH-API "C:/AppServ/LAH-API"
Alias /get_user_img.php "C:/AppServ/LAH-API/v1/get_user_img.php"
<Directory "C:/AppServ/LAH-API">
    AllowOverride None
</Directory>
LoadFile "C:/AppServ/php7/php7ts.dll"
LoadModule php7_module "C:/AppServ/php7/php7apache2_4.dll"
PHPIniDir "C:/AppServ/php7/"
        "#;

        fs::write(&conf_file, old_conf).unwrap();

        let test_root = temp.clone();
        let test_root_fwd = test_root.to_string_lossy().replace('\\', "/").trim_end_matches('/').to_string();

        calibrate_httpd_conf(&test_root);
        assert!(sync_fe_runtime_config(&test_root));

        let new_conf = fs::read_to_string(&conf_file).unwrap();
        assert!(new_conf.contains(&format!(r#"Define SRVROOT "{test_root_fwd}/Apache24""#)));
        assert!(new_conf.contains(&format!(r#"DocumentRoot "{test_root_fwd}/www""#)));
        assert!(new_conf.contains(&format!(r#"<Directory "{test_root_fwd}/www">"#)));
        assert!(new_conf.contains(&format!(
            r#"ErrorLog "|bin/rotatelogs.exe -l {test_root_fwd}/logs/apache/error.%Y.%m.%d.log 100M""#
        )));
        assert!(new_conf.contains(&format!(
            r#"CustomLog "|bin/rotatelogs.exe -l  {test_root_fwd}/logs/apache/access.%Y.%m.%d.log 100M" combined"#
        )));
        assert!(new_conf.contains(&format!(r#"Alias /LAH-API/api "{test_root_fwd}/LAH-API/v1""#)));
        assert!(new_conf.contains(&format!(r#"Alias /get_user_img.php "{test_root_fwd}/LAH-API/v1/get_user_img.php""#)));
        assert!(new_conf.contains(&format!(r#"LoadModule php7_module "{test_root_fwd}/php7/php7apache2_4.dll""#)));
        // 驗證自動補齊 :8080 VirtualHost 並解開 headers_module / rewrite_module
        assert!(new_conf.contains(FE_VHOST_BEGIN));
        assert!(new_conf.contains(&format!(r#"DocumentRoot "{test_root_fwd}/LAH-NUXTJS/dist""#)));
        assert!(!new_conf.contains("#LoadModule rewrite_module"));

        let runtime_js = fs::read_to_string(fe_dist.join("runtime-config.js")).unwrap();
        assert!(runtime_js.contains("220.1.34.75"));

        // 驗證 dump.pm2 剔除舊版 LAH-FE 並保留 LAH-SCHEDULER
        fs::write(
            pm2_dir.join("dump.pm2"),
            r#"[{"name":"LAH-FE","exec_interpreter":"C:\\AppServ\\node\\node.exe"},{"name":"LAH-SCHEDULER","exec_interpreter":"C:\\AppServ\\node\\node.exe"}]"#,
        )
        .unwrap();
        assert!(sanitize_restored_pm2_dump(&test_root));
        let dump_after: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(pm2_dir.join("dump.pm2")).unwrap()).unwrap();
        let arr = dump_after.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "LAH-SCHEDULER");
        assert_eq!(arr[0]["exec_interpreter"], "node");

        let _ = fs::remove_dir_all(&temp);
    }
}
