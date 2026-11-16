# Changelog

All notable changes to hyprdesk are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **Drawers open at once** (MLG-156). Opening the LYRA or Settings drawer
  could freeze the desktop for up to 4 s: iced re-read every system font for
  each new surface (its SVG cache is per renderer). The SVG font database is
  now shared by the whole process and loaded only for an SVG that contains
  text; icons no longer need it. Local copies `vendor/iced_wgpu` and
  `vendor/iced_tiny_skia`, checked by `scripts/verifier-vendor.sh`.

## [0.1.5] - 2026-11-16

Security hardening release (MLG-150 to MLG-155).

### Security

- **Imported archives never run their Lua unchecked** (MLG-150). **Check**
  (`--dry-run`) only compiles the archive's Lua; the import reads its
  settings in a sandbox (no files, no commands, no loading outside the
  archive, memory capped). A theme carries no code. A full setup whose
  `hyprdesk.lua` or plugins would run with your rights needs an explicit
  consent (toggle in **Sharing**, `--trust-code`). AI services declared by an
  archive are ignored. Links and special files are refused.
- **Third-party plugins are reviewed before installation** (MLG-154): they
  land in a quarantine outside the plugin folders; a review card shows the
  source, the commit, the SHA-256 and the start of the code. Git sources must
  be `https://` or ssh; `#<commit>` pins a revision.
- **Verifiable releases** (MLG-151): `SHA256SUMS`, its keyless cosign
  signature, a CycloneDX SBOM and a build provenance attestation are
  published with each release; `install.sh` installs the repository's version
  and checks checksum (and signature with cosign) before any `sudo`. CI
  actions pinned by SHA, read-only token by default, release gated on the
  checks, `cargo deny` and Dependabot. `update.sh` no longer sources remote
  code; rustup is checksum-verified; LYRA and myMFA are pinned by commit.
- **Nothing user-writable runs as root** (MLG-152): root steps and `pkexec`
  only run `/usr/bin/hyprdesk` or a root-owned `/usr/local/bin/hyprdesk`.
- **Lock and login** (MLG-153): fingerprint unlock is off by default and
  labelled "instead of the password" (through a `hyprdesk-fingerprint` PAM
  service when present, otherwise followed by the PAM account phase, and
  logged); PAM account phase after the password, growing delay after three
  failures; `/etc/pam.d/hyprdesk-lock` shipped by the packages; idle locking
  on by default (5 min); the lid only suspends once the lock holds; no `/tmp`
  fallback; passwords zeroized in memory; safer greeter restore and atomic
  writes of `/etc/pam.d/greetd` and `config.toml`.
- **polkit agent in its own process** (MLG-153): `hyprdesk polkit-ui`,
  without Lua, supervised by the daemon; the request details (command,
  program) can be unfolded and only polkit itself may call the agent.
- **Sensitive data** (MLG-154, MLG-155): the 2FA QR capture is decoded in
  memory (no file); clipboard history is off by default, text only, 100
  items, with a **Clear** button; SmartUI automatic icons are opt-in and name
  their AI provider; plain `http://` AI endpoints only towards localhost; a
  failed update opens a window with **Ask LYRA** instead of reaching LYRA on
  its own, with the log framed as untrusted data and credentials masked; AUR
  packages are no longer built in one click without review (`yay -Sua`).
- **Private runtime files**: state files, sockets and the Wi-Fi key file live
  in a checked private runtime folder (never a shared `/tmp`); the file index
  is stored 0600 in a 0700 folder and its daemon reads requests bounded.
- **Machine policy**: `/etc/hyprdesk/policy.toml` (root-owned) can forbid
  fingerprint unlock, third-party plugins, removable-media indexing and
  clipboard history, and enforce a maximum idle-lock delay.

### Added

- Lua API: `hd.runtime_dir`, `hd.write_private(path, data)`.
- `SECURITY.md`: response times, release verification, trust model.

## [0.1.4] - 2026-11-16

### Added

