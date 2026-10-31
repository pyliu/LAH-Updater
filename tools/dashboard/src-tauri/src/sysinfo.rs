use crate::types::{
    HostMetrics, LahMemoryMetrics, NetworkMetrics, Pm2ProcessInfo, PortStatus, ServiceMemoryItem,
};
use std::ffi::c_void;
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

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

#[repr(C)]
struct MIB_IF_ROW2 {
    interface_luid: u64,
    interface_index: u32,
    interface_guid: [u8; 16],
    alias: [u16; 257],
    description: [u16; 257],
    physical_address_length: u32,
    physical_address: [u8; 32],
    permanent_physical_address: [u8; 32],
    mtu: u32,
    type_: u32,
    tunnel_type: u32,
    media_type: u32,
    physical_medium_type: u32,
    access_type: u32,
    direction_type: u32,
    interface_and_oper_status_flags: u8,
    oper_status: u32,
    admin_status: u32,
    media_connect_state: u32,
    network_guid: [u8; 16],
    connection_type: u32,
    transmit_link_speed: u64,
    receive_link_speed: u64,
    in_octets: u64,
    in_ucast_pkts: u64,
    in_nucast_pkts: u64,
    in_discards: u64,
    in_errors: u64,
    in_unknown_protos: u64,
    in_ucast_octets: u64,
    in_multicast_octets: u64,
    in_broadcast_octets: u64,
    out_octets: u64,
    out_ucast_pkts: u64,
    out_nucast_pkts: u64,
    out_discards: u64,
    out_errors: u64,
    out_ucast_octets: u64,
    out_multicast_octets: u64,
    out_broadcast_octets: u64,
    out_qlen: u64,
}

#[repr(C)]
struct MIB_IF_TABLE2 {
    num_entries: u32,
    table: [MIB_IF_ROW2; 1],
}

#[repr(C)]
struct UNICODE_STRING {
    length: u16,
    maximum_length: u16,
    buffer: *mut u16,
}

#[repr(C)]
struct SYSTEM_PROCESS_INFO {
    next_entry_offset: u32,
    number_of_threads: u32,
    working_set_private_size: i64,
    hard_fault_count: u32,
    number_of_threads_high_watermark: u32,
    cycle_time: u64,
    create_time: i64,
    user_time: i64,
    kernel_time: i64,
    image_name: UNICODE_STRING,
    base_priority: i32,
    unique_process_id: *mut c_void,
    inherited_from_unique_process_id: *mut c_void,
    handle_count: u32,
    session_id: u32,
    unique_process_key: usize,
    peak_virtual_size: usize,
    virtual_size: usize,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
    private_page_count: usize,
}

#[link(name = "iphlpapi")]
#[link(name = "ntdll")]
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
    fn GetIfTable2(Table: *mut *mut MIB_IF_TABLE2) -> u32;
    fn FreeMibTable(Memory: *mut c_void);
    fn NtQuerySystemInformation(
        SystemInformationClass: u32,
        SystemInformation: *mut c_void,
        SystemInformationLength: u32,
        ReturnLength: *mut u32,
    ) -> i32;
}

fn filetime_to_u64(ft: FILETIME) -> u64 {
    ((ft.dw_high_date_time as u64) << 32) | (ft.dw_low_date_time as u64)
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    const K: f64 = 1024.0;
    const SIZES: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let i = (bytes as f64).log(K).floor() as usize;
    let i = i.min(SIZES.len() - 1);
    let val = (bytes as f64) / K.powi(i as i32);
    format!("{:.1} {}", val, SIZES[i])
}

