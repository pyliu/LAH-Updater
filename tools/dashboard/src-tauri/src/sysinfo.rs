use crate::types::{HostMetrics, PortStatus};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

#[repr(C)]
struct MEMORYSTATUSEX {
    dw_length: u32,
    dw_memory_load: u32,
    ull_total_phys: u64,
    ull_avail_phys: u64,
    ull_total_page_file: u64,
    ull_avail_page_file: u64,
    ull_total_virtual: u64,
    ull_avail_virtual: u64,
    ull_avail_extended_virtual: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct FILETIME {
    dw_low_date_time: u32,
    dw_high_date_time: u32,
}

extern "system" {
    fn GlobalMemoryStatusEx(lp_buffer: *mut MEMORYSTATUSEX) -> i32;
    fn GetDiskFreeSpaceExW(
        lp_directory_name: *const u16,
        lp_free_bytes_available: *mut u64,
        lp_total_number_of_bytes: *mut u64,
        lp_total_number_of_free_bytes: *mut u64,
    ) -> i32;
    fn GetSystemTimes(
        lp_idle_time: *mut FILETIME,
        lp_kernel_time: *mut FILETIME,
        lp_user_time: *mut FILETIME,
    ) -> i32;
}

fn filetime_to_u64(ft: FILETIME) -> u64 {
    ((ft.dw_high_date_time as u64) << 32) | (ft.dw_low_date_time as u64)
}

static LAST_CPU_TIMES: Mutex<Option<(u64, u64)>> = Mutex::new(None);

fn calculate_cpu_usage() -> f32 {
    let mut idle = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();

    unsafe {
        if GetSystemTimes(&mut idle, &mut kernel, &mut user) == 0 {
            return 0.0;
        }
    }

    let idle_time = filetime_to_u64(idle);
    let total_time = filetime_to_u64(kernel) + filetime_to_u64(user);

    let mut lock = LAST_CPU_TIMES.lock().unwrap();
    if let Some((prev_idle, prev_total)) = *lock {
        let delta_idle = idle_time.saturating_sub(prev_idle);
        let delta_total = total_time.saturating_sub(prev_total);
        *lock = Some((idle_time, total_time));

        if delta_total > 0 {
            let usage = (1.0 - (delta_idle as f32 / delta_total as f32)) * 100.0;
            return usage.clamp(0.0, 100.0);
        }
    } else {
        *lock = Some((idle_time, total_time));
    }

    0.0
}

fn get_memory_info() -> (u64, u64, u32) {
    let mut status = MEMORYSTATUSEX {
        dw_length: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        dw_memory_load: 0,
        ull_total_phys: 0,
        ull_avail_phys: 0,
        ull_total_page_file: 0,
        ull_avail_page_file: 0,
        ull_total_virtual: 0,
        ull_avail_virtual: 0,
        ull_avail_extended_virtual: 0,
    };

    unsafe {
        if GlobalMemoryStatusEx(&mut status) != 0 {
            let used = status.ull_total_phys.saturating_sub(status.ull_avail_phys);
            (status.ull_total_phys, used, status.dw_memory_load)
        } else {
            (0, 0, 0)
        }
    }
}

fn get_disk_space(dir: &Path) -> (u64, u64, u32) {
    let path_str = dir.to_string_lossy();
    let root = if path_str.len() >= 3 && &path_str[1..3] == ":\\" {
        format!("{}\\", &path_str[0..2])
    } else {
        "C:\\".to_string()
    };
    let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();

    let mut free_avail: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut total_free: u64 = 0;

    unsafe {
        if GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free_avail,
            &mut total_bytes,
            &mut total_free,
        ) != 0
            && total_bytes > 0
        {
            let used_bytes = total_bytes.saturating_sub(free_avail);
            let pct = ((used_bytes as f64 / total_bytes as f64) * 100.0) as u32;
            (total_bytes, free_avail, pct)
        } else {
            (0, 0, 0)
        }
    }
}

pub fn is_port_listening(port: u16, host: &str, timeout_ms: u64) -> bool {
    let addr_str = format!("{}:{}", host, port);
    if let Ok(addr) = addr_str.parse::<SocketAddr>() {
        return TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)).is_ok();
    }
    false
}

pub fn query_host_metrics(root_dir: &Path) -> HostMetrics {
    let cpu_usage = calculate_cpu_usage();
    let (mem_total, mem_used, mem_load_pct) = get_memory_info();
    let (disk_total, disk_free, disk_used_pct) = get_disk_space(root_dir);

    let ports_def = [
        (80, "Apache 2.4 Web 閘道 (Port 80)"),
        (8080, "智慧管控前端網站 (Port 8080)"),
        (8081, "即時通訊 WebSocket (Port 8081)"),
        (8082, "即時通訊 HTTP 附件 (Port 8082)"),
        (8085, "PHP 8.5 FastCGI 核心 (Port 8085)"),
    ];

    let mut ports = Vec::new();
    for (port, name) in ports_def {
        let is_listening = is_port_listening(port, "127.0.0.1", 120);
        ports.push(PortStatus {
            port,
            name: name.to_string(),
            is_listening,
        });
    }

    HostMetrics {
        cpu_usage,
        mem_total,
        mem_used,
        mem_load_pct,
        disk_total,
        disk_free,
        disk_used_pct,
        ports,
    }
}
