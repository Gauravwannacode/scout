pub mod audio;
pub mod pipeline;
pub mod scheduler;
pub mod settings;
pub mod sources;

use audio::AudioState;
use pipeline::PipelineResult;
use scheduler::SchedulerState;
use settings::Settings;
use sources::FetchReport;
use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_sql::{Migration, MigrationKind};

/// The local database. Lives in the OS app-data directory, not next to the
/// binary, so an app update never risks the user's data.
const DB_URL: &str = "sqlite:scout.db";

fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "initial schema",
            sql: include_str!("../migrations/001_init.sql"),
            kind: MigrationKind::Up,
        },
        // Added rather than folded into 001: an existing install already has
        // that migration recorded, so editing it would never run.
        Migration {
            version: 2,
            description: "item location",
            sql: include_str!("../migrations/002_location.sql"),
            kind: MigrationKind::Up,
        },
    ]
}

/// Sweeps every news source and hands the raw items back to the frontend.
///
/// Scoring and clustering are deliberately not done here — this returns
/// exactly what the sources said, so the adapters can be verified on their own
/// before anything interprets them.
#[tauri::command]
async fn fetch_news() -> Result<FetchReport, String> {
    Ok(sources::fetch_all().await)
}

/// The full sweep: fetch, cluster, measure reach, score significance.
#[tauri::command]
async fn refresh() -> Result<PipelineResult, String> {
    Ok(pipeline::run().await)
}

#[tauri::command]
fn get_settings() -> Settings {
    settings::load()
}

#[tauri::command]
fn save_settings(value: Settings) -> Result<(), String> {
    settings::save(&value)
}

/// What the always-on-top overlay is currently showing.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MiniMode {
    /// A running focus session or stopwatch, parked in a corner of the screen.
    Timer,
    /// An alarm going off. Carries its own text because the overlay may be
    /// opened by the scheduler while the main window is hidden and has no
    /// React state to read from.
    #[serde(rename_all = "camelCase")]
    Alarm { label: String, at: String },
}

/// Shows the small overlay and tells it what to display.
///
/// The payload is emitted after showing rather than passed as a URL parameter
/// so that a window already on screen switches modes instead of reloading.
pub fn show_mini(app: &tauri::AppHandle, mode: MiniMode) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("mini") {
        window.show()?;
        window.set_always_on_top(true)?;
        // An alarm should take focus; a timer overlay should not steal it
        // from whatever he is working in.
        if matches!(mode, MiniMode::Alarm { .. }) {
            let _ = window.set_focus();
        }
        app.emit_to("mini", "scout://mini-mode", mode)?;
    }
    Ok(())
}

#[tauri::command]
fn open_mini(app: tauri::AppHandle) -> Result<(), String> {
    show_mini(&app, MiniMode::Timer).map_err(|e| e.to_string())
}

#[tauri::command]
fn close_mini(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("mini") {
        let _ = window.hide();
    }
}

/// Brings the full app back from the overlay or the tray.
#[tauri::command]
fn show_main(app: tauri::AppHandle) {
    show_window(&app);
}

/// Rings the alarm now, exactly as the scheduler would.
///
/// Alarms are the one feature that cannot be checked by waiting — a bug only
/// shows up at 06:30, by which point it has already failed. This exercises the
/// real path: same sound, same overlay, same dismissal.
#[tauri::command]
fn test_alarm(app: tauri::AppHandle, state: tauri::State<'_, AudioState>) -> Result<(), String> {
    state.start();
    show_mini(
        &app,
        MiniMode::Alarm {
            label: "Test alarm".into(),
            at: "now".into(),
        },
    )
    .map_err(|e| e.to_string())
}

/// Silences a ringing alarm and puts the overlay away.
#[tauri::command]
fn dismiss_alarm(app: tauri::AppHandle, state: tauri::State<'_, AudioState>) {
    state.stop();
    if let Some(window) = app.get_webview_window("mini") {
        let _ = window.hide();
    }
    let _ = app.emit("scout://alarm-dismissed", ());
}

/// Brings the window back from the tray.
fn show_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Scout", true, None::<&str>)?;
    let refresh_now = MenuItem::with_id(app, "refresh", "Refresh now", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &refresh_now, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Scout")
        .menu(&menu)
        // Left-click should open the app; the menu is for everything else.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_window(app),
            "refresh" => {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let result = pipeline::run().await;
                    use tauri::Emitter;
                    let _ = handle.emit("scout://refreshed", &result);
                });
            }
            // The only route that actually ends the process. Closing the
            // window merely hides it, so alarms keep working.
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first. Two Scouts would each open the same SQLite
        // file, and a hard stop then leaves an unreplayable write-ahead log
        // that reads as "database disk image is malformed". Instead, a second
        // launch hands focus to the copy already running and exits.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_window(app);
        }))
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(DB_URL, migrations())
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(SchedulerState::default())
        .manage(AudioState::default())
        .invoke_handler(tauri::generate_handler![
            fetch_news,
            refresh,
            get_settings,
            save_settings,
            scheduler::sync_alarms,
            scheduler::sync_deadlines,
            scheduler::take_pending_items,
            audio::stop_alarm_sound,
            audio::alarm_is_ringing,
            audio::play_chime,
            open_mini,
            close_mini,
            dismiss_alarm,
            test_alarm,
            show_main,
            pipeline::ask::ask_advisor,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            build_tray(app)?;
            scheduler::spawn(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing hides to the tray instead of exiting. This is the
            // behaviour that makes alarms dependable: the scheduler keeps
            // running whether or not the window is on screen. Quit from the
            // tray menu is the deliberate way out.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                // Dismissing the overlay is also how an alarm gets silenced,
                // so a stray beeping window can always be shut up by closing it.
                if window.label() == "mini" {
                    window.app_handle().state::<AudioState>().stop();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
