# Design — packages, widgets and launcher

*[Version française](PLUGINS.md)*

hyprdesk is split into **packages**: each configurable feature (desktop,
icons, launcher, wallpaper, widgets, bar, screens, workspaces, effects) is a
self-contained module, shipped with the binary as a *built-in package*
(`builtin: true`). This document describes the contract of a package, that of
a widget, and where the launcher fits.

| Concept | Where | What it is |
|---|---|---|
| **Settings package** | `src/ui/plugins/<id>.rs` | an entry of the Settings screen: manifest, messages, state, view, update, CLI keys |
| **Widget** | `src/widgets/`, `src/ui/widget_view.rs` | an element placed on the desktop or in the bar, fed by a data source |
| **Widget plugin** | `~/.config/hypr/hyprdesk/widgets/<name>.lua` | a widget written in Lua, detected without recompiling |
| **Launcher** | `src/ui/plugins/launcher.rs` | the desktop dock: a standalone widget, the only control in `launcher-only` mode |

## 1. Settings packages

### Contract

A package is a module of `src/ui/plugins/` that exposes:

```rust
pub const INFO: PluginInfo = PluginInfo {
    id: "screens",            // stable identifier: tab, `hyprdesk settings screens`
    label: "Écrans",          // navigation: French source text, translated at display time (§ 9)
    icon: "video-display",    // glyph (ui/icons.rs) or theme icon
    summary: "Détection, disposition, …",
    builtin: true,            // shipped with hyprdesk
};

#[derive(Debug, Clone)]
pub enum Msg { … }                         // its messages, nothing else
pub fn wrap(m: Msg) -> UiMsg { … }         // envelope towards the application
pub struct State { … }                     // its screen state (optional)

impl App {
    pub(in crate::ui) fn tab_screens(&self) -> Element<'_, UiMsg>;       // view
    pub(in crate::ui) fn screens_update(&mut self, m: Msg) -> Task<UiMsg>; // update
}
```

The package only knows its own messages. It emits them through `wrap(Msg::…)`;
if it needs to talk to another package, it goes through `super::other::wrap(…)`,
which makes the dependency visible.

### Registry and shell

`src/ui/plugins/mod.rs` is the **registry**:

- `BUILTIN`: the manifests, in navigation order;
- `PluginMsg`: one variant per package, wrapped in `PanelMsg::Plugin`;
- `update`, `view`, `on_show`: routing to the relevant package;
- `find`: looks up a package by identifier or by a French alias (`fond`,
  `écrans`…), for `hyprdesk settings <package>`.

`src/ui/panel.rs` is the drawer's **shell**: its surface (its own layer
above the windows), opening and closing, the navigation built from
`BUILTIN`, and the shared helpers (`section`, `field`, `apply`, `commit`,
`commit_reload`, `panel_ctx`). It contains no setting: its only messages are
`Tab`, `Close`, `Commit` and `Plugin(..)`.

### Lifecycle of a setting

1. The view emits `wrap(Msg::X)`.
2. The package's update modifies `App::settings` (the model in
   `src/settings.rs`, mirror of `settings.lua`).
3. It calls one of the shell's helpers:
   - `apply()`: the effective configuration changes in memory, without
     saving. This is what is used while a slider is being dragged.
   - `commit()`: like `apply()`, then writes `~/.config/hypr/hyprdesk/settings.lua`.
   - `commit_reload()`: writes, then reloads the whole configuration. This is
     needed when Lua must be re-executed, for example to enable a
     widget.
4. The daemon watches `hyprdesk/`: a write coming from elsewhere (CLI,
   editor) is reloaded the same way.

`settings.lua` is loaded **before** `hyprdesk.lua`: what the user writes
by hand has the last word, and `field(…, "key")` flags it in the view.
`settings.lua` is also reloaded **on its own**, without `hyprdesk.lua`, for
the screen's state. A setting that targets an element of `hyprdesk.lua` must
therefore survive this standalone read (this is the case for `hd.place` and
`hd.disable`).

### Command line

Each package declares its keys in `src/cli_config.rs` (`KEYS` table, `plugin`
field). A key is a name, a help text, and three functions: read the effective
value, write into `Settings`, remove. `hyprdesk config` groups them by
package, in `BUILTIN` order, and writes the same `settings.lua` as the
Settings screen:

```sh
hyprdesk config                       # all keys, by package, effective values
hyprdesk config list screens          # a single package
hyprdesk config get launcher_group
hyprdesk config set show_on primary
hyprdesk config set keys.launcher "SUPER + ALT + D"
hyprdesk config set assign.firefox 2 silent   # dynamic key (workspaces)
hyprdesk config set widget.calendar on        # dynamic key (widgets)
hyprdesk config unset show_on                 # back to the default value
```

Keys per package:

| Package | Keys |
|---|---|
| `desktop` | `show_on`, `mode`, `glass_opacity`, `icon_scope`, `icon_double_click`, `float_launched`, `font`, `font_size` |
| `keys` | `suspend_binds`, `keys.toggle`, `keys.launcher`, `keys.hide`, `keys.settings`, `keys.lock`, `keys.leave`, `keys.switch`, `keys.search`, `keys.overview`, `keys.take`, `keys.media`, `keys.essentials`, `keys.essentials_off` |
| `icons` | `grid.icon_size`, `grid.cell`, `icon_theme` |
| `launcher` | `running_apps`, `launcher_group` |
| `wallpaper` | `wallpaper` (image or `#rrggbb`, all desktops), `wallpaper.fit`, `wallpaper.span` (`on`: spread across all screens) |
| `widgets` | `widget.<type>` (`on` / `off`) |
| `bar` | `bar`, `bar.position`, `bar.height`, `bar.monitor` (`all`, `primary`, or comma-separated screens) |
| `screens` | `primary` |
| `workspaces` | `workspaces`, `workspace.<n>` (screen: connector, `primary`, `auto`), `assign.<app>`, `workspace_selector` (`relief`, `rail`, `miniatures`, `icones`) |
| `layouts` | `layout`, `layout.<n>`, `dwindle.<option>`, `master.<option>`, `scrolling.<option>` |
| `apptheme` | `app_themes` (`kitty`, `none`) |
| `effects` | `transition`, `fade_ms`, `crossfade_ms` |
| `theme` | `theme.preset` (shipped theme), `theme.name`, `theme.fg`, `theme.bg`, `theme.accent`, `background`, `theme.effects.<place>.opacity` (0..1), `theme.effects.<place>.blur` (places: `bar`, `menus`, `launcher`, `settings`, `widgets` — no blur) |
| `launcher` (appearance) | `launcher_position`, `launcher_size`, `launcher_pinned`, `launcher_show_settings`, `launcher_show_assistant` |
| `apps` | `apps.<role>` (`browser`, `terminal`, `files`, `editor`, `image`, `video`, `audio`, `pdf`, `mail`, `ai`) |
| `assistant` | `assistant_mode` (`window`, `terminal`), `ai_services`, `ai_use` |
| `plugins` | `plugin.<id>` (`on` / `off`) |

The screen layout (position, rotation, mode) stays in the Settings
screen. Each change is applied live there, then must be confirmed
before being written to `monitors.lua`. A blind write from the command
line would not have this safety net.

### Navigation and advanced settings

The Settings navigation groups packages (`plugins::GROUPS`): Desktop,
Appearance, Bar and launcher, Windows and screens, Applications, System. A new
package goes into one group (a test checks that each package is in exactly
one). Wide drawer: the selected package's tabs appear under it as sub-entries;
narrow drawer: as pills above the page.

“Advanced settings” (drawer header, key `advanced_settings`): checked, everything
is shown; unchecked, only the essentials. Hidden then: the packages of
`ADVANCED_ONLY`, the tabs of `ADVANCED_TABS`, and the fields a view wraps in
`self.adv(…)`. Nothing is removed from the configuration: only the display
changes.

### Tabs of the loaded packages

| Package | Tabs |
|---|---|
| Desktop | Display · Shortcuts · Session |
| Theme | Themes · Colors · Text (font, size) · Effects |
| Launcher | Content · Appearance · Pinned |
| Bar | General · Modules |
| Screens | Layout · Screen (clicking on the map leads there) |
| Workspaces | Count and screens · Applications |
| Layouts | General · Dwindle · Master · Scrolling · Custom |
| Widgets | On this desktop (a widget selected on the desktop leads there) · Available |

### Adding a package

1. Create `src/ui/plugins/<id>.rs` with `INFO`, `Msg`, `wrap`, the state if
   needed, the view and `<id>_update`.
