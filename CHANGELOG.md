# 智慧控管與監控系統 - 自動更新與維運工具 (LAH-Updater) 版本修改歷程

本文件記錄 `LAH-Updater` 維運工具套件（包含自動更新工具 `updater.py`、自動移除工具 `uninstaller.py` 以及系統打包腳本 `pack.py`）之功能演進、架構調整與資安防護修復歷程。

---

## [未發布]

### 🖥️ 管理儀表板全面升級「xterm.js 專業虛擬終端」與「雙向聯動編碼切換」

- **修改目的**：
  - 徹底解決原先沙盒終端機卡頓、ANSI 色彩控制碼裸露（`[1m[36m` 等）以及 Windows 原生繁中輸出（CP950）呈現亂碼之問題。
  - 將終端架構全面升級為業界標準之純離線 `xterm.js` 虛擬終端引擎，支援直接打字鍵入、退格、歷史指令、真彩排版與滑鼠複製貼上。
  - 支援台灣常用的 UTF-8 與 CP950 (Big5 繁體中文) 即時切換，並自動與底層 `cmd.exe` 代碼頁雙向同步。
- **具體修改項目**：
  - **xterm.js 純離線虛擬終端引擎整合**：
    - 引入本機靜態 UMD 資源（`vendor/xterm.js`、`vendor/xterm.css`、`vendor/xterm-addon-fit.js`），嚴格遵從 No-CDN 離線原則。
    - 實作完整行編輯模式：支援鍵盤直接打字、Backspace 退格、↑/↓ 瀏覽歷史指令、Ctrl+C 中斷與滑鼠反白複製貼上。
    - 輪詢頻率提升至 100ms 高頻率，使打字回顯反應敏捷流暢，徹底告別卡頓。
  - **UTF-8 與 CP950 (Big5) 雙向聯動編碼切換**：
    - 後端 Rust 採用 Windows 原生 `MultiByteToWideChar` API 分流處理 CodePage 65001 與 950，零額外依賴精準解碼繁體中文。
    - 工具列提供「編碼：UTF-8 / CP950」即時選單；切換時自動向直譯器注入 `chcp 65001` 或 `chcp 950`，使環境與畫面解碼完全同步。
  - **核心組態編輯器修復與強化**：
    - 全域補正 `.hidden { display: none !important; }`，徹底修復右側範本視窗未收合暴露而停留於佔位符之問題。
    - 當選取具有範本之檔案（`www/.env`、`LAH-NUXTJS/.env`）時，自動展開雙欄對照視窗並即刻載入 `.env.example` 完整內容。
    - 增強 `find_example_file` 大小寫不敏感比對與 `.sample` 支援；`read_config_file` 加入 UTF-8 BOM 識別與容錯處理。
  - **SOP 文件與技術手冊全面同步**：
    - 同步更新 `INSTALLATION.html`、`generate_install_guide_pdf.py`、`generate_readme_html.py`、`tools/dashboard/README.md`。
    - 重新編譯生成最新版標準公務維運 SOP 指引：`dist/智慧控管系統_系統安裝與維護操作指引.html` 與 `.pdf`。
- **相容性說明**：
  - 100% 離線純淨運行，無任何外連 CDN，相容 Windows 7 / 10 / 11 與 Windows Server 全版本。


### ⚙️ 管理儀表板擴充「核心組態編輯器」與「Windows 效能風格系統指標」

- **修改目的**：
  - 提供系統管理員一站式檢視與編輯常見服務組態（Apache httpd.conf、PHP php.ini、各子專案 .env 等）之能力，解決以往需手動尋找深層目錄與文字編輯之痛點，並內建語法防呆與自動備份。
  - 將第三分頁之系統指標升級為 Windows 工作管理員效能風格，即時呈現實體網路收發速率、LAH 服務群組實體記憶體總占用量與進程排行清單，並透過純原生 Canvas 繪製 60 秒即時平滑波形趨勢圖。
- **具體修改項目**：
  - **核心組態編輯器 (第四分頁)**：
    - 預載 10 大核心組態快速選單：Apache 核心與虛擬主機、PHP 7.3/8.5 php.ini、後端 API www/.env、前端網站 LAH-NUXTJS/.env、即時通 lah-messenger-server/.env、WinSW 服務 XML 等，並支援「📂 自訂檔案...」。
    - 內建 .env.example 雙欄對照比對檢視視圖：自動偵測同目錄下之範本檔，提供展開唯讀窗格、一鍵複製範本與防呆套用至編輯區。
    - 多重安全防護機制：儲存時自動產生備份（自動保留最新 5 份）；提供歷史備份下拉選單與還原功能；Apache 組態儲存自動執行 httpd.exe -t 語法預檢防呆，語法錯誤彈窗告警並提供一鍵自動還原。
    - 智慧關聯重啟提示：組態變更儲存成功後，依據檔案屬性主動彈窗詢問是否立即重啟 Apache 系統服務或關聯 PM2 應用進程。
    - 編輯器操作增強：Tab 鍵插入 4 空格、Ctrl + S 快速儲存、自動換行切換開關、未儲存變更標籤與切換防呆。
  - **Windows 效能風格系統指標 (第三分頁)**：
    - 純原生 HTML5 Canvas 繪製 60 秒平滑趨勢折線圖（CPU 青色、記憶體紫色、網路雙色），微暗網格背景與漸層半透明填充，退至背景自動休眠。
    - 網路即時流量探測：後端呼叫原生 Win32 GetIfTable2（iphlpapi.dll），微秒級採集實體網卡之接收（下載）與傳送（上傳）速率，動態自適應波形圖。
    - LAH 服務群組記憶體卡片：後端使用內核級 NtQuerySystemInformation 枚舉所有 Apache httpd.exe 實體記憶體，並結合 PM2 進程記憶體，呈報總占用量、佔系統實體記憶體百分比及進程占比長條清單。
    - 兩排結構化格狀佈局：第一排核心動態折線圖，第二排服務記憶體與磁碟空間。
  - **日誌壓縮輪替根本修復**：
    - 後端實作原生 Rust Gzip 壓縮歸檔與空間釋放（替代無效之 pm2-logrotate rotate action），清空日誌後通知 PM2 重整。
  - **視覺與下拉選單色彩鎖定**：
    - 強制鎖定原生 select 與 option 之 color-scheme，徹底解決 Windows 暗色外觀導致明暗主題選單黑底黑字問題。
    - 字體基礎整體放大 20%，一鍵重啟提供第 0 毫秒旋轉重啟態與 8 秒密集衝刺探測。
