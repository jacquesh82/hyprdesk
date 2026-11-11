# hyprdesk

*[Version française](README.fr.md)*

![License](https://img.shields.io/badge/license-MIT-blue)
![Hyprland](https://img.shields.io/badge/Hyprland-%E2%89%A5%200.57-58e1ff)
![Rust](https://img.shields.io/badge/Rust-%E2%89%A5%201.99-dea584)

**An on-demand desktop for [Hyprland](https://hyprland.org), and a simple way
to configure it, live.**

With tiling, windows cover the whole screen and the desktop disappears.
hyprdesk brings it back with a single key: wallpaper, icons and widgets,
with a separate desktop for each workspace if you want one.

It also gives you **one place to set up Hyprland's essentials** (screens,
keyboard and mouse, workspaces, layouts, animations, shortcuts, idle and lock)
without editing config files by hand. **Every change applies live**, with no reload and no
restart.

<p align="center">
  <img src="assets/screenshots/desktop.png" width="880" alt="hyprdesk desktop: bar on top, icon column, clock, calendar and system widgets, dock at the bottom">
</p>

## Features

- **Hyprland made simple, applied live.** One settings drawer handles screens
  (position, rotation, scale, refresh rate), keyboard (layouts and variants,
  key repeat, Num Lock, with a test area), mouse and touchpad (speed,
  acceleration, natural scrolling, tap to click, left-handed mode),
  workspaces and their screens,
  layouts (Dwindle, Master, Scrolling, Monocle) for each workspace, animations
  and blur, shortcuts, and idle and lock delays. You see each change
  instantly. Screen changes revert on their own after 12 s unless you keep
  them. hyprdesk only touches what you change: the rest stays in
  `hyprland.lua`.
- **Three desktop modes.** `SUPER + D` sets the windows aside and shows the
  desktop. `SUPER + ALT + D` leaves the windows where they are and shows only
  the dock. A third mode hides the desktop entirely. You can also force a mode
  on a screen or a workspace.
- **Wallpapers for each workspace and screen**, with crossfades and
  slideshows. hyprdesk replaces hyprpaper.
- **A bar and a dock.** Workspaces, tray, volume, brightness and any plugin.
  You rearrange bar modules by dragging them.
- **One app launcher** (`SUPER + R`, in every mode). On the full desktop it opens as a
  large sheet with categories. In the other modes it opens as a centered
  search card. The fuzzy search finds Firefox when you type "ffx". Your most
  launched apps come first as suggestions, and `:clip` searches your
  clipboard history.
- **Window navigation.** The `ALT + Tab` switcher also offers the windows the
  desktop has set aside. Picking one hides the desktop and brings it back.
  `SUPER + Tab` opens a window overview. It uses hyprexpo or Hyprspace if you
  have one, and hyprdesk's own switcher otherwise. While the desktop is shown,
  the number row still switches workspaces, whatever your keyboard layout,
  and `SUPER + Left` / `SUPER + Right` go to the previous / next one.
- **Widgets and Lua plugins.** Clock, calendar, system stats and media player
  come built in. To add your own widget, drop a Lua file in a folder. No
  recompiling.
- **Themes and default apps.** Bundled themes, or themes built from an image,
  and one default application per role (browser, terminal, files…).
- **Session essentials.** Lock screen (with fingerprint support), greetd login
  screen, polkit agent, notifications with history and Do Not Disturb,
  screenshots and screen recording, clipboard history, pending updates.
- **AI assistant (optional).** A chat drawer that already knows your setup.
  It proposes changes with a live preview, and nothing runs until you accept.
  It is powered by **LYRA**, from
  [mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools):
  bring your own Claude Code, Codex or Zcode subscription.

## Widgets and plugins

Everything below goes on the desktop (`hd.widget("<name>")`), in the bar
(`hd.bar({ right = { "<name>" } })`), or both. You can also turn it on with a
click in Settings → Widgets.

**Built-in widgets**

| Name | What it shows |
|---|---|
| `clock` | the time, in any `strftime` format |
| `calendar` | a month view, with navigation between months |
| `system` | CPU, memory, load, disks, uptime |
| `media` | the track playing in any MPRIS player |
| `command` | the output of a shell command, refreshed at an interval |
| `lua` | whatever an inline Lua function returns |

**Bar modules**: `workspaces`, `window` (active window title), `volume`,
`brightness` (backlight, or external screens over DDC/CI), `tray` and
`desktop` (the desktop mode button).

**Bundled plugins**, written in Lua and built into the binary:

| Name | What it does | Relies on |
|---|---|---|
| `reseau` | wired and Wi-Fi, nearby networks, password prompt, VPN and WireGuard, airplane mode | NetworkManager |
| `bluetooth` | on/off, connected and paired devices with battery, scan and pair | bluetoothctl |
| `batterie` | laptop and peripheral batteries, power profiles, low-battery alerts | sysfs, power-profiles-daemon |
| `capture` | screenshot menu (area, window, screen), screen recording with a timer | hyprshot, wf-recorder |
| `clavier` | keyboard layout: pick from a menu, or scroll to switch | Hyprland |
| `lecteur` | current track in the bar; menu with position and controls | MPRIS |
| `majs` | pending updates, one-click upgrade | pacman/AUR, apt, dnf, zypper, flatpak |
| `supports` | USB drives and SD cards: open, eject, notification on plug-in | udisks2 |
| `nuit` | night light, on a schedule or on demand | hyprsunset |
| `vkbd` | show or hide the on-screen keyboard | wvkbd |
| `notifications` | Do Not Disturb and the latest notifications | hyprdesk or wayle |
| `session` | lock, suspend, log out, reboot, power off | systemd, hyprdesk lock |

Plugins with nothing to show (no battery, no USB drive, nothing playing) hide
themselves. Two more plugins, `meteo` (weather from wttr.in) and `notes` (the
first lines of a text file), are copied to `~/.config/hypr/hyprdesk/widgets/`
as examples to start from.

**Make your own** by dropping a Lua file in a folder. No recompiling: see
[Writing a plugin](docs/WRITING_PLUGINS.md).

## Quick start

You need Hyprland 0.57 or later, Rust 1.99 or later, and a Vulkan or GL
driver.

```bash
git clone https://github.com/jacquesh82/hyprdesk && cd hyprdesk
./scripts/install.sh      # builds to ~/.local/bin, writes a starter config, enables the systemd user service
hyprdesk snippet          # prints the lines to add to hyprland.lua
```

Then press `SUPER + D`. The default shortcuts are:

| Shortcut | Action |
|---|---|
| `SUPER + D` | show the desktop (press again to hide it) |
| `SUPER + ALT + D` | windows stay in place, dock only |
| `SUPER + CTRL + D` | hide the desktop |
| `SUPER + SHIFT + D` | open Settings |
| `SUPER + R` | app launcher and search, in every mode |
| `ALT + Tab` | window switcher |
| `SUPER + Tab` | window overview |
| `SUPER + Escape` | emergency exit |

hyprdesk never takes over a shortcut that `hyprland.lua` already uses,
except the launcher's `SUPER + R` (so that one key opens the same launcher
everywhere; you can turn that off) and the ones you hand over yourself.
Settings → Desktop → Shortcuts and `hyprdesk keys` show every conflict, with
the `hyprland.lua` line involved; LYRA can sort them out with you.

## Configuration

Your configuration lives in `~/.config/hypr/hyprdesk.lua`:

```lua
hd.wallpaper({ path = "~/.wallpapers/default.jpg" })
hd.bar({ left = { "workspaces", "window" }, right = { "tray", "volume", "session" } })
hd.icon({ app = "firefox" })
hd.widget("clock", { anchor = "top-right", format = "%H:%M" })
```

Changes you make in the settings drawer go to `settings.lua`. Your own file is
never rewritten, and it always has the last word. Both files are reloaded as
soon as you save them. If you make a mistake, a notification points to
`file:line` and the last working configuration stays active. The
[advanced guide](docs/ADVANCED.md#configuration-reference) lists every option.

## Documentation

- **[Advanced guide](docs/ADVANCED.md)**: how hyprdesk works and everything
  you can configure
- [Writing a plugin](docs/WRITING_PLUGINS.md): a step-by-step guide
- [Plugins, widgets and the launcher](docs/PLUGINS.en.md): internal design
- `hyprdesk --help` lists every command
- `hyprdesk doctor` shows which optional dependencies are present or missing

## Credits

- **[Mindlog](https://github.com/jacquesh82/mindlog.fonts)**: the typeface
  you see everywhere in hyprdesk. It comes bundled, so it works out of the
  box, and it is under [SIL OFL 1.1](assets/fonts/mindlog/OFL.txt).
- **[mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools)**:
  home of LYRA, the AI assistant, and of `hyprpanel-core`, the Hyprland IPC
  that hyprdesk is built on.

## License

MIT.
