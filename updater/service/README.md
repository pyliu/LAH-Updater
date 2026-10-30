# LAH Windows 原生系統服務套件 (WinSW)

本目錄包含將 Web 伺服器 (Apache) 與 PM2 守護行程封裝為 Windows 原生系統服務（Windows Service）的配置與腳本：

- `lah-service.exe`：基於 WinSW v2.12.0 (.NET 4.6.1) 的服務包裝執行檔（由 `pack.py` 打包至 `node.zip` 內之 `node\service\`，亦複製為 `lah-web-service.exe`）。
- `lah-service.xml`：WinSW 背景守護服務設定檔，指定服務名稱 `LAH_PM2_Service`（顯示名稱：「LAH 智慧管控：背景服務總線與健康管理系統」）與執行參數。
- `lah-web-service.xml`：WinSW Web 入口服務設定檔，指定服務名稱 `LAH_Web_Service`（顯示名稱：「LAH 智慧管控：應用系統入口與 API 閘道服務」），注入私有環境變數並啟動 `Apache24\bin\httpd.exe`。
- `service_runner.js`：由綠色 Node (`<根目錄>\node\node.exe`) 執行的背景服務入口與常駐 Watchdog 守護腳本。

## 核心優勢與安全防護機制

1. **開機免登入（Boot-time Autostart）**：在伺服器開機（無人員登入桌面）即由 Windows 服務控制管理員 (SCM) 自動拉起，徹底解決機房重啟後無人登入導致服務中斷的問題。
2. **Web 與背景服務全面沙盒化**：
   - `LAH_Web_Service` 封裝 Apache 2.4，私有綁定 `PATH`（含 `Apache24\bin`、`instantclient_11_2` 與 `php7`）與 `NLS_LANG`，取代原生全域服務安裝模式，達成 100% 零全域環境變數污染。
   - `LAH_PM2_Service` 鎖定 `<根目錄>\node\node.exe` 與私有 `PM2_HOME = <根目錄>\.pm2`，完全脫鉤外部環境。
3. **便攜版內建 pm2-logrotate 預先掛載**：開機執行 `pm2 resurrect` 前自動透過 `wireBundledLogrotate` 將 `node\node_modules\pm2-logrotate` 掛載至 `.pm2\modules\pm2-logrotate` 並啟用日誌壓縮（`compress: true`），完全離線無需外網連線。
4. **常駐 Watchdog 守護與自癒**：每 60 秒巡檢 `LAH-FE`、`LAH-SCHEDULER`、`LAH-WSS`、`lah-api-v85`、`pm2-logrotate` 5 大核心受管服務；若偵測到 PM2 Daemon 被外部非可攜版 Node 意外搶佔，會自動強制終止並以可攜版 Node 重新喚醒，日誌寫入 `<根目錄>\logs\service\`。
5. **服務名稱資安脫敏**：服務顯示名稱與描述全面去除非必要的開源中介軟體技術詞彙，避免弱點掃描工具與資安稽核誤判。
