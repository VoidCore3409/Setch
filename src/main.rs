#![cfg(target_os = "linux")]
use base64::{Engine, engine::general_purpose};
use inquire::Select;
use mlua::prelude::*;
use owo_colors::OwoColorize;
use shellexpand;
use std::env;
use std::fs;
use std::sync::LazyLock;
use std::time;

mod setchd;
use crate::cache::SetchCache;
mod ascii;
mod presets;
use crate::presets::*;
mod cache;

static HOME: LazyLock<String> =
    LazyLock::new(|| env::var("HOME").unwrap_or_else(|_| "/root".to_string()));

pub struct ConfigCache {
    // Basic Colors
    pub main_color: String,
    pub subtext_color: String,
    pub name_color: String,

    // Separator Config
    pub key_sep: String,
    pub key_sep_color: String,
    pub separator_line: String, // The "━━━━" string
    pub separator_color: String,

    // Global Formats
    pub color_format: String, // e.g. "󱕍 "

    // Ascii/Logo Settings
    pub ascii_enabled: bool,
    pub ascii_type: String, // "big", "small", "custom"
    pub ascii_color: String,
    pub ascii_path: String,
    pub ascii_width: u32,
    pub ascii_height: u32,
    pub top_padding: i64,
    pub padding: i64,
}

impl ConfigCache {
    pub fn from_lua(lua: &Lua, table: &LuaTable) -> Self {
        // Extract nested display table
        let display: LuaTable = table
            .get("display")
            .unwrap_or_else(|_| lua.create_table().unwrap());
        let sep_cfg: LuaTable = display
            .get("separator")
            .unwrap_or_else(|_| lua.create_table().unwrap());

        // Extract nested ascii table
        let ascii: LuaTable = table
            .get("ascii")
            .unwrap_or_else(|_| lua.create_table().unwrap());

        // Get main color first to use as a fallback for others
        let main = table
            .get("main_color")
            .unwrap_or_else(|_| "#b4befe".to_string());

        Self {
            main_color: main.clone(),
            subtext_color: table
                .get("subtext")
                .unwrap_or_else(|_| "#cdd6f4".to_string()),
            name_color: table.get("name_color").unwrap_or_else(|_| main.clone()),

            key_sep: sep_cfg.get("key").unwrap_or_else(|_| "  ".to_string()),
            key_sep_color: sep_cfg.get("color").unwrap_or_else(|_| main.clone()),
            separator_line: table
                .get("separator")
                .unwrap_or_else(|_| "━━━━━━━━━━━━━━━━━━━━━━━━".to_string()),
            separator_color: table
                .get("separator_color")
                .unwrap_or_else(|_| "#45475a".to_string()),

            color_format: table
                .get("colors_format")
                .unwrap_or_else(|_| "● ".to_string()),

            ascii_enabled: ascii.get("enabled").unwrap_or(true),
            ascii_type: ascii.get("type").unwrap_or_else(|_| "auto".to_string()),
            ascii_color: ascii.get("color").unwrap_or_else(|_| main.clone()),
            ascii_path: ascii.get("path").unwrap_or_else(|_| "".to_string()),
            ascii_width: ascii.get("width").unwrap_or(20),
            ascii_height: ascii.get("height").unwrap_or(10),
            top_padding: ascii.get("top_padding").unwrap_or(0),
            padding: ascii.get("padding").unwrap_or(3),
        }
    }
}

fn load_lua_config(lua: &Lua) -> LuaResult<LuaTable> {
    let config_dir = format!("{}/.config/setch", *HOME);
    let config_path = format!("{}/setch.lua", config_dir);

    if !std::path::Path::new(&config_path).exists() {
        let _ = std::fs::create_dir_all(&config_dir);
        let _ = std::fs::write(&config_path, DEFAULT);
    }

    let content = std::fs::read_to_string(&config_path).expect("failed to read config file");

    // Use .call() or .eval() but make sure the Lua code returns a table!
    // If your Lua file looks like: config = { ... }, you need to return it.
    lua.load(&content).eval::<LuaTable>()
}