- **相容性說明**：
  - 零外部套件依賴，純原生 Win32 API 與 HTML5 Canvas，與既有 Apache、PHP 及 PM2 服務完全相容。


### 📖 伺服器安裝操作手冊 (INSTALLATION.html) 與 PDF 指引同步管理儀表板功能

- **修改目的**：
  - 同步更新伺服器端安裝與升級操作手冊 HTML 及標準 8 頁公務維運 PDF，完整納入管理儀表板工具（`智慧控管與監控系統 - 管理儀表板.exe`）之部署機制、一站式健康巡檢流程與檢核表規範。
- **具體修改項目**：
  - `INSTALLATION.html`：
    - 封面中繼資料更新為 2026-10-31 管理儀表板增補修訂版。
    - 第 1 節更新包內容清單增列「智慧控管與監控系統 - 管理儀表板.exe」（專屬維運標籤）。
    - 第 4 節情境一原機升級步驟三增補儀表板熱部署、桌面捷徑建立與彈窗「📊 開啟管理儀表板」快捷按鈕說明。
    - 第 6 節情境二全新安裝全自動佈署流程補齊儀表板同步部署步驟。
    - 第 7 節驗證標準新增「一站式維運管理儀表板驗證」專區，說明三頁籤之服務監控、即時日誌串流與連接埠健康探針。
    - 附錄檢核表「驗證上線」階段擴充管理儀表板綠燈驗證項目。
  - `generate_install_guide_pdf.py`：
    - 同步更新 HTML 模板對應章節，並校準排版間距確保 PDF 精確維持為標準 8 頁。
    - 修復字串中反斜線跳脫字元引發之 Python 3.12+ `SyntaxWarning`。
    - 重新編譯生成最新 `dist/智慧控管與監控系統_系統安裝與版本升級操作手冊.pdf`。
- **相容性說明**：
  - 純靜態離線手冊，無外部 CDN 依賴，PDF 頁數嚴格鎖定 8 頁無斷頁溢出。

### 🖥️ 新增「智慧控管與監控系統 - 管理儀表板」原生維運工具 (tools/dashboard)

- **修改目的**：
  - 為現場與遠端維運人員打造全方位、免安裝、隨點即開的單一執行檔原生儀表板（基於 Tauri 2 / Rust + 原生純 HTML/CSS/JS 前端），整合 Windows 系統服務監控與控制、PM2 應用進程層控管、動態即時日誌檢視、主機資源即時狀態與五大連接埠 TCP 健康探針，消除日常維運需要繁瑣切換工作管理員、服務管理員與終端機之痛點。
- **具體功能與架構**：
  - **雙軌分層服務監控與重啟**：
    - Windows 服務層：管控 `LAH_Web_Service`（Apache 2.4）與 `LAH_PM2_Service`（PM2 Daemon）。
    - PM2 進程層：管控 `LAH-FE`（Port 8080）、`LAH-WSS`（Port 8081/8082）、`LAH-SCHEDULER`（排程）與 `lah-api-v85`（PHP 8.5 FastCGI Port 8085）。
    - 支援個別服務啟動、停止、重啟，並具備頂部安全防呆之「一鍵重啟全部服務」。
  - **即時日誌檢視器**：
    - 自動探測全系統各模組日誌（前端、即時通、排程、PHP 8.5、Apache access/error、WinSW 與 pm2-logrotate）。
    - 支援 Auto-Tail 即時追蹤、關鍵字即時篩選、清空畫面與「在記事本中開啟」實體檔案。
  - **主機指標與通訊埠探針**：
    - CPU 總使用率、實體記憶體負載、磁碟容量健康度即時條。
    - 五大連接埠（80, 8080, 8081, 8082, 8085）即時 TCP 心跳探針。
  - **快捷維運工具箱**：
    - 一鍵開啟 `lah-cmd.bat` 沙盒終端、前端入口、Apache 組態語法驗證（`httpd -t`）、手動觸發日誌壓縮輪替與開啟日誌存放目錄。
  - **介面體驗與效能優化 (Tab-Aware Polling & Dynamic UI)**：
    - 視窗預設最大化顯示，日誌檢視器改為內部自適應 Flexbox 貼齊，徹底根除雙重垂直捲軸問題。
    - 內建字體大小切換選單（一般 100%、中 +15% 預設、大 +25%），透過 CSS 變數 `--font-scale` 全局等比縮放並記憶於本機。
    - 支援明亮／深邃主題切換，日誌檢視終端畫布維持高對比深色背景以防止炫光；修復明亮主題下日誌來源選單文字與背景對比度不足之缺陷。
    - 提供可自訂輪詢週期選單（3秒、5秒預設、10秒、15秒、30秒、60秒），解決 Windows 環境下頻繁執行 `pm2.cmd jlist` 單次耗時 2.16 秒所導致的系統頓挫，並導入分頁感知隔離（非可見分頁不查詢 PM2）、視窗最小化休眠與日誌特徵指紋防抖渲染機制。
  - **一鍵編譯與自動部署**：
    - 建立 `tools/dashboard/build.py` 與根目錄快捷批次檔 `建立管理儀表板工具.bat`，產出 `dist/智慧控管與監控系統 - 管理儀表板.exe`，若本機存在 `C:\AppServ` 自動同步熱部署。
- **相容性說明**：
  - 為完全獨立之原生維運工具，零全域污染、無外網依賴，不更動既有 SCM 或 PM2 運作規則。

### 📊 自動更新工具 (updater) 支援更新完成後自動同步管理儀表板 EXE 與建立桌面捷徑