pub fn format_network_speed(bytes_per_sec: u64) -> String {
    if bytes_per_sec < 1024 {
        format!("{} B/s", bytes_per_sec)
    } else if bytes_per_sec < 1024 * 1024 {
        format!("{:.1} KB/s", bytes_per_sec as f64 / 1024.0)
    } else {
        format!("{:.2} MB/s", bytes_per_sec as f64 / 1024.0 / 1024.0)
    }
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

static LAST_NETWORK_SAMPLE: Mutex<Option<(Instant, u64, u64)>> = Mutex::new(None);

fn calculate_network_metrics() -> NetworkMetrics {
    let mut total_in: u64 = 0;
    let mut total_out: u64 = 0;

    unsafe {
        let mut table_ptr: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        if GetIfTable2(&mut table_ptr) == 0 && !table_ptr.is_null() {
            let count = (*table_ptr).num_entries as usize;
            let rows_ptr = (*table_ptr).table.as_ptr();

            for i in 0..count {
                let row = &*rows_ptr.add(i);
                // 排除 loopback (24)
                if row.type_ == 24 {
                    continue;
                }
                // 檢查是否為實體連接介面或具有真實名稱
                let desc = String::from_utf16_lossy(&row.description);
                let desc_lower = desc.to_lowercase();
                // 過濾 NDIS 虛擬中介篩選器與封包排程器重複計數
                if desc_lower.contains("filter")
                    || desc_lower.contains("scheduler")
                    || desc_lower.contains("miniport")
                    || desc_lower.contains("kernel debug")
                {
                    continue;
                }

                total_in += row.in_octets;
                total_out += row.out_octets;
            }
            FreeMibTable(table_ptr as *mut c_void);
        }
    }

    let now = Instant::now();
    let mut lock = LAST_NETWORK_SAMPLE.lock().unwrap();

    let (rx_speed, tx_speed) = if let Some((prev_time, prev_in, prev_out)) = *lock {
        let elapsed = now.duration_since(prev_time).as_secs_f64();
        if elapsed > 0.1 {
            let delta_in = total_in.saturating_sub(prev_in);
            let delta_out = total_out.saturating_sub(prev_out);
            let rx = (delta_in as f64 / elapsed) as u64;
            let tx = (delta_out as f64 / elapsed) as u64;
            *lock = Some((now, total_in, total_out));
            (rx, tx)
        } else {
            (0, 0)
        }
    } else {
        *lock = Some((now, total_in, total_out));
        (0, 0)
    };

    NetworkMetrics {
        rx_bytes_per_sec: rx_speed,
        tx_bytes_per_sec: tx_speed,
        rx_formatted: format_network_speed(rx_speed),
        tx_formatted: format_network_speed(tx_speed),
        total_rx_bytes: total_in,
        total_tx_bytes: total_out,
    }
}

pub fn query_apache_processes() -> Vec<ServiceMemoryItem> {
    let mut items = Vec::new();
    unsafe {
        let mut buffer_size = 256 * 1024;
        let mut buffer: Vec<u8> = vec![0; buffer_size];
        let mut return_length: u32 = 0;

        let status = NtQuerySystemInformation(
            5, // SYSTEM_PROCESS_INFORMATION
            buffer.as_mut_ptr() as *mut c_void,
            buffer_size as u32,
            &mut return_length,
        );

        if status != 0 {
            buffer_size = return_length as usize + 64 * 1024;
            buffer.resize(buffer_size, 0);
            let s2 = NtQuerySystemInformation(
                5,
                buffer.as_mut_ptr() as *mut c_void,
                buffer_size as u32,
                &mut return_length,
            );
            if s2 != 0 {
                return items;
            }
        }

        let mut offset = 0;
        let mut total_httpd_bytes: u64 = 0;
        let mut httpd_pids = Vec::new();

        loop {
            let info = &*(buffer.as_ptr().add(offset) as *const SYSTEM_PROCESS_INFO);
            if !info.image_name.buffer.is_null() && info.image_name.length > 0 {
                let len = (info.image_name.length / 2) as usize;
                let slice = std::slice::from_raw_parts(info.image_name.buffer, len);
                let name = String::from_utf16_lossy(slice);
                if name.eq_ignore_ascii_case("httpd.exe") {
                    let pid = info.unique_process_id as usize as u32;
                    let mem_bytes = info.working_set_size as u64;
                    total_httpd_bytes += mem_bytes;
                    httpd_pids.push(pid);
                }
            }

            if info.next_entry_offset == 0 {
                break;
            }
            offset += info.next_entry_offset as usize;
        }

        if total_httpd_bytes > 0 {
            items.push(ServiceMemoryItem {
                name: format!("Apache 2.4 閘道 (httpd.exe ×{})", httpd_pids.len()),
                pid: httpd_pids.first().copied().unwrap_or(0),
                memory_bytes: total_httpd_bytes,
                memory_formatted: format_bytes(total_httpd_bytes),
                category: "apache".to_string(),
            });
        }
    }
    items
}

pub fn is_port_listening(port: u16, host: &str, timeout_ms: u64) -> bool {
    let addr_str = format!("{}:{}", host, port);
    if let Ok(addr) = addr_str.parse::<SocketAddr>() {
        return TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)).is_ok();
    }
    false
}

pub fn query_host_metrics(root_dir: &Path, pm2_processes: &[Pm2ProcessInfo]) -> HostMetrics {
    let cpu_usage = calculate_cpu_usage();
    let (mem_total, mem_used, mem_load_pct) = get_memory_info();
    let (disk_total, disk_free, disk_used_pct) = get_disk_space(root_dir);
    let network = calculate_network_metrics();

    // 採集 Apache 記憶體並整合 PM2 記憶體
    let mut memory_items = query_apache_processes();
    for p in pm2_processes {
        if p.memory > 0 {
            memory_items.push(ServiceMemoryItem {
                name: format!("{} (PID: {})", p.name, p.pid),
                pid: p.pid,
                memory_bytes: p.memory,
                memory_formatted: format_bytes(p.memory),
                category: "pm2".to_string(),
            });
        }
    }

    // 依記憶體使用量降序排序
    memory_items.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));

    let total_lah_bytes: u64 = memory_items.iter().map(|item| item.memory_bytes).sum();
    let lah_pct = if mem_total > 0 {
        ((total_lah_bytes as f64 / mem_total as f64) * 100.0) as f32
    } else {
        0.0
    };

    let lah_memory = LahMemoryMetrics {
        total_bytes: total_lah_bytes,
        total_formatted: format_bytes(total_lah_bytes),
        system_pct: (lah_pct * 10.0).round() / 10.0,
        items: memory_items,
    };

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
        network,
        lah_memory,
    }
}
