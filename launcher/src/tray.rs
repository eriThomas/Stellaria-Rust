use serde::Deserialize;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};

const TRAY: &str = "main";
const MAX_LABEL: usize = 80;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayLabels {
    open: String,
    start: String,
    stop: String,
    quit: String,
    tooltip: String,
    update: Option<String>,
}

fn clean(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).take(MAX_LABEL).collect()
}

pub fn reveal<R: Runtime>(app: &AppHandle<R>, page: Option<&str>) {
    if let Some(window) = app.get_webview_window("main") {
        let hidden = !window.is_visible().unwrap_or(true);
        crate::foreground::present(&window);
        if hidden {
            let _ = app.emit("reopened", ());
        }
    }
    if let Some(page) = page {
        let _ = app.emit("open-page", page);
    }
}

fn menu<R: Runtime>(app: &AppHandle<R>, labels: &TrayLabels, dashboard_pilot: bool) -> tauri::Result<Menu<R>> {
    let menu = Menu::new(app)?;
    if let Some(update) = labels.update.as_deref() {
        menu.append(&MenuItem::with_id(app, "update", clean(update), true, None::<&str>)?)?;
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    menu.append(&MenuItem::with_id(app, "open", clean(&labels.open), true, None::<&str>)?)?;
    if dashboard_pilot {
        menu.append(&MenuItem::with_id(app, "dashboard-pilot", "★ Dashboard (Pilot)", true, None::<&str>)?)?;
        menu.append(&MenuItem::with_id(app, "synthetic-bars", "★ Toggle Synthetic Bars (M2)", true, None::<&str>)?)?;
    }
    menu.append(&MenuItem::with_id(app, "start", clean(&labels.start), true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "stop", clean(&labels.stop), true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", clean(&labels.quit), true, None::<&str>)?)?;
    Ok(menu)
}

pub fn create<R: Runtime>(app: &AppHandle<R>, dashboard_pilot: bool) -> tauri::Result<()> {
    let labels = TrayLabels {
        open: "Ouvrir Stellaria".into(),
        start: "Démarrer le thème".into(),
        stop: "Arrêter le thème".into(),
        quit: "Quitter".into(),
        tooltip: if dashboard_pilot { "Stellaria Experimental (M2)".into() } else { "Stellaria".into() },
        update: None,
    };
    let mut builder = TrayIconBuilder::with_id(TRAY)
        .tooltip(&labels.tooltip)
        .menu(&menu(app, &labels, dashboard_pilot)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => reveal(app, None),
            "dashboard-pilot" => crate::dashboard::toggle_from_tray(app),
            "synthetic-bars" => crate::monitor_host::toggle_synthetic_bars(app),
            "update" => reveal(app, Some("install")),
            "start" => {
                std::thread::spawn(|| crate::run_helper("start"));
            }
            "stop" => {
                std::thread::spawn(|| crate::run_helper("restore-shell"));
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                reveal(tray.app_handle(), None);
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

#[tauri::command]
pub fn tray_labels(app: AppHandle, labels: TrayLabels) -> Result<(), String> {
    let tray = app.tray_by_id(TRAY).ok_or("tray-missing")?;
    let dashboard_pilot = app
        .try_state::<crate::dashboard::DashboardHostState>()
        .and_then(|s| s.core.lock().ok().map(|c| c.flags().dashboard_pilot))
        .unwrap_or(false);
    let tooltip = if dashboard_pilot {
        "Stellaria Experimental (M2)".to_string()
    } else {
        clean(&labels.tooltip)
    };
    tray.set_menu(Some(menu(&app, &labels, dashboard_pilot).map_err(|error| error.to_string())?)).map_err(|error| error.to_string())?;
    tray.set_tooltip(Some(tooltip)).map_err(|error| error.to_string())
}
