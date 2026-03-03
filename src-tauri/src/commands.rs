use crate::config::{self, AppConfig, ConfigState};
#[cfg(target_os = "linux")]
use crate::conky;
use crate::stats::{StatsState, SystemStats};
use tauri::{AppHandle, Emitter, Manager, State};

/// Notify the overlay that config changed (lightweight event, no heavy payload)
fn notify_overlay(app: &AppHandle) {
    let _ = app.emit_to("overlay", "config-updated", ());
}

#[tauri::command]
pub fn get_config(state: State<'_, ConfigState>) -> AppConfig {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
pub fn get_stats(state: State<'_, StatsState>) -> SystemStats {
    state.0.lock().unwrap().collect()
}

#[tauri::command]
pub fn save_metric(app: AppHandle, state: State<'_, ConfigState>, key: String, enabled: bool) -> Result<bool, String> {
    let mut cfg = state.0.lock().unwrap();
    match key.as_str() {
        "cpu" => cfg.metrics.cpu = enabled,
        "cpu_freq" => cfg.metrics.cpu_freq = enabled,
        "ram" => cfg.metrics.ram = enabled,
        "ram_gb" => cfg.metrics.ram_gb = enabled,
        "gpu" => cfg.metrics.gpu = enabled,
        "gpu_temp" => cfg.metrics.gpu_temp = enabled,
        "gpu_power" => cfg.metrics.gpu_power = enabled,
        "gpu_clock" => cfg.metrics.gpu_clock = enabled,
        "vram" => cfg.metrics.vram = enabled,
        "disk_io" => cfg.metrics.disk_io = enabled,
        "net_io" => cfg.metrics.net_io = enabled,
        _ => return Err(format!("Unknown metric: {}", key)),
    }
    config::save_config(&cfg)?;
    #[cfg(target_os = "linux")]
    conky::apply(&cfg);
    notify_overlay(&app);
    Ok(true)
}

#[tauri::command]
pub fn save_metrics_order(app: AppHandle, state: State<'_, ConfigState>, order: Vec<String>) -> Result<bool, String> {
    let mut cfg = state.0.lock().unwrap();
    cfg.metrics_order = order;
    config::save_config(&cfg)?;
    #[cfg(target_os = "linux")]
    conky::apply(&cfg);
    notify_overlay(&app);
    Ok(true)
}

#[tauri::command]
pub fn save_appearance(
    app: AppHandle,
    state: State<'_, ConfigState>,
    key: String,
    value: serde_json::Value,
) -> Result<bool, String> {
    let mut cfg = state.0.lock().unwrap();
    match key.as_str() {
        "position_x" => {
            cfg.appearance.position_x = value.as_i64().map(|v| v as i32);
        }
        "position_y" => {
            cfg.appearance.position_y = value.as_i64().unwrap_or(15) as i32;
        }
        "text_color" => {
            cfg.appearance.text_color = value.as_str().unwrap_or("#CCCCCC").to_string();
        }
        "opacity" => {
            cfg.appearance.opacity = value.as_f64().unwrap_or(1.0);
        }
        "transparent_bg" => {
            cfg.appearance.transparent_bg = value.as_bool().unwrap_or(true);
        }
        "font_family" => {
            cfg.appearance.font_family = value.as_str().unwrap_or("Arial").to_string();
        }
        "font_size" => {
            cfg.appearance.font_size = value.as_u64().unwrap_or(9) as u32;
        }
        _ => return Err(format!("Unknown appearance key: {}", key)),
    }
    config::save_config(&cfg)?;

    // Linux: regenerate conky config and restart
    #[cfg(target_os = "linux")]
    conky::apply(&cfg);

    // Windows: move overlay window directly (instantaneous)
    #[cfg(target_os = "windows")]
    if let Some(overlay) = app.get_webview_window("overlay") {
        if key == "position_x" || key == "position_y" {
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
        }
    }

    notify_overlay(&app);

    Ok(true)
}

#[tauri::command]
pub fn toggle_startup(state: State<'_, ConfigState>, enabled: bool) -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let run_key = hkcu
            .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_SET_VALUE)
            .map_err(|e| e.to_string())?;

        if enabled {
            let exe_path = std::env::current_exe()
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .to_string();
            run_key
                .set_value("an8nymous Stats", &format!("\"{}\"", exe_path))
                .map_err(|e| e.to_string())?;
        } else {
            let _ = run_key.delete_value("an8nymous Stats");
        }

        let mut cfg = state.0.lock().unwrap();
        cfg.preferences.start_with_windows = enabled;
        config::save_config(&cfg)?;
        Ok(true)
    }

    #[cfg(target_os = "linux")]
    {
        let autostart_dir = dirs::config_dir()
            .unwrap_or_default()
            .join("autostart");
        let desktop_file = autostart_dir.join("an8nymous-stats.desktop");

        if enabled {
            let _ = std::fs::create_dir_all(&autostart_dir);
            let exe_path = std::env::current_exe()
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .to_string();
            let content = format!(
                "[Desktop Entry]\nType=Application\nName=an8nymous Stats\nExec={}\nX-GNOME-Autostart-enabled=true\n",
                exe_path
            );
            std::fs::write(&desktop_file, content).map_err(|e| e.to_string())?;
        } else {
            let _ = std::fs::remove_file(&desktop_file);
        }

        let mut cfg = state.0.lock().unwrap();
        cfg.preferences.start_with_windows = enabled;
        config::save_config(&cfg)?;
        Ok(true)
    }
}

