#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod model;

use model::{
    filter_records, record_from_api, DesktopModel, Record, RecordList, RecordQuery, Settings,
    Snapshot,
};
use moi_sms::comm::SmsGatewayClient;
use moi_sms::config::{
    read_env_settings_file, save_env_properties_file, EnvSavePatch, MailConfig, SmsApiConfig,
};
use moi_sms::lah_client::{normalize_lah_api_url, LahApiClient, MetadataResponse, QueueEnqueueRequest};
use moi_sms::log_maintenance::{
    cleanup_logs_and_backups, read_latest_daily_log_tail, tw_date_string, DailyLogTail,
    DEFAULT_MAX_ENV_BACKUPS, DEFAULT_MAX_LOG_BYTES, LOG_COMPRESS_AFTER_DAYS,
    LOG_DELETE_AFTER_DAYS,
};
use moi_sms::scheduler::{
    current_tw_minutes, is_within_schedule_window, parse_hhmm_minutes, run_dispatch_cycle,
    DispatchConfig,
};
use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State,
};

struct DesktopState {
    model: Mutex<DesktopModel>,
    config_path: PathBuf,
    log_path: PathBuf,
    startup_warning: Option<String>,
    schedule_running: AtomicBool,
    stop_requested: AtomicBool,
    trigger_immediate: AtomicBool,
    cycle_in_progress: AtomicBool,
    allow_exit: AtomicBool,
    cycle_lock: Mutex<()>,
    cached_site_id: Mutex<Option<String>>,
}

impl DesktopState {
    fn log_event(&self, level: &str, title: &str, message: &str) {
        if let Ok(mut model) = self.model.lock() {
            model.log(level, title, message);
        }
    }
    fn lah_client(&self) -> Result<LahApiClient, String> {
        let url = {
            let model = self.model.lock().map_err(|_| "桌面狀態無法讀取")?;
            model.settings.lah_api_url.clone()
        };
        LahApiClient::new(&url).map_err(|e| format!("LAH-API 設定無效：{e:#}"))
    }

    fn fetch_table_records(&self, table: &str, limit: usize) -> Result<Vec<Record>, String> {
        let client = self.lah_client()?;
        let resp = client
            .get_records(table, limit)
            .map_err(|e| format!("無法連線 LAH-API（{table}）：{e:#}"))?;
        Ok(resp
            .records
            .iter()
            .enumerate()
            .map(|(idx, item)| record_from_api(table, idx, item))
            .collect())
    }

    fn snapshot(&self) -> Result<Snapshot, String> {
        let live_state = (|| -> Result<(usize, usize, usize, usize, usize, String, String), String> {
            let client = self.lah_client()?;
            let meta = client
                .get_metadata()
                .map_err(|e| format!("無法連線 LAH-API（metadata）：{e:#}"))?;
            let main = self.fetch_table_records("main", 500)?;
            let wait = self.fetch_table_records("wait", 500)?;
            let log = self.fetch_table_records("log", 500)?;
            let success = log.iter().filter(|r| r.status == "成功").count();
            let failure = log.iter().filter(|r| r.status == "失敗").count();
            let site_label = format!(
                "站台：{} · {}（{}）",
                meta.site_id, meta.office_name, meta.county_name
            );
            Ok((
                main.len(),
                wait.len(),
                log.len(),
                success,
                failure,
                site_label,
                meta.site_id,
            ))
        })();

        let mut model = self.model.lock().map_err(|_| "桌面狀態無法讀取")?;
        let (main_count, wait_count, log_count, success_count, failure_count, site_label) =
            match live_state {
                Ok((m, w, l, s, f, label, sid)) => {
                    model.last_counts = (m, w, l, s, f);
                    if !sid.trim().is_empty() {
                        if let Ok(mut cached) = self.cached_site_id.lock() {
                            *cached = Some(sid);
                        }
                    }
                    (m, w, l, s, f, Some(label))
                }
                Err(err) => {
                    model.log("warn", "LAH-API 連線失敗", &err);
                    let (m, w, l, s, f) = model.last_counts;
                    (m, w, l, s, f, None)
                }
            };

        Ok(Snapshot {
            settings: model.settings.clone(),
            running: model.running,
            schedule_phase: model.schedule_phase.clone(),
            main_count,
            wait_count,
            log_count,
            success_count,
            failure_count,
            activities: model.activities.clone(),
            config_path: self.config_path.display().to_string(),
            log_path: self.log_path.display().to_string(),
            startup_warning: self.startup_warning.clone(),
            site_label,
            os_autostart_enabled: is_windows_autostart_enabled(),
        })
    }

