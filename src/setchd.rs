use crate::cache::{CACHE_PATH, SetchCache};
use std::{fs, thread, time::Duration};

pub fn collect_once() -> SetchCache {
    let readout = ecliptic::GeneralReadout::collect_parallel();
    let pkgs = &readout.packages;

    SetchCache {
        // CPU
        cpu: readout.cpu.model_name.unwrap_or_else(|_| "Unknown".into()),
        cpu_vendor: readout.cpu.vendor.unwrap_or_else(|_| "Unknown".into()),
        cpu_cores: readout.cpu.cores,
        cpu_threads: readout.cpu.threads,
        cpu_max_freq: readout.cpu.max_freq.unwrap_or(0.0),
        cpu_arch: readout.cpu.arch.to_string(),

        // Memory
        memory_used: readout.memory.used.map(|k| k.to_gb()).unwrap_or(0.0),
        memory_total: readout.memory.total.map(|k| k.to_gb()).unwrap_or(0.0),
        memory_percent: readout.memory.usage_percent.unwrap_or(0.0),
        swap_used: readout.memory.swap_used.map(|k| k.to_gb()).unwrap_or(0.0),
        swap_total: readout.memory.swap_total.map(|k| k.to_gb()).unwrap_or(0.0),

        // Disk
        disk_used: readout.disk.used.map(|k| k.to_gb()).unwrap_or(0.0),
        disk_total: readout.disk.total.map(|k| k.to_gb()).unwrap_or(0.0),
        disk_percent: readout.disk.usage_percent.unwrap_or(0.0),
        disk_filesystem: readout.disk.filesystem.unwrap_or_default(),

        // OS / Kernel
        os: readout
            .os
            .name
            .unwrap_or_else(|| readout.uname.sysname.clone()),
        kernel: readout.uname.release.clone(),
        sysname: readout.uname.sysname.clone(),
        hostname: readout.uname.nodename.clone(),
        arch: readout.uname.machine.clone(),

        // Host / Board
        host: readout
            .board
            .product_name
            .unwrap_or_else(|_| "Unknown".into()),
        board_name: readout
            .board
            .board_name
            .unwrap_or_else(|_| "Unknown".into()),
        board_vendor: readout
            .board
            .board_vendor
            .unwrap_or_else(|_| "Unknown".into()),
        bios_version: readout.board.version.unwrap_or_else(|_| "Unknown".into()),

        // Uptime
        uptime_secs: readout.uptime.uptime.unwrap_or(Duration::ZERO).as_secs(),

        // Packages
        packages: pkgs.count.unwrap_or(0),
        pkg_manager: pkgs.manager.clone().unwrap_or_default(),

        // Shell
        shell: readout.shell.name.clone().unwrap_or_default(),
        shell_version: readout.shell.version.clone().unwrap_or_default(),

        // Terminal
        terminal: readout.terminal.name.clone().unwrap_or_default(),
        terminal_version: readout.terminal.version.clone().unwrap_or_default(),

        // WM
        wm: readout.desktop.name.clone().unwrap_or_default(),
        wm_version: readout.desktop.version.clone().unwrap_or_default(),

        // Display
        resolution: readout.panel.resolution.clone().unwrap_or_default(),
        display_name: readout.panel.name.clone().unwrap_or_default(),
        refresh_hz: readout.panel.refresh_hz.unwrap_or(0.0),
        display_size: readout.panel.size_inches.unwrap_or(0.0),
        display_builtin: readout.panel.builtin,

        // GPU
        gpus: readout.pci.iter().filter_map(|d| d.as_string()).collect(),

        // Network
        ip: readout
            .interface
            .ip
            .map(|ip| ip.to_string())
            .unwrap_or_default(),
        iface: readout.interface.name.unwrap_or_default(),

        // Theme
        theme: readout.theme.theme.unwrap_or_default(),
        icon: readout.theme.icons.unwrap_or_default(),

        // Battery
        battery_capacity: readout.battery.capacity.unwrap_or(0),
        battery_status: readout.battery.status.unwrap_or_default(),

        // Locale
        locale: readout.locale.locale.unwrap_or_default(),

        // OS age
        os_age_days: readout.os.age.map(|d| d.as_secs() / 86400).unwrap_or(0),
    }
}

pub fn run() {
    println!("Running as daemon");
    println!("Listening on {}", CACHE_PATH);
    let _ = fs::remove_file(CACHE_PATH);

    loop {
        let cache = collect_once();
        let toml_str = toml::to_string(&cache).expect("serialize failed");
        let temp = format!("{}.tmp", CACHE_PATH);
        if fs::write(&temp, &toml_str).is_ok() {
            let _ = fs::rename(&temp, CACHE_PATH);
        }
        thread::sleep(Duration::from_secs(1));
    }
}
pub fn is_alive() -> bool {
    if let Ok(metadata) = fs::metadata(CACHE_PATH) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = modified.elapsed() {
                return elapsed < Duration::from_secs(2);
            }
        }
    }
    false
}
