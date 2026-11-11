# hyprdesk — advanced guide

*[Back to the README](../README.md)*

This guide explains how hyprdesk works under the hood and what you can
configure. If you only want to try it out, the [README](../README.md) is enough.

- [Architecture](#architecture)
- [The desktop and its three modes](#the-desktop-and-its-three-modes)
- [Talking to Hyprland](#talking-to-hyprland)
- [Keyboard: bindings and the submap](#keyboard-bindings-and-the-submap)
- [Configuration pipeline](#configuration-pipeline)
- [Session root: native and orchestrated services](#session-root-native-and-orchestrated-services)
- [Lock screen and greeter](#lock-screen-and-greeter)
- [Configuration reference](#configuration-reference)
- [Widget plugins](#widget-plugins) · [Writing a plugin](WRITING_PLUGINS.md)
- [AI assistant](#ai-assistant)
- [Files and environment](#files-and-environment)
- [Development](#development)

## Architecture

hyprdesk is **one Rust binary** that plays several roles:

| Process | Started by | Role |
|---|---|---|
| `hyprdesk daemon` | the systemd user service | draws every surface, runs the Lua config, supervises the session |
| `hyprdesk <command>` | a key binding, a script, you | short-lived client: sends one line to the daemon and exits |
| `hyprdesk lock` | the daemon, logind, hypridle | lock screen, a **separate** process that runs no Lua |
| `hyprdesk greeter` | greetd | login screen, before any user session exists |

```
                     hyprland.lua bindings / scripts
                                  │
                     hyprdesk toggle | settings | …   (client)
                                  │  one line per request
                                  ▼
  $XDG_RUNTIME_DIR/hypr/<instance>/hyprdesk.sock
                                  │
   ┌──────────────────────── hyprdesk daemon ────────────────────────┐
   │  UI thread (iced + layer-shell)   Lua thread (hd.*, render, hooks)│
   │  Hyprland IPC actor               widget data workers             │
   │  D-Bus: notifications, tray, polkit, MPRIS, logind                │
   │  supervised children: hd.component, hypridle, cliphist…          │
   └──────────────────────────────────────────────────────────────────┘
                                  │
                    Hyprland (.socket.sock / .socket2.sock)
```

- **Single instance.** The daemon takes an exclusive `flock` on
  `$XDG_RUNTIME_DIR/hypr/<instance>/hyprdesk.lock` for its whole lifetime.
  Starting two daemons at the same moment (systemd plus a manual launch) is
  safe: the kernel arbitrates, and the lock disappears with the process, even
  after a `kill -9`. `hyprdesk daemon --replace` hands over cleanly.
- **systemd.** The user service runs with `Restart=always`. `hyprdesk quit` is
  the only exit that is not restarted.
- **Nothing blocks rendering.** Hyprland calls that move windows go through
  a sequential actor. Widget data is collected off the UI thread, and only
  while the desktop is visible. External commands run with a time limit, a
  memory limit and process-group termination.

Main dependencies: [iced](https://github.com/iced-rs/iced) 0.14,
[iced_layershell](https://github.com/waycrate/exwlshelleventloop) and
`iced_sessionlock` 0.19, [mlua](https://github.com/mlua-rs/mlua) (Lua 5.4,
vendored), [zbus](https://github.com/dbus2/zbus) and `hyprpanel-core` for the
Hyprland IPC.

## The desktop and its three modes

hyprdesk creates **one layer-shell surface per screen**:

| Surface | Layer | Purpose |
|---|---|---|
| Desktop | `background` | wallpaper at rest; icons and widgets when shown |
| Bar | `top`, with an exclusive zone | Hyprland tiles windows beside it, and the desktop grid shifts by the same amount |
| Dock / launcher | its own layer | open windows, pinned apps, settings button |
| Settings, menus, assistant, notifications, OSD | separate surfaces | drawers, popovers and bubbles |

The desktop **never covers windows**. To show it, hyprdesk moves them out of
the way:

| Mode | Command | What happens |
|---|---|---|
| `full-desktop` | `hyprdesk show`, `SUPER + D` | the workspace's windows move to `special:hyprdesk`; the desktop takes the keyboard and fades in icons and widgets. Apps launched from it open floating and centered, **on top** |
| `launcher-only` | `hyprdesk launcher-only`, `SUPER + ALT + D` | windows stay tiled; only the dock is available |
| `no-desktop` | `hyprdesk hide`, `SUPER + CTRL + D` | the desktop is just the wallpaper; set-aside windows come back |

`hyprdesk cycle` goes no-desktop → launcher-only → full-desktop.
`forced_modes` pins a mode to a screen or a workspace. For example, you can
make sure a workspace never shows a desktop, or always switches to one when
you arrive.

`special:hyprdesk` is never treated as a screen's workspace, nor as a place
to return to. The Alt+Tab switcher filters it out too.

## Talking to Hyprland

- **State.** hyprdesk listens to the event stream (`.socket2.sock`). It does
  not replay each event onto a local copy, which would drift over time.
  Instead, any event that touches screens or workspaces triggers a full
  re-read of `j/monitors` (a few hundred bytes).
- **Live changes.** Screens (`hl.monitor`), layouts (`general.layout`,
  workspace rules), animations, curves and decoration effects (`hl.config`)
  are applied through Hyprland's Lua IPC, with no reload.
- **Minimal footprint.** hyprdesk only applies what *you changed in
  hyprdesk*. Everything else still comes from `hyprland.lua`. When you remove
  a setting, Hyprland re-reads its own config and hyprdesk re-applies the
  settings it still owns.
- **Screens.** A layout change applies live. **Keep** writes it to
  `monitors.lua`. If you don't confirm within 12 s, the previous layout comes
  back. Unplugged screens keep their line, so they come back as they were when
  plugged in again.
- **Keyboard, mouse, touchpad.** hyprdesk reads the current values from
  Hyprland (`getoption`) and applies changes live (`hl.config({ input = … })`).
  It then writes them to `inputs.lua`, which `hyprland.lua` loads with
  `require("inputs")`. Keyboard layouts and variants come from
  xkeyboard-config (`evdev.lst`). If another tool wrote the file before, its
  version is kept once as `inputs.lua.bak`. Sliders apply when you release
  them.

## Keyboard: bindings and the submap

- hyprdesk installs its own bindings (`keys.*`) and reinstalls them after
  every `hyprctl reload`. It **never overrides** a binding that
  `hyprland.lua` already owns. Conflicts show up in Settings → Desktop →
  Shortcuts.
- While the desktop is shown, hyprdesk enters a **submap**, so `SUPER + Q`,
  `SUPER + F` and the like no longer act on the hidden windows. Bindings
  marked `{ submap_universal = true }` in `hyprland.lua` (typically workspace
  switching and screenshots) stay active, and the desktop follows them.
- The submap still lets you move around: the **number row** goes to
  workspaces 1 to 10, with the same modifier as your workspace bindings
  (`SUPER` by default). It matches physical key codes, so it works with
  AZERTY, QWERTY and any other layout. The left and right arrows, with
  that modifier, go to the previous and next workspace. The desktop
  follows, and the new workspace's windows are set aside too. `keys.search` (or
  `keys.search_desktop`), `keys.switch` and `keys.overview` also stay
  available.
- **Shortcut conflicts.** hyprdesk compares its shortcuts with Hyprland's
  live bindings and with `hyprland.lua` (line and command of each
  `hl.bind`). Two hyprdesk actions on one combination: the second is not
  installed. A combination bound in `hyprland.lua`: hyprdesk's action only
  works while the desktop is shown. A combination marked
  `submap_universal`: hyprdesk leaves it alone, so nothing fires twice
  (media keys included). Settings → Desktop → Shortcuts lists them first,
  with "hyprdesk takes it" (`keys.take`) and a LYRA button; `hyprdesk keys`
  prints the same report and exits with 1 if there is a conflict.
- **Alt+Tab on the full desktop** also offers the windows the desktop has
  set aside, labelled as such. Picking one, or a window from another
  workspace, hides the desktop and focuses it. A window opened from the
  desktop takes focus without leaving it.
- The submap gives control back to Hyprland *before* calling hyprdesk. If the
  daemon dies mid-way, you are never stuck, and a submap left open is closed
  on the next start. `keys.leave` (`SUPER + Escape`) is the emergency exit.
- `keys.media = true` binds the XF86 volume, brightness and media keys to
  `hyprdesk osd …` and `hyprdesk media …`. These run in the client process, so
  they work even when the daemon is down.

## Configuration pipeline

```
 ~/.config/hypr/hyprdesk/settings.lua   ← generated: Settings drawer, `hyprdesk config set`
 ~/.config/hypr/hyprdesk.lua            ← yours, never rewritten, has the last word
              │  executed in this order on a dedicated Lua thread
              ▼
       configuration model  →  resolution per (screen, workspace)  →  surfaces
```

- **Two files, one order.** `settings.lua` is plain generated Lua, and it is
  the only source of truth for the Settings drawer, which re-reads it by
  executing it. It loads **before** your `hyprdesk.lua`, so anything you write
  by hand wins. The drawer and `hyprdesk config` flag keys that your file
  overrides.
- **Safe reload.** Both files are watched. A reload builds a *new* Lua state,
  and the old one is replaced only if the new files run without error. On
  error you get a notification with `file:line`, and the last valid
  configuration stays active, as with Hyprland itself.
  `hyprdesk check [file]` runs a file without touching the daemon.
- **One thread owns Lua.** The Lua state is not `Send`, so it lives on its own
  thread (widget `render` functions, `hd.on` hooks). The rest of the program
  talks to it through messages.
- **Resolution.** For each (screen, workspace) pair:
  - *wallpaper and mode*: the **most specific** rule wins (workspace + screen
    > workspace > screen > `*`). On a tie, the rule declared last wins;
  - *icons and widgets* **add up**, unless `hd.desktop({ inherit = false })`
    excludes the ones declared for all workspaces.
- **Command line.** `hyprdesk config list [package] | get | set | unset` has
  the same effect as the drawer: it writes to `settings.lua`, which the daemon
  reloads right away.

## Session root: native and orchestrated services

hyprdesk follows a hybrid approach: it is **native for what you see** and
**orchestrates what you don't**. Each service is taken only if no one else
already holds it, so your existing setup stays in charge.

| Service | How | If another one is running |
|---|---|---|
| Wallpaper | native (replaces hyprpaper) | — |
| Notifications | native `org.freedesktop.Notifications` 1.2 daemon (bubbles, history, Do Not Disturb) | name taken with `DoNotQueue`: mako, dunst, swaync… keep it |
| Tray | native StatusNotifierItem host, `dbusmenu` menus rendered natively | — |
| polkit agent | native, speaks to `polkit-agent-helper-1` | the existing agent stays |
| Lock screen | native (`ext-session-lock-v1`, PAM, optional fprintd) | — |
| Idle | orchestrates **hypridle** (`idle_manage`): hyprdesk writes its config and supervises it | a hand-configured hypridle stays in charge |
| Clipboard history | orchestrates **cliphist** (`wl-paste --watch cliphist store`) | an existing watcher is left alone |
| Night light | drives **hyprsunset** over IPC | gives way to wlsunset |
| Screenshots / recording | orchestrates **hyprshot**, **wf-recorder**, **slurp** | — |
| Brightness | sysfs + logind `SetBrightness` (no root); external screens over DDC/CI (`ddcutil`) | — |
| Updates | pacman/AUR (`yay`/`paru`), apt, dnf, zypper, flatpak; upgrade through `pkexec` | — |
| XDG autostart | `autostart = true`: runs `~/.config/autostart` and `/etc/xdg/autostart` once per session, per spec | off by default |
| Your components | `hd.component(name, { exec, restart })`: started with the daemon, restarted after a crash with growing delays, stopped with it | — |

`hyprdesk doctor` lists every external tool, what depends on it, and the
package that provides it. A missing tool never prevents startup: only the
feature that relies on it stays inactive. When a service is held elsewhere,
Settings offers a ready-made request so the assistant can find where it is
launched and offer to hand it over to hyprdesk. It always asks before acting.

## Lock screen and greeter

**`hyprdesk lock`** is designed so that a plugin can neither block it nor
unlock it:

- it uses `ext-session-lock-v1`: **Hyprland** enforces the lock, so if the
  process crashes, the session stays locked;
- it is a separate process with **no Lua**. It reads values the daemon has
  already resolved from `$XDG_RUNTIME_DIR/hyprdesk/lock.json`, and falls back
  to safe defaults if the file is missing;
- it checks the password through PAM (a small hand-written binding) on a
  separate thread. The buffer is wiped afterwards, the password is never
  logged, and pasting is disabled;
- it reads the fingerprint through fprintd over D-Bus, in parallel with the
  password (`lock_fingerprint`). No PAM stack changes are needed;
- only one instance runs, and nothing can unlock it from outside. The content
  drifts slowly and fades out to avoid burn-in.

**`hyprdesk greeter`** is a login screen for
[greetd](https://sr.ht/~kennylevinsen/greetd/). It speaks greetd's IPC
(length-prefixed JSON on `$GREETD_SOCK`) and lists the `wayland-sessions` and
`xsessions` entries. It runs under the compositor greetd starts, for example
`cage -s -- hyprdesk greeter`. `--tty` runs it in a terminal and `--demo` runs
it without greetd.

**Settings → Login screen** sets it up in place of GDM (or SDDM, LightDM…).
It shows the login manager in use and the pieces greetd needs, then does
each system step through `pkexec hyprdesk greeter setup <action>`:

- `packages` installs greetd and cage;
- `install` copies hyprdesk to `/usr/local/bin`, because the `greeter` user
  cannot read `~/.local/bin`. It then writes `/etc/greetd/config.toml`. A file
  from another tool is kept once as `config.toml.bak`;
- `enable` disables the current manager and enables greetd. This happens at
  the next boot, so the open session is not cut off. It is refused while a
  piece is missing;
- `restore` goes back to the previous manager. From a console:
  `sudo hyprdesk greeter setup restore`.

## Configuration reference

The whole API is described in LuaLS definitions (`hyprdesk stubs` writes
`hyprdesk.meta.lua`), which give you autocompletion next to Hyprland's
`hl.meta.lua`. Every rule accepts the `workspace` and `monitor` selectors: a
name, a number, a list, or `"*"`.

### `hd.config({ … })`

| Area | Keys |
|---|---|
| Desktop | `mode` (`"opaque"`/`"glass"`), `glass_opacity`, `background`, `show_on` (`"all"`/`"focused"`/`"primary"`), `primary`, `close_on_launch`, `float_launched`, `forced_modes`, `desktop_files` (Desktop folder and Trash as icons) |
| Grid and icons | `grid = { cell, gap, margin, icon_size }`, `icon_theme` |
| Look | `theme = { name, fg, bg, accent, effects = { bar, menus, launcher, settings, widgets } }` (each effect: `opacity`, `blur`), `font`, `font_size` |
| Motion | `transition` (`"fade"`/`"slide"`/`"none"`), `fade_ms`, `crossfade_ms`, `fx` (Hyprland animations, curves and decoration, as `hl.config` keys) |
| Dock | `running_apps` (`"off"`/`"workspace"`/`"all"`), `launcher_group` (`"none"`/`"app"`/`"workspace"`) |
| App launcher | `apps_unified` (`true` by default: `keys.search` opens hyprdesk's launcher in every mode, taken from `hyprland.lua` if it binds that key; `false`: `hyprland.lua` keeps it outside the full desktop), `apps_style` (`"auto"`: large sheet on the full desktop, centered card otherwise; or `"plein"` / `"central"` to always use one), `apps_suggestions` (0 to 16 most launched apps, 8 by default), `apps_categories`, `terminal` |
| Workspaces and layouts | `workspaces` (1 to 30, or `false`), `workspace_monitors`, `layout` (`dwindle`, `master`, `scrolling`, `monocle`, `grid`, `columns`, `rows`, `centered`, `lua:<name>`), `layout_workspaces`, `dwindle`, `master`, `scrolling` |
| Keys | `keys = { toggle, launcher, hide, settings, lock, leave, switch, search, search_desktop, overview, media, take }` (`false` disables a key; `search` is `SUPER + R`; `search_desktop` replaces it on the full desktop; `take` lists `hyprland.lua` combinations hyprdesk unbinds and uses — given back when removed), `suspend_binds`, `overview_command` (a Hyprland plugin's overview, such as hyprexpo or Hyprspace; otherwise hyprdesk's switcher, which stays open until you click, press Enter or Escape) |
| Session | `autostart`, `clipboard_history`, `idle_manage`, `idle_dim`, `idle_lock`, `idle_dpms`, `idle_suspend`, `lock_fingerprint`, `update_commands` |
| Plugins | `plugin_options = { [plugin] = { option = value } }`: defaults for every instance |
| Interface | `language` (`"auto"`/`"fr"`/`"en"`), `advanced_settings`, `tour_seen` |

### Rules

```lua
-- Wallpaper: an image, a slideshow, or a solid color; fit = cover|contain|fill|center.
-- span = true cuts one scene across all screens following their real layout
-- ("lock" or "desktop" limits it to one of the two).
hd.wallpaper({ path = "~/.wallpapers/default.jpg" })
hd.wallpaper({ workspace = 3, rotate = { dir = "~/.wallpapers", interval = 600, shuffle = true } })
hd.wallpaper({ monitor = "DP-2", color = "#1e1e2e" })

-- Desktop mode per workspace or screen; inherit = false drops global icons and widgets.
hd.desktop({ workspace = 4, mode = "glass", inherit = false })

-- Icons: a .desktop id, a path (opened with xdg-open), or a command.
hd.icon({ app = "firefox" })
hd.icon({ workspace = 2, label = "Projects", path = "~/Documents", pos = { 0, 1 } })
hd.icon({ label = "htop", exec = "htop", terminal = true, icon = "utilities-system-monitor" })

-- Widgets: clock, calendar, system, media, command, lua, or any plugin name.
hd.widget("clock",   { anchor = "top-right", format = "%H:%M", size = 72, card = false })
hd.widget("system",  { anchor = "bottom-right", show = { "cpu", "ram", "load", "disk:/", "uptime" } })
hd.widget("media",   { player = "spotify" })
hd.widget("command", { exec = "curl -s 'wttr.in/?format=3'", interval = 900, timeout = 5 })
hd.widget("lua",     { render = function() return os.date("%A") end })
hd.place("clock#0", { anchor = "top", y = 40 })   -- reposition by id
hd.disable("calendar#0")                          -- remove by id

-- Bar: modules are widgets; "primary" targets the primary screen.
hd.bar({ monitor = "primary", position = "top", compact = "auto",
         left = { "workspaces", "window" }, center = { { "clock", format = "%a %d %H:%M" } },
         right = { "tray", "volume", "brightness", "reseau", "session" },
         portrait = { left = { "workspaces" }, right = { "clock" } } })

-- Window rules and session components.
hd.assign({ app = "org.telegram.desktop", workspace = 9, silent = true })
hd.component("applet", { exec = "nm-applet --indicator", restart = true })

-- Hooks: show, hide, workspace.
hd.on("workspace", function(e) print(e.monitor, e.workspace) end)
```

Helpers available in the config and in plugins: `hd.spawn(cmd)` (launch
through Hyprland, without waiting), `hd.exec(cmd)` (run a command and return
its stdout), `hd.exe` (path of the running hyprdesk binary),
`hd.t(text)` and `hd.lang()` (interface language), and `hd.version`.

## Widget plugins

A plugin is `<dir>/<name>.lua` or `<dir>/<name>/init.lua`, and it returns a
table. hyprdesk searches these folders in order, and **the first one that
provides a name wins**:

1. `~/.config/hypr/hyprdesk/widgets/`
2. `$XDG_DATA_HOME/hyprdesk/widgets/` (`~/.local/share/…`)
3. `$XDG_DATA_DIRS/*/hyprdesk/widgets/` (system-wide)
4. bundled plugins, compiled into the binary: `reseau`, `bluetooth`,
   `batterie`, `capture`, `clavier`, `lecteur`, `majs`, `supports`, `nuit`,
   `vkbd`, `notifications`, `session`

```lua
return {
    name = "VPN", description = "VPN status", interval = 5, anchor = "top-right",
    options = { interface = "wg0" },                -- defaults, editable in Settings → Plugins → Configure
    render   = function(o) return { icon = "network-vpn-symbolic", tooltip = "VPN", state = "active" } end,
    on_click = function(o) end,
    on_scroll = function(o, dir) end,              -- dir = -1 or +1
    menu = function(o) return {
        { header = "VPN" },
        { label = "Connect", action = function() hd.spawn("wg-quick up " .. o.interface) end },
        "-",
        { label = "Disconnect…", confirm = "Disconnect?", action = function() hd.exec("wg-quick down " .. o.interface) end },
        { input = "Server address", action = function(text) end },   -- text field; secret = true masks it
    } end,
}
```

`render` returns text or `{ text, icon, tooltip, state }`, where `state` is
`normal`, `active`, `warning`, `critical` or `hidden` (a hidden module takes
no space in the bar). Icons ending in `-symbolic` are tinted with the theme.
A plugin can be used on the desktop with `hd.widget("<name>", { … })` and in
the bar as a compact module. To call hyprdesk from a plugin, use `hd.exe`,
never the `PATH`.

**→ [Writing a plugin](WRITING_PLUGINS.md)** walks you through building one
step by step, from "hello world" to menus, options and translations. The
internal design of packages, widgets and the launcher is in
[`PLUGINS.en.md`](PLUGINS.en.md).

## AI assistant

The assistant drawer is a front end for **LYRA** (`hyprpanel-chat`, from
[mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools)).
LYRA keeps the whole engine: providers (Claude Code, Codex, Zcode), tools,
confirmations and memory.

- **Protocol.** `hyprpanel-chat serve` runs as a child process and speaks
  JSON Lines on stdin/stdout. Unknown fields are ignored, and an unknown
  `type` is reported without error. The engine only lives while the drawer is
  open.
- **Context.** hyprdesk writes a Markdown description of the session (modes,
  settings packages, apps, screens, workspaces, widgets, files, commands) to
  `~/.config/hyprpanel/context.d/hyprdesk.md`. `hyprdesk context` prints it.
- **Testing without LYRA.** `HYPRDESK_CHAT_ENGINE="hyprdesk chat-mock"`
  replaces the engine with a fake one that speaks the same protocol.

## Files and environment

| Path | Content |
|---|---|
| `~/.config/hypr/hyprdesk.lua` | your configuration (`$HYPRDESK_CONFIG` overrides it) |
| `~/.config/hypr/hyprdesk/settings.lua` | generated settings |
| `~/.config/hypr/hyprdesk/widgets/` | your plugins |
| `~/.config/hypr/hyprdesk/themes/` | "My themes" (theme archives) |
| `~/.config/hypr/monitors.lua` | screen layout, written on **Keep** (`require("monitors")` in `hyprland.lua`) |
| `~/.config/hypr/inputs.lua` | keyboard, mouse and touchpad settings (`require("inputs")` in `hyprland.lua`) |
| `$XDG_STATE_HOME/hyprdesk/layout.toml` | icon positions moved with the mouse |
| `$XDG_STATE_HOME/hyprdesk/bar-offered` | bar modules already offered once |
| `$XDG_RUNTIME_DIR/hypr/<instance>/hyprdesk.sock` · `.lock` | control socket, single-instance lock |
| `$XDG_RUNTIME_DIR/hyprdesk/lock.json` | resolved state handed to the lock screen |

| Variable | Effect |
|---|---|
| `HYPRDESK_CONFIG` | path of the main config file |
| `HYPRDESK_LANG` | interface language for CLI commands (`fr`, `en`) |
| `HYPRDESK_LOG` | `tracing` filter, e.g. `warn,hyprdesk::ui=trace` |
| `HYPRDESK_CHAT_ENGINE` | assistant engine command (development) |
| `HYPRDESK_PREVIEW_SCENES`, `_SCALE`, `_WIDTH`, `_HEIGHT` | preview: scenes to capture, HiDPI scale, size |

**Sharing a setup.** `hyprdesk export` writes a `.tar.gz` with a
`manifest.json`. It contains `hyprdesk.lua`, `settings.lua`, third-party
plugins and the wallpapers in use, plus `monitors.lua` with `--screens`.
`--theme` exports only what you see: colors, effects, wallpapers, desktop
mode, lock veil and transitions. `hyprdesk import` backs up the current setup
first, and `--dry-run` shows what would change.

## Development

```bash
cargo test --locked                 # Lua config, resolution, placement, IPC, .desktop, i18n catalog
cargo run --locked -- preview       # isolated preview with demo data: no IPC, no persistence, no launches
cargo run --locked -- preview out/  # same, and capture every view into out/
bash scripts/verify.sh              # format, strict Clippy, tests, build, isolated D-Bus
bash scripts/verify.sh --visual     # adds screenshots at 1440, 900 and 640 px
bash scripts/package.sh             # .deb, .rpm and Arch packages (nfpm)
```

- `Cargo.lock` pins `winit-core` to `0.31.0-beta.2`, because beta.4 breaks
  `iced_exdevtools`, a dependency of `iced_layershell` 0.19. Always build
  with `--locked`.
- Drag and drop onto the desktop needs the Wayland data device, which
  `layershellev` 0.19.1 lacks. hyprdesk carries patched local copies in
  `vendor/` (see each `vendor/*/HYPRDESK.md`).
- Interface strings: French is the source language and serves as the key, as
  in gettext, and `locales/en.toml` holds the English catalog. The
  `catalogue_complet` test fails if any marked string has no translation.
- Visual language and motion rules: [`DESIGN.md`](../DESIGN.md) (in French).
  The default interface font is
  [Mindlog](https://github.com/jacquesh82/mindlog.fonts), embedded in the
  binary, so no Fontconfig cache is needed. Five weights, from Light to Bold.
