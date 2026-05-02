<div align="center">

# setch

**A lightweight, blazing fast, and highly configurable system fetch tool written in Rust.**

[![Crates.io](https://img.shields.io/crates/v/setch.svg)](https://crates.io/crates/setch)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

</div>

---

## Features

- 🚀 **Daemon mode** — pre-collect system info in the background for near-instant display
- 🌙 **Lua config** — full scripting language for your config, not just key-value pairs
- 🖼️ **Image support** — display a PNG as your logo via the raw Kitty graphics protocol
- 🎨 **Per-module overrides** — custom keys, colors, separators, and format strings per module
- 📺 **Rich display info** — `1920x1080 15", 120 Hz [Built-in]` instead of just a resolution
- 🎭 **Built-in presets** — Catppuccin, Pastel, Pfetch, and Default out of the box
- ⚡ **Benchmarking** — see exactly how fast setch is on your machine

---

## Installation

```bash
cargo install setch
```

Or build from source:

```bash
git clone https://github.com/yourusername/setch
cd setch
cargo build --release
```

---

## Usage

```bash
setch                  # run normally
setch --small          # use small ASCII logo
setch --daemon / -d    # start the background daemon
setch --preset         # interactive preset picker
setch --benchmark / -b # show execution time
```

---

## Configuration

setch is configured via Lua at `~/.config/setch/setch.lua`. A default config is created on first run.

### Minimal example

```lua
return {
  main_color    = "#b4befe",
  subtext       = "#cdd6f4",
  name_color    = "#b4befe",
  separator     = "━━━━━━━━━━━━━━━━━━━━━━━━",
  colors_format = "● ",

  display = {
    separator = { key = "  ", color = "#b4befe" }
  },

  ascii = {
    enabled = true,
    type    = "auto",  -- "auto", "big", "small", "custom", "none"
    color   = "#b4befe",
  },

  fetch_mode = "auto", -- "auto", "daemon", "runtime"

  modules = {
    "name",
    "separator",
    "os",
    "host",
    "kernel",
    "uptime",
    "packages",
    "shell",
    "terminal",
    "wm",
    "resolution",
    "cpu",
    "gpu",
    "memory",
    "disk",
    "battery",
    "ip",
    "locale",
    "separator",
    "colors",
  }
}
```

### Per-module overrides

Any module can be specified as a table instead of a string for fine-grained control:

```lua
modules = {
  "name",
  "separator",
  { type = "os",     key = "System"                              },
  { type = "cpu",    fmt = "{cpu} ({cores}C/{threads}T) @ {freq}" },
  { type = "memory", fmt = "{used}GiB / {total}GiB ({percent}%)" },
  { type = "uptime", key_color = "#f38ba8"                       },
  { type = "custom", key = "Editor",  text = "neovim"            },
  { type = "command",key = "IP",      text = "curl -s ifconfig.me"},
  "separator",
  "colors",
}
```

### Custom ASCII image (Kitty protocol)

```lua
ascii = {
  type       = "custom",
  path       = "~/.config/setch/logo.png",
  width      = 20,
  height     = 10,
  top_padding = 0,
  padding    = 3,
}
```

> Requires a Kitty-protocol compatible terminal: Kitty, Ghostty, WezTerm, or Rio.

---

## Daemon Mode

The daemon (`setchd`) runs in the background and keeps system info cached. When active, setch reads from the cache instead of querying the system — making startup nearly instantaneous.

```bash
# Start the daemon
setch -d

# In your config, set:
fetch_mode = "daemon"
```

With `fetch_mode = "auto"` (default), setch will use the daemon if it's running and fall back to direct collection if not.

---

## Modules

| Module | Description |
|---|---|
| `name` | `user@hostname` |
| `os` | Operating system + architecture |
| `host` | Machine/product name |
| `kernel` | Kernel name and release |
| `uptime` | System uptime |
| `packages` | Package count with manager name |
| `shell` | Shell name and version |
| `terminal` | Terminal name and version |
| `wm` | Window manager / desktop and version |
| `resolution` | Display info: resolution, size, refresh rate |
| `cpu` | CPU name, core/thread count, frequency |
| `gpu` | All GPUs, indexed |
| `memory` | RAM used/total + swap |
| `disk` | Disk used/total for `/` |
| `battery` | Battery percentage and charge status |
| `ip` | Local IP and interface name |
| `locale` | System locale |
| `separator` | Renders the separator line |
| `break` | Empty line |
| `colors` | Terminal color palette swatches |
| `custom` | Static custom key-value |
| `command` | Output of a shell command |

See [docs/modules.md](docs/modules.md) for all available format tokens per module.

---

## Presets

```bash
setch --preset
```

Launches an interactive picker. Available presets:

- **Default** — classic catppuccin-adjacent look
- **Catppuccin** — full Catppuccin Mocha palette
- **Pastel** — softer pastel tones
- **Pfetch** — minimal pfetch-inspired layout

---

## Supported Distros

ASCII logos are included for: Arch Linux, NyArch, Fedora, Ubuntu, Debian. All other distros fall back to a generic logo.

---

## License

MIT
