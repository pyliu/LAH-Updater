# 智慧控管與監控系統 - 管理儀表板 (LAH-Dashboard)

本工具為 LAH 智慧控管系統之專屬單一執行檔原生監控與維運儀表板，基於 **Tauri 2 (Rust) + 純原生 HTML/CSS/JS** 架構實作，可直接放置於伺服器安裝根目錄（`C:\AppServ\` 或 `D:\AppServ\`）隨點即開。

---

## 1. 核心功能與視圖架構

### 1.1 服務監控與控制（分頁一）
- **雙軌分層控管**：
  - **Windows 系統服務層**：管控 `LAH_Web_Service`（Apache 2.4）與 `LAH_PM2_Service`（PM2 Daemon）。
  - **PM2 應用進程層**：管控 `LAH-FE`（前端網站 Port 8080）、`LAH-WSS`（即時通 Port 8081/8082）、`LAH-SCHEDULER`（排程）與 `lah-api-v85`（PHP 8.5 FastCGI Port 8085）。
- **個別操作與全域一鍵重啟**：支援各服務個別啟動、停止、重啟，並提供頂部安全防呆之「一鍵重啟全部服務」。

### 1.2 即時日誌檢視器（分頁二）
- **自動日誌來源探測**：涵蓋 FE、WSS、Scheduler、PHP85、Apache (error/access)、WinSW wrapper 與 Logrotate。
- **動態串流追蹤**：支援 Auto-Tail 即時追蹤、關鍵字即時篩選、清空畫布與「在記事本中開啟」實體檔案。

### 1.3 系統指標與快捷維運（分頁三）
- **主機資源即時狀態**：CPU 總使用率、實體記憶體負載、安裝磁碟剩餘空間與容量條。
- **五大連接埠健康探針**：即時 TCP 心跳檢查（Port 80, 8080, 8081, 8082, 8085）。
- **快捷工具箱**：
  - 一鍵開啟 `lah-cmd.bat` 沙盒維運終端
  - 瀏覽器開啟前端系統入口
  - Apache 組態語法驗證（`httpd -t`）
  - 手動觸發 PM2 日誌輪替壓縮（`pm2-logrotate`）
  - 開啟日誌存放目錄

---

## 2. 編譯與建置指令

### 一鍵編譯發布
```powershell
python build.py
# 或於 LAH-Updater 根目錄執行：
.\建立管理儀表板工具.bat
```

產出執行檔將自動發布至：
- `LAH-Updater/dist/智慧控管與監控系統 - 管理儀表板.exe`
- 若偵測到本機存在 `C:\AppServ`，將同步部署至 `C:\AppServ\智慧控管與監控系統 - 管理儀表板.exe`。
