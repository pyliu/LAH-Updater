# LAH-Backuper（備份與還原工具）

獨立於 `updater` / `uninstaller` 之外的系統重要資料備份與還原工具，使用 Tauri 2 (Rust) 開發。前端為純靜態 HTML/JS（`ui/`，內建動態自適應高度無捲軸與 Modal Overlay 彈現式日誌），備份與還原邏輯全在 Rust（`src-tauri/src/`）。

## 備份範圍（完整涵蓋核心資料庫與驅動依賴）
`lah-messenger-server/db`、`lah-messenger-server/uploads`、各專案 `.env`、`www/include/config/AD.php`、`www/assets`、`php85/php.ini`、`php7/php.ini`、`Apache24/conf/httpd.conf`、`logs`、`.pm2`（`dump.pm2`、`dump.pm2.bak`、`module_conf.json`、`conf.js`、`modules`）、`instantclient_11_2`（Oracle 驅動）、`lah-cmd.bat`（專屬維運終端）以及 `php7/ext/` 與 `php85/ext/` 內自訂下載之資料庫擴充套件 DLL（`php_oci8*.dll`、`php_pdo_sqlsrv*.dll`、`php_sqlsrv*.dll`）。

## 還原安全機制
1. 還原前檢查安裝根目錄底下的 `httpd.exe` / `node.exe` 是否仍在執行，有則提示中止。UNC 路徑無法檢查本機行程，會略過此檢查。
2. ZIP 條目含 `..` 或絕對路徑時整個中止（Zip Slip 防護）。
3. 覆蓋前將被覆蓋的檔案存為 `restore_undo_<時間>.zip`（與備份檔同一資料夾），提供即時復原點。
4. UI 需二次確認，防止意外覆寫。
5. 視窗高度動態自適應（`autoFitWindow()`），初始渲染與完成後自動貼齊內容，杜絕滾動條。

## 編譯與發布

```powershell
# 方式 A：執行批次檔
.\build.bat
# 或於根目錄執行：
.\建立自動備份工具.bat

# 方式 B：執行 Python 發布腳本（自動編譯並輸出至 dist/）
python build.py
```
- 產出檔案：`dist/智慧控管與監控系統 - 備份與還原工具.exe`。
