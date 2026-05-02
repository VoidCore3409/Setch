pub const PFETCH: &str = r##"return {
    main_color = "#b4befe",
    subtext = "#cdd6f4",

    custom = {},
    ascii = {
        enabled = true,
        color = "#b4befe",
        ["type"] = "small"
    },

    display = {
        separator = {
            color = "#f5c2e7",
            key = ""
        }
    },

    modules = {
        {
            type = "name",
            username_color = "#b4befe",
            hostname_color = "#89dceb"
        },
        {
            type = "os",
            key = "os    "
        },
        {
            type = "host",
            key = "host  "
        },
        {
            type = "kernel",
            key = "kernel"
        },
        {
            type = "uptime"
        },
        {
            type = "packages",
            key = "pkgs  "
        },
        {
            type = "memory",
            key = "memory"
        },
        "break"
    }
}"##;

pub const DEFAULT: &str = r##"return {
    main_color = "#b4befe",
    name_color = "#b4befe",
    subtext = "#cdd6f4",
    separator_color = "#7a7a7a",
    separator = "----------------",
    colors_format = "● ",

    custom = {},
    ascii = {
        enabled = true,
        color = "#b4befe",
        ["type"] = "big" -- 'type' is a reserved keyword in some contexts, so we use this syntax!
    },

    display = {
        separator = {
            color = "#b4befe",
            key = " ❯"
        }
    },

    modules = {
        "name",
        "separator",
        "os",
        "host",
        "kernel",
        "uptime",
        "packages",
        "shell",
        "resolution",
        "wm",
        "theme",
        "icons",
        "terminal",
        "cpu",
        "gpu",
        "memory",
        "disk",
        "ip",
        "locale",
        "battery",
        "separator",
        "colors",
        "break"
    }
}"##;
pub const CATPPUCCIN: &str = r##"return {
    main_color = "#b4befe",
    subtext = "#cdd6f4",
    separator_color = "#45475a",
    separator = "━━━━━━━━━━━━━━━━━━━━━━━━",
    colors_format = "󱕍 ",

    custom = {},
    ascii = {
        enabled = true,
        color = "#b4befe",
        ["type"] = "big",
    },

    display = {
        separator = {
            color = "#f5c2e7",
            key = " ❯"
        }
    },

    modules = {
        {
            type = "name",
            username_color = "#b4befe",
            hostname_color = "#89dceb"
        },

        "separator",

        {
            type = "os",
            key = "󰣇 SYS",
            key_color = "#89dceb"
        },
        {
            type = "kernel",
            key = "󰒋 KRN",
            key_color = "#74c7ec"
        },
        {
            type = "uptime",
            key = "󱑂 UP",
            key_color = "#94e2d5"
        },
        {
            type = "packages",
            key = " PKG",
            key_color = "#cba6f7"
        },

        "separator",

        {
            type = "cpu",
            key = "󰻠 CPU",
            key_color = "#fab387"
        },
        {
            type = "gpu",
            key = " GPU",
            key_color = "#f38ba8"
        },
        {
            type = "memory",
            key = " RAM",
            key_color = "#a6e3a1"
        },
        {
            type = "battery",
            key = "󱊣 BAT",
            key_color = "#f9e2af",
            show_charging_status = true
        },

        "separator",

        {
            type = "wm",
            key = " WM",
            key_color = "#89b4fa"
        },
        {
            type = "shell",
            key = " SH",
            key_color = "#cba6f7"
        },
        {
            type = "terminal",
            key = " TTY",
            key_color = "#94e2d5"
        },
        {
            type = "resolution",
            key = "󰍹 RES",
            key_color = "#b4befe"
        },
        "separator",
        "colors",
        "break",
    }
}"##;

pub const PASTEL: &str = r##"return {
    main_color = "#f5c2e7",
    subtext = "#bac2de",
    separator_color = "#ccd0da",
    separator = "····················",
    colors_format = "○ ",

    custom = {},
    ascii = {
        enabled = true,
        color = "#f5c2e7",
        ["type"] = "big"
    },

    display = {
        separator = {
            color = "#94e2d5",
            key = " →"
        }
    },

    modules = {
        {
            type = "name",
            username_color = "#f2cdcd",
            hostname_color = "#89dceb"
        },

        "separator",

        {
            type = "os",
            key = "system",
            key_color = "#f9e2af"
        },
        {
            type = "host",
            key = "machine",
            key_color = "#fab387"
        },
        {
            type = "kernel",
            key = "kernel",
            key_color = "#89b4fa"
        },
        {
            type = "uptime",
            key = "uptime",
            key_color = "#94e2d5"
        },
        {
            type = "packages",
            key = "packages",
            key_color = "#cba6f7"
        },

        "separator",

        {
            type = "shell",
            key_color = "#f38ba8"
        },
        {
            type = "terminal",
            key_color = "#b4befe"
        },
        {
            type = "wm",
            key_color = "#89dceb"
        },
        {
            type = "theme",
            key_color = "#f2cdcd"
        },
        {
            type = "resolution",
            key_color = "#a6e3a1"
        },

        "separator",

        {
            type = "cpu",
            key_color = "#fab387"
        },
        {
            type = "gpu",
            key_color = "#f38ba8"
        },
        {
            type = "memory",
            key_color = "#a6e3a1"
        },
        {
            type = "disk",
            key_color = "#94e2d5"
        },
        {
            type = "battery",
            key_color = "#f9e2af"
        },

        "separator",
        "colors",
        "break"
    }
}"##;
