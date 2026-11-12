use crate::install::STANDALONE_CONSOLE_EXE_NAME;
use crate::pm2::SERVICE_ID;
use crate::system::{is_admin, CREATE_NO_WINDOW};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::os::windows::{fs::MetadataExt, process::CommandExt};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard};

static OPERATION: Mutex<()> = Mutex::new(());

pub fn operation_guard() -> Result<MutexGuard<'static, ()>, String> {
    OPERATION
        .try_lock()
        .map_err(|_| "另一項沙盒作業正在執行，請稍後再試。".into())
}

pub fn waiting_operation_guard() -> Result<MutexGuard<'static, ()>, String> {
    OPERATION
        .lock()
        .map_err(|_| "沙盒作業狀態異常，請重新啟動工具。".into())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemovalProcess {
    pub pid: u32,
    pub parent_pid: u32,
    pub name: String,
    pub executable: String,
    pub created: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemovalPlan {
    pub root_dir: String,
    pub service_installed: bool,
    pub processes: Vec<RemovalProcess>,
    pub remove: Vec<String>,
    pub preserve: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(PartialEq, Eq)]
struct RemovalScope<'a> {
    root_dir: &'a str,
    service_installed: bool,
    remove: &'a [String],
    preserve: &'a [String],
    blockers: &'a [String],
}

impl RemovalPlan {
    fn scope(&self) -> RemovalScope<'_> {
        RemovalScope {
            root_dir: &self.root_dir,
            service_installed: self.service_installed,
            remove: &self.remove,
            preserve: &self.preserve,
            blockers: &self.blockers,
        }
    }
}

#[derive(Debug, Deserialize)]
struct Service {
    name: String,
    executable: String,
    configuration: String,
    running: bool,
    pid: u32,
    #[serde(default)]
    config_node: String,
    #[serde(default)]
    config_home: String,
    #[serde(default)]
    config_runner: String,
    #[serde(default)]
    command_line: String,
}

#[derive(Debug, Default, Deserialize)]
struct Inventory {
    services: Vec<Service>,
    processes: Vec<RemovalProcess>,
}

fn normalized(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace('/', "\\")
        .trim_start_matches(r"\\?\")
        .trim_end_matches('\\')
        .to_lowercase()
}

fn mentions_path(text: &str, path: &Path) -> bool {
    let text = normalized(text);
    let target = normalized(&path.to_string_lossy());
    text.match_indices(&target).any(|(index, _)| {
        let before = text[..index].chars().next_back();
        let after = text[index + target.len()..].chars().next();
        !before.is_some_and(|c| c.is_alphanumeric() || matches!(c, '\\' | ':' | '_' | '-'))
            && !after.is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
    })
}

fn plain_path(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().trim_start_matches(r"\\?\"))
}

fn reject_link(path: &Path) -> Result<fs::Metadata, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|e| format!("無法檢查 {}：{e}", path.display()))?;
    if metadata.file_attributes() & 0x400 != 0 {
        return Err(format!("拒絕移除連結或 Junction：{}", path.display()));
    }
    Ok(metadata)
}

fn validate_root(root_dir: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(root_dir.trim());
    if !path.is_absolute()
        || path.parent().is_none()
        || !matches!(path.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), std::path::Prefix::Disk(_)))
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err("請指定本機磁碟上的完整安裝目錄，不可使用磁碟根目錄或相對路徑。".into());
    }
    for ancestor in path.ancestors() {
        reject_link(ancestor)?;
    }
    let root = plain_path(&fs::canonicalize(&path).map_err(|e| e.to_string())?);
    if !root.is_dir() {
        return Err("安裝根目錄不存在。".into());
    }
    if let Ok(windows) = std::env::var("SystemRoot") {
        if normalized(&root.to_string_lossy()).starts_with(&normalized(&windows)) {
            return Err("不可移除 Windows 系統目錄中的環境。".into());
        }
    }
    Ok(root)
}

