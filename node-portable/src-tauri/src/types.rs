use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct InstallOptions {
    pub root_dir: String,
    pub register_path: bool,
    pub set_pm2_home: bool,
    pub register_service: bool,
    pub resurrect: bool,
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
    pub service_installed: bool,
    pub service_running: bool,
}
