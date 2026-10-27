# LAH PM2 Windows 原生系統服務套件 (WinSW)

本目錄包含將 PM2 封裝為 Windows 原生系統服務（Windows Service）的配置與腳本：

- `lah-service.exe`：基於 WinSW v2.12.0 (.NET 4.6.1) 的服務包裝執行檔（由 `pack.py` 打包至 `node.zip` 內之 `node\service\`）。
- `lah-service.xml`：WinSW 服務設定檔，指定服務名稱 `LAH_PM2_Service` 與執行參數。
- `service_runner.js`：由綠色 Node (`C:\AppServ\node\node.exe`) 執行的服務入口與常駐 Watchdog 守護腳本。

## 核心優勢
1. **開機免登入（Boot-time Autostart）**：在伺服器開機（無人員登入桌面）即由 SCM 自動拉起，徹底解決機房重啟後無人登入導致服務中斷的問題。
2. **路徑絕對鎖定**：完全固定使用 `C:\AppServ\node\node.exe`，避免系統環境變數或 Volta/NVM 等墊片程式干擾。
3. **常駐 Watchdog 守護**：每 60 秒巡檢 `LAH-FE`、`LAH-SCHEDULER`、`LAH-WSS`、`lah-api-v85`、`pm2-logrotate`，發現離線自動重啟並寫入日誌 `C:\AppServ\logs\service\`。