fn check_tree(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.to_string()),
        Ok(_) => {}
    }
    if reject_link(path)?.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            check_tree(&entry.map_err(|e| e.to_string())?.path())?;
        }
    }
    Ok(())
}

fn host(request: serde_json::Value) -> Result<String, String> {
    let powershell = PathBuf::from(std::env::var("SystemRoot").map_err(|e| e.to_string())?)
        .join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let output = Command::new(powershell)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            include_str!("uninstall_host.ps1"),
        ])
        .env("LAH_NODE_UNINSTALL_REQUEST", request.to_string())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "Windows 作業失敗：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_start_matches('\u{feff}')
        .trim()
        .to_string())
}

fn inventory() -> Result<Inventory, String> {
    read_inventory(&host)
}

fn read_inventory(
    run: &dyn Fn(serde_json::Value) -> Result<String, String>,
) -> Result<Inventory, String> {
    serde_json::from_str(&run(json!({"action": "inventory"}))?)
        .map_err(|e| format!("無法讀取服務與程序清單：{e}"))
}

fn make_plan(root: &Path, inventory: &Inventory, admin: bool) -> RemovalPlan {
    let node = root.join("node");
    let expected_service = node.join(r"service\pm2-service.exe");
    let mut plan = RemovalPlan {
        root_dir: root.to_string_lossy().into_owned(),
        service_installed: false,
        processes: vec![],
        remove: vec![],
        preserve: vec![
            root.join(".pm2").display().to_string() + "（既有快照、設定與日誌）",
            root.join("logs").display().to_string() + "（服務及業務日誌）",
            root.join(STANDALONE_CONSOLE_EXE_NAME).display().to_string(),
            "所有受管應用程式、業務資料、備份與安裝包".into(),
        ],
        blockers: vec![],
    };
    if !admin {
        plan.blockers
            .push("請以系統管理員身分重新開啟工具。".into());
    }
    for service in &inventory.services {
        if service.name == SERVICE_ID {
            if normalized(&service.executable) != normalized(&expected_service.to_string_lossy()) {
                plan.blockers
                    .push(format!("{SERVICE_ID} 屬於其他目錄：{}", service.executable));
            } else {
                plan.service_installed = true;
                let exe = normalized(&node.join("node.exe").to_string_lossy());
                let home = normalized(&root.join(".pm2").to_string_lossy());
                let runner = normalized(
                    &node
                        .join(r"service\pm2_service_runner.js")
                        .to_string_lossy(),
                );
                if normalized(&service.config_node) != exe
                    || normalized(&service.config_home) != home
                    || normalized(service.config_runner.trim().trim_matches('"')) != runner
                {
                    plan.blockers
                        .push("PM2 服務設定不屬於此沙盒或無法確認，已阻擋移除。".into());
                }
            }
        } else if mentions_path(&service.executable, &node)
            || mentions_path(&service.command_line, &node)
            || mentions_path(&service.command_line, &root.join(".pm2"))
            || (service.name == "LAH_PM2_Service" && mentions_path(&service.executable, root))
            || mentions_path(&service.configuration, &node)
            || mentions_path(&service.configuration, &root.join(".pm2"))
        {
            plan.blockers
                .push(format!("{} 共用此沙盒，移除會使該服務失效。", service.name));
        }
    }
    if node.exists() {
        let pm2 = fs::read_to_string(node.join(r"node_modules\pm2\package.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
        if !plan.service_installed
            && pm2
                .as_ref()
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                != Some("pm2")
        {
            plan.blockers
                .push("無法確認 node 目錄為此工具的 Node／PM2 環境，已阻擋移除。".into());
        }
        plan.remove.push(node.display().to_string());
    }
    let helper = root.join("lah-cmd.bat");
    if helper.exists() {
        match fs::read_to_string(&helper) {
            Ok(content)
                if content.contains("set \"NODE_DIR=%ROOT_DIR%\\node\"")
                    && content.contains("set \"PM2_HOME=%ROOT_DIR%\\.pm2\"") =>
            {
                plan.remove.push(helper.display().to_string());
            }
            _ => plan
                .preserve
                .push(helper.display().to_string() + "（自訂或無法辨識的終端）"),
        }
    }
    let node_prefix = normalized(&node.to_string_lossy()) + "\\";
    let mut selected: HashSet<u32> = inventory
        .processes
        .iter()
        .filter(|p| normalized(&p.executable).starts_with(&node_prefix))
        .map(|p| p.pid)
        .collect();
    // Capture children before stopping their parents, including non-Node interpreters.
    loop {
        let count = selected.len();
        for process in &inventory.processes {
            if selected.contains(&process.parent_pid) {
                if let Some(parent) = inventory
                    .processes
                    .iter()
                    .find(|p| p.pid == process.parent_pid)
                {
                    if !parent.created.is_empty() && process.created >= parent.created {
                        selected.insert(process.pid);
                    }
                }
            }
        }
        if selected.len() == count {
            break;
        }
    }
    plan.processes = inventory
        .processes
        .iter()
        .filter(|p| selected.contains(&p.pid))
        .cloned()
        .collect();
    plan.processes.sort_by_key(|p| p.pid);
    for process in &inventory.processes {
        if process.name.eq_ignore_ascii_case("node.exe") && process.executable.is_empty() {
            plan.blockers.push(format!(
                "無法確認 Node 程序 PID {} 的路徑，已阻擋移除。",
                process.pid
            ));
        }
    }
    if plan
        .processes
        .iter()
        .any(|p| p.executable.is_empty() || p.created.is_empty() || p.pid == std::process::id())
    {
        plan.blockers
            .push("無法安全確認受影響程序，或工具本身位於 node 目錄內，已阻擋移除。".into());
    }
    for service in &inventory.services {
        if service.name != SERVICE_ID && service.running && selected.contains(&service.pid) {
            plan.blockers.push(format!(
                "程序由其他 Windows 服務 {} 使用，已阻擋移除。",
                service.name
            ));
        }
    }
    if !plan.service_installed && plan.remove.is_empty() {
        plan.blockers.push("此目錄沒有可移除的沙盒元件。".into());
    }
    plan.blockers.sort();
    plan.blockers.dedup();
    plan
}

fn prepare(root: &Path, data: &Inventory, admin: bool) -> Result<RemovalPlan, String> {
    check_tree(&root.join("node"))?;
    if root.join("lah-cmd.bat").exists() {
        reject_link(&root.join("lah-cmd.bat"))?;
    }
    for name in [".pm2", "logs"] {
        // Reject nested links as well: retained logs must never resolve into the deletion tree.
        check_tree(&root.join(name))?;
        if let Ok(resolved) = fs::canonicalize(root.join(name)) {
            if mentions_path(&resolved.to_string_lossy(), &root.join("node")) {
                return Err(format!("保留資料 {name} 位於 node 目錄內，已阻擋移除。"));
            }
        }
    }
    for snapshot in ["dump.pm2", "dump.pm2.bak"] {
        let path = root.join(".pm2").join(snapshot);
        if path.exists() {
            let dump: Vec<serde_json::Value> =
                serde_json::from_str(&fs::read_to_string(&path).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("無法檢查既有快照 {}：{e}", path.display()))?;
            for app in dump {
                let bundled_module = app
                    .get("pm_exec_path")
                    .and_then(|v| v.as_str())
                    .is_some_and(|p| {
                        normalized(p)
                            == normalized(
                                &root
                                    .join(r"node\node_modules\pm2-logrotate\app.js")
                                    .to_string_lossy(),
                            )
                    });
                for field in [
                    "pm_exec_path",
                    "pm_cwd",
                    "cwd",
                    "pm_out_log_path",
                    "pm_err_log_path",
                    "pm_log_path",
                ] {
                    if bundled_module && matches!(field, "pm_exec_path" | "pm_cwd" | "cwd") {
                        continue;
                    }
                    if app.get(field).and_then(|v| v.as_str()).is_some_and(|p| {
                        let path = Path::new(p);
                        let resolved =
                            fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
                        mentions_path(&resolved.to_string_lossy(), &root.join("node"))
                    }) {
                        return Err(format!("快照中的應用或日誌（{field}）位於 node 目錄內，請先搬移資料，再移除沙盒。"));
                    }
                }
            }
        }
    }
    Ok(make_plan(root, data, admin))
}

pub fn preview(root_dir: &str) -> Result<RemovalPlan, String> {
    let root = validate_root(root_dir)?;
    prepare(&root, &inventory()?, is_admin())
}

fn remove_tree(path: &Path) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.to_string()),
        Ok(_) => reject_link(path)?,
    };
    if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            remove_tree(&entry.map_err(|e| e.to_string())?.path())?;
        }
        fs::remove_dir(path)
    } else {
        if metadata.permissions().readonly() {
            let mut permissions = metadata.permissions();
            permissions.set_readonly(false);
            fs::set_permissions(path, permissions).map_err(|e| e.to_string())?;
        }
        fs::remove_file(path)
    }
    .map_err(|e| {
        format!(
            "無法移除 {}：{e}。已停止後續作業；保留資料不受影響。",
            path.display()
        )
    })
}

