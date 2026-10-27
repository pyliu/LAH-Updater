# LAH-Backuper（備份與還原工具）

獨立於 `updater.py` / `uninstaller.py` 之外的系統重要資料備份與還原工具，使用 Tauri 2 (Rust) 開發。前端為純靜態 HTML/JS（`ui/`），備份與還原邏輯全在 Rust（`src-tauri/src/`）。

## 備份範圍（沿用原 uninstaller 清單）
`lah-messenger-server/db`、`lah-messenger-server/uploads`、各專案 `.env`、`www/include/config/AD.php`、`www/assets`、`php85/php.ini`、`php7/php.ini`、`Apache24/conf/httpd.conf`、`logs`、`.pm2`（`dump.pm2`、`dump.pm2.bak`、`module_conf.json`、`conf.js`、`modules`）。

## 還原安全機制
1. 還原前檢查安裝根目錄底下的 `httpd.exe` / `node.exe` 是否仍在執行，有則中止。UNC 路徑無法檢查本機行程，會略過此檢查。
2. ZIP 條目含 `..` 或絕對路徑時整個中止（Zip Slip 防護）。
3. 覆蓋前將被覆蓋的檔案存為 `restore_undo_<時間>.zip`（與備份檔同一資料夾）。
4. UI 需二次確認。

## 開發與驗證
```powershell
cd LAH-Updater\backuper\src-tauri
cargo build                                   # 編譯（輸出至 C:\Temp\cargo-target\lah-backuper）
cargo run --example backup_cli -- backup  <根目錄> <目的資料夾或 .zip>
cargo run --example backup_cli -- restore <根目錄> <備份.zip>
```
正式打包（`tauri build`）需人工授權；目前 `bundle.active = false`，僅產出單一 exe。
