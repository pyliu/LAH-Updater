use serde::{Deserialize, Serialize};

/// 代表解析後的地籍異動簡訊與郵件發送設定模型
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigData {
    // === 主要維護欄位（對齊 Java 原始 UI 截圖） ===
    pub ap_url: String,            // 訊息平臺網址 (ap_url, Base64)
    pub sms_user: String,          // 簡訊使用帳號 (smsUser, Base64)
    pub sms_pwd: String,           // 簡訊使用密碼 (smsPwd, Base64)
    pub mail_user: String,         // 寄件伺服器帳號 (user, Base64)
    pub mail_pwd: String,          // 寄件伺服器密碼 (pwd, Base64)
    pub mail_sender_name: String,  // 寄件者名稱 (username, Big-5 原生文字)
    pub mail_sender_addr: String,  // 寄件者郵件位置 (mailaddr, 原生字串)
    pub db_ip: String,             // 資料庫 IP (拆解自 oUrl)
    pub db_port: String,           // 資料庫 Port (拆解自 oUrl，預設 1521)
    pub db_sid: String,            // 資料庫 SID (拆解自 oUrl)
    pub db_user: String,           // 資料庫帳號 (oUser, Base64)
    pub db_pwd: String,            // 資料庫密碼 (oPwd, Base64)

    // === 進階與背景排程參數 ===
    pub search_time: String,       // 輪詢間隔 (search_time，分鐘)
    pub resend_time: String,       // 重送間隔 (resend_time，次數/分鐘)
    pub start_hours: String,       // 啟動小時 (start_hours, e.g. 08)
    pub start_min: String,         // 啟動分鐘 (start_min, e.g. 30)
    pub end_hours: String,         // 結束小時 (end_hours, e.g. 18)
    pub end_min: String,           // 結束分鐘 (end_min, e.g. 30)

    // === 背景技術參數（儲存時維持原樣或同步更新） ===
    pub odriver_class: String,     // oracle.jdbc.driver.OracleDriver
    pub d_user: String,            // dUser (如 : ;%4>)
    pub d_pwd: String,             // dPwd
    pub ap_user: String,           // apUser (Base64)
    pub ap_pwd: String,            // apPwd (Base64)
    pub section_header: String,    // e.g. "[LAND]"

    // 未知或擴充屬性保留
    pub extra_lines: Vec<String>,
}

/// 變更差異項（供儲存前預覽 Diff）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffItem {
    pub key: String,
    pub label: String,
    pub old_val: String,
    pub new_val: String,
    pub is_secret: bool,
}

/// 儲存結果回傳結構
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveResult {
    pub success: bool,
    pub message: String,
    pub backup_path: Option<String>,
}