pub fn execute(
    plan: RemovalPlan,
    log: &dyn Fn(&str, &str),
    progress: &dyn Fn(f64, &str),
) -> Result<String, String> {
    execute_with_host(plan, is_admin(), &host, log, progress)
}

fn execute_with_host(
    plan: RemovalPlan,
    admin: bool,
    run: &dyn Fn(serde_json::Value) -> Result<String, String>,
    log: &dyn Fn(&str, &str),
    progress: &dyn Fn(f64, &str),
) -> Result<String, String> {
    let root = validate_root(&plan.root_dir)?;
    let current = prepare(&root, &read_inventory(run)?, admin)?;
    if !current.blockers.is_empty() {
        return Err(current.blockers.join("\n"));
    }
    if current.scope() != plan.scope() {
        return Err("預覽後環境已變更，請重新預覽並確認移除清單。".into());
    }
    let service_exe = root.join(r"node\service\pm2-service.exe");
    log(&format!("開始移除沙盒：{}", root.display()), "info");
    progress(10.0, "停止指定沙盒服務...");
    run(json!({"action": "stop-service", "service_exe": service_exe}))?;
    progress(30.0, "停止此沙盒的程序與子程序...");
    run(json!({"action": "stop-processes", "processes": current.processes}))?;
    let remaining = read_inventory(run)?;
    let checked = prepare(&root, &remaining, true)?;
    if !checked.blockers.is_empty() || !checked.processes.is_empty() {
        return Err("仍有程序使用此沙盒，或服務設定已變更；已停止移除，請重新預覽。".into());
    }
    progress(55.0, "解除指定沙盒的 Windows 服務...");
    run(json!({"action": "delete-service", "service_exe": service_exe}))?;
    check_tree(&root.join("node"))?;
    progress(70.0, "移除 Node／PM2 與沙盒終端...");
    for target in &current.remove {
        log(&format!("移除：{target}"), "info");
        remove_tree(Path::new(target))?;
    }
    crate::console::invalidate_pm2_cache();
    let message = "沙盒移除完成。已保留應用程式、.pm2 快照、設定、日誌與工具執行檔。";
    log(message, "success");
    progress(100.0, message);
    Ok(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn waiting_operation_waits_for_console_read_and_excludes_mutations() {
        let held = OPERATION.lock().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _guard = waiting_operation_guard().unwrap();
            tx.send(()).unwrap();
            release_rx.recv().unwrap();
        });
        let premature = rx.recv_timeout(std::time::Duration::from_millis(50));
        drop(held);
        assert!(
            premature.is_err(),
            "terminal and preview operations must wait for a running console read"
        );
        rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap();
        let mutation_blocked = operation_guard().is_err();
        release_tx.send(()).unwrap();
        worker.join().unwrap();
        assert!(mutation_blocked, "waiting operations must retain mutation exclusion");
    }

    #[test]
    fn normal_process_turnover_uses_the_current_inventory() {
        let fixture = Fixture::new();
        let preview = Inventory {
            processes: vec![process(101, 0, &fixture.0.join(r"node\node.exe"))],
            ..Inventory::default()
        };
        let current_process = process(202, 0, &fixture.0.join(r"node\node.exe"));
        let inventories = RefCell::new(0);
        let stopped = RefCell::new(vec![]);
        let result = execute_with_host(
            fixture.plan(&preview),
            true,
            &|request| match request["action"].as_str().unwrap() {
                "inventory" => {
                    *inventories.borrow_mut() += 1;
                    let processes = if *inventories.borrow() == 1 {
                        vec![current_process.clone()]
                    } else {
                        vec![]
                    };
                    Ok(json!({"services":[], "processes": processes}).to_string())
                }
                "stop-processes" => {
                    *stopped.borrow_mut() =
                        serde_json::from_value::<Vec<RemovalProcess>>(request["processes"].clone())
                            .unwrap();
                    Ok(String::new())
                }
                _ => Ok(String::new()),
            },
            &|_, _| {},
            &|_, _| {},
        );
        assert!(
            result.is_ok(),
            "normal process turnover must not invalidate removal: {result:?}"
        );
        assert_eq!(*stopped.borrow(), vec![current_process]);
        assert!(!fixture.0.join("node").exists());
    }

    #[test]
    fn new_shared_service_after_preview_blocks_before_stopping() {
        let fixture = Fixture::new();
        let result = execute_with_host(
            fixture.plan(&Inventory::default()),
            true,
            &|request| {
                assert_eq!(
                    request["action"], "inventory",
                    "a new shared service must prevent all mutation"
                );
                Ok(json!({"services":[{
                    "name":"LAH_PM2_Service", "executable":fixture.0.join("lah-service.exe"),
                    "configuration":"", "running":true, "pid":123
                }], "processes":[]})
                .to_string())
            },
            &|_, _| {},
            &|_, _| {},
        );
        assert!(result.unwrap_err().contains("共用此沙盒"));
        assert!(fixture.0.join(r"node\node.exe").exists());
    }

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let root = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .join(format!(
                    "lah-node-uninstall-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
            fs::create_dir_all(root.join(r"node\node_modules\pm2")).unwrap();
            fs::write(
                root.join(r"node\node_modules\pm2\package.json"),
                r#"{"name":"pm2"}"#,
            )
            .unwrap();
            fs::write(root.join(r"node\node.exe"), b"runtime fixture").unwrap();
            fs::write(
                root.join("lah-cmd.bat"),
                "set \"NODE_DIR=%ROOT_DIR%\\node\"\nset \"PM2_HOME=%ROOT_DIR%\\.pm2\"",
            )
            .unwrap();
            Self(plain_path(&fs::canonicalize(root).unwrap()))
        }
        fn plan(&self, data: &Inventory) -> RemovalPlan {
            prepare(&self.0, data, true).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn process(pid: u32, parent: u32, path: &Path) -> RemovalProcess {
        RemovalProcess {
            pid,
            parent_pid: parent,
            name: "node.exe".into(),
            executable: path.display().to_string(),
            created: "639000000000000000".into(),
        }
    }

    #[test]
    fn only_target_runtime_and_descendants_are_stopped() {
        let fixture = Fixture::new();
        let data = Inventory {
            processes: vec![
                process(1, 0, &fixture.0.join(r"node\node.exe")),
                process(2, 1, Path::new(r"C:\php\php-cgi.exe")),
                process(3, 0, &fixture.0.join(r"node-other\node.exe")),
                process(4, 0, Path::new(r"C:\Program Files\nodejs\node.exe")),
            ],
            ..Inventory::default()
        };
        assert_eq!(
            fixture
                .plan(&data)
                .processes
                .iter()
                .map(|p| p.pid)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn shared_lah_service_blocks_removal_even_when_stopped() {
        let fixture = Fixture::new();
        let data = Inventory {
            services: vec![Service {
                name: "LAH_PM2_Service".into(),
                executable: fixture.0.join("lah-service.exe").display().to_string(),
                configuration: format!(
                    "<executable>{}</executable>",
                    fixture.0.join(r"node\node.exe").display()
                ),
                running: false,
                pid: 0,
                config_node: String::new(),
                config_home: String::new(),
                config_runner: String::new(),
                command_line: String::new(),
            }],
            ..Inventory::default()
        };
        assert!(fixture
            .plan(&data)
            .blockers
            .iter()
            .any(|s| s.contains("LAH_PM2_Service")));
    }

    #[test]
    fn portable_service_in_another_root_blocks_removal() {
        let fixture = Fixture::new();
        let data = Inventory {
            services: vec![Service {
                name: SERVICE_ID.into(),
                executable: r"C:\other\node\service\pm2-service.exe".into(),
                configuration: String::new(),
                running: true,
                pid: 10,
                config_node: String::new(),
                config_home: String::new(),
                config_runner: String::new(),
                command_line: String::new(),
            }],
            ..Inventory::default()
        };
        assert!(!fixture.plan(&data).blockers.is_empty());
    }

    #[test]
    fn removal_preserves_snapshots_logs_business_files_and_tool() {
        let fixture = Fixture::new();
        let protected = [
            r".pm2\dump.pm2",
            r".pm2\logs\business.log",
            r"logs\pm2-service\service.log",
            r"business\main.js",
            STANDALONE_CONSOLE_EXE_NAME,
        ];
        for relative in protected {
            let path = fixture.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(
                path,
                if relative.ends_with("dump.pm2") {
                    "[]"
                } else {
                    "preserve exactly"
                },
            )
            .unwrap();
        }
        let calls = RefCell::new(vec![]);
        execute_with_host(
            fixture.plan(&Inventory::default()),
            true,
            &|request| {
                let action = request["action"].as_str().unwrap().to_string();
                calls.borrow_mut().push(action.clone());
                Ok(if action == "inventory" {
                    r#"{"services":[],"processes":[]}"#.into()
                } else {
                    String::new()
                })
            },
            &|_, _| {},
            &|_, _| {},
        )
        .unwrap();
        assert!(!fixture.0.join("node").exists());
        assert!(!fixture.0.join("lah-cmd.bat").exists());
        for relative in protected {
            let actual = fs::read_to_string(fixture.0.join(relative)).unwrap();
            assert_eq!(
                actual,
                if relative.ends_with("dump.pm2") {
                    "[]"
                } else {
                    "preserve exactly"
                }
            );
        }
        assert_eq!(
            *calls.borrow(),
            vec![
                "inventory",
                "stop-service",
                "stop-processes",
                "inventory",
                "delete-service"
            ]
        );
        assert!(!fixture.plan(&Inventory::default()).blockers.is_empty());
    }

    #[test]
    fn failed_service_stop_never_deletes_files() {
        let fixture = Fixture::new();
        let error = execute_with_host(
            fixture.plan(&Inventory::default()),
            true,
            &|request| {
                if request["action"] == "inventory" {
                    Ok(r#"{"services":[],"processes":[]}"#.into())
                } else {
                    Err("service cannot stop".into())
                }
            },
            &|_, _| {},
            &|_, _| {},
        )
        .unwrap_err();
        assert!(error.contains("cannot stop"));
        assert!(fixture.0.join(r"node\node.exe").exists());
        assert!(fixture.0.join("lah-cmd.bat").exists());
    }

    #[test]
    fn changed_preview_cannot_add_a_deletion_target() {
        let fixture = Fixture::new();
        let mut plan = fixture.plan(&Inventory::default());
        plan.remove
            .push(fixture.0.join("business").display().to_string());
        let error = execute_with_host(
            plan,
            true,
            &|request| {
                assert_eq!(request["action"], "inventory");
                Ok(r#"{"services":[],"processes":[]}"#.into())
            },
            &|_, _| {},
            &|_, _| {},
        )
        .unwrap_err();
        assert!(error.contains("重新預覽"));
        assert!(fixture.0.join(r"node\node.exe").exists());
    }

    #[test]
    fn new_process_after_stop_prevents_file_deletion() {
        let fixture = Fixture::new();
        let calls = RefCell::new(0);
        let error = execute_with_host(
            fixture.plan(&Inventory::default()),
            true,
            &|request| {
                if request["action"] == "inventory" {
                    *calls.borrow_mut() += 1;
                    let processes = if *calls.borrow() > 1 {
                        vec![process(31, 0, &fixture.0.join(r"node\node.exe"))]
                    } else {
                        vec![]
                    };
                    Ok(json!({"services":[], "processes": processes}).to_string())
                } else {
                    assert_ne!(request["action"], "delete-service");
                    Ok(String::new())
                }
            },
            &|_, _| {},
            &|_, _| {},
        )
        .unwrap_err();
        assert!(error.contains("仍有程序"));
        assert!(fixture.0.join(r"node\node.exe").exists());
    }

    #[test]
    fn unrecognized_runtime_and_unprivileged_users_cannot_remove() {
        let fixture = Fixture::new();
        assert!(!make_plan(&fixture.0, &Inventory::default(), false)
            .blockers
            .is_empty());
        fs::write(fixture.0.join(r"node\node_modules\pm2\package.json"), "{}").unwrap();
        assert!(!fixture.plan(&Inventory::default()).blockers.is_empty());
    }

    #[test]
    fn root_and_relative_paths_are_rejected() {
        for path in ["", "C:\\", ".", r"node\..", r"\\server\share\app"] {
            assert!(validate_root(path).is_err(), "{path}");
        }
    }

    #[test]
    fn links_inside_runtime_cannot_remove_an_outside_file() {
        let fixture = Fixture::new();
        let outside = Fixture::new();
        let junction = fixture.0.join(r"node\linked");
        let output = Command::new("cmd.exe")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(&junction)
            .arg(&outside.0)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(prepare(&fixture.0, &Inventory::default(), true)
            .unwrap_err()
            .contains("Junction"));
        assert!(outside.0.join(r"node\node.exe").exists());
        fs::remove_dir(junction).unwrap();
    }

    #[test]
    fn custom_terminal_and_apps_inside_node_are_protected() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("lah-cmd.bat"), "custom business terminal").unwrap();
        assert!(!fixture
            .plan(&Inventory::default())
            .remove
            .iter()
            .any(|p| p.ends_with("lah-cmd.bat")));
        fs::create_dir_all(fixture.0.join(".pm2")).unwrap();
        fs::write(
            fixture.0.join(r".pm2\dump.pm2"),
            json!([{"name":"business", "pm_exec_path":fixture.0.join(r"node\business.js")}])
                .to_string(),
        )
        .unwrap();
        assert!(prepare(&fixture.0, &Inventory::default(), true).is_err());
    }

    #[test]
    fn retained_log_links_into_node_block_before_any_removal() {
        let fixture = Fixture::new();
        fs::create_dir_all(fixture.0.join(".pm2")).unwrap();
        let logs = fixture.0.join(r"node\old-logs");
        fs::create_dir_all(&logs).unwrap();
        fs::write(logs.join("business.log"), "retained log").unwrap();
        let junction = fixture.0.join(r".pm2\logs");
        let result = Command::new("cmd.exe")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(&junction)
            .arg(&logs)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .unwrap();
        assert!(result.status.success());
        assert!(prepare(&fixture.0, &Inventory::default(), true).is_err());
        assert_eq!(
            fs::read_to_string(logs.join("business.log")).unwrap(),
            "retained log"
        );
        fs::remove_dir(junction).unwrap();
    }

    #[test]
    fn every_saved_application_data_path_is_protected() {
        let fixture = Fixture::new();
        fs::create_dir_all(fixture.0.join(".pm2")).unwrap();
        for field in [
            "pm_exec_path",
            "pm_cwd",
            "cwd",
            "pm_out_log_path",
            "pm_err_log_path",
            "pm_log_path",
        ] {
            let mut app = json!({"name":"business", "pm_exec_path": "C:\\business\\main.js"});
            app[field] = json!(fixture.0.join(r"node\business.log"));
            fs::write(fixture.0.join(r".pm2\dump.pm2"), json!([app]).to_string()).unwrap();
            assert!(
                prepare(&fixture.0, &Inventory::default(), true).is_err(),
                "{field}"
            );
        }
    }

    #[test]
    fn portable_service_must_use_its_own_node_home_and_runner() {
        let fixture = Fixture::new();
        let mut service = Service {
            name: SERVICE_ID.into(),
            executable: fixture
                .0
                .join(r"node\service\pm2-service.exe")
                .display()
                .to_string(),
            configuration: String::new(),
            running: false,
            pid: 0,
            config_node: fixture.0.join(r"node\node.exe").display().to_string(),
            config_home: fixture.0.join(".pm2").display().to_string(),
            config_runner: format!(
                "\"{}\"",
                fixture
                    .0
                    .join(r"node\service\pm2_service_runner.js")
                    .display()
            ),
            command_line: String::new(),
        };
        assert!(fixture
            .plan(&Inventory {
                services: vec![service],
                processes: vec![]
            })
            .blockers
            .is_empty());
        service = Service {
            name: SERVICE_ID.into(),
            executable: fixture
                .0
                .join(r"node\service\pm2-service.exe")
                .display()
                .to_string(),
            configuration: String::new(),
            running: false,
            pid: 0,
            config_node: fixture.0.join(r"node\node.exe").display().to_string(),
            config_home: r"C:\other\.pm2".into(),
            config_runner: fixture
                .0
                .join(r"node\service\pm2_service_runner.js")
                .display()
                .to_string(),
            command_line: String::new(),
        };
        assert!(!fixture
            .plan(&Inventory {
                services: vec![service],
                processes: vec![]
            })
            .blockers
            .is_empty());
    }

    #[test]
    fn native_host_only_stops_the_verified_fixture_process() {
        struct Running(std::process::Child);
        impl Drop for Running {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let fixture = Fixture::new();
        let ping = PathBuf::from(std::env::var("SystemRoot").unwrap()).join(r"System32\ping.exe");
        let target = fixture.0.join(r"node\node.exe");
        let unrelated = fixture.0.join(r"node-other\node.exe");
        fs::create_dir_all(unrelated.parent().unwrap()).unwrap();
        fs::copy(
            PathBuf::from(std::env::var("SystemRoot").unwrap()).join(r"System32\cmd.exe"),
            &target,
        )
        .unwrap();
        fs::copy(&ping, &unrelated).unwrap();
        let mut target_child = Running(
            Command::new(&target)
                .args(["/D", "/C", "ping.exe -n 60 127.0.0.1 >nul"])
                .stdout(std::process::Stdio::null())
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .unwrap(),
        );
        let mut other_child = Running(
            Command::new(&unrelated)
                .args(["-n", "60", "127.0.0.1"])
                .stdout(std::process::Stdio::null())
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .unwrap(),
        );
        let data = inventory().unwrap();
        let new_children: Vec<_> = data
            .processes
            .iter()
            .filter(|p| p.parent_pid == target_child.0.id())
            .map(|p| p.pid)
            .collect();
        assert!(
            !new_children.is_empty(),
            "The test must exercise newly discovered child processes"
        );
        let processes: Vec<_> = data
            .processes
            .into_iter()
            .filter(|p| p.pid == target_child.0.id())
            .collect();
        assert_eq!(processes.len(), 1);
        let mut changed = processes.clone();
        changed[0].created = "1".into();
        assert!(host(json!({"action":"stop-processes", "processes": changed})).is_err());
        assert!(target_child.0.try_wait().unwrap().is_none());
        host(json!({"action":"stop-processes", "processes": processes})).unwrap();
        assert!(target_child.0.try_wait().unwrap().is_some());
        assert!(other_child.0.try_wait().unwrap().is_none());
        assert!(inventory()
            .unwrap()
            .processes
            .iter()
            .all(|p| !new_children.contains(&p.pid)));
    }
}
