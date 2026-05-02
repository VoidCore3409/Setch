# Changelog

All notable changes to setch will be documented here.

---

## [0.2.0] - 2026-05-02 — "The Big One"

### ⚠️ Breaking Changes

- **JSONC config is gone.** Configuration has been fully migrated to **Lua** (`~/.config/setch/setch.lua`). Your old `setch.jsonc` will no longer be read. Run `setch` once to generate a new default Lua config, or pick a preset with `setch --preset`.
- The `dgpu` / `igpu` module split has been replaced by a unified `gpu` module that lists all GPUs automatically.
- `resolution` module now renders as `Display` with richer info (see below).

---

### 🚀 New Features

#### Lua Configuration
Config is now written in Lua — a real scripting language. This means you can use conditionals, variables, loops, and functions in your config. The config file lives at `~/.config/setch/setch.lua` and must return a table.

```lua
return {
  main_color = "#b4befe",
  modules = { "name", "separator", "os", "cpu", "memory" }
}
```

#### Daemon Mode (`setchd`)
setch now ships with a background daemon (`setchd`) that pre-collects system info and writes it to a cache. When the daemon is running, setch reads from cache instead of querying the system — making it nearly instantaneous.

```bash
setch -d        # start the daemon
setch --daemon  # same thing
```

Set `fetch_mode = "daemon"` in your config to always prefer the cache. Set `fetch_mode = "runtime"` to always collect fresh. The default (`"auto"`) uses the daemon if it's running, otherwise collects directly.

#### Per-Module Format Strings & Overrides
Every module now supports per-instance overrides via Lua table syntax:

```lua
modules = {
  { type = "cpu", key = "Processor", fmt = "{cpu} @ {freq}" },
  { type = "memory", fmt = "{used}GiB / {total}GiB" },
  { type = "uptime", key_color = "#f38ba8" },
}
```

Available format tokens vary per module. See [docs/modules.md](docs/modules.md) for the full list.

#### Richer Display Info
The `resolution` module is now the `Display` module and shows:

```
Display  1920x1080 15", 120 Hz [Built-in]
```

Tokens available: `{resolution}`, `{size}`, `{refresh}`, `{builtin}`, `{name}`.

#### Image ASCII Support (Kitty Protocol)
You can now use a PNG as your logo via the raw Kitty graphics protocol:

```lua
ascii = {
  type = "custom",
  path = "~/.config/setch/logo.png",
  width = 20,
  height = 10,
  top_padding = 0,
  padding = 3,
}
```

Requires a Kitty-protocol compatible terminal (Kitty, Ghostty, WezTerm, Rio).

#### Presets
Pick a built-in theme with:

```bash
setch --preset
```

Available presets: `Default`, `Catppuccin`, `Pastel`, `Pfetch`.

#### New & Improved Modules

| Module | What's new |
|---|---|
| `gpu` | Unified dGPU + iGPU detection, lists all GPUs with index |
| `shell` | Now includes shell version via `{version}` token |
| `terminal` | Now includes terminal version via `{version}` token |
| `wm` | Now includes WM version via `{version}` token |
| `battery` | More detailed status: `AC Connected, Charging` / `AC Connected, Full` |
| `disk` | Now exposes `{fs}` token for filesystem type |
| `memory` | Now exposes `{swap_used}` and `{swap_total}` tokens |
| `break` | New module — inserts a blank line |
| `custom` | Inline custom text modules via `{ type = "custom", key = "Editor", text = "neovim" }` |

#### Custom Key Separators
You can now change the separator between key and value globally or per-module:

```lua
display = {
  separator = { key = " ❯", color = "#6c7086" }
}
```

#### Benchmarking
```bash
setch --benchmark
setch -b
```
Prints execution time at the bottom of output.

---

### 🔧 Internal Changes

- Entire rendering pipeline rewritten around a `Module` enum + `Overrides` struct — much cleaner and easier to extend
- `SetchCache` struct centralises all collected system data
- `ConfigCache` struct pre-parses all config values once at startup
- System info collection is now done via `setchd::collect_once()` which is shared between daemon and runtime modes
- ANSI stripping is now handled properly for logo width calculation
- Logo and info rendering now center-aligns both sides relative to the longer one
- `LazyLock` used for `HOME` to avoid repeated env lookups

---

## [0.1.4] - prior release

- JSONC-based config
- Basic neofetch-style and cpufetch-style modes
- Threaded collection for packages, GPU, battery
- ASCII logos for Arch, Fedora, Ubuntu, Debian
- AMD/Intel CPU logo in cpu mode
