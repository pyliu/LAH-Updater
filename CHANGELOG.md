# 智慧控管與監控系統 - 自動更新與維運工具 (LAH-Updater) 版本修改歷程

本文件記錄 `LAH-Updater` 維運工具套件（包含自動更新工具 `updater.py`、自動移除工具 `uninstaller.py` 以及系統打包腳本 `pack.py`）之功能演進、架構調整與資安防護修復歷程。

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
