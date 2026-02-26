mod commands;
mod config;
mod gpu;
mod stats;

use config::ConfigState;
use stats::{StatsCollector, StatsState};
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = config::load_config();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(ConfigState(Mutex::new(cfg)))
        .manage(StatsState(Mutex::new(StatsCollector::new())))
        .setup(|app| {
            // Set up tray menu
            let settings_item = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
            let toggle_item =
                MenuItemBuilder::with_id("toggle", "Show/Hide").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&settings_item)
                .item(&toggle_item)
                .separator()
                .item(&quit_item)
                .build()?;

            if let Some(tray) = app.tray_by_id("main-tray") {
                tray.set_menu(Some(menu))?;
                tray.on_menu_event(move |app, event| match event.id().as_ref() {
                    "settings" => {
                        let _ = commands::open_settings(app.clone());
                    }
                    "toggle" => {
                        let _ = commands::toggle_overlay(app.clone());
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                });
            }

            // Position overlay according to config
            if let Some(overlay) = app.get_webview_window("overlay") {
                let state = app.state::<ConfigState>();
                let cfg = state.0.lock().unwrap();
                let x = cfg.appearance.position_x.unwrap_or_else(|| {
                    let screen_w = overlay
                        .current_monitor()
                        .ok()
                        .flatten()
                        .map(|m| m.size().width as i32)
                        .unwrap_or(1920);
                    screen_w - 700 - 15
                });
                let y = cfg.appearance.position_y;
                let _ = overlay.set_position(tauri::PhysicalPosition::new(x, y));

                // Make window click-through on Windows
                #[cfg(target_os = "windows")]
                make_click_through(&overlay);
            }

            // Re-assert always-on-top every 500ms to stay above game windows
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    if let Some(overlay) = app_handle.get_webview_window("overlay") {
                        if overlay.is_visible().unwrap_or(false) {
                            let _ = overlay.set_always_on_top(true);
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::get_stats,
            commands::save_metric,
            commands::save_metrics_order,
            commands::save_appearance,
            commands::toggle_startup,
            commands::get_startup_status,
            commands::reset_settings,
            commands::get_screen_size,
            commands::open_settings,
            commands::toggle_overlay,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(target_os = "windows")]
fn make_click_through(window: &tauri::WebviewWindow) {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    if let Ok(hwnd) = window.hwnd() {
        let hwnd_raw = hwnd.0 as *mut std::ffi::c_void;
        unsafe {
            let ex_style = GetWindowLongW(hwnd_raw, GWL_EXSTYLE);
            SetWindowLongW(
                hwnd_raw,
                GWL_EXSTYLE,
                ex_style | WS_EX_TRANSPARENT as i32 | WS_EX_LAYERED as i32,
            );
        }
    }
}
