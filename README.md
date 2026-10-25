# 🚀 智慧控管與監控系統 - 自動更新工具 (System Auto Updater)

一個專為 Windows 環境（包含 Windows 10/11 與 Windows Server）設計的自動化解壓縮更新輔助工具。本工具基於 Python 標準庫 `tkinter` 開發，具備雙欄更新包管理、Apache 服務自動控制、PM2 核心守護進程校正、雙層同名目錄防呆消除、UAC 管理員權限宣告以及一鍵資安弱點防護功能。

> 📖 **版本修訂紀錄**：請參閱完整繁體中文 [版本修改歷程 (CHANGELOG.md)](./CHANGELOG.md)。

---

## 📌 系統需求與更新包規格

本工具主要用於快速佈署與更新伺服器核心模組，預設安裝根目錄為 `C:\AppServ`。程式啟動時會自動偵測「與執行檔同目錄」下的 10 種更新壓縮包，並依功能劃分為左右雙欄呈現。

### 1. 左欄：基礎環境與 Web 伺服器 (Apache / PHP)

| 壓縮包檔名（支援萬用字元與簡寫） | 預設相對路徑 | 預設完整目標路徑 | 說明 |
| :--- | :--- | :--- | :--- |
| **`httpd-2.4.*.zip` / `Apache24.zip`** | `Apache24` | `C:\AppServ\Apache24` | Apache 2.4 網頁伺服器核心 |
| **`php-7.4.*.zip` / `php7.zip`** | `php7` | `C:\AppServ\php7` | PHP 7.4 執行環境 (舊軌 API 依賴) |
| **`php-8.*.zip` / `php85.zip`** | `php85` | `C:\AppServ\php85` | PHP 8.5 執行環境 (現代化 FastCGI 核心) |

### 2. 右欄：智慧控管系統模組 (LAH 應用與套件)

| 壓縮包檔名 | 預設相對路徑 | 預設完整目標路徑 | 說明 |
| :--- | :--- | :--- | :--- |
| **`www.zip`** | `www` | `C:\AppServ\www` | 後端 PHP7 更新包 (舊軌 PHP 7) |
| **`LAH-API.zip`** | `LAH-API` | `C:\AppServ\LAH-API` | 後端 PHP8 更新包 (新軌 PHP 8.5 / v1) |
| **`LAH-NUXTJS.zip`** | `LAH-NUXTJS` | `C:\AppServ\LAH-NUXTJS` | 前端網站更新包 |
| **`LAH-NUXTJS-node_modules.zip` / `node_modules.zip`** | `LAH-NUXTJS\node_modules` | `C:\AppServ\LAH-NUXTJS\node_modules` | 前端網站核心依賴套件 |
| **`lah-messenger-server.zip`** | `lah-messenger-server` | `C:\AppServ\lah-messenger-server` | 即時通訊推播伺服器 |
| **`lah-messenger-server-node_modules.zip`** | `lah-messenger-server\node_modules` | `C:\AppServ\lah-messenger-server\node_modules` | 即時通訊伺服端核心套件 |
| **`node.zip`** | `node` | `C:\AppServ\node` | Node.js 綠色執行環境 (含 npm, pm2, yarn) |

> 💡 **彈性安裝路徑機制**：
> - **啟動自動檢查**：程式啟動時會主動檢測預設安裝目錄（`C:\AppServ`）是否存在。若不存在會主動跳出提示導引使用者立即瀏覽選取正確路徑。
> - **各模組獨立自訂**：預設各模組目標路徑與安裝根目錄自動連動。介面提供獨立的輸入框、瀏覽按鈕與重設按鈕，允許個別指定特定模組的解壓縮目錄。

---

## ✨ 核心特色與防呆機制

1. **零外部依賴執行庫 (Zero Dependency)**：
   - 介面與核心邏輯完全採用 Python 3 標準庫 (`tkinter`, `ttk`, `zipfile`, `subprocess`, `threading`, `ctypes`)。程式不依賴第三方 GUI 套件。
2. **左右雙欄卡片式佈局與獨立路徑自訂**：
   - 主視窗採用 `1260x760` 寬螢幕雙欄佈局。左欄專責 Web 基礎環境，右欄專責系統應用模組。
   - 每個模組卡片均具備獨立路徑輸入框、瀏覽按鈕與重設按鈕。兩側底部控制列各自獨立排列（左欄底部配置全域開始更新與輔助按鈕雙列，右欄底部配置 PM2 守護與校正雙列），垂直高度精準齊平對稱，徹底消除長路徑與控制按鈕之擠壓截斷。
3. **一鍵資安弱點防護（隱藏 Web 伺服器與 PHP 版本資訊）**：
   - 左欄提供「🛡️ 一鍵套用資安弱點防護」專屬按鈕。
   - 自動向 `Apache24/conf/httpd.conf` 寫入 `ServerTokens Prod` 與 `ServerSignature Off`，消除 HTTP Server 標頭與錯誤頁面版本洩漏。
   - 自動向 `php7/php.ini` 與 `php85/php.ini` 寫入 `expose_php = Off`，消除 `X-Powered-By` 標頭。
   - 寫入前自動建立帶時間戳記的備份檔，並主動調用 `httpd.exe -t` 進行語法校驗與服務重啟防護。
4. **自動去除雙層同名目錄防呆**：
   - 解壓縮前預先解析 ZIP 壓縮包內部結構。
   - 若壓縮包頂層含有同名單一資料夾（例如 `www.zip` 內含 `www/`，`LAH-API.zip` 內含 `LAH-API/`），程式會自動對齊並剔除外層目錄，徹底防止產生雙層巢狀目錄。
