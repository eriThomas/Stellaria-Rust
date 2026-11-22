use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use stellaria_core::shell_state::{FocusSummary, MonitorOverlayState, TaskSummary};
use tauri::AppHandle;

use crate::environment;
use crate::monitor_host;

const RETRY_DELAYS_MS: [u64; 4] = [500, 1000, 2000, 5000];
const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

fn read_credentials() -> Option<(u16, String)> {
    let path = environment("LOCALAPPDATA").join("Stellaria").join("launcher.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    let value: Value = serde_json::from_str(&raw).ok()?;
    let port = value["port"]
        .as_u64()
        .filter(|p| (47610..=47619).contains(p))? as u16;
    let token = value["token"]
        .as_str()
        .filter(|t| t.len() == 64)?
        .to_string();
    Some((port, token))
}

fn encode_frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0x5A5A_A5A5);
    let mask = nanos.to_le_bytes();
    let mut frame = Vec::with_capacity(payload.len() + 14);
    frame.push(0x80 | opcode);
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
    frame.extend_from_slice(&mask);
    for (i, b) in payload.iter().enumerate() {
        frame.push(b ^ mask[i & 3]);
    }
    frame
}

fn read_text_frame(stream: &mut TcpStream) -> Result<String, std::io::Error> {
    let mut message_buf = Vec::new();
    loop {
        let mut header = [0u8; 2];
        stream.read_exact(&mut header)?;
        let fin = (header[0] & 0x80) != 0;
        let opcode = header[0] & 0x0f;
        let masked = (header[1] & 0x80) != 0;
        let len_code = header[1] & 0x7f;
        let payload_len = match len_code {
            0..=125 => len_code as usize,
            126 => {
                let mut ext = [0u8; 2];
                stream.read_exact(&mut ext)?;
                u16::from_be_bytes(ext) as usize
            }
            127 => {
                let mut ext = [0u8; 8];
                stream.read_exact(&mut ext)?;
                u64::from_be_bytes(ext) as usize
            }
            _ => unreachable!(),
        };
        if payload_len > MAX_FRAME_BYTES {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "frame too large"));
        }
        let mut mask = [0u8; 4];
        if masked {
            stream.read_exact(&mut mask)?;
        }
        let mut data = vec![0u8; payload_len];
        stream.read_exact(&mut data)?;
        if masked {
            for (i, b) in data.iter_mut().enumerate() {
                *b ^= mask[i & 3];
            }
        }
        match opcode {
            0x01 | 0x00 => {
                message_buf.extend_from_slice(&data);
                if fin {
                    return String::from_utf8(message_buf)
                        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "utf8"));
                }
            }
            0x08 => return Err(std::io::Error::new(std::io::ErrorKind::ConnectionReset, "closed")),
            0x09 => {
                let _ = stream.write_all(&encode_frame(0x0A, &data));
            }
            _ => {}
        }
    }
}

fn handle_state_topic(app: &AppHandle, topic: &str, data: &Value) {
    match topic {
        "monitors" => {
            let Some(obj) = data.as_object() else {
                return;
            };
            let mut overlays = HashMap::with_capacity(obj.len());
            for (device, info) in obj {
                overlays.insert(
                    device.clone(),
                    MonitorOverlayState {
                        fullscreen: info["fullscreen"].as_bool().unwrap_or(false),
                        covered: info["covered"].as_bool().unwrap_or(false),
                    },
                );
            }
            monitor_host::apply_helper_overlays(app, overlays);
        }
        "tasks" => {
            let list = data
                .as_array()
                .or_else(|| data["items"].as_array())
                .or_else(|| data["tasks"].as_array());
            let Some(items) = list else {
                return;
            };
            let tasks = items
                .iter()
                .filter_map(|item| {
                    let id = item["id"].as_u64().or_else(|| item["hwnd"].as_u64())?;
                    Some(TaskSummary {
                        id,
                        title: item["title"].as_str().unwrap_or_default().to_string(),
                        exe: item["exe"]
                            .as_str()
                            .or_else(|| item["process"].as_str())
                            .unwrap_or_default()
                            .to_string(),
                        monitor_id: item["monitor"].as_str().unwrap_or_default().to_string(),
                        minimized: item["minimized"].as_bool().unwrap_or(false),
                        active: item["active"].as_bool().unwrap_or(false),
                    })
                })
                .collect();
            monitor_host::apply_helper_tasks(app, tasks);
        }
        "focus" => {
            let focus = FocusSummary {
                hwnd: data["hwnd"].as_u64().or_else(|| data["id"].as_u64()),
                title: data["title"].as_str().unwrap_or_default().to_string(),
                exe: data["exe"]
                    .as_str()
                    .or_else(|| data["process"].as_str())
                    .unwrap_or_default()
                    .to_string(),
                monitor_id: data["monitor"].as_str().unwrap_or_default().to_string(),
            };
            monitor_host::apply_helper_focus(app, focus);
        }
        _ => {}
    }
}

