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
    pub deployed_exe_path: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortAuditResult {
    pub port: u16,
    pub service_name: String,
    pub is_listening: bool,
    pub leaked_headers: Vec<String>,
    pub is_secure: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditReport {
    pub system_title: String,
    pub timestamp: String,
    pub root_dir: String,
    pub sandbox_path_clean: bool,
    pub port_results: Vec<PortAuditResult>,
    pub overall_secure: bool,
    pub report_file_path: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pm2ProcessInfo {
    pub pm_id: u32,
    pub name: String,
    pub status: String,
    pub pid: u32,
    pub cpu: f32,
    pub memory: u64,
    pub uptime: u64,
    pub restarts: u32,
    pub is_module: bool,
    pub header_protection_enabled: bool,
    pub header_protection_supported: bool,
    pub version: String,
    pub exec_path: String,
    pub out_log_path: String,
    pub err_log_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleOverview {
    pub root_dir: String,
    pub installed: bool,
    pub node_version: Option<String>,
    pub pm2_version: Option<String>,
    pub service_installed: bool,
    pub service_running: bool,
    pub logrotate_installed: bool,
    pub logrotate_configured: bool,
    pub logrotate_version: Option<String>,
    pub logrotate_compress: bool,
    pub processes: Vec<Pm2ProcessInfo>,
    pub pm2_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandActionResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFileInfo {
    pub key: String,
    pub label: String,
    pub path: String,
    pub exists: bool,
    pub size_bytes: u64,
    pub modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogContentResult {
    pub path: String,
    pub lines: Vec<String>,
    pub file_size: u64,
    pub exists: bool,
}
