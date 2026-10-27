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

    // 1. extension_dir
    let re_ext = Regex::new(r"(?m)^\s*;?\s*extension_dir\s*=.*$").unwrap();
    if re_ext.is_match(&content) {
        content = re_ext.replace(&content, format!(r#"extension_dir = "{}""#, ext_fwd)).to_string();
    } else {
        content.push_str(&format!(r#"\nextension_dir = "{}"\n"#, ext_fwd));
    }

    // 2. 常用擴充功能啟用
    let target_exts = if is_php85 {
        vec!["curl", "fileinfo", "gd", "mbstring", "mysqli", "openssl", "pdo_sqlite", "sqlite3"]
    } else {
        vec!["curl", "fileinfo", "gd2", "mbstring", "mysqli", "openssl", "pdo_sqlite", "sqlite3", "oci8_11g"]
    };

    for ext in target_exts {
        let pattern = format!(r"(?m)^\s*;\s*extension\s*=\s*{}\b", ext);
        if let Ok(re) = Regex::new(&pattern) {
            content = re.replace_all(&content, format!("extension={}", ext)).to_string();
        }
    }

    // 3. date.timezone
    let re_tz = Regex::new(r"(?m)^\s*;?\s*date\.timezone\s*=.*$").unwrap();
    if re_tz.is_match(&content) {
        content = re_tz.replace(&content, r#"date.timezone = "Asia/Taipei""#).to_string();
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

                let existing_p7_re = Regex::new(r#"(?s)# =+\s*# PHP 7\.4 \(mod_php7?\).*?</FilesMatch>"#).unwrap();
                if existing_p7_re.is_match(&content) {
                    content = existing_p7_re.replace(&content, &php7_block).to_string();
                } else if !content.contains("php7_module") {
                    content.push_str(&php7_block);
                    log_fn("  ✓ 已向 Apache httpd.conf 掛載 PHP 7.4 mod_php 模組與相依程式庫", "success");
                }
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

    // 2. 注入 Proxy 轉發設定
    let fcgi_block = format!(
        "\n# =====================================================================\n\
        # LAH-API (PHP 8.5 FastCGI via PM2 on 127.0.0.1:8085)\n\
        # =====================================================================\n\
        Alias /LAH-API/api \"{api_fwd}/v1\"\n\
        Alias /lah-api/api \"{api_fwd}/v1\"\n\
        Alias /LAH-API \"{api_fwd}\"\n\
        Alias /lah-api \"{api_fwd}\"\n\
        <Directory \"{api_fwd}\">\n\
            Header set Access-Control-Allow-Origin \"*\"\n\
            Options -Indexes +FollowSymLinks\n\
            AllowOverride None\n\
            Require all granted\n\
            <FilesMatch \"^\\.|\\.(json|lock|xml|bat|ps1|md|ini|config|yml|yaml)$\">\n\
                Require all denied\n\
            </FilesMatch>\n\
            <FilesMatch \"\\.php$\">\n\
                <If \"-f %{{%REQUEST_FILENAME}}\">\n\
                    SetHandler \"proxy:fcgi://127.0.0.1:8085/\"\n\
                </If>\n\
            </FilesMatch>\n\
        </Directory>\n\
        <Proxy \"fcgi://127.0.0.1:8085/\" enablereuse=on max=16>\n\
            ProxySet timeout=300\n\
        </Proxy>\n"
    );

    if !content.contains("proxy:fcgi://127.0.0.1:8085/") {
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
            RewriteCond %{{%QUERY_STRING}} (?:^|&)%ad [NC,OR]\n\
            RewriteCond %{{%THE_REQUEST}} \\s/+(?:cgi-bin|php-cgi)/ [NC]\n\
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

    if !content.contains("PHP2Shell (CVE-2024-4577") {
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
