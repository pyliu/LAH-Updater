# MOI-RUST

將舊版 MOI_EX／SMS 的通知管理流程移植到 Rust。現有核心模組保留，新增 Tauri 2 五分頁桌面程式。

## 開啟桌面程式

已建置的 Windows 執行檔：`dist/moi-sms-desktop.exe`。雙擊即可，需有 Microsoft Edge WebView2 Runtime。

從原始碼執行：

```powershell
cargo run -p moi-sms-desktop --locked
```

也可以雙擊 `run-desktop.bat`。不需要 Node、npm、Tauri CLI 或前端開發伺服器；HTML、CSS 與 JavaScript 直接內嵌於 Tauri 執行檔。

建置可分發的 Release 執行檔：

```powershell
.\build-desktop.bat
```

此腳本執行整個 workspace 的測試、Release 建置，再複製至 `dist/moi-sms-desktop.exe`。
重新建置並覆蓋執行檔前，請先關閉正在執行的桌面程式。
建置端需要 Rust MSVC 工具鏈、Visual Studio C++ Build Tools 與 Windows SDK。第一次建置若沒有套件快取，需要網路下載 Cargo 相依套件。

## 目前可操作的桌面功能

- 已登案件、待傳案件、傳送紀錄、傳送訊息、系統設定五頁。
- Rust 後端提供展示資料、日期／管道／結果／關鍵字查詢及收件號排序。
- 選取案件後閱讀完整訊息及錯誤原因；可複製通知文字。
- 展示排程啟動／暫停、模擬案件同步（重複同步會去重）、模擬測試傳送。
- Rust 驗證設定與測試收件者；設定可儲存、重新載入或取消修改。
- 尚未儲存設定時切頁、關閉視窗會提示；F5 更新、設定頁 Ctrl+S 儲存。

**此版本為可執行的桌面展示版，尚未串接 LAH-API、真實排程、簡訊與郵件閘道。**
所有案件與聯絡資訊為虛構，日期固定為民國 115/11/13。啟動展示排程不會在背景掃描或派送。

非密碼設定存於 Tauri 的應用程式設定目錄 `tw.gov.moi.sms.desktop/desktop-demo.json`，實際完整路徑可於系統設定頁展開查看。
簡訊與郵件密碼僅保留於本次程序記憶體，不寫入設定檔。重開程式後排程為暫停，展示案件恢復初始內容，操作訊息重新開始。
原有 `env.properties` 與 `dist/env.properties` 不會被此桌面版讀取或修改。

## 原始碼

| 路徑 | 用途 |
|---|---|
| `src/` | 原有設定、編碼與 HTTP 核心，以及 CLI 展示入口 |
| `src-tauri/src/main.rs` | Tauri 啟動、IPC 指令與設定檔存取 |
| `src-tauri/src/model.rs` | Rust 桌面資料、查詢、驗證與展示操作 |
| `src-tauri/tauri.conf.json` | 原生視窗、內嵌前端與 CSP |
| `ui/` | 正式桌面入口的 HTML、CSS 與 JavaScript，透過 Tauri invoke 呼叫 Rust |
| `prototypes/` | 之前比較布局的獨立原型，不會打包進桌面執行檔 |
| `UI_PLAN.md` | UI 規劃及後續整合缺口 |
| `LAH_API_SPEC.md` | 預計串接的服務契約 |

保留原 CLI 行為：`cargo run -p moi_sms -- <env.properties路徑>`。原本 `build.bat` 仍建置 CLI；桌面版使用 `build-desktop.bat`。

## 驗證

```powershell
cargo test --workspace --locked
cargo fmt -p moi-sms-desktop -- --check
node --check ui/app.js
```

桌面模組包含篩選、同步去重、暫停保留待傳資料、設定及測試傳送邊界的 Rust 測試。實際驗收時請檢查五個頁面、設定保存與重開、未儲存離開提示，以及現場 Windows 縮放下的表格閱讀。

本次已通過 15 項 Rust 測試及 Windows Release 建置。實際啟動 Tauri/WebView2，驗證五頁導覽、IPC 查詢、案件同步去重、展示排程切換、未儲存切頁提示、測試資料驗證與設定檔寫入，未發現未處理的 JavaScript 例外。現場縮放及長時間操作仍須由使用者確認。

實作參考：[Tauri Rust commands](https://v2.tauri.app/develop/calling-rust/)、[Tauri configuration](https://v2.tauri.app/reference/config/)、[Windows prerequisites](https://v2.tauri.app/start/prerequisites/)。
