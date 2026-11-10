# hyprdesk
*[Version française](README.md)*

**An on-demand desktop for Hyprland.** With tiling, windows cover the
screen and the desktop no longer exists. hyprdesk makes it visible with a single key,
with a wallpaper, icons and widgets. Each workspace
can have its own desktop. Configuration is written in Lua, in the style of
`hyprland.lua` (Hyprland ≥ 0.57).

hyprdesk is the **root** of the graphical session.
- **Visual root.** The desktop always sits *below* every window,
  never on top.
  - At rest, it draws the wallpaper of the displayed workspace, and replaces
    hyprpaper.
  - When called up (SUPER + D), it sets the workspace's windows aside
    (`special:hyprdesk`) and appears with its icons and widgets. Whatever
    you open from it opens **on top** of it, floating and centered.
  - When hidden, it gives the set-aside windows back.
  - The `glass` mode is an effect on the background: a blurred, tinted wallpaper.
- **Session root.** hyprdesk launches and supervises the components declared
  with `hd.component` (e.g. `wayle shell`). It restarts them if they crash and
  stops them along with itself.
- **Native bar.** `hd.bar` displays a bar above the windows, with
  its reserved area. Windows and the desktop grid fit alongside it.
  - Its modules are widgets: workspaces, active window, volume,
    desktop button, and any widget or plugin in compact form.
  - It can also be configured in Settings → Bar.
- **Dock** at the bottom center: the open windows, if the option is enabled
  (a click focuses the window), followed by the ⚙ settings button.

## Settings (⚙)

Each Settings entry is a **built-in package** (manifest, messages,
state, view, CLI keys); design of packages, widgets and the launcher:
[`docs/PLUGINS.en.md`](docs/PLUGINS.en.md).

The dock's ⚙ button, or `hyprdesk settings [desktop|icons|wallpaper|effects]`, opens
a panel over the desktop, on the active screen. It groups the following settings:

| Tab | Settings |
|---|---|
| Desktop | desktop on all screens, the active screen or the primary screen; close after a launch; open windows floating; opaque or glass mode, by default and for this workspace; veil opacity; shortcuts (suspension, keys, conflicts, universal shortcuts); session (components, state, restart) |
| Icons | shortcuts to installed applications (everywhere or on this workspace); icon and cell size; icon theme; reset positions |
| Launcher | desktop dock: windows shown (none, this workspace, all); grouping (none, by application, by workspace) |
| Wallpaper | scope (all desktops, this workspace, this screen); image picked from a folder (thumbnails), slideshow (interval, shuffle) or solid color; fit |
| Bar | show, position, height, screens (all, primary, a single one); modules per zone (left, center, right): add, reorder, change zone, remove |
| Widgets | available widgets (built-in and detected plugins), enabled one by one; placement of desktop widgets: 3×3 anchoring, offsets, size, card; drag and drop on the desktop |
| Screens | detected screens (including disabled ones) on a to-scale map; position relative to a neighbor and alignment; orientation and mirroring; resolution and refresh rate; scale; enabling; primary screen; automatic layout — each change applies live, then is kept (written to `monitors.lua`) or reverts on its own after 12 s |
| Workspaces | number of persistent workspaces (or left to Hyprland); assigning an application to a workspace, with or without switching to it |
| Effects | transition (fade, slide, none) and its duration; crossfade between two wallpapers; accent color |

Everything applies **live**. The panel is a drawer on the right, and the desktop
stays visible and usable beside it.

## Plug-n-play widgets

The built-in widgets are `clock`, `calendar`, `system` and `media`, plus
`command` and `lua`, which are declared in Lua. To add another one, just
drop a Lua file into `~/.config/hypr/hyprdesk/widgets/`, without
recompiling anything:
- the file is detected immediately;
- it appears in Settings → Widgets, where you enable it with a click;
- you can also use it from Lua with `hd.widget("<name>", { … })`.

