use crate::system::run_cmd_hidden;
use regex::{NoExpand, Regex};
use std::fs;
use std::path::Path;

pub fn check_apache_syntax(apache_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    let httpd_exe = apache_dir.join("bin").join("httpd.exe");
    if !httpd_exe.is_file() {
        let msg = format!("找不到 httpd.exe: {}", httpd_exe.display());
        log_fn(&format!("警告：{}", msg), "warning");
        return (false, msg);
    }

    log_fn("正在檢驗 Apache 設定檔語法 (httpd.exe -t)...", "info");
    let httpd_str = httpd_exe.to_string_lossy();
    let apache_str = apache_dir.to_string_lossy();
    let (ok, stdout, stderr) = run_cmd_hidden(&httpd_str, &["-t", "-d", &apache_str]);

    let combined = format!("{}\n{}", stdout, stderr);
    if combined.contains("Syntax OK") {
        log_fn("  Apache 設定檔語法檢查通過 (Syntax OK)！", "success");
        (true, "Syntax OK".to_string())
    } else {
        log_fn(&format!("  警告：Apache 語法檢查警告或錯誤:\n{}", combined.trim()), "warning");
        (ok, combined)
    }
}

pub fn ensure_php_ini_configured(php_dir: &Path, is_php85: bool, log_fn: &dyn Fn(&str, &str)) -> bool {
    if !php_dir.is_dir() {
        return false;
    }

    let ini_path = php_dir.join("php.ini");
    let ver_label = if is_php85 { "PHP 8.5" } else { "PHP 7.4" };

    if !ini_path.is_file() {
        for cand in &["php.ini-production", "php.ini-development"] {
            let cand_path = php_dir.join(cand);
            if cand_path.is_file() {
                if fs::copy(&cand_path, &ini_path).is_ok() {
                    log_fn(&format!("  已依據 {cand} 初始化產生 {ver_label} 設定檔: {}", ini_path.display()), "info");
                    break;
                }
            }
        }
    }

    if !ini_path.is_file() {
        log_fn(&format!("  警告：找不到 {ver_label} 設定檔或範本: {}", ini_path.display()), "warning");
        return false;
    }

    let Ok(mut content) = fs::read_to_string(&ini_path) else {
        return false;
    };
    let orig = content.clone();
    let ext_fwd = php_dir.join("ext").to_string_lossy().replace('\\', "/");

    // 1. extension_dir (清理所有殘留設定並保證僅保留單一精準之 extension_dir)
    let re_ext = Regex::new(r"(?m)^\s*;?\s*extension_dir\s*=.*$").unwrap();
    if re_ext.is_match(&content) {
        let mut first = true;
        let mut lines = Vec::new();
        for line in content.lines() {
            if re_ext.is_match(line) {
                if first {
                    lines.push(format!(r#"extension_dir = "{}""#, ext_fwd));
                    first = false;
                }
            } else {
                lines.push(line.to_string());
            }
        }
        content = lines.join("\r\n");
    } else {
        content.push_str(&format!(r#"\r\nextension_dir = "{}"\r\n"#, ext_fwd));
    }

    // 2. 常用與必備擴充功能啟用 (檢查 ext 目錄實際存在的 DLL)
    let target_exts = if is_php85 {
        vec!["curl", "fileinfo", "gd", "imap", "intl", "ldap", "mbstring", "odbc", "openssl", "pdo_odbc", "pdo_sqlite", "sockets", "sqlite3", "zip"]
    } else {
        vec!["curl", "fileinfo", "gd2", "imap", "ldap", "mbstring", "openssl", "pdo_sqlite", "sqlite3", "oci8_11g"]
    };

    let ext_dir = php_dir.join("ext");
    for ext in target_exts {
        let dll_path = ext_dir.join(format!("php_{}.dll", ext));
        if !dll_path.is_file() {
            continue;
        }

        let active_pat = format!(r"(?m)^\s*extension\s*=\s*(?:php_)?{}(?:\.dll)?\b", regex::escape(ext));
        if let Ok(re_act) = Regex::new(&active_pat) {
            if re_act.is_match(&content) {
                continue;
            }
        }

        let commented_pat = format!(r"(?m)^\s*;\s*extension\s*=\s*(?:php_)?{}(?:\.dll)?\s*(?:;.*)?$", regex::escape(ext));
        let mut enabled = false;
        if let Ok(re_com) = Regex::new(&commented_pat) {
            if re_com.is_match(&content) {
                content = re_com.replace(&content, format!("extension={}", ext)).to_string();
                enabled = true;
            }
        }

        if !enabled {
            content.push_str(&format!("\nextension={}\n", ext));
        }
    }

    // 3. date.timezone
    let re_tz = Regex::new(r"(?m)^\s*;?\s*date\.timezone\s*=.*$").unwrap();
    if re_tz.is_match(&content) {
        content = re_tz.replace(&content, r#"date.timezone = "Asia/Taipei""#).to_string();
    } else {
        content.push_str("\ndate.timezone = \"Asia/Taipei\"\n");
    }

    // 4. PHP 7.4 / 8.5 執行期資源參數統一調優
    let re_met = Regex::new(r"(?m)^\s*;?\s*max_execution_time\s*=.*$").unwrap();
    if re_met.is_match(&content) {
        content = re_met.replace(&content, "max_execution_time = 300").to_string();
    } else {
        content.push_str("\nmax_execution_time = 300\n");
    }

    let re_mem = Regex::new(r"(?m)^\s*;?\s*memory_limit\s*=.*$").unwrap();
    if re_mem.is_match(&content) {
        content = re_mem.replace(&content, "memory_limit = 512M").to_string();
    } else {
        content.push_str("\nmemory_limit = 512M\n");
    }

    if content != orig {
        let _ = fs::write(&ini_path, content);
        log_fn(&format!("  已校正並儲存 {ver_label} 設定檔 ({})", ini_path.display()), "success");
    }

    true
}

pub fn normalize_apache_log_directives(content: &str, apache_logs_fwd: &str) -> String {
    let mut out = content.to_string();

    let re_errlog_rotate = Regex::new(
        r#"(?m)^([ \t]*)ErrorLog[ \t]+"\|(?:[^"]*?/)?rotatelogs(?:\.exe)?[ \t]+-l[ \t]+[^"]*?(error\.[^"]+)"[ \t]*(\r?)$"#,
    )
    .unwrap();
    if re_errlog_rotate.is_match(&out) {
        out = re_errlog_rotate
            .replace_all(&out, |caps: &regex::Captures| {
                let indent = &caps[1];
                let err_pattern = &caps[2];
                let cr = &caps[3];
                format!(
                    r#"{indent}ErrorLog "|bin/rotatelogs.exe -l {apache_logs_fwd}/{err_pattern}"{cr}"#
                )
            })
            .to_string();
    } else {
        let re_errlog_plain =
            Regex::new(r#"(?m)^([ \t]*)ErrorLog[ \t]+"[^"]*"[ \t]*(\r?)$"#).unwrap();
        if re_errlog_plain.is_match(&out) {
            out = re_errlog_plain
                .replace_all(&out, |caps: &regex::Captures| {
                    let indent = &caps[1];
                    let cr = &caps[2];
                    format!(
                        r#"{indent}ErrorLog "|bin/rotatelogs.exe -l {apache_logs_fwd}/error.%Y.%m.%d.log 100M"{cr}"#
                    )
                })
                .to_string();
        }
    }

    let re_custlog_rotate = Regex::new(
        r#"(?m)^([ \t]*)CustomLog[ \t]+"\|(?:[^"]*?/)?rotatelogs(?:\.exe)?[ \t]+-l[ \t]+[^"]*?(access\.[^"]+)"[ \t]+(\S+)[ \t]*(\r?)$"#,
    )
    .unwrap();
    if re_custlog_rotate.is_match(&out) {
        out = re_custlog_rotate
            .replace_all(&out, |caps: &regex::Captures| {
                let indent = &caps[1];
                let acc_pattern = &caps[2];
                let fmt_name = &caps[3];
                let cr = &caps[4];
                format!(
                    r#"{indent}CustomLog "|bin/rotatelogs.exe -l  {apache_logs_fwd}/{acc_pattern}" {fmt_name}{cr}"#
                )
            })
            .to_string();
    } else {
        let re_custlog_plain = Regex::new(
            r#"(?m)^([ \t]*)CustomLog[ \t]+"[^"]*"[ \t]+(?:common|combined)[ \t]*(\r?)$"#,
        )
        .unwrap();
        if re_custlog_plain.is_match(&out) {
            out = re_custlog_plain
                .replace_all(&out, |caps: &regex::Captures| {
                    let indent = &caps[1];
                    let cr = &caps[2];
                    format!(
                        r#"{indent}CustomLog "|bin/rotatelogs.exe -l  {apache_logs_fwd}/access.%Y.%m.%d.log 100M" combined{cr}"#
                    )
                })
                .to_string();
        }
    }

    out
}

pub fn configure_apache_mod_php7(apache_dir: &Path, root_dir: &Path, php7_dir: Option<&Path>, log_fn: &dyn Fn(&str, &str)) -> (bool, bool) {
    let conf_path = apache_dir.join("conf").join("httpd.conf");
    if !conf_path.is_file() {
        // 嘗試自範本檔初始化
        let candidates = [
            apache_dir.join("conf").join("httpd.conf.default"),
            apache_dir.join("conf").join("original").join("httpd.conf"),
            root_dir.join("LAH-API").join("httpd.conf-ref"),
            root_dir.join("LAH-API").join("API").join("httpd.conf-ref"),
        ];
        let mut restored = false;
        for c in &candidates {
            if c.is_file() {
                let _ = fs::create_dir_all(apache_dir.join("conf"));
                if fs::copy(c, &conf_path).is_ok() {
                    log_fn(&format!("  已依據範本 [{}] 初始化建立 httpd.conf", c.display()), "info");
                    restored = true;
                    break;
                }
            }
        }
        if !restored {
            log_fn(&format!("警告：未找到 httpd.conf：{}", conf_path.display()), "warning");
            return (false, false);
        }
    }

    let Ok(mut content) = fs::read_to_string(&conf_path) else {
        return (false, false);
    };
    let orig = content.clone();

    let apache_fwd = apache_dir.to_string_lossy().replace('\\', "/");
    let www_fwd = root_dir.join("www").to_string_lossy().replace('\\', "/");

    // 1. SRVROOT
    let re_srv = Regex::new(r#"(?m)^\s*Define\s+SRVROOT\s+.*$"#).unwrap();
    if re_srv.is_match(&content) {
        content = re_srv.replace(&content, format!(r#"Define SRVROOT "{}""#, apache_fwd)).to_string();
    } else {
        content = format!("Define SRVROOT \"{}\"\nServerRoot \"${{SRVROOT}}\"\n{}", apache_fwd, content);
    }

    // 2. 模組載入 headers / rewrite
    for mod_name in &["headers_module", "rewrite_module"] {
        let pat = format!(r#"(?m)^\s*#\s*(LoadModule\s+{}\s+modules/mod_{}\.so)"#, mod_name, mod_name.trim_end_matches("_module"));
        if let Ok(re) = Regex::new(&pat) {
            content = re.replace(&content, "$1").to_string();
        }
    }

    // 2.1 ServerName (避免啟動時 AH00558 警告)
    let re_sname = Regex::new(r#"(?m)^\s*#\s*ServerName\s+www\.example\.com:80.*$"#).unwrap();
    if re_sname.is_match(&content) {
        content = re_sname.replace(&content, "ServerName localhost:80").to_string();
    }

    // 3. DocumentRoot 與目錄權限
    let re_doc = Regex::new(r#"(?m)^\s*DocumentRoot\s+"[^"]*""#).unwrap();
    content = re_doc.replace(&content, format!(r#"DocumentRoot "{}""#, www_fwd)).to_string();

    let www_dir_block = format!(
        "<Directory \"{www_fwd}\">\n    Header set Access-Control-Allow-Origin \"*\"\n    Options -Indexes +FollowSymLinks\n    AllowOverride None\n    Require all granted\n</Directory>"
    );
    let re_htdocs = Regex::new(r#"(?s)<Directory\s+"\$\{SRVROOT\}/htdocs">.*?</Directory>"#).unwrap();
    if re_htdocs.is_match(&content) {
        content = re_htdocs.replace(&content, &www_dir_block).to_string();
    } else {
        let re_old_www = Regex::new(r#"(?s)<Directory\s+"[a-zA-Z]:/[^"]*?/www">.*?</Directory>"#).unwrap();
        if re_old_www.is_match(&content) {
            content = re_old_www.replace(&content, &www_dir_block).to_string();
        }
    }

    // 4. DirectoryIndex
    let re_idx = Regex::new(r#"(?m)^\s*DirectoryIndex\s+.*$"#).unwrap();
    if re_idx.is_match(&content) && !content.contains("index.php") {
        content = re_idx.replace(&content, "DirectoryIndex index.php index.html").to_string();
    }

    // 4.1 校正日誌路徑 (ErrorLog / CustomLog rotatelogs) 歸集至統一日誌儲存庫 (logs/apache)
    let unified_apache_logs_dir = root_dir.join("logs").join("apache");
    let _ = fs::create_dir_all(&unified_apache_logs_dir);
    let apache_logs_fwd = unified_apache_logs_dir.to_string_lossy().replace('\\', "/");
    let before_logs = content.clone();
    content = normalize_apache_log_directives(&content, &apache_logs_fwd);
    if content != before_logs {
        log_fn(
            "  已將 Apache 日誌路徑 (ErrorLog / CustomLog) 歸集至統一日誌儲存庫 (logs/apache)",
            "success",
        );
    }

    // 5. mod_php7 與相依 DLL (php7ts, libcrypto, libssl, libssh2, nghttp2, libsqlite3, libsodium, libsasl)
    if let Some(p7) = php7_dir {
        if p7.is_dir() {
            let p7_fwd = p7.to_string_lossy().replace('\\', "/");
            let p7_dll = p7.join("php7apache2_4.dll");
            if p7_dll.is_file() {
                let preload_dlls = [
                    "php7ts.dll",
                    "libcrypto-1_1-x64.dll",
                    "libssl-1_1-x64.dll",
                    "libssh2.dll",
                    "nghttp2.dll",
                    "libsqlite3.dll",
                    "libsodium.dll",
                    "libsasl.dll",
                ];
                let mut load_lines = Vec::new();
                for dll_name in &preload_dlls {
                    if p7.join(dll_name).is_file() {
                        load_lines.push(format!(r#"LoadFile "{}/{dll_name}""#, p7_fwd));
                    }
                }
                let load_files_str = load_lines.join("\n");
                let php7_block = format!(
                    "\n# =====================================================================\n\
                    # PHP 7.4 (mod_php7) Configuration\n\
                    # =====================================================================\n\
                    {load_files_str}\n\
                    LoadModule php7_module \"{p7_fwd}/php7apache2_4.dll\"\n\
                    PHPIniDir \"{p7_fwd}\"\n\
                    <FilesMatch \\.php$>\n\
                        SetHandler application/x-httpd-php\n\
                    </FilesMatch>\n"
                );

                // 移除既有標準 PHP 7.4 區塊
                let existing_p7_re = Regex::new(r#"(?s)# =+\s*# PHP 7\.4 \(mod_php7?\).*?</FilesMatch>\s*"#).unwrap();
                if existing_p7_re.is_match(&content) {
                    content = existing_p7_re.replace_all(&content, "").to_string();
                }

                // 徹底清理所有殘留或舊版 AppServ 散落之舊設定 (LoadFile、LoadModule、PHPIniDir、mod_php7.c)
                let re_old_loadfiles = Regex::new(r#"(?m)^\s*LoadFile\s+"[a-zA-Z]:/[^"]*?/php7/[^"]+"\r?\n"#).unwrap();
                content = re_old_loadfiles.replace_all(&content, "").to_string();

                let re_old_loadmod = Regex::new(r#"(?m)^\s*LoadModule\s+php7_module\s+.*?\r?\n"#).unwrap();
                content = re_old_loadmod.replace_all(&content, "").to_string();

                let re_old_phpinidir = Regex::new(r#"(?m)^\s*PHPIniDir\s+.*?\r?\n"#).unwrap();
                content = re_old_phpinidir.replace_all(&content, "").to_string();

                let re_old_modphp7_c = Regex::new(r#"(?s)<IfModule\s+mod_php7\.c>.*?</IfModule>\s*"#).unwrap();
                content = re_old_modphp7_c.replace_all(&content, "").to_string();

                content.push_str(&php7_block);
                log_fn("  已向 Apache httpd.conf 掛載 PHP 7.4 mod_php 模組與相依程式庫", "success");
            }
        }
    }

    let changed = content != orig;
    if changed {
        let _ = fs::write(&conf_path, content);
    }
    (true, changed)
}

pub fn configure_apache_fcgi_proxy(apache_dir: &Path, root_dir: &Path, php85_dir: Option<&Path>, log_fn: &dyn Fn(&str, &str)) -> (bool, bool) {
    let conf_path = apache_dir.join("conf").join("httpd.conf");
    if !conf_path.is_file() {
        return (false, false);
    }

    let api_dir = root_dir.join("LAH-API");
    if !api_dir.is_dir() && !php85_dir.map_or(false, |p| p.is_dir()) {
        return (true, false);
    }

    let Ok(mut content) = fs::read_to_string(&conf_path) else {
        return (false, false);
    };
    let orig = content.clone();
    let api_fwd = api_dir.to_string_lossy().replace('\\', "/");

    // 1. 啟用 proxy_module, proxy_fcgi_module
    for m in &["proxy_module", "proxy_fcgi_module"] {
        let pat = format!(r#"(?m)^\s*#\s*(LoadModule\s+{}\s+modules/mod_{}\.so)"#, m, m.trim_end_matches("_module"));
        if let Ok(re) = Regex::new(&pat) {
            content = re.replace(&content, "$1").to_string();
        }
    }

    // 2. 注入 Proxy 轉發設定 (含 ProxyFCGISetEnvIf 剝除代理前綴以支援 Windows 磁碟絕對路徑)
    let fcgi_block = format!(
        "\n# =====================================================================\n\
        # LAH-API (PHP 8.5 FastCGI via PM2 on 127.0.0.1:8085)\n\
        # =====================================================================\n\
        Alias /LAH-API/api \"{api_fwd}/v1\"\n\
        Alias /lah-api/api \"{api_fwd}/v1\"\n\
        Alias /LAH-API \"{api_fwd}\"\n\
        Alias /lah-api \"{api_fwd}\"\n\
        # 串流端點向後相容映射至 PHP 8.5 FastCGI\n\
        Alias /get_user_img.php \"{api_fwd}/v1/get_user_img.php\"\n\
        Alias /get_carousel_img.php \"{api_fwd}/v1/get_carousel_img.php\"\n\
        Alias /get_blob_img.php \"{api_fwd}/v1/get_blob_img.php\"\n\
        Alias /get_adm_reserve_pdf.php \"{api_fwd}/v1/get_adm_reserve_pdf.php\"\n\
        Alias /get_reg_foreigner_pdf.php \"{api_fwd}/v1/get_reg_foreigner_pdf.php\"\n\
        Alias /get_sur_destruction_pdf.php \"{api_fwd}/v1/get_sur_destruction_pdf.php\"\n\
        <Directory \"{api_fwd}\">\n\
            Header set Access-Control-Allow-Origin \"*\"\n\
            Options -Indexes +FollowSymLinks\n\
            AllowOverride None\n\
            Require all granted\n\
            <FilesMatch \"^\\.|\\.(json|lock|xml|bat|ps1|md|ini|config|yml|yaml)$\">\n\
                Require all denied\n\
            </FilesMatch>\n\
            <FilesMatch \"\\.php$\">\n\
                <If \"-f %{{REQUEST_FILENAME}}\">\n\
                    SetHandler \"proxy:fcgi://127.0.0.1:8085/\"\n\
                    ProxyFCGISetEnvIf \"reqenv('SCRIPT_FILENAME') =~ m#^proxy:fcgi://127\\.0\\.0\\.1:8085/(.*)$#\" SCRIPT_FILENAME \"$1\"\n\
                </If>\n\
            </FilesMatch>\n\
        </Directory>\n\
        <Proxy \"fcgi://127.0.0.1:8085/\" enablereuse=off max=16>\n\
            ProxySet timeout=300\n\
        </Proxy>\n"
    );

    // 清理舊版本可能遺留之重複百分比符號語法錯誤
    content = content.replace("%{%REQUEST_FILENAME}", "%{REQUEST_FILENAME}");

    // 清理可能散落或舊安裝根目錄之歷史 Alias 與 Directory (如 C:/AppServ 轉移至 D:/AppServ)
    let re_stale_alias = Regex::new(r#"(?m)^\s*Alias\s+/(?:(?i:lah-api)(?:/api)?|get_(?:user_img|carousel_img|blob_img|adm_reserve_pdf|reg_foreigner_pdf|sur_destruction_pdf)\.php)\s+"[a-zA-Z]:/[^"]*?"(?:\r?\n)?"#).unwrap();
    content = re_stale_alias.replace_all(&content, "").to_string();

    let re_stale_api_dir = Regex::new(r#"(?s)<Directory\s+"[a-zA-Z]:/[^"]*?/(?i:lah-api)">.*?</Directory>\s*"#).unwrap();
    content = re_stale_api_dir.replace_all(&content, "").to_string();

    let existing_fcgi_re = Regex::new(r#"(?s)# =+\s*# LAH-API \(PHP 8\.5 FastCGI.*?</Proxy>\s*"#).unwrap();
    if existing_fcgi_re.is_match(&content) {
        content = existing_fcgi_re.replace(&content, NoExpand(&fcgi_block)).to_string();
    } else {
        content.push_str(&fcgi_block);
        log_fn("  已向 Apache 注入 LAH-API (PHP 8.5 FastCGI 127.0.0.1:8085) 轉發設定", "success");
    }

    ensure_lah_api_ecosystem(&api_dir, php85_dir, log_fn);

    let changed = content != orig;
    if changed {
        let _ = fs::write(&conf_path, content);
    }
    (true, changed)
}

pub fn ensure_lah_api_ecosystem(api_dir: &Path, _php85_dir: Option<&Path>, log_fn: &dyn Fn(&str, &str)) {
    if !api_dir.is_dir() {
        return;
    }
    let eco_path = api_dir.join("ecosystem.config.js");
    if eco_path.is_file() {
        return;
    }

    let fallback_root = api_dir
        .parent()
        .unwrap_or(api_dir)
        .to_string_lossy()
        .replace('\\', "\\\\");

    let eco_content = format!(
        r#"const path = require('path');
const fs = require('fs');

function resolveRootDir() {{
  if (process.env.LAH_ROOT && fs.existsSync(process.env.LAH_ROOT)) {{
    return path.normalize(process.env.LAH_ROOT);
  }}
  const candidates = [
    path.resolve(__dirname, '..'),
    path.resolve(__dirname, '..', '..'),
    '{fallback_root}'
  ];
  return candidates.find(c => fs.existsSync(path.join(c, 'node')) || fs.existsSync(path.join(c, 'php85'))) || '{fallback_root}';
}}

const rootDir = resolveRootDir();
const php85Dir = path.join(rootDir, 'php85');
const phpCgi = path.join(php85Dir, 'php-cgi.exe');
const phpIni = path.join(php85Dir, 'php.ini');
const ociDir = [process.env.OCI_DIR, path.join(rootDir, 'instantclient_11_2'), path.resolve(__dirname, '..', 'instantclient_11_2'), 'C:\\Tools\\instantclient_11_2', 'C:\\AppServ\\instantclient_11_2'].find(c => c && fs.existsSync(c)) || path.join(rootDir, 'instantclient_11_2');

module.exports = {{
  apps: [
    {{
      name: "lah-api-v85",
      script: phpCgi,
      args: `-b 127.0.0.1:8085 -c "${{phpIni}}"`,
      cwd: __dirname,
      interpreter: "none",
      exec_mode: "fork",
      instances: 1,
      autorestart: true,
      watch: false,
      max_memory_restart: "1G",
      env: {{
        PATH: ociDir + ";" + php85Dir + ";" + (process.env.PATH || ""),
        NLS_LANG: "American_America.US7ASCII",
        PHP_FCGI_CHILDREN: "16",
        PHP_FCGI_MAX_REQUESTS: "10000"
      }}
    }}
  ]
}};
"#
    );
    if fs::write(&eco_path, eco_content).is_ok() {
        log_fn(&format!("  已自動建立 PM2 FastCGI 守護設定檔: {}", eco_path.display()), "success");
    }
}

fn build_lah_security_conf() -> String {
    let php2shell_block = format!(
        "# =====================================================================\n\
# PHP2Shell (CVE-2024-4577 PHP-CGI Argument Injection & WebShell) Mitigation\n\
# =====================================================================\n\
<IfModule rewrite_module>\n\
    RewriteEngine On\n\
    RewriteCond %{{QUERY_STRING}} (?:^|&)%ad [NC,OR]\n\
    RewriteCond %{{THE_REQUEST}} \\s/+(?:cgi-bin|php-cgi)/ [NC]\n\
    RewriteRule ^ - [F,L]\n\
</IfModule>\n\
<Directory \"${{LAH_ROOT}}/www/assets\">\n\
    <FilesMatch \"\\.(php|php5|php7|php8|phtml|phar|cgi|pl|asp|aspx|jsp)$\">\n\
        Require all denied\n\
    </FilesMatch>\n\
</Directory>\n\
<Directory \"${{LAH_ROOT}}/www/export\">\n\
    <FilesMatch \"\\.(php|php5|php7|php8|phtml|phar|cgi|pl|asp|aspx|jsp)$\">\n\
        Require all denied\n\
    </FilesMatch>\n\
</Directory>\n\
<Directory \"${{LAH_ROOT}}/www/uploads\">\n\
    <FilesMatch \"\\.(php|php5|php7|php8|phtml|phar|cgi|pl|asp|aspx|jsp)$\">\n\
        Require all denied\n\
    </FilesMatch>\n\
</Directory>\n\
{PHP2SHELL_END}"
    );

    let sensitive_block = build_sensitive_deny_block("${LAH_ROOT}");
    let mpm_block = build_mpm_winnt_block();
    let node2shell_block = build_node2shell_mitigation_block();

    format!(
        "# =====================================================================\n\
# LAH Managed Apache Security & Memory Governance Configuration\n\
# 此檔案由 LAH-Updater 自動覆寫維護，請勿手動修改\n\
# =====================================================================\n\n\
ServerTokens Prod\n\
ServerSignature Off\n\n\
{php2shell_block}\n\n\
{sensitive_block}\n\n\
{mpm_block}\n\n\
{node2shell_block}\n"
    )
}

/// 清理 `httpd.conf` 內既有的舊版資安標記區塊與指令（含無結尾標記之重複殘留區塊）。
fn strip_legacy_security_blocks(raw: &str) -> String {
    let mut content = raw
        .replace("%{%QUERY_STRING}", "%{QUERY_STRING}")
        .replace("%{%THE_REQUEST}", "%{THE_REQUEST}")
        .replace("%{%REQUEST_FILENAME}", "%{REQUEST_FILENAME}");
    let mut removed_any = false;

    for pat in [
        r"(?m)^[ \t]*ServerTokens[ \t]+.*\r?\n?",
        r"(?m)^[ \t]*ServerSignature[ \t]+.*\r?\n?",
    ] {
        let re = Regex::new(pat).unwrap();
        if re.is_match(&content) {
            content = re.replace_all(&content, "").to_string();
            removed_any = true;
        }
    }

    let existing_sec_re = Regex::new(&format!(
        r"(?s)# =+\s*# PHP2Shell \(CVE-2024-4577.*?{}\s*",
        regex::escape(PHP2SHELL_END)
    ))
    .unwrap();
    if existing_sec_re.is_match(&content) {
        content = existing_sec_re.replace_all(&content, "").to_string();
        removed_any = true;
    }

    let legacy_sec_re =
        Regex::new(r#"(?s)# =+\s*# PHP2Shell \(CVE-2024-4577.*?</Directory>\s*"#).unwrap();
    if legacy_sec_re.is_match(&content) {
        content = legacy_sec_re.replace_all(&content, "").to_string();
        removed_any = true;
    }

    let stray_dir_re = Regex::new(
        r#"(?s)<Directory\s+"[^"]*/www/(?:assets|export|uploads)">.*?</Directory>\s*"#,
    )
    .unwrap();
    if stray_dir_re.is_match(&content) {
        let mut stray_removed = false;
        content = stray_dir_re
            .replace_all(&content, |caps: &regex::Captures| -> String {
                let matched = caps.get(0).map(|m| m.as_str()).unwrap_or("");
                if matched.contains("Require all denied") {
                    stray_removed = true;
                    String::new()
                } else {
                    matched.to_string()
                }
            })
            .to_string();
        if stray_removed {
            removed_any = true;
        }
    }

    for (begin, end) in [
        (SENSITIVE_DENY_BEGIN, SENSITIVE_DENY_END),
        (MPM_WINNT_BEGIN, MPM_WINNT_END),
        (NODE2SHELL_BEGIN, NODE2SHELL_END),
    ] {
        let block_re = Regex::new(&format!(
            r"(?s){}.*?{}\s*",
            regex::escape(begin),
            regex::escape(end)
        ))
        .unwrap();
        if block_re.is_match(&content) {
            content = block_re.replace_all(&content, "").to_string();
            removed_any = true;
        }
    }

    if removed_any {
        let multi_blank_re = Regex::new(r"(?m)(?:\r?\n){3,}").unwrap();
        content = multi_blank_re.replace_all(&content, "\n\n").to_string();
    }

    content
}

pub fn apply_apache_security_hardening_with_checker<F>(
    conf_path: &Path,
    root_dir: &Path,
    syntax_checker: F,
    log_fn: &dyn Fn(&str, &str),
) -> (bool, bool)
where
    F: Fn(&Path, &dyn Fn(&str, &str)) -> (bool, String),
{
    let Ok(orig_httpd) = fs::read_to_string(conf_path) else {
        return (false, false);
    };

    let cleaned_httpd = strip_legacy_security_blocks(&orig_httpd);
    let new_httpd =
        upsert_managed_includes_block(&cleaned_httpd, root_dir, &["conf/extra/lah-security.conf"]);
    let new_sec = build_lah_security_conf();

    let conf_dir = conf_path.parent().unwrap_or_else(|| Path::new("."));
    let sec_conf_path = conf_dir.join("extra").join("lah-security.conf");
    let apache_dir = if conf_dir.file_name().and_then(|n| n.to_str()) == Some("conf") {
        conf_dir.parent().unwrap_or(conf_dir)
    } else {
        conf_dir
    };

    let (ok, changed) = write_apache_configs_transactionally(
        apache_dir,
        &[
            (conf_path, new_httpd.as_str()),
            (sec_conf_path.as_path(), new_sec.as_str()),
        ],
        syntax_checker,
        log_fn,
    );
    if ok && changed {
        log_fn(
            "  Apache 已套用受管資安設定 (conf/extra/lah-security.conf) 並於 httpd.conf 完成 Include 掛載",
            "success",
        );
    }
    (ok, changed)
}

pub fn apply_apache_security_hardening(
    conf_path: &Path,
    root_dir: &Path,
    log_fn: &dyn Fn(&str, &str),
) -> (bool, bool) {
    apply_apache_security_hardening_with_checker(conf_path, root_dir, check_apache_syntax, log_fn)
}

const PHP2SHELL_END: &str = "# === End PHP2Shell Mitigation ===";
const SENSITIVE_DENY_BEGIN: &str = "# === Sensitive Files Deny (.env / SQLite / dot-dir) ===";
const SENSITIVE_DENY_END: &str = "# === End Sensitive Files Deny ===";
const MPM_WINNT_BEGIN: &str = "# === LAH MPM WinNT Memory Governance ===";
const MPM_WINNT_END: &str = "# === End LAH MPM WinNT Memory Governance ===";
const NODE2SHELL_BEGIN: &str = "# === Node2Shell (CVE-2026-111497 Brotli OOB Write) Mitigation ===";
const NODE2SHELL_END: &str = "# === End Node2Shell Mitigation ===";
const FE_VHOST_BEGIN: &str = "# === LAH-FE Static VirtualHost (:8080) ===";
const FE_VHOST_END: &str = "# === End LAH-FE Static VirtualHost ===";
pub const MANAGED_INCLUDES_BEGIN: &str = "# === LAH Managed Includes ===";
pub const MANAGED_INCLUDES_END: &str = "# === End LAH Managed Includes ===";

/// 於 `httpd.conf` 檔尾維護單一受管 Include 區塊，定義 `Define LAH_ROOT` 並等冪合併 `Include` 清單。
pub fn upsert_managed_includes_block(content: &str, root_dir: &Path, include_paths: &[&str]) -> String {
    let root_fwd = root_dir.to_string_lossy().replace('\\', "/");
    let block_re = Regex::new(&format!(
        r"(?s){}.*?{}",
        regex::escape(MANAGED_INCLUDES_BEGIN),
        regex::escape(MANAGED_INCLUDES_END)
    ))
    .unwrap();

    let mut merged_includes: Vec<String> = Vec::new();
    let mut push_unique = |item: &str| {
        let trimmed = item.trim();
        if !trimmed.is_empty() && !merged_includes.iter().any(|existing| existing == trimmed) {
            merged_includes.push(trimmed.to_string());
        }
    };

    if let Some(matched) = block_re.find(content) {
        let inc_re = Regex::new(r"(?m)^\s*Include\s+(.+?)\s*$").unwrap();
        for cap in inc_re.captures_iter(matched.as_str()) {
            if let Some(m) = cap.get(1) {
                push_unique(m.as_str());
            }
        }
    }

    for inc in include_paths {
        push_unique(inc);
    }

    let mut lines = Vec::with_capacity(merged_includes.len() + 3);
    lines.push(MANAGED_INCLUDES_BEGIN.to_string());
    lines.push(format!(r#"Define LAH_ROOT "{}""#, root_fwd));
    for inc in &merged_includes {
        lines.push(format!("Include {}", inc));
    }
    lines.push(MANAGED_INCLUDES_END.to_string());
    let block = lines.join("\n");

    if block_re.is_match(content) {
        block_re.replace(content, NoExpand(&block)).to_string()
    } else {
        let trimmed = content.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            format!("{}\n", block)
        } else {
            format!("{}\n\n{}\n", trimmed, block)
        }
    }
}

fn rollback_apache_config_backups(backups: &[(&Path, Option<Vec<u8>>)], log_fn: &dyn Fn(&str, &str)) {
    for (path, backup) in backups.iter().rev() {
        match backup {
            Some(bytes) => {
                if let Err(err) = fs::write(path, bytes) {
                    log_fn(
                        &format!("錯誤：還原設定檔失敗 ({}): {}", path.display(), err),
                        "error",
                    );
                }
            }
            None => {
                if path.exists() {
                    if let Err(err) = fs::remove_file(path) {
                        log_fn(
                            &format!("錯誤：移除新建設定檔失敗 ({}): {}", path.display(), err),
                            "error",
                        );
                    }
                }
            }
        }
    }
}

/// 交易式寫入 Apache 設定檔：寫入前備份舊檔（或標記為新建），寫入後執行語法檢查；
/// 若檢查失敗則自動還原所有檔案並記錄錯誤，回傳 `(ok, changed)`。
pub fn write_apache_configs_transactionally<F>(
    apache_dir: &Path,
    files: &[(&Path, &str)],
    syntax_checker: F,
    log_fn: &dyn Fn(&str, &str),
) -> (bool, bool)
where
    F: Fn(&Path, &dyn Fn(&str, &str)) -> (bool, String),
{
    if files.is_empty() {
        return (true, false);
    }

    let mut any_changed = false;
    let mut backups: Vec<(&Path, Option<Vec<u8>>)> = Vec::with_capacity(files.len());

    for (path, new_content) in files {
        if path.exists() {
            match fs::read(path) {
                Ok(bytes) => {
                    if bytes != new_content.as_bytes() {
                        any_changed = true;
                    }
                    backups.push((*path, Some(bytes)));
                }
                Err(err) => {
                    log_fn(
                        &format!("錯誤：讀取設定檔備份失敗 ({}): {}", path.display(), err),
                        "error",
                    );
                    return (false, false);
                }
            }
        } else {
            any_changed = true;
            backups.push((*path, None));
        }
    }

    if !any_changed {
        return (true, false);
    }

    for (path, new_content) in files {
        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                log_fn(
                    &format!("錯誤：建立設定檔目錄失敗 ({}): {}", parent.display(), err),
                    "error",
                );
                rollback_apache_config_backups(&backups, log_fn);
                return (false, false);
            }
        }
        if let Err(err) = fs::write(path, new_content) {
            log_fn(
                &format!("錯誤：寫入設定檔失敗 ({}): {}", path.display(), err),
                "error",
            );
            rollback_apache_config_backups(&backups, log_fn);
            return (false, false);
        }
    }

    let (syntax_ok, syntax_msg) = syntax_checker(apache_dir, log_fn);
    if !syntax_ok {
        log_fn(
            &format!(
                "錯誤：Apache 語法檢查失敗，已自動還原所有設定檔：{}",
                syntax_msg.trim()
            ),
            "error",
        );
        rollback_apache_config_backups(&backups, log_fn);
        return (false, false);
    }

    (true, true)
}

pub fn sync_fe_runtime_config(root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    let fe_dir = if root_dir.join("LAH-NUXTJS").is_dir() {
        root_dir.join("LAH-NUXTJS")
    } else if root_dir.join("LAH-FE").is_dir() {
        root_dir.join("LAH-FE")
    } else {
        return false;
    };

    let mut env_map = serde_json::Map::new();

    let parse_env_into = |path: &Path, map: &mut serde_json::Map<String, serde_json::Value>| {
        if let Ok(raw) = fs::read_to_string(path) {
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
                    if !key.is_empty() && !map.contains_key(key) {
                        map.insert(key.to_string(), serde_json::Value::String(val.to_string()));
                    }
                }
            }
        }
    };

    // 1. 優先讀取前端正式 .env，若無讀取 .env.example
    if fe_dir.join(".env").is_file() {
        parse_env_into(&fe_dir.join(".env"), &mut env_map);
    } else if fe_dir.join(".env.example").is_file() {
        parse_env_into(&fe_dir.join(".env.example"), &mut env_map);
    }

    // 2. 合併後端環境變數 (如 www/.env 或 LAH-API/.env 中的 SITE 等核心參數)
    for backend_env in &[
        root_dir.join("www").join(".env"),
        root_dir.join("LAH-API").join(".env"),
    ] {
        if backend_env.is_file() {
            parse_env_into(backend_env, &mut env_map);
            break;
        }
    }

    // 3. 若仍缺少 SITE，由 SQLite dimension.db 探測並補入
    if !env_map.contains_key("SITE") {
        if let Some(probed) = crate::extractor::probe_site_config_from_db(root_dir) {
            env_map.insert("SITE".to_string(), serde_json::Value::String(probed.site));
            if let Some(site_id) = probed.site_id {
                if !env_map.contains_key("SITE_ID") {
                    env_map.insert("SITE_ID".to_string(), serde_json::Value::String(site_id));
                }
            }
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

    let site = get_str("SITE", "");
    let default_srmas_ip = match site.as_str() {
        "HA" => "220.1.34.253",
        "HB" => "220.1.35.253",
        "HC" => "220.1.36.253",
        "HD" => "220.1.37.253",
        "HE" => "220.1.38.253",
        "HF" => "220.1.39.253",
        "HG" => "220.1.40.253",
        "HH" => "220.1.41.253",
        _ => "",
    };
    let srmas_ip = if !get_str("SRMAS_HOST_IP", "").is_empty() {
        get_str("SRMAS_HOST_IP", "")
    } else if !get_str("SRMAS_HOST", "").is_empty() {
        get_str("SRMAS_HOST", "")
    } else {
        default_srmas_ip.to_string()
    };

    let pkg_version = fs::read_to_string(fe_dir.join("package.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("version").and_then(|ver| ver.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| "1.0.0".to_string());

    let is_dev = get_str("NODE_ENV", "production") != "production";
    let runtime_cfg = serde_json::json!({
        "isDev": is_dev,
        "site": site,
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
                    "ip": srmas_ip,
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

    let dist_dir = fe_dir.join("dist");
    let _ = fs::create_dir_all(&dist_dir);
    let target_js = dist_dir.join("runtime-config.js");
    let current = fs::read_to_string(&target_js).unwrap_or_default();
    if current != js_content {
        if fs::write(&target_js, &js_content).is_ok() {
            log_fn(
                &format!("  已同步前端執行期組態檔: {}", target_js.display()),
                "success",
            );
            return true;
        }
    }
    false
}

pub fn configure_apache_fe_static_vhost(
    apache_dir: &Path,
    root_dir: &Path,
    log_fn: &dyn Fn(&str, &str),
) -> (bool, bool) {
    sync_fe_runtime_config(root_dir, log_fn);

    let conf_path = apache_dir.join("conf").join("httpd.conf");
    if !conf_path.is_file() {
        return (false, false);
    }

    let Ok(mut content) = fs::read_to_string(&conf_path) else {
        return (false, false);
    };
    let orig = content.clone();

    let fe_dir = if root_dir.join("LAH-FE").is_dir() && !root_dir.join("LAH-NUXTJS").is_dir() {
        root_dir.join("LAH-FE")
    } else {
        root_dir.join("LAH-NUXTJS")
    };
    let fe_dist_dir = fe_dir.join("dist");
    let _ = fs::create_dir_all(&fe_dist_dir);
    let _ = fs::create_dir_all(root_dir.join("www").join("api"));

    let fe_dist_fwd = fe_dist_dir.to_string_lossy().replace('\\', "/");
    let www_fwd = root_dir.join("www").to_string_lossy().replace('\\', "/");
    let api_fwd = root_dir.join("LAH-API").to_string_lossy().replace('\\', "/");

    // 1. 確保必要模組啟用
    for m in &["headers_module", "rewrite_module", "alias_module"] {
        let pat = format!(
            r#"(?m)^\s*#\s*(LoadModule\s+{}\s+modules/mod_{}\.so)"#,
            m,
            m.trim_end_matches("_module")
        );
        if let Ok(re) = Regex::new(&pat) {
            content = re.replace(&content, "$1").to_string();
        }
    }

    // 2. 先移除區塊外可能散落的 Listen 8080，統一納入 FE_VHOST 標記區塊管理以維持等冪性
    let existing_vhost_re = Regex::new(&format!(
        r"(?s){}.*?{}",
        regex::escape(FE_VHOST_BEGIN),
        regex::escape(FE_VHOST_END)
    ))
    .unwrap();
    let content_without_block = existing_vhost_re.replace_all(&content, "").to_string();
    let re_stray_listen8080 = Regex::new(r#"(?m)^\s*Listen\s+(?:[0-9.]+:)?8080\s*\r?\n"#).unwrap();
    if re_stray_listen8080.is_match(&content_without_block) {
        content = re_stray_listen8080.replace_all(&content, "").to_string();
    }

    let vhost_block = format!(
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
            RewriteCond %{{REQUEST_URI}} !^/(api|LAH-API|lah-api|img|legacy|assets|project|l05proxy|smsproxy|srmas-weathermap|get_)\n\
            RewriteRule ^ /index.html [L]\n\
        </IfModule>\n\
    </Directory>\n\
</VirtualHost>\n\
{FE_VHOST_END}"
    );

    if existing_vhost_re.is_match(&content) {
        content = existing_vhost_re
            .replace(&content, NoExpand(&vhost_block))
            .to_string();
    } else {
        content.push_str(&format!("\n{}\n", vhost_block));
        log_fn(
            "  已向 Apache httpd.conf 掛載 LAH-FE 靜態網站 VirtualHost (*:8080) 與 SPA 路由規則",
            "success",
        );
    }

    let changed = content != orig;
    if changed {
        let _ = fs::write(&conf_path, content);
    }
    (true, changed)
}

fn build_node2shell_mitigation_block() -> String {
    format!(
        "{NODE2SHELL_BEGIN}\n\
<IfModule headers_module>\n\
    <If \"%{{HTTP:Content-Encoding}} =~ /br/i\">\n\
        RequestHeader unset Content-Encoding\n\
    </If>\n\
</IfModule>\n\
{NODE2SHELL_END}"
    )
}

fn build_mpm_winnt_block() -> String {
    format!(
        "{MPM_WINNT_BEGIN}\n\
<IfModule mpm_winnt_module>\n\
    ThreadsPerChild        250\n\
    MaxConnectionsPerChild 10000\n\
    MaxMemFree             2048\n\
</IfModule>\n\
{MPM_WINNT_END}"
    )
}

/// 全域拒絕敏感檔案，不論 DocumentRoot 或 Alias 皆生效。
/// `.env.example` 為前端「下載範本」功能所需，刻意放行。
/// 檔名規則皆不分大小寫，且涵蓋 Windows 結尾 `.`/空白與 `::$DATA` 變體。
fn build_sensitive_deny_block(root_fwd: &str) -> String {
    let root_re = if root_fwd == "${LAH_ROOT}" {
        root_fwd.to_string()
    } else {
        regex::escape(root_fwd)
    };
    format!(
        "{SENSITIVE_DENY_BEGIN}\n\
<FilesMatch \"(?i)^\\.(?!env\\.example$)\">\n\
    Require all denied\n\
</FilesMatch>\n\
<FilesMatch \"(?i)\\.(db|sqlite3?|bak|sql|log)([. ]+|:.*)?$|\\.(db|sqlite3?)-(wal|shm|journal)$\">\n\
    Require all denied\n\
</FilesMatch>\n\
<FilesMatch \"(?i)^composer\\.(json|lock)$\">\n\
    Require all denied\n\
</FilesMatch>\n\
<DirectoryMatch \"(?i)^{root_re}/(?:.*/)?\\.(?!well-known/)[^/]+/\">\n\
    Require all denied\n\
</DirectoryMatch>\n\
{SENSITIVE_DENY_END}"
    )
}

pub fn apply_php_security_hardening(ini_path: &Path, log_fn: &dyn Fn(&str, &str)) -> bool {
    let Ok(mut content) = fs::read_to_string(ini_path) else {
        return false;
    };
    let orig = content.clone();

    let directives = [
        ("expose_php", "Off"),
        ("allow_url_include", "Off"),
        ("cgi.force_redirect", "1"),
        ("enable_dl", "Off"),
    ];

    for (k, v) in &directives {
        let pat = format!(r#"(?m)^\s*;?\s*{}\s*=.*$"#, k);
        if let Ok(re) = Regex::new(&pat) {
            content = re.replace(&content, format!("{} = {}", k, v)).to_string();
        } else {
            content.push_str(&format!("\n{} = {}\n", k, v));
        }
    }

    if content != orig {
        let _ = fs::write(ini_path, content);
        log_fn(&format!("  已套用 PHP 安全設定 ({})", ini_path.display()), "success");
    }
    true
}

/// 檢查目標環境是否已套用資安強化設定 (CVE-2024-4577 緩解、敏感檔案防護、隱藏版本標頭)
/// 必須同時驗證 `httpd.conf` 存在未被註解之有效 `Include conf/extra/lah-security.conf`
/// 且 `conf/extra/lah-security.conf` 實體內容完整。
pub fn is_security_hardened(root_dir: &Path) -> bool {
    let conf_dir = root_dir.join("Apache24").join("conf");
    let apache_conf = conf_dir.join("httpd.conf");
    let sec_conf = conf_dir.join("extra").join("lah-security.conf");
    if !apache_conf.is_file() || !sec_conf.is_file() {
        return false;
    }

    let Ok(httpd_content) = fs::read_to_string(&apache_conf) else {
        return false;
    };
    let include_re =
        Regex::new(r#"(?i)^Include\s+"?conf/extra/lah-security\.conf"?(?:\s+#.*)?$"#).unwrap();
    let has_active_include = httpd_content.lines().any(|line| {
        let trimmed = line.trim();
        !trimmed.starts_with('#') && include_re.is_match(trimmed)
    });
    if !has_active_include {
        return false;
    }

    let Ok(sec_content) = fs::read_to_string(&sec_conf) else {
        return false;
    };
    sec_content.contains(PHP2SHELL_END)
        && sec_content.contains(SENSITIVE_DENY_END)
        && sec_content.contains(MPM_WINNT_END)
        && sec_content.contains(NODE2SHELL_END)
        && sec_content.contains("ServerTokens Prod")
        && sec_content.contains("ServerSignature Off")
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_php_ini_removes_duplicate_and_stale_extension_dir() {
        let temp_dir = std::env::temp_dir().join(format!("lah_test_php_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&temp_dir).unwrap();
        let ini_path = temp_dir.join("php.ini");
        let initial_ini = "\n; Windows directory\n;include_path = \".;c:\\php\\includes\"\n;extension_dir = \"ext\"\nextension_dir = \"C:\\\\AppServ\\\\php7\\\\ext\"\ndate.timezone = \"UTC\"\nmax_execution_time = 0\nmemory_limit = 768M\n";
        fs::write(&ini_path, initial_ini).unwrap();

        let ok = ensure_php_ini_configured(&temp_dir, false, &|_, _| {});
        assert!(ok);

        let final_content = fs::read_to_string(&ini_path).unwrap();
        assert!(!final_content.contains("C:\\\\AppServ"));
        let ext_fwd = temp_dir.join("ext").to_string_lossy().replace('\\', "/");
        assert!(final_content.contains(&format!(r#"extension_dir = "{}""#, ext_fwd)));
        let count = final_content.lines().filter(|l| l.trim().starts_with("extension_dir")).count();
        assert_eq!(count, 1);
        assert!(final_content.contains("max_execution_time = 300"));
        assert!(final_content.contains("memory_limit = 512M"));
        assert!(!final_content.contains("max_execution_time = 0"));
        assert!(!final_content.contains("memory_limit = 768M"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_configure_apache_mod_php7_cleans_legacy_appserv_paths() {
        let temp_root = std::env::temp_dir().join(format!("lah_test_apache_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let apache_dir = temp_root.join("Apache24");
        let conf_dir = apache_dir.join("conf");
        let php7_dir = temp_root.join("php7");
        fs::create_dir_all(&conf_dir).unwrap();
        fs::create_dir_all(&php7_dir).unwrap();
        fs::write(php7_dir.join("php7apache2_4.dll"), b"mock dll").unwrap();
        fs::write(php7_dir.join("php7ts.dll"), b"mock dll").unwrap();

        let conf_path = conf_dir.join("httpd.conf");
        let initial_conf = "\nDefine SRVROOT \"C:/AppServ/Apache24\"\nDocumentRoot \"C:/AppServ/www\"\nErrorLog \"|bin/rotatelogs.exe -l C:/AppServ/Apache24/logs/error.%Y.%m.%d.log 100M\"\nCustomLog \"|bin/rotatelogs.exe -l  C:/AppServ/Apache24/logs/access.%Y.%m.%d.log 100M\" combined\nLoadModule php7_module C:/AppServ/php7/php7apache2_4.dll\n<IfModule mod_php7.c>\n  AddType application/x-httpd-php .php\n  AddType application/x-httpd-php-source .phps\n</IfModule>\nPHPIniDir \"C:/AppServ/php7/\"\n";
        fs::write(&conf_path, initial_conf).unwrap();

        let (ok, changed) = configure_apache_mod_php7(&apache_dir, &temp_root, Some(&php7_dir), &|_, _| {});
        assert!(ok);
        assert!(changed);

        let final_conf = fs::read_to_string(&conf_path).unwrap();
        assert!(!final_conf.contains("C:/AppServ"));
        let p7_fwd = php7_dir.to_string_lossy().replace('\\', "/");
        let apache_fwd = apache_dir.to_string_lossy().replace('\\', "/");
        assert!(final_conf.contains(&format!(r#"Define SRVROOT "{}""#, apache_fwd)));
        assert!(final_conf.contains(&format!(r#"LoadModule php7_module "{}/php7apache2_4.dll""#, p7_fwd)));
        assert!(final_conf.contains(&format!(r#"PHPIniDir "{}""#, p7_fwd)));
        let unified_logs_fwd = temp_root.join("logs").join("apache").to_string_lossy().replace('\\', "/");
        assert!(final_conf.contains(&format!(r#"ErrorLog "|bin/rotatelogs.exe -l {}/error.%Y.%m.%d.log 100M""#, unified_logs_fwd)));
        assert!(final_conf.contains(&format!(r#"CustomLog "|bin/rotatelogs.exe -l  {}/access.%Y.%m.%d.log 100M" combined"#, unified_logs_fwd)));

        // 測試 configure_apache_fcgi_proxy: 驗證自舊根目錄 (C:/AppServ) 遷移至新目錄時的 Alias/Directory 自癒與 ProxyFCGISetEnvIf
        let api_dir = temp_root.join("LAH-API");
        fs::create_dir_all(&api_dir).unwrap();
        let (fcgi_ok, fcgi_changed) = configure_apache_fcgi_proxy(&apache_dir, &temp_root, None, &|_, _| {});
        assert!(fcgi_ok);
        assert!(fcgi_changed);

        let final_fcgi_conf = fs::read_to_string(&conf_path).unwrap();
        let api_fwd = temp_root.join("LAH-API").to_string_lossy().replace('\\', "/");
        assert!(final_fcgi_conf.contains(&format!(r#"Alias /LAH-API "{}""#, api_fwd)));
        assert!(final_fcgi_conf.contains("ProxyFCGISetEnvIf"));
        assert!(!final_fcgi_conf.contains("C:/AppServ"));
        assert!(final_fcgi_conf.contains(r#"SCRIPT_FILENAME "$1""#));

        // 二次覆蓋更新測試：驗證已有 fcgi_block 時以 NoExpand 替換，SCRIPT_FILENAME "$1" 絕不被 Regex::replace 吞噬為空字串
        let (fcgi_ok2, _) = configure_apache_fcgi_proxy(&apache_dir, &temp_root, None, &|_, _| {});
        assert!(fcgi_ok2);
        let final_fcgi_conf2 = fs::read_to_string(&conf_path).unwrap();
        assert!(final_fcgi_conf2.contains(r#"SCRIPT_FILENAME "$1""#));
        assert!(!final_fcgi_conf2.contains(r#"SCRIPT_FILENAME """#));
        assert!(!final_fcgi_conf2.contains(r#"SCRIPT_FILENAME "$$1""#));

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_security_hardening_denies_sensitive_files_idempotently() {
        let temp_root = std::env::temp_dir().join(format!(
            "lah_test_sensitive_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let conf_dir = temp_root.join("Apache24").join("conf");
        fs::create_dir_all(&conf_dir).unwrap();
        let conf_path = conf_dir.join("httpd.conf");
        let sec_conf_path = conf_dir.join("extra").join("lah-security.conf");
        fs::write(&conf_path, "ServerRoot \"C:/x\"\n").unwrap();

        let syntax_ok = |_: &Path, _: &dyn Fn(&str, &str)| (true, "Syntax OK".to_string());
        let (ok, changed) =
            apply_apache_security_hardening_with_checker(&conf_path, &temp_root, syntax_ok, &|_, _| {});
        assert!(ok);
        assert!(changed);
        assert!(sec_conf_path.is_file(), "應產出 conf/extra/lah-security.conf");

        let httpd_first = fs::read_to_string(&conf_path).unwrap();
        let root_fwd = temp_root.to_string_lossy().replace('\\', "/");
        assert!(httpd_first.contains(MANAGED_INCLUDES_BEGIN));
        assert!(httpd_first.contains(&format!(r#"Define LAH_ROOT "{root_fwd}""#)));
        assert!(httpd_first.contains("Include conf/extra/lah-security.conf"));
        assert!(!httpd_first.contains(SENSITIVE_DENY_BEGIN));
        assert!(!httpd_first.contains(MPM_WINNT_BEGIN));
        assert!(!httpd_first.contains(NODE2SHELL_BEGIN));
        assert!(!httpd_first.contains(PHP2SHELL_END));

        let sec_first = fs::read_to_string(&sec_conf_path).unwrap();
        assert!(sec_first.contains("ServerTokens Prod"));
        assert!(sec_first.contains("ServerSignature Off"));
        assert_eq!(sec_first.matches(PHP2SHELL_END).count(), 1);
        assert_eq!(sec_first.matches(SENSITIVE_DENY_BEGIN).count(), 1);
        assert_eq!(sec_first.matches(SENSITIVE_DENY_END).count(), 1);
        assert_eq!(sec_first.matches(MPM_WINNT_BEGIN).count(), 1);
        assert_eq!(sec_first.matches(MPM_WINNT_END).count(), 1);
        assert_eq!(sec_first.matches(NODE2SHELL_BEGIN).count(), 1);
        assert_eq!(sec_first.matches(NODE2SHELL_END).count(), 1);
        assert!(sec_first.contains("RequestHeader unset Content-Encoding"));
        assert!(sec_first.contains("MaxConnectionsPerChild 10000"));
        assert!(sec_first.contains("MaxMemFree             2048"));
        assert!(sec_first.contains(r#"<FilesMatch "(?i)^\.(?!env\.example$)">"#));
        assert!(sec_first.contains(r#"<FilesMatch "(?i)\.(db|sqlite3?|bak|sql|log)([. ]+|:.*)?$|\.(db|sqlite3?)-(wal|shm|journal)$">"#));
        assert!(sec_first.contains(r#"<FilesMatch "(?i)^composer\.(json|lock)$">"#));
        assert!(sec_first.contains(r"\.(?!well-known/)[^/]+/"));
        assert!(sec_first.contains("<DirectoryMatch"));
        assert!(sec_first.contains(r#"<Directory "${LAH_ROOT}/www/assets">"#));
        assert!(sec_first.contains(r#"<Directory "${LAH_ROOT}/www/export">"#));
        assert!(sec_first.contains(r#"<Directory "${LAH_ROOT}/www/uploads">"#));
        assert!(sec_first.contains("(?i)^${LAH_ROOT}/(?:.*/)?\\.(?!well-known/)[^/]+/"));
        assert!(!sec_first.contains(&root_fwd), "lah-security.conf 不應寫死實體路徑");

        let (ok2, changed2) =
            apply_apache_security_hardening_with_checker(&conf_path, &temp_root, syntax_ok, &|_, _| {});
        assert!(ok2);
        assert!(!changed2);
        assert_eq!(fs::read_to_string(&conf_path).unwrap(), httpd_first);
        assert_eq!(fs::read_to_string(&sec_conf_path).unwrap(), sec_first);

        let stale = sec_first
            .replace("Require all denied\n</DirectoryMatch>", "Require all granted\n</DirectoryMatch>")
            .replace("MaxConnectionsPerChild 10000", "MaxConnectionsPerChild 0");
        fs::write(&sec_conf_path, stale).unwrap();
        let (ok3, changed3) =
            apply_apache_security_hardening_with_checker(&conf_path, &temp_root, syntax_ok, &|_, _| {});
        assert!(ok3);
        assert!(changed3);
        let healed = fs::read_to_string(&sec_conf_path).unwrap();
        assert_eq!(healed, sec_first);

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_security_hardening_collapses_legacy_duplicate_blocks() {
        let temp_root = std::env::temp_dir().join(format!(
            "lah_test_legacy_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let conf_dir = temp_root.join("Apache24").join("conf");
        fs::create_dir_all(&conf_dir).unwrap();
        let conf_path = conf_dir.join("httpd.conf");
        let sec_conf_path = conf_dir.join("extra").join("lah-security.conf");
        let root_fwd = temp_root.to_string_lossy().replace('\\', "/");
        let dir = |name: &str| {
            format!(
                "<Directory \"{root_fwd}/www/{name}\">\n<FilesMatch \"\\.(php)$\">\nRequire all denied\n</FilesMatch>\n</Directory>\n"
            )
        };
        let legacy = format!(
            "ServerRoot \"C:/x\"\nServerTokens Full\nServerSignature On\n\
# =====\n# PHP2Shell (CVE-2024-4577 PHP-CGI Argument Injection & WebShell) Mitigation\n# =====\n\
{}{}{}{}{}\n\
{SENSITIVE_DENY_BEGIN}\n# old deny\n{SENSITIVE_DENY_END}\n\
{MPM_WINNT_BEGIN}\n# old mpm\n{MPM_WINNT_END}\n\
{NODE2SHELL_BEGIN}\n# old node2shell\n{NODE2SHELL_END}\n",
            dir("assets"),
            dir("export"),
            dir("uploads"),
            dir("export"),
            dir("uploads")
        );
        fs::write(&conf_path, legacy).unwrap();

        let syntax_ok = |_: &Path, _: &dyn Fn(&str, &str)| (true, "Syntax OK".to_string());
        let (ok, changed) =
            apply_apache_security_hardening_with_checker(&conf_path, &temp_root, syntax_ok, &|_, _| {});
        assert!(ok);
        assert!(changed);

        let httpd_once = fs::read_to_string(&conf_path).unwrap();
        assert!(!httpd_once.contains("PHP2Shell"));
        assert!(!httpd_once.contains(SENSITIVE_DENY_BEGIN));
        assert!(!httpd_once.contains(MPM_WINNT_BEGIN));
        assert!(!httpd_once.contains(NODE2SHELL_BEGIN));
        assert!(!httpd_once.contains("ServerTokens"));
        assert!(!httpd_once.contains("ServerSignature"));
        for name in ["assets", "export", "uploads"] {
            assert!(
                !httpd_once.contains(&format!("{root_fwd}/www/{name}\">")),
                "httpd.conf 不應殘留舊版 {name} Directory 區塊"
            );
        }
        assert!(httpd_once.contains("Include conf/extra/lah-security.conf"));

        let sec_once = fs::read_to_string(&sec_conf_path).unwrap();
        for name in ["assets", "export", "uploads"] {
            assert_eq!(
                sec_once.matches(&format!("${{LAH_ROOT}}/www/{name}\">")).count(),
                1,
                "{name}"
            );
        }
        assert_eq!(sec_once.matches(PHP2SHELL_END).count(), 1);

        let (ok2, changed2) =
            apply_apache_security_hardening_with_checker(&conf_path, &temp_root, syntax_ok, &|_, _| {});
        assert!(ok2);
        assert!(!changed2);
        assert_eq!(fs::read_to_string(&conf_path).unwrap(), httpd_once);
        assert_eq!(fs::read_to_string(&sec_conf_path).unwrap(), sec_once);

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_security_hardening_rolls_back_when_syntax_check_fails() {
        let temp_root = std::env::temp_dir().join(format!(
            "lah_test_sec_rollback_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let conf_dir = temp_root.join("Apache24").join("conf");
        fs::create_dir_all(&conf_dir).unwrap();
        let conf_path = conf_dir.join("httpd.conf");
        let sec_conf_path = conf_dir.join("extra").join("lah-security.conf");
        let orig_httpd = "ServerRoot \"C:/x\"\nListen 80\n";
        fs::write(&conf_path, orig_httpd).unwrap();

        let syntax_fail = |_: &Path, _: &dyn Fn(&str, &str)| (false, "AH00526: Syntax error".to_string());
        let (ok, changed) =
            apply_apache_security_hardening_with_checker(&conf_path, &temp_root, syntax_fail, &|_, _| {});
        assert!(!ok);
        assert!(!changed);
        assert_eq!(fs::read_to_string(&conf_path).unwrap(), orig_httpd);
        assert!(!sec_conf_path.exists());

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_is_security_hardened() {
        let temp_root = std::env::temp_dir().join(format!(
            "lah_test_sec_check_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let conf_dir = temp_root.join("Apache24").join("conf");
        fs::create_dir_all(&conf_dir).unwrap();
        let conf_path = conf_dir.join("httpd.conf");
        let sec_conf_path = conf_dir.join("extra").join("lah-security.conf");

        // 初始未加固狀態
        fs::write(&conf_path, "ServerRoot \"C:/AppServ/Apache24\"\nListen 80\n").unwrap();
        assert!(!is_security_hardened(&temp_root));

        // 僅有舊版內嵌標記（無 Include 與無 lah-security.conf）應回傳 false
        fs::write(
            &conf_path,
            format!(
                "ServerTokens Prod\n{PHP2SHELL_END}\n{SENSITIVE_DENY_END}\n{MPM_WINNT_END}\n{NODE2SHELL_END}\n"
            ),
        )
        .unwrap();
        assert!(!is_security_hardened(&temp_root));

        // 套用資安加固（同時有有效 Include 與完整 lah-security.conf）
        let syntax_ok = |_: &Path, _: &dyn Fn(&str, &str)| (true, "Syntax OK".to_string());
        let (ok, _) =
            apply_apache_security_hardening_with_checker(&conf_path, &temp_root, syntax_ok, &|_, _| {});
        assert!(ok);
        assert!(is_security_hardened(&temp_root));

        // 若 Include 被註解掉，必須回傳 false
        let httpd_content = fs::read_to_string(&conf_path).unwrap();
        let commented_httpd = httpd_content.replace(
            "Include conf/extra/lah-security.conf",
            "# Include conf/extra/lah-security.conf",
        );
        fs::write(&conf_path, &commented_httpd).unwrap();
        assert!(!is_security_hardened(&temp_root));

        // 恢復有效 Include，但將 lah-security.conf 內容破壞（缺少標記），必須回傳 false
        fs::write(&conf_path, &httpd_content).unwrap();
        let sec_content = fs::read_to_string(&sec_conf_path).unwrap();
        fs::write(&sec_conf_path, sec_content.replace(NODE2SHELL_END, "")).unwrap();
        assert!(!is_security_hardened(&temp_root));

        // 刪除 lah-security.conf，必須回傳 false
        fs::remove_file(&sec_conf_path).unwrap();
        assert!(!is_security_hardened(&temp_root));

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_node2shell_mitigation_injection_and_idempotency() {
        let temp_root = std::env::temp_dir().join(format!(
            "lah_test_node2shell_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let conf_dir = temp_root.join("Apache24").join("conf");
        fs::create_dir_all(&conf_dir).unwrap();
        let conf_path = conf_dir.join("httpd.conf");
        let sec_conf_path = conf_dir.join("extra").join("lah-security.conf");
        fs::write(&conf_path, "ServerRoot \"C:/x\"\n").unwrap();

        let syntax_ok = |_: &Path, _: &dyn Fn(&str, &str)| (true, "Syntax OK".to_string());
        let (ok, changed) =
            apply_apache_security_hardening_with_checker(&conf_path, &temp_root, syntax_ok, &|_, _| {});
        assert!(ok);
        assert!(changed);

        let sec_content = fs::read_to_string(&sec_conf_path).unwrap();
        assert_eq!(sec_content.matches(NODE2SHELL_BEGIN).count(), 1);
        assert_eq!(sec_content.matches(NODE2SHELL_END).count(), 1);
        assert!(sec_content.contains(r#"<If "%{HTTP:Content-Encoding} =~ /br/i">"#));
        assert!(sec_content.contains("RequestHeader unset Content-Encoding"));

        // 二次執行保持等冪且不產生變更
        let (ok2, changed2) =
            apply_apache_security_hardening_with_checker(&conf_path, &temp_root, syntax_ok, &|_, _| {});
        assert!(ok2);
        assert!(!changed2);
        assert_eq!(fs::read_to_string(&sec_conf_path).unwrap(), sec_content);
        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_configure_apache_mod_php7_upgrades_default_plain_logs_to_unified_rotatelogs() {
        let temp_root = std::env::temp_dir().join(format!(
            "lah_test_apache_plain_logs_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let apache_dir = temp_root.join("Apache24");
        let conf_dir = apache_dir.join("conf");
        fs::create_dir_all(&conf_dir).unwrap();

        let conf_path = conf_dir.join("httpd.conf");
        let default_conf = concat!(
            "Define SRVROOT \"c:/Apache24\"\r\n",
            "ServerRoot \"${SRVROOT}\"\r\n",
            "DocumentRoot \"${SRVROOT}/htdocs\"\r\n",
            "# ErrorLog: The location of the error log file.\r\n",
            "ErrorLog \"logs/error_log\"\r\n",
            "<IfModule log_config_module>\r\n",
            "    # CustomLog \"logs/access.log\" combined\r\n",
            "    CustomLog \"logs/access_log\" common\r\n",
            "</IfModule>\r\n"
        );
        fs::write(&conf_path, default_conf).unwrap();

        let (ok, changed) = configure_apache_mod_php7(&apache_dir, &temp_root, None, &|_, _| {});
        assert!(ok);
        assert!(changed);

        let unified_logs_fwd = temp_root
            .join("logs")
            .join("apache")
            .to_string_lossy()
            .replace('\\', "/");
        let first_pass = fs::read_to_string(&conf_path).unwrap();
        assert!(
            !first_pass.contains("\"logs/error_log\""),
            "plain ErrorLog was not upgraded:\n{first_pass}"
        );
        assert!(
            !first_pass.contains("\"logs/access_log\""),
            "plain CustomLog was not upgraded:\n{first_pass}"
        );
        assert!(first_pass.contains(&format!(
            r#"ErrorLog "|bin/rotatelogs.exe -l {}/error.%Y.%m.%d.log 100M""#,
            unified_logs_fwd
        )));
        assert!(first_pass.contains(&format!(
            r#"    CustomLog "|bin/rotatelogs.exe -l  {}/access.%Y.%m.%d.log 100M" combined"#,
            unified_logs_fwd
        )));

        let (ok2, changed2) = configure_apache_mod_php7(&apache_dir, &temp_root, None, &|_, _| {});
        assert!(ok2);
        assert!(!changed2);
        let second_pass = fs::read_to_string(&conf_path).unwrap();
        assert_eq!(first_pass, second_pass);

        let dot_log_conf = concat!(
            "Define SRVROOT \"c:/Apache24\"\n",
            "ErrorLog \"logs/error.log\"\n",
            "CustomLog \"logs/access.log\" common\n"
        );
        fs::write(&conf_path, dot_log_conf).unwrap();
        let (ok3, changed3) = configure_apache_mod_php7(&apache_dir, &temp_root, None, &|_, _| {});
        assert!(ok3);
        assert!(changed3);
        let third_pass = fs::read_to_string(&conf_path).unwrap();
        assert!(third_pass.contains(&format!(
            r#"ErrorLog "|bin/rotatelogs.exe -l {}/error.%Y.%m.%d.log 100M""#,
            unified_logs_fwd
        )));
        assert!(third_pass.contains(&format!(
            r#"CustomLog "|bin/rotatelogs.exe -l  {}/access.%Y.%m.%d.log 100M" combined"#,
            unified_logs_fwd
        )));

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_configure_apache_fe_static_vhost_and_runtime_config_idempotent() {
        let temp_root = std::env::temp_dir().join(format!(
            "lah_test_fe_vhost_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let apache_dir = temp_root.join("Apache24");
        let conf_dir = apache_dir.join("conf");
        let fe_dir = temp_root.join("LAH-NUXTJS");
        fs::create_dir_all(&conf_dir).unwrap();
        fs::create_dir_all(&fe_dir).unwrap();
        fs::write(
            fe_dir.join(".env"),
            "WEBSOCKET_HOST=220.1.35.95\nWEBSOCKET_PORT=8081\nMONITOR_HOST_L05=http://220.1.33.2:8080\n",
        )
        .unwrap();
        fs::write(fe_dir.join("package.json"), r#"{"version":"2.5.0"}"#).unwrap();

        let conf_path = conf_dir.join("httpd.conf");
        fs::write(
            &conf_path,
            "Define SRVROOT \"C:/AppServ/Apache24\"\nListen 80\n#LoadModule rewrite_module modules/mod_rewrite.so\nDocumentRoot \"C:/AppServ/www\"\n",
        )
        .unwrap();

        let (ok, changed) = configure_apache_fe_static_vhost(&apache_dir, &temp_root, &|_, _| {});
        assert!(ok);
        assert!(changed);

        let rt_js = fs::read_to_string(fe_dir.join("dist").join("runtime-config.js")).unwrap();
        assert!(rt_js.contains("window.__LAH_CONFIG__"));
        assert!(rt_js.contains("220.1.35.95"));
        assert!(rt_js.contains("2.5.0"));

        let conf1 = fs::read_to_string(&conf_path).unwrap();
        assert_eq!(conf1.matches(FE_VHOST_BEGIN).count(), 1);
        assert_eq!(conf1.matches(FE_VHOST_END).count(), 1);
        assert_eq!(conf1.matches("Listen 8080").count(), 1);
        assert!(conf1.contains("<VirtualHost *:8080>"));
        assert!(conf1.contains("/api/proxy.php?_target=$1&_path=$2"));

        let (ok2, changed2) = configure_apache_fe_static_vhost(&apache_dir, &temp_root, &|_, _| {});
        assert!(ok2);
        assert!(!changed2);
        assert_eq!(fs::read_to_string(&conf_path).unwrap(), conf1);

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_sync_fe_runtime_config_inherits_backend_site_and_aligns_srmas() {
        let temp_root = std::env::temp_dir().join(format!(
            "lah_test_fe_rt_site_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let fe_dir = temp_root.join("LAH-NUXTJS");
        let www_dir = temp_root.join("www");
        fs::create_dir_all(&fe_dir).unwrap();
        fs::create_dir_all(&www_dir).unwrap();

        // 前端刻意無 .env，後端提供 SITE=HE
        fs::write(fe_dir.join("package.json"), r#"{"version":"2.5.0"}"#).unwrap();
        fs::write(www_dir.join(".env"), "SITE=HE\n").unwrap();

        let updated = sync_fe_runtime_config(&temp_root, &|_, _| {});
        assert!(updated);

        let rt_path = fe_dir.join("dist").join("runtime-config.js");
        assert!(rt_path.is_file());
        let content = fs::read_to_string(&rt_path).unwrap();

        // 斷言所別精準繼承且 SRMAS 對齊蘆竹所 220.1.38.253，杜絕桃園所 220.1.34.253 滲漏
        assert!(content.contains(r#""site": "HE""#), "未包含 site: HE, 內容: {}", content);
        assert!(content.contains("220.1.38.253"), "未對齊蘆竹所 SRMAS 220.1.38.253, 內容: {}", content);
        assert!(!content.contains("220.1.34.253"), "錯誤滲漏了桃園所 SRMAS 220.1.34.253! 內容: {}", content);

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_upsert_managed_includes_block_inserts_merges_and_is_idempotent() {
        let orig = "Define SRVROOT \"D:/LAH/Apache24\"\nListen 80\n";
        let root_dir = Path::new(r"D:\LAH");

        // 第一次呼叫：插入單一 Include 與 Define LAH_ROOT
        let once = upsert_managed_includes_block(orig, root_dir, &["conf/extra/lah-security.conf"]);
        assert_eq!(once.matches(MANAGED_INCLUDES_BEGIN).count(), 1);
        assert_eq!(once.matches(MANAGED_INCLUDES_END).count(), 1);
        assert!(once.contains(r#"Define LAH_ROOT "D:/LAH""#));
        assert!(once.contains("Include conf/extra/lah-security.conf"));

        // 第二次呼叫相同參數：內容必須完全等冪
        let twice = upsert_managed_includes_block(&once, root_dir, &["conf/extra/lah-security.conf"]);
        assert_eq!(twice, once);

        // 第三次呼叫新增第二個 Include 並變更根目錄：保留既有 Include、更新 Define LAH_ROOT 且不重複
        let new_root = Path::new(r"E:\LAH_NEW");
        let merged = upsert_managed_includes_block(&twice, new_root, &["conf/extra/lah-fe-vhost.conf"]);
        assert_eq!(merged.matches(MANAGED_INCLUDES_BEGIN).count(), 1);
        assert_eq!(merged.matches("Include conf/extra/lah-security.conf").count(), 1);
        assert_eq!(merged.matches("Include conf/extra/lah-fe-vhost.conf").count(), 1);
        assert!(merged.contains(r#"Define LAH_ROOT "E:/LAH_NEW""#));
        assert!(!merged.contains(r#"Define LAH_ROOT "D:/LAH""#));
    }

    #[test]
    fn test_write_apache_configs_transactionally_rolls_back_on_syntax_failure() {
        let temp_root = std::env::temp_dir().join(format!(
            "lah_test_tx_rollback_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let apache_dir = temp_root.join("Apache24");
        let conf_dir = apache_dir.join("conf");
        fs::create_dir_all(&conf_dir).unwrap();

        let httpd_conf = conf_dir.join("httpd.conf");
        let extra_conf = conf_dir.join("extra").join("lah-security.conf");
        let original_httpd = "# original httpd.conf\nListen 80\n";
        fs::write(&httpd_conf, original_httpd).unwrap();

        let logs = std::cell::RefCell::new(Vec::<(String, String)>::new());
        let log_fn = |msg: &str, level: &str| {
            logs.borrow_mut().push((level.to_string(), msg.to_string()));
        };

        let (ok, changed) = write_apache_configs_transactionally(
            &apache_dir,
            &[
                (httpd_conf.as_path(), "# broken httpd.conf\n"),
                (extra_conf.as_path(), "# new managed conf\n"),
            ],
            |_, _| (false, "AH00526: Syntax error on line 1".to_string()),
            &log_fn,
        );

        assert!(!ok);
        assert!(!changed);
        assert_eq!(fs::read_to_string(&httpd_conf).unwrap(), original_httpd);
        assert!(!extra_conf.exists(), "新建的受管檔案應於語法檢查失敗時被刪除");
        assert!(
            logs.borrow().iter().any(|(lvl, msg)| lvl == "error" && msg.contains("AH00526")),
            "應記錄語法失敗與還原錯誤日誌: {:?}",
            logs.borrow()
        );

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_write_apache_configs_transactionally_commits_on_syntax_ok_and_skips_unchanged() {
        let temp_root = std::env::temp_dir().join(format!(
            "lah_test_tx_commit_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let apache_dir = temp_root.join("Apache24");
        let conf_dir = apache_dir.join("conf");
        fs::create_dir_all(&conf_dir).unwrap();

        let httpd_conf = conf_dir.join("httpd.conf");
        let extra_conf = conf_dir.join("extra").join("lah-security.conf");
        fs::write(&httpd_conf, "# original httpd.conf\n").unwrap();

        let syntax_calls = std::cell::Cell::new(0usize);
        let new_httpd = "# updated httpd.conf\n";
        let new_extra = "# managed security conf\n";

        // 第一次寫入：語法檢查通過，保留更新內容並回傳 (true, true)
        let (ok1, changed1) = write_apache_configs_transactionally(
            &apache_dir,
            &[(httpd_conf.as_path(), new_httpd), (extra_conf.as_path(), new_extra)],
            |_, _| {
                syntax_calls.set(syntax_calls.get() + 1);
                (true, "Syntax OK".to_string())
            },
            &|_, _| {},
        );
        assert!(ok1);
        assert!(changed1);
        assert_eq!(syntax_calls.get(), 1);
        assert_eq!(fs::read_to_string(&httpd_conf).unwrap(), new_httpd);
        assert_eq!(fs::read_to_string(&extra_conf).unwrap(), new_extra);

        // 第二次以相同內容寫入：應略過寫入與語法檢查並回傳 (true, false)
        let (ok2, changed2) = write_apache_configs_transactionally(
            &apache_dir,
            &[(httpd_conf.as_path(), new_httpd), (extra_conf.as_path(), new_extra)],
            |_, _| {
                syntax_calls.set(syntax_calls.get() + 1);
                (true, "Syntax OK".to_string())
            },
            &|_, _| {},
        );
        assert!(ok2);
        assert!(!changed2);
        assert_eq!(syntax_calls.get(), 1);

        let _ = fs::remove_dir_all(&temp_root);
    }
}
