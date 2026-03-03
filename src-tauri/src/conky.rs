use crate::config::AppConfig;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

/// PID of the currently running conky process (a reaper thread prevents zombies)
static CONKY_PID: Mutex<Option<u32>> = Mutex::new(None);

/// Path to the generated conky config
fn conky_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("conky")
        .join("an8nymous-stats.conf")
}

/// Detect the default network interface via `ip route show default`
fn detect_network_interface() -> String {
    if let Ok(output) = Command::new("ip").args(["route", "show", "default"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(idx) = line.find("dev ") {
                let rest = &line[idx + 4..];
                if let Some(iface) = rest.split_whitespace().next() {
                    return iface.to_string();
                }
            }
        }
    }
    "eth0".to_string()
}

/// Convert hex color #RRGGBB to RRGGBB (strip the #)
fn hex_to_conky(color: &str) -> &str {
    color.strip_prefix('#').unwrap_or(color)
}

/// Build the conky text line from enabled metrics + order
fn build_conky_text(cfg: &AppConfig, iface: &str) -> String {
    let mut parts: Vec<String> = Vec::new();

    for key in &cfg.metrics_order {
        let enabled = match key.as_str() {
            "cpu" => cfg.metrics.cpu,
            "cpu_freq" => cfg.metrics.cpu_freq,
            "ram" => cfg.metrics.ram,
            "ram_gb" => cfg.metrics.ram_gb,
            "gpu" => cfg.metrics.gpu,
            "gpu_temp" => cfg.metrics.gpu_temp,
            "gpu_power" => cfg.metrics.gpu_power,
            "gpu_clock" => cfg.metrics.gpu_clock,
            "vram" => cfg.metrics.vram,
            "disk_io" => cfg.metrics.disk_io,
            "net_io" => cfg.metrics.net_io,
            _ => false,
        };
        if !enabled {
            continue;
        }

        let snippet = match key.as_str() {
            "cpu" => "CPU ${cpu cpu0}%".to_string(),
            "cpu_freq" => "${freq_g}GHz".to_string(),
            "ram" => "RAM ${memperc}%".to_string(),
            "ram_gb" => "${mem}/${memmax}".to_string(),
            "gpu" => {
                "GPU ${exec nvidia-smi --query-gpu=utilization.gpu --format=csv,noheader,nounits}%"
                    .to_string()
            }
            "gpu_temp" => {
                "${exec nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader,nounits}°C"
                    .to_string()
            }
            "gpu_power" => {
                "${exec nvidia-smi --query-gpu=power.draw --format=csv,noheader,nounits}W"
                    .to_string()
            }
            "gpu_clock" => {
                "${exec nvidia-smi --query-gpu=clocks.current.graphics --format=csv,noheader,nounits}MHz"
                    .to_string()
            }
            "vram" => {
                "VRAM ${exec nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits}MB"
                    .to_string()
            }
            "disk_io" => "Disk ${diskio_read}/s".to_string(),
            "net_io" => format!(
                "↓${{downspeed {iface}}} ↑${{upspeed {iface}}}"
            ),
            _ => continue,
        };
        parts.push(snippet);
    }

    parts.join("  ")
}

/// Generate the full conky.conf content from AppConfig
pub fn generate_config(cfg: &AppConfig) -> String {
    let iface = detect_network_interface();
    let text = build_conky_text(cfg, &iface);
    let color = hex_to_conky(&cfg.appearance.text_color);
    let font_family = &cfg.appearance.font_family;
    let font_size = cfg.appearance.font_size;
    // Conky gap values must be >= 0 (negative causes X BadMatch crash)
    let gap_x = cfg.appearance.position_x.unwrap_or(15).max(0);
    let gap_y = cfg.appearance.position_y.max(0);
    let argb_value = (cfg.appearance.opacity * 255.0) as u32;

    format!(
        r#"conky.config = {{
    alignment = 'top_left',
    gap_x = {gap_x},
    gap_y = {gap_y},

    font = '{font_family}:size={font_size}',
    default_color = '{color}',

    own_window = true,
    own_window_type = 'dock',
    own_window_transparent = true,
    own_window_argb_visual = true,
    own_window_argb_value = {argb_value},
    own_window_hints = 'undecorated,above,sticky,skip_taskbar,skip_pager',

    background = false,
    double_buffer = true,
    no_buffers = true,

    use_xft = true,
    xftalpha = 1.0,
    override_utf8_locale = true,

    update_interval = 1,
    total_run_times = 0,

    draw_shades = false,
    draw_outline = false,
    draw_borders = false,
    draw_graph_borders = false,

    short_units = true,
    pad_percents = 0,

    minimum_width = 0,
    minimum_height = 0,
}};

conky.text = [[
{text}
]];
"#
    )
}

/// Write config file to disk
fn write_config(cfg: &AppConfig) {
    let path = conky_config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let conf = generate_config(cfg);
    let _ = fs::write(&path, conf);
}

/// Kill ALL conky processes (simple and reliable)
fn kill_all_conky() {
    let _ = Command::new("killall").args(["-q", "conky"]).output();
    // Wait for processes to die
    std::thread::sleep(std::time::Duration::from_millis(200));
}

/// Spawn conky and start a reaper thread to prevent zombies
fn spawn_conky() {
    let path = conky_config_path();
    match Command::new("conky")
        .args(["-c", &path.to_string_lossy()])
        .spawn()
    {
        Ok(mut child) => {
            let pid = child.id();
            *CONKY_PID.lock().unwrap() = Some(pid);

            // Reaper thread: waits for child to exit, then clears PID
            std::thread::spawn(move || {
                let _ = child.wait();
                let mut guard = CONKY_PID.lock().unwrap();
                if *guard == Some(pid) {
                    *guard = None;
                }
            });
        }
        Err(e) => {
            eprintln!("conky: failed to spawn: {}", e);
        }
    }
}

/// Write conky.conf and restart the conky process
pub fn apply(cfg: &AppConfig) {
    kill_all_conky();
    *CONKY_PID.lock().unwrap() = None;
    write_config(cfg);
    spawn_conky();
}

/// Start conky with the given config
pub fn start(cfg: &AppConfig) {
    kill_all_conky(); // kill any pre-existing conky
    *CONKY_PID.lock().unwrap() = None;
    write_config(cfg);
    spawn_conky();
}

/// Stop conky
pub fn stop() {
    kill_all_conky();
    *CONKY_PID.lock().unwrap() = None;
}

/// Check if conky is running
pub fn is_running() -> bool {
    CONKY_PID.lock().unwrap().is_some()
}
