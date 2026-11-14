use comfy_pci::from_pciid as parse_pciid;
use glob::glob;
use nix::errno::Errno;
use nix::sys::statvfs::statvfs;
use nix::sys::sysinfo::sysinfo;
use nix::sys::utsname::uname;
use phf::phf_map;
use rayon::join;
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::net::{IpAddr, UdpSocket};
use std::path::Path;
use std::process;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime};

static PKG_MANAGER: LazyLock<Option<String>> = LazyLock::new(detect_pkg_manager);

#[derive(Debug)]
pub enum ReadoutError {
    NotFound(String),
    PermissionDenied(String),
    ParseError(String),
    Unsupported,
    NoDefaultRoute,
    Io(std::io::ErrorKind),
}

impl std::fmt::Display for ReadoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::NotFound(s) => write!(f, "not found: {}", s),
            Self::PermissionDenied(s) => write!(f, "permission denied: {}", s),
            Self::ParseError(s) => write!(f, "parse error: {}", s),
            Self::Unsupported => write!(f, "unsupported"),
            Self::NoDefaultRoute => write!(f, "no default route"),
            Self::Io(e) => write!(f, "io error: {}", e),
        }
    }
}

impl std::error::Error for ReadoutError {}

// convenience conversion from io::Error
impl From<std::io::Error> for ReadoutError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.kind())
    }
}

#[derive(Debug)]
pub struct ProcessorReadout {
    pub model_name: Result<String, ReadoutError>,
    pub vendor: Result<String, ReadoutError>,
    pub cores: usize,
    pub threads: usize,
    pub max_freq: Option<f64>,
    pub arch: &'static str,
    pub fma: bool,
    pub avx: bool,
    pub avx2: bool,
}

impl ProcessorReadout {
    pub fn new() -> Self {
        let text = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
        let info = parse_cpuinfo(&text);
        let get = |keys: &[&str]| keys.iter().find_map(|k| info.get(k).copied());

        // x86: "model name", arm64: often missing, older arm/android: "Hardware",
        // riscv: "uarch", powerpc: "cpu"
        let model_name = get(&["model name", "Hardware", "uarch", "cpu"])
            .map(str::to_string)
            .ok_or(ReadoutError::NotFound("CPU brand".into()));

        // x86: "vendor_id", arm: "CPU implementer" (hex code), riscv: "mvendorid"
        let vendor = get(&["vendor_id"])
            .map(str::to_string)
            .or_else(|| get(&["CPU implementer"]).map(arm_implementer_name))
            .or_else(|| get(&["mvendorid"]).map(str::to_string))
            .ok_or(ReadoutError::NotFound("Unknown Vendor".into()));

        let cores = num_cpus::get_physical();
        let threads = num_cpus::get();
        let max_freq = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
            .ok()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .map(|khz| khz / 1_000_000.0);

        // x86: "flags", arm: "Features"
        let flags = get(&["flags", "Features"]).unwrap_or("");
        let has = |name: &str| flags.split_whitespace().any(|f| f == name);
        let fma = has("fma");
        let avx = has("avx");
        let avx2 = has("avx2");
        let arch = std::env::consts::ARCH;
        Self {
            model_name,
            vendor,
            cores,
            threads,
            max_freq,
            arch,
            fma,
            avx,
            avx2,
        }
    }
}

/// Parses `/proc/cpuinfo` into a key -> value map.
///
/// The file repeats a block per logical CPU, and some keys (like `Hardware`
/// on ARM) only appear after the last block, so the whole file is scanned and
/// the first value seen for each key is kept.
fn parse_cpuinfo(text: &str) -> HashMap<&str, &str> {
    let mut map = HashMap::new();
    for (key, value) in text.lines().filter_map(|line| line.split_once(':')) {
        let (key, value) = (key.trim(), value.trim());
        if !value.is_empty() {
            map.entry(key).or_insert(value);
        }
    }
    map
}

/// Maps the ARM `CPU implementer` hex code to a vendor name.
fn arm_implementer_name(code: &str) -> String {
    let name = match code.to_ascii_lowercase().as_str() {
        "0x41" => "ARM",
        "0x42" => "Broadcom",
        "0x43" => "Cavium",
        "0x46" => "Fujitsu",
        "0x48" => "HiSilicon",
        "0x4e" => "NVIDIA",
        "0x50" => "APM",
        "0x51" => "Qualcomm",
        "0x53" => "Samsung",
        "0x56" => "Marvell",
        "0x61" => "Apple",
        "0x69" => "Intel",
        "0xc0" => "Ampere",
        _ => return code.to_string(),
    };
    name.to_string()
}

#[derive(Debug)]
pub struct UptimeReadout {
    pub uptime: Result<Duration, Errno>,
}

impl UptimeReadout {
    pub fn new() -> Self {
        // use nix to get uptime cuz its fast :3
        let uptime = sysinfo().map(|info| Duration::from_secs(info.uptime().as_secs()));
        Self { uptime: uptime }
    }
}

