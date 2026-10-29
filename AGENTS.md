# AGENTS.md - Developer & AI Agent Context

本文件旨在為後續參與開發、維護或重構此專案的工程師與 AI 程式助理（Agents）提供系統架構說明、核心業務規則與開發守則。

---

## 1. 專案願景與基本原則

- **目標**：提供一套乾淨、輕量、秒開、免安裝、具備防呆機制且相容 Windows 權限體系的原生自動化維運工具套件。
- **核心架構**：
  - 維運四大工具全數採用 **Tauri 2 (Rust) + 純靜態 HTML/JS 前端** 原生架構實作（包含 updater、node-portable、uninstaller、backuper）。
  - 單一執行檔體積約 9~12 MB，冷啟動縮減至 0.3 秒以內，免除 Python 解壓暫存目錄，達成隨點秒開並大幅降低防毒軟體誤判率。
- **無外部運行庫依賴（Zero External Runtime Dependencies）**：
  - 核心程式碼嚴格使用 Rust 靜態編譯並自包含 WinSW 服務核心（`lah-service.exe`）。
  - 打包工具 `pack/pack.py` 嚴格僅使用 Python 標準庫，零第三方套件依賴。

---

## 2. 專案目錄結構與職責分工

```text
LAH-Updater/
├── updater/                 # 智慧控管與監控系統 - 自動更新工具 (Tauri 2 / Rust)
│   ├── src-tauri/          # Rust 核心 (5 階段更新、解壓對齊、Apache/PM2 服務配置)
│   ├── ui/                 # 純靜態 HTML/JS 介面
│   ├── service/            # WinSW 模板 (編譯時透過 include_bytes! 內嵌)
│   ├── build.py / build.bat# 一鍵編譯發布腳本 (輸出至 dist/)
│   └── README.md
├── uninstaller/            # 智慧控管與監控系統 - 自動移除工具 (Tauri 2 / Rust)
│   ├── src-tauri/          # Rust 核心 (服務停止/刪除、進程終止、PATH/排程清理、深度掃除)
│   ├── ui/                 # 純靜態 HTML/JS 介面
│   ├── build.py / build.bat# 一鍵編譯發布腳本 (輸出至 dist/)
│   └── README.md
├── backuper/               # 智慧控管與監控系統 - 備份與還原工具 (Tauri 2 / Rust)
│   ├── src-tauri/          # Rust 核心 (重要資料備份、OCI8/MSSQL DLL 掃描、復原點 undo)
│   ├── ui/                 # 彈現式日誌介面 (Modal Overlay)
│   ├── build.py / build.bat# 一鍵編譯發布腳本 (輸出至 dist/)
│   └── README.md
├── node-portable/          # 智慧控管與監控系統 - 可攜 Node 安裝工具 (Tauri 2 / Rust)
│   ├── src-tauri/          # Rust 核心 (Node 16.20.2 解壓、雙層 PATH 接管、WinSW 註冊、pm2-logrotate 預載)
│   ├── ui/                 # 純靜態 HTML/JS 介面 (Unicode 4.0 符號向下相容)
│   ├── service/            # WinSW 服務模板與 pm2_service_runner.js
│   ├── build.py / build.bat# 一鍵編譯發布腳本 (輸出至 dist/)
│   └── README.md
├── pack/                   # 系統原始碼自動打包發布工具 (Python 3.10+ 標準庫)
│   ├── pack.py / pack.bat  # 遍歷 LAH 各子專案打包為標準 .zip 更新壓縮包
│   └── README.md
└── dist/                   # 統一發布輸出目錄 (全工具產出執行檔集中於此)
```

---

## 3. 核心業務規則與安全防護

### 3.1 執行目錄定位 (`get_app_dir`)
程式執行時必須取得使用者放置 `.exe` 的真實目錄（而非暫存目錄或工作目錄）：
在 Rust 中透過 `std::env::current_exe()` 解析並取其父目錄。絕不可使用 `std::env::current_dir()`，避免捷徑或提權啟動時工作目錄被置於 `C:\Windows\System32`。

