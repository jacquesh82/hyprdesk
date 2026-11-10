# hyprdesk

*[Version française](README.fr.md)*

![License](https://img.shields.io/badge/license-MIT-blue)
![Hyprland](https://img.shields.io/badge/Hyprland-%E2%89%A5%200.57-58e1ff)
![Rust](https://img.shields.io/badge/Rust-%E2%89%A5%201.99-dea584)

**An on-demand desktop for [Hyprland](https://hyprland.org).** With tiling,
windows cover the screen and the desktop no longer exists. hyprdesk makes it
visible with a single key — wallpaper, icons and widgets — and each workspace
can have its own desktop. Configuration is written in Lua, in the style of
[`hyprland.lua`](https://github.com/hyprwm/Hyprland) (Hyprland ≥ 0.57).

<p align="center">
  <img src="assets/screenshots/desktop.png" width="880" alt="hyprdesk desktop: bar on top, icon column, clock, calendar and system widgets, dock at the bottom">
</p>

*Screenshots come from hyprdesk's isolated preview (`cargo run --locked --
preview`), rendered with demo data.*

hyprdesk is the **root** of the graphical session:

- **Visual root.** The desktop always sits *below* every window, never on top.
  At rest it draws the wallpaper of the displayed workspace, replacing
  [hyprpaper](https://github.com/hyprwm/hyprpaper). Called up with `SUPER + D`,
  it sets the workspace's windows aside (`special:hyprdesk`) and appears with
  its icons and widgets; whatever you open from it opens **on top** of it,
  floating and centered. Hidden, it gives the windows back. The `glass` mode
  blurs and tints the wallpaper behind the desktop.
- **Session root.** hyprdesk launches and supervises the components declared
  with `hd.component`; it restarts them if they crash and stops them along
  with itself.
- **Native bar.** `hd.bar` displays a bar above the windows, with its reserved
  area: windows and the desktop grid make room for it. Modules are widgets —
  workspaces, active window, volume, tray, desktop button, and any plugin in
  compact form.

<p align="center">
  <img src="assets/screenshots/bar.png" width="760" alt="Settings drawer open on the Bar tab, next to the desktop and its bar">
</p>

## Widgets, icons, dock

The built-in widgets are `clock`, `calendar`, `system` and `media`, plus
`command` and `lua` declared in Lua. In Settings → Widgets each switch reflects
the actual state of the desktop, and widgets can be dragged straight onto it.

<p align="center">
  <img src="assets/screenshots/widgets.png" width="760" alt="Settings drawer on the Widgets tab: clock, calendar and system widgets listed, drag-and-drop tip">
</p>

Shortcuts to installed applications sit in an icon column on the desktop —
everywhere or on one workspace only — with adjustable icon and cell size, and
the system icon theme.

<p align="center">
  <img src="assets/screenshots/icons.png" width="760" alt="Settings drawer on the Icons tab: add an application, icon size, cell size, icon theme">
</p>

A **dock** at the bottom center shows the open windows (none, this workspace,
or all — grouped by application or workspace), then the ⚙ settings button.

<p align="center">
  <img src="assets/screenshots/launcher.png" width="760" alt="Settings drawer on the Launcher tab: windows shown in the dock, grouping options">
</p>

## Settings — everything applies live

The ⚙ button, or `hyprdesk settings [desktop|icons|wallpaper|…]`, opens a
drawer on the right; the desktop stays visible and usable beside it.

| Tab | Settings |
|---|---|
| Desktop | desktop on all screens, the active screen or the primary screen; close after a launch; open windows floating; opaque or glass mode; veil opacity; shortcuts; session components |
| Wallpaper | scope (all desktops, this workspace, this screen); image picked from a folder (thumbnails), slideshow or solid color; fit |
| Widgets | available widgets (built-in and detected plugins); placement: 3×3 anchoring, offsets, size, card; drag and drop |
| Theme | bundled themes (Ardoise, Glass, Encre, Forêt, Ambre, Clair), custom themes created from an image, colors, text |
| Effects | transition (fade, slide, none) and duration; crossfade between wallpapers; accent color |
| Screens | detected screens on a to-scale map; position, orientation, mirroring, resolution, refresh rate, scale, primary; each change applies live, then is kept (`monitors.lua`) or reverts after 12 s |
| Workspaces | number of persistent workspaces; assigning an application to a workspace |
| Layouts | Hyprland window layouts: Dwindle, Master, Scrolling, Monocle — default and per workspace, applied live |
| Bar | show, position, height, screens; modules per zone (left, center, right) |

<p align="center">
  <img src="assets/screenshots/theme.png" width="400" alt="Settings drawer on the Theme tab: bundled themes Ardoise, Glass, Encre, Forêt, Ambre, Clair">
  <img src="assets/screenshots/wallpaper.png" width="400" alt="Settings drawer on the Wallpaper tab: scope, image, slideshow or solid color">
</p>
<p align="center">
  <img src="assets/screenshots/effects.png" width="400" alt="Settings drawer on the Effects tab: transition and crossfade durations">
  <img src="assets/screenshots/screens.png" width="400" alt="Screens layout editor: two monitors on a to-scale map, primary star, rulers, one disabled screen">
</p>

The screen layout editor detects outputs — including disabled ones — places
them on a to-scale map, rotates them, sets mode and scale, and designates the
primary screen. A change applies live; **Keep** writes it to `monitors.lua`
(required by `hyprland.lua`: `require("monitors")`), otherwise the previous
display comes back on its own.

<p align="center">
  <img src="assets/screenshots/layouts.png" width="760" alt="Settings drawer on the Layouts tab: Dwindle, Master, Scrolling, Monocle, default and per workspace">
</p>

Settings are saved as generated Lua in `~/.config/hypr/hyprdesk/settings.lua`,
loaded **before** `hyprdesk.lua`: anything you write by hand keeps the last
word, and your file is never rewritten.

## Plug-n-play widgets

To add a widget, drop a Lua file into `~/.config/hypr/hyprdesk/widgets/` —
no recompilation. It is detected immediately, appears in Settings → Widgets,
and can also be used as `hd.widget("<name>", { … })`. A plugin can render
text, an icon with a tooltip and a state, react to clicks and the scroll
wheel, and open a drop-down menu:

```lua
-- ~/.config/hypr/hyprdesk/widgets/vpn.lua
return {
    name = "VPN", description = "VPN status", interval = 5, anchor = "top-right",
    options = { interface = "wg0" },
    render = function(o)
        local up = hd.exec("ip link show " .. o.interface .. " 2>/dev/null") ~= ""
        return { icon = up and "network-vpn-symbolic" or "network-vpn-disabled-symbolic",
                 tooltip = up and "VPN up" or "VPN down", state = up and "active" or "normal" }
    end,
    menu = function(o)
        return {
            { header = "VPN" },
            { label = "Connect", icon = "network-vpn-symbolic",
              action = function() hd.spawn("wg-quick up wg0") end },
            "-",
            { label = "Disconnect…", confirm = "Disconnect the VPN?",
              action = function() hd.exec("wg-quick down wg0") end },
        }
    end,
}
```

Bundled plugins are built into the binary — a file with the same name replaces
them: `reseau` (wired and Wi-Fi), `bluetooth`, `batterie`, `notifications`
(Do Not Disturb), `session` (lock, suspend, log out, reboot, power off). The
bar adds `workspaces`, `window`, `volume`, `tray` and `desktop` modules.
`hyprdesk menu volume` opens any module menu from the keyboard.

## Configuration — `~/.config/hypr/hyprdesk.lua`

```lua
hd.config({ mode = "opaque", show_on = "all", running_apps = "workspace", icon_theme = "Papirus-Dark",
            transition = "slide", fade_ms = 150, crossfade_ms = 700,
            grid = { cell = 96, gap = 16, margin = 32, icon_size = 48 },
            theme = { fg = "#f8f8f2", bg = "#22212c", accent = "#9580ff" } })

hd.wallpaper({ path = "~/.wallpapers/default.jpg" })                       -- everywhere
hd.wallpaper({ workspace = 3, rotate = { dir = "~/.wallpapers", interval = 600, shuffle = true } })
hd.desktop({ workspace = 4, mode = "glass" })

hd.bar({ compact = "auto", left = { "workspaces", "window" }, right = { "tray", "volume", "session" } })

hd.icon({ app = "firefox" })                                               -- .desktop identifier
hd.icon({ workspace = 2, label = "Projects", path = "~/Documents", pos = { 0, 1 } })

hd.widget("clock",    { anchor = "top-right", format = "%H:%M", size = 72, card = false })
hd.widget("calendar", { anchor = "right" })
hd.widget("system",   { anchor = "bottom-right", show = { "cpu", "ram", "disk:/", "uptime" } })
hd.widget("command",  { exec = "curl -s 'wttr.in/?format=3'", interval = 900 })

hd.on("show", function(e) print(e.monitor, e.workspace) end)               -- show, hide, workspace
```

Every rule accepts `workspace` and `monitor` selectors. Errors behave like
Hyprland's: a notification shows `file:line`, and the last valid configuration
stays active. Design of packages, widgets and the launcher:
[`docs/PLUGINS.en.md`](docs/PLUGINS.en.md).

## Installation

You need Rust 1.99+ (2024 edition), Hyprland 0.57+ and a Vulkan or GL driver.

```bash
./scripts/install.sh
```

The script builds hyprdesk into `~/.local/bin`, writes `~/.config/hypr/hyprdesk.lua`
if it does not exist along with the LuaLS definitions, and enables the systemd
user service. It does not modify `hyprland.lua`; the recommended lines are
available through `hyprdesk snippet`.

To make hyprdesk the root of the session: remove from Hyprland's autostart
what it now supervises (`hd.component`), remove
[hyprpaper](https://github.com/hyprwm/hyprpaper), and hide your previous bar
if you enable hyprdesk's.

## Shortcuts

- `toggle` — `SUPER + D` by default — shows or hides the desktop;
  `settings` — `SUPER + SHIFT + D` — opens Settings; `leave` —
  `SUPER + Escape` — is the emergency exit. Shortcuts are reinstalled after
  every `hyprctl reload`, and hyprdesk never overwrites a binding that
  `hyprland.lua` already owns.
- **While the desktop is shown, Hyprland's shortcuts are suspended**: hyprdesk
  activates a submap so SUPER + Q, SUPER + F, etc. no longer target windows.
  The shortcuts you mark `{ submap_universal = true }` in `hyprland.lua` —
  typically workspace switching — stay active, and the desktop follows.
- You never get stuck: the submap hands control back to Hyprland *before*
  calling hyprdesk, and a submap left open by an abrupt shutdown is closed on
  startup.

## Commands

| Command | Effect |
|---|---|
| `hyprdesk` / `hyprdesk daemon` | starts the daemon (a single instance per session) |
| `hyprdesk show` · `launcher-only` · `hide` | the three desktop modes: full (windows set aside), launcher only, hidden |
| `hyprdesk toggle` · `cycle` | switch modes (`SUPER + D`, `SUPER + ALT + D`; the bar's Desktop button) |
| `hyprdesk settings [package]` | opens Settings without changing the desktop mode |
| `hyprdesk theme` · `wallpaper split` · `export` / `import` | themes derived from an image, a scene cut across screens, sharing your setup |
| `hyprdesk config get/set/unset` | settings from the command line, written to `settings.lua` |
| `hyprdesk doctor` | external dependencies (hypridle, wpctl, nmcli, portals…): present or missing, and what relies on them |
| `hyprdesk reload` · `status` · `check` | reread the configuration; state; check a file without touching the daemon |

`hyprdesk menu <module>` opens a module menu from the keyboard;
`hyprdesk open <role>` launches the default application for a role
(`browser`, `terminal`, `ai`…).

## In the Hyprland ecosystem

hyprdesk builds on [Hyprland](https://hyprland.org)
([source](https://github.com/hyprwm/Hyprland)) and complements it:

- replaces [hyprpaper](https://github.com/hyprwm/hyprpaper) — wallpapers are
  drawn by the desktop, per workspace and per screen, with crossfade;
- lock and suspend actions rely on [hyprlock](https://github.com/hyprwm/hyprlock)
  and [hypridle](https://github.com/hyprwm/hypridle);
- screenshot tools such as [hyprshot](https://github.com/hyprwm/hyprshot)
  keep working on the desktop: mark their bindings `submap_universal = true`:

```lua
hl.bind("Print", hl.dsp.exec_cmd("hyprshot -m region --clipboard-only"), { submap_universal = true })
```

Built with [iced](https://github.com/iced-rs/iced),
[iced_layershell](https://github.com/waycrate/exwlshelleventloop) and
[mlua](https://github.com/mlua-rs/mlua).

## Development

```bash
cargo test --locked                 # Lua config, resolution, placement, IPC, .desktop
cargo run --locked -- preview       # isolated preview with demo data, no session changes
bash scripts/verify.sh              # format, strict Clippy, tests, build, isolated D-Bus
bash scripts/verify.sh --visual     # adds screenshots at 1440, 900 and 640 px
```

`Cargo.lock` pins `winit-core` to `0.31.0-beta.2`, because beta.4 breaks
`iced_exdevtools`, a dependency of `iced_layershell` 0.19 — build with
`--locked`.

## Credits

Some pieces come from other projects of the Mindlog family; they are
referenced here rather than presented as hyprdesk's own:

- **Mindlog** — the default typeface, bundled under
  [SIL OFL 1.1](assets/fonts/mindlog/OFL.txt);
- **LYRA** — the AI assistant: the assistant drawer integrates LYRA
  (`hyprpanel-chat`) from
  [mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools);
- **hyprpanel-core** — the Hyprland IPC, from the same
  [mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools)
  repository.

Code under the MIT license.