#[derive(Debug)]
pub struct OsReadout {
    pub name: Option<String>,
    pub installed_at: Option<SystemTime>,
    pub age: Option<Duration>,
}
impl OsReadout {
    pub fn new() -> Self {
        let meta = fs::metadata("/etc/machine-id").ok();

        let installed_at = meta
            .as_ref()
            .and_then(|m| m.created().ok().or_else(|| m.modified().ok()));

        let age = installed_at.and_then(|t| std::time::SystemTime::now().duration_since(t).ok());

        let name = fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                content.lines().find_map(|line| {
                    line.strip_prefix("PRETTY_NAME=")
                        .map(|v| v.trim_matches('"').to_string())
                })
            });
        Self {
            name,
            installed_at,
            age,
        }
    }
}

#[derive(Debug)]
pub struct DiskReadout {
    pub filesystem: Option<String>,
    pub total: Option<Kilobytes>,
    pub used: Option<Kilobytes>,
    pub free: Option<Kilobytes>,
    pub usage_percent: Option<f64>,
}
impl DiskReadout {
    pub fn new() -> Self {
        Self::with_path(Some("/"))
    }
    pub fn with_path(path: Option<&str>) -> Self {
        let stat = statvfs(path.unwrap_or("/")).ok();

        let total = stat.map(|s| Kilobytes((s.blocks() * s.block_size() as u64) / 1024));

        let free = stat.map(|s| Kilobytes((s.blocks_available() * s.block_size() as u64) / 1024));

        let used = match (total, free) {
            (Some(t), Some(f)) => Some(Kilobytes(t.0 - f.0)),
            _ => None,
        };

        let usage_percent = match (total, used) {
            (Some(t), Some(u)) if t.0 != 0 => Some((u.0 as f64 / t.0 as f64) * 100.0),
            _ => None,
        };

        let filesystem = fs::read_to_string("/proc/mounts").ok().and_then(|mounts| {
            mounts
                .lines()
                .find(|line| line.split_whitespace().nth(1) == Some(path.unwrap_or("/")))
                .and_then(|line| line.split_whitespace().nth(2))
                .map(|s| s.to_string())
        });

        Self {
            total,
            used,
            free,
            usage_percent,
            filesystem,
        }
    }
}

#[derive(Debug)]
pub struct MemoryReadout {
    pub total: Option<Kilobytes>,
    pub available: Option<Kilobytes>,
    pub used: Option<Kilobytes>,
    pub swap_total: Option<Kilobytes>,
    pub swap_used: Option<Kilobytes>,
    pub usage_percent: Option<f64>,
}
impl MemoryReadout {
    pub fn new() -> Self {
        // Parse /proc/meminfo, linux only
        let meminfo = File::open("/proc/meminfo");
        let reader = meminfo.ok().map(BufReader::new);
        let mut mem_total: Option<Kilobytes> = None;
        let mut mem_available: Option<Kilobytes> = None;
        let mut swap_total: Option<Kilobytes> = None;
        let mut swap_free: Option<Kilobytes> = None;
        if let Some(reader) = reader {
            for line in reader.lines().flatten() {
                if let Some((key, val)) = line.split_once(':') {
                    let parsed = val
                        .trim()
                        .strip_suffix("kB")
                        .and_then(|v| v.trim().parse::<u64>().ok())
                        .map(Kilobytes);
                    match key {
                        "MemTotal" => mem_total = parsed,
                        "MemAvailable" => mem_available = parsed,
                        "SwapTotal" => swap_total = parsed,
                        "SwapFree" => swap_free = parsed,
                        _ => {}
                    }
                }
            }
        }
        let swap_used = match (&swap_total, &swap_free) {
            (Some(t), Some(f)) => Some(Kilobytes(t.0 - f.0)),
            _ => None,
        };
        let mem_used = match (&mem_total, &mem_available) {
            (Some(t), Some(f)) => Some(Kilobytes(t.0 - f.0)),
            _ => None,
        };
        let usage_percent = match (mem_total, mem_available) {
            (Some(t), Some(f)) => Some((t.0 - f.0) as f64 / t.0 as f64 * 100.0),
            _ => None,
        };

        Self {
            total: mem_total,
            available: mem_available,
            used: mem_used,
            swap_total,
            swap_used,
            usage_percent,
        }
    }
}
#[derive(Debug)]
pub struct BoardReadout {
    pub vendor: Result<String, ReadoutError>,
    pub version: Result<String, ReadoutError>,
    pub date: Result<String, ReadoutError>,
    pub board_name: Result<String, ReadoutError>,
    pub board_vendor: Result<String, ReadoutError>,
    pub board_version: Result<String, ReadoutError>,
    pub product_name: Result<String, ReadoutError>,
}

