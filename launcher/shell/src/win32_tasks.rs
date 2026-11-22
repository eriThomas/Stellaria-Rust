use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

use stellaria_core::taskbar_model::TaskWindow;
use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetClassNameW, GetForegroundWindow, GetWindow,
    GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindow, IsWindowVisible, PostMessageW, SetForegroundWindow, ShowWindow, GWL_EXSTYLE,
    GW_OWNER, SW_MINIMIZE, SW_RESTORE, WM_CLOSE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const EXCLUDED_CLASSES: [&str; 6] = [
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "Windows.UI.Core.CoreWindow",
    "ApplicationFrameWindow",
];

fn window_text(hwnd: HWND) -> String {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; (len + 1) as usize];
    let written = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..written.max(0) as usize])
}

fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 128];
    let len = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    let hr = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED as u32,
            (&mut cloaked as *mut u32).cast(),
            std::mem::size_of::<u32>() as u32,
        )
    };
    hr == 0 && cloaked != 0
}

fn process_image_path(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(handle);
        if ok == 0 || size == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..size as usize]))
    }
}

fn monitor_device_id(hwnd: HWND) -> String {
    unsafe {
        let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if hmon.is_null() {
            return String::new();
        }
        let mut info: MONITORINFOEXW = std::mem::zeroed();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(hmon, (&mut info as *mut MONITORINFOEXW).cast()) == 0 {
            return String::new();
        }
        let len = info
            .szDevice
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(info.szDevice.len());
        String::from_utf16_lossy(&info.szDevice[..len])
    }
}

struct EnumContext {
    own_pid: u32,
    foreground: HWND,
    windows: Vec<TaskWindow>,
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam as *mut EnumContext);
    if IsWindowVisible(hwnd) == 0 {
        return 1;
    }

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == 0 || pid == ctx.own_pid {
        return 1;
    }

    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    let is_tool = (ex_style & WS_EX_TOOLWINDOW) != 0;
    let is_app = (ex_style & WS_EX_APPWINDOW) != 0;
    let is_noactivate = (ex_style & WS_EX_NOACTIVATE) != 0;
    if is_tool || (is_noactivate && !is_app) {
        return 1;
    }

    let owner = GetWindow(hwnd, GW_OWNER);
    if !owner.is_null() && !is_app {
        return 1;
    }

    if is_cloaked(hwnd) {
        return 1;
    }

    let title = window_text(hwnd);
    if title.trim().is_empty() {
        return 1;
    }

    let cls = class_name(hwnd);
    if EXCLUDED_CLASSES.contains(&cls.as_str()) {
        return 1;
    }

    let exe_path = process_image_path(pid).unwrap_or_default();
    let exe_name = exe_path
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or_default()
        .to_string();

    ctx.windows.push(TaskWindow {
        hwnd: hwnd as usize as u64,
        pid,
        title,
        exe_path,
        exe_name,
        monitor_id: monitor_device_id(hwnd),
        minimized: IsIconic(hwnd) != 0,
        active: hwnd == ctx.foreground,
    });
    1
}

/// Enumerates all eligible taskbar windows natively via Win32 (excluding Stellaria's own PID,
/// cloaked UWP windows, tool windows, and owned popups).
pub fn enumerate_windows() -> Vec<TaskWindow> {
    let mut ctx = EnumContext {
        own_pid: std::process::id(),
        foreground: unsafe { GetForegroundWindow() },
        windows: Vec::with_capacity(32),
    };
    unsafe {
        EnumWindows(Some(enum_proc), (&mut ctx as *mut EnumContext) as LPARAM);
    }
    ctx.windows
}

fn verify_hwnd_pid(hwnd_raw: u64, expected_pid: u32) -> Result<HWND, String> {
    let hwnd = hwnd_raw as usize as HWND;
    if hwnd.is_null() || unsafe { IsWindow(hwnd) } == 0 {
        return Err("stale-window".to_string());
    }
    let mut actual_pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut actual_pid);
    }
    if expected_pid != 0 && actual_pid != expected_pid {
        return Err("pid-mismatch".to_string());
    }
    Ok(hwnd)
}

pub fn activate_or_toggle_window(hwnd_raw: u64, expected_pid: u32, allow_minimize_if_active: bool) -> Result<(), String> {
    let hwnd = verify_hwnd_pid(hwnd_raw, expected_pid)?;
    unsafe {
        let fg = GetForegroundWindow();
        if allow_minimize_if_active && fg == hwnd && IsIconic(hwnd) == 0 {
            ShowWindow(hwnd, SW_MINIMIZE);
            return Ok(());
        }
        let current = GetCurrentThreadId();
        let other = if fg.is_null() {
            0
        } else {
            GetWindowThreadProcessId(fg, std::ptr::null_mut())
        };
        let attached = other != 0 && other != current && AttachThreadInput(current, other, 1) != 0;
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
        if attached {
            AttachThreadInput(current, other, 0);
        }
    }
    Ok(())
}

pub fn close_window_graceful(hwnd_raw: u64, expected_pid: u32) -> Result<(), String> {
    let hwnd = verify_hwnd_pid(hwnd_raw, expected_pid)?;
    unsafe {
        PostMessageW(hwnd, WM_CLOSE, 0, 0);
    }
    Ok(())
}

pub fn launch_app_safe(path_str: &str) -> Result<(), String> {
    let path = Path::new(path_str);
    if !path.is_file() {
        return Err("app-not-found".to_string());
    }
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "exe" => {
            Command::new(path)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        "lnk" => {
            let windir = std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into());
            Command::new(Path::new(&windir).join("explorer.exe"))
                .arg(path)
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        _ => Err("unsupported-launch-target".to_string()),
    }
}
