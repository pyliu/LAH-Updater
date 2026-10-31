use crate::services::CREATE_NO_WINDOW;
use crate::types::CommandActionResult;
use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

extern "system" {
    fn MultiByteToWideChar(
        CodePage: u32,
        dwFlags: u32,
        lpMultiByteStr: *const u8,
        cbMultiByte: i32,
        lpWideCharStr: *mut u16,
        cchWideChar: i32,
    ) -> i32;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TerminalEncoding {
    Utf8,
    Cp950,
}

impl TerminalEncoding {
    pub fn codepage(&self) -> u32 {
        match self {
            TerminalEncoding::Utf8 => 65001,
            TerminalEncoding::Cp950 => 950,
        }
    }
}

pub fn decode_bytes(bytes: &[u8], codepage: u32) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    if codepage == 65001 {
        // UTF-8
        return String::from_utf8_lossy(bytes).to_string();
    }

    // Windows 原生 CP950 (Big5) 解碼
    unsafe {
        let len = MultiByteToWideChar(
            codepage,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            std::ptr::null_mut(),
            0,
        );
        if len > 0 {
            let mut buf = vec![0u16; len as usize];
            MultiByteToWideChar(
                codepage,
                0,
                bytes.as_ptr(),
                bytes.len() as i32,
                buf.as_mut_ptr(),
                len,
            );
            String::from_utf16_lossy(&buf)
        } else {
            String::from_utf8_lossy(bytes).to_string()
        }
    }
}

struct TerminalProcess {
    stdin: ChildStdin,
    child: Child,
}

struct TerminalManager {
    process: Option<TerminalProcess>,
    output_buffer: Arc<Mutex<String>>,
    current_encoding: Arc<Mutex<TerminalEncoding>>,
}

static TERMINAL: Mutex<Option<TerminalManager>> = Mutex::new(None);

fn get_or_init_manager() -> std::sync::MutexGuard<'static, Option<TerminalManager>> {
    let mut lock = TERMINAL.lock().unwrap();
    if lock.is_none() {
        *lock = Some(TerminalManager {
            process: None,
            output_buffer: Arc::new(Mutex::new(String::new())),
            current_encoding: Arc::new(Mutex::new(TerminalEncoding::Utf8)),
        });
    }
    lock
}

pub fn ensure_terminal_started(root_dir: &Path) -> Result<(), String> {
    let mut lock = get_or_init_manager();
    let mgr = lock.as_mut().unwrap();

    // 檢查現有進程是否仍存活
    if let Some(ref mut proc) = mgr.process {
        match proc.child.try_wait() {
            Ok(None) => return Ok(()), // 進程仍然存活運行中
            _ => {
                mgr.process = None; // 進程已退出，重新啟動
            }
        }
    }

    let node_dir = root_dir.join("node");
    let apache_dir = root_dir.join("Apache24");
    let oci_dir = root_dir.join("instantclient_11_2");
    let pm2_home = root_dir.join(".pm2");

    let sys_path = std::env::var("PATH").unwrap_or_default();
    let sandbox_path = format!(
        "{};{}\\bin;{};{}",
        node_dir.to_string_lossy(),
        apache_dir.to_string_lossy(),
        oci_dir.to_string_lossy(),
        sys_path
    );

    let mut cmd = Command::new("cmd.exe");
    cmd.args(["/Q", "/K", "chcp 65001 >nul"]);
    cmd.current_dir(root_dir);
    cmd.env("ROOT_DIR", root_dir.to_string_lossy().to_string());
    cmd.env("NODE_DIR", node_dir.to_string_lossy().to_string());
    cmd.env("APACHE_DIR", apache_dir.to_string_lossy().to_string());
    cmd.env("OCI_DIR", oci_dir.to_string_lossy().to_string());
    cmd.env("PM2_HOME", pm2_home.to_string_lossy().to_string());
    cmd.env("PATH", sandbox_path);

    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("無法啟動沙盒命令直譯器：{}", e))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "無法取得 stdin 管線".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "無法取得 stdout 管線".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "無法取得 stderr 管線".to_string())?;

    let buf_out = mgr.output_buffer.clone();
    let buf_err = mgr.output_buffer.clone();
    let enc_out = mgr.current_encoding.clone();
    let enc_err = mgr.current_encoding.clone();

    // 背景讀取 stdout
    thread::spawn(move || {
        let mut reader = stdout;
        let mut chunk = [0u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    let cp = enc_out.lock().unwrap().codepage();
                    let text = decode_bytes(&chunk[..n], cp);
                    buf_out.lock().unwrap().push_str(&text);
                }
                Err(_) => break,
            }
        }
    });

    // 背景讀取 stderr
    thread::spawn(move || {
        let mut reader = stderr;
        let mut chunk = [0u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    let cp = enc_err.lock().unwrap().codepage();
                    let text = decode_bytes(&chunk[..n], cp);
                    buf_err.lock().unwrap().push_str(&text);
                }
                Err(_) => break,
            }
        }
    });

    // 印出沙盒歡迎標題
    let welcome = format!(
        "\r\n\x1b[1;36m============================================================\x1b[0m\r\n\x1b[1;32m  LAH 智慧控管專屬沙盒終端 (Node 16 / PM2 / Apache 2.4 / OCI)\x1b[0m\r\n\x1b[90m  - 隔離安裝目錄 : {}\x1b[0m\r\n\x1b[90m  - 私有 Node 目錄: {}\x1b[0m\r\n\x1b[90m  - 私有 PM2 目錄 : {}\x1b[0m\r\n\x1b[90m  - 私有 Apache   : {}\x1b[0m\r\n\x1b[1;36m============================================================\x1b[0m\r\n\r\n",
        root_dir.to_string_lossy(),
        node_dir.to_string_lossy(),
        pm2_home.to_string_lossy(),
        apache_dir.to_string_lossy()
    );
    mgr.output_buffer.lock().unwrap().push_str(&welcome);

    // 寫入初始化換行觸發 cmd prompt
    let _ = writeln!(stdin, "");
    let _ = stdin.flush();

    mgr.process = Some(TerminalProcess { stdin, child });
    Ok(())
}

