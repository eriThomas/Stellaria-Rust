use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::OnceLock;
use std::time::Duration;

use stellaria_core::shell_state::RecoveryReason;
use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::RemoteDesktop::{
    WTSRegisterSessionNotification, WTSUnRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION,
};
use windows_sys::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, EnumWindows, FindWindowW,
    GetClassNameW, GetForegroundWindow, GetMessageW, GetWindowLongPtrW, GetWindowRect,
    RegisterClassW, RegisterWindowMessageW, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    TranslateMessage, GWL_EXSTYLE, GWL_STYLE, MSG, PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOW,
    WM_DISPLAYCHANGE, WM_POWERBROADCAST, WM_WTSSESSION_CHANGE, WNDCLASSW, WS_CAPTION,
    WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP, WTS_SESSION_LOCK,
    WTS_SESSION_UNLOCK,
};

const DESKTOP_CLASSES: [&str; 4] = ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

static TASKBAR_HIDDEN_BY_US: AtomicBool = AtomicBool::new(false);
static SESSION_LOCKED: AtomicBool = AtomicBool::new(false);
static RECOVERY_TX: OnceLock<mpsc::Sender<RecoveryReason>> = OnceLock::new();
static TASKBAR_CREATED_MSG: OnceLock<u32> = OnceLock::new();

pub fn is_session_locked() -> bool {
    SESSION_LOCKED.load(Ordering::Acquire)
}

pub fn exclude_from_alt_tab(hwnd: HWND) {
    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let updated = (ex_style | WS_EX_TOOLWINDOW as isize) & !(WS_EX_APPWINDOW as isize);
        if updated != ex_style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, updated);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

fn class_name(hwnd: HWND) -> String {
    let mut buffer = [0u16; 128];
    let len = unsafe { GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..len.max(0) as usize])
}

fn covers_its_monitor(hwnd: HWND) -> bool {
    unsafe {
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return false;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return false;
        }
        let s = info.rcMonitor;
        rect.left <= s.left && rect.top <= s.top && rect.right >= s.right && rect.bottom >= s.bottom
    }
}

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

unsafe extern "system" fn enum_secondary_taskbars(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if class_name(hwnd) == "Shell_SecondaryTrayWnd" {
        let cmd = if lparam != 0 { SW_SHOW } else { SW_HIDE };
        ShowWindow(hwnd, cmd);
    }
    1
}

/// Safely hides or restores the native Windows taskbar on all monitors.
pub fn set_windows_taskbar_visible(visible: bool) {
    unsafe {
        let primary_class: Vec<u16> = "Shell_TrayWnd\0".encode_utf16().collect();
        let primary = FindWindowW(primary_class.as_ptr(), std::ptr::null());
        if !primary.is_null() {
            ShowWindow(primary, if visible { SW_SHOW } else { SW_HIDE });
        }
        EnumWindows(Some(enum_secondary_taskbars), if visible { 1 } else { 0 });
    }
    TASKBAR_HIDDEN_BY_US.store(!visible, Ordering::Release);
}

pub fn restore_windows_shell() {
    set_windows_taskbar_visible(true);
}

/// Installs a panic hook so a crash never leaves the Windows taskbar hidden.
pub fn install_shell_restore_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if TASKBAR_HIDDEN_BY_US.load(Ordering::Acquire) {
            restore_windows_shell();
        }
        previous(info);
    }));
}

pub struct ShellRestoreGuard;

impl Drop for ShellRestoreGuard {
    fn drop(&mut self) {
        if TASKBAR_HIDDEN_BY_US.load(Ordering::Acquire) {
            restore_windows_shell();
        }
    }
}

fn emit_reason(reason: RecoveryReason) {
    if let Some(tx) = RECOVERY_TX.get() {
        let _ = tx.send(reason);
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if let Some(&tb_msg) = TASKBAR_CREATED_MSG.get() {
        if tb_msg != 0 && msg == tb_msg {
            if !is_session_locked() {
                emit_reason(RecoveryReason::ExplorerRestart);
            }
            return 0;
        }
    }

    match msg {
        WM_DISPLAYCHANGE => {
            if !is_session_locked() {
                emit_reason(RecoveryReason::DisplayChanged);
            }
        }
        WM_POWERBROADCAST => {
            let code = wparam as u32;
            if (code == PBT_APMRESUMEAUTOMATIC || code == PBT_APMRESUMESUSPEND) && !is_session_locked() {
                emit_reason(RecoveryReason::SleepResume);
            }
        }
        WM_WTSSESSION_CHANGE => {
            let code = wparam as u32;
            if code == WTS_SESSION_LOCK {
                SESSION_LOCKED.store(true, Ordering::Release);
            } else if code == WTS_SESSION_UNLOCK {
                SESSION_LOCKED.store(false, Ordering::Release);
                emit_reason(RecoveryReason::LockUnlock);
            }
        }
        _ => {}
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

pub fn start_session_watcher<F>(on_recovery: F)
where
    F: Fn(RecoveryReason) + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<RecoveryReason>();
    if RECOVERY_TX.set(tx).is_err() {
        return;
    }

    std::thread::spawn(move || {
        while let Ok(mut latest) = rx.recv() {
            let settle_ms = match latest {
                RecoveryReason::LockUnlock | RecoveryReason::SleepResume => 500,
                _ => 150,
            };
            loop {
                match rx.recv_timeout(Duration::from_millis(settle_ms)) {
                    Ok(next) => latest = next,
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
            if is_session_locked() {
                continue;
            }
            on_recovery(latest);
        }
    });

    std::thread::spawn(|| unsafe {
        let tb_name: Vec<u16> = "TaskbarCreated\0".encode_utf16().collect();
        let _ = TASKBAR_CREATED_MSG.set(RegisterWindowMessageW(tb_name.as_ptr()));

        let class_name: Vec<u16> = "StellariaShellSessionWatcher\0".encode_utf16().collect();
        let hinstance = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            class_name.as_ptr(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return;
        }

        let _ = WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = WTSUnRegisterSessionNotification(hwnd);
        DestroyWindow(hwnd);
    });
}
