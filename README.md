# 🚀 智慧控管與監控系統 - 自動更新工具 (System Auto Updater)

一個專為 Windows 環境（包含 Windows 10/11 與 Windows Server）設計的自動化解壓縮更新輔助工具，基於 Python 標準庫 `tkinter` 開發，具備 Apache 服務自動重啟控制、雙層同名目錄防呆消除、UAC 管理員權限宣告以及多執行緒即時日誌進度呈現。

---

## 📌 系統需求與更新包規格

本工具主要用於快速佈署與更新伺服器核心模組，預設安裝根目錄為 `C:\AppServ`。程式啟動時會自動偵測「與執行檔同目錄」下的 6 種更新壓縮包：

| 壓縮包檔名 | 預設相對路徑 | 預設完整目標路徑 | 說明 |
| :--- | :--- | :--- | :--- |
| **`www.zip`** | `www` | `C:\AppServ\www` | 後端 PHP7 更新包 (舊軌 PHP 7) |
| **`LAH-API.zip`** | `LAH-API` | `C:\AppServ\LAH-API` | 後端 PHP8 更新包 (新軌 PHP 8.5 / v1) |
| **`LAH-NUXTJS.zip`** | `LAH-NUXTJS` | `C:\AppServ\LAH-NUXTJS` | 前端網站 更新包 |
| **`node_modules.zip`** | `LAH-NUXTJS\node_modules` | `C:\AppServ\LAH-NUXTJS\node_modules` | 前端網站 核心套件 |
| **`lah-messenger-server.zip`** | `lah-messenger-server` | `C:\AppServ\lah-messenger-server` | 即時通訊推播伺服器 |
| **`node.zip`** | `node` | `C:\AppServ\node` | Node.js 綠色執行環境 (含 npm, pm2, yarn) |

> 💡 **彈性安裝路徑機制**：
> - **啟動自動檢查**：程式啟動時會主動檢測預設安裝目錄（`C:\AppServ`）是否存在，若不存在會主動跳出提示導引使用者立即瀏覽選取正確路徑。
> - **各模組獨立自訂**：預設各模組目標路徑與安裝根目錄自動連動，但同時提供獨立的「輸入框」、「瀏覽...」與「↺ 重設」按鈕，允許個別指定特定模組的解壓縮目錄。

---

## ✨ 核心特色與防呆機制

1. **零外部依賴執行庫 (Zero Dependency)**：
   - 介面與核心邏輯完全採用 Python 3 標準庫 (`tkinter`, `ttk`, `zipfile`, `subprocess`, `threading`, `ctypes`)，不依賴龐大的第三方 GUI 套件。
2. **預設目錄偵測與各模組獨立路徑**：
   - 啟動時自動偵測預設目錄，若不存在會彈出確認視窗引導使用者選取根目錄。
   - 介面採用清晰卡片式佈局，每個更新模組均提供獨立路徑輸入框、瀏覽按鈕與重設按鈕，既支援全域根目錄連動，又兼具個別模組自訂彈性。
3. **【關鍵防呆機制】自動去除雙層同名目錄**：
   - 解壓縮時會先解析 `.zip` 內部結構。
   - 若壓縮包頂層含有與目標資料夾同名、壓縮檔同名或目標目錄結尾同名的單一資料夾（例如 `www.zip` 內第一層就是 `www/`，`LAH-API.zip` 內為 `LAH-API/` 或 `API/`，或 `node_modules.zip` 內為 `node_modules/`），程式會**自動對齊並剔除外層目錄**，徹底避免產生 `C:\AppServ\www\www` 的雙層目錄錯誤！
   - 若頂層直接散落檔案，則直接覆蓋解壓至目標目錄。
4. **Apache 服務管理與異常容錯**：
   - 勾選「更新前後自動重啟 Apache 服務」後，自動偵測本機 Apache 服務（如 `Apache2.4`、`Apache`）。
   - 解壓縮覆蓋前先停止服務釋放檔案鎖定，更新完成後自動重新啟動。
   - **異常防護**：若解壓縮中途發生任何例外錯誤（如檔案鎖死），`finally` 區塊會**緊急強制嘗試重新啟動 Apache**，避免網站系統長時間中斷。
