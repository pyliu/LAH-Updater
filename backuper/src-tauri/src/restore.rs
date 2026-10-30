use crate::backup::Progress;
use chrono::Local;
use regex::Regex;
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
    let logs_fwd = format!("{apache_fwd}/logs");
    let api_fwd = format!("{root_fwd}/LAH-API");
    let php7_fwd = format!("{root_fwd}/php7");

    let mut updated = content;

    // 1. SRVROOT
    if let Ok(re_srv) = Regex::new(r#"(?m)^\s*Define\s+SRVROOT\s+.*$"#) {
        if re_srv.is_match(&updated) {
            updated = re_srv.replace(&updated, format!(r#"Define SRVROOT "{}""#, apache_fwd)).to_string();
        } else {
            updated = format!("Define SRVROOT \"{}\"\nServerRoot \"${{SRVROOT}}\"\n{}", apache_fwd, updated);
        }
    }

    // 2. DocumentRoot
    if let Ok(re_doc) = Regex::new(r#"(?m)^\s*DocumentRoot\s+"[^"]*""#) {
        updated = re_doc.replace(&updated, format!(r#"DocumentRoot "{}""#, www_fwd)).to_string();
    }

    // 3. Directory block for www
    if let Ok(re_old_www) = Regex::new(r#"(?s)<Directory\s+"[a-zA-Z]:/[^"]*?/www">"#) {
        updated = re_old_www.replace_all(&updated, format!(r#"<Directory "{}">"#, www_fwd)).to_string();
    }

    // 4. Rotatelogs 日誌路徑
    if let Ok(re_errlog) = Regex::new(r#"(?m)^\s*ErrorLog\s+"\|bin/rotatelogs\.exe\s+-l\s+[a-zA-Z]:/[^"]*?/logs/(error\.[^"]+)""#) {
        updated = re_errlog.replace_all(&updated, format!(r#"ErrorLog "|bin/rotatelogs.exe -l {}/$1""#, logs_fwd)).to_string();
    }
    if let Ok(re_custlog) = Regex::new(r#"(?m)^\s*CustomLog\s+"\|bin/rotatelogs\.exe\s+-l\s+[a-zA-Z]:/[^"]*?/logs/(access\.[^"]+)"\s+(.*)$"#) {
        updated = re_custlog.replace_all(&updated, format!(r#"CustomLog "|bin/rotatelogs.exe -l  {}/$1" $2"#, logs_fwd)).to_string();
    }

    // 5. FastCGI / LAH-API Alias 與 Directory
    if let Ok(re_api_alias) = Regex::new(r#"(?m)^\s*Alias\s+/(?i:lah-api)/api\s+"[a-zA-Z]:/[^"]*?""#) {
        updated = re_api_alias.replace_all(&updated, format!(r#"Alias /LAH-API/api "{api_fwd}/v1""#)).to_string();
    }
    if let Ok(re_api_root) = Regex::new(r#"(?m)^\s*Alias\s+/(?i:lah-api)\s+"[a-zA-Z]:/[^"]*?""#) {
        updated = re_api_root.replace_all(&updated, format!(r#"Alias /LAH-API "{api_fwd}""#)).to_string();
    }
    if let Ok(re_stream) = Regex::new(r#"(?m)^\s*Alias\s+/(get_[a-zA-Z0-9_]+\.(?:php|pdf))\s+"[a-zA-Z]:/[^"]*?""#) {
        updated = re_stream.replace_all(&updated, format!(r#"Alias /$1 "{api_fwd}/v1/$1""#)).to_string();
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

    if updated != orig {
        let _ = fs::write(&conf_path, updated);
    }
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

    // 還原後自動校正 Apache 設定檔路徑，避免舊環境覆蓋造成路徑失效
    on_progress(Progress { percent: 96.0, message: "正在校準系統設定檔與路徑...".into() });
    calibrate_httpd_conf(root);

    on_progress(Progress { percent: 100.0, message: format!("還原完成，共 {restored} 個檔案") });
    Ok(RestoreReport { restored, undo_zip, undo_files: existing.len() as u64 })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibrate_httpd_conf() {
        let temp = std::env::temp_dir().join("test_backuper_calibrate");
        let conf_dir = temp.join("Apache24").join("conf");
        let _ = fs::create_dir_all(&conf_dir);
        let conf_file = conf_dir.join("httpd.conf");

        let old_conf = r#"
Define SRVROOT "C:/AppServ/Apache24"
ServerRoot "${SRVROOT}"
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

        let new_conf = fs::read_to_string(&conf_file).unwrap();
        assert!(new_conf.contains(&format!(r#"Define SRVROOT "{test_root_fwd}/Apache24""#)));
        assert!(new_conf.contains(&format!(r#"DocumentRoot "{test_root_fwd}/www""#)));
        assert!(new_conf.contains(&format!(r#"<Directory "{test_root_fwd}/www">"#)));
        assert!(new_conf.contains(&format!(r#"ErrorLog "|bin/rotatelogs.exe -l {test_root_fwd}/Apache24/logs/error.%Y.%m.%d.log 100M""#)));
        assert!(new_conf.contains(&format!(r#"Alias /LAH-API/api "{test_root_fwd}/LAH-API/v1""#)));
        assert!(new_conf.contains(&format!(r#"Alias /get_user_img.php "{test_root_fwd}/LAH-API/v1/get_user_img.php""#)));
        assert!(new_conf.contains(&format!(r#"LoadModule php7_module "{test_root_fwd}/php7/php7apache2_4.dll""#)));

        let _ = fs::remove_dir_all(&temp);
    }
}
