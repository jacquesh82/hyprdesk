# hyprdesk — advanced guide

*[Back to the README](../README.md)*

This guide explains how hyprdesk works under the hood and what you can
configure. If you only want to try it out, the [README](../README.md) is enough;
every feature, as seen from Settings, is in the [user guide](GUIDE.md).

- [Architecture](#architecture)
- [The desktop and its three modes](#the-desktop-and-its-three-modes)
- [Talking to Hyprland](#talking-to-hyprland)
- [Keyboard: bindings and the submap](#keyboard-bindings-and-the-submap)
- [Configuration pipeline](#configuration-pipeline)
- [Session root: native and orchestrated services](#session-root-native-and-orchestrated-services)
- [Lock screen and greeter](#lock-screen-and-greeter)
- [Keyring: myMFA](#keyring-mymfa)
- [Indexed file search: `:f`](#indexed-file-search-f)
- [Configuration reference](#configuration-reference)
- [Widget plugins](#widget-plugins) · [Writing a plugin](WRITING_PLUGINS.md)
- [AI assistant](#ai-assistant)
- [Files and environment](#files-and-environment)
- [Development](#development)

## Architecture

hyprdesk is **one main Rust binary**, `hyprdesk`, plus **three dedicated
binaries** installed next to it. The dedicated ones only link their own code:
no daemon, no Lua, no SQLite, no file index.

| Process | Binary | Started by | Role |
|---|---|---|---|
| `hyprdesk daemon` | `hyprdesk` | the systemd user service | draws every surface, runs the Lua config, supervises the session |
| `hyprdesk <command>` | `hyprdesk` | a key binding, a script, you | short-lived client: sends one line to the daemon and exits |
| lock screen | `hyprdesk-lock` | the daemon, logind, hypridle, the laptop lid | a **separate** process that runs no Lua |
| login screen | `hyprdesk-greeter` | greetd | before any user session exists |
| polkit agent | `hyprdesk-polkit` | the daemon (supervised component) | authentication dialogs, no Lua |

`hyprdesk lock`, `hyprdesk greeter` and `hyprdesk polkit-ui` are still valid
commands, so bindings and configs written earlier keep working: they hand
over to the dedicated binary when it sits next to `hyprdesk`, and otherwise
run the same code themselves. The code is a library (`src/lib.rs`) with four
programs (`src/main.rs`, `src/bin/`). PAM and fprintd live in the
`hyprdesk-auth` crate, and the translation catalog in `hyprdesk-i18n`
(`crates/`), neither of which depends on iced or Lua.

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
`iced_sessionlock` 0.19 (with `sessionlockev`), [mlua](https://github.com/mlua-rs/mlua) (Lua 5.4,
vendored), [zbus](https://github.com/dbus2/zbus) and `hyprpanel-core` for the
Hyprland IPC.

## The desktop and its three modes

hyprdesk creates **one layer-shell surface per screen**:

| Surface | Layer | Purpose |
|---|---|---|
| Desktop | `background` | wallpaper at rest; icons and widgets when shown |
| Bar | `top`, with an exclusive zone | Hyprland tiles windows beside it, and the desktop grid shifts by the same amount |
| Dock / launcher | its own layer | open windows, pinned apps, settings button |
| Panorama | its own surface, on a strip screen or a zone | swiped pages of clocks, player, gauges and modules; never takes the keyboard (see the [user guide](GUIDE.md#panorama)) |
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
to return to. The SUPER + Tab switcher filters it out too.

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

- **`binds.lua` is held by hyprdesk.** At startup, the `hl.bind` lines of
  `hyprland.lua` move to `binds.lua`, loaded with `require("binds")`,
  together with the comments above them and the locals they use (a copy is
  kept, and everything is put back if Hyprland reports a new error). A
  hand-written `binds.lua` is adopted once, after a copy. From then on, every
  change in Settings → Shortcuts, every agreement with hyprdesk and every
  tidy-up from `hyprland.lua` regenerates the file from what it **really
  defines**, evaluated as Hyprland would: variables replaced by their value,
  loops unrolled, one binding per line, the home folder written as a `HOME`
  local (`os.getenv("HOME")`), so the file moves between machines.
  Section comments, commented-out code and disabled bindings are kept, and
  an action written as a Lua function keeps its locals. Generation is
  idempotent, and Hyprland checks every write. `hyprdesk binds generate`
  previews it and `--write` applies it.
- hyprdesk's own shortcuts (`keys.*`) and the **essentials** are set live,
  and reinstalled after every `hyprctl reload`. hyprdesk **never overrides**
  a binding that `hyprland.lua` or `binds.lua` already owns. An essential
  (workspaces, window actions, screens, `Print` captures…) only lands on a
  combination your config leaves free, and each one can be turned off in
  Settings → Shortcuts. After a reset, an essential whose original tool comes
  back is only switched off when hyprdesk's equivalent is already written,
  and a binding written twice is removed.
- While the desktop is shown with no window on it (an empty workspace),
  hyprdesk enters a **submap**, so `SUPER + Q`, `SUPER + F` and the like do
  nothing behind it. As soon as windows float on the desktop, your usual
  bindings act on them again. Bindings
  marked `{ submap_universal = true }` in `hyprland.lua` (typically workspace
  switching and screenshots) stay active, and the desktop follows them.
- The submap still lets you move around: the **number row** goes to
  workspaces 1 to 10, with the same modifier as your workspace bindings
  (`SUPER` by default). It matches physical key codes, so it works with
  AZERTY, QWERTY and any other layout. The left and right arrows, with
  that modifier, go to the previous and next workspace. The desktop
  follows, and the new workspace's windows float on it too. `keys.search` (or
  `keys.search_desktop`), `keys.switch` and `keys.overview` also stay
  available.
- **Submap and your mouse bindings.** Essentials that make sense on the
  desktop (the `SUPER` + drag of windows included) are set again inside the
  submap, on the combination `hyprland.lua` uses, so they keep working while
  the desktop is shown.
- **Shortcut conflicts.** hyprdesk compares its shortcuts with Hyprland's
  live bindings and with `hyprland.lua` and `binds.lua` (line and command of
  each `hl.bind`). Two hyprdesk actions on one combination: the second is not
  installed. A combination bound in your config: hyprdesk's action only
  works while the desktop is shown. A combination marked
  `submap_universal`: hyprdesk leaves it alone, so nothing fires twice
  (media keys included). A combination bound in Hyprland but in none of
  these files is reported as "set live, `hyprctl reload` removes it".
- **Conflicts asked as a question.** For each hyprdesk shortcut already
  bound in your config, a window asks once: take it for hyprdesk
  (`keys.take`), keep the existing one (hyprdesk then only has it while the
  desktop is shown), or capture another combination. Answers are remembered
  in `hyprdesk/.raccourcis-repondus` (key, combination, and the command that
  holds it), so an answered conflict is not asked again, but a new one is,
  including after an update or when another tool takes the combination.
  "Later" asks again at the next start; installing and resetting clear the
  answers. Settings → Shortcuts → hyprdesk shortcuts lists conflicts first,
  with a LYRA button; `hyprdesk keys` prints the same report and exits with
  1 if there is a conflict.
- **SUPER + Tab on the full desktop** lists the windows floating on it. Picking
  a window from another workspace hides the desktop and focuses it. A window opened from the
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
 ~/.config/hypr/hyprdesk.lua            ← optional, yours, never rewritten, has the last word
              │  executed in this order on a dedicated Lua thread
              ▼
       configuration model  →  resolution per (screen, workspace)  →  surfaces
```

- **Two files, one order.** `settings.lua` is plain generated Lua, and it is
  the only source of truth for the Settings drawer, which re-reads it by
  executing it. It loads **before** your `hyprdesk.lua`, so anything you write
  by hand wins. The drawer and `hyprdesk config` flag keys that your file
  overrides.
- **`hyprdesk.lua` is optional.** Everything can be set in Settings. On a
  first install, hyprdesk writes the starting desktop (icons, widgets, a bar
  at the top, five permanent workspaces) into `settings.lua`, in the form
  Settings uses, and drops an example `hyprdesk.lua` whose lines are all
  commented out. You can delete it: hyprdesk loads without it as long as
  `settings.lua` exists, and recreates neither file once one of them is
  there.
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
- **Hyprland files written by hyprdesk.** What Settings holds for Hyprland is
  written to small files next to `hyprland.lua`, loaded by a block of
  `require` at its end so they win over what comes before: `env.lua`,
  `appearance.lua` (gaps, borders, decoration, blur, groups),
  `animations.lua`, `layouts.lua`, `rules.lua` (window, layer and workspace
  rules) and `system.lua` (behaviour, XWayland, rendering, and the
  `hl.permission` lines, read by Hyprland at startup only), plus
  `binds.lua`, `inputs.lua` (its generated *Advanced* block holds the raw
  `input`/`gestures` options, the `hl.gesture` bindings and the per-device
  `hl.device` blocks), `monitors.lua` and `autostart.lua`. They only contain
  the values set in hyprdesk; an incomplete entry (a rule without a target,
  a device without a name, a gesture without an action) stays in Settings
  but is not written. Every write goes through Hyprland's check: a new
  config error puts everything back.
- **Taking over `hyprland.lua`.** Options, curves, animations, `hl.env`,
  window and layer rules, `hl.gesture`, `hl.device` and `hl.permission`
  calls found in `hyprland.lua` are imported into hyprdesk and commented out
  there. Computed values (a gesture whose action is a Lua function, an
  `os.getenv` path…) stay in `hyprland.lua` and the matching Settings page
  says on which lines.
- **Hyprland plugins.** Settings → Hyprland configuration → Hyprland
  plugins reads `hyprpm list` and `hyprctl plugin list`, enables or disables
  a plugin (`hyprpm enable|disable`, then `hyprpm reload -n`), and adds
  `hyprpm reload -n` to `autostart.lua` when enabled plugins would not be
  reloaded at login. Updating and adding a repository build Hyprland's
  headers with administrator rights, so they open in a terminal. Hyprbars
  stays loaded by hyprdesk itself (Desktop → Session).
- **Default configuration.** Each Settings component (one settings package)
  can go back to its default with **Defaults…** or `hyprdesk config reset
  <component> [--yes]` (without `--yes`, it only prints the plan). Its
  fields in `settings.lua` return to the starting desktop, the Hyprland
  files it holds are rewritten (`binds.lua` with nothing turned off,
  `inputs.lua` at Hyprland's values with the keyboard layout kept,
  `monitors.lua` in preferred mode, an empty `autostart.lua`), and its
  remembered state is forgotten (icon positions, desktop modes, window
  places, launch history). `all` does every component, and also clears kept
  notifications and sets custom themes and widgets aside. `hyprdesk.lua` is
  never touched. Replaced files are copied first to
  `~/.config/hypr/hyprdesk/sauvegardes/<date>/`.

## Session root: native and orchestrated services

hyprdesk follows a hybrid approach: it is **native for what you see** and
**orchestrates what you don't**. Each service is taken only if no one else
already holds it, so your existing setup stays in charge.

| Service | How | If another one is running |
|---|---|---|
| Wallpaper | native (replaces hyprpaper) | — |
| Notifications | native `org.freedesktop.Notifications` 1.2 daemon (bubbles, history, Do Not Disturb) | name taken with `DoNotQueue`: mako, dunst, swaync… keep it |
| Tray | native StatusNotifierItem host, `dbusmenu` menus rendered natively | — |
| polkit agent | native, speaks to `polkit-agent-helper-1`, in its own process without Lua (`hyprdesk-polkit`, supervised by the daemon) | the existing agent stays |
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

**`hyprdesk-lock`** (or `hyprdesk lock`, which hands over to it) is designed
so that a plugin can neither block it nor unlock it:

- it uses `ext-session-lock-v1`: **Hyprland** enforces the lock, so if the
  process crashes, the session stays locked. If Hyprland refuses the lock or
  ends it (`finished`), the lock process exits instead of waiting forever
  with its instance lock held: hyprdesk carries a fixed copy of
  `sessionlockev` for that (`vendor/sessionlockev/HYPRDESK.md`);
- it moves itself into its **own systemd scope**
  (`hyprdesk-lock-<pid>.scope`) before anything else. Started by hypridle,
  it would otherwise live in the cgroup of `hyprdesk.service`, and an
  update or a restart of the service would kill it, leaving Hyprland locked
  with no screen to unlock it. Without a systemd user instance, it stays
  where it is: locking comes first;
- it is a separate binary with **no Lua**. It reads values the daemon has
  already resolved from `$XDG_RUNTIME_DIR/hyprdesk/lock.json`, and falls back
  to safe defaults if the file is missing;
- it checks the password through PAM (a small hand-written binding) on a
  separate thread: `pam_authenticate`, then the account phase
  (`pam_acct_mgmt`: expired, disabled or `pam_access`-denied accounts do not
  unlock). The PAM service is `hyprdesk-lock` (shipped by the packages:
  system auth stack plus account phase), else `hyprlock`, else `login`.
  After three failures, input is blocked for a growing delay (2 s, 4 s, 8 s…
  up to 30 s). The buffer is wiped with `zeroize`, the password is never
  logged, and pasting is disabled;
- fingerprint (`lock_fingerprint`, **off by default**) replaces the password
  when enabled: a recognised finger unlocks on its own. If the administrator
  provides `/etc/pam.d/hyprdesk-fingerprint` (with `pam_fprintd`), the
  fingerprint goes through that PAM service; otherwise fprintd is read over
  D-Bus and the account phase of the lock service still decides. Every
  fingerprint unlock is logged (authpriv). Settings → Lock enrolls and
  deletes fingerprints through fprintd, pass by pass, with the password
  asked by polkit;
- only one instance runs, and nothing can unlock it from outside. If the
  instance lock file cannot be taken, it locks anyway and exits with an
  error. It never falls back to `/tmp`: `$XDG_RUNTIME_DIR` (or
  `/run/user/<uid>`) must exist, belong to the user and be mode 0700. The
  content drifts slowly and fades out to avoid burn-in.

**Workstation policy.** `/etc/hyprdesk/policy.toml` (owned by root, not
writable by others, otherwise ignored) overrides user settings:
`fingerprint = false` forbids the fingerprint; `idle_lock_max = 600` forces
idle management and locks after at most that many seconds ("never" refused).
Idle locking is on by default (5 min); an hypridle started outside hyprdesk
keeps control unless the policy imposes it.

**`hyprdesk greeter`** is a login screen for
[greetd](https://sr.ht/~kennylevinsen/greetd/). It speaks greetd's IPC
(length-prefixed JSON on `$GREETD_SOCK`) and lists the `wayland-sessions` and
`xsessions` entries. It runs under the compositor greetd starts, for example
a minimal Hyprland that hyprdesk writes to
`/etc/greetd/hyprdesk-hyprland.lua`. It uses your screen layout, keyboard and
no animations. The greeter shows on one screen, chosen in Settings (the
primary screen by default), and the other screens stay black. When the
session opens, the greeter closes, Hyprland exits and greetd starts the
session through your login shell (`$SHELL -l`), so `~/.profile` and
`~/.bash_profile` are read, as with GDM. Without Hyprland, cage is used as a
fallback: `cage -s -- hyprdesk greeter --window`, where `--window` opens a
regular full-screen window because cage has no layer-shell. A background can
be set for landscape screens and another one for a rotated (vertical) login
screen. Without the myMFA module, `/etc/pam.d/greetd` gets
`pam_gnome_keyring` so the keyring is unlocked at login, as with GDM. `--tty` runs it in a terminal and `--demo` runs it
without greetd. It offers suspend, restart and shut down.

**Settings → Login screen** sets it up in place of GDM (or SDDM, LightDM…).
It shows the login manager in use and the pieces greetd needs, then does
each system step through `pkexec hyprdesk greeter setup <action>`:

- `packages` installs greetd and cage;
- `install` makes greetd start a hyprdesk **installed as root**:
  `/usr/bin/hyprdesk` from the package, else `/usr/local/bin/hyprdesk` put
  there by `scripts/install.sh --from-source`, never the binary in
  `~/.local/bin`, which the `greeter` user cannot read and you could
  replace. greetd runs the `hyprdesk-greeter` next to it when it passes the
  same check (owned by root, neither it nor its folder writable by others),
  else `hyprdesk greeter`. `pkexec` also only runs that root-owned binary,
  so a system action new in a local build stays inactive until it is
  installed as root. It then writes `/etc/greetd/config.toml`. A file
  from another tool is kept once as `config.toml.bak`. It also installs the
  screen profile `/etc/greetd/hyprdesk.json`, taken from your session: the
  keyboard layout is passed to cage (`XKB_DEFAULT_*`), so the password is not
  typed in QWERTY, along with the theme colors. It adds its own lines to
  `/etc/pam.d/greetd`: fingerprint (`pam_fprintd`, opt-in, 10 s before the
  password) and the myMFA keyring (`pam_mymfa`, when the myMFA Linux client is
  installed). It creates `/var/lib/hyprdesk-greeter`, where the screen
  remembers the last user and session;
- `enable` disables the current manager and enables greetd. This happens at
  the next boot, so the open session is not cut off. It is refused while a
  piece is missing;
- `restore` goes back to the previous manager. From a console:
  `sudo hyprdesk greeter setup restore`.

## Keyring: myMFA

hyprdesk is the interface of the myMFA Linux client
(`mindlog.mymfa.clients/linux`), a keyring that replaces gnome-keyring. The
`mymfa` daemon serves the Secret Service to applications from the myMFA vault
(the same vault as the web and Android apps) and publishes
`today.mindlog.MyMfa1` on the session bus. hyprdesk listens to it:

- when an application needs a secret and the vault is locked, hyprdesk shows
  an unlock card in its own style (the daemon's fallback pinentry is turned
  off by a `mymfa.service.d/hyprdesk.conf` drop-in);
- in the launcher, `:2fa` lists the current 2FA codes and `:mdp` the site
  logins. Enter copies the code or the password; the daemon does the copy
  itself and clears a password after 30 s. With the vault locked, Enter opens
  the unlock card;
- the `mymfa` bar module shows whether the vault is locked or unlocked;
- **Settings → Passwords** creates the vault or installs the web backup,
  locks and unlocks it, and replaces gnome-keyring: its secrets are imported,
  then its units are masked and myMFA takes the name. Going back is one
  button, and the gnome-keyring files are never touched.

`hyprdesk mymfa unlock|lock|codes|logins` does the same from a shortcut. At
login, the hyprdesk greeter adds `pam_mymfa` to `/etc/pam.d/greetd` when the
module is installed, alongside `pam_gnome_keyring` while gnome-keyring is
still in use. The first unlock after such a login seals the vault password
with the session key, and after that the vault opens on its own.

## Indexed file search: `:f`

Linux filesystems have no central table of files, so hyprdesk builds one —
the way `plocate` does: an inverted index of trigrams over file names,
served by a small daemon and kept fresh. In the launcher, `:f` (or
`:fichiers`) searches it — typing answers in a few milliseconds, Enter opens
the file with its default application. The dock's loupe opens it too (in the
Graphite bar), and carries the index state as a small mark: accent when
ready, amber while the first scan runs, nothing when the daemon is down
(`hyprdesk files` does the same from a shortcut). A query with a `/`
searches paths (`docs/rapport`), a plain one searches names. Every result
carries its name, type (file, folder, link), path and size.

- **Scope**: the home directory and mounted volumes (`/media`,
  `/run/media/$USER` — USB drives, Windows NTFS partitions) by default,
  one filesystem per volume; the Settings screen edits roots graphically
  (add/remove rows), or `index.paths` from Lua/CLI. Build artifacts and
  caches are skipped (`.git`, `node_modules`, `target`, `.cache`…),
  `.gitignore`/`.ignore` files are respected inside repos, and hidden files
  are indexed. Exclusions also accept **regular expressions** applied to the
  full path (`index.exclude_regex`, validated on entry), **excluded
  extensions** (`index.exclude_ext`, e.g. `iso, vdi`) and a **maximum
  indexed size** per file (`index.max_size`, MiB).
- **Freshness**: inotify watches every volume it can; when a volume refuses
  watching (watch limit, FUSE quirks) a periodic rescan covers it
  (`index.interval`, 6 h by default). The trigram index survives restarts as
  a compressed snapshot in `~/.cache/hyprdesk/index/`, and unloads from RAM
  after 10 idle minutes.
- **Daemon**: the launcher starts `hyprdesk index daemon` on the first `:f`
  query with the resolved settings; or enable `hyprdesk-index.service`
  (shipped in `assets/`) to index at login. `hyprdesk index query <word>
  [--json]`, `index status` and `index rebuild` drive it from a terminal —
  and agents (LYRA, MCP clients) see those commands in `hyprdesk context`.
  The Settings screen shows the snapshot's location and size, and the
  search view filters results by kind (all / files / folders / links).
- **Cost**: about 150 bytes of RAM per indexed file — a large home
  (~1 M files) holds ~150 MB while active, and the snapshot compresses to a
  few tens of MB. While the first scan runs, `:f` reports its progress
  instead of lying about results.

## Configuration reference

The whole API is described in LuaLS definitions (`hyprdesk stubs` writes
`hyprdesk.meta.lua`), which give you autocompletion next to Hyprland's
`hl.meta.lua`. Every rule accepts the `workspace` and `monitor` selectors: a
name, a number, a list, or `"*"`.

### `hd.config({ … })`

| Area | Keys |
|---|---|
| Desktop | `mode` (`"opaque"`/`"glass"`), `glass_opacity`, `background`, `show_on` (`"all"`/`"focused"`/`"primary"`), `primary`, `float_launched`, `focus` (`mode` `"hyprland"`/`"click"`/`"mouse"`, `threshold`, `swallow_click`, `on_activate`, `on_close` `"next"`/`"cursor"`/`"recent"` — full desktop only), `icon_scope` (`"global"`/`"screen"`/`"workspace"`), `icon_double_click`, `forced_modes`, `desktop_files` (Desktop folder and Trash as icons) |
| Grid and icons | `grid = { cell, gap, margin, icon_size }`, `icon_theme` |
| Look | `theme = { name, fg, bg, accent, effects = { bar, menus, launcher, settings, widgets } }` (each effect: `opacity`, `blur`), `font`, `font_size`, `app_themes` (applications that follow hyprdesk's theme, `kitty`…: all of them by default, `{}` for none) |
| Motion | `transition` (`"fade"`/`"slide"`/`"none"`), `fade_ms`, `crossfade_ms`, `fx` (Hyprland animations, curves and decoration, as `hl.config` keys) |
| Dock | `running_apps` (`"off"`/`"workspace"`/`"all"`), `launcher_group` (`"none"`/`"app"`/`"workspace"`) |
| App launcher | `apps_unified` (`true` by default: `keys.search` opens hyprdesk's launcher in every mode, taken from `hyprland.lua` if it binds that key; `false`: `hyprland.lua` keeps it outside the full desktop), `apps_style` (`"spotlight"` by default: centered card in every mode; `"sheet"`: always the large sheet; `"auto"`: large sheet on the full desktop, centered card otherwise), `apps_suggestions` (0 to 16 most launched apps, 8 by default), `apps_categories`, `terminal` |
| Workspaces and layouts | `workspaces` (1 to 30, or `false`), `workspace_monitors`, `layout` (`dwindle`, `master`, `scrolling`, `monocle`, `grid`, `columns`, `rows`, `centered`, `lua:<name>`), `layout_workspaces`, `dwindle`, `master`, `scrolling` |
| Keys | `keys = { toggle, launcher, hide, settings, lock, leave, switch, search, search_desktop, overview, media, take }` (`false` disables a key; `search` is `SUPER + R`; `search_desktop` replaces it on the full desktop; `take` lists `hyprland.lua` combinations hyprdesk unbinds and uses — given back when removed), `suspend_binds`, `overview_command` (a Hyprland plugin's overview, such as hyprexpo or Hyprspace; otherwise hyprdesk's switcher, which stays open until you click, press Enter or Escape) |
| Session | `autostart`, `clipboard_history`, `index_paths` (roots of the `:f` file index; default: home and mounted volumes), `index_exclude` (folder names skipped, on top of the defaults), `index_interval` (safety rescan, seconds), `index_autostart` (index daemon started with hyprdesk; `false`: on the first `:f`), `idle_manage`, `idle_dim`, `idle_lock`, `idle_dpms`, `idle_suspend`, `idle_wake_volume` (screens turned off by `hyprdesk screens-off` come back on when the volume changes), `lock_fingerprint`, `update_commands` |
| Screens | `screen_zones` (screens split into zones), `screen_colors` (brightness, contrast, gamma and channels of each screen), `panorama` (strip screens and their pages: see [`PANORAMA.md`](PANORAMA.md), in French) |
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
-- Graphite integrates the launcher in the center zone; other widgets default
-- to the right. Explicit zones and drag-and-drop placements are preserved. Defaults: bottom, height 48, margin 0, rectangular surface.
-- The launcher plugin owns pinned/running apps and actions; no duplicate dock.
-- Explicit position/height/margin remain configurable. Legacy 40/10 slab presets
-- are migrated when their old window/workspaces layout is detected.
hd.bar({ monitor = "primary", position = "top", compact = "auto", margin = 0,
         left = { "workspaces", "window" }, center = { { "clock", format = "%a %d %H:%M" } },
         right = { "tray", "volume", "brightness", "network", "session", "controls" },
         portrait = { left = { "workspaces" }, right = { "clock" } } })

-- Bar colours: a fixed colour, a "dark/light" pair, or the name of a slot of
-- the active theme pack's palette (it follows light/dark mode on its own).
-- CLI equivalent: hyprdesk config set bar.color.accent active
hd.bar({ colors = { accent = "active", background = "#1c2028/#f5f7fa" },
         module_colors = { clock = "#2dd4bf/#006b5f", volume = "focus" } })

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
4. bundled plugins, compiled into the binary: `network`, `bluetooth`,
   `battery`, `capture`, `keyboard`, `player`, `updates`, `removable`, `nightlight`,
   `vkbd`, `notifications`, `session`

```lua
return {
    name = "VPN", description = "VPN status", interval = 5, anchor = "top-right",
    options = { interface = "wg0" },                -- defaults, editable in Settings → Plugins → Installed → Configure
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
| `~/.config/hypr/hyprdesk/themes/` | "Custom themes" (theme archives) |
| `~/.config/hypr/hyprdesk/sauvegardes/<date>/` | copies made before a reset to defaults |
| `~/.config/hypr/hyprdesk/.raccourcis-repondus` | answers to shortcut conflicts |
| `~/.config/hypr/monitors.lua` | screen layout, written on **Keep** (`require("monitors")` in `hyprland.lua`) |
| `~/.config/hypr/inputs.lua` | keyboard, mouse and touchpad settings (`require("inputs")` in `hyprland.lua`) |
| `~/.config/hypr/binds.lua` | your bindings, held and regenerated by hyprdesk (`require("binds")`) |
| `~/.config/hypr/autostart.lua` | programs started by hyprdesk at login (`require("autostart")`) |
| `~/.config/hypr/{env,appearance,animations,layouts,rules,system}.lua` | Hyprland settings made in hyprdesk, required at the end of `hyprland.lua` |
| `$XDG_STATE_HOME/hyprdesk/origine/` | the machine's state before hyprdesk, saved once by `install.sh` |
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

**Sharing a setup.** `hyprdesk export [file]` (or Settings → Sharing)
writes a `.tar.gz` with a `manifest.json`. It carries the whole setup:
`hyprdesk.lua`, `settings.lua`, third-party plugins, "Custom themes" and screen
calibrations; `hyprland.lua`, each local file it `require`s (followed in the
text, nothing is run), the files hyprdesk holds (`binds.lua`, `inputs.lua`,
`monitors.lua`, `autostart.lua`, `appearance.lua`…) and the files of the
config folder they cite, such as scripts; the remembered state (icon
positions, desktop mode of each workspace); the wallpapers in use and every
other file the settings point to (icons, lock image…). AI services and
their keys, passwords, light pairing and system settings (sound, network…)
are left out. `--theme` exports only what you see: colors, effects,
wallpapers, desktop mode, lock veil and image, transitions, font, icon theme
and theme modules. `hyprdesk import` backs up the current setup first, and
`--dry-run` shows what would change. Imported Hyprland files are checked by
Hyprland and count as code that needs your consent; `monitors.lua` is only
applied with `--screens`, and paths into the home folder are rewritten for
the new machine.

**Back to the original state.** `install.sh` saves the machine's state once
before touching anything, and `scripts/uninstall.sh --restore` puts it back
(`scripts/origine.sh` does the work). Details are in the
[user guide](GUIDE.md#uninstalling-and-restoring).

## Development

```bash
cargo test --locked --workspace     # Lua config, resolution, placement, IPC, .desktop, i18n catalog
cargo run --locked -- preview       # isolated preview with demo data: no IPC, no persistence, no launches
cargo run --locked -- preview out/  # same, and capture every view into out/
bash scripts/verify.sh              # format, strict Clippy, tests, build, isolated D-Bus
bash scripts/verify.sh --visual     # adds screenshots at 1440, 900 and 640 px
bash scripts/package.sh             # .deb, .rpm and Arch packages (nfpm), built with the `dist` profile
```

- The repository is a Cargo workspace: the `hyprdesk` package (library and
  four binaries) and the `crates/hyprdesk-auth` and `crates/hyprdesk-i18n`
  crates. `release` strips symbols; `dist` adds full LTO and a single
  codegen unit for packages.

- `Cargo.lock` pins `winit-core` to `0.31.0-beta.2`, because beta.4 breaks
  `iced_exdevtools`, a dependency of `iced_layershell` 0.19. Always build
  with `--locked`.
- Drag and drop onto the desktop needs the Wayland data device, which
  `layershellev` 0.19.1 lacks, and `sessionlockev` 0.19.1 ignores the end
  of a refused lock. hyprdesk carries patched local copies in `vendor/`
  (see each `vendor/*/HYPRDESK.md`); `scripts/verifier-vendor.sh` checks
  them against the published archives.
- Interface strings: French is the source language and serves as the key, as
  in gettext, and `locales/en.toml` holds the English catalog. The
  `catalogue_complet` test fails if any marked string has no translation.
- Visual language and motion rules: [`DESIGN.md`](../DESIGN.md) (in French).
  The default interface font is
  [Mindlog](https://github.com/jacquesh82/mindlog.fonts), embedded in the
  binary, so no Fontconfig cache is needed. Five weights, from Light to Bold.
