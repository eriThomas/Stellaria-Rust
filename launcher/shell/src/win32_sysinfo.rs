use std::os::windows::process::CommandExt;
use std::process::Command;

use stellaria_core::telemetry::{parse_nvidia_smi_csv, GpuTelemetry, SystemInfo};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const HKEY_LOCAL_MACHINE: isize = 0x8000_0002u32 as isize;
const RRF_RT_REG_SZ: u32 = 0x0000_0002;

#[link(name = "advapi32")]
extern "system" {
    fn RegGetValueW(
        hkey: isize,
        lp_sub_key: *const u16,
        lp_value: *const u16,
        dw_flags: u32,
        pdw_type: *mut u32,
        pv_data: *mut core::ffi::c_void,
        pcb_data: *mut u32,
    ) -> i32;
}

fn read_hklm_string(subkey: &str, value_name: &str) -> Option<String> {
    let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let value_w: Vec<u16> = value_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut bytes: u32 = 0;
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            subkey_w.as_ptr(),
            value_w.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut bytes,
        )
    };
    if status != 0 || bytes < 2 || bytes > 4096 {
        return None;
    }
    let mut buf = vec![0u16; (bytes as usize) / 2];
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            subkey_w.as_ptr(),
            value_w.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            &mut bytes,
        )
    };
    if status != 0 {
        return None;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let s = String::from_utf16_lossy(&buf[..len]).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// Queries NVIDIA GPU telemetry natively via `nvidia-smi` without needing the `.NET` helper.
pub fn query_gpu_telemetry() -> Option<GpuTelemetry> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,utilization.gpu,temperature.gpu",
            "--format=csv,noheader,nounits",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_nvidia_smi_csv(&stdout)
}

/// Queries Windows OS, CPU, GPU, and user name natively via Win32 Registry & `nvidia-smi`.
pub fn query_system_info() -> SystemInfo {
    let build_str = read_hklm_string(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "CurrentBuildNumber",
    )
    .unwrap_or_default();
    let build_num: u32 = build_str.parse().unwrap_or(0);
    let display_ver = read_hklm_string(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "DisplayVersion",
    )
    .unwrap_or_default();
    let os = if build_num >= 22000 {
        if display_ver.is_empty() {
            format!("Windows 11 (Build {build_num})")
        } else {
            format!("Windows 11 {display_ver}")
        }
    } else if !display_ver.is_empty() {
        format!("Windows 10 {display_ver}")
    } else {
        "Windows".to_string()
    };

    let cpu = read_hklm_string(
        r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
        "ProcessorNameString",
    )
    .unwrap_or_else(|| "CPU".to_string());

    let gpu = query_gpu_telemetry().map(|g| g.name);
    let user = std::env::var("USERNAME").unwrap_or_default();

    SystemInfo {
        os,
        cpu,
        gpu,
        user,
        picture_base64: None,
    }
}