impl BoardReadout {
    pub fn new() -> Self {
        let dmi = |file: &str| {
            fs::read_to_string(format!("/sys/class/dmi/id/{}", file))
                .map(|s| s.trim().to_string())
                .map_err(|_| ReadoutError::NotFound(format!("dmi/{}", file)))
        };
        Self {
            vendor: dmi("bios_vendor"),
            version: dmi("bios_version"),
            date: dmi("bios_date"),
            board_name: dmi("board_name"),
            board_vendor: dmi("board_vendor"),
            board_version: dmi("board_version"),
            product_name: dmi("product_name"),
        }
    }
}

#[derive(Debug)]
pub struct UnameReadout {
    pub sysname: String,
    pub nodename: String,
    pub release: String,
    pub version: String,
    pub machine: String,
}
impl UnameReadout {
    pub fn new() -> Self {
        let u = uname().expect("uname failed (this should not happen)");

        UnameReadout {
            sysname: u.sysname().to_string_lossy().into_owned(),
            nodename: u.nodename().to_string_lossy().into_owned(),
            release: u.release().to_string_lossy().into_owned(),
            version: u.version().to_string_lossy().into_owned(),
            machine: u.machine().to_string_lossy().into_owned(),
        }
    }
}
#[derive(Debug)]
pub struct SysinfoReadout {
    pub uptime: Option<Duration>,
    pub load_avg: Option<(f64, f64, f64)>,
    pub total_ram: Option<Kilobytes>,
    pub free_ram: Option<Kilobytes>,
    pub used_ram: Option<Kilobytes>,
    pub total_swap: Option<Kilobytes>,
    pub free_swap: Option<Kilobytes>,
    pub used_swap: Option<Kilobytes>,
    pub procs: Option<u16>,
}

impl SysinfoReadout {
    pub fn new() -> Self {
        let info = nix::sys::sysinfo::sysinfo().ok();

        let total_ram = info.map(|i| Kilobytes(i.ram_total() / 1024));
        let free_ram = info.map(|i| Kilobytes(i.ram_unused() / 1024));
        let used_ram = match (total_ram, free_ram) {
            (Some(t), Some(f)) => Some(Kilobytes(t.0 - f.0)),
            _ => None,
        };

        let total_swap = info.map(|i| Kilobytes(i.swap_total() / 1024));
        let free_swap = info.map(|i| Kilobytes(i.swap_free() / 1024));
        let used_swap = match (total_swap, free_swap) {
            (Some(t), Some(f)) => Some(Kilobytes(t.0 - f.0)),
            _ => None,
        };

        Self {
            uptime: info.map(|i| i.uptime()),
            load_avg: info.map(|i| {
                let (a, b, c) = i.load_average();
                (a, b, c)
            }),
            total_ram,
            free_ram,
            used_ram,
            total_swap,
            free_swap,
            used_swap,
            procs: info.map(|i| i.process_count()),
        }
    }
}

