use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageItem {
    pub id: String,
    pub name: String,
    pub title: String,
    pub rel_target: String,
    pub is_base_env: bool,
    pub is_nodejs: bool,
    pub is_sub_package: bool,
    pub exists: bool,
    pub file_path: String,
    pub all_file_paths: Vec<String>,
    pub file_size: u64,
    pub target_path: String,
    pub target_exists: bool,
    pub checked: bool,
    pub expected_folder: String,
    pub alt_folders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootDirCheck {
    pub exists: bool,
    pub path: String,
    pub can_write: bool,
    pub is_dir: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub root_dir: String,
    pub root_exists: bool,
    pub packages: Vec<PackageItem>,
    pub apache_service: Option<String>,
    pub apache_running: bool,
    pub pm2_installed: bool,
    pub pm2_running: bool,
    pub pm2_daemon_info: String,
    pub pm2_service_installed: bool,
    pub pm2_service_running: bool,
    pub backup_zip_found: Option<String>,
    pub release_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedPackage {
    pub id: String,
    pub name: String,
    pub zip_path: String,
    pub target_path: String,
    pub is_base_env: bool,
    pub is_nodejs: bool,
    pub is_sub_package: bool,
    pub expected_folder: String,
    pub alt_folders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOptions {
    pub root_dir: String,
    pub packages: Vec<SelectedPackage>,
    pub auto_register_apache: bool,
    #[serde(default = "default_true")]
    pub auto_register_pm2: bool,
    pub auto_security_hardening: bool,
    pub restart_apache: bool,
    pub restart_pm2: bool,
    pub restore_backup: bool,
    pub backup_zip_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfigOptions {
    pub root_dir: String,
    pub auto_register_apache: bool,
    #[serde(default = "default_true")]
    pub auto_register_pm2: bool,
    pub auto_security_hardening: bool,
    pub restart_apache: bool,
    pub restart_pm2: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    pub stage: String,
    pub percent: u32,
    pub message: String,
    pub level: String, // "info", "warning", "error", "success"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSummary {
    pub success: bool,
    pub elapsed_secs: f64,
    pub processed_packages: usize,
    pub total_files: usize,
    pub health_report: Vec<String>,
    pub zips_to_clean: Vec<String>,
    pub error_message: Option<String>,
}
