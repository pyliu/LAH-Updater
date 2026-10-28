# LAH 系統原始碼自動編譯與打包工具 (pack.py)

專門用於將智慧控管與監控系統各平級子專案（`LAH-API`、`LAH-FE`、`lah-messenger-server` 等）編譯並自動封裝為標準更新壓縮包（.zip）。

---

## 📌 輸出目錄分流規範

產出之安裝包預設輸出至專案根目錄之 `dist/`，並自動依類型分流存檔：

| 分流子資料夾 | 存放安裝包 | 說明 |
|---|---|---|
| **`dist/patch/`** | `www.zip`<br/>`LAH-API.zip`<br/>`LAH-NUXTJS.zip`<br/>`lah-messenger-server.zip`<br/>`*更新說明*.txt` | 業務程式碼更新包與發布說明文件 |
| **`dist/node_modules/`** | `LAH-NUXTJS-node_modules.zip` (及別名)<br/>`lah-messenger-server-node_modules.zip` | 前端與即時通伺服端之核心依賴套件包 |
| **`dist/base/`** | `node.zip` | 可攜式 Node.js 綠色執行環境 |

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
python pack.py --output-dir <路徑>  # 自訂輸出基底目錄（亦會自動分流至 patch/、node_modules/、base/）
```