impl Default for SysinfoReadout {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct BatteryReadout {
    pub capacity: Option<u16>,
    pub status: Option<String>,
}
impl BatteryReadout {
    pub fn new() -> Self {
        let base = Path::new("/sys/class/power_supply");

        let entries = match fs::read_dir(base) {
            Ok(e) => e,
            Err(_) => {
                return BatteryReadout {
                    capacity: None,
                    status: None,
                };
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path
                .file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |n| n.starts_with("BAT"))
            {
                continue;
            }
            let capacity = fs::read_to_string(path.join("capacity"))
                .ok()
                .and_then(|s| s.trim().parse::<u16>().ok());
            let status = fs::read_to_string(path.join("status"))
                .ok()
                .map(|s| s.trim().to_string());
            if capacity.is_some() || status.is_some() {
                return BatteryReadout { capacity, status };
            }
        }
        Self {
            capacity: None,
            status: None,
        }
    }
}

#[derive(Debug)]
pub struct LocaleReadout {
    pub locale: Option<String>,
}
impl LocaleReadout {
    pub fn new() -> Self {
        fn get_locale() -> Option<String> {
            // idk why im trying this first, dont ask me lol
            if let Ok(val) = env::var("LANG") {
                if !val.is_empty() {
                    return Some(val);
                }
            }

            // try a few common LC_* before falling back
            for key in ["LC_MESSAGES", "LC_TIME", "LC_NUMERIC"] {
                if let Ok(val) = env::var(key) {
                    if !val.is_empty() {
                        return Some(val);
                    }
                }
            }

            // final fallback
            if let Ok(val) = env::var("LC_ALL") {
                if !val.is_empty() {
                    return Some(val);
                }
            }

            None
        }

        Self {
            locale: get_locale(),
        }
    }
}

#[derive(Debug)]
pub struct ThemeReadout {
    pub theme: Option<String>,
    pub icons: Option<String>,
}
impl ThemeReadout {
    pub fn new() -> Self {
        fn parse_settings(path: &str) -> Option<(Option<String>, Option<String>)> {
            let content = fs::read_to_string(path).ok()?;

            let mut theme = None;
            let mut icons = None;

            for line in content.lines() {
                if let Some(v) = line.strip_prefix("gtk-theme-name=") {
                    theme = Some(v.trim().to_string());
                }
                if let Some(v) = line.strip_prefix("gtk-icon-theme-name=") {
                    icons = Some(v.trim().to_string());
                }
            }

            Some((theme, icons))
        }

        // 1. ENV override
        if let Ok(env_theme) = env::var("GTK_THEME") {
            let theme = env_theme.split(':').next().map(|s| s.to_string());
            return Self { theme, icons: None };
        }

        let home = match env::var("HOME") {
            Ok(h) => h,
            Err(_) => {
                return Self {
                    theme: None,
                    icons: None,
                };
            }
        };

        // 2. GTK4 / GTK3
        let paths = [
            format!("{}/.config/gtk-4.0/settings.ini", home),
            format!("{}/.config/gtk-3.0/settings.ini", home),
        ];

        for path in paths {
            if let Some((theme, icons)) = parse_settings(&path) {
                return Self { theme, icons };
            }
        }

        // 3. GTK2 fallback
        let gtk2_path = format!("{}/.gtkrc-2.0", home);

        if let Ok(content) = fs::read_to_string(gtk2_path) {
            let mut theme = None;

            for line in content.lines() {
                if line.contains("gtk-theme-name") {
                    if let Some(v) = line.split('=').nth(1) {
                        theme = Some(v.trim_matches('"').trim().to_string());
                    }
                }
            }

            return Self { theme, icons: None };
        }

        Self {
            theme: None,
            icons: None,
        }
    }
}

#[derive(Debug)]
pub struct PanelReadout {
    pub name: Option<String>,
    pub resolution: Option<String>,
    pub size_inches: Option<f32>,
    pub refresh_hz: Option<f64>,
    pub builtin: bool,
    pub connected: bool,
}
impl PanelReadout {
    pub fn new() -> Self {
        let entries = match std::fs::read_dir("/sys/class/drm") {
            Ok(e) => e,
            Err(_) => {
                return Self {
                    name: None,
                    resolution: None,
                    size_inches: None,
                    refresh_hz: None,
                    builtin: false,
                    connected: false,
                };
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            let status_path = path.join("status");
            let modes_path = path.join("modes");
            let edid_path = path.join("edid");

            if !status_path.exists() || !modes_path.exists() {
                continue;
            }

            if std::fs::read_to_string(&status_path)
                .unwrap_or_default()
                .trim()
                != "connected"
            {
                continue;
            }

            let resolution = std::fs::read_to_string(&modes_path)
                .ok()
                .and_then(|m| m.lines().next().map(|s| s.to_string()));

            let edid = std::fs::read(&edid_path).ok();

            let mut name: Option<String> = None;
            let mut size_inches: Option<f32> = None;
            let mut refresh_hz: Option<f64> = None;

            if let Some(edid) = edid {
                if edid.len() >= 128 {
                    for block in edid[54..126].chunks(18) {
                        if block[0] == 0 && block[1] == 0 && block[2] == 0 && block[3] == 0xFC {
                            let raw = &block[5..18];
                            let s = raw
                                .iter()
                                .take_while(|&&c| c != 0x0A)
                                .map(|&c| c as char)
                                .collect::<String>()
                                .trim()
                                .to_string();
                            if !s.is_empty() {
                                name = Some(s);
                            }
                        }
                    }

                    let w_cm = edid[21] as f32;
                    let h_cm = edid[22] as f32;
                    if w_cm > 0.0 && h_cm > 0.0 {
                        size_inches = Some(((w_cm.powi(2) + h_cm.powi(2)).sqrt()) / 2.54);
                    }

                    let mut best: Option<(u32, u32, f64)> = None;

                    for block in edid[54..126].chunks(18) {
                        let pixel_clock = u16::from_le_bytes([block[0], block[1]]) as u32 * 10_000;
                        if pixel_clock == 0 {
                            continue;
                        }

                        let h_active = (block[4] as u32 & 0xF0) << 4 | block[2] as u32;
                        let h_blank = (block[4] as u32 & 0x0F) << 8 | block[3] as u32;
                        let v_active = (block[7] as u32 & 0xF0) << 4 | block[5] as u32;
                        let v_blank = (block[7] as u32 & 0x0F) << 8 | block[6] as u32;

                        let h_total = h_active + h_blank;
                        let v_total = v_active + v_blank;

                        if h_total == 0 || v_total == 0 {
                            continue;
                        }

                        let r = pixel_clock as f64 / (h_total as f64 * v_total as f64);
                        if r < 20.0 || r > 2000.0 {
                            continue;
                        }

                        match best {
                            None => best = Some((h_active, v_active, r)),
                            Some((bw, bh, br)) => {
                                if (h_active * v_active) > (bw * bh)
                                    || ((h_active * v_active) == (bw * bh) && r > br)
                                {
                                    best = Some((h_active, v_active, r));
                                }
                            }
                        }
                    }

                    if let Some((_, _, r)) = best {
                        refresh_hz = Some(r);
                    }
                }
            }

            let connector = path.file_name().unwrap().to_string_lossy();
            let builtin = connector.contains("eDP") || connector.contains("LVDS");

            return Self {
                name,
                resolution,
                size_inches,
                refresh_hz,
                builtin,
                connected: true,
            };
        }

        Self {
            name: None,
            resolution: None,
            size_inches: None,
            refresh_hz: None,
            builtin: false,
            connected: false,
        }
    }
}

#[derive(Debug)]
pub struct TerminalReadout {
    pub name: Option<String>,
    pub version: Option<String>,
}
impl TerminalReadout {
    pub fn new() -> Self {
        use std::{env, fs};

        // --- env first ---
        let env_name = env::var("TERM_PROGRAM").ok();
        let env_version = env::var("TERM_PROGRAM_VERSION").ok();

        if let Some(name) = env_name {
            return Self {
                name: Some(prettify(&name).to_string()),
                version: env_version,
            };
        }

        // --- fallback helpers ---
        fn is_terminal(name: &str) -> bool {
            matches!(
                name,
                "kitty" | "ghostty" | "wezterm" | "alacritty" | "cosmic-term" | "gnome-terminal"
            )
        }

        fn get_comm(pid: u32) -> Option<String> {
            fs::read_to_string(format!("/proc/{}/comm", pid))
                .ok()
                .map(|s| s.trim().to_string())
        }

        fn get_ppid(pid: u32) -> Option<u32> {
            let stat = fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
            stat.split_whitespace().nth(3)?.parse().ok()
        }

        fn find_terminal_pid() -> Option<(u32, String)> {
            let mut pid = std::process::id();

            while pid > 1 {
                let name = get_comm(pid)?;

                // skip multiplexers
                if matches!(name.as_str(), "tmux" | "screen" | "zellij") {
                    pid = get_ppid(pid)?;
                    continue;
                }

                if is_terminal(&name) {
                    return Some((pid, name));
                }

                pid = get_ppid(pid)?;
            }

            None
        }

        fn resolve_exe(pid: u32) -> Option<String> {
            fs::read_link(format!("/proc/{}/exe", pid))
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
        }

        fn pacman_version(exe: &str) -> Option<String> {
            let entries = fs::read_dir("/var/lib/pacman/local/").ok()?;

            for entry in entries.flatten() {
                let pkg_dir = entry.path();
                let files_path = pkg_dir.join("files");

                let content = fs::read_to_string(files_path).ok()?;

                if content.contains(exe) {
                    let pkg = pkg_dir.file_name()?.to_string_lossy();
                    return pkg.splitn(2, '-').nth(1).map(|s| s.to_string());
                }
            }

            None
        }

        // --- 2. PROC WALK ---
        if let Some((pid, raw_name)) = find_terminal_pid() {
            let pretty = prettify(&raw_name).to_string();
            let version = resolve_exe(pid).and_then(|exe| pacman_version(&exe));

            return Self {
                name: Some(pretty),
                version,
            };
        }

        // --- fallback ---
        Self {
            name: Some("Unknown".to_string()),
            version: None,
        }
    }
}

#[derive(Debug)]
pub struct InterfaceReadout {
    pub name: Result<String, ReadoutError>,
    pub ip: Result<IpAddr, ReadoutError>,
}
impl InterfaceReadout {
    pub fn new() -> Self {
        fn get_default_interface() -> Result<String, ReadoutError> {
            let route = fs::read_to_string("/proc/net/route")?;
            route
                .lines()
                .skip(1)
                .find_map(|line| {
                    let mut cols = line.split_whitespace();
                    let iface = cols.next()?;
                    let dest = cols.next()?;
                    if dest == "00000000" {
                        Some(iface.to_string())
                    } else {
                        None
                    }
                })
                .ok_or_else(|| ReadoutError::NotFound("no default route found".into()))
        }

        fn get_primary_ip() -> Result<IpAddr, ReadoutError> {
            let socket = UdpSocket::bind("0.0.0.0:0")?;
            socket.connect("8.8.8.8:80")?;
            Ok(socket.local_addr()?.ip())
        }
        Self {
            name: get_default_interface(),
            ip: get_primary_ip(),
        }
    }
}

#[derive(Debug)]
pub struct ShellReadout {
    pub name: Option<String>,
    pub path: Option<String>,
    pub version: Option<String>,
}
impl ShellReadout {
    pub fn new() -> Self {
        let shell_path = env::var("SHELL").ok();

        let name = shell_path
            .as_deref()
            .and_then(|p| Path::new(p).file_name())
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());
        let version = name
            .as_deref()
            .and_then(|n| get_shell_version(n, PKG_MANAGER.as_deref()));
        fn get_shell_version(shell: &str, pkg: Option<&str>) -> Option<String> {
            match pkg? {
                "pacman" => get_version_from_pacman(shell),
                "apt" => get_version_from_dpkg(shell),
                "dnf" => get_version_from_rpm(shell),
                _ => None,
            }
        }

