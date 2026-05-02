use serde::{Deserialize, Serialize};
use std::fs;

pub const CACHE_PATH: &str = "/tmp/setchd.toml";
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SetchCache {
    // CPU
    pub cpu: String,
    pub cpu_vendor: String,
    pub cpu_cores: usize,
    pub cpu_threads: usize,
    pub cpu_max_freq: f64,
    pub cpu_arch: String,

    // Memory
    pub memory_used: f64,
    pub memory_total: f64,
    pub memory_percent: f64,
    pub swap_used: f64,
    pub swap_total: f64,

    // Disk
    pub disk_used: f64,
    pub disk_total: f64,
    pub disk_percent: f64,
    pub disk_filesystem: String,

    // OS / Kernel
    pub os: String,
    pub kernel: String,
    pub sysname: String,
    pub hostname: String,
    pub arch: String,

    // Host / Board
    pub host: String, // product_name
    pub board_name: String,
    pub board_vendor: String,
    pub bios_version: String,

    // Uptime
    pub uptime_secs: u64,

    // Packages
    pub packages: usize,
    pub pkg_manager: String,

    // Shell
    pub shell: String,
    pub shell_version: String,

    // Terminal
    pub terminal: String,
    pub terminal_version: String,

    // Desktop / WM
    pub wm: String,
    pub wm_version: String,

    // Display
    pub resolution: String,
    pub display_name: String,
    pub refresh_hz: f64,
    pub display_size: f32,
    pub display_builtin: bool,

    // GPU
    pub gpus: Vec<String>,

    // Network
    pub ip: String,
    pub iface: String,

    // Theme
    pub theme: String,
    pub icon: String,

    // Battery
    pub battery_capacity: u16,
    pub battery_status: String,

    // Locale
    pub locale: String,

    // OS install age
    pub os_age_days: u64,
}
pub fn read_from_cache() -> SetchCache {
    let raw = fs::read_to_string(CACHE_PATH).unwrap_or_default();
    toml::from_str(&raw).unwrap_or_default()
}
