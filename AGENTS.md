# LAH-Updater — 系統自動更新部署與離線維運規範

> **專案架構**：`tools/` 收納全系列 **Tauri 2 (Rust) + 純靜態 HTML/JS** 獨立維運工具（`updater`、`uninstaller`、`backuper`、`node-portable`、`dashboard`、`sms-env-editor`），`pack/pack.py`（Python 3.10+ 純標準庫）負責三向分流打包至 `dist/`（`base/`、`patch/`、`node_modules/`）。

---

## 1. 桌面維運工具核心防呆與 UI 鐵律

1. **真實執行目錄定位（`get_app_dir`）**：
   - Rust 必須透過 `std::env::current_exe()` 取父目錄定位 `.exe` 所在位置，**嚴禁使用 `std::env::current_dir()`**（防捷徑或提權啟動時落入 `C:\Windows\System32`）。
2. **主視窗禁止出現捲軸（`autoFitWindow` 強制規範）**：
   - 所有桌面工具主視窗在任何狀態（初始開啟、展開／收合選項、掃描完成後）**嚴禁出現垂直或水平 Scrollbar**，視窗必須貼齊內容。
   - 前端 `autoFitWindow()` 量測文檔流真實累積高度（排除 `position: sticky` 底部列位置，加 `4px` 浮點進位裕度），呼叫後端 `fit_window_to_content` 設定客戶區高度；在初始渲染、掃描完成、展開或收合後皆須觸發，**禁止留成空函式（no-op）**。
   - Tauri 2 `window.set_size()` 設定的是客戶區（inner size），不得疊加標題列與邊框高度；`tauri.conf.json` 預設高度須大於最高展開狀態，`body` 設為 `overflow-y: hidden`（彈窗內日誌長文字區塊不在此限）。
3. **更新包三向探測、壓縮包缺失防呆與雙層巢狀自癒**：
   - 優先探測分類子目錄（基礎環境 `base/`、業務程式 `patch/`、套件 `node_modules/`），未找到則依序回退至同層目錄、`C:\Temp` 與安裝根目錄；缺失對應 ZIP 時強制將核取方塊設為 `disabled`（`cursor: not-allowed`）。
   - 解壓前必須先停止 Windows 服務（`LAH_Web_Service` / `Apache2.4`、`LAH_PM2_Service`）、呼叫 `kill_pm2_daemon` 並確認 `node.exe` / `httpd.exe` 未被鎖定（`is_file_locked`，防 `os error 5`）。
   - 解壓時自動剝除同名單一頂層資料夾（`detect_strip_prefix`），解壓後巡檢並自動攤平 `node\node`、`php7\php7`、`php85\php85`、`Apache24\Apache24` 等雙層巢狀目錄（`auto_flatten_nested_dirs`）。
4. **離線環境（Zero-CDN）與舊版 Windows Server 相容**：
   - 所有工具介面與離線手冊（`README.html`、`INSTALLATION.html`）**嚴禁引用 Google Fonts、cdnjs、unpkg 等任何外部 URL**（防封閉內網 Timeout 卡死）；手冊須提供字體大小切換（一般、中[+15%]、大[+25%]，預設「中」）。
   - 避免使用 Unicode 11.0+ 新版 Emoji，改用內嵌 SVG 或向下相容之 Unicode 4.0 符號（如 `⚡`、`⚙️`、`🏠`），防止 Windows Server 2012R2/2016 顯示豆腐方塊。
5. **新工具專屬圖示（ICO）設計流程**：
   - 於 `tools/` 新增工具時**嚴禁沿用既有圖示**；動手前必須向使用者提出 2～3 組視覺與配色方案供選擇，定案後以 `generate_icons.ps1` 產出多解析度 `icon.ico`（16~256px）及完整 PNG 套裝並內嵌至 `Cargo.toml` / `tauri.conf.json`。

---

## 2. 伺服器沙盒隔離與組態自癒鐵律（Lesson Learnt）

