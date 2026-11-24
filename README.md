# hyprdesk

[Version française](README.fr.md)

[![Checks](https://github.com/jacquesh82/hyprdesk/actions/workflows/ci.yml/badge.svg)](https://github.com/jacquesh82/hyprdesk/actions/workflows/ci.yml)
[![License: GPL v3+](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)](LICENSE)
![Hyprland 0.57+](https://img.shields.io/badge/Hyprland-0.57%2B-58e1ff)
![Rust 1.99+](https://img.shields.io/badge/Rust-1.99%2B-dea584)

**A complete desktop for [Hyprland](https://hyprland.org) that stays out of
your way until you call it.**

[![Login on the hyprdesk greetd screen over a looping video, SUPER + D through dock and full desktop, a file found with the :f search, LYRA fixing a duplicate shortcut after approval, SmartUI proposing new icons, a myMFA code filled into a form, then the closing card](assets/screenshots/hyprdesk-demo.gif)](assets/screenshots/hyprdesk-demo.mp4)

<sub>70-second film; click it for the video with sound.</sub>

hyprdesk turns Hyprland into a full desktop without giving up tiling. It
includes a bar, dock, launcher, desktop icons and widgets, notifications,
login and lock screens, and a settings drawer. They all come from one native
Rust program, configured from the drawer or in Lua.

Your windows stay tiled. Press `SUPER + D` and the desktop comes up over
them; leave it and they go back where they were. hyprdesk also writes
Hyprland's configuration for you: shortcuts, keyboard, screens, layouts.
Hyprland checks every write, and any change can be rolled back.

[Highlights](#highlights) · [Installation](#installation) · [Quick start](#quick-start) · [Configuration](#configuration) · [Theme engine](#theme-engine) · [User guide](docs/GUIDE.md) · [Contributing](#contributing)

![hyprdesk desktop with a top bar, app shortcuts, clock, calendar, system widgets, and a dock](assets/screenshots/desktop.png)

## Highlights

### A desktop on demand

`SUPER + D` cycles through three modes: hidden, dock only, and full desktop
with wallpaper, icons, and widgets. In full desktop mode your windows float
above it and return to their tiled layout when you leave. Each workspace
remembers its mode across sessions, and each screen follows the workspace it
shows. Wallpapers (images, slideshows, **looping videos**) are set per screen
or workspace, and a theme can be built from any image.

### Hyprland, configured from Settings

Arrange monitors, set the keyboard and touchpad, edit shortcuts by pressing
the new combination, choose a layout per workspace, or split a screen into
zones (1×2 up to 3×3). You can also adjust animations, borders, window
rules, the rendering profile, and the GPU that renders each frame.
hyprdesk writes the files `hyprland.lua` loads (`binds.lua`, `inputs.lua`,
`monitors.lua`, `autostart.lua`…) and has Hyprland check each write. If
Hyprland reports a new error, the previous file comes back. Screen changes
revert after 12 seconds unless you keep them, and 30 days of configuration
snapshots let you go back to any point.

### One theme, every app

Pick a theme and Hyprland, hyprdesk, GTK 3 and 4 apps (Brave and Firefox
included), Qt apps, and kitty follow it live, down to fonts, icons,
cursors, and sounds. Seven packs ship with hyprdesk (Ardoise, Clair, Verre,
Ambre, Encre, Forêt, Contraste élevé), and any image can become a new one.
Packs are plain data, never code, so a `.hyprtheme` archive from someone
else is safe to install, and your own apps can follow the palette too. See
the [Theme engine](#theme-engine) chapter.

### Panorama: a second life for strip and touch screens

A very wide screen (2560×720, 1920×480…), touch or not, becomes a full
screen of pages you swipe through. It can show a media player, video and
music services, web radios, system graphs with a task manager, clocks, and
touch shortcuts. Compositions are free (forest, telemetry, studio,
cinematic) and the screen splits into up to nine zones. No strip screen?
Reserve a Panorama band on an ordinary monitor, or try it in a window with
`hyprdesk panorama preview`. A video playing in the Panorama keeps the
session awake, picture-in-picture included.

| ![Panorama on a 2560×720 strip split into three zones: media player, clock, and system cards](assets/screenshots/panorama-split.png) | ![Forest composition: CPU, memory, disk, and load graphs next to an analog clock](assets/screenshots/panorama-forest.png) |
| ------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------- |

### LYRA, an assistant that knows your session

LYRA is optional. It explains a setting or fixes a shortcut conflict using
what hyprdesk knows about your session: monitors, workspaces, shortcuts,
apps. You approve every command it proposes, often with a live preview
first. It runs on your existing Claude Code, Codex, or Zcode subscription.
Say “Lyra” from the Panorama to ask by voice; recognition and speech run
offline. **SmartUI** redraws app icons in one consistent style.

### Every file, one keystroke away

The dock's loupe, `:f` in the launcher, or `hyprdesk files` searches an
index of your home and mounted volumes. Search by name, path, or category,
and results come back in milliseconds. inotify keeps the index fresh in the
background.

### Login, lock, and keyring

hyprdesk offers an optional greetd login screen with a video background,
and a native lock screen with PAM and fingerprints that you enroll from
Settings. The lock screen and the login screen are small dedicated binaries,
so restarting or updating hyprdesk never takes them down. The optional
**myMFA** keyring adds passwords and 2FA codes (`:2fa`, `:mdp` in the
launcher), serves the Secret Service, and unlocks at login.

### Lua all the way

Settings save to `settings.lua`. For dotfiles, `hyprdesk.lua` takes
precedence and reloads on save, and a broken line is reported with its file
and line number. Widgets and bar modules are plain Lua files. About a dozen
plugins ship with hyprdesk: network, Bluetooth, battery, updates with
progress, captures, night light, GPU passthrough switch, and more.

### Safe to try

Before touching anything, the installer records your original state.
`./scripts/uninstall.sh --restore` puts back every file, service, and
package as it was. Each Settings page has a **Defaults…** button, and
`hyprdesk export` packs your whole setup (hyprdesk, Hyprland, themes,
images) into one archive to import on another machine.

<details>
<summary>More screenshots</summary>

| Desktop and dock                                                                                                    | Settings and appearance                                                                                  |
| ------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| ![Monitor arrangement and display settings](assets/screenshots/screens.png)                                         | ![Widget settings with clock, calendar, and system stats on the desktop](assets/screenshots/widgets.png) |
| ![Launcher-only mode](assets/screenshots/mode-launcher.png)                                                         | ![Desktop settings](assets/screenshots/settings-desktop.png)                                             |
| ![Hidden desktop mode](assets/screenshots/mode-hidden.png)                                                          | ![Keyboard and mouse settings](assets/screenshots/settings-input.png)                                    |
| ![Bar settings](assets/screenshots/bar.png)                                                                         | ![Workspace layouts](assets/screenshots/layouts.png)                                                     |
| ![App launcher](assets/screenshots/launcher.png)                                                                    | ![Animation settings](assets/screenshots/settings-animations.png)                                        |
| ![Desktop icons](assets/screenshots/icons.png)                                                                      | ![Dock settings](assets/screenshots/settings-launcher.png)                                               |
| ![Wallpaper settings](assets/screenshots/wallpaper.png)                                                             | ![Theme settings](assets/screenshots/theme.png)                                                          |
| ![Desktop transitions and wallpaper crossfade](assets/screenshots/effects.png)                                      | ![About and system information](assets/screenshots/settings-about.png)                                   |
| ![Telemetry composition: flip clock, player, and circular gauges](assets/screenshots/panorama-telemetry.png)        | ![Studio composition: large clock and the player](assets/screenshots/panorama-studio.png)                |
| ![LYRA’s drawer showing a live preview and an approval prompt for a command](assets/screenshots/assistant-chat.png) | ![SmartUI icon generation settings](assets/screenshots/settings-smartui.png)                             |

</details>

## Installation

Run the installer from a terminal in your Hyprland session. It checks your
system and shows the selected options before making changes.

| Requirement     | Details                                                                        |
| --------------- | ------------------------------------------------------------------------------ |
| System          | Linux on x86_64 or aarch64, systemd, a regular user session, and `sudo` access |
| Desktop         | Hyprland 0.57+ and a Vulkan or GL driver                                       |
| Package manager | pacman (Arch), apt (Debian/Ubuntu), dnf (Fedora), or zypper (openSUSE)         |
| Build           | Rust and ~4 GiB free, only to build the repository or LYRA and myMFA           |

```bash
git clone https://github.com/jacquesh82/hyprdesk
cd hyprdesk
./scripts/install.sh
```

`scripts/install.sh` downloads the
[latest release](https://github.com/jacquesh82/hyprdesk/releases/latest)
package, installs it with your package manager, and sets up the rest. To
build this repository as it stands, use `scripts/install_nightly_build.sh`
instead. The setup assistant asks for your language, keyboard, and
integrations. By default it installs the desktop only: the login screen,
LYRA, and myMFA stay off until you pick them. On first launch, a visual
wizard walks through screens, keyboard, theme, bar, and default apps.

| Distribution              | 0.2.0 release package                                                                                                                                                                   |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Debian, Ubuntu (amd64)    | [hyprdesk_0.2.0-1_amd64.deb](https://github.com/jacquesh82/hyprdesk/releases/download/v0.2.0/hyprdesk_0.2.0-1_amd64.deb)                                                                |
| Fedora, openSUSE (x86_64) | [hyprdesk-0.2.0-1.x86_64.rpm](https://github.com/jacquesh82/hyprdesk/releases/download/v0.2.0/hyprdesk-0.2.0-1.x86_64.rpm)                                                              |
| Arch Linux (x86_64)       | [hyprdesk-0.2.0-1-x86_64.pkg.tar.zst](https://github.com/jacquesh82/hyprdesk/releases/download/v0.2.0/hyprdesk-0.2.0-1-x86_64.pkg.tar.zst), or the [AUR package](https://aur.archlinux.org/packages/hyprdesk) |

### Update and uninstall

```bash
./scripts/update.sh                # review and apply updates (--check, --yes)
./scripts/uninstall.sh --dry-run   # see what would change, touch nothing
./scripts/uninstall.sh             # summary, confirmation, then uninstall
./scripts/uninstall.sh --restore   # put everything back exactly as before hyprdesk
```

Installer options, integrations, and what uninstalling restores are covered
in the [user guide](docs/GUIDE.md#installation-in-detail).

## Quick start

| Shortcut                                  | Action                                                                                               |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `SUPER + D`                               | Next mode: hidden → dock → full desktop → hidden                                                     |
| `SUPER + ALT + D`                         | Toggle the dock and keep windows in place                                                            |
| `SUPER + CTRL + D`                        | Hide the desktop and dock                                                                            |
| `SUPER + SHIFT + D`                       | Open Settings                                                                                        |
| `CTRL + F3`                               | Give the keyboard to the dock of the active screen (hidden desktop: it appears under the windows)   |
| `CTRL + F8`                               | Open the notification center and give it the keyboard                                               |
| `CTRL + F6`                               | Give the keyboard to the Settings drawer: it opens if closed, and never closes                       |
| `SUPER + SHIFT + L`                       | Open or close LYRA, the AI assistant                                                                 |
| `SUPER + R`                               | `:f` searches files, `:r` settings, `:clip` the clipboard history, `:2fa`/`:mdp` the myMFA keyring   |
| `ALT + Tab`                               | Switch windows                                                                                       |
| `SUPER + Tab`                             | Workspaces overview: drag windows between workspaces                                                 |
| `Print` / `SUPER + Print` / `ALT + Print` | Capture an area, the screen, or a window                                                             |
| `SUPER + L`                               | Lock the session                                                                                     |
| `SUPER + Escape`                          | Emergency exit                                                                                       |

hyprdesk respects the shortcuts already in `hyprland.lua`. Where an
essential one is missing (workspaces 1 to 10, maximize, pin, next screen…),
it adds it on a free combination. **Settings → Shortcuts** and
`hyprdesk keys` list every active binding with its source line.

## Configuration

Open Settings from the dock, or jump to a page with
`hyprdesk settings screens`. Changes apply live and save to `settings.lua`.
Turn on **Advanced configuration** in the header to see every page.

For a setup you keep in your dotfiles, edit `~/.config/hypr/hyprdesk.lua`.
It ships fully commented, takes precedence over Settings, and is optional.

```lua
hd.wallpaper({ path = "~/.wallpapers/default.jpg" })
hd.bar({ left = { "workspaces", "window" }, right = { "tray", "volume", "session" } })
hd.icon({ app = "firefox" })
hd.widget("clock", { anchor = "top-right", format = "%H:%M" })
```

The [configuration reference](docs/ADVANCED.md#configuration-reference)
lists every key, and [Writing a plugin](docs/WRITING_PLUGINS.md) covers
widgets and bar modules.

### Command line

| Command                                          | Use it to                                                      |
| ------------------------------------------------ | -------------------------------------------------------------- |
| `hyprdesk show` / `launcher-only` / `hide`       | Set the workspace's desktop mode                               |
| `hyprdesk settings [page]`                       | Open Settings, or jump to a page                               |
| `hyprdesk files` / `index query <word>`          | Search files from the dock's loupe, or in the terminal         |
| `hyprdesk panorama [show\|hide\|preview]`        | Drive the Panorama, or try it in a window                      |
| `hyprdesk assistant` / `ask <title> <text>`      | Open LYRA, or start a conversation from a script               |
| `hyprdesk config get\|set\|list\|history\|reset` | Read and change settings, browse snapshots, return to defaults |
| `hyprdesk-theme-engine list` / `use <pack>`      | List theme packs, or switch the theme from a script            |
| `hyprdesk export` / `import <archive>`           | Share or restore a whole setup (`--theme` for the theme only)  |
| `hyprdesk check [file]`                          | Validate a Lua configuration without touching the daemon       |
| `hyprdesk status` / `doctor`                     | Inspect the session, or diagnose missing dependencies          |
| `hyprdesk --help`                                | See every command                                              |

## Theme engine

`hyprdesk-theme-engine` is a small dedicated binary that owns the theme. It
finds the theme packs, works out the full palette, writes each app's files,
reloads the apps, and reports what happened. Settings → Theme, the setup
wizard, `hyprdesk config set theme.*`, and LYRA all go through it: the
active pack is the reference, and the `theme.*` settings mirror it. A theme
set from the command line shows up in Settings, and the other way round.

### What follows the theme

| App | What the engine writes | When it shows |
| --- | --- | --- |
| Hyprland | A generated fragment: borders, gaps, rounding, blur, animations | At once (`hyprctl reload`); `--mode hot` previews without writing |
| hyprdesk, lock and login screens | Nothing: they read the published palette | At once |
| GTK 3 and 4 | Your base GTK theme recoloured to the pack, installed as `hyprdesk-<pack>` | At once in open GTK 3 apps, Brave and Firefox included |
| Qt | `qt6ct/colors/hyprdesk.conf` (and the qt5ct copy); pick it once in qt6ct | When the app restarts |
| kitty, hyprlock | An included file, a sourced fragment | At once, at the next lock |
| Fonts, icons, cursors, sounds | The GNOME settings and the hyprdesk keys | At once; cursors at the next session |
| Any other app | A file declared by the pack (`[targets.<name>]`) | Through a bounded reload: signal, socket, gsettings |

You choose which apps take the theme in **Settings → Theme modules**.
Uninstalling removes everything the engine wrote and puts your base GTK
theme back.

### Packs

Seven packs ship with hyprdesk: **Ardoise** (the default), **Clair**,
**Verre**, **Ambre**, **Encre**, **Forêt**, and **Contraste élevé**. A pack
is a folder with a `theme.toml` manifest and optional templates and
wallpapers. It sets semantic colours (background, surface, text, accent,
success, warning, danger…) for a dark and a light variant, the opacity and
blur of each hyprdesk surface, a few Hyprland options, and the fonts, icons,
cursors, and sounds. Missing colours are derived with readable contrast, and
a pack can inherit from another one.

```toml
[meta]
id = "sauge"
name = "Sauge"
version = "1.0.0"

[inherits]
themes = ["ardoise"]

[palette]
bg = "#18201b"
fg = "#eef3ee"
accent = "#8fbf9a"

[variant.light]
palette.bg = "#f3f6f2"
palette.fg = "#18201b"

[effects.bar]
opacity = 0.80
blur = true
```

Your packs go in `~/.local/share/hyprdesk/themes/<id>/` and win over the
system and bundled ones. With `hyprdesk-theme-engine watch` running, the
active theme follows each save.

**A pack never runs code.** Reloads are limited to a fixed list (signal,
kitty socket, gsettings, hyprctl), Hyprland options come from an allow-list
and fit on one line, and rendered files are checked line by line: no
`exec`, `source`, `bind`, or `env`. A `.hyprtheme` archive is size-bounded,
so a pack from someone else is safe to install. If an app fails to take the
theme, its files and settings roll back; the other apps keep the new one.

### Commands

```bash
hyprdesk-theme-engine list                        # packs, origin, active one
hyprdesk-theme-engine use verre --variant light   # switch theme
hyprdesk-theme-engine use foret --mode dry-run    # see what would change, write nothing
hyprdesk-theme-engine verify                      # check text and accent contrast
hyprdesk-theme-engine recolor --from ~/Images/sea.jpg   # build a pack from an image
hyprdesk-theme-engine install sauge.hyprtheme     # add a pack (archive or folder)
hyprdesk-theme-engine export sauge --out sauge.hyprtheme
hyprdesk-theme-engine palette --format css        # the palette as json, env, css, or hex
```

`HYPRDESK_THEME=clair:light` overrides the active theme for one command,
like `GTK_THEME`. `hyprdesk-theme-engine --help` lists every command, and
the [specification](docs/THEME-ENGINE.md) (French) covers the manifest,
templates, and targets in full.

### Your own apps

After every theme change the engine publishes the resolved palette to
`~/.local/state/hyprdesk/theme-engine/palette.json`, with a revision number
that only moves when the colours do. Apps started from hyprdesk also get
`HYPRDESK_BG`, `HYPRDESK_FG`, `HYPRDESK_ACCENT`, and the other `HYPRDESK_*`
variables. A Rust app can follow the theme live with the
`hyprdesk-theme-client` crate, which needs neither the daemon nor iced.
[Integrating an app with the theme](docs/THEME-CLIENT.md) (French) picks the
right channel for each kind of app.

## Documentation

- [User guide](docs/GUIDE.md): every feature in detail, installer options,
  troubleshooting
- [Panorama configuration](docs/PANORAMA.md) (French)
- [Advanced guide](docs/ADVANCED.md): architecture and configuration
  reference
- [Theme engine](docs/THEME-ENGINE.md) and
  [integrating an app with the theme](docs/THEME-CLIENT.md) (French)
- [Writing a plugin](docs/WRITING_PLUGINS.md) and
  [plugin internals](docs/PLUGINS.en.md)
- [SDK](docs/SDK.md): every way to extend, drive and integrate with
  hyprdesk, and the [command socket](docs/CONTROL.md) reference (French)
- [Changelog](CHANGELOG.md)

## Contributing

For a bug report, include what you expected, what happened, and the shortest
way to reproduce it. **Settings → About** copies the hyprdesk, system,
Hyprland, and LYRA versions for your report.
[Open an issue](https://github.com/jacquesh82/hyprdesk/issues).

Use **Rust 1.99 or newer** and keep builds locked to `Cargo.lock`.

```bash
cargo test --locked --workspace
bash scripts/verify.sh
cargo run --locked -- preview
```

The repository is a Cargo workspace: the `hyprdesk` package (library, main
binary, and the dedicated binaries in `src/bin/`), plus `crates/hyprdesk-auth`
(PAM, fprintd), `crates/hyprdesk-i18n` (translations), and
`crates/hyprdesk-theme-client` (the theme palette for other apps). The
[development guide](docs/ADVANCED.md#development) and the
[contributing guide](CONTRIBUTING.md) cover builds, checks, and pull
requests.

## Credits and license

Copyright © 2026 Jacques HULLU. hyprdesk is free software, released under the
[GNU General Public License, version 3 or any later version](LICENSE)
(GPL-3.0-or-later).

- [Hyprland](https://hyprland.org), the compositor hyprdesk lives in;
- [iced](https://github.com/iced-rs/iced), the GUI toolkit, with patched
  copies of [exwlshelleventloop](https://github.com/waycrate/exwlshelleventloop)'s
  `layershellev`, `iced_layershell`, and `sessionlockev` under `vendor/` (MIT);
- the [Mindlog typeface](https://github.com/jacquesh82/mindlog.fonts),
  bundled under the [SIL Open Font License 1.1](assets/fonts/mindlog/OFL.txt);
- [mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools),
  home of LYRA and of the `hyprpanel-core` Hyprland IPC library;
- the myMFA keyring, from
  [mindlog.mymfa.clients](https://github.com/lelabnet-creator/mindlog.mymfa.clients).

The full dependency list lives in `Cargo.lock`.
