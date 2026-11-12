use crate::system::CREATE_NO_WINDOW;
use serde::Serialize;
use std::io::Read;
use std::net::{SocketAddr, TcpStream};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StreamEvent {
    Output {
        session_id: String,
        stream: &'static str,
        message: String,
    },
    Status {
        session_id: String,
        status: &'static str,
        message: String,
    },
}

type EventSink = Arc<dyn Fn(StreamEvent) + Send + Sync>;

struct Subscription {
    session_id: String,
    stop: mpsc::Sender<()>,
    worker: Option<JoinHandle<()>>,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

static SUBSCRIPTION: OnceLock<Mutex<Option<Subscription>>> = OnceLock::new();

struct DaemonConnection {
    socket: TcpStream,
}

fn parse_rpc_endpoint(endpoint: &str) -> Result<SocketAddr, String> {
    let address = endpoint
        .trim()
        .strip_prefix("tcp://")
        .ok_or("沙盒 PM2 RPC 必須使用本機 TCP 端點。")?
        .parse::<SocketAddr>()
        .map_err(|_| "沙盒 PM2 RPC 端點格式無效。")?;
    if !address.ip().is_loopback() || address.port() == 0 {
        return Err("沙盒 PM2 RPC 必須使用有效的本機 Loopback 端點。".into());
    }
    Ok(address)
}

impl DaemonConnection {
    fn open(root: &Path) -> Result<Self, String> {
        let output = Command::new(root.join("node").join("node.exe"))
            .args([
                "-e",
                "process.stdout.write(require(process.argv[1])().DAEMON_RPC_PORT)",
            ])
            .arg(
                root.join("node")
                    .join("node_modules")
                    .join("pm2")
                    .join("paths.js"),
            )
            .env("PM2_HOME", root.join(".pm2"))
            .current_dir(root)
            .stdin(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|error| format!("讀取沙盒 PM2 RPC 端點失敗：{error}"))?;
        if !output.status.success() {
            return Err(format!(
                "讀取沙盒 PM2 RPC 端點失敗：{}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let endpoint =
            String::from_utf8(output.stdout).map_err(|_| "沙盒 PM2 RPC 端點不是有效的 UTF-8。")?;
        Self::connect(parse_rpc_endpoint(&endpoint)?)
    }

    fn connect(address: SocketAddr) -> Result<Self, String> {
        let socket =
            TcpStream::connect_timeout(&address, Duration::from_secs(2)).map_err(|error| {
                format!("無法連線至沙盒 PM2 RPC {address}，請確認沙盒服務已啟動：{error}")
            })?;
        socket
            .set_nonblocking(true)
            .map_err(|error| format!("設定 PM2 RPC 監看失敗：{error}"))?;
        Ok(Self { socket })
    }

    fn alive(&mut self) -> bool {
        let mut buffer = [0; 1024];
        for _ in 0..16 {
            match self.socket.read(&mut buffer) {
                Ok(0) => return false,
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return true,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => return false,
            }
        }
        true
    }
}

pub fn stop(session_id: Option<&str>) {
    let mut current = SUBSCRIPTION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if current
        .as_ref()
        .is_some_and(|active| session_id.map_or(true, |id| active.session_id == id))
    {
        current.take();
    }
}

pub fn start(
    root: &Path,
    pm_id: u32,
    session_id: String,
    max_lines: usize,
    emit: EventSink,
) -> Result<(), String> {
    if session_id.is_empty() || session_id.len() > 128 {
        return Err("日誌監看識別碼無效。".into());
    }
    let root = root
        .canonicalize()
        .map_err(|e| format!("沙盒目錄無效：{e}"))?;
    let node = root.join("node").join("node.exe");
    let pm2 = root
        .join("node")
        .join("node_modules")
        .join("pm2")
        .join("bin")
        .join("pm2");
    if !node.is_file() || !pm2.is_file() {
        return Err("找不到沙盒內的 Node 或 PM2 執行檔。".into());
    }
    let daemon = DaemonConnection::open(&root)?;
    let mut current = SUBSCRIPTION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    current.take();
    let child = Command::new(node)
        .arg(pm2)
        .arg("logs")
        .arg(pm_id.to_string())
        .args(["--raw", "--lines"])
        .arg(max_lines.clamp(1, 5000).to_string())
        .env("PM2_HOME", root.join(".pm2"))
        .env("NO_COLOR", "1")
        .env("FORCE_COLOR", "0")
        .current_dir(&root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("啟動 PM2 日誌監看失敗：{e}"))?;
    *current = Some(subscribe(child, session_id, emit, Some(daemon)));
    Ok(())
}

enum PipeEvent {
    Output(&'static str, String),
    Closed,
}

fn read_pipe(mut pipe: impl Read, stream: &'static str, tx: mpsc::SyncSender<PipeEvent>) {
    let mut buffer = [0u8; 4096];
    let mut pending = Vec::new();
    loop {
        match pipe.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                pending.extend_from_slice(&buffer[..count]);
                let length = match std::str::from_utf8(&pending) {
                    Ok(_) => pending.len(),
                    Err(error) if error.error_len().is_none() => error.valid_up_to(),
                    Err(_) => pending.len(),
                };
                if length > 0 {
                    let message = String::from_utf8_lossy(&pending[..length]).into_owned();
                    pending.drain(..length);
                    if tx.send(PipeEvent::Output(stream, message)).is_err() {
                        return;
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                let _ = tx.send(PipeEvent::Output(
                    "err",
                    format!("\n讀取 PM2 日誌管線失敗：{error}\n"),
                ));
                break;
            }
        }
    }
    if !pending.is_empty() {
        let _ = tx.send(PipeEvent::Output(
            stream,
            String::from_utf8_lossy(&pending).into_owned(),
        ));
    }
    let _ = tx.send(PipeEvent::Closed);
}

fn subscribe(
    mut child: Child,
    session_id: String,
    emit: EventSink,
    mut daemon: Option<DaemonConnection>,
) -> Subscription {
    let stdout = child.stdout.take().expect("PM2 stdout pipe");
    let stderr = child.stderr.take().expect("PM2 stderr pipe");
    let (output_tx, output_rx) = mpsc::sync_channel(16);
    let stderr_tx = output_tx.clone();
    let out_reader = thread::spawn(move || read_pipe(stdout, "out", output_tx));
    let err_reader = thread::spawn(move || read_pipe(stderr, "err", stderr_tx));
    let (stop_tx, stop_rx) = mpsc::channel();
    let worker_session = session_id.clone();
    let worker = thread::spawn(move || {
        let status = |status, message: String| {
            emit(StreamEvent::Status {
                session_id: worker_session.clone(),
                status,
                message,
            })
        };
        status("running", "PM2 日誌監看已啟動，等待輸出。".into());
        let mut closed = 0;
        let mut stopped = false;
        let mut exit_status = None;
        let mut daemon_lost = false;
        while closed < 2 || exit_status.is_none() {
            if !stopped && stop_rx.try_recv().is_ok() {
                stopped = true;
                let _ = child.kill();
                exit_status = Some(child.wait());
            }
            if exit_status.is_none() && daemon.as_mut().is_some_and(|daemon| !daemon.alive()) {
                daemon_lost = true;
                let _ = child.kill();
                exit_status = Some(child.wait());
            }
            if exit_status.is_none() {
                match child.try_wait() {
                    Ok(Some(result)) => exit_status = Some(Ok(result)),
                    Ok(None) => {}
                    Err(error) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        exit_status = Some(Err(error));
                    }
                }
            }
            match output_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(PipeEvent::Output(stream, message)) if !stopped => emit(StreamEvent::Output {
                    session_id: worker_session.clone(),
                    stream,
                    message,
                }),
                Ok(PipeEvent::Closed) => closed += 1,
                Err(mpsc::RecvTimeoutError::Disconnected) => closed = 2,
                _ => {}
            }
        }
        let _ = out_reader.join();
        let _ = err_reader.join();
        if stopped {
            status("stopped", "PM2 日誌監看已停止。".into());
        } else {
            let message = if daemon_lost {
                "PM2 已停止或重新啟動，日誌連線已中斷。".into()
            } else {
                match exit_status {
                    Some(Ok(result)) => format!(
                        "PM2 日誌連線已結束（結束碼 {}）。",
                        result.code().map_or("未知".into(), |code| code.to_string())
                    ),
                    Some(Err(error)) => format!("PM2 日誌連線失敗：{error}"),
                    None => "PM2 日誌連線已中斷。".into(),
                }
            };
            status("disconnected", message);
        }
    });
    Subscription {
        session_id,
        stop: stop_tx,
        worker: Some(worker),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::{Shutdown, TcpListener};
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
        fn WaitForSingleObject(handle: *mut std::ffi::c_void, milliseconds: u32) -> u32;
    }

    #[test]
    fn rpc_endpoint_accepts_only_loopback_tcp() {
        assert_eq!(
            parse_rpc_endpoint("tcp://127.0.0.1:42681").unwrap().port(),
            42681
        );
        assert!(parse_rpc_endpoint("tcp://[::1]:42681").is_ok());
        assert!(parse_rpc_endpoint("tcp://192.0.2.1:42681").is_err());
        assert!(parse_rpc_endpoint("tcp://127.0.0.1:0").is_err());
        assert!(parse_rpc_endpoint("rpc.sock").is_err());
    }

    #[test]
    fn rpc_monitor_needs_no_pid_file_or_process_handle() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut daemon = DaemonConnection::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        assert!(daemon.alive());
        server.write_all(b"pending server data").unwrap();
        server.shutdown(Shutdown::Write).unwrap();
        let mut disconnected = false;
        for _ in 0..100 {
            if !daemon.alive() {
                disconnected = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            disconnected,
            "EOF must not be hidden behind unread RPC bytes"
        );
    }

    #[test]
    fn rpc_monitor_rejects_an_offline_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        assert!(DaemonConnection::connect(address).is_err());
    }

    #[test]
    fn rpc_disconnect_reaps_the_log_child() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let daemon = DaemonConnection::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        let child = Command::new("cmd.exe")
            .args(["/C", "pause"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .unwrap();
        let handle =
            unsafe { OwnedHandle::from_raw_handle(OpenProcess(0x00100000, 0, child.id())) };
        let (tx, rx) = mpsc::channel();
        let subscription = subscribe(
            child,
            "rpc-test".into(),
            Arc::new(move |event| {
                let _ = tx.send(event);
            }),
            Some(daemon),
        );
        drop(server);
        loop {
            if matches!(
                rx.recv_timeout(Duration::from_secs(5)).unwrap(),
                StreamEvent::Status {
                    status: "disconnected",
                    ..
                }
            ) {
                break;
            }
        }
        drop(subscription);
        assert_eq!(unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) }, 0);
    }
    struct SplitReader(Vec<Vec<u8>>);
    impl Read for SplitReader {
        fn read(&mut self, target: &mut [u8]) -> std::io::Result<usize> {
            if self.0.is_empty() {
                return Ok(0);
            }
            let part = self.0.remove(0);
            target[..part.len()].copy_from_slice(&part);
            Ok(part.len())
        }
    }
    #[test]
    fn preserves_split_utf8_and_newlines() {
        let data = "即時\n日誌\n".as_bytes();
        let reader = SplitReader(data.chunks(2).map(|part| part.to_vec()).collect());
        let (tx, rx) = mpsc::sync_channel(16);
        read_pipe(reader, "out", tx);
        let text: String = rx
            .iter()
            .filter_map(|event| match event {
                PipeEvent::Output("out", message) => Some(message),
                _ => None,
            })
            .collect();
        assert_eq!(text, "即時\n日誌\n");
    }
    #[test]
    fn cancellation_reaps_child_and_drains_both_pipes() {
        let child = Command::new("cmd.exe")
            .args(["/C", "echo stdout & echo stderr 1>&2 & pause"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .unwrap();
        let handle =
            unsafe { OwnedHandle::from_raw_handle(OpenProcess(0x00100000, 0, child.id())) };
        let (tx, rx) = mpsc::channel();
        let subscription = subscribe(
            child,
            "test".into(),
            Arc::new(move |event| {
                let _ = tx.send(event);
            }),
            None,
        );
        let mut out = String::new();
        let mut err = String::new();
        while out.is_empty() || err.is_empty() {
            match rx.recv_timeout(Duration::from_secs(5)).unwrap() {
                StreamEvent::Output {
                    stream: "out",
                    message,
                    ..
                } => out.push_str(&message),
                StreamEvent::Output {
                    stream: "err",
                    message,
                    ..
                } => err.push_str(&message),
                _ => {}
            }
        }
        assert!(out.contains("stdout"));
        assert!(err.contains("stderr"));
        drop(subscription);
        assert_eq!(unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) }, 0);
        assert!(rx.iter().any(|event| matches!(
            event,
            StreamEvent::Status {
                status: "stopped",
                ..
            }
        )));
    }

    #[test]
    fn stale_stop_cannot_cancel_current_subscription() {
        let child = Command::new("cmd.exe")
            .args(["/C", "pause"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .unwrap();
        let handle =
            unsafe { OwnedHandle::from_raw_handle(OpenProcess(0x00100000, 0, child.id())) };
        let subscription = subscribe(child, "current".into(), Arc::new(|_| {}), None);
        *SUBSCRIPTION
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap() = Some(subscription);
        stop(Some("previous"));
        assert_eq!(
            unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) },
            258
        );
        stop(Some("current"));
        assert_eq!(unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) }, 0);
    }
}