5. **Apache 與 PM2 服務自動重啟與核心校正**：
   - 更新前先停止相關服務釋放檔案鎖定，解壓縮完成後自動安全重啟。
   - 實時偵測 PM2 守護核心是否由本機綠色 Node.js 喚醒運行，提供「⚡ 立即校正 PM2 核心」與「⚙️ 開機自啟動設定」一鍵工具。
6. **獨立大尺寸日誌彈窗與版本說明瀏覽**：
   - 更新執行時自動彈出獨立進度視窗（`860x600`），內嵌彩色終端日誌與進度條。
   - 支援模糊比對自動讀取最新發布之版本說明文字檔。
7. **專屬自動移除工具支援 (`uninstaller.py`)**：
   - 專案內建專屬自動移除程式，支援服務解除安裝、開機腳本清除、環境變數還原、手動 PHP 目錄移除與刪除前關鍵資料備份。
8. **未偵測到安裝包預設隱藏與動態展開**：
   - 更新目錄下未偵測到實體壓縮包的項目預設自動隱藏其卡片，大幅簡化操作視圖。
   - 畫面底部左側提供「👁️ 顯示全部安裝包 (含未偵測到項目)」核取方塊，勾選時可隨時展開所有 10 種規格供檢視。
   - 當特定欄位未包含任何可用安裝包時，主動顯示指引橫幅消除使用者疑惑。
9. **從備份壓縮包還原設定與資料 (`_LAH_Backup_*.zip`)**：
   - 頂部提供「📦 安裝/更新後自動從備份包回復原始設定」勾選框與「⚡ 立即單獨還原備份」按鈕。
   - 自動偵測移除工具產出之 `_LAH_Backup_*.zip`，一鍵還原 `.env`、`AD.php`、即時通 `db/` 與 `uploads/`、`httpd.conf` 及 `php.ini`，並自動完成路徑校正與資安防護套用。
10. **全新安裝收尾全自動化與壓縮包保留詢問**：
    - 安裝完成後自動建立並啟動 `Apache2.4` 服務、套用 Apache 與 PHP 資安設定修改、確認 PM2 綁定本機綠色版 Node 並自動配置 `LAH_PM2_Service` 開機自啟動服務。
    - 安裝完成後主動詢問使用者是否保留或刪除安裝壓縮包（預設保留不自動刪除），若選擇刪除則一併清除同模組所有別名壓縮檔。

---

## 🛠️ 開發環境與執行方式

### 1. 環境需求
- Python 3.10 或 3.11 以上版本
- 作業系統：Windows 10 / 11 或 Windows Server

### 2. 原始碼直接執行
```powershell
# 切換至專案目錄
cd .\LAH-Updater

# 啟動自動更新工具
python updater.py

# 啟動自動移除工具
python uninstaller.py
```

---

## 📦 打包成單一獨立執行檔 (Build EXE)

專案使用 `PyInstaller` 打包為單一獨立無黑視窗的 Windows 執行檔 (`.exe`)。

### 步驟 1：安裝 PyInstaller
```powershell
pip install pyinstaller
```

### 步驟 2：執行打包指令
```powershell
# 一鍵打包自動更新工具
.\build.bat

# 或使用 Python 腳本打包
python build.py
```

打包自動移除工具指令：
```powershell
python -m PyInstaller --noconfirm --onefile --windowed --uac-admin --name "智慧控管與監控系統 - 自動移除工具" uninstaller.py
```

---

## 🗜️ 打包系統更新壓縮包 (`pack.py` / `pack.bat`)

專案內建 `pack.bat` 與 `pack.py` 輔助腳本：
1. 自動進入 `LAH-FE` 執行 `yarn build` 編譯前端。
2. 將 `LAH-API/www` 打包為 `www.zip`（自動排除 `.env` 與機敏設定檔）。
3. 將現代化 PHP 8.5 後端打包為 `LAH-API.zip`。
4. 將編譯後前端產物打包為 `LAH-NUXTJS.zip`。
5. 支援獨立打包即時通伺服端與綠色 Node 環境。

```powershell
# 一鍵執行完整打包流程
.\pack.bat

# 或使用 Python 執行特定模式
python pack.py --skip-build   # 略過編譯直接打包
python pack.py --api-only     # 僅打包後端 API
python pack.py --fe-only      # 僅打包前端網站
```

---

## 📂 專案檔案結構

```text
LAH-Updater/
 ├── updater.py                           # 自動更新工具主程式 (雙欄卡片、Apache/PHP/PM2 管理、資安防護)
 ├── uninstaller.py                       # 自動移除工具主程式 (服務解除安裝、自啟動清理、前置備份)
 ├── build.bat                            # 一鍵編譯自動更新工具執行檔批次檔
 ├── build.py                             # Python 原生打包輔助腳本
 ├── pack.bat                             # 一鍵編譯前端並打包全模組壓縮包批次檔
 ├── pack.py                              # Python 原始碼打包腳本
 ├── CHANGELOG.md                         # 完整繁體中文版本修改歷程文件
 ├── README.md                            # 專案說明與操作指南
 ├── AGENTS.md                            # AI 代理人架構指南與安全契約
 └── service/                             # WinSW 原生服務守護套件與腳本
```

---

## 📝 授權說明
Internal System Utility - 供內部系統維運與自動更新使用。
