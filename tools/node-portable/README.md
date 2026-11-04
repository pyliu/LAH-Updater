# LAH-NodePortable（智慧控管與監控系統 - Node 沙盒監管工具）

因應資安要求，提供綠色版 Node.js 16.20.2 與 PM2 的離線安裝與沙盒專屬維運控制台，基於 Tauri 2 (Rust) 開發。採用**零全域污染沙盒隔離模式**，不修改系統或使用者全域環境變數，可供其他 Node.js 程式共用。

## 雙模式二合一架構（安裝部署 + 沙盒專屬控制台）

1. **📦 沙盒安裝與部署分頁**：
   - 解壓 `node.zip` 至 `<根目錄>\node`（自動剝除頂層 `node/` 前綴）。
   - 修補 PM2：統一 `<根目錄>\.pm2`、IPC 改為 Loopback TCP（127.0.0.1:42681/42682），避免 `connect EPERM`。
   - 直接掛載便攜版內建之 `pm2-logrotate`（v3.0.0）並啟用 `compress: true`。
   - **自動駐留控制台副本**：安裝完成時自動於 `<根目錄>` 複製留存一份 `智慧控管與監控系統 - Node 沙盒監管工具.exe` 與 `lah-cmd.bat`，方便日後直接開啟維運。
2. **🖥️ 沙盒 PM2 專屬控制台分頁**：
   - **智慧預設分頁**：啟動工具時若偵測到 `<根目錄>\node\node.exe` 與 `pm2` 已經安裝，自動直接進入「沙盒 PM2 專屬控制台」分頁；安裝完成後亦可一鍵切換進入。
   - **沙盒核心與服務監控**：即時顯示沙盒 Node / PM2 版本與 `PM2_Portable_Service` Windows 服務狀態，支援一鍵啟動、停止、重啟服務與喚醒 `lah-cmd.bat` 專屬沙盒終端。
   - **內建 `pm2-logrotate` 專區**：獨立監控日誌輪替模組狀態、PID、記憶體與 Gzip 壓縮設定，支援一鍵手動觸發 Gzip 日誌壓縮輪替（含 `pm2 reloadLogs`）及模組修復掛載。
   - **受管業務系統清單**：即時監控所有受該沙盒 PM2 控管之業務系統（ID、名稱、狀態、PID、CPU、記憶體、運行時間、重啟次數），支援單一或全部進程之啟動、停止、重啟、儲存快照（`pm2 save`）與還原快照（`pm2 resurrect`）。
   - **即時日誌預覽（Log Tail 彈窗）**：支援預覽各受管系統之 `out.log` / `err.log` 與 `.pm2\logs` 日誌尾端（自動相容 UTF-8 與 Big5/CP950 編碼），並提供一鍵以記事本開啟或開啟日誌資料夾。

## 使用方式

1. 執行根目錄 `建立可攜NODE工具.bat`（或 `python tools/node-portable/build.py`），產出 `dist\智慧控管與監控系統 - Node 沙盒監管工具.exe`。
2. 將 `node.zip`（內含 `node/node.exe`、`node/node_modules/pm2`，與 updater 使用之同一包）放在工具旁的 `base\` 資料夾、工具同層或 `C:\Temp`。
3. 以系統管理員身分執行工具：
   - **首次安裝**：選擇安裝根目錄（預設 `C:\AppServ`）後按「🚀 開始安裝」，完成後點擊「🖥️ 進入沙盒控制台」。
   - **日常維運**：直接執行根目錄下駐留之執行檔或原工具，即自動進入「🖥️ 沙盒 PM2 專屬控制台」。

工具不連外網，全程離線運作。

## 系統全域 Node.js 因資安要求升級時的影響說明

當伺服器因資安稽核要求，日後由 IT 或資安人員以官方 MSI 安裝包升級全域 Node.js（例如將 `C:\Program Files\nodejs` 升級至 v22 或 v24）時，**完全不會影響 PM2 與本工具受管程式的正常執行**。

本工具透過四重隔離機制保證受管程式之執行期穩定性：

1. **Windows 服務層級硬鎖定（`PM2_Portable_Service`）**
   服務設定檔 `pm2-service.xml` 直接將執行檔路徑指定為 `<根目錄>\node\node.exe`，並將 `<根目錄>\node` 注入服務程序的私有 `PATH` 最前端。
2. **命令列執行檔硬鎖定（`pm2.cmd`）**
   可攜版 `node\pm2.cmd` 腳本透過 `%~dp0` 強制設定 `SET "_prog=%dp0%\node.exe"` 與 `SET "PATH=%dp0%;%PATH%"`。
3. **PM2 守護進程與子程序解譯器繼承（`process.execPath`）**
   PM2 Daemon 與業務子程序皆直接繼承可攜版 Node.js 16.20.2，絕不呼叫系統全域 `node.exe`。
4. **沙盒專屬控制台與 60 秒常駐守護自癒**
   維運統一使用內建「沙盒 PM2 專屬控制台」或 `lah-cmd.bat` 終端。背景服務 `pm2_service_runner.js` 每 60 秒檢查一次 PM2 Daemon 的實體執行路徑，若發現被外部非可攜版 Node 意外搶佔，會自動強制終止並以可攜版 Node 重新喚醒。

## 目錄結構

```text
node-portable/
├── src-tauri/   # Rust 核心 (install.rs 解壓與駐留、console.rs 沙盒控制台與日誌、pm2.rs 修補與服務、audit.rs 資安自檢、system.rs 沙盒隔離)
├── ui/          # 純靜態 HTML/JS (雙分頁：沙盒安裝部署 + 沙盒 PM2 專屬控制台 + 即時日誌預覽彈窗)
├── service/     # WinSW 執行檔與服務入口腳本 (編譯時 include_bytes! 內嵌)
├── build.py     # 編譯並發布至 dist/
└── build.bat
```