pub struct RawModuleObject {
    pub module_type: Option<String>,
    pub key: Option<String>,
    pub key_color: Option<String>,
    pub key_sep: Option<String>,
    pub key_sep_color: Option<String>,
    pub fmt: Option<String>,
    pub format: Option<String>,
    pub text: Option<String>,
    pub username_color: Option<String>,
    pub hostname_color: Option<String>,
    pub index: Option<bool>,
}

pub enum RawModule {
    String(String),
    Object(RawModuleObject),
}

impl RawModule {
    pub fn from_lua(value: LuaValue) -> Self {
        match value {
            // Case 1: Simple string like "os" or "kernel"
            LuaValue::String(s) => RawModule::String(
                s.to_str()
                    .map(|res| res.to_string())
                    .unwrap_or_else(|_| "".to_string()),
            ),

            // Case 2: Table with metadata
            LuaValue::Table(t) => {
                // We check for "type" first, just like an untagged enum!
                let m_type: Option<String> = t.get("type").ok();

                RawModule::Object(RawModuleObject {
                    module_type: m_type,
                    key: t.get("key").ok(),
                    key_color: t.get("key_color").ok(),
                    key_sep: t.get("key_sep").ok(),
                    key_sep_color: t.get("key_sep_color").ok(),
                    fmt: t.get("fmt").ok(),
                    format: t.get("format").ok(),
                    text: t.get("text").ok(),
                    username_color: t.get("username_color").ok(),
                    hostname_color: t.get("hostname_color").ok(),
                    index: t.get("index").ok(),
                })
            }
            _ => RawModule::String("unknown".to_string()),
        }
    }
}
#[derive(Default, Debug)]
pub struct Overrides {
    pub key: Option<String>,
    pub fmt: Option<String>,
    pub key_color: Option<String>,
    pub key_sep: Option<String>,
    pub key_sep_color: Option<String>,
    pub username_color: Option<String>,
    pub hostname_color: Option<String>,
    pub index: Option<bool>,
}

#[derive(Debug)]
pub enum Module {
    Name(Overrides),
    Break,
    Separator,
    Colors(Overrides),

    Os(Overrides),
    Host(Overrides),
    Kernel(Overrides),
    Uptime(Overrides),
    Packages(Overrides),
    Shell(Overrides),
    Resolution(Overrides),
    WM(Overrides),
    Theme(Overrides),
    Icon(Overrides),
    Terminal(Overrides),
    Cpu(Overrides),
    Gpu(Overrides),
    Memory(Overrides),
    Disk(Overrides),
    LocalIP(Overrides),
    Interface(Overrides),
    Locale(Overrides),
    Battery(Overrides),

    Custom(Overrides, String),
    Command { cmd: String, overrides: Overrides },
    Unknown(String),
}

