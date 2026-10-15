# 🚀 智慧控管與監控系統 - 自動更新工具 (System Auto Updater)

一個專為 Windows 環境（包含 Windows 10/11 與 Windows Server）設計的自動化解壓縮更新輔助工具，基於 Python 標準庫 `tkinter` 開發，具備 Apache 服務自動重啟控制、雙層同名目錄防呆消除、UAC 管理員權限宣告以及多執行緒即時日誌進度呈現。

---

## 📌 系統需求與更新包規格

本工具主要用於快速佈署與更新伺服器核心模組，預設安裝根目錄為 `C:\AppServ`。程式啟動時會自動偵測「與執行檔同目錄」下的 4 種更新壓縮包：

| 壓縮包檔名 | 預設相對路徑 | 預設完整目標路徑 | 說明 |
| :--- | :--- | :--- | :--- |
| **`www.zip`** | `www` | `C:\AppServ\www` | 前端與網站核心目錄 |
| **`LAH-NUXTJS.zip`** | `LAH-NUXTJS` | `C:\AppServ\LAH-NUXTJS` | Nuxt.js 應用服務系統 |
| **`node_modules.zip`** | `LAH-NUXTJS\node_modules` | `C:\AppServ\LAH-NUXTJS\node_modules` | Node.js 依賴套件包 |
| **`lah-messenger-server.zip`** | `lah-messenger-server` | `C:\AppServ\lah-messenger-server` | 即時通訊推播伺服器 |

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
   - 若壓縮包頂層含有與目標資料夾同名、壓縮檔同名或目標目錄結尾同名的單一資料夾（例如 `www.zip` 內第一層就是 `www/`，或 `node_modules.zip` 內為 `node_modules/`），程式會**自動對齊並剔除外層目錄**，徹底避免產生 `C:\AppServ\www\www` 的雙層目錄錯誤！
   - 若頂層直接散落檔案，則直接覆蓋解壓至目標目錄。
4. **Apache 服務管理與異常容錯**：
   - 勾選「更新前後自動重啟 Apache 服務」後，自動偵測本機 Apache 服務（如 `Apache2.4`、`Apache`）。
   - 解壓縮覆蓋前先停止服務釋放檔案鎖定，更新完成後自動重新啟動。
   - **異常防護**：若解壓縮中途發生任何例外錯誤（如檔案鎖死），`finally` 區塊會**緊急強制嘗試重新啟動 Apache**，避免網站系統長時間中斷。
5. **版本說明檔智慧載入**：
   - 支援彈性模糊比對：檔名只要包含「`更新說明`」的 `.txt` 檔案（例如 `★更新說明★.txt` 或 `1.9.5-★更新說明★.txt`）皆可自動讀取。
   - 若同目錄下有多個符合的檔案（歷史殘留），**自動依檔案最後修改時間 (mtime) 優先載入最新發布的版本**。
   - 支援 UTF-8 / UTF-8-BOM / CP950 自動編碼容錯。
6. **更新完成空間清理確認**：
   - 更新全部順利完成後，會主動跳出詢問是否刪除本次已更新完成的 `.zip` 壓縮包以釋放磁碟空間（更新說明文字檔會完整保留）。
7. **多執行緒 (Threading) 與終端風格日誌**：
   - 解壓縮與服務操作均在獨立背景執行緒進行，介面流暢不卡頓。
   - 包含平滑進度條（Progressbar）、當前解壓檔名即時狀態與彩色即時日誌（INFO、SUCCESS、WARNING、ERROR）。
8. **UAC 系統管理員權限**：
   - 編譯時已內嵌 `--uac-admin` 清單，在 Windows 啟動時會主動要求系統管理員權限以確保有權控制 Windows 服務及修改受保護目錄。

---

## 🛠️ 開發環境與執行方式

### 1. 環境需求
- Python 3.10 或 3.11 以上版本
- 作業系統：Windows 10 / 11 或 Windows Server

### 2. 原始碼直接執行
```powershell
# 切換至專案目錄
cd d:\DEV\lah-updater

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
```powershell
pyinstaller --noconfirm --onefile --windowed --uac-admin --name "智慧控管與監控系統 - 自動更新工具" updater.py
```

**參數解析：**
- `--noconfirm`：若已存在先前的輸出目錄，直接覆蓋不反覆詢問。
- `--onefile`：將所有依賴庫打包為單一獨立執行檔 (`.exe`)。
- `--windowed` (或 `-w`)：隱藏後台終端機黑視窗，僅顯示 GUI 視窗。
- `--uac-admin`：在 exe 內建 Manifest 加入系統管理員權限要求 (`requireAdministrator`)，確保啟動時自動彈出 UAC 提權視窗。
- `--name "智慧控管與監控系統 - 自動更新工具"`：指定產出檔名。

### 步驟 3：產出確認
打包完成後，執行檔將位於 `dist/` 目錄中：
- `dist/智慧控管與監控系統 - 自動更新工具.exe` (或可更名為 `dist/Updater.exe`)

---

## 🚀 發布與使用者操作說明

發布版本時，請將產出的執行檔與更新壓縮包放在**同一個資料夾**下打包提供給維運人員：

```text
📦 20261015-系統更新包/
 ├── 智慧控管與監控系統 - 自動更新工具.exe  (或 Updater.exe)
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
5. 程式將自動停止 Apache、執行解壓對齊覆蓋、重新啟動 Apache，完成後彈出完成訊息並可選擇是否清除已更新的壓縮包。

---

## 📂 專案檔案結構

```text
lah-updater/
 ├── updater.py                           # 主程式原始碼 (GUI、解壓防呆、Apache 控制)
 ├── 智慧控管與監控系統 - 自動更新工具.spec # PyInstaller 編譯設定檔
 ├── AGENTS.md                            # AI 代理人維護與架構指南
 ├── README.md                            # 專案說明與操作指南
 ├── .gitignore                           # Git 忽略檔案設定
 ├── build/                               # 打包過程暫存目錄 (Git 忽略)
 └── dist/                                # 打包最終產出目錄
      └── 智慧控管與監控系統 - 自動更新工具.exe
```

---

## 📝 授權說明
Internal System Utility - 供內部系統維運與自動更新使用。