```lua
-- ~/.config/hypr/hyprdesk/widgets/meteo.lua
return {
    name = "Weather", description = "wttr.in", interval = 900, anchor = "top-left",
    options = { city = "Paris" },                        -- default values
    render = function(o)
        return hd.exec("curl -s 'wttr.in/" .. o.city .. "?format=3'")
    end,
}
```

A plugin can do more than text. This is what lets you add to the
bar anything that is missing, in the usual way:

```lua
return {
    name = "VPN", description = "VPN status", interval = 5, anchor = "top-right",
    options = { interface = "wg0" },
    -- text, or a table: icon (preferably “…-symbolic”, tinted to match the theme),
    -- tooltip, state (normal, active, warning, critical, hidden)
    render = function(o)
        local up = hd.exec("ip link show " .. o.interface .. " 2>/dev/null") ~= ""
        return { icon = up and "network-vpn-symbolic" or "network-vpn-disabled-symbolic",
                 tooltip = up and "VPN up" or "VPN down", state = up and "active" or "normal" }
    end,
    on_click  = function(o) end,              -- click (right-click if a menu exists)
    on_scroll = function(o, dir) end,         -- scroll wheel: +1 / -1
    menu = function(o)                        -- drop-down menu below the module
        return {
            { header = "VPN" },
            { label = "Connect", icon = "network-vpn-symbolic", action = function() hd.spawn("wg-quick up wg0") end },
            "-",
            { label = "Disconnect all…", confirm = "Disconnect the VPN?", action = function() hd.exec("wg-quick down wg0") end },
        }
    end,
}
```

- **`hd.exec(cmd)`** returns the output of a command.
- **`hd.spawn(cmd)`** launches a program without waiting, through Hyprland.
- **Menu entries**: `label`, `detail`, `icon`, `checked`, `disabled`,
  `action` and `confirm`. An entry with `confirm` asks for confirmation
  within the menu itself.

**Bundled plugins.** They are built into the binary, and a file with the same name
replaces them:

| Plugin | Role |
|---|---|
| `reseau` | wired and Wi-Fi, nearby networks, connecting, toggling Wi-Fi |
| `bluetooth` | toggling, paired devices (connect, disconnect) |
| `batterie` | battery of the computer and of peripherals (mouse, keyboard); hidden if there is none |
| `notifications` | Do Not Disturb, latest notifications, clear all (wayle service) |
| `session` | lock, suspend, log out, reboot, power off (with confirmation) |

**Bar-specific modules.**
- `workspaces`: dots; click to go there, scroll to move from one
  workspace to the next.
- `window`: title of the active window.
- `volume`: menu with a slider and mute; middle-click to mute,
  scroll to adjust.
- `tray`: system tray. Click to activate the application, right-click
  for its native menu (dbusmenu).
- `desktop`: button that shows or hides the empty desktop.

The clock opens a calendar. `hyprdesk menu <module>` opens a menu from the
keyboard, for example `hyprdesk menu volume` or `hyprdesk menu reseau`.

Plugin locations:
- a `<name>.lua` file, or a `<name>/init.lua` folder;
- also searched in `~/.local/share/hyprdesk/widgets` and
  `/usr/share/hyprdesk/widgets`; the first folder found wins;
- `hyprdesk init` installs two examples: `meteo` and `notes`.

A failing plugin is reported in Settings without blocking the rest.

In Settings → Widgets, each switch reflects the actual state of the desktop,
whether the widget comes from Settings or from `hyprdesk.lua`. Turning off a widget from
`hyprdesk.lua` writes `hd.disable("clock#0")` to `settings.lua`, without
modifying your file. Turning it back on removes that line.

To reposition a widget declared in `hyprdesk.lua`, use
`hd.place("clock#0", { anchor = "top", y = 20 })`. When you move it from
Settings, it is this `hd.place` that gets written to `settings.lua`.

