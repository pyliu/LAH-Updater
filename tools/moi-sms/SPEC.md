# MOI_EX 與 SMS 系統逆向分析規格書（SPEC）

> **分析標的**：
> 1. [MOI_EX.exe](file:///r:/exchange/@TBD/1151113%20MOI_SMS/MOI_EX.exe)（exe4j 封裝）與內嵌主程式 [MOI_EX.jar](file:///r:/exchange/@TBD/1151113%20MOI_SMS/MOI_EX.jar)
> 2. [SMS.exe](file:///r:/exchange/@TBD/1151113%20MOI_SMS/SMS.exe)（exe4j 封裝）與內嵌主程式 [SMS.jar](file:///r:/exchange/@TBD/1151113%20MOI_SMS/SMS.jar)
>
> **分析依據**：REA PE 靜態分析（`inspect-pe-imports`、`inspect-pe-resources`、`inspect-pe-certificates`、`inspect-binary-hardening`）、OpenJDK 22 `javap.exe` 位元組碼反組譯，以及 JAR / Overlay 結構檢視。

---

## 第一部分：MOI_EX.exe / MOI_EX.jar（案件資料同步批次程式）

## 1. 程式類型與進入點參數

### 1.1 程式類型
- **純背景／主控台批次程式（Console / Background Batch）**
- **無 GUI 視窗**：應用程式核心類別（`MOIEx`、`DeleteTempDir`、`filterFile`、`util.*`）均未引用 `java.awt.*` 或 `javax.swing.*`。
- **執行期封裝行為**：外層由 **exe4j**（64 位元 `PE32+`，stub 名稱 `jredl`）啟動，將內嵌的 `MOI_EX.jar` 解壓至暫存目錄後以無視窗或背景主控台行程執行，並將標準輸出（stdout）與標準錯誤（stderr）分別重新導向至工作目錄的 `output.log` 與 `error.log`。

### 1.2 進入點與命令列參數
- **主進入點**：`MOIEx.main(java.lang.String[])`（亦為 `META-INF/MANIFEST.MF` 指定之 `Main-Class: MOIEx`）。
  - **命令列參數**：`MOIEx.main` **不讀取 `args[]`**，執行時不接受任何 CLI 參數或旗標；所有執行期組態皆由工作目錄檔案讀取。
- **輔助進入點**：`DeleteTempDir.main(java.lang.String[])`（暫存目錄清理工具）。
  - 若 `args.length == 0`，預設掃描並清理 `c:/Documents and Settings`；若有傳入 `args`，則逐一對傳入路徑呼叫 `DeleteTempDir.startProcess(String)`。
- **第三方套件進入點**：JAR 內因 Fat Jar Eclipse Plug-In 打包而包含 33 個第三方套件 `main` 類別（如 `com.ibm.db2.jcc.DB2Jcc`、`net.sourceforge.jtds.jdbc.Driver`、`javassist.CtClass`、`tw.com.chttl.CertUtil`、`oracle.*`、`sqlj.*` 等），與本系統主流程無關。

---

## 2. 外部連線與本機檔案 I/O

### 2.1 資料庫連線方式
系統內建兩套資料庫連線類別，主程式實際僅使用 `util.ODatabase`：

| 項目 | 主程式實際使用（`util.ODatabase`） | 備用／舊版實作（`util.Database`，未被 `MOIEx` 呼叫） |
| :--- | :--- | :--- |
| **組態來源** | 相對路徑 `env.properties` | 相對路徑 `TPCEx.properties` |
| **讀取屬性鍵** | `odriver_class`、`oUrl`、`oUser`、`oPwd` | `sdriver_class`、`sUrl`、`sUser`、`sPwd` |
| **憑證編碼** | `oUrl`、`oUser`、`oPwd` 經 Base64 編碼，讀取後呼叫 `util.Common.getFromBASE64(String)` 解碼 | 明文讀取（未做 Base64 解碼） |
| **連線機制** | **HikariCP**（`com.zaxxer.hikari.HikariConfig` / `HikariDataSource`） | 原生 `Class.forName(cName)` + `java.sql.DriverManager.getConnection(...)` |
| **連線池參數** | `minimumIdle = 1`、`maximumPoolSize = 5000`、`autoCommit = true`、`readOnly = false` | 單一 `Connection` 實例 |
| **測試查詢與目標資料庫** | `connectionTestQuery = "SELECT current date FROM sysibm.sysdummy1"`（**IBM DB2** 專用測試語法） | 依 `sdriver_class` 而定 |

### 2.2 HTTP / Socket / 外部 API 端點
- **`MOIEx` 主流程無任何外部 HTTP / Socket / REST / SMS API 呼叫**。
  - 位元組碼追蹤顯示 `MOIEx` 僅進行資料庫查詢與寫入（`SSMSMAIN` 資料表），實際簡訊／郵件發送推測由同目錄另一支執行檔 [SMS.exe](file:///r:/exchange/@TBD/1151113%20MOI_SMS/SMS.exe) 接手處理。
- **未使用之工具方法與套件**：
  - `util.Common.getRemoteData(String)` 實作了 `java.net.URL.openConnection()` 與 `BufferedReader` 讀取遠端文字內容之功能，但整個 `MOIEx` 與 `util.*` 均未呼叫該方法。
  - `tw.com.chttl.*`（中華電信憑證／Token／OCSP 套件）雖打包於 JAR 內，但 `MOIEx` 與 `util.*` 均未引用。

### 2.3 讀寫之本機檔案路徑
所有檔案路徑皆相對於啟動時的工作目錄（Working Directory）：

| 路徑 | 讀／寫 | 對應類別與方法 | 說明 |
| :--- | :---: | :--- | :--- |
| `env.properties` | 讀取 | `util.ODatabase.readProperty()` | 讀取資料庫驅動類別與 Base64 編碼之連線資訊 |
| `<YYYMMDD>moniter_sms.log` | 附加寫入 | `MOIEx.WriteLog(String)` | 以民國年月日（`util.Datetime.getYYYMMDD()`，如 `1151113moniter_sms.log`）為前綴，透過 `FileWriter(file, true)` 附加寫入執行紀錄 |
| `output.log` / `error.log` | 寫入 | `exe4j` 啟動器 Overlay 組態 | 重新導向標準輸出與標準錯誤 |
| `TPCEx.properties` | 讀取 | `util.Database.readProperty()` | 僅於呼叫 `util.Database` 時讀取（主流程未使用） |
| `c:/Documents and Settings`（或 `args`） | 刪除檔案 | `DeleteTempDir.main(String[])` | 輔助清理類別（主流程未呼叫） |

---

## 3. 核心資料處理流程、加解密邏輯與字元編碼

### 3.1 核心批次處理流程（`MOIEx.main`）
1. **啟動與連線**：印出啟動時間（`util.Datetime.getHHMMSS()`），建立 `util.ODatabase` 並呼叫 `getConnection()` 初始化 HikariCP 連線池取得 DB2 `Connection`。
2. **一般案件查詢與寫入**：
   - 呼叫 `MOIEx.queryCaseInfo(ODatabase)` 查詢相關案件資料（結合 `SSMS_USERINFO_M`、`SCRSMS`、`SSMS_CODE06`、`SRKEYN` 等資料表），回傳 `Vector`。
   - 逐筆呼叫 `MOIEx.CheckData(String[], Connection)` 檢查 `SSMSMAIN` 是否已存在重複案件；若無重複則執行 `INSERT INTO SSMSMAIN(MS03, MS04_1, MS04_2, MS07_1, MS07_2, MS14, MS30, MS_TYPE, MS_NOTE)`（或包含 `MS_MAIL` 欄位之變體），並呼叫 `WriteLog(String)` 寫入 `<YYYMMDD>moniter_sms.log`。
3. **政府機關案件查詢與寫入**：
   - 呼叫 `MOIEx.queryGCaseInfo(ODatabase)` 查詢機關案件（結合 `SSMS_GOVINFO_D`、`SMS_GOVINFO_M`、`srsindx`、`SCRSMS`、`SCRSMS_RM1215`、`SSMS_CODE06`、`SRKEYN`）。
   - 逐筆呼叫 `MOIEx.CheckGData(String[], Connection)` 檢查並寫入 `SSMSMAIN`。
4. **收尾**：於 `finally` 區段呼叫 `ODatabase.shutdown()` 與 `ODatabase.closeAll()` 關閉連線池與資源，並印出結束時間。

### 3.2 加解密與雜湊邏輯
- **Base64 解碼（`util.Common.getFromBASE64(String)`）**：
  - 使用 `sun.misc.BASE64Decoder.decodeBuffer(String)` 對 `env.properties` 中的 `oUrl`、`oUser`、`oPwd` 進行解碼（屬可逆編碼／混淆，非強加密）。
- **MD5 雜湊（`util.Common.getMD5PassWord(String)`）**：
  - 使用 `java.security.MessageDigest.getInstance("MD5")` 計算輸入字串之 MD5 十六進位摘要（`MOIEx` 主流程未呼叫）。
- **外層 exe4j 封裝混淆**：
  - `MOI_EX.exe` Overlay 中的 `MOI_EX.jar` 以單一位元組 `0x88` XOR 編碼儲存，執行時由 exe4j stub 解碼還原。

### 3.3 字元編碼機制（Big5 / ISO-8859-1）
- **資料庫欄位的 Big5 ↔ ISO-8859-1 雙向轉換（明確存在，使用 `"big5"` 而非 `"MS950"` 字串）**：
  - **讀取時**：`MOIEx` 在 `queryCaseInfo`、`queryGCaseInfo`、`CheckData`、`CheckGData` 等方法中，共計 **33 處**呼叫 `util.Common.isoToBig5(String)`。該方法執行 `str.getBytes("ISO-8859-1")` 後再以 `new String(bytes, "big5")` 轉成正確中文，顯示 DB2 資料表欄位以 `ISO-8859-1` 編碼存放 Big5 位元組。
  - **寫入時**：在 `CheckData` 與 `CheckGData` 組合 `INSERT INTO SSMSMAIN` SQL 前，共計 **4 處**呼叫 `util.Common.big5ToIso(String)`，執行 `str.getBytes("big5")` 後以 `new String(bytes, "ISO-8859-1")` 轉回資料庫相容編碼。
  - **備註**：`util.Common.isoToutf8(String)` 雖名為 UTF-8，但位元組碼實際內容與 `isoToBig5` 完全相同（皆為 `ISO-8859-1` → `big5`）；另有 `util.Common.encodeUTF8(String)` 使用 `"UTF-8"`。
- **Log 檔與 Base64 解碼預設編碼**：
  - `MOIEx.WriteLog(String)` 使用 `java.io.FileWriter(file, true)`、`util.Common.getFromBASE64(String)` 使用 `new String(bytes)`，兩者皆未指定字元集，會沿用 JVM 執行環境的預設編碼（Windows 繁體中文環境通常為 `MS950` / `Big5`，或依 `-Dfile.encoding` 設定而定）。

---

## 4. 功能模組與原始類別／方法對照表

| 功能模組 | 原始類別 | 關鍵方法／欄位證據 |
| :--- | :--- | :--- |
| **程式進入點與批次排程控制** | `MOIEx` | `main(String[])` |
| **一般案件查詢與寫入檢查** | `MOIEx` | `queryCaseInfo(ODatabase)`、`CheckData(String[], Connection)`、`getUser_Info(Connection)` |
| **機關案件查詢與寫入檢查** | `MOIEx` | `queryGCaseInfo(ODatabase)`、`CheckGData(String[], Connection)`、`getSMS_GOVInfo(Connection)` |
| **通用 SQL 查詢與更新** | `MOIEx` | `queryData(String, Connection)`、`updateData(String, Connection)` |
| **執行紀錄輸出（Log）** | `MOIEx` | `WriteLog(String)`（寫入 `<YYYMMDD>moniter_sms.log`） |
| **檔案／資料夾刪除** | `MOIEx`、`DeleteTempDir`、`filterFile` | `MOIEx.delFolder(String)`、`MOIEx.delAllFile(String)`；`DeleteTempDir.main(String[])`、`startProcess(String)`、`deleteDir(String, String)` |
| **DB2 連線池與組態讀取（主用）** | `util.ODatabase` | `readProperty()`（讀 `env.properties`）、`init()`（`HikariDataSource` + `sysibm.sysdummy1`）、`getConnection()`、`querySQL(...)`、`exeSQL(...)`、`shutdown()`、`closeAll()` |
| **傳統 JDBC 連線（備用）** | `util.Database` | `readProperty()`（讀 `TPCEx.properties`）、`getConnection()`（`DriverManager.getConnection`） |
| **字元編碼、Base64 與 HTTP 工具** | `util.Common` | `isoToBig5(String)`、`big5ToIso(String)`、`isoToutf8(String)`、`encodeUTF8(String)`、`getFromBASE64(String)`、`getMD5PassWord(String)`、`getRemoteData(String)` |
| **民國日期／時間格式化** | `util.Datetime` | `getYYYMMDD()`、`getHHMMSS()`、`getDateDiff(...)`、`DateSubtraction(...)` |
| **全域訊息暫存** | `util.MsgUtil` | `setMsg(String)`、`getMsg()` |


---

## 第二部分：SMS.exe / SMS.jar（簡訊與郵件派送主程式）

### 0. 封裝結構與基本資訊
- **打包工具**：同樣由 **exe4j**（64 位元 `PE32+`，imphash `de361320741c3eefff5c87665ab4f6f4`）封裝，Overlay 起始於 `0x7e600`，內嵌 `SMS.jar`（`7,036,094` bytes，SHA-256 `2266a9c660f0d415916d3c501bf122e01e6313697152ae10ec0d509b13407234`，以 `0x88` XOR 編碼）。
- **JAR 結構**：由 Fat Jar Eclipse Plug-In 打包，共 `96` 個 packages、`1,746` 個 `.class` 檔；應用程式核心套件為 `sms`（29 個類別）與 `util`（8 個類別），打包之第三方套件包含 `oracle.*`（400）、`javassist.*`（398）、`org.*`（254，含 `org.apache.commons.httpclient` 3.1）、`com.alibaba.fastjson.*`（186）、`javax.*`（134，含 `javax.mail`）、`com.codahale.metrics.*`（85）、`com.zaxxer.hikari.*`（77）。

### 1. 程式類型與進入點參數
- **程式類型**：**Java Swing GUI 桌面應用程式**（含排程背景 `java.util.Timer`）。
  - 視窗介面類別包含 `sms.Login`、`sms.SmsMainFrame`、`sms.InitSms`、`sms.SmsChangePwd`、`sms.SmsAboutBox`、`util.ClockFrame`（皆繼承 `javax.swing.JFrame` 或 Swing 元件）。
- **進入點與命令列參數**：
  - **主進入點**：`sms.Login.main(java.lang.String[])`（亦為 `META-INF/MANIFEST.MF` 與 exe4j Overlay 指定之 `Main-Class: sms.Login`）。使用者登入驗證通過後啟動 `sms.SmsApplication.main(String[])` 並開啟 `sms.SmsMainFrame`。
  - **其他應用程式進入點**：`sms.InitSms.main(String[])`（環境與郵件組態設定視窗）、`sms.SmsApplication.main(String[])`（主視窗啟動器）。
  - **命令列參數**：上述 `main(String[])` 均不依賴外部 CLI 旗標參數，組態由工作目錄檔案讀取或透過 GUI 表單設定。

### 2. 外部連線與本機檔案 I/O

#### 2.1 資料庫連線方式
- **主用連線池（`util.ODatabase`）**：與 `MOI_EX.jar` 相同，透過 `util.ODatabase.readProperty()` 讀取 `env.properties` 中的 `odriver_class`、`oUrl`、`oUser`、`oPwd`（經 `util.Common.getFromBASE64` 解碼），並以 `com.zaxxer.hikari.HikariDataSource` 建立連線池。
  - **注意**：`SMS.jar` 內打包的 JDBC 驅動僅有 `oracle.*`（Oracle JDBC），未像 `MOI_EX.jar` 同時打包 DB2 JCC 與 jTDS。
- **備用連線（`util.Database`）**：保留以 `DriverManager.getConnection` 讀取 `TPCEx.properties` 的實作。

#### 2.2 HTTP / SMTP 外部 API 端點
1. **簡訊閘道 HTTP API（`sms.SmsMainFrame`）**：
   - 使用 **Apache Commons HttpClient 3.1**（`org.apache.commons.httpclient.HttpClient`、`PostMethod`，timeout 設為 `4000` ms）。
   - 端點 URL 來源：自 `env.properties` 讀取 `ap_url`（經 Base64 解碼），搭配 `ap_userid`、`ap_pwd` 進行驗證。
   - **兩種發送模式**：
     - `SendMsg(...)`（傳統表單 POST）：以 `NameValuePair` 傳遞 `ap_userid`、`ap_pwd`、收件門號與 Base64 編碼的 `UTF-8` 簡訊內容。
     - `SendNewMsg(...)`（JSON POST）：先查詢資料庫 `SELECT krmk from srkeyn where kcde_1='55' and kcde_2='01'` 取得 `siteID`，再以 `com.alibaba.fastjson.JSONObject` 組裝含 `siteID` 等欄位之 JSON 請求送往 `ap_url`。
2. **SMTP 郵件伺服器（`sms.SmsMainFrame`）**：
   - `sendMail(...)` / `SendMailMsg(...)` 使用 `javax.mail` 與 `com.sun.mail.util.MailSSLSocketFactory`。
   - 連線設定讀取自 `env.properties` / `mail.properties`：`mail_stmp`（SMTP 主機）、`mail_port`、`mail_acct`、`mail_pwd`（Base64 解碼）、`mail_from`（`mailaddr`），並啟用 `mail.transport.protocol=smtp`、`mail.smtp.auth=true`、`mail.smtp.ssl.trust=mail_server`。

#### 2.3 讀寫之本機檔案路徑
| 路徑 | 讀／寫 | 對應類別與方法 | 說明 |
| :--- | :---: | :--- | :--- |
| `env.properties` | 讀／寫 | `SmsMainFrame.readenv()`、`changeValueByPropertyName()`、`InitSms.readenv()`、`util.ODatabase.readProperty()` | 儲存資料庫、`ap_url`、郵件帳號與排程時間參數（`SmsMainFrame` 以 `BIG5` 編碼讀寫） |
| `mail.properties` | 讀取 | `SmsMainFrame.readMailenv()` | 讀取 `mail_stmp`、`mail_acct`、`mail_pwd`、`mailaddr`、`username` |
| `<YYYMMDD>sms.log` | 附加寫入 | `SmsMainFrame.WriteLog(String)` | 記錄發送歷程（對應目錄下 `1151113sms.log`） |
| `<YYYMMDD>sms.out` | 附加寫入 | `SmsMainFrame.WriteOut(String, boolean)` | 記錄計時器查詢輸出（對應目錄下 `1151113sms.out`） |
| `<YYYMMDD>memo.log` | 附加寫入 | `SmsMainFrame.WriteMemo(String)` | 記錄備忘與異常訊息（對應目錄下 `1151113memo.log`） |
| `output.log` / `error.log` | 寫入 | `exe4j` 啟動器 Overlay 組態 | 重新導向標準輸出與標準錯誤 |

### 3. 核心資料處理流程、加解密與字元編碼
- **排程輪詢與派送流程**：
  1. `sms.Login` 驗證帳號密碼後開啟 `sms.SmsMainFrame`。
  2. `SmsMainFrame` 載入 `env.properties`（`search_time`、`resend_time`、`start_hours`/`min`、`end_hours`/`min`）與資料表檢視（`rebuildMainTable`、`rebuildWAITTable`、`rebuildLOGTable`）。
  3. 透過 `java.util.Timer` 驅動 `TTSearchTimer()` 與 `TTSendTimer()`，定期掃描資料庫待發送資料表，依類型呼叫 `SendNewMsg` / `SendMsg`（簡訊）或 `SendMailMsg` / `sendMail`（Email），並以 `UpdSmswait`、`DelSmsWait`、`InsSmsLog` 更新狀態與寫入 Log。
- **加解密／編碼**：
  - 組態憑證（`oUrl`、`oUser`、`oPwd`、`ap_url`、`ap_userid`、`ap_pwd`、`mail_stmp`、`mail_acct`、`mail_pwd`）皆使用 Base64 編碼（`util.Common.getFromBASE64` / `sun.misc.BASE64Encoder`）。
  - `SendMsg` 送出簡訊內文前，會將 `UTF-8` 位元組以 `sun.misc.BASE64Encoder` 編碼。
- **字元編碼（Big5 / ISO-8859-1 / UTF-8）**：
  - `env.properties` 讀寫明確指定 **`"BIG5"`**（`InputStreamReader(..., "BIG5")`、`OutputStreamWriter(..., "BIG5")`）。
  - 資料庫欄位讀寫與 `MOI_EX.jar` 一致，大量使用 `util.Common.isoToBig5(String)` 及 `getBytes("ISO-8859-1")` ↔ `"Big5"` 轉換（未出現 `"MS950"` 字串）。
  - HTTP 簡訊傳輸則使用 `"UTF-8"`。

### 4. 功能模組與原始類別／方法對照表

| 功能模組 | 原始類別 | 關鍵方法／欄位證據 |
| :--- | :--- | :--- |
| **登入驗證與主程式啟動** | `sms.Login`、`sms.SmsApplication` | `Login.main(String[])`、`Login.readProperty()`、`SmsApplication.main(String[])` |
| **組態精靈介面** | `sms.InitSms`、`sms.SmsChangePwd` | `InitSms.main(String[])`、`readenv()`、`changeValueByPropertyName()` |
| **主控台 GUI 與排程輪詢** | `sms.SmsMainFrame` | `TTSearchTimer()`、`TTSendTimer()`、`rebuildMainTable()`、`rebuildWAITTable()`、`rebuildLOGTable()` |
| **簡訊 HTTP API 派送** | `sms.SmsMainFrame` | `SendNewMsg(...)`（`fastjson` + `HttpClient`）、`SendMsg(...)`（`PostMethod` + `NameValuePair` + `BASE64Encoder`） |
| **Email SMTP 派送** | `sms.SmsMainFrame` | `SendMailMsg(...)`、`sendMail(...)`（`javax.mail` + `MailSSLSocketFactory`） |
| **佇列狀態與紀錄寫回 DB** | `sms.SmsMainFrame` | `UpdSmswait(...)`、`DelSmsWait(...)`、`InsSmsLog(...)`、`updateData(String)` |
| **本機組態與 Log 讀寫** | `sms.SmsMainFrame` | `readenv()`、`readMailenv()`、`WriteLog(String)`、`WriteOut(String, boolean)`、`WriteMemo(String)` |
| **資料庫連線與工具** | `util.ODatabase`、`util.Database`、`util.Common`、`util.Datetime` | `ODatabase.init()`、`Common.isoToBig5`、`Common.getFromBASE64`、`Datetime.getYYYMMDD()` |
