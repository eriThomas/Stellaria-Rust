use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use stellaria_core::telemetry::{GpuTelemetry, MediaAction, MediaSessionInfo, MediaSnapshot, SystemInfo};

use crate::environment;

const HELPER_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

struct HelperCredentials {
    port: u16,
    token: String,
}

fn read_credentials() -> Result<HelperCredentials, String> {
    let path = environment("LOCALAPPDATA").join("Stellaria").join("launcher.json");
    let raw = std::fs::read_to_string(&path).map_err(|_| "helper-offline".to_string())?;
    let value: Value = serde_json::from_str(&raw).map_err(|_| "helper-offline".to_string())?;
    let port = value["port"]
        .as_u64()
        .filter(|p| (47610..=47619).contains(p))
        .ok_or_else(|| "helper-offline".to_string())? as u16;
    let token = value["token"]
        .as_str()
        .filter(|t| t.len() == 64)
        .ok_or_else(|| "helper-offline".to_string())?
        .to_string();
    Ok(HelperCredentials { port, token })
}

fn encode_client_text_frame(payload: &[u8], mask_key: [u8; 4]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 14);
    frame.push(0x81); // FIN + text opcode
    let len = payload.len();
    if len < 126 {
        frame.push(0x80 | (len as u8));
    } else if len <= u16::MAX as usize {
        frame.push(0x80 | 126);
        frame.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        frame.push(0x80 | 127);
        frame.extend_from_slice(&(len as u64).to_be_bytes());
    }
    frame.extend_from_slice(&mask_key);
    for (index, byte) in payload.iter().enumerate() {
        frame.push(byte ^ mask_key[index & 3]);
    }
    frame
}

fn read_server_text_frame(stream: &mut TcpStream) -> Result<String, String> {
    loop {
        let mut header = [0u8; 2];
        stream.read_exact(&mut header).map_err(|e| e.to_string())?;
        let opcode = header[0] & 0x0f;
        let masked = (header[1] & 0x80) != 0;
        let len_code = header[1] & 0x7f;
        let payload_len = match len_code {
            0..=125 => len_code as usize,
            126 => {
                let mut ext = [0u8; 2];
                stream.read_exact(&mut ext).map_err(|e| e.to_string())?;
                u16::from_be_bytes(ext) as usize
            }
            127 => {
                let mut ext = [0u8; 8];
                stream.read_exact(&mut ext).map_err(|e| e.to_string())?;
                u64::from_be_bytes(ext) as usize
            }
            _ => unreachable!(),
        };
        if payload_len > MAX_FRAME_BYTES {
            return Err("frame-too-large".to_string());
        }
        let mut mask = [0u8; 4];
        if masked {
            stream.read_exact(&mut mask).map_err(|e| e.to_string())?;
        }
        let mut data = vec![0u8; payload_len];
        stream.read_exact(&mut data).map_err(|e| e.to_string())?;
        if masked {
            for (i, b) in data.iter_mut().enumerate() {
                *b ^= mask[i & 3];
            }
        }
        match opcode {
            0x01 => return String::from_utf8(data).map_err(|_| "invalid-utf8".to_string()),
            0x08 => return Err("helper-closed".to_string()),
            0x09 => {
                // Respond to ping with pong
                let pong = encode_client_text_frame(&data, [0x12, 0x34, 0x56, 0x78]);
                let mut pong_frame = pong;
                pong_frame[0] = 0x8A;
                let _ = stream.write_all(&pong_frame);
            }
            _ => continue,
        }
    }
}

fn pseudo_mask() -> [u8; 4] {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0xA5A5_5A5A);
    nanos.to_le_bytes()
}