        Self {
            name,
            path: shell_path,
            version,
        }
    }
}

#[derive(Debug)]
pub struct PackageReadout {
    pub manager: Option<String>,
    pub count: Option<usize>,
}
impl PackageReadout {
    pub fn new() -> Self {
        let manager = PKG_MANAGER.as_deref().map(|s| s.to_string());
        let count = PKG_MANAGER.as_deref().and_then(|pkg| match pkg {
            "pacman" => fs::read_dir("/var/lib/pacman/local")
                .ok()
                .map(|entries| entries.flatten().count()),
            "apt" => {
                let status = fs::read_to_string("/var/lib/dpkg/status").ok()?;
                Some(status.lines().filter(|l| l.starts_with("Package:")).count())
            }
            "dnf" => {
                let conn = rusqlite::Connection::open("/var/lib/rpm/rpmdb.sqlite").ok()?;
                conn.query_row("SELECT COUNT(*) FROM Packages", [], |row| {
                    row.get::<_, i64>(0)
                })
                .ok()
                .map(|n| n as usize)
            }
            _ => None,
        });
        Self { manager, count }
    }
}
static PRETTIFY: phf::Map<&'static str, &'static str> = phf_map! {
    "xterm-kitty" => "Kitty",
    "kitty" => "Kitty",
    "xterm-ghostty" => "Ghostty",
    "ghostty" => "Ghostty",
    "wezterm" => "WezTerm",
    "WezTerm" => "WezTerm",
    "iTerm.app" => "iTerm2",
    "gnome-terminal" => "GNOME Terminal",
    "alacritty" => "Alacritty",
    "cosmic-term" => "COSMIC Terminal",
    "Hyprland" => "Hyprland",
    "hyprland" => "Hyprland",
    "sway" => "Sway",
    "gnome-shell" => "GNOME",
    "gnome" => "GNOME",
    "GNOME" => "GNOME",
    "kwin_wayland" => "KDE Plasma",
    "kwin_x11" => "KDE Plasma",
    "KDE" => "KDE Plasma",
    "plasma" => "KDE Plasma",
    "xfwm4" => "XFCE",
    "xfce" => "XFCE",
    "openbox" => "Openbox",
    "i3" => "i3",
    "bspwm" => "bspwm",
    "niri" => "Niri",
};

fn prettify(text: &str) -> &str {
    PRETTIFY.get(text).copied().unwrap_or(text)
}

#[derive(Debug)]
pub struct DesktopReadout {
    pub name: Option<String>,
    pub version: Option<String>,
}
impl DesktopReadout {
    pub fn new() -> Self {
        fn is_wm(name: &str) -> bool {
            matches!(
                name,
                "Hyprland"
                    | "hyprland"
                    | "sway"
                    | "kwin_wayland"
                    | "kwin_x11"
                    | "gnome-shell"
                    | "xfwm4"
                    | "openbox"
                    | "i3"
                    | "bspwm"
                    | "niri"
            )
        }

        fn detect() -> Option<String> {
            // fast path: env
            if let Ok(desktop) = env::var("XDG_CURRENT_DESKTOP") {
                if !desktop.is_empty() {
                    return Some(prettify(&desktop).to_string());
                }
            }
            if let Ok(session) = env::var("DESKTOP_SESSION") {
                if !session.is_empty() {
                    return Some(prettify(&session).to_string());
                }
            }
            // scan /proc
            let entries = fs::read_dir("/proc").ok()?;
            for entry in entries.flatten() {
                let fname = entry.file_name();
                let fname = fname.to_string_lossy();
                if !fname.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                if let Ok(comm) = fs::read_to_string(entry.path().join("comm")) {
                    let comm = comm.trim();
                    if is_wm(comm) {
                        return Some(prettify(comm).to_string());
                    }
                }
            }
            None
        }

        let name = detect();

        let version = name.as_deref().and_then(|n| {
            let pkg_name = match n {
                "Hyprland" => "hyprland",
                "Sway" => "sway",
                "KDE Plasma" => "plasma-desktop",
                "GNOME" => "gnome-shell",
                "XFCE" => "xfwm4",
                "i3" => "i3-wm",
                "Niri" => "niri",
                _ => return None,
            };
            match PKG_MANAGER.as_deref() {
                Some("pacman") => get_version_from_pacman(pkg_name),
                Some("apt") => get_version_from_dpkg(pkg_name),
                Some("dnf") => get_version_from_rpm(pkg_name),
                _ => None,
            }
        });

        Self { name, version }
    }
}

#[derive(Debug, Clone)]
pub struct PciReadout {
    pub vendor: Option<u16>,
    pub device: Option<u16>,
}
impl PciReadout {
    pub fn new() -> Vec<Self> {
        let (drm, pci) = rayon::join(Self::try_drm, Self::try_pci_devices);

        if !drm.is_empty() {
            drm
        } else {
            pci
        }
    }

