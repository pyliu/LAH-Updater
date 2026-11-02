# LAH 系統原始碼自動編譯與打包工具 (pack.py)

專門用於將智慧控管與監控系統各平級子專案（`LAH-API`、`LAH-FE`、`lah-messenger-server` 等）編譯並自動封裝為標準更新壓縮包（.zip），同時支援自動自系統收集 VC++ 執行期程式庫封裝為 `vcredist-x64.zip`。

---

## 📌 輸出目錄分流規範

產出之安裝包預設輸出至專案根目錄之 `dist/`，並自動依類型分流存檔：

| 分流子資料夾 | 存放安裝包 | 說明 |
|---|---|---|
| **`dist/patch/`** | `www.zip`<br/>`LAH-API.zip`<br/>`LAH-NUXTJS.zip`<br/>`lah-messenger-server.zip`<br/>`*更新說明*.txt` | 業務程式碼更新包與發布說明文件 |
| **`dist/node_modules/`** | `LAH-NUXTJS-node_modules.zip`<br/>`lah-messenger-server-node_modules.zip` | 前端與即時通伺服端之核心依賴套件包 |
| **`dist/base/`** | `node.zip`<br/>`vcredist-x64.zip` | 可攜式 Node.js 綠色執行環境與 VC++ 2015–2022 獨立執行期依賴 |

---

## 🚀 開發機發布資料夾自動增量同步

打包作業順利完成後，工具預設自動將 `dist/` 內之成品增量同步至開發機發布資料夾：
- **發布基底路徑**：`d:\DEV\@各所分享區\@智慧管控系統\@更新程式\`（定時自動同步至公用 NAS）。
- **版號子目錄**：依據 `LAH-FE/package.json` 中的 `version` 欄位建立（例如 `v1.9.9`）。
- **副檔名白名單**：僅限 `*.txt`、`*.pdf`、`*.exe`、`*.zip`（排除 `*.jpg` 等輔助圖解）。
- **防呆與略過**：若本機未偵測到發布基底資料夾，或傳入 `--skip-copy` 參數，則自動略過複製步驟，不影響打包流程。

---

## 🛠️ 使用方式

```powershell
# 1. 互動選單模式（雙擊或執行批次檔）
.\打包智慧控管及監控系統.bat
# 或執行：
python pack.py

# 2. 常用命令列參數
python pack.py --api-only           # 僅打包 LAH-API PHP7 (patch/www.zip)
python pack.py --api8-only          # 僅打包 LAH-API PHP8 (patch/LAH-API.zip)
python pack.py --fe-only            # 僅編譯並打包 LAH-FE (patch/LAH-NUXTJS.zip)
python pack.py --messenger-only     # 僅打包即時通伺服器 (patch/lah-messenger-server.zip)
python pack.py --modules-only       # 僅打包前端與即時通依賴套件至 node_modules/
python pack.py --node-only          # 僅打包 Node.js 綠色環境至 base/
python pack.py --vcredist-only      # 僅收集本機 VC++ 執行期封裝為 base/vcredist-x64.zip
python pack.py --skip-copy          # 略過將成品複製至開發機發布資料夾的步驟
python pack.py --output-dir <路徑>  # 自訂輸出基底目錄（亦會自動分流至 patch/、node_modules/、base/）
```