/// Starts a single background WebSocket bridge subscribing to `.NET helper` state topics.
pub fn start_helper_stream(app: AppHandle) {
    std::thread::spawn(move || {
        let mut attempt: usize = 0;
        loop {
            let Some((port, token)) = read_credentials() else {
                monitor_host::set_helper_connected(&app, false);
                let delay = RETRY_DELAYS_MS[attempt.min(RETRY_DELAYS_MS.len() - 1)];
                attempt = attempt.saturating_add(1);
                std::thread::sleep(Duration::from_millis(delay));
                continue;
            };

            let addr = SocketAddr::from(([127, 0, 0, 1], port));
            let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_secs(3)) else {
                monitor_host::set_helper_connected(&app, false);
                let delay = RETRY_DELAYS_MS[attempt.min(RETRY_DELAYS_MS.len() - 1)];
                attempt = attempt.saturating_add(1);
                std::thread::sleep(Duration::from_millis(delay));
                continue;
            };

            let _ = stream.set_read_timeout(Some(Duration::from_secs(20)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

            let handshake = format!(
                "GET / HTTP/1.1\r\n\
                 Host: 127.0.0.1:{port}\r\n\
                 Upgrade: websocket\r\n\
                 Connection: Upgrade\r\n\
                 Origin: http://tauri.localhost\r\n\
                 Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                 Sec-WebSocket-Version: 13\r\n\r\n"
            );
            if stream.write_all(handshake.as_bytes()).is_err() {
                continue;
            }

            let mut resp = Vec::with_capacity(256);
            let mut b = [0u8; 1];
            let mut handshake_ok = false;
            while resp.len() < 4096 && stream.read_exact(&mut b).is_ok() {
                resp.push(b[0]);
                if resp.ends_with(b"\r\n\r\n") {
                    handshake_ok = String::from_utf8_lossy(&resp).starts_with("HTTP/1.1 101");
                    break;
                }
            }
            if !handshake_ok {
                std::thread::sleep(Duration::from_millis(1000));
                continue;
            }

            let auth = json!({
                "kind": "auth",
                "token": token,
                "widget": "shell-core",
                "monitor": ""
            });
            if stream.write_all(&encode_frame(0x01, auth.to_string().as_bytes())).is_err() {
                continue;
            }

            loop {
                match read_text_frame(&mut stream) {
                    Ok(text) => {
                        let Ok(msg) = serde_json::from_str::<Value>(&text) else {
                            continue;
                        };
                        match msg["kind"].as_str() {
                            Some("hello") => {
                                attempt = 0;
                                monitor_host::set_helper_connected(&app, true);
                            }
                            Some("state") => {
                                if let Some(topic) = msg["topic"].as_str() {
                                    handle_state_topic(&app, topic, &msg["data"]);
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::TimedOut || err.kind() == std::io::ErrorKind::WouldBlock => {
                        // Send lightweight keepalive heartbeat on idle read timeout.
                        let hb = json!({ "kind": "heartbeat", "visible": false });
                        if stream.write_all(&encode_frame(0x01, hb.to_string().as_bytes())).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            monitor_host::set_helper_connected(&app, false);
            let delay = RETRY_DELAYS_MS[attempt.min(RETRY_DELAYS_MS.len() - 1)];
            attempt = attempt.saturating_add(1);
            std::thread::sleep(Duration::from_millis(delay));
        }
    });
}