    fn build_gateway_and_dispatch_cfg(&self) -> Result<(SmsGatewayClient, DispatchConfig), String> {
        let settings = {
            let model = self.model.lock().map_err(|_| "桌面狀態無法讀取")?;
            model.settings.clone()
        };
        let site_id = self
            .cached_site_id
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .filter(|s| !s.trim().is_empty());

        let sms_cfg = SmsApiConfig {
            ap_url: settings.gateway_url.clone(),
            ap_userid: settings.sms_user.clone(),
            ap_pwd: settings.sms_password.clone(),
        };
        let gateway =
            SmsGatewayClient::new(sms_cfg).map_err(|e| format!("派送閘道設定無效：{e:#}"))?;

        let mail_cfg = MailConfig {
            smtp_host: String::new(),
            smtp_port: None,
            account: settings.mail_account,
            password: settings.mail_password,
            from_addr: settings.mail_from,
            username: settings.username,
        };
        let dispatch_cfg = DispatchConfig {
            site_id,
            max_retries: settings.max_retries,
            mail: mail_cfg,
        };
        Ok((gateway, dispatch_cfg))
    }

    fn resolve_site_id(&self, cached: Option<String>) -> Result<String, String> {
        if let Some(sid) = cached.filter(|s| !s.trim().is_empty()) {
            return Ok(sid);
        }
        let client = self.lah_client()?;
        let meta = client
            .get_metadata()
            .map_err(|e| format!("無法取得站台代碼 site_id（請確認 LAH-API 連線）：{e:#}"))?;
        let sid = meta.site_id.trim().to_string();
        if sid.is_empty() {
            return Err("LAH-API 回傳的 site_id 為空".into());
        }
        if let Ok(mut guard) = self.cached_site_id.lock() {
            *guard = Some(sid.clone());
        }
        Ok(sid)
    }
}

#[tauri::command]
fn get_snapshot(state: State<'_, Arc<DesktopState>>) -> Result<Snapshot, String> {
    state.snapshot()
}

#[tauri::command]
fn query_records(
    query: RecordQuery,
    state: State<'_, Arc<DesktopState>>,
) -> Result<RecordList, String> {
    let records = state.fetch_table_records(&query.table, 500).map_err(|err| {
        state.log_event("error", "查詢案件失敗", &err);
        err
    })?;
    let list = filter_records(&records, &query)?;
    state.log_event(
        "info",
        "查詢案件清單",
        &format!(
            "資料表={}、日期='{}'、管道='{}'、狀態='{}'、關鍵字='{}' -> 符合 {} / {} 筆。",
            query.table,
            query.date,
            query.channel,
            query.status,
            query.keyword,
            list.items.len(),
            list.total
        ),
    );
    Ok(list)
}

#[tauri::command]
fn set_schedule(running: bool, state: State<'_, Arc<DesktopState>>) -> Result<Snapshot, String> {
    state.schedule_running.store(running, Ordering::Relaxed);
    state.stop_requested.store(!running, Ordering::Relaxed);
    if running {
        state.trigger_immediate.store(true, Ordering::Relaxed);
    }
    {
        let mut model = state.model.lock().map_err(|_| "桌面狀態無法讀取")?;
        model.set_running(running);
        if !running && state.cycle_in_progress.load(Ordering::Relaxed) {
            model.schedule_phase = "停止中".into();
        }
    }
    state.snapshot()
}

fn run_manual_sync_with_client<T: moi_sms::comm::HttpTransport>(
    state: &DesktopState,
    client: &LahApiClient<T>,
) -> Result<String, String> {
    let _guard = match state.cycle_lock.try_lock() {
        Ok(guard) => guard,
        Err(std::sync::TryLockError::WouldBlock) => {
            let msg = "目前背景排程正在執行同步或派送作業，請稍候本輪完成後再手動同步。";
            state.log_event("warn", "手動同步已略過", msg);
            return Err(msg.to_string());
        }
        Err(std::sync::TryLockError::Poisoned(_)) => {
            return Err("同步作業鎖定狀態異常，請重新啟動程式。".to_string());
        }
    };

    state.log_event("info", "手動同步：步驟 1/2", "開始呼叫 LAH-API 掃描當日案件（POST /cases/sync）。");
    let sync = client.sync_cases(None).map_err(|e| {
        let err = format!("案件掃描同步失敗：{e:#}");
        state.log_event("error", "案件掃描同步失敗", &err);
        err
    })?;
    state.log_event(
        "info",
        "手動同步：步驟 2/2",
        &format!(
            "掃描完成（一般 {} / 新增 {}、機關 {} / 新增 {}），開始搬移待傳佇列（POST /queue/enqueue）。",
            sync.user_cases_scanned,
            sync.user_notifications_inserted,
            sync.gov_cases_scanned,
            sync.gov_notifications_inserted
        ),
    );
    let enqueue = client
        .enqueue_queue(&QueueEnqueueRequest::default())
        .map_err(|e| {
            let err = format!("佇列搬移失敗：{e:#}");
            state.log_event("error", "佇列搬移失敗", &err);
            err
        })?;

    let summary = format!(
        "同步日期 {}：掃描一般 {} 筆（新增 {} 筆）、機關 {} 筆（新增 {} 筆），已搬移 {} 筆至待傳佇列。",
        sync.target_date,
        sync.user_cases_scanned,
        sync.user_notifications_inserted,
        sync.gov_cases_scanned,
        sync.gov_notifications_inserted,
        enqueue.moved_to_wait_count
    );
    state
        .model
        .lock()
        .map_err(|_| "桌面狀態無法讀取")?
        .log("success", "已同步 LAH-API 案件", &summary);
    Ok(summary)
}