    fn try_drm() -> Vec<PciReadout> {
        let mut devices = Vec::new();
        let entries = match fs::read_dir("/sys/class/drm/") {
            Ok(e) => e,
            Err(_) => return devices,
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("card") || name.contains('-') {
                continue;
            }
            let uevent_path = entry.path().join("device/uevent");
            let content = match fs::read_to_string(uevent_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            for line in content.lines() {
                if let Some(id) = line.strip_prefix("PCI_ID=") {
                    let (v, d) = id.split_once(':').unwrap();
                    let vendor = u16::from_str_radix(v, 16).ok();
                    let device = u16::from_str_radix(d, 16).ok();
                    devices.push(PciReadout { vendor, device });
                    break;
                }
            }
        }
        devices
    }

    fn try_pci_devices() -> Vec<PciReadout> {
        let mut devices = Vec::new();
        let paths = match glob("/sys/bus/pci/devices/*/class") {
            Ok(p) => p,
            Err(_) => return devices,
        };
        for class_path in paths.filter_map(Result::ok) {
            let class = fs::read_to_string(&class_path).unwrap_or_default();
            if !class.trim().starts_with("0x030") {
                continue;
            }
            let device_dir = match class_path.parent() {
                Some(p) => p,
                None => continue,
            };
            let vendor = fs::read_to_string(device_dir.join("vendor"))
                .ok()
                .and_then(|s| u16::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok());
            let device = fs::read_to_string(device_dir.join("device"))
                .ok()
                .and_then(|s| u16::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok());

            devices.push(PciReadout { vendor, device });
        }
        devices
    }
    pub fn as_string(&self) -> Option<String> {
        let (vendor, device) = from_pciid(self.vendor?, self.device?);
        match (vendor, device) {
            (Some(v), Some(d)) => Some(d.to_string()),
            (None, Some(d)) => Some(d.to_string()),
            (Some(v), None) => Some(v.to_string()),
            (None, None) => None,
        }
    }
}
pub fn from_pciid(vendor: u16, device: u16) -> (Option<&'static str>, Option<&'static str>) {
    parse_pciid(vendor, device)
}

#[derive(Debug, Copy, Clone)]
pub struct Kilobytes(pub u64);

impl Kilobytes {
    pub fn to_gb(self) -> f64 {
        self.0 as f64 / (1024.0 * 1024.0)
    }

