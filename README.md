# hyprdesk

*[Version française](README.fr.md)*

![License](https://img.shields.io/badge/license-MIT-blue)
![Hyprland](https://img.shields.io/badge/Hyprland-%E2%89%A5%200.57-58e1ff)
![Rust](https://img.shields.io/badge/Rust-%E2%89%A5%201.99-dea584)

**An on-demand desktop for [Hyprland](https://hyprland.org), a simple way
to configure it live, and LYRA, the AI built right into the desktop.**

With tiling, windows cover the whole screen and the desktop disappears.
hyprdesk brings it back with a single key: wallpaper, icons and widgets,
with a separate desktop for each workspace if you want one.

It also gives you **one place to set up Hyprland's essentials** (screens,
keyboard and mouse, workspaces, layouts, animations, shortcuts, idle and lock,
login screen,
passwords with the myMFA keyring)
without editing config files by hand. **Every change applies live**, with no reload and no
restart.

And it ships with **LYRA, an AI that lives in your desktop**. One click on the
dock opens her drawer: she already knows your screens, workspaces, shortcuts
and settings, explains what is going on, and fixes it with you. She shows
every command and every change before anything happens. LYRA runs on the
subscription you already have (Claude Code, Codex or Zcode): no API key to
manage. See [LYRA, the AI in your desktop](#lyra-the-ai-in-your-desktop).

<p align="center">
  <img src="assets/screenshots/desktop.png" width="880" alt="hyprdesk desktop: bar on top, icon column, clock, calendar and system widgets, dock at the bottom">
</p>

## Features

- **LYRA, the built-in AI.** A chat drawer one click away on the dock, that
  knows your whole setup. It explains, proposes changes with a live preview
  and runs nothing until you accept. "Ask LYRA" buttons sit wherever
  something needs sorting out (shortcut conflicts, a service held by another
  program, a failed update…). LYRA also powers hyprdesk's other AI features,
  such as SmartUI design.
- **SmartUI design.** Your app icons redrawn by the AI in one consistent
  style (iOS/macOS-like by default, or your own prompt). They form a real
  icon theme, used everywhere: dock, desktop, `SUPER + R`, bar and
  notifications. New apps get their icon automatically.
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
- **Three desktop modes.** `SUPER + D` shows the desktop with the
  workspace's windows floating on top of it, arranged on the screen, or where
  you left them the last time on that workspace. Windows opened from the
  desktop or brought back from the dock float too. When you leave, they
  return to their original tiled layout and their positions are remembered
  per workspace. `SUPER + ALT + D` leaves the windows where they are and shows only
  the dock. A third mode hides the desktop entirely. You can also force a mode
  on a screen or a workspace.
- **Wallpapers for each workspace and screen**, with crossfades and
  slideshows. hyprdesk replaces hyprpaper.
- **A bar and a dock.** Workspaces, tray, volume, brightness and any plugin.
  You rearrange bar modules by dragging them. The dock's Apps, LYRA and
  Settings buttons open and close with one click, light up while open, and
  open on the screen you clicked.
- **One app launcher** (`SUPER + R`, in every mode). On the full desktop it opens as a
  large sheet with categories. In the other modes it opens as a centered
  search card. The fuzzy search finds Firefox when you type "ffx". Your most
  launched apps come first as suggestions, and `:clip` searches your
  clipboard history.
- **Window navigation.** `ALT + Tab` switches windows, and `SUPER + Tab`
  opens a window overview. It uses hyprexpo or Hyprspace if you
  have one, and hyprdesk's own switcher otherwise. While the desktop is shown,
  the number row still switches workspaces, whatever your keyboard layout,
  and `SUPER + Left` / `SUPER + Right` go to the previous / next one.
- **Widgets and Lua plugins.** Clock, calendar, system stats and media player
  come built in. To add your own widget, drop a Lua file in a folder. No
  recompiling. With several screens, each widget goes on one screen (the
  primary one by default), and widgets never overlap.
- **Themes and default apps.** Bundled themes, or themes built from an image,
  and one default application per role (browser, terminal, files…).
- **Session essentials.** Lock screen (with fingerprint support), greetd login
  screen, polkit agent, notifications with history and Do Not Disturb,
  screenshots and screen recording, clipboard history, pending updates.

## Settings

The gear on the dock, `SUPER + SHIFT + D` or `hyprdesk settings <page>` opens
the settings drawer on the right of the screen you clicked (or the active
one), over the desktop, which stays usable next to it. Pages are grouped (Desktop, Appearance, Bar and launcher,
Windows and screens, System, Apps and AI); every change **applies live and
saves on its own**. The header has a guided tour and an "Advanced
configuration" switch that reveals the expert pages. "Ask LYRA" buttons appear
wherever something needs sorting out.

<table>
  <tr>
    <td width="50%" valign="top"><img src="assets/screenshots/settings-desktop.png" alt="Settings → Desktop"><br><sub><b>Desktop</b> — screens shown, desktop mode, shortcuts and their conflicts with hyprland.lua</sub></td>
    <td width="50%" valign="top"><img src="assets/screenshots/settings-launcher.png" alt="Settings → Launcher"><br><sub><b>Launcher</b> — dock windows, grouping, look, pinned apps, SmartUI icons</sub></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="assets/screenshots/screens.png" alt="Settings → Screens"><br><sub><b>Screens</b> — layout on a scaled plan, mode, rotation, scale; reverts after 12 s unless kept</sub></td>
    <td width="50%" valign="top"><img src="assets/screenshots/settings-input.png" alt="Settings → Keyboard and mouse"><br><sub><b>Keyboard and mouse</b> — layouts and variants, key repeat, touchpad, test area</sub></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="assets/screenshots/layouts.png" alt="Settings → Layouts"><br><sub><b>Layouts</b> — Dwindle, Master, Scrolling, Monocle, per workspace</sub></td>
    <td width="50%" valign="top"><img src="assets/screenshots/settings-animations.png" alt="Settings → Animations"><br><sub><b>Animations</b> — Hyprland animations and effects, live</sub></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="assets/screenshots/theme.png" alt="Settings → Theme"><br><sub><b>Theme</b> — bundled themes, themes built from an image, colors and effects</sub></td>
    <td width="50%" valign="top"><img src="assets/screenshots/assistant-settings.png" alt="Settings → AI → Assistant"><br><sub><b>AI → Assistant</b> — LYRA, her subscription and what she knows</sub></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="assets/screenshots/settings-ai-services.png" alt="Settings → AI → Other AI services"><br><sub><b>AI → Other AI services</b> — providers and the provider of each feature; keys in the keyring</sub></td>
    <td width="50%" valign="top"><img src="assets/screenshots/settings-smartui.png" alt="Settings → AI → SmartUI design"><br><sub><b>AI → SmartUI design</b> — icons redrawn by AI, approved or generated automatically</sub></td>
  </tr>
  <tr>
    <td width="50%" valign="top"><img src="assets/screenshots/settings-about.png" alt="Settings → About"><br><sub><b>About</b> — version, system, Hyprland, LYRA; copy for a bug report</sub></td>
  </tr>
</table>

## LYRA, the AI in your desktop

LYRA is the assistant of the Linux desktop: hyprdesk embeds her and gives her
everything she needs to help.

- **One click away.** The LYRA button of the dock (or `hyprdesk assistant`)
  opens her drawer next to your windows, without changing the desktop mode;
  the same click closes it. Several conversations can run side by side.
- **She knows your desktop.** hyprdesk describes its whole environment to her
  (modes, settings, apps, screens, workspaces, widgets, files, commands):
  `hyprdesk context` prints exactly what she receives.
- **Nothing happens behind your back.** Answers come as typed cards: tool runs
  with their result, commands to accept or refuse with their reason and risk,
  and choices with a live preview that is only kept once you accept.
- **Help where you need it.** "Ask LYRA" buttons in Settings open a
  conversation already filled with the problem: a shortcut conflict, a
  service held by another program, a locked keyring, a failed update.
  `hyprdesk ask <title> <text>` does the same from a script.
- **Your subscription, no API key.** LYRA works with Claude Code, Codex or
  Zcode, picked in Settings → AI → Assistant.
- **The AI for the whole desktop.** Other AI features go through LYRA by
  default. Settings → AI → Other AI services also lets you add providers
  (Anthropic, OpenAI, Z.ai, any OpenAI-compatible endpoint such as Ollama) and
  choose one per feature; API keys stay in the session keyring, never in a
  config file.
- **SmartUI design** (Settings → AI → SmartUI design): app icons redrawn by
  the AI in one style. Approved icons form the `hyprdesk-smartui` icon theme
  (in `~/.local/share/icons`, inheriting your theme), used everywhere in
  hyprdesk. Automatic generation is on by default when LYRA is installed.

<p align="center">
  <img src="assets/screenshots/assistant-chat.png" width="760" alt="LYRA's drawer: a suggestion with live preview, and an hyprctl command to accept or refuse">
</p>

LYRA herself (`hyprpanel-chat`) comes from
[mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools).
Without her, hyprdesk works the same; only the AI features stay off.

## Widgets and plugins

Everything below goes on the desktop (`hd.widget("<name>")`), in the bar
(`hd.bar({ right = { "<name>" } })`), or both. You can also turn it on with a
click in Settings → Widgets.

**Several screens.** A desktop widget is shown on one screen only: the
primary one by default. Settings → Widgets → Screen (or `screen = …`) picks
another screen, or all of them, and what happens when that screen is
unplugged: the widget moves to the primary screen, or hides. Widgets never
overlap: one that would cover another slides below it (above it for bottom
anchors), or beside it when the column is full.

```lua
hd.widget("clock",    { screen = "DP-2", if_missing = "primary" }) -- "primary" (default), "all", or a screen name
hd.widget("calendar", { screen = "DP-2", if_missing = "hide" })
```

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

You need Hyprland 0.57 or later and a Vulkan or GL driver. The install
script takes care of the rest: missing distribution packages (C compiler,
pkg-config, PAM headers, Wayland, xkbcommon, Vulkan, EGL… through pacman,
apt, dnf or zypper, with sudo), and Rust 1.99 or later. If the machine's Rust
is older, rustup installs the latest stable in `~/.cargo`, without sudo and
without touching the distribution's Rust. `scripts/deps.sh --dry-run` shows
what is missing; `--no-deps` skips it all.

```bash
git clone https://github.com/jacquesh82/hyprdesk && cd hyprdesk
./scripts/installer.sh    # guided install: language, keyboard, options, then a verified report
```

The installer asks for the language, then checks the basics before changing
anything, and stops with what to do if one fails: not run as root, x86_64 or
aarch64, systemd user manager, Hyprland 0.57 or newer (installed and running;
if it is older, it offers to update it, then you log back in), a known package
manager, 4 GiB free to build, network access to crates.io and GitHub. It then
asks for the keyboard (the current one comes first), then the options to enable: missing prerequisites, start at login,
`SUPER + L` lock, removing hyprpaper, the same keyboard system-wide, and
replacing GDM (or SDDM, LightDM) with the hyprdesk login screen. Nothing
changes before you confirm the summary; `hyprland.lua` is backed up before
each edit. The install then runs in numbered steps with a progress bar,
colours and icons; long commands (build, packages) show a spinner with their
last output line, and their full output goes to
`~/.local/state/hyprdesk/install-<date>.log`. If a step fails, the install
stops and says which step, the last lines of the error, how to fix it, and
that running it again is safe. The optional dependencies step lists what is
missing (idle, screenshots, Bluetooth, login screen…), offers to install
those available in the distribution's repositories (all, none, or by
number; `--optional-deps ask|all|none`), shows the others with their AUR
command, and says what follows: the hyprdesk login screen is only offered
when greetd and cage are present or installable, and the question says it
will install them. It ends with a report checked point by point
(binary, service, Hyprland config without errors, start at login, keyboard,
language, login screen), then **what is left to do** for hyprdesk to run (log
in to Hyprland, log back in after an update, reboot for the login screen…),
each with the command to type, the optional extras (missing optional
packages, `~/.local/bin` in `PATH`), and a verdict: running, installed with
actions left, or not working yet. `--verbose` shows the full output instead
of the log; `NO_COLOR=1` or `HYPRDESK_COLOR=0` turns colours off,
`HYPRDESK_ASCII=1` uses ASCII icons. Without questions: `./scripts/install.sh --help` lists the same
options (`--lang`, `--keyboard fr:azerty`, `--greeter`…).

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
- Settings → About shows the version, the system, Hyprland and LYRA, and
  copies them for a bug report

## Credits

- **[Mindlog](https://github.com/jacquesh82/mindlog.fonts)**: the typeface
  you see everywhere in hyprdesk. It comes bundled, so it works out of the
  box, and it is under [SIL OFL 1.1](assets/fonts/mindlog/OFL.txt).
- **[mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools)**:
  home of LYRA, the AI built into hyprdesk, and of `hyprpanel-core`, the
  Hyprland IPC that hyprdesk is built on.

## License

MIT.