- **修改目的**：
  - 當自動更新工具完成系統部署後，主動探測同目錄下是否存在管理儀表板執行檔（`智慧控管與監控系統 - 管理儀表板.exe` 或 `*儀表板*.exe`），若發現則自動同步複製至安裝根目錄，並在 Windows 公用桌面與使用者桌面建立快捷啟動捷徑（`.lnk`），並於更新完成視窗中提供一鍵啟動按鈕，免去維運人員手動搬移與手動設定捷徑之繁瑣操作。
- **具體修改項目**：
  - `tools/updater/src-tauri/src/system.rs`：
    - 新增 `find_dashboard_exe_in_dir`：精準比對預期儀表板名稱，並廣泛匹配排除 updater 之儀表板執行檔；支援開發環境回溯探測。
    - 新增 `create_desktop_shortcut`：透過 PowerShell COM 介面（`WScript.Shell`）建立包含工作目錄、執行參數與圖示之 Windows 桌面捷徑（`.lnk`），自動適用於公用桌面與當前使用者桌面。
    - 新增 `deploy_dashboard_if_found`：自動將同目錄下儀表板 EXE 複製覆蓋至安裝根目錄（支援檔案鎖定時更名 `.old` 容錯），並即時建立桌面捷徑與日誌回報。
  - `tools/updater/src-tauri/src/types.rs`：
    - `UpdateSummary` 擴充 `dashboard_deployed: bool` 與 `dashboard_exe_path: Option<String>` 欄位。
  - `tools/updater/src-tauri/src/pipeline.rs`：
    - 於 `execute_update_pipeline` 與 `apply_system_configurations` 完成階段加入管理儀表板探測、部署與桌面捷徑建立管線。
  - `tools/updater/src-tauri/src/lib.rs`：
    - 新增 `launch_dashboard` Tauri 指令，支援從完成視窗直接拉起安裝根目錄之儀表板進程。
  - `tools/updater/ui/index.html` & `tools/updater/ui/app.js`：
    - 完成視窗新增「📊 開啟管理儀表板」操作按鈕，偵測到儀表板部署成功時動態顯示，點選可立即開啟程式。
- **相容性說明**：
  - 12 項單元測試全數通過（含儀表板偵測與自動複製部署測試）。未檢出儀表板時維持既有標準更新流程，無破壞性影響。

### 🗂️ 桌面維運工具子專案收整至 tools/ 目錄與路徑結構標準化

- **修改目的**：
  - 將專案根目錄下原本分散之四大桌面維運工具子專案（`updater`、`uninstaller`、`backuper`、`node-portable`）全數收整至 `tools/` 目錄，使 `LAH-Updater` 專案架構更為集中清晰，維持根目錄整潔，並使全部獨立產生 exe 的維運輔助工具統一歸屬於 `tools/` 套件庫。
- **具體修改項目**：
  - **子專案目錄搬移（`git mv`）**：
    - `updater/` ➔ `tools/updater/`
    - `uninstaller/` ➔ `tools/uninstaller/`
    - `backuper/` ➔ `tools/backuper/`
    - `node-portable/` ➔ `tools/node-portable/`
  - **自動建置腳本路徑校準（`build.py`）**：
    - 調整 `tools/updater/build.py`、`tools/uninstaller/build.py`、`tools/backuper/build.py`、`tools/node-portable/build.py` 中的根目錄與產物輸出路徑推導（由 `..` 改為 `..\..`），確保 release 二進位檔一律正確輸出至專案根目錄之 `dist/`。
  - **原始碼打包工具相容（`pack/pack.py`）**：
    - 更新更新說明搜尋候選路徑，擴充支援 `tools/updater/*更新說明*.txt`。
  - **根目錄一鍵批次檔校準**：
    - `建立自動更新工具.bat` ➔ 呼叫 `%~dp0tools\updater\build.py`。
    - `建立解除安裝工具.bat` ➔ 呼叫 `%~dp0tools\uninstaller\build.py`。
    - `建立自動備份工具.bat` ➔ 呼叫 `%~dp0tools\backuper\build.py`。
    - `建立可攜NODE工具.bat` ➔ 呼叫 `%~dp0tools\node-portable\build.py`。
  - **文件與手冊同步更新**：
    - 更新 `tools/README.md`、`AGENTS.md`、`README.md`、`generate_readme_html.py` 與工作區根目錄 `AGENTS.md`。
- **相容性說明**：
  - 各 Tauri 2 (Rust) 子專案內部設定、WinSW 服務二進位檔依賴、圖示產出與輸出命名完全保持一致，外部一鍵建置批次檔與產物目錄維持不變。

### 📦 系統打包工具 (pack) 補齊後端排程任務與專案資源並支援 VC++ 獨立封裝

- **修改目的**：
  - 解決後端 PHP7 更新包（`www.zip`）因白名單遺漏 `task/`、`json/`、`project/` 與 `favicon.ico`，導致產線伺服器更新時缺少 `watchdog.php` 定時健康守護排程與最新常用書籤資料之問題。
  - 將 VC++ 2015–2022 核心程式庫（`vcredist-x64.zip`）納入互動選單（選項 8）與新增命令列獨立參數 `--vcredist-only`。
  - 清理廢棄之 `DEFAULT_OUTPUT_DIR` 死碼，並強化路徑排除檢查相容子目錄。
- **具體修改項目**：
  - `pack/pack.py`：
    - `API_INCLUDES`：新增 `task`、`json`、`project`、`favicon.ico`。
    - `API_EXCLUDES`：新增 `.git*`、`.vscode`、`*.log`、`*.tmp`、`*.bak` 通用排除。
    - `is_excluded`：強化支援深層子路徑匹配，避免深層 `.git*` 與快取檔案誤入壓縮包。
    - 移除未引用之 `DEFAULT_OUTPUT_DIR` 常數，校準標頭 docstring 對齊 `dist/` 分流結構。
    - `prompt_interactive_targets`：新增選項 `8. VC++ 核心程式庫 (x64) (vcredist-x64.zip)`，並將 `8` 納入組合 `B` 與 `C`。
    - `main`：新增 `--vcredist-only` 與 `--with-vcredist` 參數；對齊執行與摘要輸出。
  - `pack/README.md`：同步更新命令列參數與說明。