pub fn send_command(root_dir: &Path, command: &str) -> CommandActionResult {
    if let Err(e) = ensure_terminal_started(root_dir) {
        return CommandActionResult {
            success: false,
            message: e,
        };
    }

    let mut lock = get_or_init_manager();
    let mgr = lock.as_mut().unwrap();
    if let Some(ref mut proc) = mgr.process {
        let write_res = (|| -> Result<(), std::io::Error> {
            writeln!(proc.stdin, "{}", command)?;
            proc.stdin.flush()?;
            Ok(())
        })();

        match write_res {
            Ok(_) => CommandActionResult {
                success: true,
                message: "指令已送出".to_string(),
            },
            Err(e) => CommandActionResult {
                success: false,
                message: format!("發送指令失敗：{}", e),
            },
        }
    } else {
        CommandActionResult {
            success: false,
            message: "沙盒終端進程未就緒".to_string(),
        }
    }
}

pub fn set_terminal_encoding(encoding: &str) -> CommandActionResult {
    let new_enc = match encoding.to_lowercase().as_str() {
        "cp950" | "big5" | "950" => TerminalEncoding::Cp950,
        _ => TerminalEncoding::Utf8,
    };

    let mut lock = get_or_init_manager();
    let mgr = lock.as_mut().unwrap();
    *mgr.current_encoding.lock().unwrap() = new_enc;

    // 雙向聯動：向 cmd.exe 發送對應的 chcp 指令
    let chcp_cmd = match new_enc {
        TerminalEncoding::Cp950 => "chcp 950 >nul\r\n",
        TerminalEncoding::Utf8 => "chcp 65001 >nul\r\n",
    };

    if let Some(ref mut proc) = mgr.process {
        let _ = proc.stdin.write_all(chcp_cmd.as_bytes());
        let _ = proc.stdin.flush();
    }

    let label = match new_enc {
        TerminalEncoding::Cp950 => "CP950 (Big5 繁體中文)",
        TerminalEncoding::Utf8 => "UTF-8 (Unicode)",
    };

    CommandActionResult {
        success: true,
        message: format!("✓ 已切換終端編碼為：{}", label),
    }
}

pub fn poll_output() -> String {
    let lock = get_or_init_manager();
    let mgr = lock.as_ref().unwrap();
    let mut buf = mgr.output_buffer.lock().unwrap();
    let output = buf.clone();
    buf.clear();
    output
}

pub fn reset_session(root_dir: &Path) -> CommandActionResult {
    {
        let mut lock = get_or_init_manager();
        let mgr = lock.as_mut().unwrap();
        if let Some(mut proc) = mgr.process.take() {
            let _ = proc.child.kill();
            let _ = proc.child.wait();
        }
        mgr.output_buffer.lock().unwrap().clear();
    }

    match ensure_terminal_started(root_dir) {
        Ok(_) => CommandActionResult {
            success: true,
            message: "✓ 沙盒終端 Session 已成功重設".to_string(),
        },
        Err(e) => CommandActionResult {
            success: false,
            message: format!("重設失敗：{}", e),
        },
    }
}