impl From<RawModule> for Module {
    fn from(raw: RawModule) -> Self {
        match raw {
            // "cpu", "memory", "break"
            RawModule::String(name) => match name.as_str() {
                "name" => Module::Name(Overrides::default()),
                "break" => Module::Break,
                "separator" => Module::Separator,
                "colors" => Module::Colors(Overrides::default()),
                "os" => Module::Os(Overrides::default()),
                "host" => Module::Host(Overrides::default()),
                "kernel" => Module::Kernel(Overrides::default()),
                "uptime" => Module::Uptime(Overrides::default()),
                "packages" => Module::Packages(Overrides::default()),
                "shell" => Module::Shell(Overrides::default()),
                "terminal" => Module::Terminal(Overrides::default()),
                "wm" => Module::WM(Overrides::default()),
                "theme" => Module::Theme(Overrides::default()),
                "icons" => Module::Icon(Overrides::default()),
                "resolution" => Module::Resolution(Overrides::default()),
                "cpu" => Module::Cpu(Overrides::default()),
                "gpu" => Module::Gpu(Overrides::default()),
                "memory" => Module::Memory(Overrides::default()),
                "disk" => Module::Disk(Overrides::default()),
                "battery" => Module::Battery(Overrides::default()),
                "ip" => Module::LocalIP(Overrides::default()),
                "locale" => Module::Locale(Overrides::default()),
                other => Module::Unknown(other.into()),
            },

            // { "type": "cpu", "key": "Processor" }
            RawModule::Object(obj) => {
                let overrides = Overrides {
                    fmt: obj.fmt,
                    key: obj.key,
                    key_color: obj.key_color,
                    key_sep: obj.key_sep,
                    key_sep_color: obj.key_sep_color,
                    username_color: obj.username_color,
                    hostname_color: obj.hostname_color,
                    index: obj.index,
                };

                match obj.module_type.as_deref() {
                    Some("name") => Module::Name(overrides),
                    Some("os") => Module::Os(overrides),
                    Some("host") => Module::Host(overrides),
                    Some("kernel") => Module::Kernel(overrides),
                    Some("uptime") => Module::Uptime(overrides),
                    Some("packages") => Module::Packages(overrides),
                    Some("shell") => Module::Shell(overrides),
                    Some("terminal") => Module::Terminal(overrides),
                    Some("wm") => Module::WM(overrides),
                    Some("theme") => Module::Theme(overrides),
                    Some("icons") => Module::Icon(overrides),
                    Some("resolution") => Module::Resolution(overrides),
                    Some("cpu") => Module::Cpu(overrides),
                    Some("gpu") => Module::Gpu(overrides),
                    Some("memory") => Module::Memory(overrides),
                    Some("disk") => Module::Disk(overrides),
                    Some("battery") => Module::Battery(overrides),
                    Some("ip") => Module::LocalIP(overrides),
                    Some("locale") => Module::Locale(overrides),
                    Some("break") => Module::Break,
                    Some("custom") => Module::Custom(overrides, obj.text.unwrap_or_default()),
                    Some("command") => Module::Command {
                        cmd: obj.text.unwrap_or_default(),
                        overrides,
                    },
                    Some("colors") => Module::Colors(overrides),
                    Some(other) => Module::Unknown(other.into()),
                    None => Module::Unknown("missing_type".into()),
                }
            }
        }
    }
}

fn get_key<'a>(o: &'a Overrides, default: &'a str) -> &'a str {
    o.key.as_deref().unwrap_or(default)
}
fn get_key_sep<'a>(cache: &'a ConfigCache, o: &'a Overrides) -> &'a str {
    o.key_sep.as_deref().unwrap_or(&cache.key_sep)
}
fn get_key_color<'a>(cache: &'a ConfigCache, o: &'a Overrides) -> &'a str {
    // If the module has a specific color, use it.
    // Otherwise, use the main_color from our config.
    o.key_color.as_deref().unwrap_or(&cache.main_color)
}
fn get_key_sep_color<'a>(cache: &'a ConfigCache, o: &'a Overrides) -> &'a str {
    o.key_sep_color.as_deref().unwrap_or(&cache.key_sep_color)
}
fn hex_to_rgb(hex: &str) -> owo_colors::Rgb {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
    owo_colors::Rgb(r, g, b)
}

fn colorize(text: &str, hex: &str) -> String {
    let owo_colors::Rgb(r, g, b) = hex_to_rgb(hex);
    format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, text)
}

fn render_line(cache: &ConfigCache, o: &Overrides, key: &str, value: &str) -> String {
    let k_color = get_key_color(cache, o);
    let sep = get_key_sep(cache, o);
    let s_color = get_key_sep_color(cache, o);

    format!(
        "{}{} {}",
        colorize(key, k_color),
        colorize(sep, s_color),
        value
    )
}
fn render_separator(cache: &ConfigCache) -> String {
    // Instead of cloning a global, we use the values from our cache
    // and colorize them instantly!
    colorize(&cache.separator_line, &cache.separator_color)
}

fn render_name(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let fmt = o.fmt.as_deref().unwrap_or("{user}@{host}");
    let key = o.key.as_deref().unwrap_or("");
    let uc = o.username_color.as_deref().unwrap_or(&cache.name_color);
    let hc = o.hostname_color.as_deref().unwrap_or(&cache.name_color);
    let username = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let output = fmt
        .replacen("{user}", &colorize(&username, uc), 1)
        .replacen("{host}", &colorize(&sys.hostname, hc), 1);
    format!("{}{}", colorize(key, &cache.main_color), output)
}

fn render_os(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let output = o.fmt.as_deref().unwrap_or("{os}").replace("{os}", &sys.os);
    render_line(cache, o, get_key(o, "OS"), &output)
}

