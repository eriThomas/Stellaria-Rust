use std::ffi::c_void;
use std::ptr;

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterAudioState {
    pub volume: u32,
    pub muted: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

// CLSID_MMDeviceEnumerator = {BCDE0395-E52F-467C-8E3D-C4579291692E}
const CLSID_MM_DEVICE_ENUMERATOR: Guid = Guid {
    data1: 0xBCDE0395,
    data2: 0xE52F,
    data3: 0x467C,
    data4: [0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E],
};

// IID_IMMDeviceEnumerator = {A95664D2-9614-4F35-A746-DE8DB63617E6}
const IID_IMM_DEVICE_ENUMERATOR: Guid = Guid {
    data1: 0xA95664D2,
    data2: 0x9614,
    data3: 0x4F35,
    data4: [0xA7, 0x46, 0xDE, 0x8D, 0xB6, 0x36, 0x17, 0xE6],
};

// IID_IAudioEndpointVolume = {5CDF2C82-841E-4546-9722-0CF74078229A}
const IID_IAUDIO_ENDPOINT_VOLUME: Guid = Guid {
    data1: 0x5CDF2C82,
    data2: 0x841E,
    data3: 0x4546,
    data4: [0x97, 0x22, 0x0C, 0xF7, 0x40, 0x78, 0x22, 0x9A],
};

const CLSCTX_ALL: u32 = 23;
const COINIT_MULTITHREADED: u32 = 0;
const E_RENDER: u32 = 0;
const E_CONSOLE: u32 = 0;

#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(pv_reserved: *mut c_void, dw_co_init: u32) -> i32;
    fn CoUninitialize();
    fn CoCreateInstance(
        rclsid: *const Guid,
        p_unk_outer: *mut c_void,
        dw_cls_context: u32,
        riid: *const Guid,
        ppv: *mut *mut c_void,
    ) -> i32;
}

unsafe fn com_release(obj: *mut c_void) {
    if obj.is_null() {
        return;
    }
    let vtbl = *(obj as *const *const usize);
    let release: extern "system" fn(*mut c_void) -> u32 = std::mem::transmute(*vtbl.add(2));
    release(obj);
}

unsafe fn with_endpoint_volume<T, F: FnOnce(*mut c_void) -> Result<T, String>>(f: F) -> Result<T, String> {
    let hr_init = CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED);
    let should_uninit = hr_init == 0 || hr_init == 1; // S_OK or S_FALSE

    let res = (|| {
        let mut enumerator: *mut c_void = ptr::null_mut();
        let hr = CoCreateInstance(
            &CLSID_MM_DEVICE_ENUMERATOR,
            ptr::null_mut(),
            CLSCTX_ALL,
            &IID_IMM_DEVICE_ENUMERATOR,
            &mut enumerator,
        );
        if hr < 0 || enumerator.is_null() {
            return Err(format!("CoCreateInstance failed: 0x{:08X}", hr as u32));
        }

        // IMMDeviceEnumerator::GetDefaultAudioEndpoint is vtable index 4
        let enum_vtbl = *(enumerator as *const *const usize);
        let get_default: extern "system" fn(*mut c_void, u32, u32, *mut *mut c_void) -> i32 =
            std::mem::transmute(*enum_vtbl.add(4));
        let mut device: *mut c_void = ptr::null_mut();
        let hr = get_default(enumerator, E_RENDER, E_CONSOLE, &mut device);
        com_release(enumerator);
        if hr < 0 || device.is_null() {
            return Err(format!("GetDefaultAudioEndpoint failed: 0x{:08X}", hr as u32));
        }

        // IMMDevice::Activate is vtable index 3
        let dev_vtbl = *(device as *const *const usize);
        let activate: extern "system" fn(
            *mut c_void,
            *const Guid,
            u32,
            *mut c_void,
            *mut *mut c_void,
        ) -> i32 = std::mem::transmute(*dev_vtbl.add(3));
        let mut endpoint_vol: *mut c_void = ptr::null_mut();
        let hr = activate(
            device,
            &IID_IAUDIO_ENDPOINT_VOLUME,
            CLSCTX_ALL,
            ptr::null_mut(),
            &mut endpoint_vol,
        );
        com_release(device);
        if hr < 0 || endpoint_vol.is_null() {
            return Err(format!("IMMDevice::Activate failed: 0x{:08X}", hr as u32));
        }

        let out = f(endpoint_vol);
        com_release(endpoint_vol);
        out
    })();

    if should_uninit {
        CoUninitialize();
    }
    res
}

