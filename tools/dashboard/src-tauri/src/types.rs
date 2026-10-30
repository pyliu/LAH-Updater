use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub root_dir: String,
    pub is_valid_root: bool,
    pub has_apache: bool,
    pub has_pm2: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemServiceStatus {
    pub name: String,
    pub display_name: String,
    pub status: String,
    pub is_running: bool,
    pub is_installed: bool,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortStatus {
    pub port: u16,
    pub name: String,
    pub is_listening: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostMetrics {
    pub cpu_usage: f32,
    pub mem_total: u64,
    pub mem_used: u64,
    pub mem_load_pct: u32,
    pub disk_total: u64,
    pub disk_free: u64,
    pub disk_used_pct: u32,
    pub ports: Vec<PortStatus>,
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
    pub key: String,
    pub path: String,
    pub lines: Vec<String>,
    pub file_size: u64,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandActionResult {
    pub success: bool,
    pub message: String,
}
