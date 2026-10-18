# AGENTS.md - Developer & AI Agent Context

本文件旨在為後續參與開發、維護或重構此專案的工程師與 AI 程式助理（Agents）提供系統架構說明、核心業務規則與開發守則。

---

## 1. 專案願景與基本原則

- **目標**：提供一個乾淨、輕量、免安裝、具備防呆機制且相容 Windows 權限體系的自動化解壓更新 GUI 工具。
- **無外部運行庫依賴（Zero External Runtime Dependencies）**：
  - 核心程式碼 **嚴格僅使用 Python 標準庫**（`tkinter`, `ttk`, `zipfile`, `subprocess`, `threading`, `ctypes`, `shutil` 等）。
  - **切勿** 引入如 `psutil`、`pywin32`、`requests` 等需要編譯或額外 pip 安裝的執行期套件，以確保編譯產出的單一執行檔體積小、啟動快且無 DLL 缺失風險。唯一允許的開發期套件僅為 `pyinstaller`。

---

## 2. 核心架構與重要元件說明

### 2.1 檔案與目錄定位 (`get_app_dir`)
由於打包為 PyInstaller `--onefile` 模式時，執行期會解壓縮至 `_MEIxxxxxx` 暫存目錄，但程式需要讀取的是 **使用者放置 `.exe` 的真實目錄**：
```python
def get_app_dir():
    if getattr(sys, 'frozen', False):
        return os.path.dirname(os.path.abspath(sys.executable))
    return os.path.dirname(os.path.abspath(__file__))
```
> ⚠️ **注意**：絕不可使用 `os.getcwd()` 作為更新包來源目錄，因為捷徑啟動或管理員提權啟動時，工作目錄可能被重置為 `C:\Windows\System32`。

---

### 2.2 更新包設定規範 (`PACKAGE_CONFIGS`) 與跨專案對應
目前系統維護 4 種更新包，分別對應 `LAH-Workspace` 工作區內之子專案產出。若未來有新增或調整模組需求，僅需擴充 `PACKAGE_CONFIGS` 清單：
```python
PACKAGE_CONFIGS = [
    {
        "id": "www",
        "name": "www.zip",
        "rel_target": "www",
        "title": "前端 / 網站核心 (www.zip)",
        "expected_folder": "www",
        "is_nodejs": False
    },
    ...
]
```
- `name`：同目錄下比對的 zip 檔名。
- `rel_target`：相對於根目錄（如 `C:\AppServ`）的解壓縮路徑。
- `expected_folder`：預期的頂層資料夾名稱（用於雙層目錄防呆對齊）。
- `is_nodejs`：標記該更新包是否屬於由 PM2 託管的 Node.js 服務模組（`LAH-NUXTJS.zip`、`node_modules.zip`、`lah-messenger-server.zip` 均為 `True`；`www.zip` 為 `False`）。

| 壓縮包檔名 | 對應工作區子專案 | `is_nodejs` | 目標部署說明 |
| :--- | :--- | :---: | :--- |
| `www.zip` | `./LAH-API/www` | `False` | Apache PHP 後端核心與舊版網頁 |
| `LAH-NUXTJS.zip` | `./LAH-FE` | `True` | Nuxt 2 智慧管控前端與排程 (`LAH-FE`, `LAH-SCHEDULER`) |
| `node_modules.zip` | `./LAH-FE/node_modules` | `True` | 前端執行期 Node.js 依賴套件 |
| `lah-messenger-server.zip` | `./lah-messenger-server` | `True` | 即時通訊 WebSocket 伺服器 (`LAH-WSS`) |

---

### 2.3 【關鍵防呆機制】雙層目錄剔除 (`detect_strip_prefix`)
不同打包人員在壓縮 zip 時習慣不同：
- **情況 A**：點開 zip 第一層就是 `www/` 目錄，內層才是檔案。若直接解壓縮到 `C:\AppServ\www`，會變成 `C:\AppServ\www\www`。
- **情況 B**：點開 zip 直接就是 `index.php` 等散落檔案。

**判定邏輯**：
1. 檢視 zip 內所有非隱藏檔案之頂層路徑。
2. 若頂層「**不存在任何直接散落的檔案**」且「**僅有單一頂層資料夾**」。
3. 且該資料夾名稱符合 `expected_folder`、壓縮檔主檔名或目標路徑結尾目錄名稱（不分大小寫）。
4. 判定為外層包裝目錄，回傳前綴（如 `www/`），解壓縮時自動剔除此外層前綴；否則回傳空字串 `""` 直接解壓。

---

### 2.4 Windows 繁體中文檔名編碼支援 (`get_zip_member_path`)
舊版 Windows 內建壓縮工具或 WinRAR 經常使用 Big5/CP950 編碼，而未使用 UTF-8 標記（Bit 11）：
- 當 `info.flag_bits & 0x800` 為真：表示原檔名即為 UTF-8。
- 否則：以 `cp437` 取得原始 byte 串列後，依序嘗試以 `utf-8`、`cp950`、`gbk` 進行解碼，避免檔案覆蓋時產生亂碼路徑。

---

### 2.5 Apache 服務控制與容錯機制
- **服務偵測**：依序查詢 `["Apache2.4", "Apache24", "Apache2.2", "Apache2", "Apache", "httpd"]`，若均未中則退回 PowerShell 查詢 `*apache*`。
- **停止與啟動**：透過 `net.exe stop/start` 搭配 `sc.exe query` 輪詢驗證狀態（上限 15 秒）。
- **重大異常防護**：
  在 `_run_update_process` 中，以 `apache_was_stopped` 與 `apache_restarted` 雙旗標追蹤。在 `finally` 區塊中：
  ```python
  if should_restart_apache and svc_name and apache_was_stopped and not apache_restarted:
      self.log(f"⚠️ 偵測到更新中斷或異常，緊急嘗試恢復 Apache 服務 [{svc_name}]...", "warning")
      start_apache_service(svc_name, self.log)
      apache_restarted = True
  ```
  確保即便解壓縮出錯（如權限被鎖定），Apache 也能盡最大可能被拉起，避免服務停擺。