### 2.1 Node.js 與 PM2 零全域污染沙盒隔離
1. **零全域環境變數污染**：嚴禁將 `<root_dir>\node` 或 `PM2_HOME` 寫入 `HKLM` / `HKCU` 全域登錄檔，並主動清理舊版殘留路徑；日常維運統一使用安裝根目錄之 `lah-cmd.bat`。
2. **WinSW 實體絕對路徑與私有 `PM2_HOME`**：`lah-service.xml`（`LAH_PM2_Service`）直接鎖定 `<root_dir>\node\node.exe`、`<root_dir>\node\node_modules\pm2\bin\pm2` 與私有環境變數 `PM2_HOME = <root_dir>\.pm2`，徹底脫鉤系統全域 Node 升級與 `%USERPROFILE%\.pm2`。
3. **殘留模組淨化（`wire_bundled_pm2_logrotate`）**：自動清除 `module_conf.json` 中舊版 `@jessety/pm2-logrotate`、`pm2-logrotate-ext` 及磁碟無對應 `package.json` 之幽靈模組，將輪替參數無損平移至標準 `pm2-logrotate`（預設 `compress: true`）。
4. **PM2 `NPM.js` 防崩潰保護**：於 `node\node_modules\pm2\lib\API\Modules\NPM.js` 注入路徑檢查與 `try/catch` 包裹 `require()`，防止缺失模組拋出 `MODULE_NOT_FOUND` 導致 PM2 Daemon 啟動崩潰。

### 2.2 Apache 2.4 & 雙 PHP 組態自癒與正則 `NoExpand` 防吞噬
1. **Web 服務沙盒化與資安脫敏**：以 WinSW 封裝 `LAH_Web_Service`（注入私有 `PATH` 含 `Apache24\bin`、`instantclient_11_2`、`php7` 並卸載舊 `Apache2.4` 服務）；`services.msc` 服務名稱全面使用公務中性名稱以防弱掃誤判；`vcredist-x64.zip` 解壓時同步補齊至 `Apache24\bin`、`php7` 與 `php85`。
2. **舊組態清理與路徑校準**：動態將 `httpd.conf` 中 `rotatelogs.exe` 舊路徑重寫為當前日誌目錄；清除散落之舊版 AppServ `LoadFile` / `mod_php7` 與失效 `Alias /LAH-API`；確保 `php.ini` 僅保留單一精準 `extension_dir = "<php_dir>/ext"`。
3. **Rust 正則替換強制 `regex::NoExpand`（防 `$1` 吞噬）**：
   - 當動態組態區塊（如 `fcgi_block`、`php2shell_block`、`xml_content`）含 `$` 字元時，**嚴禁直接傳入 `&str` 呼叫 `re.replace(text, &block)`**（正則引擎會將 `$1` 當成反向引用靜默吞噬為空字串 `""`，導致 FastCGI HTTP 503）。
   - **強制寫法**：一律包裝為 `regex::NoExpand(&block)`。
4. **`ProxyFCGISetEnvIf` 捕獲組語法**：
   - 剝除代理前綴時必須寫為 `SCRIPT_FILENAME "$1"`，**嚴禁誤寫為 `"$$1"`**（會被解析為 `$C:/...` 導致 `No input file specified.` HTTP 404）。
5. **雙軌等冪性單元測試**：所有組態修改函式的單元測試必須**連續執行兩次呼叫（初次注入 ➔ 二次覆蓋更新）**，並同時斷言包含 `"$1"`、不包含 `""`、不包含 `"$$1"`。

### 2.3 打包完整性守門員（`pack/pack.py`）
- **動態提取**：自動解析 `LAH-FE/nuxt.config.js` 的 `serverMiddleware` 與 `ecosystem.config.js`，動態納入中介軟體；`lah-messenger-server` 採用 `*.js` 萬用字元收錄。
- **事前與事後雙重守門員**：打包前缺失宣告檔案即拋出 `PackIntegrityError` 中止；打包後重開 ZIP 驗證核心檔案，不完整立即自動刪除銷毀 ZIP，並由 `python tests\test_pack_integrity.py` 把關。

### 2.4 Windows 維運排程與 PowerShell 腳本防禦規範
- **環境與編碼**：鎖定原生 PowerShell 5.1（零第三方模組）；含繁體中文之 `.ps1` / `.bat` 嚴格維持 **`CP950 / BIG5 + CRLF`**，嚴禁無 BOM 之 UTF-8 覆蓋造成終端亂碼。
- **無人值守防護**：週期巡檢腳本須具備公務上班時間檢查（非辦公時段 Early Return）；以 `NT AUTHORITY\SYSTEM`（如神網派送）執行時須先複製至 `C:\Temp` 本機或以專用網域帳號驗證掛載 UNC 路徑；呼叫含 `PAUSE` 之批次檔須透過管道注入空輸入防卡死。

---

## 3. 常用建置與測試指令

- **打包完整性測試**：`python tests\test_pack_integrity.py`
- **個別工具編譯（輸出至 `dist/`）**：`python tools\<updater|uninstaller|node-portable|backuper|sms-env-editor|dashboard>\build.py`
- **全系統原始碼打包**：`python pack\pack.py`