#[tauri::command]
async fn sync_cases(state: State<'_, Arc<DesktopState>>) -> Result<String, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let client = state.lah_client().map_err(|err| {
            state.log_event("error", "案件同步失敗", &err);
            err
        })?;
        run_manual_sync_with_client(&state, &client)
    })
    .await
    .map_err(|e| format!("手動同步背景工作異常終止：{e}"))?
}

#[tauri::command]
fn save_settings(mut settings: Settings, state: State<'_, Arc<DesktopState>>) -> Result<Snapshot, String> {
    settings.lah_api_url = normalize_lah_api_url(&settings.lah_api_url);
    settings.validate()?;
    let (start_hours, start_min) = parse_clock_parts(&settings.start_time)?;
    let (end_hours, end_min) = parse_clock_parts(&settings.end_time)?;
    let patch = EnvSavePatch {
        lah_api_url: settings.lah_api_url.trim().to_string(),
        ap_url: settings.gateway_url.trim().to_string(),
        sms_user: settings.sms_user.clone(),
        sms_pwd: settings.sms_password.clone(),
        mail_account: settings.mail_account.clone(),
        mail_pwd: settings.mail_password.clone(),
        mail_from: settings.mail_from.trim().to_string(),
        username: settings.username.trim().to_string(),
        search_minutes: settings.search_minutes,
        max_retries: settings.max_retries,
        start_hours,
        start_min,
        end_hours,
        end_min,
        auto_start_schedule: settings.auto_start_schedule,
    };

    let backup_tag = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    let backup_path = save_env_properties_file(&state.config_path, &patch, &backup_tag)
        .map_err(|e| format!("儲存 env.properties 失敗：{e:#}"))?;
    {
        let mut model = state.model.lock().map_err(|_| "桌面狀態無法讀取")?;
        let summary = format!(
            "API={}、閘道={}、簡訊帳號='{}'、郵件帳號='{}'、時段={}-{}、間隔={}分、重試={}次、自動啟動={}。",
            settings.lah_api_url,
            settings.gateway_url,
            settings.sms_user,
            settings.mail_account,
            settings.start_time,
            settings.end_time,
            settings.search_minutes,
            settings.max_retries,
            settings.auto_start_schedule
        );
        model.settings = settings;
        let msg = match backup_path {
            Some(bak) => format!(
                "設定已寫回 {}（備份 {}）。{summary}",
                state.config_path.display(),
                bak.file_name().and_then(|n| n.to_str()).unwrap_or("bak")
            ),
            None => format!("設定已寫入 {}。{summary}", state.config_path.display()),
        };
        model.log("success", "設定已儲存", &msg);
    }
    state.snapshot()
}

#[tauri::command]
fn test_lah_connection(
    url: String,
    state: State<'_, Arc<DesktopState>>,
) -> Result<MetadataResponse, String> {
    let client = LahApiClient::new(&url).map_err(|e| {
        let err = format!("LAH-API 網址無效：{e:#}");
        state.log_event("error", "LAH-API 連線測試失敗", &err);
        err
    })?;
    let meta = client.get_metadata().map_err(|e| {
        let err = format!("連線 LAH-API 失敗：{e:#}");
        state.log_event("error", "LAH-API 連線測試失敗", &err);
        err
    })?;
    let summary = format!(
        "連線成功：{} · {}（{}）［{}］",
        meta.site_id,
        meta.office_name,
        meta.county_name,
        client.base_url()
    );
    state
        .model
        .lock()
        .map_err(|_| "桌面狀態無法讀取")?
        .log("success", "LAH-API 連線測試成功", &summary);
    Ok(meta)
}

const RUN_REG_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE_NAME: &str = "MOI_SMS_Desktop";

fn reg_command(args: &[&str]) -> std::io::Result<std::process::Output> {
    let mut cmd = std::process::Command::new("reg");
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.output()
}

