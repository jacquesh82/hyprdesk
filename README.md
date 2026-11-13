# hyprdesk

[Version française](README.fr.md)

[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
![Hyprland 0.57+](https://img.shields.io/badge/Hyprland-0.57%2B-58e1ff)
![Rust 1.99+](https://img.shields.io/badge/Rust-1.99%2B-dea584)

**A desktop you can bring up whenever you need it in [Hyprland](https://hyprland.org).**

When your tiled windows fill the screen, press `SUPER + D` to reach your
wallpaper, shortcuts, and widgets. Your windows float above the desktop while
you use it and return to their tiled layout when you leave. Each workspace
remembers how you left it.

Use the settings drawer to arrange your monitors, change your keyboard layout,
or adjust shortcuts and see the result as you go. For help with a shortcut
conflict or a setting you’re unfamiliar with, open LYRA, the optional desktop
assistant, from the dock.

[Install](#install) · [Quick start](#quick-start) · [Configuration](#configuration) · [LYRA](#lyra) · [Contributing](#contributing)

![hyprdesk desktop with a top bar, app shortcuts, clock, calendar, system widgets, and a dock](assets/screenshots/desktop.png)

## Install

Run the installer from a terminal in your Hyprland session. It checks your
system and shows the selected options before making changes.

| Requirement | Details |
| --- | --- |
| System | Linux on x86_64 or aarch64, systemd, a regular user session, and `sudo` access |
| Desktop | Hyprland 0.57+ and a Vulkan or GL driver |
| Package manager | pacman (Arch), apt (Debian/Ubuntu), dnf (Fedora), or zypper (openSUSE) |
| Build | About 4 GiB of free space, or 8 GiB if building LYRA, plus access to GitHub and crates.io |

```bash
git clone https://github.com/jacquesh82/hyprdesk
cd hyprdesk
./scripts/install.sh
```

The setup assistant lets you choose a language and keyboard layout, review
dependencies, and decide which integrations to enable. If your Hyprland
version is older than 0.57, the assistant offers to update it. Log back into
Hyprland after that update.

The installer builds hyprdesk, installs it in `~/.local/bin`, and sets up its
systemd user service. It creates `~/.config/hypr/hyprdesk.lua` if needed and
backs up `hyprland.lua` before editing it
(`hyprland.lua.avant-hyprdesk-*.bak`). The final report shows which components
are running and any actions still needed. Settings open on the first launch.

<details>
<summary>Installer options and integrations</summary>

```bash
./scripts/install.sh --help
```

Passing options or running without a terminal skips the setup assistant.
The installer can install missing build dependencies and use rustup in
`~/.cargo` if the system Rust is too old.

| Option | What it controls |
| --- | --- |
| `--defaults` | Use the default choices without the setup assistant |
| `--lang fr\|en\|auto` | Choose the interface and installer language |
| `--keyboard fr:azerty` | Set the session keyboard using `layout[:variant[:options]]` |
| `--system-keyboard` | Apply the same layout system-wide |
| `--greeter` / `--no-greeter` | Enable or skip the greetd login screen, which uses cage |
| `--optional-deps ask\|all\|none` | Choose how to handle optional dependencies |
| `--no-lyra` | Skip the LYRA assistant |
| `--no-mymfa` | Skip the myMFA keyring, Secret Service integration, and PAM module |
| `--no-hyprbars` | Skip the Hyprbars title-bar plugin |
| `--no-deps` | Manage prerequisites yourself. Check them with `scripts/deps.sh --dry-run` |
| `--no-autostart` | Skip the Hyprland startup entry |
| `--no-lock-key` | Skip the `SUPER + L` lock shortcut |
| `--remove-hyprpaper` | Remove hyprpaper from autostart |
| `--verbose` | Show full command output in the terminal |

- **LYRA** is built from [mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools). Set `HYPRDESK_LYRA_SRC` to use a local checkout or `HYPRDESK_LYRA_REF` to select the branch.
- **myMFA** provides a keyring for passwords and 2FA codes. Set `HYPRDESK_MYMFA_SRC` to use a local checkout of [mindlog.mymfa.clients](https://github.com/lelabnet-creator/mindlog.mymfa.clients). If the installer cannot access the repository, it logs the reason and continues.
- **Hyprbars** adds minimize, maximize, and close buttons in full desktop mode. The installer builds it with `hyprpm` against the running Hyprland version. The updater rebuilds it when that version changes.

Set `HYPRDESK_INSTALLER_TUI=0` to use terminal prompts in place of whiptail.
`NO_COLOR=1` or `HYPRDESK_COLOR=0` disables color, and `HYPRDESK_ASCII=1`
selects ASCII icons.

</details>

## Quick start

| Shortcut | Action |
| --- | --- |
| `SUPER + D` | Toggle the full desktop. Windows float above the wallpaper, icons, and widgets |
| `SUPER + ALT + D` | Toggle the dock and keep windows in place |
| `SUPER + CTRL + D` | Hide the desktop and dock |
| `SUPER + SHIFT + D` | Open Settings |
| `SUPER + R` | Search apps in any mode. Use `:clip` to search clipboard history |
| `ALT + Tab` | Switch windows |
| `SUPER + Tab` | Open the window overview with hyprexpo, Hyprspace, or hyprdesk’s own switcher |
| `SUPER + Escape` | Emergency exit |

Desktop modes are saved per workspace across sessions. With multiple monitors,
each screen follows the workspace it displays. Use **Settings → Desktop →
Forced modes** to set a mode for a specific screen or workspace. Windows you
move or resize in full desktop mode keep their positions for that application
and workspace. Clear those positions in **Settings → Desktop**.

**Settings → Desktop → Shortcuts** and `hyprdesk keys` show existing bindings
and their source lines in `hyprland.lua`. hyprdesk respects existing bindings
unless you assign them to hyprdesk or leave its default `SUPER + R` override
enabled.

<details>
<summary>Keyboard navigation in the dock</summary>

Open the dock in launcher-only mode or press Tab from the full desktop to
focus it. Use the arrow keys, Tab, and Shift + Tab to move between items, Home
and End to jump to either end, and Enter or Space to activate an item. Escape
closes the dock or returns focus to the full desktop.

</details>

## What you can do

- **Make the desktop yours.** Set wallpapers and slideshows per screen or
  workspace, add app and file shortcuts, and build a theme from an image. Drag
  modules to rearrange the bar, or use the dock to group windows by app.
- **Tune Hyprland from Settings.** Arrange monitors, set keyboard and touchpad
  options, pick a layout per workspace (Dwindle, Master, Scrolling, or Monocle),
  and adjust animations, shortcuts, and idle delays. Display layout changes
  revert after 12 seconds unless you keep them.
- **Set up your session.** Configure autostart, laptop lid behavior, the lock
  screen with fingerprint support, notifications, screenshots, and screen
  recording. The installer also offers a greetd login screen and a myMFA keyring.
- **Add widgets in Lua.** Start with the clock, calendar, system stats, or media
  player, then add your own widgets and bar modules as Lua files.

| Monitor settings | Widgets |
| --- | --- |
| ![Monitor arrangement and display settings](assets/screenshots/screens.png) | ![Widget settings with clock, calendar, and system stats on the desktop](assets/screenshots/widgets.png) |

<details>
<summary>More screenshots</summary>

| Desktop and dock | Settings and appearance |
| --- | --- |
| ![Launcher-only mode](assets/screenshots/mode-launcher.png) | ![Desktop settings](assets/screenshots/settings-desktop.png) |
| ![Hidden desktop mode](assets/screenshots/mode-hidden.png) | ![Keyboard and mouse settings](assets/screenshots/settings-input.png) |
| ![Bar settings](assets/screenshots/bar.png) | ![Workspace layouts](assets/screenshots/layouts.png) |
| ![App launcher](assets/screenshots/launcher.png) | ![Animation settings](assets/screenshots/settings-animations.png) |
| ![Desktop icons](assets/screenshots/icons.png) | ![Dock settings](assets/screenshots/settings-launcher.png) |
| ![Wallpaper settings](assets/screenshots/wallpaper.png) | ![Theme settings](assets/screenshots/theme.png) |
| ![Desktop transitions and wallpaper crossfade](assets/screenshots/effects.png) | ![About and system information](assets/screenshots/settings-about.png) |

</details>

## Configuration

Open Settings from the dock or run `hyprdesk settings screens` to go straight
to the monitor page. Changes in the drawer apply live and save automatically to
`settings.lua`. Enable **Advanced configuration** in the header to see more
pages.

For a setup you can keep in your dotfiles, edit `~/.config/hypr/hyprdesk.lua`.

```lua
hd.wallpaper({ path = "~/.wallpapers/default.jpg" })
hd.bar({ left = { "workspaces", "window" }, right = { "tray", "volume", "session" } })
hd.icon({ app = "firefox" })
hd.widget("clock", { anchor = "top-right", format = "%H:%M" })
```

Your Lua configuration takes precedence over settings saved by the drawer.
Both files reload when saved. If a configuration contains an error, hyprdesk
reports the file and line and keeps using the last working configuration.
See the [configuration reference](docs/ADVANCED.md#configuration-reference).

<details>
<summary>Session controls and recovery</summary>

- **Autostart** lists programs managed by hyprdesk, eligible entries in the
  `hl.on("hyprland.start", …)` block of `hyprland.lua`, and XDG autostart
  entries. hyprdesk-managed entries go in `autostart.lua`, loaded with
  `require("autostart")`. Use **Run** to try a command.
- **Screen colours** adjusts brightness, contrast, gamma, and individual color
  channels. It accepts ArgyllCMS `.cal` curves and passes `.icc` or `.icm`
  profiles to Hyprland via `monitors.lua`. Run `hyprdesk color-test` for a full-screen test
  pattern or `hyprdesk colors diag` to diagnose gamma-table conflicts. The
  settings page can stop a program holding the gamma tables, with a separate
  option to also disable its user service or XDG autostart entry.
- **Laptop lid** lets you choose what happens on battery, on AC power, or with
  an external monitor attached. Available actions include using the system
  default, suspending, hibernating, locking, turning off the laptop display,
  or ignoring the lid event. hyprdesk locks before suspending.
- **Hyprland configuration** flags errors, duplicate bindings or startup
  entries, and programs that overlap with built-in services. **Tidy
  everything** backs up `hyprland.lua` and applies the fixes marked safe.
- **Restore the default configuration** in **Settings → About**, or run
  `hyprdesk config reset --yes`, to restore the sample `hyprdesk.lua` and clear
  the settings saved in the drawer. Both files are backed up under
  `~/.config/hypr/hyprdesk/sauvegardes/<date>/`. Hyprland files, themes, and
  widgets are outside the scope of the reset.

</details>

## LYRA

Open LYRA from the dock or run `hyprdesk assistant`. The assistant can explain a
setting or help work through an issue using context from your session, such as
your monitors, workspaces, shortcuts, and apps. Run `hyprdesk context` to see
what hyprdesk provides to LYRA.

When LYRA proposes a command or a change, you can review and approve it in the
drawer. Some changes include a live preview that you can accept or discard.
**Ask LYRA** buttons open a conversation with the relevant details already
filled in. Scripts can do the same with `hyprdesk ask <title> <text>`.

Choose Claude Code, Codex, or Zcode in **Settings → AI → Assistant**. LYRA uses
your existing CLI subscription. You can also select providers for other AI
features in **Settings → AI → Other AI services**, including Anthropic, OpenAI,
Z.ai, and OpenAI-compatible endpoints such as Ollama. API keys for those
providers are stored in the session keyring.

**SmartUI design** redraws app icons in a consistent style. Approved icons form
the `hyprdesk-smartui` icon theme under `~/.local/share/icons`, used across the
desktop, dock, app launcher, bar, and notifications. Automatic generation is
enabled by default when LYRA is installed.

![LYRA’s drawer showing a live preview and an approval prompt for a command](assets/screenshots/assistant-chat.png)

<details>
<summary>AI settings screenshots</summary>

| Assistant | Other AI services | SmartUI design |
| --- | --- | --- |
| ![Assistant and CLI subscription settings](assets/screenshots/assistant-settings.png) | ![Provider settings](assets/screenshots/settings-ai-services.png) | ![SmartUI icon generation settings](assets/screenshots/settings-smartui.png) |

</details>

## Widgets and plugins

Enable widgets in **Settings → Widgets** or add them to your Lua configuration.
The built-in widgets are `clock`, `calendar`, `system`, `media` (MPRIS),
`command` (periodic shell output), and `lua` (output of an inline function).
Desktop widgets default to the primary screen. You can target a specific
output or all outputs and choose whether a widget moves or hides when a
screen is unplugged.

```lua
hd.widget("clock", { screen = "DP-2", if_missing = "primary" })
hd.widget("calendar", { screen = "DP-2", if_missing = "hide" })
```

The bar includes `workspaces`, `window`, `volume`, `brightness`, `tray`,
and `desktop`. To add a Lua widget or plugin, see [Writing a plugin](docs/WRITING_PLUGINS.md).

<details>
<summary>Bundled plugins and their dependencies</summary>

| Plugin | What it does | Relies on |
| --- | --- | --- |
| `reseau` | Wired and wireless networks, VPN, WireGuard, airplane mode | NetworkManager |
| `bluetooth` | Scan, pair, and manage devices | bluetoothctl |
| `batterie` | Battery status, alerts, and power profiles | sysfs, power-profiles-daemon |
| `capture` | Screenshots and screen recording | hyprshot, wf-recorder |
| `clavier` | View and switch keyboard layouts | Hyprland |
| `lecteur` | Now playing and playback controls | MPRIS |
| `majs` | Pending updates and upgrades | pacman/AUR, apt, dnf, zypper, flatpak |
| `supports` | Open and eject removable drives | udisks2 |
| `nuit` | Night light on demand or on a schedule | hyprsunset |
| `vkbd` | On-screen keyboard | wvkbd |
| `notifications` | Recent notifications and Do Not Disturb | hyprdesk or wayle |
| `session` | Lock, suspend, log out, reboot, and power off | systemd, hyprdesk lock |

The installer places two editable examples, `meteo` and `notes`, in
`~/.config/hypr/hyprdesk/widgets/`.

</details>

## Command line

| Command | Use it to |
| --- | --- |
| `hyprdesk show` / `hyprdesk launcher-only` / `hyprdesk hide` | Set the workspace’s desktop mode |
| `hyprdesk settings [package]` | Open Settings or jump to a page |
| `hyprdesk theme new <image>` | Build a theme from an image |
| `hyprdesk wallpaper split <image>` | Split a scene across your monitors |
| `hyprdesk export [file]` / `hyprdesk import <archive>` | Share or restore a setup. Add `--theme` for just the theme or `--screens` to include display settings |
| `hyprdesk check [file]` | Validate a Lua configuration without touching the running daemon |
| `hyprdesk status` / `hyprdesk doctor` | Inspect the current state or diagnose missing dependencies |
| `hyprdesk --help` | See the complete command list and options |

## Update and troubleshoot

From the cloned repository, run `./scripts/update.sh` to review and apply
updates. The updater reuses your installer options and stops if the checkout
has local changes. Use `--check` for the report alone (exit code `10` means
an update is available), or `--yes` to apply without prompting.

- **Install failed.** See `~/.local/state/hyprdesk/install-<date>.log` for the
  failed step and its full output, then rerun the installer after addressing it.
- **hyprdesk won’t start.** Run `journalctl --user -u hyprdesk -n 50` and
  `hyprdesk doctor`.
- **myMFA is missing.** Check **Settings → Passwords** and the install log for
  the cause. If the installer cannot access the myMFA repository, rerun it with
  `HYPRDESK_MYMFA_SRC` pointing to a local checkout.
- **Hyprbars is missing.** From a terminal inside Hyprland, run
  `hyprpm update`, then
  `hyprpm add https://github.com/hyprwm/hyprland-plugins`.
- **Restore the previous login screen.** Run `sudo hyprdesk greeter setup restore`.

<details>
<summary>Uninstall hyprdesk</summary>

If you enabled the hyprdesk login screen, restore the previous one with
`sudo hyprdesk greeter setup restore` before removing the binary.

```bash
systemctl --user disable --now hyprdesk.service
rm ~/.local/bin/hyprdesk
```

Remove the hyprdesk-managed entries from `hyprland.lua` or restore the
appropriate `hyprland.lua.avant-hyprdesk-*.bak` backup. You can keep your
`hyprdesk.lua` and `settings.lua` for a later install.

</details>

## Contributing

For a bug report, include what you expected, what happened, and the shortest
way to reproduce it. **Settings → About** can copy the hyprdesk, system,
Hyprland, and LYRA version information for your report. [Open an issue](https://github.com/jacquesh82/hyprdesk/issues).

Use **Rust 1.99 or newer** and keep builds locked to `Cargo.lock`. The lockfile
pins `winit-core` to `0.31.0-beta.2` for compatibility with the UI stack.

```bash
cargo test --locked
bash scripts/verify.sh
cargo run --locked -- preview
```

The verification script runs formatting checks, strict Clippy, tests, a build,
and isolated D-Bus checks. Add `--visual` to capture the demo views at 1440,
900, and 640 px under `/tmp/hyprdesk-visual` by default. The interactive preview
uses demo data and leaves session settings untouched. Check compositor blur and
multi-monitor behavior in a Hyprland session.

See the [development guide](docs/ADVANCED.md#development) for more build and verification details.

## Documentation

- [Advanced guide](docs/ADVANCED.md)
- [Writing a plugin](docs/WRITING_PLUGINS.md)
- [Plugin, widget, and launcher internals](docs/PLUGINS.en.md)
- [Project work log](WORKLOG.md) (French)

## Credits and license

hyprdesk is released under the [MIT license](LICENSE). The bundled
[Mindlog typeface](https://github.com/jacquesh82/mindlog.fonts) is licensed under
the [SIL Open Font License 1.1](assets/fonts/mindlog/OFL.txt).
[mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools)
provides LYRA and the `hyprpanel-core` Hyprland IPC library.
