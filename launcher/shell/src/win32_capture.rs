use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use stellaria_core::preview_cache::PreviewEntry;
use stellaria_core::taskbar_model::WindowIdentity;
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    GetObjectW, ReleaseDC, SelectObject, StretchBlt, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, HALFTONE, HDC, SetStretchBltMode, SRCCOPY,
};
use windows_sys::Win32::UI::Shell::ExtractIconExW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, GetIconInfo, GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindow,
    ICONINFO,
};

#[link(name = "user32")]
extern "system" {
    fn PrintWindow(hwnd: HWND, hdc_blt: HDC, n_flags: u32) -> i32;
}

const PW_RENDERFULLCONTENT: u32 = 2;
const MAX_ICON_CACHE: usize = 256;

static ICON_CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();

fn icon_cache() -> &'static Mutex<HashMap<String, Option<String>>> {
    ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i];
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };
        let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
        out.push(if i + 1 < input.len() {
            TABLE[((triple >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if i + 2 < input.len() {
            TABLE[(triple & 0x3f) as usize] as char
        } else {
            '='
        });
        i += 3;
    }
    out
}

/// Encodes a top-down 32-bit BGRA pixel buffer into a standard BMP file byte vector.
fn encode_bgra_as_bmp(width: u32, height: u32, bgra: &[u8]) -> Vec<u8> {
    let pixel_bytes = bgra.len() as u32;
    let file_size = 54 + pixel_bytes;
    let mut bmp = Vec::with_capacity(file_size as usize);
    // BITMAPFILEHEADER (14 bytes)
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&file_size.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&54u32.to_le_bytes());
    // BITMAPINFOHEADER (40 bytes)
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&(width as i32).to_le_bytes());
    bmp.extend_from_slice(&(-(height as i32)).to_le_bytes()); // top-down
    bmp.extend_from_slice(&1u16.to_le_bytes()); // planes
    bmp.extend_from_slice(&32u16.to_le_bytes()); // 32bpp BGRA
    bmp.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    bmp.extend_from_slice(&pixel_bytes.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(bgra);
    bmp
}

pub fn extract_app_icon_data_url(exe_path: &str) -> Option<String> {
    if exe_path.trim().is_empty() {
        return None;
    }
    let key = exe_path.to_ascii_lowercase();
    if let Ok(cache) = icon_cache().lock() {
        if let Some(found) = cache.get(&key) {
            return found.clone();
        }
    }

    let extracted = extract_icon_uncached(exe_path);
    if let Ok(mut cache) = icon_cache().lock() {
        if cache.len() >= MAX_ICON_CACHE {
            cache.clear();
        }
        cache.insert(key, extracted.clone());
    }
    extracted
}

fn extract_icon_uncached(exe_path: &str) -> Option<String> {
    let wide: Vec<u16> = exe_path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let mut large_icon = std::ptr::null_mut();
        let count = ExtractIconExW(wide.as_ptr(), 0, &mut large_icon, std::ptr::null_mut(), 1);
        if count == 0 || large_icon.is_null() {
            return None;
        }
        let mut icon_info: ICONINFO = std::mem::zeroed();
        if GetIconInfo(large_icon, &mut icon_info) == 0 {
            DestroyIcon(large_icon);
            return None;
        }

        let result = if !icon_info.hbmColor.is_null() {
            let mut bm: BITMAP = std::mem::zeroed();
            if GetObjectW(
                icon_info.hbmColor.cast(),
                std::mem::size_of::<BITMAP>() as i32,
                (&mut bm as *mut BITMAP).cast(),
            ) > 0
                && bm.bmWidth > 0
                && bm.bmHeight > 0
            {
                let w = bm.bmWidth as u32;
                let h = bm.bmHeight as u32;
                let screen_dc = GetDC(std::ptr::null_mut());
                let mut bi: BITMAPINFO = std::mem::zeroed();
                bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                bi.bmiHeader.biWidth = w as i32;
                bi.bmiHeader.biHeight = -(h as i32);
                bi.bmiHeader.biPlanes = 1;
                bi.bmiHeader.biBitCount = 32;
                bi.bmiHeader.biCompression = BI_RGB;
                let mut pixels = vec![0u8; (w * h * 4) as usize];
                let got = GetDIBits(
                    screen_dc,
                    icon_info.hbmColor,
                    0,
                    h,
                    pixels.as_mut_ptr().cast(),
                    &mut bi,
                    DIB_RGB_COLORS,
                );
                ReleaseDC(std::ptr::null_mut(), screen_dc);
                if got > 0 {
                    let bmp = encode_bgra_as_bmp(w, h, &pixels);
                    Some(format!("data:image/bmp;base64,{}", base64_encode(&bmp)))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if !icon_info.hbmColor.is_null() {
            DeleteObject(icon_info.hbmColor.cast());
        }
        if !icon_info.hbmMask.is_null() {
            DeleteObject(icon_info.hbmMask.cast());
        }
        DestroyIcon(large_icon);
        result
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Captures a scaled live preview of `hwnd` via `PrintWindow` (with safe fallback for minimized/protected windows).
pub fn capture_window_preview(
    hwnd_raw: u64,
    expected_pid: u32,
    session_generation: u64,
) -> PreviewEntry {
    let identity = WindowIdentity {
        hwnd: hwnd_raw,
        pid: expected_pid,
        session_generation,
    };
    let ts = now_ms();
    let hwnd = hwnd_raw as usize as HWND;
    if hwnd.is_null() || unsafe { IsWindow(hwnd) } == 0 {
        return PreviewEntry {
            identity,
            width: 0,
            height: 0,
            png_base64: None,
            captured_at_ms: ts,
            fallback_reason: Some("stale".into()),
        };
    }

    let mut actual_pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut actual_pid) };
    if expected_pid != 0 && actual_pid != expected_pid {
        return PreviewEntry {
            identity,
            width: 0,
            height: 0,
            png_base64: None,
            captured_at_ms: ts,
            fallback_reason: Some("pid-mismatch".into()),
        };
    }

    if unsafe { IsIconic(hwnd) } != 0 {
        return PreviewEntry {
            identity,
            width: 0,
            height: 0,
            png_base64: None,
            captured_at_ms: ts,
            fallback_reason: Some("minimized".into()),
        };
    }

    unsafe {
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return PreviewEntry {
                identity,
                width: 0,
                height: 0,
                png_base64: None,
                captured_at_ms: ts,
                fallback_reason: Some("no-rect".into()),
            };
        }
        let src_w = (rect.right - rect.left).max(1);
        let src_h = (rect.bottom - rect.top).max(1);
        let target_w = 280i32.min(src_w);
        let target_h = ((src_h as f64 / src_w as f64) * target_w as f64)
            .round()
            .clamp(80.0, 200.0) as i32;

        let screen_dc = GetDC(std::ptr::null_mut());
        let src_dc = CreateCompatibleDC(screen_dc);
        let src_bmp = CreateCompatibleBitmap(screen_dc, src_w, src_h);
        let old_src = SelectObject(src_dc, src_bmp.cast());

        let printed = PrintWindow(hwnd, src_dc, PW_RENDERFULLCONTENT) != 0
            || PrintWindow(hwnd, src_dc, 0) != 0;

        let data_url = if printed {
            let dst_dc = CreateCompatibleDC(screen_dc);
            let dst_bmp = CreateCompatibleBitmap(screen_dc, target_w, target_h);
            let old_dst = SelectObject(dst_dc, dst_bmp.cast());
            SetStretchBltMode(dst_dc, HALFTONE);
            let blitted = if src_w == target_w && src_h == target_h {
                BitBlt(dst_dc, 0, 0, target_w, target_h, src_dc, 0, 0, SRCCOPY) != 0
            } else {
                StretchBlt(dst_dc, 0, 0, target_w, target_h, src_dc, 0, 0, src_w, src_h, SRCCOPY) != 0
            };

            let encoded = if blitted {
                let mut bi: BITMAPINFO = std::mem::zeroed();
                bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                bi.bmiHeader.biWidth = target_w;
                bi.bmiHeader.biHeight = -target_h;
                bi.bmiHeader.biPlanes = 1;
                bi.bmiHeader.biBitCount = 32;
                bi.bmiHeader.biCompression = BI_RGB;
                let mut pixels = vec![0u8; (target_w * target_h * 4) as usize];
                let got = GetDIBits(
                    dst_dc,
                    dst_bmp,
                    0,
                    target_h as u32,
                    pixels.as_mut_ptr().cast(),
                    &mut bi,
                    DIB_RGB_COLORS,
                );
                if got > 0 {
                    // Force opaque alpha for captured window frames
                    for px in pixels.chunks_exact_mut(4) {
                        px[3] = 255;
                    }
                    let bmp = encode_bgra_as_bmp(target_w as u32, target_h as u32, &pixels);
                    Some(format!("data:image/bmp;base64,{}", base64_encode(&bmp)))
                } else {
                    None
                }
            } else {
                None
            };

            SelectObject(dst_dc, old_dst);
            DeleteObject(dst_bmp.cast());
            DeleteDC(dst_dc);
            encoded
        } else {
            None
        };

        SelectObject(src_dc, old_src);
        DeleteObject(src_bmp.cast());
        DeleteDC(src_dc);
        ReleaseDC(std::ptr::null_mut(), screen_dc);

        PreviewEntry {
            identity,
            width: target_w as u32,
            height: target_h as u32,
            png_base64: data_url.clone(),
            captured_at_ms: ts,
            fallback_reason: if data_url.is_some() {
                None
            } else {
                Some("capture-unavailable".into())
            },
        }
    }
}