fn is_windows_autostart_enabled() -> bool {
    #[cfg(windows)]
    {
        reg_command(&["query", RUN_REG_KEY, "/v", RUN_VALUE_NAME])
            .is_ok_and(|out| out.status.success())
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn set_windows_autostart(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        if enabled {
            let exe = std::env::current_exe()
                .map_err(|e| format!("無法取得目前執行檔路徑：{e}"))?;
            let quoted = format!("\"{}\"", exe.display());
            let out = reg_command(&[
                "add",
                RUN_REG_KEY,
                "/v",
                RUN_VALUE_NAME,
                "/t",
                "REG_SZ",
                "/d",
                &quoted,
                "/f",
            ])
            .map_err(|e| format!("無法執行 reg.exe：{e}"))?;
            if !out.status.success() {
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                return Err(format!("寫入開機自動啟動失敗：{err}"));
            }
        } else if is_windows_autostart_enabled() {
            let out = reg_command(&["delete", RUN_REG_KEY, "/v", RUN_VALUE_NAME, "/f"])
                .map_err(|e| format!("無法執行 reg.exe：{e}"))?;
            if !out.status.success() {
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                return Err(format!("清除開機自動啟動失敗：{err}"));
            }
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Err("僅支援 Windows 系統設定開機自動啟動".into())
    }
}

#[tauri::command]
fn set_os_autostart(
    enabled: bool,
    state: State<'_, Arc<DesktopState>>,
) -> Result<Snapshot, String> {
    set_windows_autostart(enabled)?;
    state.log_event(
        "info",
        if enabled {
            "開機自動啟動已啟用"
        } else {
            "開機自動啟動已清除"
        },
        if enabled {
            "已將本程式註冊至 Windows 登入啟動項目（HKCU\\...\\Run）。"
        } else {
            "已從 Windows 登入啟動項目移除本程式。"
        },
    );
    state.snapshot()
}

#[tauri::command]
fn read_current_log(
    max_lines: Option<usize>,
    state: State<'_, Arc<DesktopState>>,
) -> Result<DailyLogTail, String> {
    let limit = max_lines.unwrap_or(300).clamp(50, 2000);
    read_latest_daily_log_tail(
        &state.log_path,
        current_epoch_ms(),
        DEFAULT_MAX_LOG_BYTES,
        limit,
    )
    .map_err(|e| format!("讀取日誌檔失敗：{e:#}"))
}

#[tauri::command]
fn reload_env_properties(state: State<'_, Arc<DesktopState>>) -> Result<Snapshot, String> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from));
    let env_path = if state.config_path.exists() {
        state.config_path.clone()
    } else {
        find_env_properties_path(exe_dir.as_deref())
            .ok_or_else(|| "找不到 env.properties（請先儲存設定建立該檔案，或確認執行檔同目錄存在該檔案）".to_string())?
    };
    let mut loaded = settings_from_env_properties(&env_path)?;
    {
        let mut model = state.model.lock().map_err(|_| "桌面狀態無法讀取")?;
        if !loaded.lah_api_url.is_empty() && loaded.lah_api_url == Settings::default().lah_api_url {
            loaded.lah_api_url = model.settings.lah_api_url.clone();
        }
        model.settings = loaded;
        model.log(
            "success",
            "已重新讀取 env.properties",
            &format!("已從 {} 載入設定。", env_path.display()),
        );
    }
    state.snapshot()
}

#[tauri::command]
fn test_delivery(
    channel: String,
    recipient: String,
    message: String,
    state: State<'_, Arc<DesktopState>>,
) -> Result<String, String> {
    {
        let model = state.model.lock().map_err(|_| "桌面狀態無法讀取")?;
        model.validate_delivery_input(&channel, &recipient, &message)?;
    }

    let (gateway, dispatch_cfg) = state.build_gateway_and_dispatch_cfg()?;
    let site_id = state.resolve_site_id(dispatch_cfg.site_id.clone())?;
    let trimmed_recipient = recipient.trim();
    state.log_event(
        "info",
        "開始發送測試訊息",
        &format!("管道={channel}、收件者={trimmed_recipient}、site_id={site_id}。"),
    );

    let result = match channel.as_str() {
        "sms" => gateway.send_sms_gateway(&site_id, trimmed_recipient, &message),
        "email" => gateway.send_email_gateway(
            &site_id,
            &dispatch_cfg.mail,
            "地籍異動系統測試通知",
            trimmed_recipient,
            &message,
        ),
        _ => return Err("未知的傳送管道".into()),
    };

    match result {
        Ok(res) => {
            let summary = format!(
                "測試訊息已送出至 {}（desc={}）。",
                trimmed_recipient,
                if res.desc.trim().is_empty() {
                    "OK"
                } else {
                    res.desc.trim()
                }
            );
            state.log_event("success", "傳送測試成功", &summary);
            Ok(summary)
        }
        Err(err) => {
            let msg = format!("測試訊息發送失敗：{err:#}");
            state.log_event("error", "傳送測試失敗", &msg);
            Err(msg)
        }
    }
}

fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn run_log_cleanup_with_summary(
    log_dir: &std::path::Path,
    env_dir: Option<&std::path::Path>,
    now_ms: u64,
) -> Option<(&'static str, &'static str, String)> {
    let report = cleanup_logs_and_backups(
        log_dir,
        env_dir,
        now_ms,
        LOG_COMPRESS_AFTER_DAYS,
        LOG_DELETE_AFTER_DAYS,
        DEFAULT_MAX_ENV_BACKUPS,
    );
    let has_actions = report.compressed_count > 0
        || report.deleted_logs_count > 0
        || report.deleted_zips_count > 0
        || report.deleted_backups_count > 0;
    if !has_actions && report.errors.is_empty() {
        return None;
    }

    let mut summary = format!(
        "壓縮舊日誌 {} 份、刪除過期日誌 {} 份、刪除過期壓縮檔 {} 份、清理過期設定備份 {} 份。",
        report.compressed_count,
        report.deleted_logs_count,
        report.deleted_zips_count,
        report.deleted_backups_count,
    );
    if !report.errors.is_empty() {
        summary.push_str(&format!("部分項目失敗：{}", report.errors.join("；")));
        return Some(("warning", "日誌清理完成（部分失敗）", summary));
    }
    Some(("info", "日誌清理完成", summary))
}

fn spawn_scheduler_loop(state: Arc<DesktopState>) {
    std::thread::spawn(move || {
        let mut last_run: Option<Instant> = None;
        let mut last_outside_window_logged = false;
        let mut last_cleanup_date = tw_date_string(current_epoch_ms());

        loop {
            std::thread::sleep(Duration::from_secs(2));

            let now_ms = current_epoch_ms();
            let today = tw_date_string(now_ms);
            if today != last_cleanup_date {
                last_cleanup_date = today;
                if let Some((level, title, msg)) = run_log_cleanup_with_summary(
                    &state.log_path,
                    state.config_path.parent(),
                    now_ms,
                ) {
                    state.log_event(level, title, &msg);
                }
            }

            if !state.schedule_running.load(Ordering::Relaxed) {
                last_run = None;
                last_outside_window_logged = false;
                continue;
            }

            let (start_str, end_str, search_minutes) = {
                let Ok(model) = state.model.lock() else {
                    continue;
                };
                (
                    model.settings.start_time.clone(),
                    model.settings.end_time.clone(),
                    model.settings.search_minutes.max(1) as u64,
                )
            };

            let in_window = match (
                parse_hhmm_minutes(&start_str),
                parse_hhmm_minutes(&end_str),
            ) {
                (Some(s), Some(e)) => is_within_schedule_window(current_tw_minutes(), s, e),
                _ => false,
            };

            if !in_window {
                if !last_outside_window_logged {
                    if let Ok(mut model) = state.model.lock() {
                        model.schedule_phase = "等待執行時段".into();
                        model.log(
                            "info",
                            "等待執行時段",
                            &format!("目前不在排程執行時段（{start_str}–{end_str}）內，暫停自動派送。"),
                        );
                    }
                    last_outside_window_logged = true;
                }
                continue;
            }

            last_outside_window_logged = false;
            let force_now = state.trigger_immediate.swap(false, Ordering::Relaxed);
            let interval = Duration::from_secs(search_minutes * 60);
            let due = force_now
                || last_run
                    .map(|prev| prev.elapsed() >= interval)
                    .unwrap_or(true);

            if !due {
                continue;
            }

            let Ok(_guard) = state.cycle_lock.lock() else {
                continue;
            };
            state.cycle_in_progress.store(true, Ordering::Relaxed);
            if let Ok(mut model) = state.model.lock() {
                model.schedule_phase = "執行中".into();
                model.log(
                    "info",
                    "排程開始本輪作業",
                    &format!("位於執行時段（{start_str}–{end_str}）內，間隔 {search_minutes} 分鐘。"),
                );
            }

            let cycle_res = (|| -> Result<(), String> {
                let lah = state.lah_client()?;
                let (gateway, dispatch_cfg) = state.build_gateway_and_dispatch_cfg()?;
                let summary =
                    run_dispatch_cycle(&lah, &gateway, &dispatch_cfg, &state.stop_requested)
                        .map_err(|e| format!("{e:#}"))?;

                if let Ok(mut model) = state.model.lock() {
                    for entry in &summary.item_logs {
                        model.log(&entry.level, &entry.title, &entry.message);
                    }
                    model.log(
                        if summary.failed_count > 0 || summary.report_error_count > 0 {
                            "warn"
                        } else {
                            "success"
                        },
                        "排程本輪作業結束",
                        &format!(
                            "新增一般 {} / 機關 {} 筆，搬移 {} 筆，待傳 {} 筆（成功 {}、失敗 {}、回報異常 {}、提前中止={}）。",
                            summary.sync.user_notifications_inserted,
                            summary.sync.gov_notifications_inserted,
                            summary.enqueue.moved_to_wait_count,
                            summary.pending_count,
                            summary.sent_count,
                            summary.failed_count,
                            summary.report_error_count,
                            summary.stopped_early
                        ),
                    );
                }
                Ok(())
            })();

            if let Err(err) = cycle_res {
                state.log_event("error", "排程執行異常", &err);
            }

            last_run = Some(Instant::now());
            state.cycle_in_progress.store(false, Ordering::Relaxed);
            if !state.schedule_running.load(Ordering::Relaxed) {
                if let Ok(mut model) = state.model.lock() {
                    model.schedule_phase = "已暫停".into();
                }
            }
        }
    });
}

/// 固定優先讀寫執行檔同目錄的 `env.properties`（單一事實來源），
/// 若無法取得執行檔目錄才退回 `app_config_dir/env.properties`。
fn resolve_env_save_path(app_config_dir: PathBuf, exe_dir: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = exe_dir {
        return dir.join("env.properties");
    }
    app_config_dir.join("env.properties")
}

/// 僅於 `env.properties` 尚未建立時，向下相容尋找舊版 `settings.json` / `desktop-demo.json`。
fn find_legacy_json_path(
    app_config_dir: &std::path::Path,
    exe_dir: Option<&std::path::Path>,
) -> Option<PathBuf> {
    if let Some(dir) = exe_dir {
        let portable = dir.join("settings.json");
        if portable.exists() {
            return Some(portable);
        }
    }
    let primary = app_config_dir.join("settings.json");
    if primary.exists() {
        return Some(primary);
    }
    let legacy = app_config_dir.join("desktop-demo.json");
    legacy.exists().then_some(legacy)
}

fn parse_clock_parts(value: &str) -> Result<(u8, u8), String> {
    let trimmed = value.trim();
    let (h_str, m_str) = trimmed
        .split_once(':')
        .ok_or_else(|| format!("時間格式須為 HH:MM: {trimmed}"))?;
    let h: u8 = h_str
        .parse()
        .map_err(|_| format!("小時格式錯誤: {trimmed}"))?;
    let m: u8 = m_str
        .parse()
        .map_err(|_| format!("分鐘格式錯誤: {trimmed}"))?;
    if h >= 24 || m >= 60 {
        return Err(format!("時間數值超出範圍: {trimmed}"));
    }
    Ok((h, m))
}

fn resolve_log_path(app_log_dir: PathBuf, exe_dir: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = exe_dir {
        let portable_dir = dir.join("logs");
        if fs::create_dir_all(&portable_dir).is_ok() {
            let probe = portable_dir.join(".write_probe");
            if fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&probe)
                .is_ok()
            {
                let _ = fs::remove_file(&probe);
                return portable_dir;
            }
        }
    }
    let _ = fs::create_dir_all(&app_log_dir);
    app_log_dir
}

fn find_env_properties_path(exe_dir: Option<&std::path::Path>) -> Option<PathBuf> {
    if let Some(dir) = exe_dir {
        let candidate = dir.join("env.properties");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    let cwd_candidate = PathBuf::from("env.properties");
    cwd_candidate.exists().then_some(cwd_candidate)
}

fn format_clock(hours: Option<u8>, mins: Option<u8>, fallback: &str) -> String {
    match (hours, mins) {
        (Some(h), Some(m)) if h < 24 && m < 60 => format!("{h:02}:{m:02}"),
        _ => fallback.to_string(),
    }
}

fn settings_from_env_properties(path: &std::path::Path) -> Result<Settings, String> {
    let cfg = read_env_settings_file(path).map_err(|e| format!("讀取 env.properties 失敗：{e:#}"))?;
    let mut settings = Settings::default();
    if let Some(url) = cfg.lah_api_url {
        settings.lah_api_url = normalize_lah_api_url(&url);
    }
    if let Some(ap_url) = cfg.ap_url.filter(|s| !s.trim().is_empty()) {
        settings.gateway_url = ap_url.trim().to_string();
    }
    settings.sms_user = cfg.sms_user;
    settings.sms_password = cfg.sms_pwd;
    settings.mail_account = cfg.mail_account;
    settings.mail_password = cfg.mail_pwd;
    if !cfg.mail_from.trim().is_empty() {
        settings.mail_from = cfg.mail_from.trim().to_string();
    }
    if !cfg.username.trim().is_empty() {
        settings.username = cfg.username.trim().to_string();
    }
    if let Some(search) = cfg.search_minutes {
        if (1..=1440).contains(&search) {
            settings.search_minutes = search as u32;
        }
    }
    if let Some(resend) = cfg.max_retries {
        if resend <= 99 {
            settings.max_retries = resend as u32;
        }
    }
    settings.start_time = format_clock(cfg.start_hours, cfg.start_min, &settings.start_time);
    settings.end_time = format_clock(cfg.end_hours, cfg.end_min, &settings.end_time);
    if let Some(auto_start) = cfg.auto_start_schedule {
        settings.auto_start_schedule = auto_start;
    }
    settings.validate()?;
    Ok(settings)
}

fn load_initial_settings(
    env_path: &std::path::Path,
    legacy_json_path: Option<&std::path::Path>,
) -> (Settings, Option<String>) {
    if env_path.exists() {
        return match settings_from_env_properties(env_path) {
            Ok(s) => (s, None),
            Err(e) => (Settings::default(), Some(e)),
        };
    }

    let Some(json_path) = legacy_json_path else {
        return (Settings::default(), None);
    };

    match fs::read(json_path) {
        Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes)
            .map_err(|_| "舊版設定檔 JSON 格式不符".to_string())
            .and_then(|mut settings| {
                settings.lah_api_url = normalize_lah_api_url(&settings.lah_api_url);
                settings.validate()?;
                Ok(settings)
            }) {
            Ok(settings) => (settings, None),
            Err(error) => (
                Settings::default(),
                Some(format!(
                    "無法載入舊版桌面設定，已改用預設值。{error}。"
                )),
            ),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (Settings::default(), None),
        Err(_) => (
            Settings::default(),
            Some("無法讀取舊版桌面設定，已改用預設值。".into()),
        ),
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "顯示視窗", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "關閉程式", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .tooltip("智慧控管與監控系統 - 地籍異動即時通簡訊控管")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => {
                if let Some(state) = app.try_state::<Arc<DesktopState>>() {
                    state.allow_exit.store(true, Ordering::Relaxed);
                    state.log_event("info", "關閉程式", "由系統匣右鍵選單結束桌面程式。");
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => show_main_window(tray.app_handle()),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(PathBuf::from));
            let app_cfg_dir = app.path().app_config_dir()?;
            let config_path = resolve_env_save_path(app_cfg_dir.clone(), exe_dir.clone());
            let legacy_json = find_legacy_json_path(&app_cfg_dir, exe_dir.as_deref());
            let log_path = resolve_log_path(app.path().app_log_dir()?, exe_dir);
            let (settings, startup_warning) =
                load_initial_settings(&config_path, legacy_json.as_deref());
            let auto_start = settings.auto_start_schedule;
            let mut initial_model = DesktopModel::with_log_path(settings, Some(log_path.clone()));
            if let Some((level, title, msg)) =
                run_log_cleanup_with_summary(&log_path, config_path.parent(), current_epoch_ms())
            {
                initial_model.log(level, title, &msg);
            }
            if auto_start {
                initial_model.set_running(true);
            }
            let state = Arc::new(DesktopState {
                model: Mutex::new(initial_model),
                config_path,
                log_path,
                startup_warning,
                schedule_running: AtomicBool::new(auto_start),
                stop_requested: AtomicBool::new(!auto_start),
                trigger_immediate: AtomicBool::new(auto_start),
                cycle_in_progress: AtomicBool::new(false),
                allow_exit: AtomicBool::new(false),
                cycle_lock: Mutex::new(()),
                cached_site_id: Mutex::new(None),
            });
            app.manage(state.clone());
            setup_tray(app.handle())?;
            spawn_scheduler_loop(state);
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let should_exit = window
                    .try_state::<Arc<DesktopState>>()
                    .is_some_and(|state| state.allow_exit.load(Ordering::Relaxed));
                if !should_exit {
                    api.prevent_close();
                    let _ = window.hide();
                    if let Some(state) = window.try_state::<Arc<DesktopState>>() {
                        state.log_event(
                            "info",
                            "視窗已最小化至系統匣",
                            "程式持續於背景執行；可由右下角系統匣圖示右鍵選單關閉程式。",
                        );
                    }
                }
            }
            tauri::WindowEvent::Destroyed => {
                let should_exit = window
                    .try_state::<Arc<DesktopState>>()
                    .is_some_and(|state| state.allow_exit.load(Ordering::Relaxed));
                if should_exit {
                    window.app_handle().exit(0);
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            query_records,
            set_schedule,
            sync_cases,
            save_settings,
            set_os_autostart,
            read_current_log,
            reload_env_properties,
            test_lah_connection,
            test_delivery
        ])
        .run(tauri::generate_context!())
        .expect("無法啟動 MOI 桌面程式");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn resolve_config_paths_prefers_portable_and_falls_back_to_legacy() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("moi_cfg_test_{unique}"));
        let app_dir = root.join("appdata");
        let exe_dir = root.join("exe");
        fs::create_dir_all(&app_dir).unwrap();
        fs::create_dir_all(&exe_dir).unwrap();

        // 1. 固定優先使用執行檔同目錄的 env.properties
        let target_env = resolve_env_save_path(app_dir.clone(), Some(exe_dir.clone()));
        assert_eq!(target_env, exe_dir.join("env.properties"));

        // 2. 當 env.properties 尚未建立時，向下相容讀取舊版 desktop-demo.json 並自動升級 PHP 8.5 路徑
        let legacy = app_dir.join("desktop-demo.json");
        fs::write(&legacy, r#"{"lah_api_url":"http://10.0.0.5/api/sms_v1_api.php"}"#).unwrap();
        let found_legacy = find_legacy_json_path(&app_dir, Some(&exe_dir));
        assert_eq!(found_legacy.as_deref(), Some(legacy.as_path()));
        let (loaded, warn) = load_initial_settings(&target_env, found_legacy.as_deref());
        assert!(warn.is_none());
        assert_eq!(loaded.lah_api_url, "http://10.0.0.5/LAH-API/v1/sms_v1_api.php");
        assert!(!loaded.auto_start_schedule);

        // 3. 若 env.properties 存在，則以 env.properties 為單一事實來源（優先於舊 JSON）
        fs::write(
            &target_env,
            "ap_url=aHR0cDovLzIyMC4xLjMzLjEyMjo4MDgwL0lNU1AvT05FU2VydmljZVNlcnZsZXQ=\nsmsUser=dXNlcjI=\nsmsPwd=cHdkMg==\nsearch_time=5\nresend_time=4\nstart_hours=9\nstart_min=30\nend_hours=17\nend_min=45\nauto_start_schedule=true\n",
        )
        .unwrap();
        let (from_env, warn) = load_initial_settings(&target_env, found_legacy.as_deref());
        assert!(warn.is_none(), "unexpected warning: {warn:?}");
        assert_eq!(
            from_env.gateway_url,
            "http://220.1.33.122:8080/IMSP/ONEServiceServlet"
        );
        assert_eq!(from_env.sms_user, "user2");
        assert_eq!(from_env.sms_password, "pwd2");
        assert_eq!(from_env.search_minutes, 5);
        assert_eq!(from_env.max_retries, 4);
        assert_eq!(from_env.start_time, "09:30");
        assert_eq!(from_env.end_time, "17:45");
        assert!(from_env.auto_start_schedule);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn manual_sync_runs_two_steps_with_sync_timeout_and_rejects_overlap() {
        use moi_sms::anyhow::Result as AnyResult;
        use moi_sms::comm::HttpTransport;
        use moi_sms::lah_client::{DEFAULT_LAH_TIMEOUT, SYNC_LAH_TIMEOUT};
        use std::cell::RefCell;
        use std::collections::VecDeque;

        struct ScriptedSyncTransport {
            responses: RefCell<VecDeque<AnyResult<String>>>,
            calls: Arc<Mutex<Vec<(String, Duration)>>>,
        }

        impl HttpTransport for ScriptedSyncTransport {
            fn get(&self, _url: &str, _timeout: Duration) -> AnyResult<String> {
                unreachable!("manual sync 不呼叫 GET")
            }

            fn post_form(
                &self,
                _url: &str,
                _params: &[(&str, &str)],
                _timeout: Duration,
            ) -> AnyResult<String> {
                unreachable!("manual sync 不呼叫 form POST")
            }

            fn post_json(
                &self,
                url: &str,
                _body: &serde_json::Value,
                timeout: Duration,
            ) -> AnyResult<String> {
                self.calls.lock().unwrap().push((url.to_string(), timeout));
                self.responses
                    .borrow_mut()
                    .pop_front()
                    .unwrap_or_else(|| moi_sms::anyhow::bail!("no response queued"))
            }
        }

        let state = DesktopState {
            model: Mutex::new(DesktopModel::with_log_path(Settings::default(), None)),
            config_path: PathBuf::from("env.properties"),
            log_path: PathBuf::from("logs"),
            startup_warning: None,
            schedule_running: AtomicBool::new(false),
            stop_requested: AtomicBool::new(true),
            trigger_immediate: AtomicBool::new(false),
            cycle_in_progress: AtomicBool::new(false),
            allow_exit: AtomicBool::new(false),
            cycle_lock: Mutex::new(()),
            cached_site_id: Mutex::new(None),
        };

        let recorded_calls = Arc::new(Mutex::new(Vec::new()));
        let transport = ScriptedSyncTransport {
            responses: RefCell::new(VecDeque::from([
                Ok(r#"{"target_date":"1151113","user_cases_scanned":4,"user_notifications_inserted":2,"gov_cases_scanned":1,"gov_notifications_inserted":1}"#.to_string()),
                Ok(r#"{"moved_to_wait_count":3,"deleted_from_main_count":3}"#.to_string()),
            ])),
            calls: recorded_calls.clone(),
        };
        let client = LahApiClient::with_timeouts(
            "http://127.0.0.1/LAH-API/v1/sms_v1_api.php",
            transport,
            DEFAULT_LAH_TIMEOUT,
            SYNC_LAH_TIMEOUT,
        )
        .unwrap();

        // 1. 排程正在佔用 cycle_lock 時應立即拒絕重複同步
        {
            let _held = state.cycle_lock.lock().unwrap();
            let err = run_manual_sync_with_client(&state, &client).unwrap_err();
            assert!(err.contains("正在執行"), "unexpected error: {err}");
        }
        assert!(recorded_calls.lock().unwrap().is_empty());

        // 2. 釋放鎖定後應依序完成 POST /cases/sync 與 POST /queue/enqueue，並使用 SYNC_LAH_TIMEOUT
        let summary = run_manual_sync_with_client(&state, &client).unwrap();
        assert!(summary.contains("1151113") && summary.contains("搬移 3 筆"), "summary={summary}");
        assert_eq!(
            *recorded_calls.lock().unwrap(),
            vec![
                (
                    "http://127.0.0.1/LAH-API/v1/sms_v1_api.php/cases/sync".to_string(),
                    SYNC_LAH_TIMEOUT
                ),
                (
                    "http://127.0.0.1/LAH-API/v1/sms_v1_api.php/queue/enqueue".to_string(),
                    SYNC_LAH_TIMEOUT
                ),
            ]
        );
        let model = state.model.lock().unwrap();
        assert!(model.activities.iter().any(|a| a.title == "手動同步已略過"));
        assert!(model.activities.iter().any(|a| a.title == "已同步 LAH-API 案件"));
    }
}
