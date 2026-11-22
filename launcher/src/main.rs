#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dashboard;
mod foreground;
mod helper_bridge;
mod helper_stream;
mod installation;
mod monitor_host;
mod session_watcher;
mod telemetry;
mod tray;
mod update;
mod web_services;

use std::io::{Read, Seek, SeekFrom};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use serde::Serialize;
use tauri::ipc::Response;
use tauri::{Emitter, Manager};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const HELPER_VERBS: [&str; 5] = ["start", "restore-shell", "install", "uninstall", "shutdown"];
const LINK_PREFIX: &str = "stellaria://";
const SETTINGS_LINKS: [&str; 1] = ["ms-settings:appsfeatures"];
const LOG_BYTES: u64 = 96 * 1024;
const MAX_WALLPAPER: u64 = 64 * 1024 * 1024;
const WALLPAPER_TYPES: [&str; 6] = ["jpg", "jpeg", "png", "bmp", "webp", "gif"];

#[derive(Serialize)]
struct Runtime {
    port: u16,
    token: String,
}

fn environment(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_default())
}

fn helper_path() -> PathBuf {
    environment("USERPROFILE").join(".stellaria").join("bin").join("stellaria-helper.exe")
}

fn run_helper(verb: &str) -> Result<i32, String> {
    if !HELPER_VERBS.contains(&verb) {
        return Err("unknown-verb".into());
    }
    let path = helper_path();
    if !path.exists() {
        return Err("helper-missing".into());
    }
    Command::new(path)
        .arg(format!("--{verb}"))
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|status| status.code().unwrap_or(-1))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn runtime() -> Result<Runtime, String> {
    let path = environment("LOCALAPPDATA").join("Stellaria").join("launcher.json");
    let text = std::fs::read_to_string(&path).map_err(|_| "helper-offline".to_string())?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let port = value["port"].as_u64().filter(|port| (47610..=47619).contains(port)).ok_or("helper-offline")?;
    let token = value["token"].as_str().filter(|token| token.len() == 64).ok_or("helper-offline")?;
    Ok(Runtime { port: port as u16, token: token.to_string() })
}

#[tauri::command]
async fn helper(verb: String) -> Result<i32, String> {
    tauri::async_runtime::spawn_blocking(move || run_helper(&verb)).await.map_err(|error| error.to_string())?
}

#[tauri::command]
async fn installation() -> Result<installation::Installation, String> {
    tauri::async_runtime::spawn_blocking(installation::detect).await.map_err(|error| error.to_string())
}

#[tauri::command]
async fn install_zebar() -> Result<i32, String> {
    tauri::async_runtime::spawn_blocking(installation::install_zebar).await.map_err(|error| error.to_string())?
}

#[tauri::command]
fn open_folder(target: String) -> Result<(), String> {
    let path = installation::folder(&target).ok_or("unknown-folder")?;
    if !path.is_dir() {
        return Err("folder-missing".into());
    }
    Command::new(environment("WINDIR").join("explorer.exe")).arg(path).spawn().map(|_| ()).map_err(|error| error.to_string())
}

#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    let allowed = SETTINGS_LINKS.contains(&url.as_str())
        || (url.starts_with("https://") && url.len() < 512 && !url.chars().any(|c| c.is_whitespace() || c == '"' || c == '^' || c == '&'));
    if !allowed {
        return Err("refused".into());
    }
    Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", &url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn initial_link() -> Option<String> {
    std::env::args().find(|argument| argument.starts_with(LINK_PREFIX))
}

#[tauri::command]
fn log_tail(lines: usize) -> Result<Vec<String>, String> {
    let path = environment("LOCALAPPDATA").join("Stellaria").join("logs").join("helper.log");
    let mut file = std::fs::File::open(path).map_err(|_| "log-missing".to_string())?;
    let length = file.metadata().map_err(|error| error.to_string())?.len();
    file.seek(SeekFrom::Start(length.saturating_sub(LOG_BYTES))).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|error| error.to_string())?;
    let text = String::from_utf8_lossy(&bytes);
    let all: Vec<&str> = text.lines().filter(|line| !line.trim().is_empty()).collect();
    let skip = all.len().saturating_sub(lines.min(400));
    Ok(all[skip..].iter().map(|line| line.to_string()).collect())
}

