# Changelog

All notable changes to hyprdesk are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-13

First release.

### Added

- **Desktop on demand, per workspace.** One key brings back the desktop —
  wallpaper, icons and widgets — under the windows, which float over it
  arranged on the screen or where they were last left, then return to their
  tiled layout when the desktop closes. Each workspace has its own desktop
  and mode, remembered across sessions. Wallpapers per workspace and per
  screen, with slideshows and crossfades; hyprdesk replaces hyprpaper.
- **Bar and dock.** A native bar with its reserved space in every mode:
  workspaces, active window, volume, brightness (external screens over
  DDC/CI), keyboard layout, media player, calendar with month navigation,
  tray with submenus at any depth, night light, pending updates, and any
  plugin in compact form; modules are rearranged by dragging them. A dock
  at the bottom: open windows (grouped by application, restored by a click),
  then the Applications, LYRA and Settings buttons.
- **Launcher.** `SUPER + R` in every mode: fuzzy search over all
  applications, ranked by use, with a full-screen application grid on the
  desktop and a centered card elsewhere. `:clip` searches the clipboard
  history, `:2fa` and `:mdp` the myMFA vault.
- **Lock screen.** `ext-session-lock-v1`, so Hyprland enforces the lock; a
  separate process without Lua, which checks the password through PAM with
  a hand-written binding, in parallel with fprintd fingerprint reading. The
  buffer is wiped and the password never logged. Configurable from Settings
  (wallpaper, contents, idle delays, laptop lid).
- **Login screen (greeter).** A graphical greeter for greetd, speaking its
  IPC protocol, listing the Wayland and X sessions, running under a minimal
  Hyprland or cage. Session selector, demo mode, suspend / restart / shut
  down; installed and configured in place of GDM through `pkexec hyprdesk
  greeter setup`, with a one-step restore.
- **myMFA keyring integration.** hyprdesk is the interface of the myMFA
  Linux client (every tab of its window: Codes, Passwords, Activity,
  Dashboard, Device, Settings, and the QR Scanner): its daemon serves the
  Secret Service
  from the myMFA vault, and the `pam_mymfa` module opens the vault at
  login. Unlock card in hyprdesk's own style, `mymfa` bar module,
  `hyprdesk mymfa` commands, and Settings → Passwords: create the vault or
  install the web backup, import gnome-keyring secrets and take its place,
  with a one-button way back.
- **Hyprbars title bars.** In full-desktop mode only: compiled by hyprpm at
  installation (rebuilt by `update.sh` when Hyprland changes), loaded when
  a screen enters full desktop and unloaded otherwise, with close, maximize
  and minimize buttons; a minimized window waits in the dock.
- **LYRA, the AI assistant in the desktop.** A drawer one click away on the
  dock, that knows the screens, workspaces, shortcuts and settings, explains
  what is going on and fixes it with you, showing every command before it
  runs. Several conversations, visible reasoning, text paste. Runs on an
  existing subscription (Claude Code, Codex or Zcode) — no API key; other
  AI providers are configurable per feature, and SmartUI design redraws the
  application icons in one consistent style.
- **Themes and application theme.** Bundled themes (Ardoise, Glass, Encre,
  Forêt, Ambre, Clair), themes created from an image, effects per location.
  The hyprdesk theme applied to other applications: kitty and Thunar out of
  the box, and a configurable plugin (color file, include line, reload
  command) for anything else.
- **Per-screen colors.** Brightness, contrast, gamma and per-channel gains
  through gamma tables, with `.cal` (ArgyllCMS) and `.icc`/`.icm` profiles
  entrusted to Hyprland; full-screen test pattern across all screens to
  compare them (`hyprdesk color-test`).
- **English interface.** French is the source language and the key; the
  English catalog (`locales/en.toml`) is embedded in the binary, and a test
  keeps it complete. `hd.config({ language = "auto" | "fr" | "en" })`.
- **Guided installer.** `scripts/install.sh`: a five-step wizard (language,
  prerequisites, keyboard, options, summary), numbered steps with progress,
  a verified end-of-install report, nothing modified before confirmation,
  and options to run unattended (`--defaults`, `--lang`, `--no-mymfa`…).
- **Packaging.** DEB, RPM and Arch packages built with nfpm
  (`scripts/package.sh`), with the complete dependency lists picked out of
  the binary and the code.
- **Command line.** One binary, several roles: `daemon`, `show`/`hide`/
  `toggle`/`cycle`, `settings`, `theme`, `wallpaper`, `export`/`import`,
  `config`, `reload`, `status`, `check`, `doctor`, `lock`, `greeter`,
  `preview`, `open`, `context`, `assistant`, `mymfa`, `color-test`,
  `init`, `stubs`, `snippet`.
- **Plugins and widgets.** Plug-and-play widget plugins in Lua: drop a file
  in a folder, no recompiling; catalog and placement in the settings, text
  inputs and image entries in plugin menus, options and commands behind
  “Configure”. Built-in clock, calendar, system stats and media player;
  documented in `docs/PLUGINS.md` and `docs/WRITING_PLUGINS.md`.
- **Settings, applied live.** One drawer, grouped pages, every change
  applied without reload and saved on its own: screens on a scaled plan
  (12-second revert), keyboard and mouse with a test area, workspaces and
  application assignments, layouts (Dwindle, Master, Scrolling, Monocle,
  plus Lua layouts), animations and effects, sound per application, network
  and Bluetooth (Wi-Fi, VPN, airplane mode, pairing), power profiles and
  battery alerts, date and time, autostart (hyprdesk components,
  `hyprland.lua` lines, XDG), default applications, guided tour.
- **Session essentials.** Single instance per session with an exclusive
  lock and a systemd user service; supervised components (`hd.component`)
  restarted when they crash; native Alt+Tab switcher and window overview;
  notifications with history and Do Not Disturb; polkit agent; screenshots
  and screen recording; clipboard history; volume and brightness OSD;
  removable media; idle and sleep through hypridle; accessibility (high
  contrast theme, virtual keyboard).
- **`hyprdesk doctor` and `hyprland.lua` maintenance.** External
  dependencies listed as present or missing with what needs them; cleanup
  of `hyprland.lua` (parse errors, duplicate bindings and autostarts,
  programs that duplicate hyprdesk), each point with its fix and a safe
  “clean all”; setup export/import, in full or theme only.