#[tauri::command]
pub fn get_startup_status() -> bool {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(run_key) = hkcu.open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            KEY_READ,
        ) {
            run_key.get_value::<String, _>("an8nymous Stats").is_ok()
        } else {
            false
        }
    }

    #[cfg(target_os = "linux")]
    {
        let desktop_file = dirs::config_dir()
            .unwrap_or_default()
            .join("autostart/an8nymous-stats.desktop");
        desktop_file.exists()
    }
}

#[tauri::command]
pub fn reset_settings(app: AppHandle, state: State<'_, ConfigState>) -> Result<AppConfig, String> {
    let mut cfg = state.0.lock().unwrap();
    *cfg = AppConfig::default();
    config::save_config(&cfg)?;

    // Linux: regenerate conky config and restart
    #[cfg(target_os = "linux")]
    conky::apply(&cfg);

    // Windows: reset overlay position
    #[cfg(target_os = "windows")]
    if let Some(overlay) = app.get_webview_window("overlay") {
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
    }

    notify_overlay(&app);
    Ok(cfg.clone())
}

#[tauri::command]
pub fn get_screen_size() -> (u32, u32) {
    #[cfg(target_os = "windows")]
    {
        let width = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(0) } as u32;
        let height = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(1) } as u32;
        (width, height)
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        // Parse xrandr output for primary monitor resolution
        if let Ok(output) = Command::new("xrandr").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains(" connected primary") {
                    // e.g. "HDMI-0 connected primary 3440x1440+0+0"
                    if let Some(res) = line.split_whitespace().find(|s| s.contains('x') && s.contains('+')) {
                        let res_part = res.split('+').next().unwrap_or("1920x1080");
                        let parts: Vec<&str> = res_part.split('x').collect();
                        if parts.len() == 2 {
                            let w = parts[0].parse().unwrap_or(1920);
                            let h = parts[1].parse().unwrap_or(1080);
                            return (w, h);
                        }
                    }
                }
            }
        }
        (1920, 1080)
    }
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let _settings_window = tauri::WebviewWindowBuilder::new(
        &app,
        "settings",
        tauri::WebviewUrl::App("settings.html".into()),
    )
    .title("an8nymous Stats - Settings")
    .inner_size(620.0, 720.0)
    .resizable(false)
    .center()
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn toggle_overlay(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        if conky::is_running() {
            conky::stop();
        } else {
            let state = app.state::<ConfigState>();
            let cfg = state.0.lock().unwrap();
            conky::start(&cfg);
        }
    }

    #[cfg(target_os = "windows")]
    if let Some(window) = app.get_webview_window("overlay") {
        if window.is_visible().unwrap_or(false) {
            window.hide().map_err(|e| e.to_string())?;
        } else {
            window.show().map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
