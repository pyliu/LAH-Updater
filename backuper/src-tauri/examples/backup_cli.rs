use lah_backuper_lib::{backup, restore};
use std::path::Path;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let show = |p: backup::Progress| println!("[{:>5.1}%] {}", p.percent, p.message);
    let code = match (a.get(1).map(String::as_str), a.len()) {
        (Some("backup"), 4) => match backup::backup(Path::new(&a[2]), Path::new(&a[3]), show) {
            Ok(r) => { println!("zip={:?} files={} skipped={} bytes={}", r.zip_path, r.files, r.skipped, r.size_bytes); 0 }
            Err(e) => { eprintln!("失敗: {e}"); 1 }
        },
        (Some("restore"), 4) => match restore::restore(Path::new(&a[2]), Path::new(&a[3]), show) {
            Ok(r) => { println!("restored={} undo={:?} undo_files={}", r.restored, r.undo_zip, r.undo_files); 0 }
            Err(e) => { eprintln!("失敗: {e}"); 1 }
        },
        _ => { eprintln!("用法: backup_cli backup <根目錄> <目的地> | restore <根目錄> <zip>"); 2 }
    };
    std::process::exit(code);
}
