# 專案交接文件（HANDOFF）：`moi-sms` 遷移至 `LAH-Workspace`

> **建立時間**：2026-11-13  
> **交接目標專案**：`d:\DEV\lah-workspace\LAH-Updater\tools\moi-sms`  
> **定位**：取代舊版 Java (`MOI_EX.exe` + `SMS.exe`) 的 Rust + Tauri 2 桌面控管程式，所有資料庫查詢與交易已解耦至 `LAH-API`，本機程式僅負責設定管理、查詢檢視、排程控管與派送串接。

---

## 1. 目前完成狀態

1. **核心函式庫（`src/`）**
   - [src/config.rs](src/config.rs)：相容讀取舊版 Big5 與 Base64 編碼的 `env.properties`（`EnvConfig::from_file`）。
   - [src/encoding.rs](src/encoding.rs)：實作 `ISO-8859-1` ↔ `Big5 (MS950)` ↔ `UTF-8` 轉換與民國日期格式處理。
   - [src/comm.rs](src/comm.rs)：實作簡訊／郵件閘道用戶端（`SmsGatewayClient`）與可測試 `HttpTransport`（支援 `GET` / `POST`）。
   - [src/lah_client.rs](src/lah_client.rs)：實作 `LAH-API` REST 用戶端（`LahApiClient`），對應全部 6 個端點（`/metadata`、`/cases/sync`、`/queue/enqueue`、`/queue/pending`、`/queue/report`、`/records`）。

2. **Tauri 2 桌面控管端（`src-tauri/` 與 `ui/`）**
   - 已串接 `LAH-API`：支援首頁即時統計（`main` / `wait` / `log`）、案件清單查詢與篩選、一鍵觸發案件同步與入列（`POST /cases/sync` + `POST /queue/enqueue`）。
   - 設定與連線測試：
     - 預設端點：`http://127.0.0.1/LAH-API/v1/sms_v1_api.php`（PHP 8.5 FastCGI 軌道，並自動將舊版 PHP 7.4 `/api/sms_v1_api.php` 與相容別名 `/LAH-API/api/sms_v1_api.php` 升級至 PHP 8.5 路徑）。
     - 支援於「系統設定」修改並記住遠端 `LAH-API URL`，提供「測試連線（`GET /metadata`）」與「從 `env.properties` 匯入」功能。
     - 設定檔優先讀取執行檔同目錄 `settings.json`（可攜式部署），否則存於系統 `app_config_dir/settings.json`（並相容讀取舊版 `desktop-demo.json`）。
   - 檔案日誌與視窗生命週期：
     - 優先輸出時間戳日誌至執行檔同目錄 `logs/moi-sms.log`（否則退回系統 `app_log_dir`）。
     - 已修正 Tauri v2 關閉視窗卡住問題：僅在設定未儲存時攔截關閉提示，確認捨棄後直接 `destroy()` 並確保行程正常結束。

3. **`LAH-Updater` 整合與建置**
   - [.cargo/config.toml](.cargo/config.toml)：已將 `target-dir` 導向 `C:/Temp/cargo-target/moi-sms`。
   - [build.py](build.py) 與上層 [../../建立地籍異動即時通簡訊控管工具.bat](../../建立地籍異動即時通簡訊控管工具.bat)：執行單元測試、Release 編譯並輸出至 `LAH-Updater/dist/智慧控管與監控系統 - 地籍異動即時通簡訊控管.exe`。
   - [.gitignore](.gitignore)：已排除 `target/`、`dist/`、`logs/`、`env.properties`、`settings.json`、`*.exe` 等敏感或大型建置產物。

---

## 2. 已驗證項目

- **離線單元測試**（`cargo test --workspace --locked`）：`moi_sms` 14 項 + `moi-sms-desktop` 6 項，共 20 項全數通過。
- **實機 HTTP 聯調**（[tests/live_lah_api.rs](tests/live_lah_api.rs)）：已於原專案路徑通過 `LAH-API` PHP 7.4 與 PHP 8.5 端點測試（於本目錄可用 `cargo test --test live_lah_api -- --ignored` 重跑）。
- **發布建置**：`python tools\moi-sms\build.py` 已成功產出 `LAH-Updater/dist/智慧控管與監控系統 - 地籍異動即時通簡訊控管.exe`。

---

## 3. 尚未完成與後續待辦（Next Steps）

1. **背景自動排程派送迴圈（高優先）**
   - 目前桌面端的「啟動排程／暫停排程」按鈕僅切換狀態旗標；下一步需在 Rust 後端依 `search_minutes`、`max_retries` 與 `start_time`～`end_time` 實作背景排程迴圈：
     - 定期呼叫 `sync_cases` + `enqueue_queue`
     - 呼叫 `get_pending_queue` 取得待發送清單
     - 透過 `SmsGatewayClient`（需將 `comm.rs` 的 `send_new_msg` payload 對齊 `LAH_API_SPEC.md` 第 4.4 節 `method: "m" / "e"` 與 `code == "1"` 判斷）派送至 `ap_url`
     - 呼叫 `report_queue_item` 回報成功或失敗重試
2. **專屬圖示與 `LAH-Updater` 文件同步**
   - 依 `LAH-Updater/tools/README.md` 規範，目前 `src-tauri/icons/` 仍為預設圖示，需設計專屬圖示替換，並將 `moi-sms` 補入 `LAH-Updater/tools/README.md` 工具清單。
3. **正式環境上線前切換**
   - 目前本機 `LAH-API` 端為 `MOCK=true`（SQLite 快照庫）；正式聯調 Oracle 前需確認 `LAH-API` 端切換環境變數與跨機連線安全性（如是否需補 Bearer Token）。

---

## 4. 重要參考文件（勿重複複製，請直接查閱）

- API 與架構規格：[LAH_API_SPEC.md](LAH_API_SPEC.md)
- 舊版 Java 逆向規格：[SPEC.md](SPEC.md)
- 桌面 UI 設計規劃：[UI_PLAN.md](UI_PLAN.md)
- 工具套件庫規範：[../README.md](../README.md)

---

## 5. Suggested Skills（建議接手 Agent 載入之技能）

- **`/code-review`**：接手後先針對搬遷過來的程式碼進行 Standards 與 Spec 雙軸檢查，確認符合 `LAH-Workspace` 規範。
- **`/implement`**（搭配 **`/tdd`**）：實作背景排程派送迴圈與 `SmsGatewayClient` 新版 `method: "m" / "e"` payload 對齊。
- **`/diagnosing-bugs`**：若實機聯調 `LAH-API` 或閘道派送發生異常時，搭配 `logs/moi-sms.log` 進行最小重現與除錯。