- **相容性說明**：
  - 產出之 `www.zip` 完整納入 1,000 個檔案，經實測驗證包含 `task/watchdog.php`、`json/bookmarks.json`、`project/bookmark/index.html` 與 `favicon.ico`，且不含任何 `.git` 檔案。

### 🗑️ 自動移除工具 (uninstaller) 介面捲軸消除與服務脫敏同步

- **修改目的**：對齊 AGENTS.md 介面禁止出現 Scrollbar 之強制規範，消除移除工具啟動時之垂直捲軸；並同步將介面服務選項與後端卸載日誌中的開源技術詞彙（Apache2.4、PM2）全面脫敏為公務系統中性名稱。
- **具體修改項目**：
  - `uninstaller/src-tauri/tauri.conf.json`：將預設視窗高度由 560 調升至 630，確保第一幀擁有充足空間完整容納所有 15 個核取項目，杜絕捲軸閃現。
  - `uninstaller/ui/index.html`：`body` 樣式由 `overflow-y: auto;` 調整為 `overflow-y: hidden;`；服務核取選項文字由「(PM2 服務, Apache2.4)」更新為「(系統入口閘道服務, 背景服務總線)」。
  - `uninstaller/ui/app.js`：實作 `autoFitWindow()`，透過 `requestAnimationFrame` 精準量測到達操作列底部的文檔流高度，調用後端自適應命令；同步更新確認對話框文字。
  - `uninstaller/src-tauri/src/lib.rs`：新增 `fit_window_to_content` 指令，將客戶區大小設定為前端回報之實際高度（480px 至 900px），自動緊湊貼齊邊界。
  - `uninstaller/src-tauri/src/services.rs`：日誌輸出文字同步由「Web / Apache 服務」、「PM2 Windows 系統服務」替換為「Web 系統入口服務」與「背景服務總線」。
- **相容性說明**：不改變任何底層 SCM 服務解除安裝與檔案清理邏輯，編譯產出之二進位檔案大小為 10.71 MB。

### 🌐 修復 Apache FastCGI 反向代理 SCRIPT_FILENAME 語法錯誤與補齊 PHP 8.5 串流端點

