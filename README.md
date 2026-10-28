# 🚀 智慧控管與監控系統 - 維運部署工具套件 (LAH-Updater)

本專案為桃園市各地政事務所「智慧控管與監控系統」設計之專屬自動化維運工具套件，全數採用 **Tauri 2 (Rust) + 純靜態 HTML/JS 前端** 原生架構實作。具備隨點秒開（冷啟動 0.3 秒以內）、零外部運行庫依賴、免 Python 解壓目錄、大幅降低防毒軟體誤判率之特性，產出單一執行檔集中於 `dist/`。

> 📖 **版本修訂紀錄**：請參閱完整繁體中文 [版本修改歷程 (CHANGELOG.md)](./CHANGELOG.md)。

---

## 🗂️ 模組架構與職責

本專案採獨立模組化架構分工：

| 子目錄 | 工具名稱 | 核心技術 | 主要職責與特色 |
|---|---|---|---|
| **`updater/`** | **自動更新工具**<br/>`智慧控管與監控系統 - 自動更新工具.exe` | Tauri 2 (Rust) | 支援 10 大模組更新包智慧對齊解壓、`base/`、`patch/`、`node_modules/` 分流優先探測與平滑回退、5 階段自動化系統配置引擎（Apache mod_php7、PHP 8.5 FastCGI Proxy、CVE-2024-4577 資安強化、綠色 Node/PM2 設定與 WinSW 服務部署、服務重啟與全系統健康檢查）。 |
| **`uninstaller/`** | **自動移除工具**<br/>`智慧控管與監控系統 - 自動移除工具.exe` | Tauri 2 (Rust) | 完整停止並刪除 Windows 服務（`LAH_PM2_Service`、`Apache2.4`）、精確終止殘留進程、清理開機自啟動與工作排程、還原環境變數 `PATH`、註銷 Apache FastCGI 轉發、解除唯讀安全刪除檔案並深度清理安裝根目錄（嚴格保護備份檔）。 |
| **`backuper/`** | **備份與還原工具**<br/>`智慧控管與監控系統 - 備份與還原工具.exe` | Tauri 2 (Rust) | 支援系統重要資料與設定（即時通 `db/`、`uploads/`、各專案 `.env`、`AD.php`、`www/assets/`、`php.ini`、`httpd.conf`、`.pm2`）打包備份；擴充涵蓋 Oracle `instantclient_11_2` 與 PHP 自訂資料庫擴充 DLL；還原前自動建立 `restore_undo_*.zip` 復原點，採彈現式對話框節省空間。 |
| **`pack/`** | **原始碼打包發布工具**<br/>`pack.py` | Python 3.10+ 標準庫 | 供開發管理人員遍歷工作區內平級子專案（`LAH-API`、`LAH-FE`、`lah-messenger-server` 等）全自動產出 7 大標準更新壓縮包，支援動態工作區根目錄探測。 |

---

## 📌 更新包規格與分流搜尋機制

自動更新工具探測更新包時，優先至分類專屬子目錄尋找，若無則自動平滑回退至既有目錄：

### 1. 基礎服務環境 (優先搜尋 `base/` 資料夾)
| 壓縮包檔名（支援萬用字元與簡寫） | 預設相對路徑 | 說明 |
| :--- | :--- | :--- |
| **`httpd-2.4.*.zip` / `Apache24.zip`** | `Apache24` | Apache 2.4 網頁伺服器核心 |
| **`php-7.4.*.zip` / `php7.zip`** | `php7` | PHP 7.4 執行環境 (舊軌 API 依賴) |
| **`php-8.*.zip` / `php85.zip`** | `php85` | PHP 8.5 執行環境 (現代化 FastCGI 核心) |
| **`node.zip`** | `node` | Node.js 綠色執行環境 (含 npm, pm2, yarn) |

### 2. 業務程式更新包 (優先搜尋 `patch/` 資料夾)
| 壓縮包檔名 | 預設相對路徑 | 說明 |
| :--- | :--- | :--- |
| **`www.zip`** | `www` | 後端 PHP7 更新包 (舊軌 PHP 7) |
| **`LAH-API.zip`** | `LAH-API` | 後端 PHP8 更新包 (新軌 PHP 8.5 / v1) |
| **`LAH-NUXTJS.zip`** | `LAH-NUXTJS` | 前端網站更新包 |
| **`lah-messenger-server.zip`** | `lah-messenger-server` | 即時通訊推播伺服器 |
| **`*更新說明*.txt`** | - | 最新版本更新說明文件（優先載入最新修改檔） |

### 3. 依賴套件包 (優先搜尋 `node_modules/` 資料夾)
| 壓縮包檔名 | 預設相對路徑 | 說明 |
| :--- | :--- | :--- |
| **`LAH-NUXTJS-node_modules.zip` / `node_modules.zip`** | `LAH-NUXTJS\node_modules` | 前端網站核心依賴套件 |
| **`lah-messenger-server-node_modules.zip`** | `lah-messenger-server\node_modules` | 即時通訊伺服端核心套件 |

---

## 🛠️ 開發與建置發布 (Build)

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

# 4. 原始碼全系統打包（產出更新壓縮包至 dist/）
.\打包智慧控管及監控系統.bat
# 或執行：python pack\pack.py
```

### 產出執行檔集中於 `dist/`：
- `dist/智慧控管與監控系統 - 自動更新工具.exe`（翡翠綠升級齒輪圖示，約 11.7 MB）
- `dist/智慧控管與監控系統 - 自動移除工具.exe`（警戒深紅解構叉號圖示，約 10.7 MB）
- `dist/智慧控管與監控系統 - 備份與還原工具.exe`（科技深藍資料庫循環圖示，約 9.5 MB）
