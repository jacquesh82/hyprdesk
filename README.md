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
  the dock. A third mode hides the desktop entirely. **The mode belongs to the
  workspace**: each screen shows the mode of the workspace it displays,
  changing it only affects that workspace, and every workspace finds its
  mode again when you come back, across sessions (no-desktop until you set
  one). You can also force a mode on a screen or a workspace (Settings → Desktop → Forced
  modes): it applies when
  you arrive there, without switching the other screens.
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

They open by themselves the first time hyprdesk starts. **Autostart** (Apps
group) lists what starts with the session: programs hyprdesk starts (pick an
application or type a command; kept in `autostart.lua`, loaded by
`require("autostart")`), the lines of the `hl.on("hyprland.start", …)` block
of `hyprland.lua`, turned off or back on by commenting them (the lines that
prepare the session stay locked; `hyprland.lua` is backed up first), and the
XDG applications (`~/.config/autostart`). "Run" tries a command right away.

**Desktop icons** (Desktop → Icons) are global, per screen or per workspace;
one click selects, a double click opens (or a single click, as you prefer),
and a right click offers open, pin to the dock, rename, remove from the
desktop (plus parent folder, trash, copy path and wallpaper for files), and
Properties: name, type, location, size, dates, permissions, owner and the
application that opens a file; identifier, `.desktop` file, command and window
class for an application.
Launching from the desktop, Alt+Tab or Escape never change the desktop mode.

**Desktop menus.** Right-click the desktop background to align the icons, add
a widget, change the wallpaper, open the desktop settings or see its
Properties (screen, workspace, mode, wallpaper, theme, icons and widgets); with
Settings open, right-click a widget to remove it, open its settings or see its
Properties. Properties can be copied.

**Full desktop.** Windows float, arranged on the screen; a window you move or
resize keeps its place (per workspace and per application, across sessions)
and is never rearranged again; Desktop → "Forget these positions" clears
them. The bar's mode switch has three states (no-desktop, launcher-only,
full-desktop) and shows the mode of its own screen. Optionally, Hyprbars
(Hyprland plugin, turned on in Settings → Desktop) adds title bars in full
desktop only: loaded as soon as one screen is in full desktop, unloaded when
none is, and hidden on the windows of workspaces that no full-desktop screen
shows, so another screen's tiled windows keep no bar. The bars have close,
maximize and minimize buttons (a minimized window waits in the dock, a click
brings it back; `hyprdesk window minimize|maximize|close`). The installer
builds it with hyprpm (`--no-hyprbars` to skip), and `scripts/update.sh`
rebuilds it when the running Hyprland version changes.

**Screen colours** (Windows and screens → Screen colours): per screen,
brightness, contrast, gamma, and the gain and gamma of each channel (curves),
applied live through a gamma table; a `.cal` calibration file (ArgyllCMS) adds
its curves, an `.icc`/`.icm` profile is handed to Hyprland (`monitors.lua`).
A reference image helps judge; the full-screen test pattern shows it on every
active screen (or gradients spread across all of them), with each screen's
controls on it to compare them (`hyprdesk color-test`, `hyprdesk colors …`,
`hyprdesk colors diag`).

**Laptop lid** (Lock → Sleep): what closing the lid does, on battery, on AC
power and with an external screen: same as the system (logind), suspend,
hibernate, lock, turn off the laptop screen (disabled, its workspaces moving to
the external screen, back when the lid opens) or nothing. hyprdesk then holds a
logind `handle-lid-switch` inhibitor and acts on Hyprland's lid switch
(`hyprdesk lid`); it always locks before suspending.

**Hyprland configuration** (System group) tidies up `hyprland.lua`: errors
reported by Hyprland, duplicate shortcuts, duplicate startups or missing
programs, programs that duplicate what hyprdesk does (wallpaper, bar,
notifications, idle, polkit agent, clipboard), `require` of a missing file,
lines hyprdesk already turned off, old backup copies. Each point comes with
its fix; "Tidy everything" applies the safe ones (after copying
`hyprland.lua`), and LYRA handles the rest. By default, the dock shows the
windows of the current workspace, grouped by application.

**Back to defaults.** Settings → About → "Restore the default configuration"
(or `hyprdesk config reset --yes`) puts back the example `hyprdesk.lua` and
clears every setting made in Settings; a copy of both is kept in
`~/.config/hypr/hyprdesk/sauvegardes/<date>/`. The Hyprland files (keyboard,
screens, autostart), your themes and widgets stay as they are.

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
[mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools). The install script builds and installs her by default
(`--no-lyra` to skip; `HYPRDESK_LYRA_REF` picks the branch), with her
integration service, which starts with hyprdesk.
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

