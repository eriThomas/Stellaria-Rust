use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows_sys::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetClassNameW, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsIconic,
    SetForegroundWindow, ShowWindow, GWL_STYLE, SW_RESTORE, SW_SHOWNOACTIVATE, WS_CAPTION,
};

const DESKTOP_CLASSES: [&str; 3] = ["Progman", "WorkerW", "Shell_TrayWnd"];

fn class_name(hwnd: HWND) -> String {
    let mut buffer = [0u16; 128];
    let length = unsafe { GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

fn covers_its_monitor(hwnd: HWND) -> bool {
    unsafe {
        let mut window = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if GetWindowRect(hwnd, &mut window) == 0 {
            return false;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return false;
        }
        let screen = info.rcMonitor;
        window.left <= screen.left && window.top <= screen.top && window.right >= screen.right && window.bottom >= screen.bottom
    }
}

pub fn game_running(own: HWND) -> bool {
    let mut state = 0;
    if unsafe { SHQueryUserNotificationState(&mut state) } == 0 && state == QUNS_RUNNING_D3D_FULL_SCREEN {
        return true;
    }
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.is_null() || foreground == own || DESKTOP_CLASSES.contains(&class_name(foreground).as_str()) {
        return false;
    }
    covers_its_monitor(foreground)
}

/// Returns true only for exclusive D3D fullscreen or true borderless fullscreen (not normal maximized apps).
pub fn is_true_fullscreen(own: HWND) -> bool {
    let mut state = 0;
    if unsafe { SHQueryUserNotificationState(&mut state) } == 0 && state == QUNS_RUNNING_D3D_FULL_SCREEN {
        return true;
    }
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.is_null() || foreground == own || DESKTOP_CLASSES.contains(&class_name(foreground).as_str()) {
        return false;
    }
    let style = unsafe { GetWindowLongPtrW(foreground, GWL_STYLE) } as u32;
    let has_caption = (style & WS_CAPTION) == WS_CAPTION;
    !has_caption && covers_its_monitor(foreground)
}

pub fn bring_to_front(hwnd: HWND) {
    unsafe {
        let foreground = GetForegroundWindow();
        if foreground == hwnd {
            return;
        }
        let current = GetCurrentThreadId();
        let other = if foreground.is_null() { 0 } else { GetWindowThreadProcessId(foreground, std::ptr::null_mut()) };
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
}

pub fn present<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) -> bool {
    let Ok(handle) = window.hwnd() else {
        let _ = window.show();
        return false;
    };
    let hwnd = handle.0 as HWND;
    if game_running(hwnd) {
        unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
        return false;
    }
    let _ = window.unminimize();
    let _ = window.show();
    bring_to_front(hwnd);
    true
}

#[tauri::command]
pub fn present_window(window: tauri::WebviewWindow) -> bool {
    present(&window)
}
