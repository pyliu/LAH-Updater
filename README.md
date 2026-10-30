# 🚀 智慧控管與監控系統 - 維運部署工具套件 (LAH-Updater)

本專案為桃園市各地政事務所「智慧控管與監控系統」設計之專屬自動化維運工具套件，全數採用 **Tauri 2 (Rust) + 純靜態 HTML/JS 前端** 原生架構實作。具備隨點秒開（冷啟動 0.3 秒以內）、零外部運行庫依賴、免 Python 解壓暫存目錄、介面動態自適應無捲軸、大幅降低防毒軟體誤判率之特性，產出單一執行檔集中於 `dist/`。

> 📖 **版本修訂紀錄與離線手冊**：
> - 完整繁體中文版本歷程：[CHANGELOG.md](./CHANGELOG.md)
> - 視覺化架構技術手冊（純離線 HTML）：[README.html](./README.html)
> - 伺服器安裝與版本升級標準操作手冊（純離線 HTML）：[INSTALLATION.html](./INSTALLATION.html)

---

## 🗂️ 模組架構與職責

本專案採獨立模組化架構分工：

| 子目錄 | 工具名稱 | 核心技術 | 主要職責與特色 |
|---|---|---|---|
| **`updater/`** | **自動更新工具**<br/>`智慧控管與監控系統 - 自動更新工具.exe` | Tauri 2 (Rust) | 支援 11 大模組更新包智慧對齊解壓、`base/`、`patch/`、`node_modules/` 分流優先探測與平滑回退、5 階段自動化系統配置引擎（Web 沙盒服務 `LAH_Web_Service` 守護與 Apache mod_php7、VC++ 依賴自動補齊、PHP 8.5 FastCGI Proxy、CVE-2024-4577 資安強化、綠色 Node/PM2 零全域污染沙盒隔離与 `LAH_PM2_Service` 守護、內建 `pm2-logrotate` 離線掛載、服務名稱資安脫敏、服務重啟與五大連接埠健康巡檢）。 |
| **`uninstaller/`** | **自動移除工具**<br/>`智慧控管與監控系統 - 自動移除工具.exe` | Tauri 2 (Rust) | 完整停止並刪除 Windows 系統服務（`LAH_Web_Service`、`LAH_PM2_Service` 與向下相容 `Apache2.4`）、精確終止殘留進程（含 `php-cgi.exe`）、清理開機自啟動與工作排程、清理舊有登錄檔殘留 `PATH`、註銷 Apache FastCGI 轉發、內置擬真二次確認對話框、解除唯讀安全刪除檔案並深度清理安裝根目錄（嚴格保護備份檔）。 |
| **`backuper/`** | **備份與還原工具**<br/>`智慧控管與監控系統 - 備份與還原工具.exe` | Tauri 2 (Rust) | 支援系統重要資料與設定（即時通 `db/`、`uploads/`、各專案 `.env`、`AD.php`、`www/assets/`、`php.ini`、`httpd.conf`、`.pm2`、`lah-cmd.bat`）打包備份；擴充涵蓋 Oracle `instantclient_11_2` 與 PHP 自訂資料庫擴充 DLL（OCI8 / SQLSRV）；還原前自動建立 `restore_undo_*.zip` 復原點，支援動態視窗高度自適應與彈現式進度對話框。 |
| **`node-portable/`** | **可攜 Node 安裝工具**<br/>`智慧控管與監控系統 - 可攜 Node 安裝工具.exe` | Tauri 2 (Rust) | 因應資安要求提供純綠色版 Node.js 16.20.2 與 PM2 離線安裝，採**零全域污染沙盒隔離模式**（不修改系統或使用者 `PATH`，主動清理登錄檔舊殘留並產出專屬維運終端 `lah-cmd.bat`），直接掛載內建 `pm2-logrotate`（v3.0.0）、清理 `dump.pm2` 舊路徑、註冊開機背景自啟服務 `PM2_Portable_Service`（WinSW）並具備每 60 秒 Daemon 自癒機制。 |
| **`tools/sms-env-editor/`** | **地籍異動簡訊參數設定工具**<br/>`地籍異動簡訊參數設定工具.exe` | Tauri 2 (Rust) | 內政部地籍異動簡訊與郵件發送服務（`SMSWEB_MOI\env.properties`）專用可視化設定工具。預設載入 `\\220.1.34.221\C$\SMSWEB_MOI\env.properties`，支援平臺網址、簡訊/郵件/Oracle 帳密 BASE64 自動加解密、Big-5 / CP950 編碼無損相容、儲存前 Diff 差異預覽與自動時間戳記備份。 |
| **`pack/`** | **原始碼打包發布工具**<br/>`pack.py` | Python 3.10+ 標準庫 | 供開發管理人員遍歷工作區內平級子專案（`LAH-API`、`LAH-FE`、`lah-messenger-server` 等）全自動產出標準更新壓縮包（含自動收集系統 VC++ 執行期 DLL 封裝為 `vcredist-x64.zip`），依三向分流結構輸出至 `dist/patch/`、`dist/node_modules/`、`dist/base/`。 |

