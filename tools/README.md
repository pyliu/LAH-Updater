# LAH-Updater Tools（獨立維運小工具套件庫）

本目錄收納使用 Rust (Tauri 2) 開發之各類獨立日常維運小工具，所有工具均遵循以下共通規範：

---

## 1. 架構原則

1. **技術棧一致性**：採用 Tauri 2 (Rust) + 純靜態 HTML/CSS/JS 前端架構，零外部運行庫依賴。
2. **Zero-CDN**：全工具介面嚴禁外連 CDN 或遠端字型，以 inline SVG 與標準字型相容封閉隔離內網。
3. **單一免安裝執行檔**：編譯產出直接輸出至 `LAH-Updater/dist/`，隨點秒開、冷啟動小於 0.3 秒。
4. **獨立目錄結構**：每個工具為一獨立子資料夾，包含 `src-tauri/`、`ui/`、`build.py` 與 `build.bat`。

---

## 2. 現有工具清單

| 目錄 | 工具名稱 | 主要職責與特點 |
|---|---|---|
| `sms-env-editor/` | 地籍異動簡訊參數設定工具 | 編輯與儲存 `\\220.1.34.221\C$\SMSWEB_MOI\env.properties`，支援 BASE64 自動加解密、Big-5 編碼相容與自動時間戳備份。 |

---

## 3. 新增小工具流程

1. 於 `tools/` 下建立新工具資料夾（如 `tools/<tool-name>/`）。
2. 初始化 Tauri 2 專案架構（`src-tauri/` 與 `ui/`）。
3. 配置 `.cargo/config.toml` 將 target-dir 指向 `C:/Temp/cargo-target/<tool-name>` 避免工作區膨脹。
4. **專屬圖示設計與確認**：嚴禁複製或沿用既有工具圖示。必須為新工具量身設計專屬意象，動手前主動提出至少 2 至 3 組視覺方案供使用者選擇，定案後以 `generate_icons.ps1` 產出 16~256 多解析度 `icon.ico` 與標準 PNG 套裝。
5. 編寫 `build.py` 將編譯產物統一發布至 `LAH-Updater/dist/`。
6. 於根目錄提供對應之 `建立<工具名稱>.bat` 快捷指令。
