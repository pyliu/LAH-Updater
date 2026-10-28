# LAH PM2 Windows 原生系統服務套件 (WinSW)

本目錄包含將 PM2 封裝為 Windows 原生系統服務（Windows Service）的配置與腳本：

- `lah-service.exe`：基於 WinSW v2.12.0 (.NET 4.6.1) 的服務包裝執行檔（由 `pack.py` 打包至 `node.zip` 內之 `node\service\`）。
- `lah-service.xml`：WinSW 服務設定檔，指定服務名稱 `LAH_PM2_Service` 與執行參數。
- `service_runner.js`：由綠色 Node (`<根目錄>\node\node.exe`) 執行的服務入口與常駐 Watchdog 守護腳本。

## 核心優勢與安全防護機制

1. **開機免登入（Boot-time Autostart）**：在伺服器開機（無人員登入桌面）即由 Windows 服務控制管理員 (SCM) 自動拉起，徹底解決機房重啟後無人登入導致服務中斷的問題。
2. **路徑絕對硬鎖定**：設定檔直接鎖定 `<根目錄>\node\node.exe`，並將 `<根目錄>\node` 置於程序 `PATH` 最前端，避免系統環境變數或外部 Node.js（如全域 v22/v24）干擾。
3. **便攜版內建 pm2-logrotate 預先掛載**：開機執行 `pm2 resurrect` 前自動透過 `wireBundledLogrotate` 將 `node\node_modules\pm2-logrotate` 掛載至 `.pm2\modules\pm2-logrotate` 並啟用日誌壓縮（`compress: true`），完全離線無需外網連線。
4. **常駐 Watchdog 守護與自癒**：每 60 秒巡檢 `LAH-FE`、`LAH-SCHEDULER`、`LAH-WSS`、`lah-api-v85`、`pm2-logrotate` 5 大核心服務；若偵測到 PM2 Daemon 被外部非可攜版 Node 意外搶佔，會自動強制終止並以可攜版 Node 重新喚醒，日誌寫入 `<根目錄>\logs\service\`。
