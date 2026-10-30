use crate::system::run_cmd_hidden;
use regex::Regex;
use std::fs;
use std::path::Path;

pub fn check_apache_syntax(apache_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, String) {
    let httpd_exe = apache_dir.join("bin").join("httpd.exe");
    if !httpd_exe.is_file() {
        let msg = format!("找不到 httpd.exe: {}", httpd_exe.display());
        log_fn(&format!("⚠️ {}", msg), "warning");
        return (false, msg);
    }

    log_fn("🔍 正在檢驗 Apache 設定檔語法 (httpd.exe -t)...", "info");
    let httpd_str = httpd_exe.to_string_lossy();
    let apache_str = apache_dir.to_string_lossy();
    let (ok, stdout, stderr) = run_cmd_hidden(&httpd_str, &["-t", "-d", &apache_str]);

    let combined = format!("{}\n{}", stdout, stderr);
    if combined.contains("Syntax OK") {
        log_fn("  ✓ Apache 設定檔語法檢查通過 (Syntax OK)！", "success");
        (true, "Syntax OK".to_string())
    } else {
        log_fn(&format!("  ⚠️ Apache 語法檢查警告或錯誤:\n{}", combined.trim()), "warning");
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
                    log_fn(&format!("  ✓ 已依據 {cand} 初始化產生 {ver_label} 設定檔: {}", ini_path.display()), "info");
                    break;
                }
            }
        }
    }

    if !ini_path.is_file() {
        log_fn(&format!("  ⚠️ 找不到 {ver_label} 設定檔或範本: {}", ini_path.display()), "warning");
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

    // 4. PHP 8.5 執行期資源參數調優
    if is_php85 {
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
    }

    if content != orig {
        let _ = fs::write(&ini_path, content);
        log_fn(&format!("  ✓ 已校正並儲存 {ver_label} 設定檔 ({})", ini_path.display()), "success");
    }

    true
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
                    log_fn(&format!("  ✓ 已依據範本 [{}] 初始化建立 httpd.conf", c.display()), "info");
                    restored = true;
                    break;
                }
            }
        }
        if !restored {
            log_fn(&format!("⚠️ 未找到 httpd.conf：{}", conf_path.display()), "warning");
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

    // 4.1 校正日誌路徑 (ErrorLog / CustomLog rotatelogs) 避免舊環境硬編碼
    let apache_logs_fwd = apache_dir.join("logs").to_string_lossy().replace('\\', "/");
    let re_errlog = Regex::new(r#"(?m)^\s*ErrorLog\s+"\|bin/rotatelogs\.exe\s+-l\s+[a-zA-Z]:/[^"]*?/logs/(error\.[^"]+)""#).unwrap();
    if re_errlog.is_match(&content) {
        content = re_errlog.replace_all(&content, format!(r#"ErrorLog "|bin/rotatelogs.exe -l {}/$1""#, apache_logs_fwd)).to_string();
    }
    let re_custlog = Regex::new(r#"(?m)^\s*CustomLog\s+"\|bin/rotatelogs\.exe\s+-l\s+[a-zA-Z]:/[^"]*?/logs/(access\.[^"]+)"\s+(.*)$"#).unwrap();
    if re_custlog.is_match(&content) {
        content = re_custlog.replace_all(&content, format!(r#"CustomLog "|bin/rotatelogs.exe -l  {}/$1" $2"#, apache_logs_fwd)).to_string();
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
                log_fn("  ✓ 已向 Apache httpd.conf 掛載 PHP 7.4 mod_php 模組與相依程式庫", "success");
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
        <Proxy \"fcgi://127.0.0.1:8085/\" enablereuse=on max=16>\n\
            ProxySet timeout=300\n\
        </Proxy>\n"
    );

    // 清理舊版本可能遺留之重複百分比符號語法錯誤
    content = content.replace("%{%REQUEST_FILENAME}", "%{REQUEST_FILENAME}");

    // 清理可能散落或舊安裝根目錄之歷史 Alias 與 Directory (如 C:/AppServ 轉移至 D:/AppServ)
    let re_stale_alias = Regex::new(r#"(?m)^\s*Alias\s+/(?i:lah-api)(?:/api)?\s+"[a-zA-Z]:/[^"]*?"(?:\r?\n)?"#).unwrap();
    content = re_stale_alias.replace_all(&content, "").to_string();

    let re_stale_api_dir = Regex::new(r#"(?s)<Directory\s+"[a-zA-Z]:/[^"]*?/(?i:lah-api)">.*?</Directory>\s*"#).unwrap();
    content = re_stale_api_dir.replace_all(&content, "").to_string();

    let existing_fcgi_re = Regex::new(r#"(?s)# =+\s*# LAH-API \(PHP 8\.5 FastCGI.*?</Proxy>\s*"#).unwrap();
    if existing_fcgi_re.is_match(&content) {
        content = existing_fcgi_re.replace(&content, &fcgi_block).to_string();
    } else {
        content.push_str(&fcgi_block);
        log_fn("  ✓ 已向 Apache 注入 LAH-API (PHP 8.5 FastCGI 127.0.0.1:8085) 轉發設定", "success");
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

    let eco_content = r#"const path = require('path');
const fs = require('fs');

function resolveRootDir() {
  if (process.env.LAH_ROOT && fs.existsSync(process.env.LAH_ROOT)) {
    return path.normalize(process.env.LAH_ROOT);
  }
  const candidates = [
    path.resolve(__dirname, '..'),
    path.resolve(__dirname, '..', '..'),
    'C:\\AppServ'
  ];
  return candidates.find(c => fs.existsSync(path.join(c, 'node')) || fs.existsSync(path.join(c, 'php85'))) || 'C:\\AppServ';
}

const rootDir = resolveRootDir();
const php85Dir = path.join(rootDir, 'php85');
const phpCgi = path.join(php85Dir, 'php-cgi.exe');
const phpIni = path.join(php85Dir, 'php.ini');
const ociDir = path.join(rootDir, 'instantclient_11_2');

module.exports = {
  apps: [
    {
      name: "lah-api-v85",
      script: phpCgi,
      args: `-b 127.0.0.1:8085 -c "${phpIni}"`,
      cwd: __dirname,
      interpreter: "none",
      exec_mode: "fork",
      instances: 1,
      autorestart: true,
      watch: false,
      max_memory_restart: "1G",
      env: {
        PATH: ociDir + ";" + php85Dir + ";" + (process.env.PATH || ""),
        NLS_LANG: "American_America.US7ASCII",
        PHP_FCGI_CHILDREN: "16",
        PHP_FCGI_MAX_REQUESTS: "10000"
      }
    }
  ]
};
"#;
    if fs::write(&eco_path, eco_content).is_ok() {
        log_fn(&format!("  ✓ 已自動建立 PM2 FastCGI 守護設定檔: {}", eco_path.display()), "success");
    }
}

pub fn apply_apache_security_hardening(conf_path: &Path, root_dir: &Path, log_fn: &dyn Fn(&str, &str)) -> (bool, bool) {
    let Ok(mut content) = fs::read_to_string(conf_path) else {
        return (false, false);
    };
    let orig = content.clone();
    let root_fwd = root_dir.to_string_lossy().replace('\\', "/");

    // 1. ServerTokens Prod
    let re_tokens = Regex::new(r#"(?m)^\s*#?\s*ServerTokens\s+.*$"#).unwrap();
    if re_tokens.is_match(&content) {
        content = re_tokens.replace(&content, "ServerTokens Prod").to_string();
    } else {
        content.push_str("\nServerTokens Prod\n");
    }

    // 2. ServerSignature Off
    let re_sig = Regex::new(r#"(?m)^\s*#?\s*ServerSignature\s+.*$"#).unwrap();
    if re_sig.is_match(&content) {
        content = re_sig.replace(&content, "ServerSignature Off").to_string();
    } else {
        content.push_str("\nServerSignature Off\n");
    }

    // 3. PHP2Shell 緩解防護
    let php2shell_block = format!(
        "\n# =====================================================================\n\
        # PHP2Shell (CVE-2024-4577 PHP-CGI Argument Injection & WebShell) Mitigation\n\
        # =====================================================================\n\
        <IfModule rewrite_module>\n\
            RewriteEngine On\n\
            RewriteCond %{{QUERY_STRING}} (?:^|&)%ad [NC,OR]\n\
            RewriteCond %{{THE_REQUEST}} \\s/+(?:cgi-bin|php-cgi)/ [NC]\n\
            RewriteRule ^ - [F,L]\n\
        </IfModule>\n\
        <Directory \"{root_fwd}/www/assets\">\n\
            <FilesMatch \"\\.(php|php5|php7|php8|phtml|phar|cgi|pl|asp|aspx|jsp)$\">\n\
                Require all denied\n\
            </FilesMatch>\n\
        </Directory>\n\
        <Directory \"{root_fwd}/www/export\">\n\
            <FilesMatch \"\\.(php|php5|php7|php8|phtml|phar|cgi|pl|asp|aspx|jsp)$\">\n\
                Require all denied\n\
            </FilesMatch>\n\
        </Directory>\n\
        <Directory \"{root_fwd}/www/uploads\">\n\
            <FilesMatch \"\\.(php|php5|php7|php8|phtml|phar|cgi|pl|asp|aspx|jsp)$\">\n\
                Require all denied\n\
            </FilesMatch>\n\
        </Directory>\n"
    );

    // 清理舊版本可能遺留之重複百分比符號語法錯誤
    content = content
        .replace("%{%QUERY_STRING}", "%{QUERY_STRING}")
        .replace("%{%THE_REQUEST}", "%{THE_REQUEST}")
        .replace("%{%REQUEST_FILENAME}", "%{REQUEST_FILENAME}");

    let existing_sec_re = Regex::new(r#"(?s)# =+\s*# PHP2Shell \(CVE-2024-4577.*?</Directory>\s*"#).unwrap();
    if existing_sec_re.is_match(&content) {
        content = existing_sec_re.replace(&content, &php2shell_block).to_string();
    } else if !content.contains("PHP2Shell (CVE-2024-4577") {
        content.push_str(&php2shell_block);
        log_fn("  ✓ Apache httpd.conf 已套用資安防護與 CVE-2024-4577 PHP2Shell 緩解設定", "success");
    }

    let changed = content != orig;
    if changed {
        let _ = fs::write(conf_path, content);
    }
    (true, changed)
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
        log_fn(&format!("  ✓ 已套用 PHP 安全設定 ({})", ini_path.display()), "success");
    }
    true
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_php_ini_removes_duplicate_and_stale_extension_dir() {
        let temp_dir = std::env::temp_dir().join(format!("lah_test_php_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&temp_dir).unwrap();
        let ini_path = temp_dir.join("php.ini");
        let initial_ini = "\n; Windows directory\n;include_path = \".;c:\\php\\includes\"\n;extension_dir = \"ext\"\nextension_dir = \"C:\\\\AppServ\\\\php7\\\\ext\"\ndate.timezone = \"UTC\"\n";
        fs::write(&ini_path, initial_ini).unwrap();

        let ok = ensure_php_ini_configured(&temp_dir, false, &|_, _| {});
        assert!(ok);

        let final_content = fs::read_to_string(&ini_path).unwrap();
        assert!(!final_content.contains("C:\\\\AppServ"));
        let ext_fwd = temp_dir.join("ext").to_string_lossy().replace('\\', "/");
        assert!(final_content.contains(&format!(r#"extension_dir = "{}""#, ext_fwd)));
        let count = final_content.lines().filter(|l| l.trim().starts_with("extension_dir")).count();
        assert_eq!(count, 1);

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
        assert!(final_conf.contains(&format!(r#"ErrorLog "|bin/rotatelogs.exe -l {}/logs/error.%Y.%m.%d.log 100M""#, apache_fwd)));

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

        let _ = fs::remove_dir_all(&temp_root);
    }
}
