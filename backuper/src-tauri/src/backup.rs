use chrono::Local;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// 備份項目：(相對於安裝根目錄的路徑, ZIP 內部前綴)。與 uninstaller.py 舊版清單一致。
pub const BACKUP_ITEMS: &[(&str, &str)] = &[
    ("lah-messenger-server/db", "lah-messenger-server/db"),
    ("lah-messenger-server/uploads", "lah-messenger-server/uploads"),
    ("lah-messenger-server/.env", "lah-messenger-server/.env"),
    ("LAH-NUXTJS/.env", "LAH-NUXTJS/.env"),
    ("LAH-API/.env", "LAH-API/.env"),
    ("www/.env", "www/.env"),
    ("www/include/config/AD.php", "www/include/config/AD.php"),
    ("www/assets", "www/assets"),
    ("php85/php.ini", "php85/php.ini"),
    ("php7/php.ini", "php7/php.ini"),
    ("Apache24/conf/httpd.conf", "Apache24/conf/httpd.conf"),
    ("logs", "logs"),
];

/// .pm2 項目：來源目錄由 `resolve_pm2_dir` 決定。
pub const PM2_ITEMS: &[&str] = &["dump.pm2", "dump.pm2.bak", "module_conf.json", "conf.js", "modules"];

#[derive(Serialize, Clone)]
pub struct Progress {
    pub percent: f64,
    pub message: String,
}

#[derive(Serialize)]
pub struct BackupReport {
    pub zip_path: Option<String>,
    pub files: u64,
    pub skipped: u64,
    pub size_bytes: u64,
}

fn is_unc(p: &Path) -> bool {
    p.to_string_lossy().starts_with("\\\\")
}

/// 優先 `<root>\.pm2`；不存在時退回 `%USERPROFILE%\.pm2`（UNC 根目錄不退回，以免誤備份本機設定）。
pub fn resolve_pm2_dir(root: &Path) -> PathBuf {
    let primary = root.join(".pm2");
    if primary.is_dir() || is_unc(root) {
        return primary;
    }
    match std::env::var_os("USERPROFILE") {
        Some(up) if Path::new(&up).join(".pm2").is_dir() => Path::new(&up).join(".pm2"),
        _ => primary,
    }
}

fn resolve_zip_path(dest: &Path) -> io::Result<PathBuf> {
    if dest.extension().map_or(false, |e| e.eq_ignore_ascii_case("zip")) {
        if let Some(dir) = dest.parent() {
            fs::create_dir_all(dir)?;
        }
        Ok(dest.to_path_buf())
    } else {
        fs::create_dir_all(dest)?;
        Ok(dest.join(format!("_LAH_Backup_{}.zip", Local::now().format("%Y%m%d_%H%M%S"))))
    }
}

fn collect_items(root: &Path) -> Vec<(PathBuf, String)> {
    let mut items: Vec<(PathBuf, String)> = BACKUP_ITEMS
        .iter()
        .map(|(rel, arc)| (root.join(rel), (*arc).to_string()))
        .collect();
    let pm2 = resolve_pm2_dir(root);
    for name in PM2_ITEMS {
        items.push((pm2.join(name), format!(".pm2/{}", name)));
    }
    items
}

fn add_file(zw: &mut ZipWriter<File>, src: &Path, arcname: &str) -> io::Result<()> {
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated).large_file(true);
    zw.start_file(arcname, opts).map_err(io::Error::other)?;
    let mut rd = BufReader::new(File::open(src)?);
    io::copy(&mut rd, zw)?;
    Ok(())
}

pub fn backup(root: &Path, dest: &Path, mut on_progress: impl FnMut(Progress)) -> Result<BackupReport, String> {
    if !root.is_dir() {
        return Err(format!("安裝根目錄不存在或無法存取: {}", root.display()));
    }
    let zip_path = resolve_zip_path(dest).map_err(|e| format!("無法建立備份目的地: {e}"))?;
    let zip_norm = zip_path.to_string_lossy().to_lowercase();
    let items = collect_items(root);
    let total = items.len() as f64;
    let (mut files, mut skipped) = (0u64, 0u64);

    let file = File::create(&zip_path).map_err(|e| format!("建立備份檔失敗: {e}"))?;
    let mut zw = ZipWriter::new(file);

    for (idx, (src, arc)) in items.iter().enumerate() {
        on_progress(Progress { percent: 5.0 + (idx as f64 / total) * 90.0, message: format!("正在打包: {arc}") });
        if src.is_file() {
            match add_file(&mut zw, src, arc) {
                Ok(()) => files += 1,
                Err(_) => skipped += 1,
            }
        } else if src.is_dir() {
            for entry in WalkDir::new(src).into_iter().filter_map(Result::ok).filter(|e| e.file_type().is_file()) {
                if entry.path().to_string_lossy().to_lowercase() == zip_norm {
                    continue;
                }
                let rel = entry.path().strip_prefix(src).unwrap_or(entry.path());
                let arcname = format!("{}/{}", arc, rel.to_string_lossy().replace('\\', "/"));
                match add_file(&mut zw, entry.path(), &arcname) {
                    Ok(()) => files += 1,
                    Err(_) => skipped += 1,
                }
            }
        }
    }
    zw.finish().map_err(|e| format!("寫入備份檔失敗: {e}"))?;

    if files == 0 {
        let _ = fs::remove_file(&zip_path);
        on_progress(Progress { percent: 100.0, message: "未偵測到需備份的資料".into() });
        return Ok(BackupReport { zip_path: None, files, skipped, size_bytes: 0 });
    }
    let size_bytes = fs::metadata(&zip_path).map(|m| m.len()).unwrap_or(0);
    on_progress(Progress { percent: 100.0, message: format!("備份完成，共 {files} 個檔案") });
    Ok(BackupReport { zip_path: Some(zip_path.to_string_lossy().into_owned()), files, skipped, size_bytes })
}