2. Declare it in `plugins/mod.rs`: `pub mod`, a line in `BUILTIN`, a
   variant in `PluginMsg`, an arm in `update`, `view` and, if needed,
   `on_show`.
3. If the package has a screen state, add its field to `Panel` (`panel.rs`).
   If it has many settings, split them into tabs: a `TABS` constant
   (`(identifier, label)`), an arm in `plugins::tabs`, and the view
   only shows the tab of `self.subtab(INFO.id)`. The drawer draws the
   pills and remembers the choice; `hyprdesk settings <package>/<tab>`
   opens it directly.
4. Persist in `Settings` (`src/settings.rs`): the field, `from_config`,
   `apply_to` and `to_lua`. On the Lua side, the key must be recognized in
   `config/lua.rs` and documented in `assets/hyprdesk.meta.lua`.
5. Declare the CLI keys in `cli_config.rs`.
6. Add a test for the package, and a scene in the preview
   (`ui/preview.rs`, `scripts/verify.sh`).

The `plugins::tests::registre` test checks that identifiers are unique.
The `cli_config::tests::chaque_cle_appartient_a_un_paquet` test checks that
each CLI key points to an existing package.

### Built-in packages

| id | Label | Controls | Domain outside the UI |
|---|---|---|---|
| `desktop` | Desktop | screens the desktop is shown on, mode, font, session | `config::General` |
| `keys` | Shortcuts | every active shortcut (outside the full desktop, desktop shown), hyprdesk's own, essential window, workspace and screen shortcuts set by hyprdesk | `keymap.rs`, `essentials.rs` |
| `icons` | Icons | application shortcuts, size, theme | `launch.rs`, `layout.rs` |
| `launcher` | Launcher | windows shown, grouping | — (widget, § 3) |
| `theme` | Theme | colors, backgrounds, effect of each place (opacity, blur), desktop mode, lock screen veil; shipped themes and “My themes” | `theme.rs`, `config::Theme` |
| `apptheme` | App theming | the theme applied to other software (kitty: `~/.config/kitty/hyprdesk-theme.conf`, included at the end of kitty.conf, reloaded with SIGUSR1) | `apptheme.rs` |
| `wallpaper` | Wallpaper | image, slideshow, color; scope | `config::WallpaperRule` |
| `widgets` | Widgets | activation, placement | `widgets/` (§ 2) |
| `bar` | Bar | position, height, screens, modules | `ui/bar.rs` |
| `screens` | Screens | detection, layout, orientation, mode, primary screen | `screens.rs` → `monitors.lua` |
| `workspaces` | Workspaces | persistent workspaces, application assignment, bar selector style, the “Workspaces” organising screen | `hypr.rs` (Hyprland rules), `ui/overview.rs` |
| `layouts` | Layouts | default and per-workspace layout, Dwindle, Master, Scrolling options, custom layouts (`lua:<name>`), each layout's shortcuts | `tiling.rs` |
| `effects` | Effects | transitions, crossfade between wallpapers | `config::General` |
| `apps` | Applications | default applications (browser, terminal, files, editor, images, video, audio, PDF, mail, AI), applied to the system (`xdg-mime`); only installed applications are offered (`TryExec` / `Exec` program present), recognized by the role's file type or by their freedesktop category (`WebBrowser`, `TextEditor`, `FileManager`, `Email`, `TerminalEmulator`); a choice that has gone missing is flagged “not installed” | `apps.rs` |
| `assistant` | AI | tabs Assistant (LYRA, mindlog.hyprland_tools, with Claude Code, Codex or Zcode, and hyprdesk's context), Other AI services (providers, the provider of each feature, keys in the keyring) and SmartUI design (icons redrawn by AI, `hyprdesk-smartui` theme, automatic generation) | `assistant.rs`, `ai_services.rs`, `smartui.rs` |
| `share` | Sharing | export / import of a setup or of the theme alone (archive) | `share.rs` |
| `plugins` | Plugins | state and activation of packages, Lua plugins: active, third-party, adding, removing | `widgets/plugins.rs` |
| `about` | About | version, system, Hyprland, LYRA, configuration files, dependencies, SmartUI theme; copy for a bug report | `doctor.rs` |

The **domain** (model, Lua, Hyprland rules, `monitors.lua`) does not depend on
the UI. The package bridges the two. This is what will later make it
possible to turn them into separate *crates*: the domain first, the UI
next. For now, packages are modules of the binary, because their views
need the application state (`App`).

## 2. Widgets

### Model

A widget is a **type** and an **instance**:

- **Type** (`config::WidgetType`, catalog `Config::catalog`):
  - built-in, declared in `widgets::BUILTINS`: `clock`, `calendar`,
    `system`, `media`, `command`, `lua`;
  - or a detected Lua plugin (§ 2.3).
- **Instance** (`config::WidgetDef`): a type and its options, plus its
  placement (`anchor` on a 3×3 grid, `x`/`y` offsets, `size`, `card`).
  It also carries its scope (`workspace`, `monitor`) and its origin
  (`hyprdesk.lua` or `settings.lua`).

Identifiers. An instance declared without an `id` gets `<type>#<n>`, counted
per type **and** per origin: a widget added from the settings does not shift
those of `hyprdesk.lua`. Bar modules have their own
identifiers (`bar:<zone>:<n>`) and do not count.

### Lifecycle

1. **Declaration**: `hd.widget("calendar", { anchor = "right" })` in
   `hyprdesk.lua`, or activation in Settings → Widgets (written to
   `settings.lua`).
2. **Placement**: `hd.place("calendar#0", { … })` repositions a widget from
   `hyprdesk.lua` without touching the file.
3. **Deactivation**: `hd.disable("calendar#0")` turns off a widget from
   `hyprdesk.lua`. Turning it back on removes the line.
4. **Polling**: the scheduler (`ui/mod.rs`, `poll`) polls each widget at
   its `interval`, off the UI thread, and only if the widget is
   visible. Results from a stale configuration are ignored (generation
   number).
5. **Rendering**: `ui/widget_view.rs` turns the value (`widgets::Value`: text,
   system lines, media, rich content) into iced elements, inside its card.

### Lua plugins

A plugin is a `<name>.lua` file or a `<name>/init.lua` folder. Search
folders, the first match wins:

1. `~/.config/hypr/hyprdesk/widgets`
2. `~/.local/share/hyprdesk/widgets`
3. `/usr/share/hyprdesk/widgets`

The shipped plugins (`reseau`, `bluetooth`, `batterie`, `capture`, `clavier`, `lecteur`, `majs`, `supports`, `nuit`, `vkbd`, `notifications`,
`session`) are built into the binary, and a file with the same name replaces them.

```lua
return {
    name = "VPN", description = "VPN state", interval = 5, anchor = "top-right",
    options = { interface = "wg0" },          -- default values of the options
    render = function(o)                      -- text, or a rich table:
        return { icon = "network-vpn-symbolic", tooltip = "VPN up", state = "active" }
    end,                                      -- state: normal, active, warning, critical, hidden
    on_click  = function(o) end,
    on_scroll = function(o, dir) end,         -- +1 / -1
    menu = function(o) return { { header = "VPN" }, { label = "Disconnect", action = function() end } } end,
}
```

A menu entry can also be a **text field**: `{ input = "Password",
label = "Wi-Fi “Home”", secret = true, action = function(text) … end }`. Enter
calls the action with the typed text, then the menu closes; `secret` masks the input,
which is wiped from hyprdesk's memory as soon as it is sent. An entry `{ image = "/path.png" }`
(or `file:///…`) shows an image, an album cover for instance: only a local file,
nothing is downloaded.

The same plugin works on the desktop and in the bar (`hd.bar({ right = { "vpn" } })`).
Available helpers: `hd.exec` (output of a command, time-bounded) and
`hd.spawn` (launch through Hyprland); `hd.exe` is the path of the running
hyprdesk, to call it without relying on PATH (`hd.spawn(hd.exe .. " lock")`). The Lua runs on a dedicated thread,
with a time budget per call. A failing plugin is flagged in the
settings without blocking the others.

### Bar

Bar modules are widgets placed in a zone (`left`,
`center`, `right`) of `hd.bar`, with `in_bar = true`. Some are specific to
the bar and are not widgets: `workspaces`, `window`, `volume`, `brightness` (backlight, or external screens through DDC/CI), `tray`,
`desktop`. `hd.bar({ monitor = "primary" })` shows the bar on the primary
screen.

The bar adapts to each screen:

- **Compact mode** (`compact = "auto" | "always" | "never"`, `auto` by
  default): on a screen narrower than 1600 px or in portrait, the window
  title is shortened, the clock is reduced to the time, and icons lose
  their label (volume, plugins).
- **Modules for vertical screens** (`portrait = { left, center, right }`):
  they can also be set in Bar → Modules → Vertical screens.
- **Style** (`style = "continuite" | "graphite"`): a complete bar per style,
  one rendering module in `src/ui/bar_style/`. `continuite` (default) is the
  smoked glass with a gliding capsule; `graphite` is a machined neutral
  acrylic slab — a rounded block floating above the screen edge, modules
  engraved into the surface, the active workspace raised — and enforces its
  relief keys (the workspace selector only applies to Continuité). Picking a
  style also applies its **layout** (Graphite: window title left, workspaces
  centered, network/battery/volume + short clock + one settings panel right)
  and its **geometry** (bottom, 40 px, 10 px margin) — in Settings, through
  the CLI (`bar.style`), and in `hd.bar` when no zone is written. Fields and
  zones adjusted afterwards stay authoritative.
- **Margin** (`margin = 0..32`, 0 by default in Continuité): distance between
  the bar and the screen edge; the reserved area accounts for it.

Each zone is clipped: even when too long, they no longer overlap.

## 3. The launcher

The launcher used to be a part of the desktop. It is now a **standalone
widget**, with its own package (`launcher`):

- **Role.** It is the dock at the bottom center, with the open windows followed
  by the ⚙ button. It is the only control in `launcher-only` mode, where tiled
  windows leave room for it (per-screen `gaps_out` rule).
- **Windows shown** (`running_apps`):
  - `off`: only the settings button;
  - `workspace`: the windows of the displayed workspace;
  - `all`: all windows, including those set aside, so they can be
    called back.
- **Grouping** (`launcher_group`):
  - `none`: one entry per window;
  - `app`: one entry per application, that is, per window class,
    hence per binary. Dots show the number of windows, and a click
    cycles from one to the next;
  - `workspace`: windows grouped under their workspace number.
- **Golden rule.** Using it never changes the desktop mode:
  - in `launcher-only`, a click focuses the window;
  - in `full-desktop`, the chosen window comes back alone on top of the desktop,
    and the others stay set aside;
  - ⚙ opens the settings in their own layer.

## 4. The three desktop modes

| Mode | Windows | Visible | Shortcut |
|---|---|---|---|
| `no-desktop` | normal | wallpaper | SUPER + CTRL + D |
| `launcher-only` | in place, clear the launcher | launcher | SUPER + ALT + D |
| `full-desktop` | set aside (`special:hyprdesk`) | icons, widgets, launcher | SUPER + D |

The current mode's shortcut returns to `no-desktop`. The bar's Desktop button
cycles from one mode to the next: `no-desktop → launcher-only → full-desktop`.
Neither the launcher nor the settings change the mode.

## 5. Enabling, disabling, adding

The **Plugins** package shows everything that is installed:

- **Built-in packages.** Each one has a switch. Turning a package off
  (`disabled_plugins`, or `hyprdesk config set plugin.bar off`) removes it from
  the navigation **and** disables its feature: no bar, no launcher, no
  workspace rules, no icons or widgets on the desktop, plain background,
  no new-screen notice. `Config::plugin_on(id)` is the only check, placed
  at the point where each feature acts. Desktop, Effects, Theme and Plugins form
  the core and always stay enabled (`CORE_PLUGINS`).
- **Lua plugins.** Each one shows whether it is active and where it comes from:
  built-in, shipped, or third-party (with its path). A third-party plugin is
  installed from a `.lua` file, an `init.lua` folder or a git repository, into
  `~/.config/hypr/hyprdesk/widgets/` (`widgets::plugins::install`).
  Removing it moves it to `.retired/`, from where it can be restored.
- **Configure.** A plugin that declares `options` has a "Configure" button:
  each option is set there (a switch for a boolean, a field for a number or
  text). The setting goes to
  `hd.config({ plugin_options = { <plugin> = { <option> = <value> } } })` and
  applies to every instance of the plugin, except options written on the
  instance itself. "Plugin defaults" goes back to the default values. For the
  `majs` plugin, the check and update commands of each source are shown and
  can be changed (`hd.config({ update_commands = { aur = { upgrade = "…" } } })`).

A third-party *compiled* package cannot be hot-loaded. Third-party extensions
therefore go through Lua plugins, or through a contribution to the `BUILTIN`
registry.

## 6. Launcher: its own layer

The launcher has one surface per screen, on Hyprland's `Top` layer, above
the windows. Its **exclusive zone** (like the bar's) makes windows
leave room for it, without any margin rule. Tooltips and the window list of a
grouped application open in a **bubble**, itself a surface
(`Overlay`) placed at the cursor. A 250 ms delay leaves time to move from the
launcher to the bubble. Settings: edge (`bottom`, `top`, `left`, `right`),
size, pinned applications, assistant and settings buttons.

## 7. AI assistant and LYRA

The `assistant` package opens **LYRA** (`hyprpanel-chat`, repository
mindlog.hyprland_tools). LYRA works with the subscription chosen for the `ai`
role (Claude Code, Codex, Zcode). hyprdesk provides its environment:

- `hyprdesk context`: modes and shortcuts, packages, default applications,
  launcher, workspaces, bar, widgets, files, commands;
- the same text in `~/.config/hyprpanel/context.d/hyprdesk.md`, kept up
  to date on every configuration change.

Contract expected on LYRA's side, detected in its `--help`:

- `--provider <zcode|claude_code|codex>`: the session's provider;
- reading the files in `~/.config/hyprpanel/context.d/*.md`, **added**
  to the context (without replacing LYRA's role).

As long as LYRA does not offer them, it keeps its own provider choice,
and the context waits in `context.d`.

### The assistant drawer (`serve` protocol)

When LYRA offers `hyprpanel-chat serve` (seen in its `--help`) and the
assistant opens “in its own window”, hyprdesk **draws** the
conversation **itself** in a drawer styled like Settings (same layer, header,
cards, Esc); LYRA keeps the whole engine: providers, tools,
confirmations, memory, scope, `context.d`. Only one drawer at a time.
Without `serve`, LYRA opens in its own window, placed like the drawer.

The engine is a child process, started on opening and stopped on
closing. It talks in **JSON Lines** (one object per line, distinguished by
`type`; unknown fields ignored) — code: `src/chat.rs`, view: `src/ui/chat.rs`.

| Direction | `type` | Fields |
|---|---|---|
| → LYRA | `hello` | `client`, `protocol` (1), `language` |
| → LYRA | `message` | `id`, `text`, `attachments` [`{path}`] |
| → LYRA | `confirm` | `request`, `accept` |
| → LYRA | `choice` | `request`, `value` |
| → LYRA | `cancel`, `new_conversation` | — |
| → LYRA | `provider` | `value` |
| ← LYRA | `ready` | `protocol`, `provider`, `providers`, `fs` |
| ← LYRA | `turn_start` / `turn_end` | `id` (+ full `text` at the end) |
| ← LYRA | `delta` | `text` (Markdown, streamed) |
| ← LYRA | `thinking` / `progress` | `active` / `phase`, `fraction` |
| ← LYRA | `tool` | `id`, `name`, `summary`, `status` (`running`, `done`, `error`), `result` |
| ← LYRA | `confirm_request` | `id`, `command`, `reason`, `risk` (`low`, `high`) |
| ← LYRA | `choice_request` | `id`, `kind`, `prompt`, `options` [`{label, value, preview}`] |
| ← LYRA | `error` | `message`, `fatal` |

A confirmation is **never** accepted by default: it waits for the click.
To try it without LYRA: `HYPRDESK_CHAT_ENGINE="hyprdesk chat-mock"` in
the daemon's environment (fake engine: streamed text, one tool, a
confirmation if the message mentions “gaps”, a choice if it mentions
“couleur” (French for “color”).

## 8. Sharing a setup

`hyprdesk export [fichier] [--theme] [--screens]` and `hyprdesk import
<archive> [--theme] [--dry-run] [--screens]`, or the **Sharing** package, which
offers two scopes: **the entire setup**, or **the theme only**.

- **The archive** contains `hyprdesk.lua`, `settings.lua`, third-party plugins,
  the wallpaper **images**, a `manifest.json`, and `monitors.lua`
  on request (it describes the machine's screens).
- **On import**:
  - the imported configuration is checked before any change;
  - the current configuration is backed up into `hyprdesk/backups/`;
  - wallpapers are copied into `~/.local/share/hyprdesk/wallpapers/<archive>/`,
    and their paths rewritten;
  - plugins already present are kept.

### The theme only

A **theme** (`theme.rs`) gathers what can be seen: name, colors (text,
surfaces, accent, background without image), wallpapers, effect of each place
(background opacity and Hyprland blur for the bar, menus, launcher and
settings; widget opacity), desktop mode and glass veil, lock screen veil and
blur, transitions.

- **Export** (`--theme`, or “Theme only”): the **effective** theme (settings
  and `hyprdesk.lua` combined), written to a `settings.lua` that contains
  only the theme, with the images of its wallpapers; `manifest.json` carries
  `kind = "theme"` and a summary (`theme`: name, colors).
- **Import**: only the theme changes; bar, shortcuts, widgets, plugins and
  screens stay as they are. A theme archive is always imported this way;
  the theme of a full archive can be too, on request (`--theme`). A wallpaper
  that is neither in the archive nor present on the machine is dropped.
- **My themes**: theme archives stored in `hyprdesk/themes/`
  (Theme package: “Save to My themes”, one click to apply).
  Shipped themes (`theme::PRESETS`) change colors and effects, not the
  wallpapers.

### Multi-screen scenes

A **scene** is an image designed for the whole screen layout.

- **Automatic slicing** (`span.rs`): `hd.wallpaper({ path = …, span = true })`
  (or “Span across all screens”, or `wallpaper.span on`). The canvas
  encloses the screens as Hyprland places them (logical positions and
  sizes); the image covers it, centered, and each screen shows its share.
  An L-shaped, stacked or offset layout stays continuous; areas of the
  canvas without a screen are not seen. A slideshow spans the same way.
- **Resolution**: choosing an image that is too small for the targeted screens (or
  for the whole canvas if it is spanned) shows a non-blocking
  warning: upscaling factor and recommended size, in physical
  pixels (Hyprland's scale included).
- **Theme derived from a scene** (`theme::create`, `hyprdesk theme new`, or
  “Create from an image” in the Theme package): colors extracted from
  the image (surfaces in the hue of the shadows, accent in the dominant vivid
  hue), spanned scene, translucent and blurred surfaces; stored in
  My themes. The source can be an image, or a **pack** (folder or
  `.zip`) whose `manifest.json` designates the master scene (`master`) and
  its name (`theme`): the pack's pre-cut slices are ignored, slicing
  is done according to the actual screens.
- **Lock screen**: it shows the whole scene (same image as the desktop),
  unless it has its own image (`lock_image`). Per-screen slicing on the lock
  screen will come once the lock screen knows which output carries each
  surface.
- `hyprdesk wallpaper split <image> [folder] [--scale N]` writes each
  screen's share (`<screen>.png`), for another tool or for checking.

## 9. Interface language (i18n)

hyprdesk speaks French (source language) or English:
`hd.config({ language = "auto" | "fr" | "en" })`, Settings → Desktop → Language, or
`hyprdesk config set language en`. `auto` follows `LC_ALL`, `LC_MESSAGES` then `LANG`
(French if the language is French, English otherwise). A chosen language also
applies to dates; `auto` lets dates follow `LC_TIME`.

**The French text is the key** (gettext-style): the
`locales/en.toml` catalog, built into the binary, maps each French text to its
translation. A missing text is shown in French: nothing breaks.

- **Views (Rust)**: `text` comes from `crate::i18n` (not from iced) in views;
  it translates any text present in the catalog, literal or computed. Other
  labels go through `crate::i18n::tr("…")` (`radio`, `.label`, `text_input`,
  `placeholder`, `label: …` fields); a text with placeholders is written
  `crate::tr!("Écran {mon}", mon = m)`. Constants (`INFO`, `TABS`, built-in
  widgets) are translated at display time.
- **Command line**: messages through `tr!`, help texts translated line by line
  (`i18n::tr_lines`, columns included).
- **Lua plugins**: `hd.t("texte")` translates, `hd.lang()` returns `"fr"` or `"en"`
  for a plugin that has its own texts.
- **Stay in French**: logs, generated files (`settings.lua`,
  `monitors.lua`), context passed to LYRA.
- **Checks**: `i18n::tests::catalogue_complet` (texts marked in
  `src/` and `assets/`), `registres_traduits` (packages, tabs, widgets),
  `aide_traduite` and `aide_config_traduite`. To complete the catalog:
  `HYPRDESK_I18N_TODO=missing.toml HYPRDESK_I18N_TODO_DIR=<dir> cargo test i18n`
  writes the missing entries, to be translated and then added to `locales/en.toml`.
