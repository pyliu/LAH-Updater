# LAH-NodePortable（可攜 Node.js + PM2 安裝工具）

因應資安要求，提供綠色版 Node.js 16.20.2 與 PM2 的離線安裝工具，基於 Tauri 2 (Rust) 開發。採用**零全域污染沙盒隔離模式**，不修改系統或使用者全域環境變數，可供其他 Node.js 程式共用。

## 使用方式

1. 執行根目錄 `建立可攜NODE工具.bat`，產出 `dist\智慧控管與監控系統 - 可攜 Node 安裝工具.exe`。
2. 將 `node.zip`（內含 `node/node.exe`、`node/node_modules/pm2`，與 updater 使用之同一包）放在工具旁的 `base\` 資料夾、工具同層或 `C:\Temp`。
3. 以系統管理員身分執行工具，選擇安裝根目錄（預設 `C:\AppServ`）後按「開始安裝」。

工具不連外網，全程離線。

## 安裝內容與沙盒架構

- 解壓 `node.zip` 至 `<根目錄>\node`（自動剝除頂層 `node/` 前綴）。
- 修補 PM2：統一 `<根目錄>\.pm2`、IPC 改為 Loopback TCP（127.0.0.1:42681/42682），避免 `connect EPERM`。
- 直接掛載便攜版內建之 `pm2-logrotate`（v3.0.0）：自 `<根目錄>\node\node_modules\pm2-logrotate` 預先掛載至 `<根目錄>\.pm2\modules\pm2-logrotate`，並在 `.pm2\module_conf.json` 寫入預設設定（啟用日誌壓縮 `compress: true`），完全不連外網、零外部依賴離線啟動。
- **零全域污染沙盒隔離**：
  - 絕不將 `<根目錄>\node` 寫入系統（`HKLM`）或使用者（`HKCU`）`PATH`，避免全域 `node -v` 被覆蓋。
  - 主動清理舊版 `pm2-installer` 或舊安裝器於登錄檔遺留之 `PATH` 與 `PM2_INSTALL_DIRECTORY`、`PM2_SERVICE_DIRECTORY`。
  - 自動於安裝根目錄產出專屬沙盒維運終端 `lah-cmd.bat`，注入私有環境變數，關閉視窗即自動釋放。
- 自動清理 `dump.pm2`：將舊全域殘留之解譯器路徑重設為可攜版 `node`，並修正 PID 路徑至統一 `.pm2\pids`。
- 可選：註冊 `PM2_Portable_Service`（WinSW）開機自啟，開機與每 60 秒偵測 PM2 Daemon，離線即 `pm2 resurrect` 還原 `dump.pm2`。
- 安裝後以沙盒環境執行驗證，回報健康狀態。
- 介面採用無捲軸版面設計，高度精準收斂至內容邊界。

重複執行會先停止服務、`pm2 kill` 並清理 `<根目錄>\node` 下的行程，再覆蓋安裝。`<根目錄>\.pm2` 不會被刪除。

## 系統全域 Node.js 因資安要求升級時的影響說明

當伺服器因資安稽核要求，日後由 IT 或資安人員以官方 MSI 安裝包升級全域 Node.js（例如將 `C:\Program Files\nodejs` 升級至 v22 或 v24）時，**完全不會影響 PM2 與本工具受管程式的正常執行**。

本工具透過四重隔離機制保證受管程式之執行期穩定性：

1. **Windows 服務層級硬鎖定（`PM2_Portable_Service`）**
   服務設定檔 `pm2-service.xml` 直接將執行檔路徑指定為 `<根目錄>\node\node.exe`，並將 `<根目錄>\node` 注入服務程序的私有 `PATH` 最前端。無論作業系統全域環境如何變更，背景服務啟動必定使用可攜版 Node.js 16.20.2。
2. **命令列執行檔硬鎖定（`pm2.cmd`）**
   可攜版 `node\pm2.cmd` 腳本透過 `%~dp0` 強制設定 `SET "_prog=%dp0%\node.exe"` 與 `SET "PATH=%dp0%;%PATH%"`。手動於命令提示字元輸入 `pm2` 時，必定透過可攜版 Node.js 啟動 PM2 CLI。
3. **PM2 守護進程與子程序解譯器繼承（`process.execPath`）**
   - PM2 Daemon 在 `Client.js` 中透過 `process.execPath` 啟動，鎖定為可攜版 `node.exe`。
   - PM2 孵化業務子程序時（`ForkMode.js` 與 `ClusterMode.js`），預設解譯器 `"node"` 會自動解析為當前 Daemon 自身的 `process.execPath`，直接繼承可攜版 Node.js 16.20.2，絕不呼叫系統全域的 `C:\Program Files\nodejs\node.exe`。
4. **沙盒專屬終端與 60 秒常駐守護自癒**
   維運統一使用專屬 `lah-cmd.bat` 終端。背景服務 `pm2_service_runner.js` 每 60 秒檢查一次 PM2 Daemon 的實體執行路徑，若發現被外部非可攜版 Node 意外搶佔，會自動強制終止並以可攜版 Node 重新喚醒。

## 目錄結構

```text
node-portable/
├── src-tauri/   # Rust 核心 (install.rs 解壓與流程、pm2.rs 修補與服務、system.rs 登錄檔清理/沙盒終端)
├── ui/          # 純靜態 HTML/JS (無捲軸緊緻版面)
├── service/     # WinSW 執行檔與服務入口腳本 (編譯時 include_bytes! 內嵌)
├── build.py     # 編譯並發布至 dist/
└── build.bat
```