Settings are saved as generated Lua, in
`~/.config/hypr/hyprdesk/settings.lua`, just as hyprdmc does for
`monitors.lua`. This file is loaded **before** `hyprdesk.lua`: anything you
write by hand has the last word. Your file is never rewritten, and the
panel flags the settings that `hyprdesk.lua` overrides.

## Installation

You need Rust 1.99+ (2024 edition), Hyprland 0.57+ and a Vulkan or GL driver.

```bash
./scripts/install.sh
```

The script:
- builds `hyprdesk` and places it in `~/.local/bin`;
- writes `~/.config/hypr/hyprdesk.lua` if it does not exist, along with the
  LuaLS definitions;
- enables the systemd user service.

It does not modify `hyprland.lua`. The recommended lines are available with
`hyprdesk snippet`: the `submap_universal` marking for shortcuts to keep
on the desktop (see below), and the blur behind the bar.

To make hyprdesk the root of the session:
- remove from Hyprland's autostart what it now supervises
  (`hd.component`);
- remove `hyprpaper`;
- hide wayle's bar if you enable hyprdesk's.

## Shortcuts

- **hyprdesk's shortcuts are installed by hyprdesk itself**, using
  those from `hd.config({ keys = … })` or from Settings → Desktop:
  - `toggle`, `SUPER + D` by default, shows or hides the desktop;
  - `settings`, `SUPER + SHIFT + D` by default, opens Settings (including while the desktop
    is shown, without changing mode);
  - `lock` locks the session (`hyprdesk lock`). Not set by default:
    it is the `SUPER + L` from `hyprland.lua`, added by `scripts/install.sh`.
    When set in hyprdesk, it replaces it (the combination is released);
  - `leave`, `SUPER + Escape` by default, is the emergency exit.

  They are reinstalled after every `hyprctl reload`.
- **No conflict with Hyprland.** Before installing a shortcut, hyprdesk
  checks that it is not already bound. If it is, it does not install it outside the
  desktop (the one from `hyprland.lua` stays in charge) and reports it with a
  notification and in Settings; while the desktop is shown, since Hyprland's
  shortcuts are suspended, hyprdesk installs it anyway.
- **While the desktop is shown, Hyprland's shortcuts are suspended.** hyprdesk
  activates the `hyprdesk` submap: SUPER + Q, SUPER + F, etc. no longer target
  windows. As soon as a window is opened from the desktop, they are
  given back so they act on it. During the suspension, the following remain active:
  - hyprdesk's shortcuts;
  - those you mark `{ submap_universal = true }` in `hyprland.lua`,
    typically workspace switching, and the desktop then follows.

  `suspend_binds = false` disables this behavior.
- **You never get stuck.** The submap's shortcuts hand control back
  to Hyprland *before* calling hyprdesk. On startup, hyprdesk closes a
  submap left open by an abrupt shutdown.

Remember to also remove `hyprpaper` from autostart.

## A single instance

Only one daemon runs per session: it holds an exclusive lock (`flock`) on
`$XDG_RUNTIME_DIR/hypr/<instance>/hyprdesk.lock` for its whole lifetime.
- **Second instance**: whether it comes from the service, an `hl.exec_cmd` or a
  manual launch, it reports the running one (its PID) and exits without
  error. systemd therefore does not restart it in a loop.
- **Replacement**: `hyprdesk daemon --replace` cleanly replaces the
  running instance. This is how the systemd service starts.
- **Clean shutdown**: on `SIGTERM` (`systemctl --user stop`) or `SIGINT`,
  hyprdesk gives back the set-aside windows and removes its shortcuts.
- **Abrupt shutdown**: the kernel releases the lock; on the next startup,
  hyprdesk gives back the windows left aside.

## Commands

