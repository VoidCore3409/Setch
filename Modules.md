# Module Reference

Every module in setch can be specified as either a plain string or a Lua table with overrides.

```lua
-- Plain string
"cpu"

-- Table with overrides
{ type = "cpu", key = "Processor", fmt = "{cpu} @ {freq}", key_color = "#f38ba8" }
```

---

## Global Overrides

These override keys are available on **every** module that supports table syntax:

| Key | Type | Description |
|---|---|---|
| `type` | string | The module type (required when using table syntax) |
| `key` | string | Override the label shown on the left |
| `key_color` | string | Hex color for the key label |
| `key_sep` | string | Override the separator between key and value (e.g. `" ❯"`) |
| `key_sep_color` | string | Hex color for the key separator |
| `fmt` | string | Format string with `{tokens}` for the value side |

---

## Modules

### `name`
Renders `user@hostname`.

| Token | Description |
|---|---|
| `{user}` | Current username |
| `{host}` | Hostname |

Additional overrides:

| Key | Description |
|---|---|
| `username_color` | Hex color for the username part |
| `hostname_color` | Hex color for the hostname part |

Default fmt: `{user}@{host}`

---

### `os`
| Token | Description |
|---|---|
| `{os}` | OS name + architecture |

Default fmt: `{os}`

---

### `host`
| Token | Description |
|---|---|
| `{name}` | Product/machine name (from DMI) |

Default fmt: `{name}`

---

### `kernel`
| Token | Description |
|---|---|
| `{sysname}` | Kernel type (e.g. `Linux`) |
| `{release}` | Kernel release string |
| `{arch}` | CPU architecture |

Default fmt: `{sysname} {release}`

---

### `uptime`
| Token | Description |
|---|---|
| `{days}` | Days of uptime |
| `{hours}` | Hours (remainder after days) |
| `{minutes}` | Minutes (remainder after hours) |
| `{seconds}` | Seconds (remainder after minutes) |

Default fmt: `{days}d {hours}h {minutes}m`

---

### `packages`
| Token | Description |
|---|---|
| `{packages}` | Package count |
| `{manager}` | Package manager name |

Default fmt: `{packages} ({manager})`

---

### `shell`
| Token | Description |
|---|---|
| `{name}` | Shell binary name (e.g. `zsh`) |
| `{version}` | Shell version string |

Default fmt: `{name} {version}`

---

### `terminal`
| Token | Description |
|---|---|
| `{term}` | Terminal name |
| `{version}` | Terminal version string |

Default fmt: `{term} {version}`

---

### `wm`
| Token | Description |
|---|---|
| `{name}` | WM or desktop environment name |
| `{version}` | WM version string |

Default fmt: `{name} {version}`

---

### `resolution`
Renders as `Display` by default. Shows monitor info sourced from DRM/EDID.

| Token | Description |
|---|---|
| `{name}` | Display name (e.g. `eDP-1`) |
| `{resolution}` | Resolution string (e.g. `1920x1080`) |
| `{size}` | Physical size in inches (e.g. `15"`) |
| `{refresh}` | Refresh rate (e.g. `120 Hz`) |
| `{builtin}` | `[Built-in]` if internal display, empty otherwise |

Default fmt: `{name} {resolution} {size}{refresh} {builtin}`

Default key: `Display`

---

### `cpu`
| Token | Description |
|---|---|
| `{cpu}` | CPU brand string |
| `{vendor}` | CPU vendor (e.g. `AuthenticAMD`) |
| `{cores}` | Physical core count |
| `{threads}` | Logical thread count |
| `{freq}` | Max frequency in GHz |
| `{arch}` | CPU architecture |

Default fmt: `{cpu} ({cores}C/{threads}T)`

---

### `gpu`
Lists all detected GPUs. Each GPU is rendered on its own line with an index number.

| Override | Description |
|---|---|
| `index` | `true` (default) shows `GPU 1`, `GPU 2`, etc. Set to `false` to hide index |

---

### `memory`
| Token | Description |
|---|---|
| `{used}` | RAM used in GiB |
| `{total}` | RAM total in GiB |
| `{percent}` | RAM usage percentage |
| `{swap_used}` | Swap used in GiB |
| `{swap_total}` | Swap total in GiB |

Default fmt: `{used}GiB / {total}GiB ({percent}%)`

---

### `disk`
Reports usage for `/`.

| Token | Description |
|---|---|
| `{used}` | Space used in GiB |
| `{total}` | Total space in GiB |
| `{percent}` | Usage percentage |
| `{fs}` | Filesystem type (e.g. `ext4`, `btrfs`) |

Default fmt: `{used}GiB / {total}GiB ({percent}%)`

---

### `battery`
| Token | Description |
|---|---|
| `{capacity}` | Battery percentage |
| `{status}` | Charge status — one of: `AC Connected, Charging` / `AC Connected, Full` / `Discharging` / `Empty` / `Unknown` |

Default fmt: `{capacity}% [{status}]`

---

### `ip`
| Token | Description |
|---|---|
| `{ip}` | Local IP address |
| `{iface}` | Network interface name |

Default fmt: `{ip} ({iface})`

---

### `locale`
| Token | Description |
|---|---|
| `{lang}` | System locale string (e.g. `en-US`) |

Default fmt: `{lang}`

---

### `separator`
Renders the separator line defined in your config (`separator = "━━━━..."`). No format tokens.

---

### `break`
Inserts a blank line. No format tokens, no overrides.

---

### `colors`
Renders the 8 standard terminal color swatches using the `colors_format` string from your config.

| Token | Description |
|---|---|
| `{colors}` | The rendered color swatches |

Default fmt: `{colors}`

---

### `custom`
A static key-value pair defined entirely in your config.

```lua
{ type = "custom", key = "Editor", text = "neovim" }
```

| Key | Description |
|---|---|
| `key` | The label |
| `text` | The value to display |

---

### `command`
Runs a shell command and displays its stdout as the value.

```lua
{ type = "command", key = "Public IP", text = "curl -s ifconfig.me" }
{ type = "command", key = "Song",      text = "playerctl metadata --format '{{ artist }} - {{ title }}'" }
```

| Key | Description |
|---|---|
| `key` | The label |
| `text` | The shell command to run |

> Note: commands run synchronously. Keep them fast to avoid slowing down setch.
