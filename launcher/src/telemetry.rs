use std::time::Instant;

use stellaria_core::telemetry::{
    compute_cpu_percent, DiskTelemetry, GpuTelemetry, NetworkTelemetry, TelemetrySnapshot,
};
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::NetworkManagement::IpHelper::{GetIfTable, MIB_IFROW, MIB_IFTABLE};
use windows_sys::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives};
use windows_sys::Win32::System::SystemInformation::{
    GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX,
};
use windows_sys::Win32::System::Threading::GetSystemTimes;

const DRIVE_FIXED: u32 = 3;
const IF_TYPE_SOFTWARE_LOOPBACK: u32 = 24;

fn filetime_to_u64(ft: FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

fn read_system_times() -> Option<(u64, u64, u64)> {
    let mut idle = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let mut kernel = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let mut user = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    if ok == 0 {
        return None;
    }
    Some((filetime_to_u64(idle), filetime_to_u64(kernel), filetime_to_u64(user)))
}

fn read_memory() -> (u64, u64) {
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return (0, 0);
    }
    let total = status.ullTotalPhys;
    let used = total.saturating_sub(status.ullAvailPhys);
    (used, total)
}

fn read_disks() -> Vec<DiskTelemetry> {
    let mask = unsafe { GetLogicalDrives() };
    let mut disks = Vec::new();
    for index in 0..26u32 {
        if (mask & (1 << index)) == 0 {
            continue;
        }
        let letter = (b'A' + index as u8) as char;
        let mount = format!("{letter}:\\");
        let wide: Vec<u16> = mount.encode_utf16().chain(std::iter::once(0)).collect();
        if unsafe { GetDriveTypeW(wide.as_ptr()) } != DRIVE_FIXED {
            continue;
        }
        let mut free_caller = 0u64;
        let mut total = 0u64;
        let mut total_free = 0u64;
        let ok = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_caller,
                &mut total,
                &mut total_free,
            )
        };
        if ok != 0 && total > 0 {
            disks.push(DiskTelemetry {
                mount_point: mount,
                total_bytes: total,
                available_bytes: free_caller,
            });
        }
    }
    disks
}

fn read_network_totals() -> (u64, u64) {
    let mut size: u32 = 0;
    unsafe {
        GetIfTable(std::ptr::null_mut(), &mut size, 0);
    }
    if size == 0 {
        return (0, 0);
    }
    let mut buffer = vec![0u8; size as usize];
    let status = unsafe { GetIfTable(buffer.as_mut_ptr().cast::<MIB_IFTABLE>(), &mut size, 0) };
    if status != 0 {
        return (0, 0);
    }
    let table = unsafe { &*(buffer.as_ptr().cast::<MIB_IFTABLE>()) };
    let count = table.dwNumEntries as usize;
    let rows_ptr = table.table.as_ptr();
    let max_rows = (buffer.len().saturating_sub(std::mem::size_of::<u32>()))
        / std::mem::size_of::<MIB_IFROW>();
    let rows = unsafe { std::slice::from_raw_parts(rows_ptr, count.min(max_rows)) };

    let mut total_rx = 0u64;
    let mut total_tx = 0u64;
    for row in rows {
        if row.dwType == IF_TYPE_SOFTWARE_LOOPBACK {
            continue;
        }
        total_rx = total_rx.saturating_add(row.dwInOctets as u64);
        total_tx = total_tx.saturating_add(row.dwOutOctets as u64);
    }
    (total_rx, total_tx)
}

pub struct SystemMetricsSampler {
    prev_cpu: Option<(u64, u64, u64)>,
    prev_net: Option<(u64, u64, Instant)>,
    cached_disks: Vec<DiskTelemetry>,
    last_disk_scan: Option<Instant>,
    revision: u64,
}

impl Default for SystemMetricsSampler {
    fn default() -> Self {
        Self {
            prev_cpu: read_system_times(),
            prev_net: None,
            cached_disks: Vec::new(),
            last_disk_scan: None,
            revision: 0,
        }
    }
}

impl SystemMetricsSampler {
    pub fn sample(&mut self, gpu: Option<GpuTelemetry>) -> TelemetrySnapshot {
        self.revision += 1;
        let now = Instant::now();
        let uptime_ms = unsafe { GetTickCount64() };

        let cpu_percent = match (self.prev_cpu, read_system_times()) {
            (Some((pi, pk, pu)), Some(curr @ (ci, ck, cu))) => {
                self.prev_cpu = Some(curr);
                compute_cpu_percent(pi, pk, pu, ci, ck, cu)
            }
            (None, Some(curr)) => {
                self.prev_cpu = Some(curr);
                0.0
            }
            _ => 0.0,
        };

        let (memory_used_bytes, memory_total_bytes) = read_memory();

        let refresh_disks = self
            .last_disk_scan
            .map(|t| t.elapsed().as_secs() >= 30)
            .unwrap_or(true);
        if refresh_disks {
            self.cached_disks = read_disks();
            self.last_disk_scan = Some(now);
        }

        let (total_rx, total_tx) = read_network_totals();
        let (rx_rate, tx_rate) = match self.prev_net {
            Some((prev_rx, prev_tx, prev_at)) => {
                let elapsed = prev_at.elapsed().as_secs_f64().max(0.1);
                let rx = (total_rx.saturating_sub(prev_rx) as f64 / elapsed).round() as u64;
                let tx = (total_tx.saturating_sub(prev_tx) as f64 / elapsed).round() as u64;
                (rx, tx)
            }
            None => (0, 0),
        };
        self.prev_net = Some((total_rx, total_tx, now));

        TelemetrySnapshot {
            revision: self.revision,
            uptime_ms,
            cpu_percent,
            memory_used_bytes,
            memory_total_bytes,
            disks: self.cached_disks.clone(),
            network: NetworkTelemetry {
                rx_bytes_per_sec: rx_rate,
                tx_bytes_per_sec: tx_rate,
                total_rx_bytes: total_rx,
                total_tx_bytes: total_tx,
            },
            gpu,
        }
    }
}
