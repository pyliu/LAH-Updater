# LAH-Updater（系統自動更新工具）

智慧控管與監控系統之自動更新與環境部署工具，基於 Tauri 2 (Rust) 開發。前端為純靜態 HTML/JS（`ui/`），系統更新與服務自動化配置核心全在 Rust（`src-tauri/src/`）。

## 核心功能與更新範圍

1. **多模組套件自動對齊解壓 (Smart Extractor)**：
   - 支援 10 大更新壓縮包（`Apache24.zip`, `php7.zip`, `php85.zip`, `www.zip`, `LAH-API.zip`, `LAH-NUXTJS.zip`, `LAH-NUXTJS-node_modules.zip`, `lah-messenger-server.zip`, `lah-messenger-server-node_modules.zip`, `node.zip`）。
   - **分流目錄優先搜尋**：基礎服務優先至 `base/`、程式更新包優先至 `patch/`、依賴套件包優先至 `node_modules/` 探測，若無則自動平滑回退至執行目錄與既有搜尋路徑。
   - 內建智慧頂層目錄對齊機制（`detect_strip_prefix`），自動去除 Apache Lounge 等多餘一層資料夾。
   - Zip Slip 安全跨越防護與檔案鎖定檢查。
2. **5 階段自動化系統配置引擎 (5-Stage Automation Pipeline)**：
   - **階段 1**：PHP 7.4 `php.ini` 初始化、掛載 Apache `mod_php7` 模組、註冊 `Apache2.4` 服務。
   - **階段 2**：PHP 8.5 `php.ini` 初始化、掛載 Apache FastCGI Proxy (`fcgid` 轉發至 PM2 `lah-api-v85`)。
   - **階段 3**：資安防護（隱藏 Server 標頭、CVE-2024-4577 PHP2Shell 參數注入防護）與 `httpd.exe -t` 語法檢驗。
   - **階段 4：Node.js 綠色環境與 PM2 全域接管引擎**：
     - **系統與使用者雙層註冊表接管**：將 `<根目錄>\node` 置於系統（`HKLM`）與使用者（`HKCU`）`PATH` 最前端，全面清除舊全域 Node/PM2 路徑（`C:\Program Files\nodejs`、`C:\ProgramData\npm`、`C:\ProgramData\pm2`、`*\AppData\Roaming\npm`），避免全域高版本 Node 搶佔。
     - **環境變數淨化與統一**：清除舊版 `pm2-installer` 殘留變數 `PM2_INSTALL_DIRECTORY` 與 `PM2_SERVICE_DIRECTORY`，統一設定 `PM2_HOME=<根目錄>\.pm2`。
     - **舊存檔遷移與 dump.pm2 淨化**：自動自舊全域目錄遷移最新 `dump.pm2` 與 `module_conf.json`，並自動修復存檔中殘留的外部解譯器路徑與 PID 路徑。
     - **舊全域服務安全清除**：自動查詢、停止並刪除舊全域 `pm2.exe` 與 `PM2` 服務，終止殘留 Daemon 進程。
     - **便攜版內建 pm2-logrotate 預先掛載與啟動**：直接將便攜版 `node\node_modules\pm2-logrotate` 預先掛載至 `.pm2\modules\pm2-logrotate`，並在 `.pm2\module_conf.json` 設定啟用日誌壓縮（`compress: true`）；修補 PM2 `NPM.js` 使其在模組缺失時直接回退自便攜版目錄啟動。
     - **WinSW 原生系統服務註冊 (`LAH_PM2_Service`)**：釋出自包含服務核心與常駐守護腳本 (`service_runner.js`)，開機免登入自動拉起受管服務，具備每 60 秒常駐 Watchdog 巡檢與非可攜 Node 搶佔自癒切換機制。
   - **階段 5**：Apache 與 PM2 服務自動重啟、全系統連接埠 (80, 8080, 8081, 8082) 健康巡檢。
3. **初次安裝引導 (Initial Setup Guide)**：
   - 程式啟動時自動檢測預設安裝目錄（如 `C:\AppServ`）。若不存在，即刻彈出引導對話框，提供「一鍵建立預設目錄」、「瀏覽選取其他磁碟」或「稍後手動設定」。
4. **備份還原與更新包清理**：
   - 支援從最新備份包 (`_LAH_Backup_*.zip`) 一鍵還原自訂設定與機敏資料。
   - 更新完成後提供清理詢問對話框，可一鍵刪除安裝壓縮包釋放磁碟空間。

## 系統全域 Node.js 因資安要求升級時的影響說明

當伺服器因資安稽核要求，日後由 IT 或資安人員以官方 MSI 安裝包升級全域 Node.js（例如將 `C:\Program Files\nodejs` 升級至 v22 或 v24）時，**完全不會影響 PM2 與本系統各項前端、排程與即時通受管程式之正常執行**。

自動更新工具與常駐守護服務透過四重隔離機制保證受管程式之執行期穩定性：

1. **Windows 服務層級硬鎖定（`LAH_PM2_Service`）**
   WinSW 服務設定檔 `lah-service.xml` 直接將執行檔路徑絕對指定為 `<根目錄>\node\node.exe`，並將 `<根目錄>\node` 注入服務程序的 `PATH` 最前端。無論作業系統全域環境變數如何變更，背景服務啟動必定使用可攜版 Node.js 16.20.2。
2. **命令列執行檔硬鎖定（`pm2.cmd`）**
   可攜版 `node\pm2.cmd` 腳本透過 `%~dp0` 強制鎖定 `SET "_prog=%dp0%\node.exe"` 與 `SET "PATH=%dp0%;%PATH%"`。手動於命令提示字元輸入 `pm2` 時，必定透過可攜版 Node.js 啟動 PM2 CLI。
3. **PM2 守護進程與子程序解譯器繼承（`process.execPath`）**
   PM2 Daemon 在 `Client.js` 中透過 `process.execPath` 啟動，鎖定為可攜版 `node.exe`。PM2 孵化業務子程序時（`ForkMode.js` 與 `ClusterMode.js`），預設解譯器 `"node"` 會自動解析為當前 Daemon 自身的 `process.execPath`，直接繼承可攜版 Node.js 16.20.2，絕不呼叫系統全域的 `C:\Program Files\nodejs\node.exe`。
4. **環境變數雙層優先權與 60 秒常駐守護自癒**
   官方 Node.js MSI 安裝包預設將路徑附加於系統 `PATH` 最末端，而本工具已將 `<根目錄>\node` 置於系統（`HKLM`）與使用者（`HKCU`）`PATH` 最前端，優先權最高。常駐背景服務 `service_runner.js` 每 60 秒檢查一次 PM2 Daemon 的實體執行路徑，若發現被外部非可攜版 Node 意外搶佔，會自動強制終止並以可攜版 Node 重新喚醒。

> 💡 **注意事項**：若升級系統全域 Node.js 時，有人手動調整「環境變數」將全域路徑移至 `<根目錄>\node` 之前，已運作中的背景服務不受影響；但新開的命令提示字元直接輸入 `node -v` 可能會解析到新版。此時只需重新執行一次本更新工具點擊「立即套用配置與註冊服務」，即可自動將可攜版路徑重新置頂。

## 編譯與發布

```powershell
# 方式 A：執行批次檔
.\build.bat

# 方式 B：執行 Python 發布腳本（自動編譯並輸出至 dist/）
python build.py
```
- 產出檔案：`dist/智慧控管與監控系統 - 自動更新工具.exe`。