#[tauri::command]
async fn wallpaper() -> Result<Response, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let desktop = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Control Panel\Desktop").map_err(|_| "no-wallpaper".to_string())?;
        let path: String = desktop.get_value("WallPaper").map_err(|_| "no-wallpaper".to_string())?;
        let path = PathBuf::from(path);
        let extension = path.extension().and_then(|value| value.to_str()).unwrap_or_default().to_ascii_lowercase();
        let size = std::fs::metadata(&path).map_err(|_| "no-wallpaper".to_string())?.len();
        if !WALLPAPER_TYPES.contains(&extension.as_str()) || size > MAX_WALLPAPER {
            return Err("no-wallpaper".to_string());
        }
        std::fs::read(&path).map(Response::new).map_err(|_| "no-wallpaper".to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

fn main() {
    let dashboard_state = dashboard::DashboardHostState::initialize(&environment("LOCALAPPDATA"));
    let dashboard_pilot = dashboard_state
        .core
        .lock()
        .map(|core| core.flags().dashboard_pilot)
        .unwrap_or(false);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, arguments, _| {
            let is_pilot = app
                .try_state::<dashboard::DashboardHostState>()
                .and_then(|s| s.core.lock().ok().map(|c| c.flags().dashboard_pilot))
                .unwrap_or(false);
            if is_pilot && !arguments.iter().any(|arg| arg == "--launcher") {
                dashboard::toggle_from_tray(app);
            } else {
                tray::reveal(app, None);
            }
            if let Some(link) = arguments.into_iter().find(|argument| argument.starts_with(LINK_PREFIX)) {
                let _ = app.emit("deep-link", link);
            }
        }))
        .manage(update::UpdateState::default())
        .manage(dashboard_state)
        .manage(monitor_host::ShellHostState::initialize())
        .manage(web_services::WebServicesState::default())
        .setup(move |app| {
            tray::create(app.handle(), dashboard_pilot)?;
            if dashboard_pilot {
                monitor_host::reconcile_topology(app.handle(), None);
                helper_stream::start_helper_stream(app.handle().clone());
                session_watcher::start_session_watcher(app.handle().clone());
                dashboard::start_telemetry_loop(app.handle().clone());
                dashboard::start_hover_edge_watcher(app.handle().clone());
                if !std::env::args().any(|arg| arg == "--hidden") {
                    dashboard::toggle_from_tray(app.handle());
                }
            }
            let state = app.state::<dashboard::DashboardHostState>();
            state.logger.timing("startup", "rust_setup_complete", state.elapsed_ms());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            runtime,
            helper,
            installation,
            install_zebar,
            open_folder,
            open_external,
            initial_link,
            log_tail,
            wallpaper,
            tray::tray_labels,
            update::check_update,
            update::download_update,
            update::install_update,
            update::installed,
            update::release_notes,
            foreground::present_window,
            dashboard::dashboard_get_snapshot,
            dashboard::dashboard_set_visibility,
            dashboard::dashboard_set_active_tab,
            dashboard::dashboard_update_settings,
            dashboard::dashboard_get_sysinfo,
            dashboard::dashboard_get_telemetry,
            dashboard::dashboard_get_media,
            dashboard::dashboard_media_action,
            dashboard::dashboard_refresh_weather,
            dashboard::dashboard_get_lyrics,
            monitor_host::shell_get_snapshot,
            monitor_host::shell_set_synthetic_bars_enabled
        ])
        .run(tauri::generate_context!())
        .expect("Stellaria could not start");
}
