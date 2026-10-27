# LAH-Updater（系統自動更新工具）

智慧控管與監控系統之自動更新與環境部署工具，基於 Tauri 2 (Rust) 開發。前端為純靜態 HTML/JS（`ui/`），系統更新與服務自動化配置核心全在 Rust（`src-tauri/src/`）。

## 核心功能與更新範圍

1. **多模組套件自動對齊解壓 (Smart Extractor)**：
   - 支援 10 大更新壓縮包（`Apache24.zip`, `php7.zip`, `php85.zip`, `www.zip`, `LAH-API.zip`, `LAH-NUXTJS.zip`, `LAH-NUXTJS-node_modules.zip`, `lah-messenger-server.zip`, `lah-messenger-server-node_modules.zip`, `node.zip`）。
   - 內建智慧頂層目錄對齊機制（`detect_strip_prefix`），自動去除 Apache Lounge 等多餘一層資料夾。
   - Zip Slip 安全跨越防護與檔案鎖定檢查。
2. **5 階段自動化系統配置引擎 (5-Stage Automation Pipeline)**：
   - **階段 1**：PHP 7.4 `php.ini` 初始化、掛載 Apache `mod_php7` 模組、註冊 `Apache2.4` 服務。
   - **階段 2**：PHP 8.5 `php.ini` 初始化、掛載 Apache FastCGI Proxy (`fcgid` 轉發至 PM2 `lah-api-v85`)。
   - **階段 3**：資安防護（隱藏 Server 標頭、CVE-2024-4577 PHP2Shell 參數注入防護）與 `httpd.exe -t` 語法檢驗。
   - **階段 4**：Node.js 綠色環境 PATH 註冊、PM2 打補丁、內嵌 WinSW 服務部署 (`LAH_PM2_Service`)。
   - **階段 5**：Apache 與 PM2 服務自動重啟、全系統連接埠 (80, 8080, 8081, 8082) 健康巡檢。
3. **初次安裝引導 (Initial Setup Guide)**：
   - 程式啟動時自動檢測預設安裝目錄（如 `C:\AppServ`）。若不存在，即刻彈出引導對話框，提供「一鍵建立預設目錄」、「瀏覽選取其他磁碟」或「稍後手動設定」。
4. **備份還原與更新包清理**：
   - 支援從最新備份包 (`_LAH_Backup_*.zip`) 一鍵還原自訂設定與機敏資料。
   - 更新完成後提供清理詢問對話框，可一鍵刪除安裝壓縮包釋放磁碟空間。

## 編譯與發布

```powershell
# 方式 A：執行批次檔
.\build.bat

# 方式 B：執行 Python 發布腳本（自動編譯並輸出至 dist/）
python build.py
```
- 產出檔案：`dist/智慧控管與監控系統 - 自動更新工具.exe`。