    pub fn to_mb(self) -> f64 {
        self.0 as f64 / 1024.0
    }
}

fn detect_pkg_manager() -> Option<String> {
    let content = fs::read_to_string("/etc/os-release").ok()?;
    let mut id = None;
    let mut id_like = None;
    // Parse ID and ID_LIKE from /etc/os-release
    for line in content.lines() {
        if let Some(v) = line.strip_prefix("ID=") {
            id = Some(v.trim_matches('"').to_lowercase());
        } else if let Some(v) = line.strip_prefix("ID_LIKE=") {
            id_like = Some(v.trim_matches('"').to_lowercase());
        }
    }
    let family = id_like.as_deref().unwrap_or("");
    // Helper closure to check if the ID or family contains a specific needle
    let is = |needle: &str| id.as_deref() == Some(needle) || family.contains(needle);
    // Return package manager based on ID or ID_LIKE
    if is("arch") {
        return Some("pacman".to_string());
    }
    if is("debian") {
        return Some("apt".to_string());
    }
    if is("fedora") || is("rhel") {
        return Some("dnf".to_string());
    }
    // fallback
    if Path::new("/var/lib/pacman/local").exists() {
        return Some("pacman".to_string());
    }
    if Path::new("/var/lib/dpkg/status").exists() {
        return Some("apt".to_string());
    }
    if Path::new("/var/lib/rpm/Packages.db").exists() {
        return Some("dnf".to_string());
    }
    None
}

fn get_version_from_pacman(pkg: &str) -> Option<String> {
    let entries = fs::read_dir("/var/lib/pacman/local/").ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(rest) = name.strip_prefix(pkg).and_then(|s| s.strip_prefix('-')) {
            // make sure it's the exact package, not e.g. hyprland-guiutils
            if rest.starts_with(|c: char| c.is_ascii_digit()) {
                return rest.split('-').next().map(|s| s.to_string());
            }
        }
    }
    None
}
fn get_version_from_dpkg(shell: &str) -> Option<String> {
    let status = fs::read_to_string("/var/lib/dpkg/status").ok()?;
    let mut in_target_package = false;
    for line in status.lines() {
        if let Some(pkg) = line.strip_prefix("Package: ") {
            in_target_package = pkg.trim().starts_with(shell);
        }
        if in_target_package {
            if let Some(version) = line.strip_prefix("Version: ") {
                return Some(version.split('-').next()?.to_string());
            }
        }
    }
    None
}
fn get_version_from_rpm(shell: &str) -> Option<String> {
    let output = process::Command::new("rpm")
        .args(["-q", "--queryformat", "%{VERSION}", shell])
        .output()
        .ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() && !version.starts_with("not installed") {
            return Some(version);
        }
    }
    None
}
#[derive(Debug)]
pub struct GeneralReadout {
    pub cpu: ProcessorReadout,
    pub memory: MemoryReadout,
    pub disk: DiskReadout,
    pub uptime: UptimeReadout,
    pub os: OsReadout,
    pub board: BoardReadout,
    pub locale: LocaleReadout,
    pub theme: ThemeReadout,
    pub panel: PanelReadout,
    pub interface: InterfaceReadout,
    pub shell: ShellReadout,
    pub desktop: DesktopReadout,
    pub pci: Vec<PciReadout>,
    pub uname: UnameReadout,
    pub sysinfo: SysinfoReadout,
    pub battery: BatteryReadout,
    pub terminal: TerminalReadout,
    pub packages: PackageReadout,
}
impl GeneralReadout {
    pub fn collect_parallel() -> Self {
        let ((cpu, memory), (disk, uptime)) = join(
            || join(ProcessorReadout::new, MemoryReadout::new),
            || join(DiskReadout::new, UptimeReadout::new),
        );

        let ((os, board), (locale, theme)) = join(
            || join(OsReadout::new, BoardReadout::new),
            || join(LocaleReadout::new, ThemeReadout::new),
        );

        let ((panel, interface), (shell, desktop, pci)) = join(
            || join(PanelReadout::new, InterfaceReadout::new),
            || {
                let (shell, desktop) = join(ShellReadout::new, DesktopReadout::new);
                let pci = PciReadout::new();
                (shell, desktop, pci)
            },
        );
        let (uname, sysinfo) = join(UnameReadout::new, SysinfoReadout::new);
        let (battery, terminal) = join(BatteryReadout::new, TerminalReadout::new);
        let packages = PackageReadout::new();

        Self {
            cpu,
            memory,
            disk,
            uptime,
            os,
            board,
            locale,
            theme,
            panel,
            interface,
            shell,
            desktop,
            pci,
            uname,
            sysinfo,
            battery,
            terminal,
            packages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const X86: &str = "processor\t: 0\nvendor_id\t: GenuineIntel\nmodel name\t: Intel(R) Core(TM) i5-8500 CPU @ 3.00GHz\nflags\t\t: fpu sse fma avx avx2\npower management:\n\nprocessor\t: 1\nvendor_id\t: GenuineIntel\nmodel name\t: Intel(R) Core(TM) i5-8500 CPU @ 3.00GHz\n";

    const ARM: &str = "processor\t: 0\nBogoMIPS\t: 108.00\nFeatures\t: fp asimd evtstrm crc32 cpuid\nCPU implementer\t: 0x41\nCPU part\t: 0xd08\n\nprocessor\t: 1\nCPU implementer\t: 0x41\n\nHardware\t: BCM2835\nModel\t\t: Raspberry Pi 4 Model B Rev 1.4\n";

    #[test]
    fn parses_x86() {
        let info = parse_cpuinfo(X86);
        assert_eq!(info.get("vendor_id"), Some(&"GenuineIntel"));
        assert_eq!(
            info.get("model name"),
            Some(&"Intel(R) Core(TM) i5-8500 CPU @ 3.00GHz")
        );
        assert_eq!(info.get("flags"), Some(&"fpu sse fma avx avx2"));
        // empty values like "power management:" are skipped
        assert_eq!(info.get("power management"), None);
    }

    #[test]
    fn parses_arm_trailing_keys() {
        let info = parse_cpuinfo(ARM);
        assert_eq!(info.get("model name"), None);
        assert_eq!(info.get("Hardware"), Some(&"BCM2835"));
        assert_eq!(info.get("CPU implementer"), Some(&"0x41"));
        assert_eq!(info.get("Features"), Some(&"fp asimd evtstrm crc32 cpuid"));
    }

    #[test]
    fn maps_arm_implementers() {
        assert_eq!(arm_implementer_name("0x41"), "ARM");
        assert_eq!(arm_implementer_name("0x61"), "Apple");
        assert_eq!(arm_implementer_name("0xff"), "0xff");
    }

    #[test]
    fn reads_real_cpuinfo() {
        let cpu = ProcessorReadout::new();
        if cfg!(target_arch = "x86_64") {
            assert!(cpu.model_name.is_ok());
            assert!(cpu.vendor.is_ok());
        }
    }
}