- **Screens: drag to place** (MLG-147): screens move with the mouse on the
  Settings → Screens plan, snapping to the edges and centres of their
  neighbours (Shift: no snapping), with arrow buttons for a pixel-exact
  adjustment (Shift: 10 px). Overlapping positions are refused.
- **Identify shows alignment guides** (MLG-147): each screen gets a band on
  every edge it shares with a neighbour (with the segment's ends), and grid
  lines at the same global coordinates on all screens, so misalignment is
  visible across the seam. Click-through; Identify again removes them.
- **Advanced settings marked in cyan** (MLG-148): a dot in the menu for
  advanced packages and tabs, an "Advanced" badge on their pages, a cyan
  frame around advanced option sections.
- **About updated** (MLG-149): renderer in use, lines taken over from
  `hyprland.lua`, history; the Lua files of `~/.config/hypr` (generated or
  yours) with Open folder and History buttons; What's new.
- **Setup wizard** (MLG-145). A large card over the dimmed desktop walks
  through ten visual steps: language, primary screen (proportional layout
  map), keyboard layout (keycap cards and a try-it field), theme cards,
  wallpaper thumbnails and theme-coloured borders, bar style (Continuité,
  Graphite, Island, Convex, with captures) and position, permanent
  workspaces, opaque or glass desktop and Desktop-folder icons, performance
  profiles and renderer, an idle timeline (dim → lock → screens off → sleep)
  that warns when the session would lock after the screens turn off, default
  terminal, browser and file manager, then a summary. Each choice goes
  through the existing settings and applies at once; a history snapshot
  (“Setup assistant”) taken on opening backs **Undo the wizard**. It opens
  on the first launch instead of Settings (the guided tour stays offered
  there), from the **Assistant** button in the Settings header, and with
  `hyprdesk wizard`.
- **Animations → Curves** (MLG-143): your curves (taken over from
  `hyprland.lua` or created here) drawn and editable — Bézier control
  points or spring mass, stiffness and damping — next to hyprdesk's
  catalogue; they are written to `animations.lua` and offered for every
  animated element. A curve in use cannot be deleted.

- **hyprdesk writes the Hyprland settings it manages** (MLG-137, MLG-139).
  Effects, animations, curves, layouts, borders, rules, environment and the
  new options below are written to generated files, one per domain
  (`appearance.lua`, `animations.lua`, `layouts.lua`, `rules.lua`,
  `env.lua`, `system.lua`, plus an advanced block in `inputs.lua`), loaded at
  the end of `hyprland.lua`. They used to be applied live only, so
  `hyprland.lua` showed values that were not running. At startup (and with
  `hyprdesk hyprland-adopt [--dry-run]`) the lines of `hyprland.lua` on those
  domains are taken over: values imported, lines commented with
  `-- [hyprdesk]`; computed values (`os.getenv`…) stay. Every write is
  checked by reloading Hyprland and rolled back on a new config error, and
  the history keeps the previous state.
- **Settings → Windows** (MLG-141): gaps (one value or four sides), border
  size and gradients, borders in theme colours, resize on border, snapping,
  window rules editor (class/title, floating, pinned, opacity, border
  colour, size, idle inhibit, workspace), tabbed groups and their bar.
- **Keyboard and mouse → Cursor, Gestures, Advanced** (MLG-142): cursor
  theme and size (applied at once), hiding, warping, zoom; workspace swipe;
  no acceleration, scroll method and speed, touchpad drag options.
- **Configuration → Environment, Behaviour, Advanced** (MLG-144):
  environment variables, `misc` options (DPMS wake, focus, swallowing,
  logo, session lock restore…), binds options, XWayland, rendering,
  ecosystem and debug.
- One renderer for every Hyprland option in Settings, telling where a value
  comes from: set here, the `hyprland.lua` line, or Hyprland's default.

- **Settings → Performance → hyprdesk engine.** How hyprdesk itself draws and
  runs, applied on restart (a "Restart hyprdesk" button appears when the
  settings differ from the running process):
  - **Rendering engine**: hardware (wgpu, Vulkan or OpenGL, falling back to
    software when no card is usable) or software (tiny-skia, CPU). The page
    shows the engine actually obtained and warns about a silent fallback.
  - **Graphics card**: power-saving (integrated, default) or powerful
    (discrete). iced asked for the most powerful card by default, which wakes
    a hybrid laptop's discrete GPU just to draw a bar.
  - **Worker threads**: 4 by default instead of one per core (iced's tokio
    executor started 20 idle threads on a 20-core CPU).
  CLI and MCP keys: `renderer`, `gpu`, `worker_threads`. External
  `ICED_BACKEND` / `WGPU_POWER_PREF` still win and are shown as forced; the
  daemon no longer passes them on to the processes it starts.
- **Configuration history with rollback** (MLG-138). Before every write to a
  configuration file (`hyprland.lua`, `binds.lua`, `autostart.lua`,
  `monitors.lua`, `inputs.lua`, `hyprdesk.lua`, `hyprdesk/settings.lua` and
  any top-level `*.lua` generated by hyprdesk), hyprdesk snapshots them under
  `$XDG_STATE_HOME/hyprdesk/history/`, tagged with the reason (Settings,
  tidy-up, shortcuts, autostart, reset, import, command line); the daemon also
  snapshots manual edits. Identical states are not stored again, a series of
  writes with the same reason within 60 s keeps a single "before" snapshot,
  and snapshots older than 30 days are pruned (the newest is always kept).
  **Settings → Hyprland configuration → History** lists them by day, shows
  which files changed and an excerpt of the diff against the current state,
  and reverts with a two-step confirmation; Hyprland is reloaded and the
  rollback is undone if it reports a new error. CLI:
  `hyprdesk config history [--json]`, `hyprdesk config diff <id>`,
  `hyprdesk config rollback <id|YYYY-MM-DD[THH:MM]> [--yes]`.

- **Settings → Performance.** A new package lists the Hyprland 0.57 options
  that cost CPU or GPU time — blur (passes, xray, popups, special workspace),
  shadow, glow, motion blur, wobble, animations, animated resizes and drags,
  VFR, direct scanout, new render scheduling — each with its cost and what
  each profile does with it. Three priority profiles set them at once:
  **Performance** (no blur, shadow or animation), **Balanced** (light blur,
  costliest effects off) and **Visual quality** (every effect, finer blur).
  Profiles write the same settings as the Animations package, so any option
  can still be adjusted afterwards; "Back to hyprland.lua" removes them all.
  CLI: `hyprdesk config set perf.profile performance|equilibre|rendu|hyprland`.
  Without Hyprland blur (Performance profile, a setting here or
  hyprland.lua), the Settings drawer turns opaque so it stays readable.

- **Bar styles Island and Convex** (inspired by
  [minflair](https://github.com/t4lentles5/minflair), GPL-3.0). **Island**
  shows a single centered black capsule (112 × 28 px, margin 8), empty at
  rest like the supplied reference. Hover reveals the configured modules;
  live notifications temporarily expand the center. Transparent sides pass
  clicks through to the desktop. **Convex** carves the edge into a full-width
  slab with three rounded notches joined by 4 px bezier bridges, following
  the measured zones. Both share the existing modules, panels, overflow and
  multi-monitor machinery; only the skin changes. The strip now reports the
  measured zone extents (`Msg::BarLayout`) and the style paints its zone
  background behind them. New preview scenes `bar-island-*` and
  `bar-convex-*`.

### Changed

- **Settings navigation reorganised** for the many new settings: ten groups,
  one topic each — Desktop; Bar and launcher; Appearance (themes,
  animations, effects); Windows (windows, workspaces, layouts); Screens;
  Keyboard and mouse (with Shortcuts); Sound and network; System;
  Applications; Configuration. **History** is now its own entry
  (Configuration → History) instead of a tab of the Hyprland configuration,
  and the Advanced tab of Hyprland configuration shows with advanced
  settings only. Advanced Hyprland options no longer sit in catch-all tabs:
  they appear, with advanced settings on, on the page where one looks for
  them — mouse and touchpad options under Mouse and Touchpad, workspace
  navigation under Workspaces, shortcut timing under Shortcuts, wake and
  lock restore under Lock → Idle, logo and default wallpaper under
  Wallpaper, advanced blur under Animations → Effects, focus, fullscreen and
  swallowing in a new Windows → Behaviour tab.

- **Readable `settings.lua`** (MLG-146): settings grouped in commented
  sections (Desktop, Appearance, Shortcuts, Launcher, Hyprland, Performance,
  Lock and idle, Applications and AI, File index, Session) and long tables
  spread over several indented lines.

- **Config consistency fixes** (MLG-140):
  - `binds.lua` agrees with what hyprdesk handles: combinations hyprdesk
    takes (unified launcher, media and capture keys, `keys.take`) are
    disabled there with `-- [hyprdesk off]`, and shortcuts that start a tool
    hyprdesk replaces (clipboard picker, window switcher, power menu,
    hyprshot) now call hyprdesk, the old line kept as a comment
    (`hyprdesk binds sync [--dry-run]`, also at startup). New command
    `hyprdesk clipboard` opens the search on `:clip`.
  - Tidy-up of `hyprland.lua` finds commented-out old screens and workspace
    rules that contradict the running config, stale `hyprdmc` mentions,
    redundant `systemctl --user import-environment`, and config files
    nothing reads any more (`hypridle.conf`, `hyprpaper.conf`, archived to
    `~/.local/state/hyprdesk/archive`). Fixes go through the history and a
    checked Hyprland reload.
  - Idle: a warning (Lock → Idle, tidy-up) when the screens turn off before
    the session locks; the delays are left as set.
  - Settings → Workspaces explains a workspace assigned beyond the permanent
    ones; Mouse warns when “No acceleration” overrides the profile; every
    Settings entry has its icon.

- **Idle and background cost** (MLG-131 to MLG-136, measured with pidstat):
  - A message now redraws only the surfaces it touches: a video frame redraws
    the desktops showing it, unchanged readings (volume, tray, widgets, index
    state) and clock polls within the same minute redraw nothing. Every
    surface used to be redrawn and presented after any message.
  - The video wallpaper stops decoding while nothing shows it (session
    locked, screen off, fullscreen window on top) and resumes afterwards.
  - Polling waits while the session is locked or every screen is off.
  - Hyprland events are read in bursts: v1/v2 duplicates and the events of
    one gesture trigger a single read of each state, and windows and their
    geometries come from one `clients` query.
  - Volume follows PipeWire events (`pactl subscribe`) instead of running
    `wpctl` every 2 s.
  - The clipboard writes through `wl-copy`; smithay-clipboard, whose thread
    woke up with every Wayland event (≈ 44 times/s), is only connected when
    wl-clipboard is missing. The screen-colours thread sleeps until asked
    instead of waking every 120 ms. Notification bubbles tick only while one
    can expire; the file index state is sent only when it changes.

- **Settings navigation.** Performance moves from Appearance to System, next
  to Energy, and splits into two tabs: *Hyprland effects* (profiles and
  options) and *hyprdesk engine*.

- **Convex motion rendering.** Retained widgets now animate only their host at the compositor's frame cadence instead of rebuilding every desktop on a 16 ms timer. Drawer and dock springs retain velocity when reversing. Icons react to pointer proximity and presses; controls receive local hover feedback. Menus, notifications and dock tooltips share the local reveal driver, and ordinary pointer movement no longer rebuilds the desktop. Text stays at a constant raster size during transitions.

- **Convex interaction and performance.** Auto-hide dock with an immediate edge reveal, independent left/right drawers available on hover, dark opaque defaults, and a notification drawer revealed downward from the top center. Surface opening animations now start after presentation instead of being consumed by window creation. `convex.auto_hide` is configurable in Settings/Lua/CLI; hover transitions use a short close grace period and stop drawing at rest.

- **Convex reference animation.** Continuous screen frame with rounded inner corners, attached dock, side drawers and a launcher revealed from the bottom edge. The frame passes pointer events through to windows. Settings → Bar, Lua (`bar.convex`) and CLI expose frame thickness, corner radius, animation duration and attached panels. Zero duration and reduced motion are immediate; canvases cache their idle geometry. The selected bar height scales the notches; empty side groups no longer stretch a lone clock across the screen.

- **License.** hyprdesk is now distributed under the GNU General Public
  License, version 3 or any later version (GPL-3.0-or-later), instead of
  MIT. Releases up to 0.1.3 remain available under MIT. The vendored
  `layershellev` and `iced_layershell` stay under their MIT license.

## [0.1.3] - 2026-11-15

### Added

- **Video badge.** Settings → Wallpaper marks video wallpapers with a
  small play badge on their thumbnail.

### Fixed

- **Video wallpapers no longer flicker.** Each frame is split into strips
  under 2 MiB that iced uploads synchronously, so no empty frame shows
  between two images — on the desktop, the lock screen, and the login
  screen without mpv. Frames are capped to the source resolution.
- **“Browse” opens a folder picker.** The wallpaper folder is chosen
  through the XDG desktop portal (`org.freedesktop.portal.FileChooser`),
  which shows the active implementation's dialog (GTK, GNOME, KDE…).

### Changed

- **README.** The demo is now the final 70-second film (GIF, linking to the
  video with sound), and every screenshot was retaken from the current
  interface.

## [0.1.2] - 2026-11-15

### Added

- **Video wallpapers.** A rule takes an image or a looping video
  (mp4, m4v, mkv, webm, mov, avi) — desktop, lock screen, and login screen
  alike. Decoded by ffmpeg into RGBA frames at a refresh-aligned pace
  (the largest frame rate that divides the screen's refresh rate, so it
  never beats), looped seamlessly, restarted on failure; scaled, cropped and
  blurred per rule, spanning screens like images do. The login screen goes
  further: its background video is played by mpv on the GPU when present
  (transparent greeter on top, buttery motion at any refresh rate), a
  bundled loop ships as the default, and the login form can sit
  on any of nine screen anchors (Settings shows the grid; demo overrides
  `HYPRDESK_GREETER_VIDEO` and `HYPRDESK_GREETER_ANCHOR`).
- **Settings.** The wallpaper and login-screen pickers list videos with
  thumbnails (first frame), the login page warns when mpv/ffmpeg are
  missing, and `hyprdesk doctor` reports both.

## [0.1.1] - 2026-11-14

### Added

- **Indexed file search.** An inverted trigram index of every file on the
  home and mounted volumes (ext4, btrfs, xfs, mounted NTFS/FAT), served by a
  background daemon: instant answers in the launcher (`:f`, or the dock's
  loupe, or `hyprdesk files`), kept fresh by inotify with a periodic rescan
  fallback, persisted as a compressed snapshot and unloaded after ten idle
  minutes. Results carry name, type, path and size; filters by kind (files,
  folders, links) and by category (documents, images, videos, programs,
  audio, archives); Enter opens with the default application.
- **File index settings.** A dedicated Settings screen: graphically edited
  roots, exclusions by name or regular expression (validated on entry),
  ignored extensions and categories, maximum indexed size per file, safety
  rescan interval, and the snapshot's location and size.
- **Dock loupe.** In the Graphite bar, the loupe opens the file search and
  carries the index state as a small mark (accent: ready, amber: scanning,
  none: down); hover says the file count.
- **Installers.** `scripts/install.sh` installs the latest published release
  as a prebuilt package (deb, rpm, or pkg.tar.zst) instead of building from
  source; `scripts/install_nightly_build.sh` builds this repository as it
  stands (last commit, or the working tree) — outside releases;
  `scripts/update.sh` follows published releases.
- **Assistant and agents.** `hyprdesk context` documents
  `hyprdesk index query <word> [--json]`, `status` and `rebuild`, so LYRA
  and MCP agents can search the index.

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