---

## 📌 更新包規格與分流搜尋機制

自動更新工具探測更新包時，優先至分類專屬子目錄尋找，若無則自動平滑回退至既有目錄：

### 1. 基礎服務環境 (優先搜尋 `base/` 資料夾)
| 壓縮包檔名（支援萬用字元與簡寫） | 預設相對路徑 | 說明 |
| :--- | :--- | :--- |
| **`httpd-2.4.*.zip` / `Apache24.zip`** | `Apache24` | Apache 2.4 網頁伺服器核心（由 `LAH_Web_Service` 沙盒守護） |
| **`vcredist-x64.zip` / `vcredist*.zip`** | `Apache24\bin` | VC++ 2015–2022 獨立執行期依賴（自動補齊至 `Apache24\bin`、`php7` 與 `php85`，達成零全域依賴） |
| **`php-7.4.*.zip` / `php7.zip`** | `php7` | PHP 7.4 執行環境 (舊軌 API 依賴) |
| **`php-8.*.zip` / `php85.zip`** | `php85` | PHP 8.5 執行環境 (現代化 FastCGI 核心，監聽 Port 8085) |
| **`node.zip`** | `node` | Node.js 16.20.2 綠色執行環境 (含 npm, pm2, pm2-logrotate, WinSW 服務核心) |

### 2. 業務程式更新包 (優先搜尋 `patch/` 資料夾)
| 壓縮包檔名 | 預設相對路徑 | 說明 |
| :--- | :--- | :--- |
| **`www.zip`** | `www` | 後端 PHP7 更新包 (舊軌 PHP 7) |
| **`LAH-API.zip`** | `LAH-API` | 後端 PHP8 更新包 (新軌 PHP 8.5 / v1) |
| **`LAH-NUXTJS.zip`** | `LAH-NUXTJS` | 前端網站更新包 (Port 8080) |
| **`lah-messenger-server.zip`** | `lah-messenger-server` | 即時通訊推播與附件伺服器 (Port 8081 / 8082) |
| **`*更新說明*.txt`** | - | 最新版本更新說明文件（優先載入最新修改檔） |

### 3. 依賴套件包 (優先搜尋 `node_modules/` 資料夾)
| 壓縮包檔名 | 預設相對路徑 | 說明 |
| :--- | :--- | :--- |
| **`LAH-NUXTJS-node_modules.zip` / `node_modules.zip`** | `LAH-NUXTJS\node_modules` | 前端網站核心依賴套件 |
| **`lah-messenger-server-node_modules.zip`** | `lah-messenger-server\node_modules` | 即時通訊伺服端核心套件 |

---

## 🛠️ 開發、建置發布與維運腳本

所有工具均提供專屬批次檔與建置腳本，自動編譯為 Release 二進位執行檔並集中發布至 `dist/` 目錄：

```powershell
# 1. 建立自動更新工具
.\建立自動更新工具.bat
# 或執行：python updater\build.py

# 2. 建立自動移除工具
.\建立解除安裝工具.bat
# 或執行：python uninstaller\build.py

# 3. 建立自動備份工具
.\建立自動備份工具.bat
# 或執行：python backuper\build.py

# 4. 建立可攜 Node 安裝工具
.\建立可攜NODE工具.bat
# 或執行：python node-portable\build.py

# 5. 建立地籍異動簡訊參數設定工具
.\建立簡訊設定工具.bat
# 或執行：python tools\sms-env-editor\build.py

# 6. 原始碼全系統打包（產出更新壓縮包至 dist/base、dist/patch、dist/node_modules）
.\打包智慧控管及監控系統.bat
# 或執行：python pack\pack.py

# 7. 現有主機 Windows 服務名稱與描述資安脫敏熱校準（免重裝服務直接更新 services.msc 顯示名稱）
.\校準現有服務資安脫敏名稱.bat
```

### 產出執行檔集中於 `dist/`：
- `dist/智慧控管與監控系統 - 自動更新工具.exe`（翡翠綠升級齒輪圖示，約 11.86 MB）
- `dist/智慧控管與監控系統 - 自動移除工具.exe`（警戒深紅解構叉號圖示，約 10.71 MB）
- `dist/智慧控管與監控系統 - 備份與還原工具.exe`（科技深藍資料庫循環圖示，約 11.09 MB）
- `dist/智慧控管與監控系統 - 可攜 Node 安裝工具.exe`（亮綠六角形加 PM2 循環箭頭圖示，約 11.50 MB）
- `dist/地籍異動簡訊參數設定工具.exe`（專屬簡訊信封與加密金鑰齒輪圖示，約 10.50 MB）