fn render_host(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{name}")
        .replace("{name}", &sys.host);
    render_line(cache, o, get_key(o, "Host"), &output)
}

fn render_custom(cache: &ConfigCache, o: &Overrides, text: String) -> String {
    render_line(cache, o, o.key.as_deref().unwrap_or(""), &text)
}

fn render_uptime(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let secs = sys.uptime_secs;
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{days}d {hours}h {minutes}m")
        .replace("{days}", &(secs / 86400).to_string())
        .replace("{hours}", &((secs % 86400) / 3600).to_string())
        .replace("{minutes}", &((secs % 3600) / 60).to_string())
        .replace("{seconds}", &(secs % 60).to_string());
    render_line(cache, o, get_key(o, "Uptime"), &output)
}

fn render_pkgs(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{packages} ({manager})")
        .replace("{packages}", &sys.packages.to_string())
        .replace("{manager}", &sys.pkg_manager);
    render_line(cache, o, get_key(o, "Packages"), &output)
}

fn render_shell(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{name} {version}")
        .replace("{name}", &sys.shell)
        .replace("{version}", &sys.shell_version);
    render_line(cache, o, get_key(o, "Shell"), &output)
}

fn render_terminal(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{term} {version}")
        .replace("{term}", &sys.terminal)
        .replace("{version}", &sys.terminal_version);
    render_line(cache, o, get_key(o, "Terminal"), &output)
}

fn render_wm(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{name} {version}")
        .replace("{name}", &sys.wm)
        .replace("{version}", &sys.wm_version);
    render_line(cache, o, get_key(o, "WM"), &output)
}

fn render_theme(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{theme}")
        .replace("{theme}", &sys.theme);
    render_line(cache, o, get_key(o, "Theme"), &output)
}

fn render_icon(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{icon}")
        .replace("{icon}", &sys.icon);
    render_line(cache, o, get_key(o, "Icons"), &output)
}

fn render_locale(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{lang}")
        .replace("{lang}", &sys.locale);
    render_line(cache, o, get_key(o, "Locale"), &output)
}

fn render_local_ip(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{ip} ({iface})")
        .replace("{ip}", &sys.ip)
        .replace("{iface}", &sys.iface);
    render_line(cache, o, get_key(o, "Local IP"), &output)
}

fn render_panel(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let size = if sys.display_size > 0.0 {
        format!("{:.0}\"", sys.display_size)
    } else {
        String::new()
    };
    let refresh = if sys.refresh_hz > 0.0 {
        format!(", {:.0} Hz", sys.refresh_hz)
    } else {
        String::new()
    };
    let builtin = if sys.display_builtin {
        "[Built-in]"
    } else {
        ""
    };
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{name} {resolution} {size}{refresh} {builtin}")
        .replace("{name}", &sys.display_name)
        .replace("{resolution}", &sys.resolution)
        .replace("{size}", &size)
        .replace("{refresh}", &refresh)
        .replace("{builtin}", builtin);
    render_line(cache, o, get_key(o, "Display"), output.trim())
}

fn render_cpu(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let freq = if sys.cpu_max_freq > 0.0 {
        format!("{:.2} GHz", sys.cpu_max_freq)
    } else {
        String::new()
    };
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{cpu} ({cores}C/{threads}T)")
        .replace("{cpu}", &sys.cpu)
        .replace("{vendor}", &sys.cpu_vendor)
        .replace("{cores}", &sys.cpu_cores.to_string())
        .replace("{threads}", &sys.cpu_threads.to_string())
        .replace("{freq}", &freq)
        .replace("{arch}", &sys.cpu_arch);
    render_line(cache, o, get_key(o, "CPU"), &output)
}

