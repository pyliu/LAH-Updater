# 地籍異動簡訊參數設定工具 (SMS-Env-Editor)

內政部地籍異動簡訊與郵件發送服務（SMSWEB_MOI）設定檔 `env.properties` 之專用可視化編輯工具。

---

## 1. 核心特點

- **架構**：採用 Tauri 2 (Rust) + 純靜態 HTML/CSS/JS 前端，單一免安裝執行檔（約 9~12 MB），隨點秒開。
- **預設路徑**：預設載入 `\\220.1.34.221\C$\SMSWEB_MOI\env.properties`，支援切換本機與網路檔案。
- **BASE64 自動加解密**：平臺網址、簡訊帳密、郵件帳密、Oracle 資料庫帳密與連線 URL 自動於載入時解密、於儲存時加密。
- **Big-5 與 CRLF 無損相容**：繁體中文（如「桃園地政事務所」）採用 Big-5 / CP950 編碼儲存，完美相容 Java 舊版程式。
- **防呆與自動備份**：
  - 儲存前跳出變更差異（Diff）確認彈窗。
  - 寫入前自動於同目錄建立時間戳備份（`env.properties.YYYYMMDD_HHMMSS.bak`）。
  - 密碼欄位支援 👁️ 顯隱切換防呆。
  - 背景自動保留 `odriver_class`、`search_time` 等未顯示之技術參數。

---

## 2. 編譯打包

```powershell
python build.py
# 或執行 build.bat
```

編譯完成後執行檔將自動發布至 `LAH-Updater/dist/地籍異動簡訊參數設定工具.exe`。