5. **版本更新說明彈窗瀏覽（省空間緊湊設計）**：
   - 支援彈性模糊比對：檔名只要包含「`更新說明`」的 `.txt` 檔案（例如 `★更新說明★.txt` 或 `1.9.5-★更新說明★.txt`）皆可自動讀取。
   - 主畫面移除冗長文字框，於「開始更新」旁設置「📋 查看更新說明」按鈕，點擊即彈出獨立置中對話框供完整閱讀（支援 Esc 快速鍵關閉）；若未提供說明檔時按鈕自動反灰停用（顯示「📋 無更新說明」），大幅降低主視窗高度。
   - 若同目錄下有多個符合的檔案（歷史殘留），**自動依檔案最後修改時間 (mtime) 優先載入最新發布的版本**，支援 UTF-8 / UTF-8-BOM / CP950 自動編碼容錯。
6. **PM2 管理套件偵測、守護核心 (Daemon) 綁定確認、智慧還原與一鍵校正**：
   - **實時核心進程探測**：主畫面獨立顯示 PM2 God Daemon 目前是由哪一個 `node.exe` 喚醒運行（`🟢 核心：C:\AppServ\node\node.exe (已綁定綠色版)` 或 `🟡 外部 Node: ...`）。
   - **更新時自動無痛切換與智慧還原**：解壓 `node.zip` 前若需關閉守護進程，會主動先執行 `pm2 save` 備份在線服務；更新完成後若進程先前已停止，直接調用 `pm2 resurrect` 自備份完整喚醒並還原服務；在呼叫 `pm2 restart all` 時若遇到 PM2 守護程序為空（`No process found`），內建自動 fallback 執行 `pm2 resurrect`，徹底消除誤報彈窗。
   - **自動註冊 Node 環境變數 (User PATH)**：更新 `node.zip` 或啟動校正時，自動透過 Python 標準庫 `winreg` 檢測並將 `<root_dir>\node` 寫入 Windows 目前使用者的 `PATH` 最前端，並透過 Win32 API 廣播 `WM_SETTINGCHANGE`，徹底解決一般終端機下執行 `pm2 ls`、`node`、`yarn` 出現「找不到指令」的問題。
   - **手動「⚡ 立即校正 PM2 核心」按鈕**：平時毋須執行完整系統更新，隨時點擊即可一鍵完成核心行程切換，並自動檢視伺服器開機自啟動（Windows 服務 / 工作排程器）配置狀態。
7. **更新完成空間清理確認**：
   - 更新全部順利完成後，會主動跳出詢問是否刪除本次已更新完成的 `.zip` 壓縮包以釋放磁碟空間（更新說明文字檔會完整保留）。
8. **文字放大 25% 舒適排版與獨立大尺寸日誌彈窗**：
   - 全介面標準字體與各區塊字級全數等比例放大 25%，視覺更加清晰好讀；主視窗隨之最佳化為 `980x620`，所有控制按鈕永遠置底清晰可見。
   - 點擊「🚀 開始更新」時自動彈出獨立大尺寸視窗（`860x600`），提供平滑進度條（Progressbar）、即時處理項目狀態與寬敞的深色終端風格彩色執行日誌（INFO、SUCCESS、WARNING、ERROR）。
   - 主畫面底部亦保留「📊 檢視執行日誌」按鈕，方便隨時開啟查看最新或歷史執行紀錄。
9. **UAC 系統管理員權限**：
   - 編譯時已內嵌 `--uac-admin` 清單，在 Windows 啟動時會主動要求系統管理員權限以確保有權控制 Windows 服務及修改受保護目錄。

---

## 🛠️ 開發環境與執行方式

### 1. 環境需求
- Python 3.10 或 3.11 以上版本
- 作業系統：Windows 10 / 11 或 Windows Server

### 2. 原始碼直接執行
```powershell
# 切換至專案目錄
cd .\LAH-Updater

# 直接啟動 GUI 程式
python updater.py
```

---

## 📦 打包成單一獨立執行檔 (Build EXE)

專案使用 `PyInstaller` 打包為單一獨立無黑視窗的 Windows 執行檔 (`.exe`)。

### 步驟 1：安裝 PyInstaller
```powershell
pip install pyinstaller
```

### 步驟 2：執行打包指令
您可直接在專案目錄執行一鍵打包批次檔（已內建 UTF-8 與 Python 原生防衝碼處理）：
```powershell
.\build.bat
```
或手動執行 Python 打包腳本：
```powershell
python build.py
```
或直接輸入完整 PyInstaller 指令：
```powershell
python -m PyInstaller --noconfirm --onefile --windowed --uac-admin --name "智慧控管與監控系統 - 自動更新工具" updater.py
```

