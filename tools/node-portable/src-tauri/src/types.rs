use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstallOptions {
    pub root_dir: String,
    pub register_service: bool,
    pub resurrect: bool,
    #[serde(default = "default_true")]
    pub install_logrotate: bool,
    #[serde(default)]
    pub register_path: bool,
    #[serde(default)]
    pub set_pm2_home: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressPayload {
    pub percent: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogPayload {
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallReport {
    pub success: bool,
    pub message: String,
    pub elapsed_secs: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanSummary {
    pub root_dir: String,
    pub zip_found: Option<String>,
    pub node_version: Option<String>,
    pub pm2_version: Option<String>,
    pub logrotate_version: Option<String>,
    pub logrotate_installed: bool,
    pub logrotate_configured: bool,
    pub service_installed: bool,
    pub service_running: bool,
}