/// Reads the real Windows master playback volume (0..=100) and mute state via WASAPI COM.
pub fn get_master_audio() -> Result<MasterAudioState, String> {
    unsafe {
        with_endpoint_volume(|ep| {
            let vtbl = *(ep as *const *const usize);
            // IAudioEndpointVolume::GetMasterVolumeLevelScalar = index 9
            let get_scalar: extern "system" fn(*mut c_void, *mut f32) -> i32 =
                std::mem::transmute(*vtbl.add(9));
            // IAudioEndpointVolume::GetMute = index 15
            let get_mute: extern "system" fn(*mut c_void, *mut i32) -> i32 =
                std::mem::transmute(*vtbl.add(15));

            let mut scalar: f32 = 0.0;
            let mut muted: i32 = 0;
            if get_scalar(ep, &mut scalar) < 0 {
                return Err("GetMasterVolumeLevelScalar failed".to_string());
            }
            if get_mute(ep, &mut muted) < 0 {
                return Err("GetMute failed".to_string());
            }
            Ok(MasterAudioState {
                volume: (scalar.clamp(0.0, 1.0) * 100.0).round() as u32,
                muted: muted != 0,
            })
        })
    }
}

/// Sets the real Windows master playback volume (0..=100) via WASAPI COM.
pub fn set_master_volume(volume: u32) -> Result<MasterAudioState, String> {
    let clamped = volume.min(100);
    let scalar = (clamped as f32) / 100.0;
    unsafe {
        with_endpoint_volume(|ep| {
            let vtbl = *(ep as *const *const usize);
            // IAudioEndpointVolume::SetMasterVolumeLevelScalar = index 7
            let set_scalar: extern "system" fn(*mut c_void, f32, *const Guid) -> i32 =
                std::mem::transmute(*vtbl.add(7));
            // IAudioEndpointVolume::SetMute = index 14
            let set_mute: extern "system" fn(*mut c_void, i32, *const Guid) -> i32 =
                std::mem::transmute(*vtbl.add(14));

            if set_scalar(ep, scalar, ptr::null()) < 0 {
                return Err("SetMasterVolumeLevelScalar failed".to_string());
            }
            if clamped > 0 {
                let _ = set_mute(ep, 0, ptr::null());
            }
            Ok(MasterAudioState {
                volume: clamped,
                muted: false,
            })
        })
    }
}

/// Sets or toggles the real Windows master mute state via WASAPI COM.
pub fn toggle_master_mute() -> Result<MasterAudioState, String> {
    unsafe {
        with_endpoint_volume(|ep| {
            let vtbl = *(ep as *const *const usize);
            let get_scalar: extern "system" fn(*mut c_void, *mut f32) -> i32 =
                std::mem::transmute(*vtbl.add(9));
            let set_mute: extern "system" fn(*mut c_void, i32, *const Guid) -> i32 =
                std::mem::transmute(*vtbl.add(14));
            let get_mute: extern "system" fn(*mut c_void, *mut i32) -> i32 =
                std::mem::transmute(*vtbl.add(15));

            let mut scalar: f32 = 0.0;
            let mut muted: i32 = 0;
            let _ = get_scalar(ep, &mut scalar);
            let _ = get_mute(ep, &mut muted);
            let next_muted = muted == 0;
            if set_mute(ep, if next_muted { 1 } else { 0 }, ptr::null()) < 0 {
                return Err("SetMute failed".to_string());
            }
            Ok(MasterAudioState {
                volume: (scalar.clamp(0.0, 1.0) * 100.0).round() as u32,
                muted: next_muted,
            })
        })
    }
}

/// Steps the real Windows master volume by `delta` (-100..=100).
pub fn step_master_volume(delta: i32) -> Result<MasterAudioState, String> {
    let cur = get_master_audio()?;
    let next = (cur.volume as i32 + delta).clamp(0, 100) as u32;
    set_master_volume(next)
}