| Command | Effect |
|---|---|
| `hyprdesk` / `hyprdesk daemon` | starts the daemon (a single instance per session) |
| `hyprdesk daemon --replace` | cleanly replaces the running instance |
| `hyprdesk show` (`full-desktop`) · `launcher-only` · `hide` (`no-desktop`) | the three desktop modes: full (windows set aside), launcher only (dock reachable, tiled windows make room for it), hidden |
| `hyprdesk toggle` · `toggle-launcher` | full-desktop / launcher-only, or no-desktop if already there (shortcuts SUPER + D, SUPER + ALT + D; SUPER + CTRL + D: no-desktop) |
| `hyprdesk cycle` | next mode: no-desktop → launcher-only → full-desktop → no-desktop (Desktop button on the bar; right-click: no-desktop) |
| `hyprdesk settings [package]` | opens Settings without changing the desktop mode (`desktop`, `launcher`, `screens`, `wallpaper`…) |
| `hyprdesk theme [list]` · `theme new <image\|pack.zip\|folder> [--name NAME]` · `theme apply <name>` | “My themes”; a theme derived from a scene (colors from the image, scene spread across all screens, desktop and lock screen) |
| `hyprdesk wallpaper split <image> [folder] [--scale N]` | cuts a scene according to the screen layout: one image per screen |
| `hyprdesk export [file] [--theme] [--screens]` · `hyprdesk import <archive> [--theme] [--dry-run] [--screens]` | share your setup: configs, plugins, wallpapers — or, with `--theme`, the theme alone (colors, wallpapers, effects); import backs up the current one |
| `hyprdesk open <role>` · `hyprdesk context` | default application for a role (`browser`, `terminal`, `ai`…); environment context for an AI assistant |
| `hyprdesk config [list [package]] \| get \| set \| unset` | settings from the command line, per package, written to `settings.lua` just as from the Settings screen (e.g. `hyprdesk config set launcher_group app`) |
| `hyprdesk reload` | rereads the configuration (it is also reread on every save) |
| `hyprdesk status` | state: shown or hidden, workspaces, whether the config has errors |
| `hyprdesk check [file]` | checks a configuration without touching the daemon |
| `hyprdesk init` · `stubs` · `snippet` | example config, LuaLS definitions, lines for `hyprland.lua` |

Options and variables:
- `--config <file>` or `$HYPRDESK_CONFIG`: configuration file to use;
- `$HYPRDESK_LOG`: log level (`debug`, `info`…).

## Screens — `~/.config/hypr/monitors.lua`

hyprdesk manages screens (it replaces hyprdmc): the **Screens** tab detects
outputs, positions them, rotates them, sets mode and scale, and designates the primary
screen. A change applies live; **Keep** writes it to
`monitors.lua` (required by `hyprland.lua`: `require("monitors")`), otherwise
the previous display comes back on its own. Unplugged screens keep their line, and
an unknown screen is welcomed in its preferred mode, placed automatically. A
`monitors.lua` written by another tool is preserved once as
`monitors.lua.bak`.

## Configuration — `~/.config/hypr/hyprdesk.lua`