---

### 2.6 PM2 Node.js 服務重啟機制 (`resolve_pm2_command` / `restart_pm2_services`)
- **智慧觸發條件**：僅當本次勾選更新的項目包含 `is_nodejs=True` 之模組（`LAH-NUXTJS.zip`、`node_modules.zip`、`lah-messenger-server.zip`）時，於全部壓縮包解壓與 Apache 恢復完成後、彈出刪除壓縮包確認前，主動詢問使用者是否執行 `pm2 restart all`。
- **Windows `pm2.cmd` 尋址與執行規範**：
  - 在 Windows 下，`pm2` 為批次檔（`pm2.cmd`），不可直接以 `subprocess.run(["pm2", ...])` 呼叫（會拋出 `FileNotFoundError`）。
  - 透過 `resolve_pm2_command()` 優先以 `shutil.which("pm2.cmd")` / `shutil.which("pm2")` 解析；若因 UAC 管理員提權導致 `PATH` 遺失，自動回退檢查 `%APPDATA%\npm\pm2.cmd`、`%USERPROFILE%\AppData\Roaming\npm\pm2.cmd`、`C:\Program Files\nodejs\pm2.cmd` 與 `C:\nvm4w\nodejs\pm2.cmd`。
  - 執行時透過 `cmd.exe /d /s /c "... restart all --no-color"` 呼叫，設定 60 秒逾時與 `CREATE_NO_WINDOW`，並透過 `_decode_console_bytes` 進行 `utf-8` / `cp950` 解碼與 ANSI 控制碼剝除，將輸出即時寫入 GUI 日誌視窗。

---

### 2.7 版本更新說明比對與獨立彈窗規範 (`find_release_notes` / `_show_release_notes_dialog`)
- **省空間彈窗設計**：主畫面不常態內嵌長篇說明文字框（主視窗採 `880x580` 緊湊佈局），改於底部操作列「開始更新」旁設置 `self.btn_view_notes` 按鈕。
- **動態按鈕狀態**：同目錄下若偵測到說明檔，按鈕啟用（顯示「📋 查看更新說明」）；若未提供說明檔，按鈕自動反灰停用（顯示「📋 無更新說明」）。
- **獨立彈出對話框**：點擊按鈕彈出置中之 `Toplevel` 對話框（約 `640x500`），文字區塊唯讀呈現，並支援點擊「關閉」或按下鍵盤 `Esc` 快速退出。
- **比對與排序規則**：檔名以 `.txt` 結尾且含有「`更新說明`」字串（如 `★更新說明★.txt`），依 `os.path.getmtime` 由新至舊排序，**永遠載入最新修改日期的那一份**；編碼依序嘗試 `utf-8-sig` ➔ `utf-8` ➔ `cp950` ➔ `big5` ➔ `gbk` ➔ `latin-1`。

---

### 2.8 執行緒模型與 UI 互動
- 所有解壓縮與外部指令（`net`, `sc`, `pm2`）必須放在 `threading.Thread(..., daemon=True)` 背景執行緒中執行。
- 主執行緒（Main GUI Thread）不得被阻斷。所有日誌附加、進度條更新與 `messagebox` 彈窗必須透過 `self.root.after(0, ...)` 派發回主執行緒更新（若需於背景執行緒等待使用者對話框選擇，應搭配 `threading.Event` 同步）。

---

## 3. 驗證與打包發布標準流程

### 3.1 語法檢查（修改後必跑）
```powershell
python -m py_compile updater.py
```

### 3.2 使用 PyInstaller 進行編譯（需人工授權）
建議直接使用專案內建之一鍵打包腳本（已整合純 ASCII 批次檔轉發與 Python 原生 UTF-8 繁體中文檔名防衝碼處理）：
```powershell
# 方式 A：一鍵執行批次檔
.\build.bat

# 方式 B：直接執行 Python 打包腳本
python build.py

# 方式 C：手動執行 PyInstaller 指令
python -m PyInstaller --noconfirm --onefile --windowed --uac-admin --name "智慧控管與監控系統 - 自動更新工具" updater.py
```

- `--uac-admin`：會在二進位清單中注入 `<requestedExecutionLevel level="requireAdministrator" ... />`，啟動時即觸發 Windows UAC。
- 輸出位置：`dist/智慧控管與監控系統 - 自動更新工具.exe`。
- `build.py` 在編譯完成後會自動同步於 `dist/` 額外複製一份命名為 `dist/Updater.exe`，以符合不同維運習慣。

---

## 4. 變更檢查清單 (Review Checklist for Agents)

當您修改或擴充本專案時，請逐項檢查：
- [ ] 是否保持標準庫設計，未引入額外第三方執行期套件？
- [ ] 是否執行過 `python -m py_compile updater.py` 確認語法無誤？
- [ ] 是否保持 `sys.frozen` 相容性？
- [ ] 若更動解壓縮邏輯，是否維持 `detect_strip_prefix` 的雙層目錄防呆與 Zip Slip 安全防護？
- [ ] 若更動服務管理，是否維持 `finally` 區塊的 Apache 自動緊急重啟與 PM2 背景非阻斷重啟？
- [ ] 介面修改後，在變更根目錄時是否仍然正確動態更新目標路徑標籤？
- [ ] `.gitignore` 是否持續排除測試用 `*.zip`、`*更新說明*.txt` 與 `build/`、`dist/` 產物？
