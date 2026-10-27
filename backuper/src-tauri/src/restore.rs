use crate::backup::Progress;
use chrono::Local;
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

const PS_LIST_EXE: &str = "Get-CimInstance Win32_Process -Filter \"Name='httpd.exe' or Name='node.exe'\" | ForEach-Object { $_.ExecutablePath }";

/// 回傳執行檔位於 `root` 底下、仍在執行的 httpd/node 路徑。僅檢查屬於該安裝目錄的行程，
/// 避免被機器上其他無關的 node 誤擋。UNC 根目錄無法檢查本機行程，回傳空陣列。
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
            percent: 5.0 + (n as f64 / total) * 95.0,
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
    on_progress(Progress { percent: 100.0, message: format!("還原完成，共 {restored} 個檔案") });
    Ok(RestoreReport { restored, undo_zip, undo_files: existing.len() as u64 })
}