fn render_gpu(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let key = get_key(o, "GPU");
    let show_index = o.index.unwrap_or(true);
    let k_col = get_key_color(cache, o);
    let sep = get_key_sep(cache, o);
    let s_col = get_key_sep_color(cache, o);

    if sys.gpus.is_empty() {
        return render_line(cache, o, key, "Unknown");
    }

    sys.gpus
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if show_index {
                format!(
                    "{} {}{} {}",
                    colorize(key, k_col),
                    colorize(&(i + 1).to_string(), k_col),
                    colorize(sep, s_col),
                    name
                )
            } else {
                render_line(cache, o, key, name)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_memory(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{used}GiB / {total}GiB ({percent}%)")
        .replace("{used}", &format!("{:.2}", sys.memory_used))
        .replace("{total}", &format!("{:.2}", sys.memory_total))
        .replace("{percent}", &format!("{:.1}", sys.memory_percent))
        .replace("{swap_used}", &format!("{:.2}", sys.swap_used))
        .replace("{swap_total}", &format!("{:.2}", sys.swap_total));
    render_line(cache, o, get_key(o, "Memory"), &output)
}

fn render_disk(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{used}GiB / {total}GiB ({percent}%)")
        .replace("{used}", &format!("{:.2}", sys.disk_used))
        .replace("{total}", &format!("{:.2}", sys.disk_total))
        .replace("{percent}", &format!("{:.1}", sys.disk_percent))
        .replace("{fs}", &sys.disk_filesystem);
    render_line(cache, o, get_key(o, "Disk"), &output)
}

fn render_battery(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let status = match sys.battery_status.as_str() {
        "Charging" => "AC Connected, Charging",
        "Full" => "AC Connected, Full",
        other => other,
    };
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{capacity}% [{status}]")
        .replace("{capacity}", &sys.battery_capacity.to_string())
        .replace("{status}", status);
    render_line(cache, o, get_key(o, "Battery"), &output)
}

fn render_kernel(cache: &ConfigCache, sys: &SetchCache, o: &Overrides) -> String {
    let output = o
        .fmt
        .as_deref()
        .unwrap_or("{sysname} {release}")
        .replace("{sysname}", &sys.sysname)
        .replace("{release}", &sys.kernel)
        .replace("{arch}", &sys.arch);
    render_line(cache, o, get_key(o, "Kernel"), &output)
}

// Update this function signature
fn render_colors(cache: &ConfigCache, o: &Overrides) -> String {
    let key = o.key.as_deref().unwrap_or("");
    let f = &cache.color_format; // Use the format from the cache!

    let colors = format!(
        "{}{}{}{}{}{}{}{}",
        f.black(),
        f.red(),
        f.green(),
        f.yellow(),
        f.blue(),
        f.magenta(),
        f.cyan(),
        f.white()
    );
    let value = o
        .fmt
        .as_deref()
        .unwrap_or("{colors}")
        .replace("{colors}", &colors);
    format!("{}{}", colorize(key, get_key_color(cache, o)), value)
}
fn render_break() -> String {
    "\u{00A0}".to_string() // non-breaking space, not stripped by comfy_table
}
fn render_module(cache: &ConfigCache, sys: &SetchCache, module: Module) -> Option<String> {
    match module {
        Module::Break => Some(render_break()),
        Module::Separator => Some(render_separator(cache)),
        Module::Colors(o) => Some(render_colors(cache, &o)),
        Module::Name(o) => Some(render_name(cache, sys, &o)),
        Module::Os(o) => Some(render_os(cache, sys, &o)),
        Module::Host(o) => Some(render_host(cache, sys, &o)),
        Module::Kernel(o) => Some(render_kernel(cache, sys, &o)),
        Module::Uptime(o) => Some(render_uptime(cache, sys, &o)),
        Module::Packages(o) => Some(render_pkgs(cache, sys, &o)),
        Module::Shell(o) => Some(render_shell(cache, sys, &o)),
        Module::Terminal(o) => Some(render_terminal(cache, sys, &o)),
        Module::WM(o) => Some(render_wm(cache, sys, &o)),
        Module::Theme(o) => Some(render_theme(cache, sys, &o)),
        Module::Icon(o) => Some(render_icon(cache, sys, &o)),
        Module::Resolution(o) => Some(render_panel(cache, sys, &o)),
        Module::Cpu(o) => Some(render_cpu(cache, sys, &o)),
        Module::Gpu(o) => Some(render_gpu(cache, sys, &o)),
        Module::Memory(o) => Some(render_memory(cache, sys, &o)),
        Module::Disk(o) => Some(render_disk(cache, sys, &o)),
        Module::Battery(o) => Some(render_battery(cache, sys, &o)),
        Module::LocalIP(o) => Some(render_local_ip(cache, sys, &o)),
        Module::Locale(o) => Some(render_locale(cache, sys, &o)),
        Module::Custom(o, t) => Some(render_custom(cache, &o, t)),
        Module::Unknown(key) => Some(format!(
            "{}{} Unknown Module",
            colorize(&key, &cache.main_color),
            colorize(&cache.key_sep, &cache.key_sep_color),
        )),
        _ => None,
    }
}
#[derive(Debug, Clone)]
pub struct Theme {
    pub separator: String,
}
fn set_preset(preset: &str) {
    let chosen = match preset {
        "pfetch" => PFETCH,
        "default" => DEFAULT,
        "catppuccin" => CATPPUCCIN,
        "pastel" => PASTEL,
        _ => return,
    };

    let home = std::env::var("HOME").unwrap();
    let config_dir = format!("{}/.config/setch", home);
    let config_path = format!("{}/setch.lua", config_dir);

    std::fs::create_dir_all(&config_dir).ok();
    std::fs::write(&config_path, chosen).expect("failed to write preset");
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn render_fetch_image(
    path: &str,
    info: &[String],
    width: u32,
    height: u32,
    top_padding: i64,
    padding: i64,
) {
    let data = std::fs::read(path).unwrap();
    let encoded = general_purpose::STANDARD.encode(&data);

    let mut chunks = encoded.as_bytes().chunks(4096).peekable();
    let mut first = true;
    println!(); // push past cargo output

    if top_padding > 0 {
        print!("\x1b[{}A", top_padding);
    }

    // 1. send image FIRST
    while let Some(chunk) = chunks.next() {
        let more = if chunks.peek().is_some() { 1 } else { 0 };
        if first {
            print!(
                "\x1b_Ga=T,f=100,c={},r={},m={};{}\x1b\\",
                width,
                height,
                more,
                std::str::from_utf8(chunk).unwrap()
            );
            first = false;
        } else {
            print!(
                "\x1b_Gm={};{}\x1b\\",
                more,
                std::str::from_utf8(chunk).unwrap()
            );
        }
    }

    // 2. move back up to top of image
    // 3. apply top padding (push DOWN)
    //print!("\x1b[{}B", top_padding);

    // 3. move up to info start position
    // height gets us to image top, then we go back down top_padding to align with image
    print!("\x1b[{}A", height);
    if top_padding > 0 {
        print!("\x1b[{}B", top_padding);
    }

    // 4. print info
    for line in info.iter().skip(top_padding as usize) {
        print!("\x1b[{}G", width + padding as u32);
        println!("{}", line);
    }
}

fn render_fetch(logo: &[&str], info: &[String], ascii_color: &str) {
    let max_rows = logo.len().max(info.len());
    let logo_width = logo
        .iter()
        .map(|l| strip_ansi(l).chars().count())
        .max()
        .unwrap_or(0);

    let logo_pad = (max_rows - logo.len()) / 2;
    let info_pad = (max_rows - info.len()) / 2;

    for i in 0..max_rows {
        let left = if i < logo_pad || i >= logo_pad + logo.len() {
            " ".repeat(logo_width)
        } else {
            let line = logo[i - logo_pad];
            let colored = colorize(line, ascii_color).to_string();
            let pad = logo_width - strip_ansi(line).chars().count();
            format!("{}{}", colored, " ".repeat(pad))
        };

        let right = if i < info_pad || i >= info_pad + info.len() {
            String::new()
        } else {
            info[i - info_pad].clone()
        };

        println!("{}   {}", left, right);
    }
}
fn main() -> LuaResult<()> {
    let lua = Lua::new();
    let args: Vec<String> = env::args().collect();

    // Handle presets first...
    if args.len() > 1 && args[1] == "--preset" {
        let options: Vec<&str> = vec!["Pfetch", "Default", "Pastel", "Catppuccin"];
        if let Ok(choice) = Select::new("Pick a preset: ", options).prompt() {
            set_preset(&choice.to_lowercase());
        }
        return Ok(());
    }

    let config_table = load_lua_config(&lua)?;
    let cache = ConfigCache::from_lua(&lua, &config_table);
    let fetch_mode: String = config_table
        .get("fetch_mode")
        .unwrap_or_else(|_| "auto".to_string());
    let sys: SetchCache = match fetch_mode.as_str() {
        "daemon" => {
            if setchd::is_alive() {
                cache::read_from_cache()
            } else {
                eprintln!(
                    "setch: daemon mode is set but setchd is not running. start it with 'setch -d'"
                );
                std::process::exit(1);
            }
        }
        "runtime" => setchd::collect_once(),
        _ => {
            if setchd::is_alive() {
                cache::read_from_cache()
            } else {
                setchd::collect_once()
            }
        }
    };
    let daemon = args.len() > 1 && ["--daemon", "-d"].contains(&args[1].as_str());
    if daemon {
        setchd::run(); // blocks forever, never returns
    }

    // --- EXTRACT CONFIG VALUES HERE ---
    let ascii_cfg: LuaTable = config_table
        .get("ascii")
        .unwrap_or_else(|_| lua.create_table().unwrap());

    let ascii_enabled: bool = ascii_cfg.get("enabled").unwrap_or(true);
    let ascii_type: String = ascii_cfg.get("type").unwrap_or_else(|_| "auto".to_string());
    let ascii_path_raw: String = ascii_cfg.get("path").unwrap_or_default();

    // --- MODULES LOADING ---
    let modules_table: LuaTable = config_table.get("modules")?;
    let mut modules = Vec::new();
    for i in 1..=modules_table.len()? {
        let val: LuaValue = modules_table.get(i)?;
        modules.push(Module::from(RawModule::from_lua(val)));
    }

    // --- ASCII LOGIC ---
    let mut ascii_path = String::new();
    if cache.ascii_type == "custom" && !ascii_path_raw.is_empty() {
        ascii_path = shellexpand::tilde(&ascii_path_raw).to_string();
    }

    let mut ascii_file: Vec<String> = vec![];
    if !ascii_path.is_empty() && !ascii_path.ends_with(".png") {
        let content = fs::read_to_string(&ascii_path).unwrap_or_default();
        ascii_file = content.lines().map(|l| l.to_string()).collect();
    }

    let use_small = (args.len() > 1 && args[1] == "--small") || ascii_type == "small";
    let benchmark = args.len() > 1 && ["--benchmark", "-b"].contains(&args[1].as_str());
    let start = time::Instant::now();

    // --- DISTRO MATCHING ---
    use crate::ascii::*;
    let (ascii, ascii_small) = match &sys.os {
        // Use *DISTRO because it's a lazy_static
        name if name.contains("Arch") => (ARCH, ARCH_SMALL),
        name if name.contains("Nyarch") => (NYARCH, NYARCH_SMALL),
        name if name.contains("Fedora") => (FEDORA, FEDORA_SMALL),
        name if name.contains("Ubuntu") => (UBUNTU, UBUNTU_SMALL),
        name if name.contains("Debian") => (DEBIAN, DEBIAN_SMALL),
        _ => (DEFAULT, DEFAULT_SMALL),
    };

    // --- RENDERING ---
    let info_lines: Vec<String> = modules
        .into_iter()
        .filter_map(|m| render_module(&cache, &sys, m))
        .flat_map(|s| s.lines().map(|l| l.to_string()).collect::<Vec<_>>())
        .collect();

    print!("\x1b[?7l");
    if cache.ascii_type == "custom" && !ascii_path.is_empty() {
        if ascii_path.ends_with(".png") {
            // Pass the padding here! ⚡
            render_fetch_image(
                &ascii_path,
                &info_lines,
                cache.ascii_width,
                cache.ascii_height,
                cache.top_padding,
                cache.padding,
            );
        } else {
            let refs: Vec<&str> = ascii_file.iter().map(|s| s.as_str()).collect();
            render_fetch(&refs, &info_lines, &cache.ascii_color);
        }
    } else {
        let active_logo = match (ascii_enabled, use_small) {
            (false, _) => NONE,
            (true, true) => ascii_small,
            (true, false) => ascii,
        };
        render_fetch(active_logo, &info_lines, &cache.ascii_color);
    }

    if benchmark {
        println!("Execution Time: {:.5?}", start.elapsed());
    }
    print!("\x1b[?7h");
    Ok(())
}
