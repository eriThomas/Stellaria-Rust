use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::OnceLock;
use std::time::Duration;

use stellaria_core::shell_state::RecoveryReason;
use tauri::AppHandle;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::RemoteDesktop::{
    WTSRegisterSessionNotification, WTSUnRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    RegisterClassW, RegisterWindowMessageW, TranslateMessage, MSG, PBT_APMRESUMEAUTOMATIC,
    PBT_APMRESUMESUSPEND, WM_DISPLAYCHANGE, WM_POWERBROADCAST, WM_WTSSESSION_CHANGE, WNDCLASSW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP, WTS_SESSION_LOCK, WTS_SESSION_UNLOCK,
};

use crate::monitor_host;

static RECOVERY_TX: OnceLock<mpsc::Sender<RecoveryReason>> = OnceLock::new();
static TASKBAR_CREATED_MSG: OnceLock<u32> = OnceLock::new();
static SESSION_LOCKED: AtomicBool = AtomicBool::new(false);

pub fn is_session_locked() -> bool {
    SESSION_LOCKED.load(Ordering::Acquire)
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
                // Pause all window/hover activity immediately while the workstation is locked.
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

/// Starts a native Win32 hidden tool window listening for display changes, sleep/resume,
/// lock/unlock, and Explorer (`TaskbarCreated`) restarts, coalescing bursts before reconciling.
pub fn start_session_watcher(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<RecoveryReason>();
    if RECOVERY_TX.set(tx).is_err() {
        return;
    }

    // Coalescing worker thread (debounces bursts and waits for unlock/resume desktop settle)
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
            monitor_host::reconcile_topology(&app, Some(latest));
        }
    });

    // Dedicated Win32 message pump thread (never shown, WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE)
    std::thread::spawn(|| unsafe {
        let tb_name: Vec<u16> = "TaskbarCreated\0".encode_utf16().collect();
        let _ = TASKBAR_CREATED_MSG.set(RegisterWindowMessageW(tb_name.as_ptr()));

        let class_name: Vec<u16> = "StellariaSessionWatcher\0".encode_utf16().collect();
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