```lua
hd.config({ mode = "opaque", show_on = "all", running_apps = "workspace", icon_theme = "Papirus-Dark",
            transition = "slide", fade_ms = 150, crossfade_ms = 700,
            grid = { cell = 96, gap = 16, margin = 32, icon_size = 48 },
            theme = { fg = "#f8f8f2", bg = "#22212c", accent = "#9580ff" } })

hd.wallpaper({ path = "~/.wallpapers/default.jpg" })                       -- everywhere
hd.wallpaper({ monitor = "DP-1", path = "~/.wallpapers/portrait.png" })    -- one screen
hd.wallpaper({ workspace = 3, rotate = { dir = "~/.wallpapers", interval = 600, shuffle = true } })
hd.desktop({ workspace = 4, mode = "glass" })
hd.desktop({ workspace = "special:magic", inherit = false })               -- without the global elements

hd.config({ primary = "HDMI-A-2", show_on = "primary" })                  -- primary screen
hd.bar({ monitor = "primary" })                                            -- bar on the primary screen
-- Narrow or vertical screens: compact bar (auto), and modules specific to vertical screens.
hd.bar({ compact = "auto", left = { "workspaces", "window" }, right = { "tray", "volume", "session" },
         portrait = { left = { "workspaces" }, right = { "volume", "session" } } })

hd.config({ workspaces = 7 })                                              -- 1 to 7 always present
hd.config({ workspace_monitors = { ["HDMI-A-2"] = { 1, 2, 3, 4, 5 }, primary = { 6, 7 } } }) -- screen for each workspace
hd.assign({ app = "firefox", workspace = 2 })                              -- opens on 2, and takes you there
hd.assign({ class = "^(org\\.telegram\\..*)$", workspace = 5, silent = true }) -- without going there

hd.icon({ app = "firefox" })                                               -- .desktop identifier
hd.icon({ workspace = 2, label = "Projects", path = "~/Documents/DEV", pos = { 0, 1 } })
hd.icon({ label = "htop", exec = "htop", terminal = true, icon = "utilities-system-monitor" })

hd.widget("clock",    { anchor = "top-right", format = "%H:%M", size = 72, card = false })
hd.widget("calendar", { anchor = "right" })
hd.widget("system",   { anchor = "bottom-right", show = { "cpu", "ram", "disk:/", "uptime" } })
hd.widget("media",    { anchor = "bottom", player = "*" })
hd.widget("command",  { exec = "curl -s 'wttr.in/?format=3'", interval = 900 })
hd.widget("lua",      { interval = 5, render = function() return os.date("%A") end })

hd.on("show", function(e) print(e.monitor, e.workspace) end)               -- show, hide, workspace
```

**Selectors.** Every rule accepts `workspace` and `monitor` (a string, a
number, a list, or `"*"`). Rules combine as follows:
- for the wallpaper and the mode, the most specific rule wins
  (workspace + screen > workspace > screen > all); on a tie, the last one
  declared wins;
- icons and widgets add up.

**Errors.** Hyprdesk reacts to an error the way Hyprland does:
- an unknown field, an invalid value or a Lua error produce a
  Hyprland notification that shows `file:line`;
- the last valid configuration stays active.

**Modules.** `require("x")` loads `~/.config/hypr/hyprdesk/x.lua`. These modules
are watched too.

**Layout.** Icons are placed column by column. An icon moved with
the mouse keeps its place, per screen and per workspace, in
`~/.local/state/hyprdesk/layout.toml`. The Lua is never rewritten.

**Keyboard.** The arrow keys select, Enter launches, Escape closes the Settings
panel, then the desktop. Hyprland shortcuts marked `submap_universal` remain active: switching workspaces with the desktop open shows the desktop of
that workspace.

**Launching.** Programs are launched by Hyprland
(`hl.dsp.exec_cmd`): they belong to it, and its exec rules apply.

## Development

```bash
cargo test --locked                 # Lua config, resolution, placement, IPC, .desktop
cargo run --locked -- --config example.lua   # daemon with a test config
```

`Cargo.lock` pins `winit-core` to `0.31.0-beta.2`, because beta.4 breaks
`iced_exdevtools`, a dependency of `iced_layershell` 0.19. You must therefore
build with `--locked`.

The Hyprland IPC comes from `hyprpanel-core`
([mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools)),
pinned to a commit.

Code under the MIT license; Mindlog font under [SIL OFL 1.1](assets/fonts/mindlog/OFL.txt).

## Polish and typography

Mindlog 0.7 is the default font on the desktop, the bar, the menus and
Settings. Its five weights (Light, Regular, Medium, SemiBold and Bold) are
embedded in the binary; `scripts/install.sh` also installs them in
`${XDG_DATA_HOME:-~/.local/share}/fonts/hyprdesk/Mindlog`, along with their OFL license.
Settings → Theme → Text offers Mindlog and the font families installed
on the machine, a preview and the text size. The choice is saved in
`settings.lua`; `hd.config({ font = "Mindlog", font_size = 14 })` takes
precedence if it is declared in the user file.