## Installation

### Requirements

- Linux on x86_64 or aarch64, with systemd (a logged-in user session, not
  `su` or `sudo -i`);
- **Hyprland 0.57 or newer**, installed and running (if it is older, the
  install updates it; you then log back in);
- a Vulkan or GL driver;
- a known package manager: pacman (Arch), apt (Debian, Ubuntu), dnf (Fedora)
  or zypper (openSUSE), and `sudo` rights;
- about 4 GiB free to build (8 GiB when LYRA has to be built), and access to
  GitHub and crates.io.

Everything else (compiler, libraries, a recent Rust…) is installed by the
script. There is nothing to prepare by hand.

### Install

```bash
git clone https://github.com/jacquesh82/hyprdesk && cd hyprdesk
./scripts/install.sh
```

In a terminal and without options, the script opens the **setup assistant**
(whiptail windows if present, otherwise questions in the terminal), in 5
steps:

1. **Language** of hyprdesk and of the messages.
2. **Preflight checks**: the requirements above. If one is missing, the
   install stops **before changing anything**, with what to do; if only
   Hyprland is too old, the assistant offers to update it.
3. **Keyboard** of the session (the current one first).
4. **Options** to tick: LYRA, myMFA, missing prerequisites, start at login,
   `SUPER + L` to lock, removing hyprpaper, the same keyboard system-wide, the
   hyprdesk login screen instead of GDM (SDDM, LightDM) — offered only when
   greetd and cage are present or installable.
5. **Summary**: nothing changes before you confirm.

### Install steps

The install then runs in numbered steps with a progress bar. Long commands
show a spinner, their last output line and the elapsed time; their full
output goes to `~/.local/state/hyprdesk/install-<date>.log`.

