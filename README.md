# Stellaria Rust

![Stellaria on a Windows 11 desktop](docs/images/promo/hero-en.jpg)

**Stellaria Rust** is an incremental, performance-focused rewrite of [Stellaria](https://github.com/SMIDR76/stellaria-windows) (an unofficial Windows 11 port of [Caelestia](https://github.com/caelestia-dots/shell)) built on **Rust + Tauri 2 + React/TypeScript**.

The goal of this project is to replace the multi-process Zebar widget host and `.NET` runtime dependency with a **single authoritative Rust desktop shell (`stellaria-shell.exe`)** — delivering lower idle CPU and memory usage, faster startup, deterministic multi-monitor & mixed-DPI layout, and guaranteed Windows shell recovery, while preserving Stellaria's Material You design, expressive motion, and desktop panels.

---

## Why Rebuild in Rust?

- **One authoritative core state:** A single Rust state machine (`stellaria-core`) owns monitor topology, DPI math, taskbar grouping, pins, focus, and fullscreen state — feeding lightweight per-monitor WebViews instead of every widget independently enumerating Windows.
- **Zero hidden-state waste:** Telemetry, media polling, and animations pause automatically whenever a panel is hidden or the Windows session is locked (`WTS_SESSION_LOCK`).
- **Native Win32 integration:** Direct `windows-sys` calls for taskbar enumeration, alpha-transparent icon extraction, live & cached minimized window previews (`PrintWindow`), per-monitor work-area reservation (`SPI_SETWORKAREA` + `SHAppBarMessage`), network adapter telemetry, keyboard layouts, Keep-Awake, and session recovery.
- **Safety & recovery by design:** Every internal shell window is excluded from Windows `Alt+Tab` (`WS_EX_TOOLWINDOW`), native `Alt+Tab` is preserved, no administrator elevation is required, and a Rust `Drop` + panic hook guarantees the Windows taskbar and monitor work areas are restored on quit or crash.

---

## Features

- **Multi-Monitor Vertical Taskbar (`bar.html`):**
  - Per-monitor work-area reservation (including secondary displays, mixed DPI, and interior monitor edges).
  - Stable app grouping with window count badges, pinning, true alpha-transparent `.exe` icons, live hover window previews (including cached live frames for minimized windows), and floating right-click context menus.
  - Per-monitor fullscreen auto-hide: fullscreen games or videos hide *only* that monitor's bar while keeping other monitors usable.
- **Top Dashboard (`dashboard.html`):**
  - Hover the top screen edge or click the taskbar clock/weather pills to open; dismisses on outside click or `Esc`.
  - Five tabs: **Home** (calendar, quick status, profile), **Media** (now playing, artwork, playback controls, synced LRCLIB lyrics), **Performance** (live native CPU, RAM, disk, network, and GPU gauges), **Weather** (Open-Meteo forecast), and **Tools** (grouped productivity & system utilities including Keep-Awake).
- **Status Popouts (`popout.html`):**
  - **Audio:** Master controls, per-app volume sliders, and mute toggles.
  - **Network:** Active Windows adapter name, IPv4, MAC address, link speed, and live download/upload rates.
  - **Keyboard:** Installed Windows keyboard layouts with one-click switching.
  - **Notifications:** Notification history with unread badge on the taskbar, per-item dismiss, and clear all.
- **App Launcher (`launcher.html`):** Fast Start Menu application catalog search and built-in calculator.
- **Right-Edge Session Drawer (`session.html`):** Material You power drawer for Lock, Sleep, Sign out, Restart, and Shut down (always restoring the Windows shell first).
- **OSD & Dynamic Island (`osd.html`, `island.html`):** Right-edge vertical Material 3 volume OSD pill and top-center toast notifications that never steal focus.
- **Material You Theming & Localization:** Wallpaper-inspired Material 3 schemes, dark/light modes, and multi-language support (English, Brazilian Portuguese, French).

---

## Architecture

| Layer | Crate / Directory | Responsibility |
| :--- | :--- | :--- |
| **Pure Core** | `launcher/core` (`stellaria-core`) | Authoritative `ShellState`, monitor & DPI placement math, `TaskbarModel` grouping/pinning/stable ordering, `AppBar` & fullscreen policy, versioned settings, weather/lyrics parsers. |
| **Native Shell Host** | `launcher/shell` (`stellaria-shell`) | Tauri 2 host, Win32 window/task/icon/preview/network/session/fullscreen modules, allow-listed HTTPS services, tray menu, panic/drop shell recovery, and temporary headless `.NET` sidecar bridge for modules still being ported. |
| **UI Surfaces** | `launcher/frontend` | React 19 + TypeScript + Vite multi-entry UI (`bar`, `dashboard`, `launcher`, `preview`, `popout`, `session`, `osd`, `island`) sharing Stellaria's Material You CSS tokens and motion curves. |

---

## Migration Roadmap

- [x] **Milestone 0 — Baseline & Scaffolding:** Cargo workspace, React/TS/Vite/Vitest setup, structured JSONL logging, isolated app identity.
- [x] **Milestone 1 — Top Dashboard Pilot:** Full dashboard parity, native telemetry, Open-Meteo weather, LRCLIB synced lyrics, top-edge hover auto-show/hide.
- [x] **Milestone 2 — Rust State Core & Multi-Monitor Host:** Authoritative `ShellState`, 4-monitor mixed-DPI host, lock/unlock & sleep/resume recovery.
- [x] **Milestone 3 — Standalone Rust Taskbar:** `stellaria-shell.exe`, native Win32 task grouping, pins, stable ordering, alpha icons, work-area reservation.
- [x] **Milestone 4 — Previews & Context Actions:** Single/grouped/minimized cached previews, pin/close/close-all context menus, stale HWND protection.
- [x] **Milestone 5 — Fullscreen, AppBars & Multi-Monitor:** Per-monitor fullscreen auto-hide, `SPI_SETWORKAREA` + `SHAppBarMessage`, guaranteed restoration.
- [ ] **Milestone 6 — Tray & Remaining Surfaces:** Launcher, Session drawer, Status popouts, OSD, Island, and Notification badge are complete; **System Tray icons inside the bar** and **Master Volume / Audio Device switching** are next.
- [ ] **Milestone 7 — Native Rust Replacement of `.NET` Modules:** Network adapter, telemetry, taskbar, icons, previews, keyboard layouts, weather, lyrics, and power are native Rust; porting **Audio (WASAPI)**, **GPU telemetry**, **System Info**, **Media (GSMTC)**, and **Notifications** next.
- [ ] **Milestone 8 — Tools Tab:** Grouped Tools tab scaffolded; Crest-inspired Tools redesign planned.
- [ ] **Milestone 9 — Legacy Removal & Release Hardening:** Performance baselines, installer migration, and removing legacy Zebar/`.NET` code once manual acceptance passes.

---

## Building & Running from Source

### Requirements
- Windows 11 (x64)
- [Rust](https://rustup.rs) (stable MSVC toolchain)
- [Node.js](https://nodejs.org) 20+

### Build & Run
```powershell
# Debug build
powershell -ExecutionPolicy Bypass -File launcher\shell\build.ps1

# Optimized release build
powershell -ExecutionPolicy Bypass -File launcher\shell\build.ps1 -Release

# Run side-by-side (keeps Windows taskbar visible)
.\launcher\target\release\stellaria-shell.exe

# Run replacing the Windows taskbar (restored automatically on quit/crash)
.\launcher\target\release\stellaria-shell.exe --replace-taskbar
```

### Automated Tests
```powershell
# Rust core unit tests
cargo test --manifest-path launcher\Cargo.toml -p stellaria-core

# Frontend unit & component tests (Vitest + React Testing Library)
npm --prefix launcher\frontend test
```

---

## Recovery & Privacy

- **Instant Recovery:** Right-click the Stellaria tray icon and choose **Restore Windows Taskbar** or **Quit Stellaria Shell**. On exit or crash, `stellaria-shell.exe` automatically unregisters AppBars, restores every monitor's work area, and shows the Windows taskbar.
- **No Telemetry & Strict Security:** No elevation required; frontend code never calls Win32 or arbitrary commands directly; all native actions are exposed through typed, allow-listed Tauri commands. External network requests are limited to Open-Meteo (weather) and LRCLIB (lyrics), both toggleable in settings.

---

## Credits

Built on the design language, CSS tokens, and original Windows port created by [SMIDR76/stellaria-windows](https://github.com/SMIDR76/stellaria-windows) and inspired by [Caelestia](https://github.com/caelestia-dots/shell).
