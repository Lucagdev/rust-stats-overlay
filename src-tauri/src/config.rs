use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_true")]
    pub cpu: bool,
    #[serde(default)]
    pub cpu_freq: bool,
    #[serde(default = "default_true")]
    pub ram: bool,
    #[serde(default = "default_true")]
    pub ram_gb: bool,
    #[serde(default = "default_true")]
    pub gpu: bool,
    #[serde(default)]
    pub gpu_temp: bool,
    #[serde(default)]
    pub gpu_power: bool,
    #[serde(default)]
    pub gpu_clock: bool,
    #[serde(default)]
    pub vram: bool,
    #[serde(default = "default_true")]
    pub disk_io: bool,
    #[serde(default)]
    pub net_io: bool,
}

fn default_true() -> bool {
    true
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            cpu: true,
            cpu_freq: false,
            ram: true,
            ram_gb: true,
            gpu: true,
            gpu_temp: false,
            gpu_power: false,
            gpu_clock: false,
            vram: false,
            disk_io: true,
            net_io: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    #[serde(default)]
    pub position_x: Option<i32>,
    #[serde(default = "default_position_y")]
    pub position_y: i32,
    #[serde(default = "default_text_color")]
    pub text_color: String,
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    #[serde(default = "default_true")]
    pub transparent_bg: bool,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: u32,
}

fn default_position_y() -> i32 {
    15
}
fn default_text_color() -> String {
    "#CCCCCC".to_string()
}
fn default_opacity() -> f64 {
    1.0
}
fn default_font_family() -> String {
    "Arial".to_string()
}
fn default_font_size() -> u32 {
    9
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            position_x: None,
            position_y: 15,
            text_color: "#CCCCCC".to_string(),
            opacity: 1.0,
            transparent_bg: true,
            font_family: "Arial".to_string(),
            font_size: 9,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferencesConfig {
    #[serde(default)]
    pub start_with_windows: bool,
}

impl Default for PreferencesConfig {
    fn default() -> Self {
        Self {
            start_with_windows: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub metrics: MetricsConfig,
    #[serde(default = "default_metrics_order")]
    pub metrics_order: Vec<String>,
    #[serde(default)]
    pub appearance: AppearanceConfig,
    #[serde(default)]
    pub preferences: PreferencesConfig,
}

fn default_metrics_order() -> Vec<String> {
    vec![
        "cpu".into(),
        "cpu_freq".into(),
        "ram".into(),
        "ram_gb".into(),
        "gpu".into(),
        "gpu_temp".into(),
        "gpu_power".into(),
        "gpu_clock".into(),
        "vram".into(),
        "disk_io".into(),
        "net_io".into(),
    ]
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            metrics: MetricsConfig::default(),
            metrics_order: default_metrics_order(),
            appearance: AppearanceConfig::default(),
            preferences: PreferencesConfig::default(),
        }
    }
}

pub struct ConfigState(pub Mutex<AppConfig>);

pub fn config_path() -> PathBuf {
    // Use the project root directory so config.json stays alongside the project
    // instead of inside target/release/. Path is resolved at compile-time via
    // CARGO_MANIFEST_DIR (src-tauri/), going up one level to reach the root.
    let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // project root (one level up from src-tauri/)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| std::env::current_dir().unwrap())
        });
    project_dir.join("config.json")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}
