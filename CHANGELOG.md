# 智慧控管與監控系統 - 自動更新與維運工具 (LAH-Updater) 版本修改歷程

本文件記錄 `LAH-Updater` 維運工具套件（包含自動更新工具 `updater.py`、自動移除工具 `uninstaller.py` 以及系統打包腳本 `pack.py`）之功能演進、架構調整與資安防護修復歷程。

---

## [未發布]

### 🎨 重新設計備份工具 (backuper) 專屬圖示並升級建置管線

- **修改目的**：
  - 全面重新設計自動備份與還原工具（backuper）應用程式圖示，捨棄舊版平面打勾徽章圖樣，改為高度契合資料庫備份還原意象之 2.5D 微立體雙向循環儲存樞紐圖示。
  - 對齊 LAH-Updater 維運三件套整體視覺語彙（更新：翡翠綠、備份：科技藍、移除：警戒紅），採用 Squircle 圓角科技深藍底座搭配高光青天藍邊框。
  - 解決 Cargo 建置快取未自動偵測圖示變更之問題，於 `build.rs` 注入依賴宣告與標準 UAC / Common-Controls 清單，並重新編譯發布二進位執行檔。
- **具體修改項目**：
  - **圖示重新設計 (`backuper/src-tauri/icons/`)**：
    - 重新繪製 512x512 高解析度母圖與全尺寸點陣圖（`icon.png`、`128x128@2x.png`、`128x128.png`、`32x32.png`）。
    - 採用現代 2.5D 等角微立體視角，融合三層深藍金屬光澤資料庫磁碟盤、同心雷射軌道刻紋、主軸心指示燈與流暢順逆時針雙向環抱之青天藍發光循環箭頭。
    - 產出包含 16x16、24x24、32x32、48x48、64x64、128x128、256x256 之標準 Windows 多解析度 `icon.ico`。
  - **建置管線升級與自動化工具 (`backuper/src-tauri/build.rs` & `generate_icons.ps1`)**：
    - 於 `build.rs` 新增 `cargo:rerun-if-changed` 追蹤 `icon.ico` 與 `tauri.conf.json`，並配置 WindowsAttributes（Common-Controls v6.0 與 requireAdministrator UAC 資訊清單）。
    - 建立確定性圖示自動生成腳本 `backuper/generate_icons.ps1`，便於未來自動重新產出全尺寸圖示。
    - 執行 `backuper/build.py` 重新編譯產出最新 `dist/智慧控管與監控系統 - 備份與還原工具.exe`（9.54 MB），並實機完成二進位檔內嵌圖示提取與外觀驗證。
- **相容性說明**：
  - 僅更新應用程式視覺圖示與建置清單，不變更任何備份還原邏輯、壓縮包路徑或跨系統介面契約。
  - 最新執行檔已覆蓋至 `LAH-Updater/dist/` 目錄。

### 📖 新增全系統架構純離線手冊 (README.html) 與更新工具單元測試擴充

- **背景與目的**：
  - 為使維運與開發人員能於無外部網路連線（Zero-CDN）的伺服器與隔離網路環境中，完整查閱工作區 5 大平級 Git 專案拓撲、LAH-Updater 5 大維運工具實作細節、打包分流目錄規範以及實機崩潰根因防禦機制。
  - 將前次針對舊全域模組殘留崩潰之修復與單元測試防禦完整覆蓋至自動更新工具（updater），並重新編譯產出最新 Release 二進位檔。
- **具體改進項目**：
  - **純離線技術手冊 (`README.html` & `generate_readme_html.py`)**：
    - 於 `LAH-Updater/` 根目錄新增純 HTML5/CSS3/Vanilla JS 打造之獨立技術手冊 `README.html`，100% 離線可用，絕無外部 CDN、遠端字型或腳本依賴。
    - 內嵌 4 大向量 SVG 架構圖：工作區全域系統拓撲圖（圖 1）、資產打包與更新部署流向圖（圖 2）、自動更新管線 5 階段流程圖（圖 3）、可攜 Node 16 與全域升級四重安全隔離架構圖（圖 4）。
    - 完整解析 5 大平級 Git 子專案（`LAH-API`、`LAH-FE`、`lah-messenger-server`、`lah-messenger`、`LAH-Updater`）之職責、邊界與通訊契約。
    - 深入說明 LAH-Updater 5 大工具（`updater`、`node-portable`、`uninstaller`、`backuper`、`pack.py`）之功能、參數規格、自癒機制與本機快捷腳本。
    - 內建側邊欄即時關鍵字篩選、目錄滾動監聽（ScrollSpy）、模組頁籤切換與命令一鍵複製等現代化原生互動體驗。
  - **自動更新工具防護與單元測試擴充 (`updater/src-tauri/src/pm2.rs`)**：
    - 將 `test_wire_bundled_pm2_logrotate_idempotent` 單元測試擴充納入 `updater` 測試套件，斷言舊版 `@jessety/pm2-logrotate` 殘留參數繼承與孤立幽靈模組清理 100% 正確通過。
    - 通過全部 6 項 Rust 單元測試（`cargo test --lib`）。
    - 重新編譯發布 `dist/智慧控管與監控系統 - 自動更新工具.exe`（11.82 MB）。

### 🐛 修復舊全域殘留 @jessety/pm2-logrotate 導致 PM2 啟動 MODULE_NOT_FOUND 錯誤

