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

### 2.2 更新包設定規範 (`PACKAGE_CONFIGS`)
目前系統維護 4 種更新包。若未來有新增或調整模組需求，僅需擴充 `PACKAGE_CONFIGS` 清單：
```python
PACKAGE_CONFIGS = [
    {
        "id": "www",
        "name": "www.zip",
        "rel_target": "www",
        "title": "前端 / 網站核心 (www.zip)",
        "expected_folder": "www"
    },
    ...
]
```
- `name`：同目錄下比對的 zip 檔名。
- `rel_target`：相對於根目錄（如 `C:\AppServ`）的解壓縮路徑。
- `expected_folder`：預期的頂層資料夾名稱（用於雙層目錄防呆對齊）。

---

### 2.3 【關鍵防呆機制】雙層目錄剔除 (`detect_strip_prefix`)
不同打包人員在壓縮 zip 時習慣不同：
- **情況 A**：點開 zip 第一層就是 `www/` 目錄，內層才是檔案。若直接解壓縮到 `C:\AppServ\www`，會變成 `C:\AppServ\www\www`。
- **情況 B**：點開 zip 直接就是 `index.php` 等散落檔案。

**判定邏輯**：
1. 檢視 zip 內所有非隱藏檔案之頂層路徑。
2. 若頂層「**不存在任何直接散落的檔案**」且「**僅有單一頂層資料夾**」。
3. 且該資料夾名稱符合 `expected_folder` 或與 zip 檔名相同（不分大小寫）。
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

### 2.6 版本更新說明比對規範 (`find_release_notes`)
- 採模糊比對：只要檔名以 `.txt` 結尾且含有「`更新說明`」字串（如 `★更新說明★.txt` 或 `1.9.5-★更新說明★.txt`）。
- 排序規則：取得所有符合檔案的 `os.path.getmtime`，由新至舊排序，**永遠載入最新修改日期的那一份**。
- 編碼容錯：依序嘗試 `utf-8-sig` ➔ `utf-8` ➔ `cp950` ➔ `big5` ➔ `latin-1`。

---

### 2.7 執行緒模型與 UI 互動
- 所有解壓縮與外部指令（`net`, `sc`）必須放在 `threading.Thread(..., daemon=True)` 執行。
- 主執行緒（Main GUI Thread）不得被阻斷。所有日誌附加與進度條更新必須透過 `self.root.after(0, ...)` 派發回主執行緒更新。

---

## 3. 打包與發布標準流程

使用 PyInstaller 進行編譯：
```powershell
pyinstaller --noconfirm --onefile --windowed --uac-admin --name "系統自動更新工具" updater.py
```

- `--uac-admin`：會在二進位清單中注入 `<requestedExecutionLevel level="requireAdministrator" ... />`，啟動時即觸發 Windows UAC。
- 輸出位置：`dist/系統自動更新工具.exe`。
- 通常發布時可將該檔額外複製一份命名為 `dist/Updater.exe`，以符合不同維運習慣。

---

## 4. 變更檢查清單 (Review Checklist for Agents)

當您修改或擴充本專案時，請逐項檢查：
- [ ] 是否保持標準庫設計，未引入額外第三方執行期套件？
- [ ] 是否保持 `sys.frozen` 相容性？
- [ ] 若更動解壓縮邏輯，是否維持 `detect_strip_prefix` 的雙層目錄防呆與 Zip Slip 安全防護？
- [ ] 若更動服務管理，是否維持 `finally` 區塊的 Apache 自動緊急重啟？
- [ ] 介面修改後，在變更根目錄時是否仍然正確動態更新目標路徑標籤？
- [ ] 是否在 Windows 終端執行過 `pyinstaller` 確認打包無錯誤產出？