**Language.** The interface is available in French and English: Settings →
Desktop → Language, `hyprdesk config set language fr|en|auto`, or
`hd.config({ language = "en" })`. `auto` follows the system language (`LANG`);
an explicit language also applies to dates. Lua plugins translate with
`hd.t("text")` (details: [`docs/PLUGINS.en.md`](docs/PLUGINS.en.md), § 9).

**Layouts.** Settings → Layouts sets how Hyprland tiles windows: default and
per-workspace layout (Dwindle, Master, Scrolling, Monocle), their options, and
four custom layouts written in Lua (grid, columns, rows, centered master).
Everything applies live; only the settings made in hyprdesk override
`hyprland.lua`. In Lua:
`hd.config({ layout = "master", layout_workspaces = { [3] = "monocle" }, master = { mfact = 0.6 } })`;
from the command line: `hyprdesk config set layout grid`, `layout.3 monocle`,
`master.orientation center`.

Cards and the dock share a smoked glass, subtle outlines and an
iris accent. Application icons come from the system theme;
commands have built-in SVG fallbacks. `transition = "none"` disables
the new animations. Explicitly chosen wallpapers and colors
keep precedence. See [DESIGN.md](DESIGN.md) for the shared rules.

The system tray can provide its own `StatusNotifierWatcher` when
no other service provides one. It coexists with an existing watcher.
The network, Bluetooth and notifications widgets use
`nmcli`, `bluetoothctl` and `wayle` respectively. Volume uses `wpctl`, the font list
`fc-list`, and session actions use `hyprlock` and `systemctl`.
A missing or overly slow command is reported by the widget.

## Reliability and execution limits

Window moves and submap changes are processed
in order by a dedicated actor. Restoration is verified before a journal entry
is forgotten; an IPC error keeps the pending window restorations.
State files are written atomically, and an older save
cannot overwrite a more recent change.

A widget runs only one poll at a time. Results from previous
configurations are discarded. `hd.exec` limits duration to five seconds and
output to 64 KiB per stream; the `command` widget keeps its `timeout` option.
Commands that exceed their limits are killed along with their process
group. Lua calls have a five-second budget, also
applied to coroutines.

Lua remains a trusted local environment, with access to the standard
libraries: these limits are not a sandbox for untrusted code.
`check` runs the Lua and loads the plugins; it is not merely
a syntax check.

## Autonomous verification loop

Rust 1.99 or later is required. `Cargo.lock` must keep being used.

```bash
bash scripts/verify.sh            # format, strict Clippy, tests, build, isolated D-Bus
bash scripts/verify.sh --visual   # adds screenshots at 1440, 900 and 640 px
cargo run --locked -- preview     # interactive preview without modifying the session
```

The preview uses the real views with demo data. It does not
launch applications and does not write settings. Screenshots are placed
by default in `/tmp/hyprdesk-visual` and let you review all nine views.
The preview's controls are limited to navigation and typography.
Compositor-provided blur and multi-screen interactions also
need to be checked in a Hyprland session.

### Hyprshot and screenshot tools

Screenshot shortcuts must remain active in the desktop submap.
Add `submap_universal = true` to the existing declarations, for example:

```lua
hl.bind("Print", hl.dsp.exec_cmd("hyprshot -m region --clipboard-only"), { submap_universal = true })
hl.bind("SUPER + Print", hl.dsp.exec_cmd("hyprshot -m output -m active"), { submap_universal = true })
```

The same option applies to `SHIFT + Print`, `ALT + Print` and to a custom
shortcut such as `SUPER + SHIFT + S`. The region and output modes capture the
desktop and layer-shell surfaces. Hyprshot's window mode targets an
application window; the desktop and the bar are not XDG windows.