- **根本原因排查**：
  - 在既有 Windows 伺服器環境中（如先前曾使用 `pm2-installer` 部署之主機），`C:\ProgramData\pm2\home\module_conf.json` 曾登記舊全域模組 `@jessety/pm2-logrotate`。
  - 當工具自舊全域遷移 `module_conf.json` 至 `<根目錄>\.pm2\` 後，PM2 Daemon 在啟動時會讀取 `module-db-v2` 嘗試載入所有登記之模組。
  - 由於新可攜版環境下並不存在 `@jessety/pm2-logrotate` 目錄（且未連網下載），PM2 `NPM.js` 的 `StartModule` 在執行 `require(package.json)` 時丟出未捕獲之 `MODULE_NOT_FOUND` 異常，導致 `pm2 -v` 初始化直接崩潰（回傳碼 1）。
- **修復與防禦強化**：
  - **自動清理與設定遷移 (`wire_bundled_pm2_logrotate`)**：
    - 自動清除 `module_conf.json` 中之 `@jessety/pm2-logrotate` 與 `pm2-logrotate-ext` 鍵值，並將其既有自訂參數（`max_size`、`retain`、`dateFormat` 等）完整繼承至官方標準 `pm2-logrotate`。
    - 遍歷 `module-db-v2`，若在 `.pm2\modules` 與 `node\node_modules` 均未發現模組 `package.json`，自動將該孤立項目自 `module-db-v2` 剔除，防止 PM2 啟動崩潰。
  - **PM2 模組載入器防禦性修補 (`NPM.js`)**：
    - 修補 `NPM.js` 的 `start` 函式，當模組路徑不存在且便攜版目錄亦無此模組時，安全跳過並呼叫回呼函式，不再傳入無效路徑。
    - 修補 `StartModule` 函式，以 `try/catch` 封裝 `require(opts.cmd)`，避免 require 異常中斷主進程。
  - **背景服務守護同步 (`pm2_service_runner.js` / `service_runner.js`)**：
    - 在服務背景巡檢與預掛載時同步執行舊全域與孤立模組清理。
  - **單元測試防護**：
    - 更新 `test_wire_bundled_pm2_logrotate_idempotent` 測試，模擬包含 `@jessety/pm2-logrotate` 與孤立模組之情境，斷言自動淨化與設定遷移 100% 成功。


- **背景與目的**：解決在 Windows Server 2016 等既有全域已安裝 Node.js (v22) 與 global PM2 的伺服器環境中，可攜版工具未能自動辨識已安裝根目錄、字型缺少 Emoji 導致方框缺字、舊全域 Node/PM2 環境變數與服務衝突，以及便攜版內建之 `pm2-logrotate` 顯示為「尚未安裝」之問題。
- **具體改進項目**：
  - **Windows Server 2016 相容性與介面圖示修復**：
    - 將 `<h1>` 與 `<h2>` 標題之綠色方塊 Emoji 替換為與應用程式圖示一致的內嵌向量 SVG 圖示。
    - 將核取方塊與狀態文字之 Unicode 11.0/12.0 圖示替換為向下相容之 Unicode 4.0 符號（如 `⚡`、`⚙️`、`🏠`），根除舊版 Windows Server 上的字型豆腐方塊缺字。
  - **安裝根目錄智慧自動偵測**：
    - 啟動時自動依序探測 `PM2_HOME`、`PATH`（尋找包含 `node.exe` 之目錄上層）、系統預設目錄（`D:\DEV`、`C:\AppServ`），精準載入已安裝環境。
  - **系統與使用者環境變數全面接管**：
    - 同時清理並前置注入系統（`HKLM`）與使用者（`HKCU`）`PATH`，徹底剔除舊全域 Node/PM2 路徑。
    - 移除舊版 `pm2-installer` 遺留之系統與使用者變數 `PM2_INSTALL_DIRECTORY` 與 `PM2_SERVICE_DIRECTORY`。
    - 統一設定系統與使用者環境變數 `PM2_HOME` 為 `<根目錄>\.pm2`。
  - **舊全域 PM2 服務與殘留進程清理**：
    - 安裝前自動查詢、停止並刪除舊全域 `pm2.exe` Windows 系統服務。
    - 終止非目標可攜版路徑之 Node.js、PM2 Daemon 及殘留 FastCGI 進程。
    - 自動清理與重寫 `dump.pm2`，將其內殘留之舊全域 node.exe 解譯器路徑重設為可攜版 `node`，並修正 PID 路徑至統一 `.pm2\pids`。
  - **便攜版內建 `pm2-logrotate` 模組直接掛載與啟動**：
    - 新增 `logrotate_bundled_version`，讀取 `<根目錄>\node\node_modules\pm2-logrotate\package.json`（版本 3.0.0）。
    - 實作 `wire_bundled_pm2_logrotate` 與 `pm2_service_runner.js` 之 `wireBundledLogrotate`，直接將便攜版 `node_modules\pm2-logrotate\app.js` 掛載至 `.pm2\modules\pm2-logrotate`，並於 `.pm2\module_conf.json` 寫入預設設定（啟用 `compress: true`）。
    - 修補 PM2 之 `NPM.js` 模組啟動函式，若 `.pm2\modules` 缺失自動退回便攜版目錄載入。
    - 改善前端環境狀態顯示，明確呈現 `3.0.0（便攜版內建，已掛載啟用）`。
  - **系統升級全域 Node 防護說明文件化**：
    - 於 `updater/README.md` 詳細增補「系統全域 Node.js 因資安要求升級時的影響說明」專章，闡述 WinSW 服務硬鎖定、`pm2.cmd` 前置注入、Daemon/子程序 `process.execPath` 繼承與 60 秒自癒機制。
    - 於 `updater/service/README.md` 同步補齊 WinSW 原生系統服務之四重安全防護機制與便攜版內建 `pm2-logrotate` 預先掛載說明。
    - 於 `node-portable/README.md` 詳細增補「系統全域 Node.js 因資安要求升級時的影響說明」專章。
    - 於專案根目錄 `README.md` 補齊 `updater/` 與 `node-portable/` 模組職責說明、一鍵建置批次指令與發布執行檔清單。
    - 同步更新 `updater/` 內部之 PM2 日誌輪替直接掛載機制與單元測試。
  - **單元測試防護**：
    - 新增 `test_clean_and_prepend_path_replaces_global_node_and_pm2` 與 `test_wire_bundled_pm2_logrotate_idempotent` 測試，確保環境變數替換與模組掛載之正確性與冪等性。


### ✨ 新增可攜 Node.js + PM2 安裝工具 (node-portable)
- **背景與目的**：因應資安要求，原以 Node.js 16.20.2 開發的程式需搭配綠色版 Node 與 PM2；自 `updater/` 抽出相關核心程式碼，建立不含 LAH 專屬巡檢的通用離線安裝工具。
- **新增項目**：
  - `node-portable/`：Tauri 2 (Rust) + 純靜態 HTML/JS 工具，離線讀取 `node.zip`（依序搜尋工具旁 `base/`、工具同層、`C:\Temp`、安裝根目錄），解壓至 `<根目錄>\node`，並自動剝除頂層 `node/` 前綴。
  - `node-portable/src-tauri/src/pm2.rs`：抽取自 updater 的 PM2 綠色環境修補（統一 `<根目錄>\.pm2`、IPC 改為 Loopback TCP 42681/42682、Daemon/Utility/NPM/logrotate 修補），並新增 `PM2_Portable_Service` 服務註冊（WinSW）。
  - `node-portable/src-tauri/src/system.rs`：抽取使用者 PATH 註冊並新增 `PM2_HOME` 使用者環境變數寫入。
  - `node-portable/src-tauri/src/install.rs`：安裝流程（停止舊服務與 pm2 kill、解壓、修補、環境變數、`node -v` / `pm2 -v` 驗證、服務註冊或 `pm2 resurrect`）。
  - `node-portable/service/`：複製自 updater 的 WinSW 執行檔 `pm2-service.exe`，以及新寫的通用服務入口 `pm2_service_runner.js`（開機與每 60 秒探測 PM2 Daemon，離線即 `pm2 resurrect`，不綁定任何 LAH 服務）。
  - `node-portable/build.py`、`build.bat`、`README.md`，以及根目錄 `建立可攜NODE工具.bat`（雙擊即編譯並輸出至 `dist/`）。
  - 應用程式圖示採「六角形 + PM2 循環箭頭」設計，延續各維運工具之圓角深色底與彩色外框風格。
  - `.gitignore`：排除 `node-portable` 編譯產物。
- **相容性說明**：服務名稱為 `PM2_Portable_Service`，與 updater 的 `LAH_PM2_Service` 互不衝突；未修改 updater、uninstaller、backuper 既有程式碼。
- **驗證狀態**：`cargo check` 與 `cargo build --release` 已通過；GUI 實機安裝（PATH、服務註冊）尚未驗證。
### 🐛 修復自動更新工具分流探測 ID 不一致問題 (Issue: 3 個安裝包未偵測到)
- **根本原因排查**：
  - `ALL_PACKAGE_SPECS` 中定義的套件識別代碼分別為 `messenger`（即時通伺服器）、`lah_nuxtjs_modules`（前端核心套件）與 `messenger_modules`（即時通核心套件）。
  - `get_preferred_subfolder` 優先分類子資料夾比對時，原本寫入長命名（`lah_messenger_server`、`lah_nuxtjs_node_modules`、`lah_messenger_server_node_modules`），導致比對失效回傳 `None`，未進入 `patch/` 與 `node_modules/` 優先探測路徑。
  - 由於未在分流資料夾被偵測到，且更新包已分流不在根目錄，因而前端顯示為「未找到壓縮包」。
- **修復與健全性強化**：
  - 同步更新 `get_preferred_subfolder` 模式匹配，同時支援標準 ID（`messenger`、`lah_nuxtjs_modules`、`messenger_modules`）與長命名別名。
  - 將專屬子資料夾優先探測升級為遍歷所有 `search_dirs`，確保在各搜尋目錄下的 `base/`、`patch/`、`node_modules/` 皆能自動優先探測。
  - 補齊單元測試確保所有 10 大安裝包之分類目錄配置 100% 覆蓋。
  - 重新編譯產出 Release 二進位執行檔至 `dist/智慧控管與監控系統 - 自動更新工具.exe`。

### 🧹 移除舊版英文建置批次檔 (build_uninstaller.bat)
- **統一使用標準繁體中文建置指令**：
  - 移除專案根目錄之舊英文別名批次檔 `build_uninstaller.bat`，全面統一使用標準繁體中文命名 `建立解除安裝工具.bat`。
  - 移除 `uninstaller/` 子目錄內殘留之 `build_uninstaller.bat`，子目錄內建置統一使用 `build.bat` 或 `build.py`。
  - 清理專案結構，維持目錄命名與操作規範之一致性與清晰度。



### 📦 原始碼打包工具 (pack.py) 支援分流輸出 (`patch/`、`node_modules/`、`base/`)
- **預設輸出基底導向專案 `dist/` 並分流歸檔**：
  - 業務程式更新包（`www.zip`、`LAH-API.zip`、`LAH-NUXTJS.zip`、`lah-messenger-server.zip`）自動輸出至 `dist/patch/`。
  - 核心依賴套件包（`LAH-NUXTJS-node_modules.zip` 與 `lah-messenger-server-node_modules.zip`）自動輸出至 `dist/node_modules/`。
  - 基礎服務環境包（`node.zip`）自動輸出至 `dist/base/`。
  - 自動將最新版 `*更新說明*.txt` 複製同步至 `dist/patch/`。
  - 支援 `--output-dir` 參數自訂基底目錄時維持同等分流架構。


### 🧹 全面移除舊版 Python 原始碼，邁向純 Rust (Tauri 2) 架構
- **移除舊版 Tkinter 主程式原始碼**：
  - 徹底移除 `updater/updater.py`（原 5,074 行 Python 更新工具）與歷史 `.spec` 打包設定檔。
  - 徹底移除 `uninstaller/uninstaller.py`（原 3,000+ 行 Python 移除工具）與 `uninstaller/build_uninstaller.py`。
  - 清理全專案殘留之 `__pycache__` 快取目錄。
- **保留專用建置與打包工具**：
  - 保留各工具目錄下之 `build.py`（自動調用 `cargo build --release` 並發布至 `dist/`）。
  - 保留 `pack/pack.py` 系統原始碼全自動打包發布工具。
- **更新專案規格與文檔架構**：
  - 同步重構更新根目錄與子專案 `AGENTS.md`、`README.md`，全面轉向純 Tauri 2 (Rust) 開發與維護規格。


### 🗂️ 專案維運工具模組化拆分至獨立子目錄 (`updater/`, `uninstaller/`, `pack/`, `backuper/`)
- **檔案與依賴隔離至專屬資料夾**：
  - `updater/`：收納 `updater.py`、`build.py`、WinSW 服務模板 `service/`、更新說明文字檔以及打包規格檔。
  - `uninstaller/`：收納 `uninstaller.py`、`build_uninstaller.py`。
  - `pack/`：收納系統原始碼打包腳本 `pack.py`。
  - `backuper/`：收納重要資料備份與還原工具 (Tauri 2 / Rust)。
- **產出路徑統一收納至根目錄 `dist/`**：
  - `updater/build.py`、`uninstaller/build_uninstaller.py`、`pack/pack.py` 與 `backuper/build.py` 全數將執行檔產出目錄導向至專案根目錄 `dist/`（`LAH-Updater/dist/`），免去各子目錄分散尋找困擾。
- **路徑動態探測強化與 BAT 轉發調校**：
  - `pack/pack.py`：`resolve_project_dirs` 升級為向上自動探測包含 `LAH-API` 與 `LAH-FE` 的工作區根目錄，相容直接執行或自任何層級呼叫。
  - `updater/updater.py`：WinSW `service` 模板搜尋路徑擴充 `updater/service`，`find_release_notes` 支援同時查找同層與上層目錄更新說明檔。
  - 根目錄批次檔（`建立自動更新工具.bat`、`build_uninstaller.bat`、`打包智慧控管及監控系統.bat`）自動轉發至各子目錄執行檔，各子目錄內亦提供專屬 `.bat`。

- **新增一鍵建置批次檔與發布腳本**：
  - 於專案根目錄新增 `建立自動備份工具.bat`（相容別名 `建立backuper.bat`、`build_backuper.bat`），對齊 `建立自動更新工具.bat` 習慣。
  - 新增 `backuper/build.py` 與 `backuper/build.bat`，自動執行 `cargo build --release` 並將產出直接輸出至專案根目錄 `dist/智慧控管與監控系統 - 備份與還原工具.exe`。

### 🦀 自動更新工具改以 Tauri 2 (Rust) 全新改寫 (`updater/`)
- **安裝包依分類分流優先搜尋與回退相容（雙軌支援 Tauri 2 與 Python）**：
  - 基礎服務安裝包（`apache24`、`php7`、`php85`、`node`）優先至執行目錄下之 `base/` 資料夾探測。
  - 程式更新包（`lah_nuxtjs`、`lah_api`、`www`、`lah_messenger_server`）優先至執行目錄下之 `patch/` 資料夾探測。
  - 依賴套件包（`lah_nuxtjs_node_modules`、`lah_messenger_server_node_modules`）優先至執行目錄下之 `node_modules/` 資料夾探測。
  - 若各專屬子資料夾不存在或未偵測到相應壓縮包，自動平滑回退至執行目錄、`C:\Temp` 與安裝根目錄搜尋，100% 維持既有使用習慣與相容性。
  - 更新說明文字檔（`*更新說明*.txt`）亦優先探測 `patch/` 目錄，並依最後修改時間排序載入最新檔案。
- **底層核心完全改以 Rust 實作**：
  - 以 Tauri 2 + 純靜態 HTML/JS 完整移植 Python 既有更新功能，包含 10 大更新壓縮包智慧對齊解壓（`detect_strip_prefix` 消除多餘頂層目錄）、5 階段系統自動化配置引擎（Apache mod_php7、PHP 8.5 FastCGI Proxy、CVE-2024-4577 資安強化、綠色 Node/PM2 設定與 WinSW 服務部署、服務重啟與全系統健康檢查）。
  - 透過 `include_bytes!` 內嵌自包含 WinSW 核心（`lah-service.exe` 與相關腳本），免除外部資料夾遺失相依性問題。
  - Windows 單一執行檔體積約 11.5 MB，冷啟動縮減至 0.3 秒以內。
- **初次安裝智慧引導**：
  - 程式啟動時自動檢測預設安裝目錄（`C:\AppServ`）；若不存在即刻彈出引導對話框，提供「一鍵建立預設目錄」、「瀏覽選取其他磁碟」或「稍後手動設定」。
- **介面效能與版面調校**：
  - 實作初始清單 0 延遲同步渲染，啟動瞬間立即可見完整版面，消除背景非同步掃描造成的延遲感。
  - 視窗預設高度設定為 665px，密合卡片與底端操作按鈕，消除多餘空白。
- **建置管線對齊規範與專屬圖示更新**：
  - 重新繪製具備深翡翠綠底座、系統工程齒輪、向上躍升升級箭頭與同步光環之專屬 ICO/PNG 圖示，替換舊有即時通圖示。
  - 更新 `updater/build.py` 與 `updater/build.bat`，自動執行 `cargo build --release` 並發布至 `dist/智慧控管與監控系統 - 自動更新工具.exe`。
  - `.gitignore` 新增 `updater/src-tauri/target/`、`updater/src-tauri/gen/`。

### 🦀 自動移除工具改以 Tauri 2 (Rust) 全新改寫 (`uninstaller/`)
- **底層核心完全改以 Rust 實作**：
  - 以 Tauri 2 + 純靜態 HTML/JS 完整移植 Python 既有移除功能，包含停止/刪除 Windows 服務（`LAH_PM2_Service`、`Apache2.4`）、精確終止殘留進程（`pm2 kill` 與 PowerShell 樹狀行程刪除）、清理開機自啟動與工作排程、自系統與使用者 `PATH` 環境變數剔除並廣播 `WM_SETTINGCHANGE`、Apache PHP 轉發設定防護性註銷與語法驗證、解除檔案唯讀屬性強健刪除、以及安裝根目錄深度清理（嚴格保護 `_LAH_Backup_*.zip`）。
  - 產出之 Windows 單一執行檔體積縮減至 10.68 MB，免去 Python 解壓暫存目錄，達成隨點隨開與零防毒誤判。
- **建置管線對齊 backuper 規範與專屬圖示更新**：
  - 重新繪製具備警戒深紅底座、系統齒輪解構、清理垃圾桶與立體粗體移除叉號之專屬 ICO/PNG 圖示，替換舊有即時通圖示。
  - 新增 `uninstaller/build.py` 與 `uninstaller/build.bat`，自動執行 `cargo build --release` 並將二進位檔直接發布為 `dist/智慧控管與監控系統 - 自動移除工具.exe`。
  - 專案根目錄之 `建立解除安裝工具.bat` 與 `build_uninstaller.bat` 同步轉發至最新 Rust 建置腳本。
  - `.gitignore` 新增 `uninstaller/src-tauri/target/`、`uninstaller/src-tauri/gen/`。

### 📦 備份功能獨立為 `backuper/`（Tauri 2）
- 新增 `backuper/`：以 Tauri 2 + 純靜態 HTML/JS 實作系統重要資料備份與還原；還原前檢查服務已停止、防 Zip Slip，並自動建立 `restore_undo_<時間>.zip` 復原點。
- **擴充關鍵資料庫連線依賴備份**：
  - 自動偵測並打包 `php7/ext/` 與 `php85/ext/` 底下之 OCI8 與 MSSQL 擴充套件 DLL（`php_oci8*.dll`、`php_pdo_sqlsrv*.dll`、`php_sqlsrv*.dll`）。
  - 自動完整打包 Oracle 核心驅動資料夾 `instantclient_11_2/`。
- **專屬科技藍備份與還原圖示**：
  - 重新繪製具備三層資料庫磁碟、雙向循環備份還原箭頭與安全守護意象之專屬 ICO/PNG 圖示，替換舊有即時通圖示。
- **視窗版面空間最佳化（彈現式日誌視窗）**：
  - 下方執行進度條與黑色訊息日誌區塊改為浮動彈現式對話框（Modal Overlay），日常維持精簡緊湊雙卡片佈局（高度 480px、無任何垂直捲軸）；按下「立即備份」或「還原到安裝根目錄」時才平滑彈出顯示進度與即時日誌，支援完成後點擊按鈕或 Esc 關閉，亦提供「📋 查看最新日誌」按鈕供隨時調閱。
- `uninstaller.py` 移除備份功能（`backup_critical_data`、備份進度視窗模式、「立即備份」按鈕、備份勾選框），改為提示使用者先使用 backuper；仍保留 `_LAH_Backup_*.zip` 於根目錄清理時不被刪除。
- `.gitignore` 新增 `backuper/src-tauri/target/`、`backuper/src-tauri/gen/`。

---

## [v1.6.8] - 2026-10-26

### 🛡️ 一鍵資安弱點防護與自動安裝核取方塊整合 PHP2Shell 緩解功能
- **一鍵資安防護按鈕與安裝核取方塊文字明確納入 PHP2Shell 緩解**：
  - 更新主畫面核取方塊標籤為「安裝/更新時自動套用資安防護 (含 PHP2Shell 緩解 / 隱藏版本資訊)」，讓管理師直觀掌握已包含防護。
  - 更新獨立按鈕文字為「🛡️ 一鍵套用資安防護與 PHP2Shell 緩解 (CVE-2024-4577 / 隱藏版本)」，點擊即一鍵完整套用並重啟相關服務。
- **全方位擴充 PHP2Shell (CVE-2024-4577) 與 WebShell 防護範圍**：
  - **Apache (`httpd.conf`)**：除既有 `www/assets`、`www/export` 禁執行腳本與 `%ad` 注入攔截外，新增保護 `www/import`、`www/uploads` 上傳目錄（`<FilesMatch "\.(php|php5|php7|php8|phtml|phar|cgi|pl|asp|aspx|jsp)$"> Require all denied </FilesMatch>`）並阻斷對 `www/log` 的直接存取，修復替換區塊正規表示式轉義問題。
  - **PHP (`php.ini`)**：新增清除潛在被惡意植入的 `auto_prepend_file` 與 `auto_append_file` 指令，杜絕 WebShell 透過全域前置載入掛馬。
  - 更新 PyInstaller 單一可執行檔 `dist\智慧控管與監控系統 - 自動更新工具.exe`。

---

## [v1.6.7] - 2026-10-25

### 📋 系統整合每週更新說明 (20261025_系統更新說明.txt) 支援與發布
- **全系統近 1 週更新歷程統整發布**：
  - 統整 2026-10-18 ～ 2026-10-25 全工作區 5 大子系統重大演進，產出專供更新工具內嵌預覽之 `20261025_系統更新說明.txt`。
  - 涵蓋重點：PHP2Shell (CVE-2024-4577) 緩解資安防護、PHP 8.5 雙軌 API (Port 8085) 4 大批次全模組移植與 72 業務單元測試、5 階段主機環境自動化配置與健康驗證管線、即時通訊桌面端遷移至 Tauri v2 (v1.6.0) 以及前端即時通輸入歷史記憶等。
- **更新工具內建說明文字載入對齊與視窗尺寸優化**：
  - 更新工具 `updater.py` 之 `find_release_notes` 自動辨識載入最新 `20261025_系統更新說明.txt`。
  - 同步更新 `.gitignore` 白名單規則，確保官方版本更新說明文字檔納入版本控制並同步至發布目錄。
  - 重構 `_show_release_notes_dialog` 彈出視窗排版幾何：依說明文字實際最長行動態量測最適寬度（預設由 `720x560` 擴增至 `1080x700`），並依螢幕大小自動安全調整與精準置中，徹底根絕長行分隔線與內文折行問題。

---

## [v1.6.6] - 2026-10-25

### 🛡️ PHP2Shell (CVE-2024-4577) 緩解設定自動套用與 5 大必備 PHP Extension 強制啟用驗證
- **Apache (`httpd.conf`) PHP2Shell 緩解與上傳目錄禁執行防護 (`apply_apache_security_hardening`)**：
  - 自動確保 `LoadModule rewrite_module modules/mod_rewrite.so` 啟用，並註解停用非必要的 `ScriptAlias /cgi-bin/`。
  - 自動冪等注入 CVE-2024-4577 (PHP-CGI 軟連字號 `%AD` 參數注入漏洞) 攔截規則（`RewriteCond %{QUERY_STRING} (?:^|&)%ad [NC,OR]` 及 `RewriteCond %{THE_REQUEST} \s/+(?:cgi-bin|php-cgi)/ [NC]` ➔ `RewriteRule ^ - [F,L]` 回傳 `HTTP 403 Forbidden`）。
  - 針對靜態與匯出目錄（`<root_dir>/www/assets`、`<root_dir>/www/export`）自動配置 `<FilesMatch "\.(php|php5|php7|php8|phtml|phar|cgi|pl|asp|aspx|jsp)$"> Require all denied </FilesMatch>`，防止上傳目錄遭植入並執行 WebShell。
- **PHP (`php7/php.ini` 與 `php85/php.ini`) PHP2Shell 安全強化 (`apply_php_security_hardening`)**：
  - 除既有 `expose_php = Off` 外，自動強制寫入 `allow_url_include = Off`、`cgi.force_redirect = 1`、`enable_dl = Off`（同時保留業務 Ping/Oracle CLI 所需之系統指令執行能力）。
- **5 大必備 PHP Extension 強制啟用與 `libsasl.dll` 依賴掛載 (`ensure_php_ini_configured` / `configure_apache_mod_php7`)**：
  - 確保 PHP 7.4 (`gd2`, `ldap`, `imap`, `pdo_sqlite`, `sqlite3`) 與 PHP 8.5 (`gd`, `ldap`, `imap`, `pdo_sqlite`, `sqlite3`) 於安裝及資安強化時自動解除註解並啟用。
  - 解決 `php7/ext/php_ldap.dll` 在 Apache `mod_php7` 模式下因缺少 `libsasl.dll` 預先載入而無法啟用的問題，於 `configure_apache_mod_php7` 自動加入 `LoadFile "<php7_dir>/libsasl.dll"`（共 8 個核心依賴 DLL）。
  - 強化 `restart_pm2_services` 與手動資安套用重啟邏輯，於重啟 `lah-api-v85` 前自動清理舊有 `php-cgi.exe` 子工作程序 (`PHP_FCGI_CHILDREN`)，確保最新 `php85/php.ini` 擴充與安全設定立即生效。
  - 於階段 5 (`verify_all_services_health`) 新增 PHP 7.4 與 PHP 8.5 必備擴充模組實際載入狀態 (`php.exe -m`) 最終驗證。

---

## [v1.6.5] - 2026-10-25

### 📦 移除工具 (`uninstaller.py`) 獨立備份按鈕與 `www/assets/*`、`.pm2` 備份範圍擴充
- **擴充系統重要資料備份範圍 (`www/assets/*` 與 `.pm2` 設定檔)**：
  - 於 `uninstaller.py` 的 `backup_critical_data` 新增備份 `<安裝根目錄>\www\assets` 下所有檔案（`www/assets/*`）以及 `<安裝根目錄>\.pm2` 內的設定與狀態檔（`dump.pm2`、`dump.pm2.bak`、`module_conf.json`、`conf.js`、`modules/`，排除 `.pm2/logs/` 日誌檔；若根目錄尚無 `.pm2` 則自動向下相容備份 `%USERPROFILE%\.pm2`）。
  - 同步強化 `updater.py` 的 `restore_from_backup_zip` 還原摘要與說明，支援一鍵完整還原 `www/assets/*` 與 `.pm2` 設定檔。
- **新增「📦 立即備份重要資料」獨立按鈕與自動取消勾選連動**：
  - 於 `uninstaller.py` 主視窗下方「🚨 開始執行移除作業」按鈕左側新增醒目深藍色「📦 立即備份重要資料」按鈕。
  - 點擊後彈出「另存新檔」對話框（預設檔名 `_LAH_Backup_YYYYMMDD_HHMMSS.zip`），讓使用者自由選擇儲存位置與自訂檔名，並透過獨立進度日誌視窗顯示打包進度。
  - 當手動備份順利完成後，系統**自動取消勾選**主畫面的「📦 執行前自動備份重要資料」核取方塊，避免後續執行移除時重複備份，同時於移除確認視窗與根目錄清理階段智慧識別並保護該手動備份檔。

---

## [v1.6.4] - 2026-10-25

### 🛠️ 綠色版 Node (PM2) 模組相容強化與跨權限 IPC / 離線安裝修復
- **徹底修復 `pm2-logrotate` 安裝失敗 (`ENOENT: mkdir ...`) 與離線秒級啟用**：
  - 根因：`PM2` 原廠 `Utility.js` (`getCanonicModuleName`) 未處理含磁碟機代號與反斜線之 Windows 本機路徑（`C:\AppServ\node\node_modules\pm2-logrotate`），導致將整串路徑拼入 `mkdir` 引發 `ENOENT`；且 `NPM.js` 預設呼叫外部 `npm`/`bun` 連網下載或全量複製數千個小檔案導致逾時。
  - 解法：修補綠色版 `node` 內之 `pm2/lib/Utility.js` 與 `pm2/lib/API/Modules/NPM.js`（並於 `updater.py` 內建 `ensure_portable_pm2_patched` 自動冪等修補），支援 Windows 路徑解析與輕量代理載入 `node\node_modules\pm2-logrotate`，100% 離線免外網且於數秒內完成註冊與啟動。
- **徹底解決一般 CMD 視窗執行 `pm2 ls` 出現 `connect EPERM \\.\pipe\rpc.sock` 錯誤**：
  - 根因：Windows Named Pipe (`\\.\pipe\rpc.sock`) 由系統服務 (`SYSTEM`) 或管理員權限 (`updater.exe`) 建立時，Windows 預設安全描述符 (DACL) 拒絕一般未提權 CMD 視窗寫入；且系統服務與一般使用者分屬不同 `.pm2` 目錄。
  - 解法：將綠色版 `pm2/paths.js`（含 `pm2-logrotate` 內嵌之 `pm2`）之 Windows 內部通訊改為本機 Loopback TCP 埠（`127.0.0.1:42681` RPC / `127.0.0.1:42682` PUB），並將預設 `PM2_HOME` 統一固定於 `<安裝根目錄>\.pm2`（如 `C:\AppServ\.pm2`），使系統服務 (`LAH_PM2_Service`)、管理員與任何一般 CMD 視窗無縫共用同一個 PM2 守護核心。
- **綠色版 Node 執行期路徑隔離**：
  - 於 `node\pm2.cmd` 內部自動將 `%dp0%` 置於子程序 `PATH` 最前端，確保 PM2 及其啟動之所有服務 100% 使用綠色版 Node，同時不干擾主機系統層級 (HKLM) 之其他 Node.js 環境。
- **重新打包 `dist\node.zip` 與內嵌 `WinSW` 服務包裝器**：
  - 將修補完成之 `pm2` 模組與 `service\lah-service.exe`、`service_runner.js` 同步打包更新至 `dist\node.zip`。

---

## [v1.6.3] - 2026-10-25

### 🚀 5 階段基礎環境自動化管線與「從 0 到 1 / 部分更新」雙向相容
- **設定檔升級保護機制（不覆寫既有 `httpd.conf` / `php.ini`）**：解壓縮 `Apache24`、`php7`、`php85` 官方壓縮包時，若目標路徑已存在 `conf/httpd.conf` 或 `php.ini`，自動將壓縮包內的原廠預設檔另存為 `.default`，完整保留既有設定與備份還原設定，再進行冪等注入。
- **[階段 1/5] 建立 Apache 服務並搭配 PHP 7.4 (`mod_php`)**：
  - 新增 `ensure_php_ini_configured` 自動由 `php.ini-production` 初始化 `php.ini`、校正 `extension_dir` 為正斜線絕對路徑、自動啟用 `ext/` 下的必備擴充模組（`curl, fileinfo, gd2, mbstring, openssl, pdo_sqlite, sqlite3`）與 `date.timezone = "Asia/Taipei"`。
  - 新增 `configure_apache_mod_php7` 校正 `Define SRVROOT`、`DocumentRoot "<root>/www"`、`DirectoryIndex index.php`，啟用 `headers_module` 與 `rewrite_module`，並自動掃描掛載 `php7` 的 7 個核心依賴 DLL（`LoadFile`）與 `LoadModule php7_module`。
  - 強化 `install_apache_service`：若系統既有 `Apache2.4` 服務指向舊路徑，自動停止並重新對齊註冊至當前目標 `httpd.exe`。
- **[階段 2/5] 增加 FCGI 的 API Proxy 設定（以 PM2 的 `lah-api-v85` 保護執行）**：
  - 自動初始化 `php85/php.ini` 並啟用 PHP 8.5 擴充模組（含 `odbc, pdo_odbc, intl, sockets, zip` 等）及調優 `memory_limit = 512M`、`max_execution_time = 300`。
  - 新增 `configure_apache_fcgi_proxy` 與 `ensure_lah_api_ecosystem_config`：啟用 `proxy_module`、`proxy_fcgi_module`，注入 `/LAH-API` 與 `/LAH-API/api` Alias、機敏副檔名與核心子目錄阻擋防護、以及 `fcgi://127.0.0.1:8085/` 轉發池，並確保 `LAH-API/ecosystem.config.js`（由 PM2 `lah-api-v85` 守護 `php-cgi.exe -b 127.0.0.1:8085`）到位。
- **[階段 3/5] 套用資安弱點防護並執行語法檢驗**：
  - 自動向 Apache 寫入 `ServerTokens Prod`、`ServerSignature Off`，向 PHP 7.4 與 8.5 寫入 `expose_php = Off`（不吐伺服器或 PHP 版本資訊），並執行 `httpd.exe -t` 確認 `Syntax OK`。
- **[階段 4/5] 綠色版 Node 設定與 PM2 自動化啟動**：
  - 自動註冊 `<root_dir>\node` 與 `instantclient_11_2` 至 PATH 最前端、將 PM2 Daemon 對齊綁定至本機綠色版 `node.exe`、智慧判斷重啟或補齊受管服務，並確保 `LAH_PM2_Service` 開機自動啟動服務就緒。
- **[階段 5/5] 全系統服務啟動與連接埠最終健康驗證 (`verify_all_services_health`)**：
  - 啟動或重啟 Apache 服務後，自動對 `Apache2.4` (`Port 80`)、`lah-api-v85` (`Port 8085`)、`LAH-FE` (`Port 8080`)、`LAH-WSS` (`Port 8081/8082`) 進行服務與 TCP Port 監聽檢查；若有未就緒項目自動重試拉起一次，並於日誌與完成對話框列出完整驗證報告。

---

## [v1.6.2] - 2026-10-25

### 🎨 左右雙欄版面重構與全域操作列左移平衡
- **左右雙欄底部齊平平衡**：將原本橫跨主視窗最下方的全域操作控制列移入左欄（`left_col`）最下方，採雙列平衡排版：
  - **上列（輔助操作）**：配置「👁️ 顯示全部安裝包 (含未偵測到)」核取方塊（靠左）以及「📋 查看更新說明」與「📊 檢視執行日誌」按鈕（靠右）。
  - **下列（主行動列）**：配置「關閉離開」按鈕（靠左）與深藍色醒目主按鈕「🚀 開始更新」（靠右）。
  - 垂直高度與右欄底部的 PM2 控制項（雙列約 72px）精準齊平對稱，達成左右兩欄 1:1 視覺平衡。
- **Apache 與資安選項上移**：將原本位於左欄底部的「🛡️ Apache 服務與資安防護設定」區塊（含重啟 Apache、重新掃描、自動註冊服務、自動套用資安以及一鍵資安按鈕）上移，緊接在「PHP 8.5 執行環境」卡片正下方，資訊流程更直覺自然。

### 🐛 徹底修復 Node.js 綠色執行環境選項垂直擠壓缺陷
- **空間釋放與幾何微調**：全域底部按鈕列移入左欄後，中央卡片區塊（`pkg_frame`）獲得充裕的垂直可用高度。主視窗尺寸微調為 `1260x760`（最小視窗 `1100x740`）。
- **右欄 7 大模組高度整齊劃一**：右側 7 個模組卡片（含最後一個 Node.js 綠色執行環境）高度全部整齊劃一（`card_height=56`, `entry_height=24`），路徑輸入框、瀏覽按鈕與重設按鈕完全正常顯示，徹底根除 Entry 與按鈕被壓縮成細線的問題。

---

## [v1.6.1] - 2026-10-25

### 📦 備份設定與資料還原支援（`_LAH_Backup_*.zip`）
- **自動偵測與一鍵還原備份包**：於主視窗頂部「📂 安裝根目錄與備份設定還原」區塊新增備份還原選項與獨立按鈕。啟動時自動於 `C:\Temp`、程式目錄與桌面搜尋最新的 `_LAH_Backup_*.zip` 備份檔並帶入路徑。
- **雙模式還原流程**：支援勾選「📦 安裝/更新後自動從備份包回復原始設定」於全新安裝時自動套用，亦支援點擊「⚡ 立即單獨還原備份」隨時獨立還原 `.env`、`AD.php`、即時通資料庫 `lah-messenger-server/db`、附件 `uploads`、`httpd.conf` 與 `php.ini`，還原後自動校正路徑與資安防護。

### 🚀 全新安裝與更新收尾自動化完善
- **Apache 雙層巢狀目錄防呆與自動攤平**：強化 `detect_strip_prefix` 支援識別含外層散落說明檔（如 `ReadMe.txt`、`-- Win64 VS18 --`）的 Apache Lounge 官方壓縮包並自動剝除 `Apache24/` 外層目錄。同時新增 `flatten_nested_apache_dir` 自動偵測並攤平既有的 `Apache24\Apache24` 雙層目錄。
- **全流程自動化收尾**：安裝或更新完成後，自動產生或校正 `httpd.conf` 與 `php7/php.ini`、`php85/php.ini` 並套用資安弱點防護、自動註冊並立即啟動 `Apache2.4` Windows 服務、自動將本機綠色版 Node（`<root_dir>\node\node.exe`）註冊至 PATH 最前端並綁定 PM2 核心守護進程、拉起 5 大核心服務，最後自動配置 `LAH_PM2_Service` 原生系統服務與開機自動化。

### 🐛 錯誤修復與壓縮包清理體驗優化
- **修復更新完成後卡在「更新進行中，請稍候...」無法關閉視窗之問題**：修正 `_run_update_process` 的 `finally` 區塊中未定義變數 `should_restart_apache` 導致的 `NameError`，確保 `_on_update_completed_ui` 100% 執行，完成後進度視窗按鈕立即切換為可用的「✅ 完成並關閉 (Esc)」。
- **壓縮包清理詢問（預設保留）與同模組多別名清理**：更新完成後彈出詢問視窗（預設焦點為「否」保留壓縮包），僅在使用者明確點選「是」時才執行刪除。同時將同模組匹配到的所有別名壓縮檔（如 `LAH-NUXTJS-node_modules.zip` 與 `LAH-FE-node_modules.zip`）一併納入 `all_filepaths`，徹底解決別名壓縮包殘留問題。

---

## [v1.6.0] - 2026-10-25

### 🌐 Web 基礎環境與伺服器安裝擴充
- **三核心環境 ZIP 包整合**：新增 Apache 2.4、PHP 7.4 以及 PHP 8.5 的 x64 Windows 壓縮包支援。維運管理師可自官方下載最新壓縮包直接放入程式目錄，免手動繁瑣解壓。
- **智慧模糊與萬用字元辨識**：支援官方下載原始檔名（例如 `httpd-2.4.*.zip`、`php-7.4.*.zip`、`php-8.*.zip`）與標準簡稱（例如 `Apache24.zip`、`php7.zip`、`php85.zip`）。程式內部自動探測並對齊外層目錄，避免產生巢狀雙層資料夾。
- **左右雙欄卡片式視覺佈局**：主視窗幾何尺寸調整為 `1260x720`（最小視窗尺寸設定為 `1100x620`）。左欄專責「基礎環境與 Web 伺服器」，右欄專責「智慧控管系統模組」，兩欄皆提供即時偵測標籤、目標路徑自訂輸入框、瀏覽按鈕與重設按鈕。

### 🛡️ 資安弱點防護（隱藏 Web 伺服器與 PHP 版本資訊）
- **一鍵獨立強化按鈕**：左欄新增「🛡️ 一鍵套用資安弱點防護 (隱藏 Apache / PHP 版本資訊)」專屬功能按鈕。平時無須執行完整系統更新，隨時點擊即可針對既有環境之設定檔進行一鍵資安強化。
- **Apache 伺服器標頭與錯誤頁防護**：向 `Apache24/conf/httpd.conf` 自動寫入並確保 `ServerTokens Prod`（HTTP Server 回應標頭僅回傳 `Server: Apache`，消除詳細版本號與模組清單）與 `ServerSignature Off`（關閉伺服器產生的 404 或 500 等錯誤頁面底部版本簽名）。
- **PHP 語言版本標頭防護**：向 `php7/php.ini` 與 `php85/php.ini` 自動寫入並確保 `expose_php = Off`，徹底移除 HTTP 回應標頭中的 `X-Powered-By: PHP/...` 版本資訊洩漏。
- **安全備份與即時語法檢驗**：寫入前自動建立帶有時間戳記的備份檔（`.bak_YYYYMMDD_HHMMSS`）。修改後自動調用 `httpd.exe -t` 檢驗語法，若 Apache 正在執行則主動跳出確認視窗協助重啟服務以立即生效。

### ⚙️ 智慧相容配置與服務有序管線
- **升級保留與路徑校正**：若目標目錄已有現存 `httpd.conf` 或 `php.ini`，解壓前自動備份，優先保留使用者既有設定並僅確保 `Define SRVROOT` 正斜線路徑對齊。
- **初次安裝自動產生標準範本**：若為初次安裝，以專案標準範本（`httpd.conf-ref`）為基底自動產生設定檔，自動連結 PHP 7 模組（`LoadModule php7_module`）與 PHP 8.5 FastCGI 轉發規則（`/LAH-API` 轉發至 `127.0.0.1:8085`）。
- **Windows 服務自動掛載**：提供「若未安裝 Apache 服務則自動註冊 (Apache2.4)」核取選項，解壓完成後自動調用 `httpd.exe -k install -n Apache2.4` 完成系統服務註冊。
- **有序防鎖定管線**：執行更新時優先停止服務避免檔案鎖定，依序先解壓配置左側基礎環境，再解壓更新右側應用模組，最後自動重啟 Apache 與還原 PM2 服務。

### 🎨 雙欄底部控制列佈局重構與文字截斷修復
- **Apache 與 Web 伺服器控制項歸位**：將 Apache 服務自動重啟選項與「🔄 重新掃描」按鈕歸併至左側「基礎環境與 Web 伺服器」欄位底部，消除與右側 PM2 控制項擠壓所產生的水平溢位。
- **PM2 控制列雙列平衡化**：右欄底部重整為乾淨雙列格局。第 1 列配置 PM2 自動重啟核取方塊與「⚙️ 開機自啟動設定」按鈕，第 2 列配置 PM2 守護核心即時綁定狀態標籤與「⚡ 立即校正 PM2 核心」按鈕，徹底解決核取框文字被擠壓為單點以及按鈕被推擠至視窗外之顯示問題。
- **視窗尺寸與留白微調**：主視窗幾何尺寸調整為 `1260x740`（最小視窗設定為 `1100x720`），微調頂部標題、目錄輸入框與卡片間距，確保在不同螢幕縮放下所有文字與按鈕皆能 100% 完整呈現。

### 👁️ 未偵測到安裝包預設隱藏與動態展開支援
- **智慧過濾預設隱藏**：啟動與重新掃描時，若更新目錄下未偵測到對應的 ZIP 壓縮包，預設自動隱藏該模組選項卡片，使介面視覺聚焦於本次實際需要更新的項目。
- **一鍵展開全部項目選項**：主畫面底部左側新增「👁️ 顯示全部安裝包 (含未偵測到項目)」獨立核取方塊，勾選時立即展開所有 10 種安裝包卡片供管理師查閱與路徑自訂。
- **空欄位友善提示橫幅**：當左欄（基礎環境）或右欄（系統模組）在當前目錄未偵測到任何安裝包時，自動顯示提示文字引導使用者，消除空白欄位困惑。

---

## [v1.5.0] - 2026-10-25

### 🗑️ 自動移除工具體驗優化 (uninstaller.py)
- **介面尺寸寬度放寬**：主視窗預設尺寸調整為 `1180x530`（最小視窗設定為 `1050x480`），兩欄等寬權重配置提供充足邊距，徹底解決各類螢幕 DPI 縮放下的長文字截斷問題。
- **抽離常駐進度為彈出式對話框 (`ProgressDialog`)**：主視窗移除常駐之進度條與日誌區塊，點選開始後彈出專屬獨立視窗（`980x620`），內嵌彩色 ScrolledText 終端日誌、進度條與完成關閉控制。
- **單次完整確認防護對話框**：點選開始移除時，彈出一次性詳細確認視窗，完整列出目標根目錄、備份狀態與所有勾選之預計移除項目，強化維運防呆保護。

---

## [v1.4.0] - 2026-10-25

### 🛑 專屬自動移除工具全新發布 (uninstaller.py)
- **全方位系統清理**：一鍵停止並解除安裝 Windows 服務（`LAH_PM2_Service`、`Apache2.4`），終止殘留進程（`pm2 kill`、`node.exe`、`php-cgi`）。
- **開機自啟動與環境變數清除**：清理 Windows 啟動資料夾 VBS 腳本、工作排程器任務（`LAH_Startup_PM2`），並自使用者環境變數 `PATH` 移除 Node、Instant Client、PHP 等路徑。
- **手動 PHP 目錄清除**：支援一併清理手動安裝之 `php7` 與 `php85` 目錄，若保留 Apache 則自動清理 `httpd.conf` 中的 PHP 載入規則與 FastCGI 轉發區塊。
- **刪除前關鍵資料自動備份**：支援執行前自動打包備份關鍵持久化資料（即時通 SQLite `db/`、附件 `uploads/`、環境變數 `.env`、LDAP 設定 `AD.php`、`php.ini` 與 `httpd.conf`），存放於安裝根目錄。
- **安裝根目錄殘留檔案深度清理**：安全清理安裝根目錄其餘殘留檔案（如 `.zip`、`.bat`、`.cmd`、`.pm2` 快照），若全數清空則安全移除資料夾，若有備份檔則完整保留備份壓縮檔。
- **WinSW 原生服務守護套件整合**：於 `service/` 目錄內建 WinSW x64 原生服務二進位檔與 XML 配置檔，支援將 PM2 與 Node.js 封裝為標準 Windows 系統服務（`LAH_PM2_Service`）。

---

## [v1.3.0] - 2026-10-24

### 📦 系統一鍵打包腳本發布 (pack.py / pack.bat)
- **全模組交互式選單**：支援一鍵打包 `www.zip` (舊版 PHP7)、`LAH-API.zip` (現代化 PHP8)、`LAH-NUXTJS.zip` (前端網站)、`LAH-NUXTJS-node_modules.zip` (前端核心套件)、`lah-messenger-server.zip` (即時通伺服端)、`lah-messenger-server-node_modules.zip` (即時通核心套件) 與 `node.zip` (Node 綠色環境)。
- **排除不必要檔案**：打包時自動排除 `.git`、`.env`、`tests/`、單元測試檔案、本機資料庫與暫存檔案，壓縮產物直接輸出至 `dist/`。
- **雙軌後端 API 支援**：全面對齊 `LAH-API` (PHP 8.5) 現代化架構，將預設 FastCGI 監聽埠更新為 `8085`。
- **pm2-logrotate 模組巡檢與啟動**：於綠色 Node 環境內建 `pm2-logrotate` 日誌輪替模組，支援離線自動回退機制。

---

## [v1.2.0] - 2026-10-23

### 🟩 Node.js 綠色環境與 PM2 服務管理升級
- **Node.js 綠色便攜環境支援 (`node.zip`)**：支援將完整綠色 Node.js (含 npm, pm2, yarn) 解壓部署至目標目錄。
- **實時核心進程探測**：主視窗實時探測 PM2 God Daemon 是由哪一個 `node.exe` 喚醒運行，防止核心進程錯置。
- **一鍵無痛核心校正按鈕**：提供「⚡ 立即校正 PM2 核心」按鈕，執行 `pm2 save` 備份、`pm2 kill` 終止舊進程並調用綠色 Node 執行 `pm2 resurrect` 完整還原服務。
- **自動註冊 Node 環境變數 (User PATH)**：更新 `node.zip` 或校正時，自動透過 `winreg` 將綠色 Node 目錄寫入 Windows 使用者 `PATH` 最前端並廣播變更。
- **更新安全停機防護**：解壓 `node.zip` 或 `node_modules.zip` 前自動偵測 `node.exe` 檔案鎖定，必要時安全暫停 PM2 守護進程，解壓後自動復原。

---

## [v1.1.0] - 2026-10-18

### 📋 介面優化與大尺寸日誌彈窗
- **版本更新說明彈窗**：主畫面新增「📋 查看更新說明」按鈕，自動讀取同目錄下檔名包含「更新說明」的 `.txt` 檔案並彈出置中對話框瀏覽，支援 Esc 關閉。
- **獨立大尺寸日誌彈窗**：點選更新時彈出專屬進度視窗（`860x600`），內建深色終端風格彩色執行日誌與平滑進度條。
- **全介面文字等比例放大 25%**：縮放 Tkinter 內建 Named Fonts，在高解析度螢幕下視覺舒適好讀。

---

## [v1.0.0] - 2026-10-18

### 🚀 初版發布 (updater.py)
- **基礎解壓縮更新機制**：支援後端、前端與即時通更新包之自動偵測與解壓縮覆蓋。
- **關鍵防呆機制（自動去除雙層同名目錄）**：解壓時先解析 `.zip` 內部結構，自動對齊並剔除頂層同名目錄，徹底避免產生雙層資料夾錯誤。
- **Apache 服務管理與異常容錯**：解壓前自動停止 Apache 服務釋放鎖定，更新後自動重新啟動；更新中途若遇異常，`finally` 區塊強制重新啟動服務以維持網站運作。
- **更新完成空間清理確認**：更新完成後主動提示是否刪除已更新完成之 ZIP 壓縮包以節省磁碟空間。
- **單一執行檔打包**：基於 Python 標準庫開發，透過 PyInstaller 支援打包為單一無黑視窗的 Windows 執行檔 (`.exe`)，內嵌 `--uac-admin` 管理員權限宣告。