| Step | What happens |
|---|---|
| 🔎 Preflight checks | the basic criteria, checked before anything is touched |
| 📦 Build prerequisites | missing distribution packages (with `sudo`), Hyprland 0.57+, Rust: if the machine's one is too old, rustup installs the latest stable in `~/.cargo`, without touching the distribution's |
| 🦀 Build | `cargo build --release` (3 to 10 min the first time) |
| 🔤 Font · 📥 Binary · 🛠 Configuration | Mindlog font, `hyprdesk` in `~/.local/bin`, `~/.config/hypr/hyprdesk.lua` if missing (an existing configuration is kept) |
| 🤖 LYRA | the AI assistant, built from [mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools), with her service, which starts with hyprdesk; already installed: kept |
| 🔑 myMFA | the keyring (passwords, 2FA codes): daemon, Secret Service, PAM module that opens the vault at login. "Already installed" means the binary **and** its D-Bus activation; otherwise it is installed again. It is cloned from its **public** GitHub repository ([lelabnet-creator/mindlog.mymfa.clients](https://github.com/lelabnet-creator/mindlog.mymfa.clients)), with the machine's SSH key or a logged-in `gh` as fallbacks, or a local copy (`HYPRDESK_MYMFA_SRC`); the step stays optional (`--no-mymfa`) and is skipped without blocking anything when the repository is unreachable. Each outcome (already installed, access used, unreachable, failure, `--no-mymfa`) goes to the log |
| 🪟 Hyprbars | the title bars of full desktop: `hyprpm`, `cmake`, `meson` and `cpio` if missing, then `hyprpm update` and `hyprpm add` (2 to 5 min, one retry), against the running Hyprland, so only inside a Hyprland session. hyprpm runs in the foreground because it asks for the `sudo` password itself; already built: kept |
| 🌍 Language · ⌨ Keyboard | if chosen: hyprdesk's language, keyboard in `inputs.lua` (and system-wide with `localectl`) |
| 🩺 Optional dependencies | the missing ones (idle, screenshots, Bluetooth, login screen…): those in the repositories are offered (all, none, or by number), the others are shown with their AUR command |
| 🔁 Service | systemd user service, started right away in a Hyprland session |
| 🚀 Start at login | one line in `hl.on("hyprland.start")` of `hyprland.lua`: hyprdesk starts even when the distribution does not activate `graphical-session.target` |
| 🔐 `SUPER + L` · 🖼 hyprpaper | if chosen: lock with hyprdesk; hyprpaper removed from autostart |
| 🚪 Login screen | if chosen: greetd and the hyprdesk screen (the session's keyboard, theme and screens; its logo and wallpaper, landscape or portrait to match the screen) |

`hyprland.lua` is backed up before each change
(`hyprland.lua.avant-hyprdesk-*.bak`). Running the install again is safe:
what is already in place is recognised and left alone.

### At the end

- a **report checked point by point**: binary, configuration, service,
  running Hyprland version, hyprdesk answering, Hyprland configuration
  without errors, start at login, LYRA, myMFA (installed, then answering),
  Hyprbars, keyboard, language, login screen;
- **what is left to do** for hyprdesk to run, each with its command: log in
  to Hyprland, log back in after a Hyprland update, reboot for the login
  screen… then the optional extras (missing packages, `~/.local/bin` in
  `PATH`, a subscription for LYRA, a myMFA vault to create);
- a **verdict**: running, installed with actions left, or not working yet.

On first start, Settings open by themselves. The default configuration shows
the bar at the top, in every mode.

### Without questions

With options, or without a terminal (scripts, containers), the script
installs straight away. `./scripts/install.sh --help` lists them:

| Option | Effect |
|---|---|
| `--defaults` | default choices, no assistant |
| `--lang fr\|en\|auto` | language of hyprdesk and of the messages |
| `--keyboard fr:azerty` | session keyboard (`layout[:variant[:options]]`) |
| `--system-keyboard` | the same keyboard system-wide (TTY, other login screens) |
| `--greeter` / `--no-greeter` | replace GDM with the hyprdesk login screen, or not (otherwise: asked) |
| `--optional-deps ask\|all\|none` | optional dependencies: offered, all installed, or none |
| `--no-lyra` · `--no-mymfa` · `--no-hyprbars` | without LYRA, without myMFA, without the Hyprbars title bars |
| `--no-deps` | do not install prerequisites (by hand; `scripts/deps.sh --dry-run` shows what is missing) |
| `--no-autostart` · `--no-lock-key` · `--remove-hyprpaper` | start at login, `SUPER + L`, hyprpaper |
| `--verbose` | full command output on screen |

Useful variables: `HYPRDESK_LYRA_REF` (LYRA's branch, by default the one with
the `serve` mode the drawer needs), `HYPRDESK_LYRA_SRC` and
`HYPRDESK_MYMFA_SRC` (local copies), `HYPRDESK_INSTALLER_TUI=0` (terminal
assistant even with whiptail), `NO_COLOR=1` or `HYPRDESK_COLOR=0` (no
colours), `HYPRDESK_ASCII=1` (ASCII icons).

### Update

```bash
./scripts/update.sh             # what changed, then the update after confirmation
./scripts/update.sh --check     # the report only (exit code 10: update available)
./scripts/update.sh --yes       # no question
```

The report shows the commits to pull and the areas they touch (code,
scripts, assets, translations, documentation), an installed version behind
the repository, LYRA and myMFA if they changed, and the copy of hyprdesk used
by the login screen. The update pulls the commits (not when there are local
changes: nothing is overwritten), reinstalls with the options of the last
install, rebuilds LYRA and myMFA when needed, and refreshes the login screen.

### Troubleshooting

- A failing step stops the install and says which one, the last lines of the
  error and how to fix it; the full log is in
  `~/.local/state/hyprdesk/install-<date>.log`. Fix it, then run again.
- hyprdesk does not start: `journalctl --user -u hyprdesk -n 50`;
  `hyprdesk doctor` lists the dependencies and what relies on them.
- In Settings, the "Ask LYRA" buttons hand a problem to the assistant (she
  asks before changing anything).
- myMFA missing: check the network access to its public GitHub repository,
  or set `HYPRDESK_MYMFA_SRC=<copy of the sources>`, and run
  `./scripts/install.sh` again; the log says why the step was skipped, and
  Settings → Passwords says the same.
- Hyprbars missing: in a terminal inside Hyprland,
  `hyprpm update && hyprpm add https://github.com/hyprwm/hyprland-plugins`
  (hyprpm asks for the password).
- Back to the previous login screen: `sudo hyprdesk greeter setup restore`.

### Uninstall

```bash
systemctl --user disable --now hyprdesk.service
sudo hyprdesk greeter setup restore      # if the hyprdesk login screen is in place
rm ~/.local/bin/hyprdesk
```

Then remove the lines marked "hyprdesk" from `hyprland.lua` (start at login,
`SUPER + L`, `require("autostart")`), or restore a
`hyprland.lua.avant-hyprdesk-*.bak` copy. Your configuration
(`~/.config/hypr/hyprdesk.lua`, `settings.lua`) stays in place.

## Quick start

Press `SUPER + D`. The default shortcuts are:

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
