use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UninstallOptions {
    pub root_dir: String,
    pub do_services: bool,
    pub do_kill_procs: bool,
    pub do_autostart: bool,
    pub do_pm2_dumps: bool,
    pub do_remove_fe: bool,
    pub do_remove_wss: bool,
    pub do_remove_api: bool,
    pub do_remove_www: bool,
    pub do_remove_apache: bool,
    pub do_remove_node: bool,
    pub do_remove_php85: bool,
    pub do_remove_php7: bool,
    pub do_remove_instantclient: bool,
    pub do_remove_logs: bool,
    pub do_clean_root: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressPayload {
    pub percent: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogPayload {
    pub level: String, // "info", "success", "warning", "error"
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UninstallReport {
    pub success: bool,
    pub message: String,
    pub elapsed_secs: f64,
}