**參數解析：**
- `--noconfirm`：若已存在先前的輸出目錄，直接覆蓋不反覆詢問。
- `--onefile`：將所有依賴庫打包為單一獨立執行檔 (`.exe`)。
- `--windowed` (或 `-w`)：隱藏後台終端機黑視窗，僅顯示 GUI 視窗。
- `--uac-admin`：在 exe 內建 Manifest 加入系統管理員權限要求 (`requireAdministrator`)，確保啟動時自動彈出 UAC 提權視窗。
- `--name "智慧控管與監控系統 - 自動更新工具"`：指定產出檔名。

### 步驟 3：產出確認
打包完成後，執行檔將位於 `dist/` 目錄中：
- `dist/智慧控管與監控系統 - 自動更新工具.exe`

---

## 🗜️ 打包系統更新壓縮包 (`www.zip` & `LAH-NUXTJS.zip`)

專案內建 `pack.bat` 與 `pack.py`，整合原 `zip_api.*` 與 `zip_fe.*` 打包規則：
1. 自動進入 `LAH-FE` 執行 `yarn build` 並等待編譯完成。
2. 將 `LAH-API/www` 打包為 `www.zip`（自動排除 `.env` 與 `include/config/AD*.php` 等機敏設定檔）。
3. 將 `LAH-FE` 編譯後檔案打包為 `LAH-NUXTJS.zip`。
4. 預設自動輸出至：`d:\DEV\@各所分享區\@智慧管控系統\@更新程式\`。

```powershell
# 一鍵執行完整流程（yarn build + 打包 www.zip 與 LAH-NUXTJS.zip）
.\pack.bat

# 或使用 Python 執行（支援選用參數）
python pack.py
python pack.py --skip-build   # 略過 yarn build 直接打包
python pack.py --api-only     # 僅打包 www.zip
python pack.py --fe-only      # 僅編譯並打包 LAH-NUXTJS.zip
```

---

## 🚀 發布與使用者操作說明

發布版本時，請將產出的執行檔與更新壓縮包放在**同一個資料夾**下打包提供給維運人員：

```text
📦 20261015-系統更新包/
 ├── 智慧控管與監控系統 - 自動更新工具.exe
 ├── ★更新說明★.txt                       (版本說明文件，非必填)
 ├── www.zip                              (本次更新內容，若有則放)
 ├── LAH-NUXTJS.zip                       (本次更新內容，若有則放)
 ├── node_modules.zip                     (視需求提供)
 └── lah-messenger-server.zip             (視需求提供)
```

### 操作步驟：
1. 雙擊執行 `智慧控管與監控系統 - 自動更新工具.exe`（系統將跳出 UAC 管理員權限確認，請點選「是」）。
2. 確認介面上偵測到的更新包狀態與目標路徑（若 Apache 安裝於非 C 槽，可按「瀏覽...」更改安裝根目錄，亦可為各模組個別自訂路徑）。
3. 勾選需要更新的項目（預設已偵測到的檔案會自動勾選）。
4. 點選 **「🚀 開始更新」**。
5. 程式將自動停止 Apache、執行解壓對齊覆蓋、重新啟動 Apache；若包含 Node.js 相關模組，會詢問是否執行 `pm2 restart all`，最後可選擇是否清除已更新的壓縮包。

---

## 📂 專案檔案結構

```text
LAH-Updater/
 ├── updater.py                           # 主程式原始碼 (GUI、解壓防呆、Apache 與 PM2 控制)
 ├── build.bat                            # 一鍵打包安裝程式批次檔 (純 ASCII 轉發，避開 CMD 編碼與 BOM 衝碼)
 ├── build.py                             # Python 原生 PyInstaller 打包輔助腳本
 ├── pack.bat                             # 一鍵編譯前端並打包原始碼壓縮包批次檔 (純 ASCII 轉發)
 ├── pack.py                              # Python 原始碼打包腳本 (執行 yarn build 並產出 www.zip / LAH-NUXTJS.zip)
 ├── AGENTS.md                            # AI 代理人維護與架構指南
 ├── README.md                            # 專案說明與操作指南
 ├── .gitignore                           # Git 忽略檔案設定
 ├── build/                               # 打包過程暫存目錄 (Git 忽略)
 └── dist/                                # 打包最終產出目錄 (Git 忽略)
      └── 智慧控管與監控系統 - 自動更新工具.exe
```

---

## 📝 授權說明
Internal System Utility - 供內部系統維運與自動更新使用。