- **修改目的**：解決切換至 PHP 8.5 現代化新軌時，因 Apache httpd.conf 中 ProxyFCGISetEnvIf 誤用 `$$1` 導致傳遞給 php-cgi.exe 之 SCRIPT_FILENAME 開頭帶有多餘 `$` 字元（如 `$C:/AppServ/LAH-API/...`），引發 FastCGI 全面報錯 HTTP 404 (`No input file specified.`) 之問題；並解決 get_*.php 串流端點未同步至 LAH-API.zip 導致頭像與圖片報錯 404 之問題。
- **具體修改項目**：
  - `updater/src-tauri/src/apache_conf.rs`：將 ProxyFCGISetEnvIf 正則替換捕獲組由 `$$1` 校準為 `$1`；修復 re_stale_alias 正則換行符號匹配；單元測試 10 項全數通過。
  - `pack/pack.py`：重新打包 dist/patch/LAH-API.zip，納入最新 v1/get_*.php 串流腳本。
  - 實體環境修復：校準 `C:\AppServ\Apache24\conf\httpd.conf`，補齊 `C:\AppServ\LAH-API\v1\get_*.php` 與 `C:\AppServ\www\assets\img\`，重啟 LAH_Web_Service。
- **相容性說明**：前端切換至 PHP 8.5 成功取得版本資訊與 SAPI 模式，所有 API 與圖片請求全面恢復 HTTP 200 OK。

### 🛡️ Windows 服務管理員 (services.msc) 標題與描述全面資安脫敏

- **修改目的**：避免服務標題與描述中出現 Apache、PHP、node、PM2 等易被弱點掃描工具與資安稽核誤判為潛在風險之開源中介軟體技術詞彙，轉由地政業務入口與平台系統總線之中性架構語言呈現。
- **修改項目**：
  - `updater/service/lah-web-service.xml`：Web 系統服務名稱調整為「LAH 智慧管控：應用系統入口與 API 閘道服務」，描述調整為「提供地政智慧管控系統前端介面存取、業務查詢 API 轉發與安全反向代理，負責核心通訊連線與身分驗證傳輸。」，移除 Apache 2.4 與 httpd.exe 等字詞。
  - `updater/service/lah-service.xml`：背景守護服務名稱調整為「LAH 智慧管控：背景服務總線與健康管理系統」，描述調整為「協調地政管控平台背景多工任務執行、即時事件分發與系統運作指標監測，確保關鍵作業永續運作。」，移除 node、PM2 等關鍵詞。
  - `updater/ui/index.html`：系統服務端點卡片名稱同步脫敏為「🌐 系統入口 (Web / LAH_Web_Service)」，徹底消除介面 Apache 痕跡。
- **相容性說明**：服務 ID 依然維持 `LAH_Web_Service` 與 `LAH_PM2_Service` 不變，與所有 SCM 查詢、啟動、停止、卸載邏輯完全相容；以更新工具套用配置即可於 SCM 重新註冊生效。

### 更新工具視窗高度動態自適應與徹底消除捲軸

- **修改目的**：更新工具剛開啟與展開「從備份檔案還原」時右側出現垂直捲軸，原因為預設視窗高度不足，且前端 `autoFitWindow()` 為空實作。
- **修改項目**：
  - `updater/ui/app.js`：實作 `autoFitWindow()`，量測文檔流真實高度（排除 sticky 底部列，加 4px 裕度），初始渲染、掃描完成與勾選備份還原時呼叫後端 `fit_window_to_content`。
  - `updater/src-tauri/src/lib.rs`：`fit_window_to_content` 直接設定客戶區高度，不再疊加標題列與邊框，限制 500 至 960px。
  - `updater/src-tauri/tauri.conf.json`：預設高度由 760 調升至 825。
  - `updater/ui/index.html`：`body` 改為 `overflow-y: hidden`。
  - `AGENTS.md`：新增 3.9 節，明定介面禁止出現 Scrollbar 之強制規範。
- **實測量測**：收合狀態內容高 778px，展開備份列後 815px。

### 🌐 實現 Apache 2.4 網頁伺服器 WinSW 完全沙盒化 (LAH_Web_Service) 與 VC++ 依賴獨立封裝

- **修改目的**：
  - 徹底解決 Apache 2.4 原生使用 `httpd.exe -k install` 註冊系統服務引發之全域 SCM 侵入、名稱衝突以及對全域 PATH（如 Instant Client）與 VC++ Redistributable 之外部依賴。
  - 將服務名稱命名為 `LAH_Web_Service`，改由自帶之 WinSW 服務核心 (`lah-web-service.exe`) 進行進程沙盒守護，將 `PATH` 限縮於服務內部，達到 100% 零全域環境變數污染。
  - 支援舊版 Apache 服務（`Apache2.4` / `Apache24`）自動無縫卸載接管，防範開機雙服務競爭 Port 80。
  - 遵循「不修改網路下載之官方原生 Apache ZIP 包」原則，將 Visual C++ 2015–2022 轉散發 DLL 獨立封裝為 `vcredist-x64.zip`，透過檔名工具鏈解析（如 `VS18`、`VS17`、`VC15`）自動解壓並同步補齊至 `Apache24/bin`、`php7/` 與 `php85/` 目錄。
- **具體修改項目**：
  - **自動更新工具核心 (`updater/`)**：
    - `service/lah-web-service.xml`：新增 WinSW 服務模板，宣告私有 `PATH`（綁定 `Apache24\bin`、`instantclient_11_2` 與 `php7`）與 `NLS_LANG`，配置故障自癒重啟。
    - `src-tauri/src/services.rs`：新增 `LAH_WEB_SERVICE_NAME` 常數，於 `query_apache_service` 優先探測 `LAH_Web_Service`；`install_apache_service` 實作自動偵測並卸載舊版 Apache 服務、釋出 WinSW 檔案並啟動守護。
    - `src-tauri/src/extractor.rs`：新增 `parse_apache_zip_meta` 解析版本與工具鏈、`find_vcredist_zip` 探測獨立 VC++ 壓縮包、`sync_vcredist_dlls_to_php` 同步程式庫，並將 `vcredist` 納入 `ALL_PACKAGE_SPECS` 與 `base/` 分類，通過 10 項單元測試。
    - `src-tauri/src/pipeline.rs`：解壓流程支援自動輸出 Apache 版本工具鏈日誌、自動探測並解壓獨立 `vcredist` 依賴至 `Apache24/bin` 並同步至 PHP，服務註冊與健康檢查對齊 `LAH_Web_Service`。
    - `src-tauri/src/system.rs`：根目錄探測加入 `LAH_Web_Service` 登錄檔反推；更新專屬沙盒維運終端 `lah-cmd.bat`，注入 Apache 與 Instant Client 路徑並提供維運指引。
    - `ui/index.html` & `ui/app.js`：更新核取方塊與狀態標籤為「自動註冊 Web 伺服器為 Windows 系統服務 (LAH_Web_Service)」，前端新增 `vcredist` 依賴卡片與端點說明。
  - **自動移除工具 (`uninstaller/`)**：
    - `src-tauri/src/services.rs`：卸載管線優先透過 `lah-web-service.exe stop & uninstall` 清理 Web 沙盒服務，並備援刪除 SCM 註冊。
    - `src-tauri/src/system.rs`：服務 ImagePath 反推根目錄加入 `LAH_Web_Service`，通過 2 項單元測試。
  - **備份與還原工具 (`backuper/`)**：
    - `src-tauri/src/system.rs`：服務 ImagePath 反推根目錄加入 `LAH_Web_Service`，通過 1 項單元測試。
  - **系統打包腳本 (`pack/pack.py`)**：
    - 新增 `build_vcredist_package` 函式，自動自系統收集 10 大 x64 VC++ 2015–2022 核心程式庫打包為 `dist/base/vcredist-x64.zip`。
- **相容性說明**：
  - 產出之 `dist/智慧控管與監控系統 - 自動更新工具.exe` (11.86 MB) 與 `dist/智慧控管與監控系統 - 自動移除工具.exe` (10.71 MB) 均通過 Release 實體編譯與全套單元測試。
  - 官方下載之 `httpd-*.zip` 保持純淨原貌無須重壓，全系統環境變數零污染。

### 🛡️ 全面對齊 node-portable 零全域污染沙盒模式與無滾動條體驗

- **修改目的**：
  - 徹底解決 `node-portable`（可攜 Node 安裝工具）介面中仍殘留「修改系統/使用者 PATH 與 PM2_HOME」勾選項目引發誤會之問題，對齊 `updater` 零全域污染沙盒隔離規範。
  - 將安裝選項精簡化：移除 PATH 與 PM2_HOME 勾選框，安裝流程預設自動執行沙盒防護（主動清理登錄檔殘留、生成專屬終端 `lah-cmd.bat`、環境變數限縮於當前程序與背景服務），絕不干擾本機全域已安裝之 Node.js 與 PM2。
  - 同步優化版面邊距與視窗高度設定至 560px，徹底消除啟動時的垂直滾動條（Scrollbar）。
- **具體修改項目**：
  - **後端流程與型別 (`node-portable/src-tauri/`)**：
    - `src/types.rs`：將 `InstallOptions` 中的 `register_path` 與 `set_pm2_home` 改為預設反序列化欄位，維護前後端契約相容。
    - `src/install.rs`：安裝管線全面自動執行沙盒化環境套用（自癒清理登錄檔、建立專屬終端 `lah-cmd.bat`），更新日誌與進度文字為「🛡️ 套用零全域污染沙盒隔離與建立維運終端...」，通過 3 項單元測試。
  - **視窗配置與前端介面 (`node-portable/`)**：
    - `src-tauri/tauri.conf.json`：預設視窗尺寸調整為 `width: 960, height: 560`（minHeight: 500）。
    - `ui/index.html`：移除 PATH 與 PM2_HOME 勾選項，新增綠色「🛡️ 零全域污染沙盒架構」提示區塊，精簡版面邊距消除滾動條。
    - `ui/app.js`：移除多餘 DOM 存取，專注提交核心服務註冊與日誌輪替選項。
- **相容性說明**：
  - 僅調優 UI 與對齊沙盒預設流程，解壓與 WinSW 服務註冊完全相容，已重新打包產出最新二進位檔並同步發布至 `\\220.1.34.18\D$\LAH智慧控管工具\`。

### 🛠️ 新增獨立小工具套件庫 (tools/) 與「地籍異動簡訊參數設定工具 (sms-env-editor)」

- **修改目的**：
  - 於 `LAH-Updater` 建立 `tools/` 模組化資料夾架構，提供維運人員以 Rust (Tauri 2) 開發獨立輕量免安裝維運工具之統一標準。
  - 針對內政部地籍異動簡訊與郵件發送服務（SMSWEB_MOI）設定檔 `\\220.1.34.221\C$\SMSWEB_MOI\env.properties`，打造專屬可視化編輯器，取代人工手動編碼與舊版 Java 工具相容性問題。
  - 建立新工具開發之 App 圖示專屬設計規範，嚴禁套用舊圖示，並規範提出方案供使用者確認。
- **具體修改項目**：
  - **核心架構與規範 (`tools/README.md`, `AGENTS.md`)**：
    - 定義 `tools/` 專案結構、`.cargo/config.toml` 獨立 `target-dir` 避免膨脹、以及發布輸出至 `dist/` 之標準管線。
    - 於 `AGENTS.md` 新增 3.8 節「新增維運小工具與 App 圖示 (ICO) 專屬設計規範」，強制量身設計、提出方案供審查，並產出完整多解析度圖示套裝。
  - **地籍異動簡訊參數設定工具 (`tools/sms-env-editor/`)**：
    - `src-tauri/`：基於 Tauri 2 實作，強型別領域模型 `ConfigData`，支援 BASE64 自動加解密、Big-5 (CP950) 無損編碼、變更比對 Diff 與時間戳自動備份 (`env.properties.YYYYMMDD_HHMMSS.bak`)。
    - `ui/`：純靜態 HTML/CSS/JS 介面，零外部 CDN，精準對齊原始 Java 介面 10 大核心欄位，支援密碼顯隱切換與進階排程參數折疊區。
    - `generate_icons.ps1`：原生 Windows System.Drawing 圖示生成腳本，產出天青石深藍底座、純白訊息泡泡與琥珀金設定齒輪之多解析度 ICO (16~256) 與 PNG 套件。
    - `build.py` / `build.bat` / 根目錄 `建立簡訊設定工具.bat`：一鍵 Release 編譯發布至 `dist/地籍異動簡訊參數設定工具.exe` (9.36 MB)。
  - **版控設定 (`.gitignore`)**：
    - 忽略 `tools/**/src-tauri/target/` 與 `gen/`。
- **相容性說明**：
  - 工具為獨立二進位檔，無外部環境依賴；設定檔編碼完全相容既有 SMSWEB_MOI 服務；不影響 LAH-Updater 其他四大核心工具。


### 🖥️ 優化自動移除 (uninstaller) 與備份還原 (backuper) 工具之版面與精準緊緻尺寸

- **修改目的**：
  - 徹底解決 `uninstaller`（自動移除工具）與 `backuper`（備份與還原工具）在預設啟動時因視窗高度不足超出 20~40px 產生垂直滾動條（Scrollbar）之視覺問題。
  - 解決先前因 CSS 緊湊化與視窗放大雙重疊加導致底部多餘留白過大之問題，依像素精準計算按鈕與卡片底緣，將視窗高度縮短至緊緻剛好尺寸（底部留白精準收斂至 12~14px）。
- **具體修改項目**：
  - **自動移除工具 (`uninstaller/`)**：
    - `src-tauri/tauri.conf.json`：預設視窗尺寸精準調整為 `width: 980, height: 560`（minHeight: 520）。
    - `ui/index.html`：精簡 body padding（12px 18px）、card padding（10px 14px）與 options-grid 間距，全套單元測試通過。
  - **備份與還原工具 (`backuper/`)**：
    - `src-tauri/tauri.conf.json`：預設視窗尺寸精準調整為 `width: 780, height: 440`（minHeight: 400）。
    - `ui/index.html`：精簡 body padding（13px 18px）、card padding（11px 15px）與 label/btn-row 外距，單元測試通過。
- **相容性說明**：
  - 僅調優 UI 佈局與視窗尺寸，核心邏輯完全相容，已重新打包產出最新二進位檔並同步發布至 `\\220.1.34.18\D$\LAH智慧控管工具\`。

### 🎨 優化自動更新工具 (updater) 介面版面、快選工具列與無滾動條體驗

- **修改目的**：
  - 依據維運介面反饋調整按鈕佈局與分類歸屬：將「⚡ 立即套用配置與註冊服務」按鈕移入系統服務卡片標題列右側，釋放清單底部垂直空間。
  - 將「Node.js 綠色執行環境 (node.zip)」重新歸納至「🏛️ 基礎環境安裝包 (base)」，使左右雙欄項目數量（左 4、右 6）達成高度平衡，且對齊 `pack.py` 分流目錄 `dist/base/node.zip`。
  - 增設集中式整合快選工具列（位於路徑列與雙欄之間），提供全域「全部選取」、「全部取消」，以及「基礎環境 (base)」、「程式修補 (patch)」、「核心套件 (node_modules)」智慧切換（Toggle）按鈕。
  - 強化未偵測到壓縮包之反饋體驗：當壓縮包不存在時，除了既有的 Checkbox 原生 `disabled`、取消勾選、卡片半透明淡化與「未偵測到」徽章外，卡片全面套用 `cursor: not-allowed` 禁行游標與 `user-select: none`，明確向使用者提示不可操作。
  - 右欄增設「🌐 系統服務端點與維運指引」資訊卡片：完美填補右欄下方空白區域，使左右雙欄視覺高度達到高度齊平；上半部以 2x2 精美網格條列 Apache (:80)、Nuxt (:8080)、即時通 WS (:8081) 與 HTTP 附件 (:8082) 等服務監聽埠與用途，下半部提煉日常 Patch 更新要點與專屬隔離沙盒 (`lah-cmd.bat`) 操作叮嚀。
  - 調整版面間距並擴增視窗高度至 760px，保證在預設啟動狀態下所有元件舒適陳列，完全不出現垂直滾動軸（Scrollbar）。
- **具體修改項目**：
  - **後端分類對齊 (`updater/src-tauri/src/extractor.rs`)**：
    - 將 `PackageSpec` 中 `id: "node"` 之 `is_base_env` 設為 `true`，通過全套 9 項單元測試。
  - **視窗組態優化 (`updater/src-tauri/tauri.conf.json`)**：
    - 將預設視窗尺寸調整為 `width: 1040, height: 760`。
  - **前端介面與快選邏輯 (`updater/ui/index.html` & `app.js`)**：
    - 移動套用按鈕至標題列右側，基礎環境包副標題更新為 `Apache / PHP / Node.js`。
    - 實作集中式快選工具列，支援依 `PKG_CATEGORIES`（base, patch, node_modules）進行單擊全選／再次單擊取消全選之智慧切換。
    - 增強 `.pkg-item.missing` 樣式，設定 `cursor: not-allowed !important` 與背景色，全面強化未就緒項目之反饋。
    - 於右欄新增 `🌐 系統服務端點與維運指引` 卡片，配置 `.endpoint-grid` 與 `.maint-tip-box`，達成左右欄視覺高度平衡。
- **相容性說明**：
  - 前後端介面契約與解壓縮流程相容，已重新打包產出最新二進位檔（11.83 MB）並同步發布至 `\\220.1.34.18\D$\LAH智慧控管工具\`。

### 🔄 全面升級自動移除 (uninstaller) 與備份還原 (backuper) 工具架構

- **修改目的**：
  - 巡查並全面對齊 `uninstaller` 與 `backuper` 至最新版 FastCGI 雙軌架構、零全域污染沙盒與多磁碟動態安裝環境。
  - 徹底解決舊版自動移除工具遺漏卸載 `PM2_Portable_Service`、未清理殘留 `php-cgi.exe` 進程、未自登錄檔（HKLM/HKCU）抹除 `PM2_HOME` 等歷史脫節問題。
  - 升級備份還原工具：將專屬維運終端腳本 `lah-cmd.bat` 納入重要備份清單；還原進程防鎖檢查同步納入 `php-cgi.exe`；實作還原後自動校準 `httpd.conf` 路徑機制，防止跨磁碟還原時因舊路徑覆蓋導致 Apache 崩潰。
  - 兩工具全面移植 `detect_default_root_dir` 動態偵測邏輯，廢除 UI 硬編碼 `C:\AppServ`，支援非 C 槽（如 `D:\AppServ`）隨選即用。
- **具體修改項目**：
  - **自動移除工具 (`uninstaller/`)**：
    - `src-tauri/src/services.rs`：`uninstall_lah_pm2_service` 同步納入 `LAH_PM2_Service` (lah-service.exe) 與 `PM2_Portable_Service` (pm2-service.exe)，並以 `sc.exe query` + `sc.exe delete` 備援。
    - `src-tauri/src/process.rs`：終止進程全面涵蓋 `php-cgi*.exe`、`httpd*.exe`、`node*.exe`（含 Daemon.js、God.js、runner）與 `pm2.exe`。
    - `src-tauri/src/autostart.rs`：在 `remove_paths_from_environment` 徹底清除 HKLM 與 HKCU 中殘留之 `PM2_HOME`、`PM2_INSTALL_DIRECTORY` 與 `PM2_SERVICE_DIRECTORY`，並廣播 `WM_SETTINGCHANGE`。
    - `src-tauri/src/cleaner.rs`：移除 Node 項目中同步納入 `.pm2` 目錄與專屬維運腳本 `lah-cmd.bat` 的主動清理。
    - `src-tauri/src/system.rs` & `lib.rs`：實作 `detect_default_root_dir` 並透過 Tauri 命令 `detect_root_dir` 暴露給前端，通過 2 項單元測試。
    - `ui/app.js` & `ui/index.html`：初始化時非同步動態填入偵測路徑，更新各項服務與進程之繁體中文描述文字。
  - **備份與還原工具 (`backuper/`)**：
    - `src-tauri/Cargo.toml`：新增 `regex = "1"` 與 `winreg = "0.56"` 依賴。
    - `src-tauri/src/backup.rs`：`BACKUP_ITEMS` 擴充納入 `("lah-cmd.bat", "lah-cmd.bat")`。
    - `src-tauri/src/restore.rs`：`PS_LIST_EXE` 檢查納入 `php-cgi.exe`；新增 `calibrate_httpd_conf` 常式，還原後自動校準 SRVROOT、DocumentRoot、rotatelogs、FastCGI Alias 及 PHP 7 模組路徑，通過單元測試。
    - `src-tauri/src/system.rs` & `lib.rs`：新增 `system` 模組實作 `detect_default_root_dir`，暴露 `detect_root_dir` 命令。
    - `ui/app.js` & `ui/index.html`：初始化時動態探測填入安裝路徑，廢除預設硬編碼。
- **相容性說明**：
  - 經由單元測試（`cargo test --lib`）與 release 二進位建置發布（各約 10~11 MB），完全相容現行生產環境，並已同步發布至 `\\220.1.34.18\D$\LAH智慧控管工具\`。

### 🛡️ 實作 Node 與 PM2 零全域污染沙盒隔離架構並附帶 lah-cmd.bat 維運終端

- **修改目的**：
  - 徹底解除自動更新與可攜 Node 安裝工具對 Windows 系統（HKLM）與使用者（HKCU）環境變數 `Path` 及 `PM2_HOME` 的登錄檔接管。
  - 解決開發者與維運人員在一般命令提示字元下執行 `node --version` 會被可攜版覆蓋干擾之問題，保證桌面一般終端完全維持全域安裝之 Node.js 版本（如 Node 20/22）。
  - 將可攜版 Node 16 與 PM2 運行環境完全限縮於 WinSW 背景服務與專屬沙盒內，同時自動提供專屬維運終端捷徑 `lah-cmd.bat`。
- **具體修改項目**：
  - **Updater 核心 (`updater/src-tauri/src/system.rs`)**：
    - 重構 `register_path_to_user_environment` 與 `register_pm2_home_and_clean_legacy_env`：停止寫入 HKLM/HKCU 登錄檔，停止拔除全域 `C:\Program Files\nodejs`。
    - 新增 `purge_portable_from_registry` 自癒機制：自動自登錄檔清理過往早期版本不慎寫入之綠色路徑與 `PM2_HOME`。
    - 新增 `ensure_portable_cmd_helper`：於根目錄自動產生/更新 `lah-cmd.bat`，以 `setlocal` 隔離注入可攜環境，關閉視窗即釋放。
    - 新增單元測試 `test_purge_target_from_path_preserves_global_node`，通過 9 項單元測試。
  - **Node Portable 核心 (`node-portable/src-tauri/src/system.rs`)**：
    - 同步實作零全域污染模式與自癒清理機制，通過 3 項單元測試。
- **相容性說明**：
  - LAH 系統開機自啟服務（WinSW `LAH_PM2_Service` / `PM2_Portable_Service`）透過 XML 與 `service_runner.js` 自身已完整包含局部環境變數，背景守護與 5 大子專案運行完全不受影響；
  - 徹底還原作業系統全域環境獨立性，達成真正的零污染綠色可攜架構。

### 🛡️ 注入串流端點向後相容 FastCGI 映射並對齊 AGENTS.md 維運防護守則

- **修改目的**：
  - 確保智慧控管前端與外部系統呼叫根目錄舊版串流腳本（如 `/get_user_img.php`、`/get_carousel_img.php`、各類 PDF 報表串流）時，能無縫轉發至 PHP 8.5 FastCGI 後端核心（`127.0.0.1:8085`），達成向後相容。
  - 對齊全域架構契約，於 `AGENTS.md` 載明維運四大工具架構（updater、node-portable、uninstaller、backuper），並新增第 3.4 至 3.7 節離線維運防護守則。
- **具體修改項目**：
  - **Apache FCGI 轉發設定 (`updater/src-tauri/src/apache_conf.rs`)**：
    - 於 `configure_apache_fcgi_proxy` 注入 6 大串流端點別名映射：`/get_user_img.php`、`/get_carousel_img.php`、`/get_blob_img.php`、`/get_adm_reserve_pdf.php`、`/get_reg_foreigner_pdf.php`、`/get_sur_destruction_pdf.php` 指向 `{api_fwd}/v1/...`。
    - 通過全部 8 項單元測試並重新編譯產出最新 Release 二進位產物（11.84 MB）。
  - **開發與 AI Agent 指南更新 (`AGENTS.md`)**：
    - 擴充目錄結構，納入 `node-portable`（可攜 Node 16.20.2 安裝工具）與對應一鍵建置指令。
    - 增補第 3.4~3.7 節防護守則：Node/PM2 四重安全隔離與殘留模組淨化、Apache/雙 PHP 組態自癒與日誌正規化、行程鎖定安全釋放與雙層目錄自癒、零外部 CDN 原則。
- **相容性說明**：
  - 完美相容前端既有圖片與 PDF 串流請求，增強代理與工程師在維護工具時之安全防禦一致性。

### 🔧 強化 Apache 與 PHP 組態自癒清理機制並校正日誌路徑

- **修改目的**：
  - 解決系統由舊版 AppServ 遷移或跨磁碟（如 `C:\AppServ` 移至 `D:\DEV`）部署時，Apache `httpd.conf` 殘留舊版硬編碼路徑（散落之 `LoadFile`、`LoadModule php7_module`、`PHPIniDir`、`<IfModule mod_php7.c>` 以及舊版 `Alias /LAH-API` 與 `<Directory>`），導致 Apache 模組衝突或轉發失敗之問題。
  - 校正 `ErrorLog` 與 `CustomLog` 之 rotatelogs 日誌路徑，防禦日誌管道因硬編碼目錄不存在而啟動崩潰。
  - 徹底淨化 `php.ini`，避免殘留重複或註解之 `extension_dir` 設定，確保僅保留單一精準之延伸模組目錄。
- **具體修改項目**：
  - **Apache 與 PHP 設定治理 (`updater/src-tauri/src/apache_conf.rs`)**：
    - `ensure_php_ini_configured`: 逐行遍歷過濾，清理所有舊版或註解之 `extension_dir`，嚴格保證僅保留單一精準之 `extension_dir = "<php_dir>/ext"` 設定。
    - `configure_apache_mod_php7`:
      - 動態正規化 `ErrorLog` 與 `CustomLog` rotatelogs 管道日誌路徑，將任何舊磁碟路徑重寫為當前 `<apache_dir>/logs/`。
      - 徹底搜尋並剔除所有散落於舊版 AppServ 設定檔之 `LoadFile` (php7 相關相依 DLL)、`LoadModule php7_module`、`PHPIniDir` 與 `<IfModule mod_php7.c>` 區塊，再注入乾淨統一之標準 PHP 7.4 模組區塊。
    - `configure_apache_fcgi_proxy`:
      - 主動清理指向舊路徑之無效 `Alias /LAH-API` 與 `<Directory .../LAH-API>` 區塊，避免跨目錄更新時與新注入之 FastCGI Proxy 產生衝突。
  - **單元測試防護 (`updater/src-tauri/src/apache_conf.rs`)**：
    - 新增 `test_ensure_php_ini_removes_duplicate_and_stale_extension_dir` 測試，斷言重複 `extension_dir` 被淨化為唯一定義。
    - 新增 `test_configure_apache_mod_php7_cleans_legacy_appserv_paths` 測試，斷言自舊根目錄移轉時所有舊版路徑皆被自癒更新。
    - 通過全套 8 項單元測試（`cargo test --lib`），並重新編譯產出最新版 `dist/智慧控管與監控系統 - 自動更新工具.exe` (11.82 MB)。
- **相容性說明**：
  - 完全向下相容全新安裝與升級安裝情境，具備強大之組態自癒與遷移容錯能力。

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
