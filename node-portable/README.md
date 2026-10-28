# LAH-NodePortable（可攜 Node.js + PM2 安裝工具）

因應資安要求，提供綠色版 Node.js 16.20.2 與 PM2 的離線安裝工具，基於 Tauri 2 (Rust) 開發。相關核心程式碼（PM2 綠色環境修補、PATH 註冊）抽取自 `updater/`，不含任何 LAH 專屬服務巡檢，可供其他 Node.js 程式共用。

## 使用方式

1. 執行根目錄 `建立可攜NODE工具.bat`，產出 `dist\智慧控管與監控系統 - 可攜 Node 安裝工具.exe`。
2. 將 `node.zip`（內含 `node/node.exe`、`node/node_modules/pm2`，與 updater 使用之同一包）放在工具旁的 `base\` 資料夾、工具同層或 `C:\Temp`。
3. 以系統管理員身分執行工具，選擇安裝根目錄（預設 `C:\AppServ`）後按「開始安裝」。

工具不連外網，全程離線。

## 安裝內容

- 解壓 `node.zip` 至 `<根目錄>\node`（自動剝除頂層 `node/` 前綴）。
- 修補 PM2：統一 `<根目錄>\.pm2`、IPC 改為 Loopback TCP（127.0.0.1:42681/42682），避免 `connect EPERM`。
- 可選：寫入使用者 PATH 與 `PM2_HOME`。
- 可選：註冊 `PM2_Portable_Service`（WinSW）開機自啟，開機與每 60 秒偵測 PM2 Daemon，離線即 `pm2 resurrect` 還原 `dump.pm2`。與 updater 的 `LAH_PM2_Service` 互不衝突。
- 安裝後以 `node -v` 與 `pm2 -v` 驗證，失敗即回報錯誤。

重複執行會先停止服務、`pm2 kill` 並清理 `<根目錄>\node` 下的行程，再覆蓋安裝。`<根目錄>\.pm2` 不會被刪除。

## 目錄結構

```text
node-portable/
├── src-tauri/   # Rust 核心 (install.rs 解壓與流程、pm2.rs 修補與服務、system.rs PATH/環境變數)
├── ui/          # 純靜態 HTML/JS
├── service/     # WinSW 執行檔與服務入口腳本 (編譯時 include_bytes! 內嵌)
├── build.py     # 編譯並發布至 dist/
└── build.bat
```
