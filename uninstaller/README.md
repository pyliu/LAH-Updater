# LAH-Uninstaller（自動移除工具）

智慧控管與監控系統之自動移除與環境清理工具，基於 Tauri 2 (Rust) 開發。前端為純靜態 HTML/JS（`ui/`），系統清理核心邏輯全在 Rust（`src-tauri/src/`）。

## 核心功能與清理範圍

1. **Windows 服務卸載**：
   - 停止並自 SCM 完全刪除 `LAH_PM2_Service` 與 `Apache2.4` 服務。
2. **守護行程與殘留進程終止**：
   - 執行 `pm2 kill`，並以 PowerShell 遞迴終止所有以目標安裝根目錄運行的 `node.exe` 與 `php-cgi.exe`。
3. **開機自啟動與環境變數清理**：
   - 清理 Windows 工作排程器（`LAH_PM2_Autostart` 等）。
   - 清理使用者與系統啟動資料夾中之自啟動腳本。
   - 自註冊表 `HKCU\Environment` 與 `HKLM\...\Environment` 精準剔除已刪除的 PATH 目錄，並發送 `WM_SETTINGCHANGE` 系統廣播通知。
4. **Apache PHP 設定防護性註銷**：
   - 註銷 `httpd.conf` 中 PHP 8.5 FastCGI 轉發規則與 PHP 7 模組載入指令，備份設定檔並以 `httpd.exe -t` 驗證語法。
5. **程式碼與環境檔案刪除**：
   - 刪除 `LAH-NUXTJS`、`lah-messenger-server`、`LAH-API`、`www`、`Apache24`、`node`、`php85`、`php7`、`instantclient_11_2`、`logs`。
   - 自動清除唯讀屬性（Read-only）並支援鎖定重試。
6. **安裝根目錄深度清理與備份檔保護**：
   - 嚴格保留 `_LAH_Backup_*.zip` 備份檔不被刪除。
   - 保留使用者未勾選之目錄。
   - 磁碟機根路徑防呆，避免誤刪整個硬碟。

## 編譯與發布

```powershell
# 方式 A：執行批次檔
.\build.bat

# 方式 B：執行 Python 發布腳本（自動編譯並輸出至 dist/）
python build.py
```
- 產出檔案：`dist/智慧控管與監控系統 - 自動移除工具.exe`。