fn request_helper_sync(method: &str, params: Value) -> Result<Value, String> {
    let creds = read_credentials()?;
    let addr = SocketAddr::from(([127, 0, 0, 1], creds.port));
    let mut stream = TcpStream::connect_timeout(&addr, HELPER_TIMEOUT).map_err(|_| "helper-offline".to_string())?;
    let _ = stream.set_read_timeout(Some(HELPER_TIMEOUT));
    let _ = stream.set_write_timeout(Some(HELPER_TIMEOUT));

    let handshake = format!(
        "GET / HTTP/1.1\r\n\
         Host: 127.0.0.1:{}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Origin: http://tauri.localhost\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
         Sec-WebSocket-Version: 13\r\n\r\n",
        creds.port
    );
    stream.write_all(handshake.as_bytes()).map_err(|e| e.to_string())?;

    let mut response_buf = Vec::with_capacity(512);
    let mut byte = [0u8; 1];
    while response_buf.len() < 4096 {
        stream.read_exact(&mut byte).map_err(|e| e.to_string())?;
        response_buf.push(byte[0]);
        if response_buf.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header_str = String::from_utf8_lossy(&response_buf);
    if !header_str.starts_with("HTTP/1.1 101") {
        return Err("helper-handshake-rejected".to_string());
    }

    let auth = json!({
        "kind": "auth",
        "token": creds.token,
        "widget": "dashboard-pilot",
        "monitor": ""
    });
    stream
        .write_all(&encode_client_text_frame(auth.to_string().as_bytes(), pseudo_mask()))
        .map_err(|e| e.to_string())?;

    // Wait for hello
    let mut authed = false;
    for _ in 0..16 {
        let text = read_server_text_frame(&mut stream)?;
        let msg: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        if msg["kind"] == "hello" {
            authed = true;
            break;
        }
    }
    if !authed {
        return Err("helper-auth-failed".to_string());
    }

    let req_id = "dash-1";
    let req = json!({
        "kind": "request",
        "id": req_id,
        "method": method,
        "params": params
    });
    stream
        .write_all(&encode_client_text_frame(req.to_string().as_bytes(), pseudo_mask()))
        .map_err(|e| e.to_string())?;

    for _ in 0..32 {
        let text = read_server_text_frame(&mut stream)?;
        let msg: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        if msg["kind"] == "reply" && msg["id"] == req_id {
            if msg["ok"].as_bool().unwrap_or(false) {
                return Ok(msg["data"].clone());
            }
            let code = msg["error"]["code"].as_str().unwrap_or("failed");
            return Err(code.to_string());
        }
    }

    Err("helper-timeout".to_string())
}

pub async fn fetch_system_info() -> Result<SystemInfo, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let data = request_helper_sync("system.info", json!({}))?;
        Ok(SystemInfo {
            os: data["os"].as_str().unwrap_or("Windows").to_string(),
            cpu: data["cpu"].as_str().unwrap_or("—").to_string(),
            gpu: data["gpu"].as_str().map(str::to_string),
            user: data["name"]
                .as_str()
                .or_else(|| data["user"].as_str())
                .unwrap_or_default()
                .to_string(),
            picture_base64: data["picture"].as_str().map(str::to_string),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn fetch_gpu_telemetry() -> Result<GpuTelemetry, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let data = request_helper_sync("system.gpu", json!({}))?;
        Ok(GpuTelemetry {
            name: data["name"].as_str().unwrap_or("GPU").to_string(),
            usage_percent: data["usage"].as_f64(),
            temperature_c: data["temperature"].as_f64(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn fetch_media_snapshot() -> Result<MediaSnapshot, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let data = request_helper_sync("media.artwork", json!({}))?;
        if data.is_null() {
            return Ok(MediaSnapshot::default());
        }
        let session = MediaSessionInfo {
            session_id: data["app"].as_str().unwrap_or("current").to_string(),
            title: data["title"].as_str().unwrap_or_default().to_string(),
            artist: data["artist"].as_str().unwrap_or_default().to_string(),
            album: data["album"].as_str().unwrap_or_default().to_string(),
            is_playing: data["playing"].as_bool().unwrap_or(false),
            position_secs: 0.0,
            duration_secs: 0.0,
        };
        let artwork_base64 = data["image"].as_str().map(str::to_string);
        Ok(MediaSnapshot {
            sessions: vec![session.clone()],
            current: Some(session),
            artwork_base64,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn send_media_action(action: MediaAction) -> Result<(), String> {
    let verb = action.as_helper_action();
    tauri::async_runtime::spawn_blocking(move || {
        request_helper_sync("media.control", json!({ "action": verb })).map(|_| ())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_and_unmasks_short_rfc6455_text_frame() {
        let payload = b"hello stellaria";
        let mask = [0x11, 0x22, 0x33, 0x44];
        let frame = encode_client_text_frame(payload, mask);
        assert_eq!(frame[0], 0x81);
        assert_eq!(frame[1], 0x80 | (payload.len() as u8));
        assert_eq!(&frame[2..6], &mask);
        let decoded: Vec<u8> = frame[6..]
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ mask[i & 3])
            .collect();
        assert_eq!(decoded, payload);
    }
}