### 3.2 安裝包分類優先搜尋與平滑回退
自動更新工具探測更新包時，優先至分類專屬子目錄尋找：
- **基礎服務環境**（`Apache24.zip`、`php7.zip`、`php85.zip`、`node.zip`）：優先至 **`base/`** 子資料夾。
- **業務程式更新包**（`LAH-NUXTJS.zip`、`LAH-API.zip`、`www.zip`、`lah-messenger-server.zip`）：優先至 **`patch/`** 子資料夾。
- **依賴套件包**（`LAH-NUXTJS-node_modules.zip`、`lah-messenger-server-node_modules.zip`）：優先至 **`node_modules/`** 子資料夾。
- **更新說明文字檔**（`*更新說明*.txt`）：優先至 **`patch/`** 子資料夾，並依最後修改時間排序讀取最新檔案。
- **回退機制**：若各子資料夾未放置或不存在，自動回退至執行目錄根部、`C:\Temp` 與安裝根目錄搜尋。

### 3.3 雙層同名目錄防呆對齊 (`detect_strip_prefix`)
解壓壓縮包時必須檢測頂層結構，若含有同名單一資料夾（例如 `www.zip` 內含 `www/`），自動剔除外層前綴對齊解壓，徹底防範雙層巢狀目錄。

### 3.4 Node 16 與 PM2 四重安全隔離與殘留模組淨化
- **實體路徑綁定**：WinSW `lah_pm2_service.xml` 必須以實體絕對路徑綁定 `<root_dir>\node\node.exe` 與 `<root_dir>\node\node_modules\pm2\bin\pm2`，杜絕外部全域 Node 22+ 升級干擾。
- **PM2_HOME 統一**：強制作業系統與使用者環境變數 `PM2_HOME = <root_dir>\.pm2`。
- **殘留模組清除**：`wire_bundled_pm2_logrotate` 自動清除 `module_conf.json` 中之舊版 `@jessety/pm2-logrotate` 與磁碟不存在之幽靈模組，並繼承舊有輪替設定。
- **NPM.js 例外防禦**：修補 PM2 核心 `NPM.js` 之 `StartModule`，以 `try/catch` 封裝模組 `require()`，防止 `MODULE_NOT_FOUND` 拋出造成守護程序崩潰。

### 3.5 Apache 與 PHP 組態自癒與日誌校準
- **日誌路徑正規化**：動態將 `httpd.conf` 中 `ErrorLog` 與 `CustomLog` 之 `rotatelogs.exe` 目錄重寫為當前 `<apache_dir>/logs/`，防範跨磁碟移轉失敗。
- **舊版 AppServ 指令深度清理**：自動清除散落之 `LoadFile`、`LoadModule php7_module`、`PHPIniDir`、`<IfModule mod_php7.c>` 與舊版 `Alias /LAH-API`。
- **延伸模組去重**：過濾 `php.ini` 所有重複或註解之 `extension_dir`，嚴格保證僅保留唯一精準定義。

### 3.6 行程鎖定釋放與雙層目錄自癒
- **行程安全釋放**：解壓覆蓋前依序停止 Windows 服務、終止 PM2 Daemon，並確認 `node.exe` 檔案鎖已釋放（防止 `os error 5` 拒絕存取）。
- **雙層巢狀自癒**：解壓後若產生 `node\node`、`php7\php7` 等巢狀目錄，自動平移提升並清除空目錄。

### 3.7 零外部 CDN 與 Windows Server 相容
- **Zero-CDN**：全工具介面與文件嚴禁外連 CDN 或遠端字型，確保封閉隔離內網不逾時。
- **符號向下相容**：介面使用 SVG 或 Unicode 4.0 符號，根除 Windows Server 2012R2/2016 缺字方塊。

---

## 4. 常用測試與打包指令

- **建立自動更新工具**：
  ```powershell
  python updater\build.py
  # 或執行：.\建立自動更新工具.bat
  ```
- **建立自動移除工具**：
  ```powershell
  python uninstaller\build.py
  # 或執行：.\建立解除安裝工具.bat
  ```
- **建立可攜 Node 工具**：
  ```powershell
  python node-portable\build.py
  # 或執行：.\建立可攜式NODE安裝工具.bat
  ```
- **建立自動備份工具**：
  ```powershell
  python backuper\build.py
  # 或執行：.\建立自動備份工具.bat
  ```
- **原始碼全系統打包**：
  ```powershell
  python pack\pack.py
  # 或執行：.\打包智慧控管及監控系統.bat
  ```
- **輸出確認**：產出之執行檔統一集中於 `LAH-Updater/dist/` 目錄。
