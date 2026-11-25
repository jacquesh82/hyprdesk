# Changelog

All notable changes to hyprdesk are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Focus on the full desktop** (MLG-327): Settings → Desktop → « Focus on
  the full desktop » (`hd.config({ focus = { … } })`, `focus.*` in
  `hyprdesk config`). « On click »: the clicked window takes the keyboard and
  comes to the front, hovering changes nothing; « When the mouse hovers over
  it »: the hovered window takes the keyboard and comes to the front, also
  between floating windows, after an optional distance; « As outside the
  desktop » leaves Hyprland's own setting. Also: whether the click that
  brings a window forward reaches it, whether an application may come to the
  front by itself, and where the focus goes when a window closes. Hyprland
  only brings a window to the front on a click: following the mouse,
  hyprdesk brings the hovered window forward itself. Set while a screen shows
  the full desktop; the previous values come back when leaving it, and after
  a Hyprland reload.
- `hyprdesk menu hyprdesk | windows | workspaces | help` opens the bar's menus
  (the `menus` module), like a click on their title.

- **Menus, after Apple's « Menus and actions » guidelines** (MLG-306 to
  MLG-311):
  - one menu component for the bar and every right-click menu: at most
    three groups split by a line, an icon per item, a pressed state
    distinct from hover, destructive items in red, keyboard navigation
    (arrows, Home/End, Enter, Escape);
  - **dock menu** — right-click an app in the dock: its open windows
    (with their workspace), New window, the app's own `.desktop` actions
    (e.g. « New private window »), pin or unpin, close its windows;
  - **app actions** — the same menu on right-click in the launcher grid,
    and `.desktop` actions in the menu of an app icon on the desktop;
  - **edit menu** — right-click in any text field: Cut, Copy, Paste,
    Select all (Cut/Copy greyed without a selection or in a password
    field, Paste greyed with an empty clipboard);
  - **Share…** on desktop files: copy the file (paste it in a file manager
    or chat), send it by email (`xdg-email`), send it to a phone paired
    with KDE Connect;
  - **window menu** — click the active window's title in the bar:
    minimize, maximize, fullscreen, floating, on all workspaces, move to
    another workspace, close.

- **Per-screen overrides** (MLG-281): bar height and text scale for one
  screen, chosen by output name or logical resolution (e.g. a 36 px bar on
  a 2570×720 strip) — `screen.<screen>.bar_height` / `.text_scale` keys,
  Settings → Screens, and `[screens."…"] bar_height` in theme packs (your
  own setting wins).
- **Wallpaper framing and treatment** (MLG-280): focal point and zoom for
  « fill », brightness, saturation, contrast and vignette applied when the
  image is decoded; speed and frame rate of video wallpapers; glass tinted
  by the wallpaper's average colour; a light-mode and a dark-mode wallpaper
  in your own settings. `wallpaper.*` keys, Settings → Theme → Components,
  and pack keys.
- **Component appearance** (MLG-279), one registry for CLI keys, theme
  packs and Settings → Theme → Components: dock icon spacing, hover
  magnification, auto-hide and active-app marker; workspace selector as
  numbers or dots, capsule width, wheel switching, occupied marker;
  quick-settings columns and width; notification corner, width, visible
  count, display time and stacking; monitoring alert threshold and gauges;
  clock date line; media cover art (size, card tint from the cover); lock
  screen clock position and panel density.
- **Launcher shape** (MLG-278): card width and maximum height (results
  scroll inside), corner radius, padding, grid columns, row height, visible
  results; what shows before typing (most launched, pinned apps, or the
  field alone); backdrop dimming; opening effect (fade, grow, slide,
  instant) and duration; card at the centre or the top with an offset;
  selection radius, outline and shadow; clicking outside to close; glass
  effect quality. `launcher.*` keys, `launcher_shape = { … }`, Settings →
  Launcher → Launcher shape, and pack `[launcher]` keys.
- **Bar shape** (MLG-277). Left and right edges (vertical bar: stacked
  zones and workspaces, menus opening beside it); length (whole edge, fit
  content, pixels or a percentage) with bounds and alignment; side margins
  for a floating bar; detached groups; corner radius, inner padding, module
  spacing, icon and text size; auto-hide on every style with show/hide
  delays and a reveal zone; reserved space on or off; layer (over windows,
  over fullscreen, under windows). New `bar.*` keys, `hd.bar { shape = … }`,
  a Shape section in Settings → Bar, and pack `[bar]` keys.
- **Theme packs carry shape and layout** (MLG-276). `[bar]`, `[dock]`,
  `[launcher]`, `[workspaces]`, `[lock]`, `[desktop]`, `[notifications]`,
  `[widgets]`, `[controls]` sections set hyprdesk settings (bar thickness and
  edge, dock position and size, launcher style, workspace selector…) when
  the pack is chosen, through the same validated keys as `hyprdesk config`
  and an appearance-only allowlist (never shortcuts, commands or apps).
- **Surface roles and accessibility preferences** (MLG-282, from Apple's
  Human Interface Guidelines). Theme packs give each surface a role —
  `transparent`, `glass_clear`, `glass_regular`, `content_surface` — and a
  fallback material for when it cannot see what is behind it; a stable
  `content` surface sits under launcher results and the Settings detail
  inside their glass; lists fade under the edges of their surface; a
  transparent bar picks its text colour from the wallpaper behind it. New
  colour roles (`focus`, `separator`, `error`, `text_primary`,
  `text_secondary`) and text styles (`label`, `metric`). Settings → Theme →
  Accessibility adds increased contrast and text size next to reduced
  transparency and motion.
- **True glass refraction** (MLG-275). With a theme that asks for refraction
  (the bundled Verre does), hyprdesk's surfaces now bend, blur and tint what
  is really behind them on the GPU (approach of iced_glass, MIT): desktop
  widget cards over the wallpaper and icons; the bar, the dock and the
  full-desktop launcher over their slice of the wallpaper; bar menus and
  Settings over a capture of the screen taken as they open (frozen while
  open). Software rendering keeps the simulated rim. `hyprdesk preview` takes
  `HYPRDESK_PREVIEW_PACK=<pack>` to try a pack.
- **Theme glass on every bar style, true tabular figures, contrast notice**
  (MLG-274). Island's capsule and Convex's niches get the theme's highlight
  and refraction rim; clocks and monitoring use the font's real `tnum`
  figures (vendored iced_graphics passes the OpenType feature); Settings →
  Theme → Colours flags text or accent colours below the contrast targets
  (your own colours are never rewritten).
- **Theme design tokens** (MLG-267 to 273). A theme pack now covers the whole
  look of the desktop, not just colours and opacities:
  - glass: `[glass]` tint, highlight (optionally following the pointer),
    edge refraction, rim and shadow, drawn by hyprdesk over the bar, menus,
    Settings, launcher and widget cards; reusable `light`, `tinted`, `dense`
    presets (and the pack's own) per surface; Hyprland blur size and passes.
    The bundled Verre pack uses them;
  - palette: `action` and `selection` slots, distinct from the accent;
  - contrast: text and UI-marker targets (4.5:1, 3:1) enforced on the pack's
    slots, minimum density of readable surfaces, reduced transparency;
  - typography: sizes per role (every hyprdesk text follows its role),
    weights, line height, tabular figures for clocks and monitoring, pack font
    family;
  - interaction states: hover, pressed, selected, keyboard focus ring,
    disabled — launcher results and tiles;
  - motion: reference duration, easing of all fades, scale and offset of bar
    menus, reduced motion;
  - per-screen overrides: `[screens."DP-1"]` or `[screens."2570x720"]`
    surface effects and text scale.
  Templates read them (`fonts.*`, `motion.*`, `states.*`, `contrast.*`,
  `effects.<surface>.refraction`…), `palette.json` publishes them, and
  Settings → Theme gains Accessibility (reduced transparency, reduced motion;
  `theme.reduced_transparency`, `theme.reduced_motion`).
- **Window borders follow the theme pack in a hyprdesk session** (MLG-264).
  With theme-coloured borders on (Settings → Windows), a pack with its own
  Hyprland template (Nacre…) now gives `appearance.lua` its border, group and
  shadow colours, gradients and alphas included, for the current variant;
  packs without one keep the accent gradient.
- **Adaptive transparency** (MLG-261). A theme pack can declare
  `adaptive = true` (and an `opacity_max` cap) on a surface: over a busy or
  bright wallpaper, the bar and desktop widgets thicken their glass just
  enough to keep text at 4.5:1 where the wallpaper behind them is least
  favourable, measured once per wallpaper. Settings → Theme → Transparency
  has an **Adaptive** switch for the bar and widgets and shows the cap and
  the bar's current opacity; CLI key `theme.effects.<surface>.adaptive`
  (`off`, `on` or a cap). The launcher already adapted and still does.
- **Bar colours that follow light/dark mode** (MLG-263). A bar or module
  colour can name a slot of the theme pack's palette (`active`, `focus`…),
  which changes with the variant, or a fixed dark/light pair
  (`#2dd4bf/#006b5f`): in `hd.bar({ colors = … })`, `hyprdesk config set
  bar.color.accent active`, and Settings → Bar → Colours (theme slot chips,
  **By mode** button; **Choose…** then sets the current mode's colour).

- **Compare blur variants on screen** (MLG-262). Hyprland has no
  refraction and screenshots do not show blur, so the variant (Prism, Water,
  Fluid…) is chosen by eye: `hyprdesk fx compare blur.variant 4,5,6` (or
  `all`, `--seconds`, `--rounds`) sets each value live with its name in a
  Hyprland notification, then puts yours back (Ctrl-C too). Settings →
  Animations → Effects has a **Compare on screen** button.

### Changed

- **The launcher opens as a centered card in every mode by default**
  (MLG-322): `apps_style` defaults to `spotlight` instead of `auto` (large
  sheet on the full desktop); `sheet` and `auto` stay available.
- **Desktop is a plugin like the others** (MLG-324): Settings list it first
  in the « Plugins » group, with Bar, Dock, Launcher and Panorama (it stays
  in the core: it cannot be turned off). Wallpaper moves to « Appearance »
  and desktop icons to « Plugins », next to widgets; the former « Desktop »
  group is gone. In the code, the desktop surface and its widget layout live
  in `src/ui/plugins/desktop/` (`surface.rs`, `layout.rs`).
- **Bar laid out like the macOS menu bar** (MLG-313), Continuity style:
  - one even slab, edge to edge: no darker islands under its zones (they
    stay with « detached groups »), no accent fill on modules that are on
    (Wi-Fi, Bluetooth — their icon tells the state), no track under the
    desktop-mode switch, a hover that lightens the bar;
  - the « hyprdesk » menu shows the hyprdesk logo, in the bar's ink, like
    the Apple menu;
  - default layout: menus on the left, the active window in the middle,
    status icons, Control Centre and the clock at the right end; Session
    lives in the hyprdesk menu and the workspace numbers are hidden (the
    Workspaces menu lists them; add the `workspaces` module back from
    Settings → Bar). A bar still on the former default (clock in the
    middle) moves to it once at start-up.
- **The bar menus are a module, `menus`** (MLG-317): placed, moved or removed from
  Settings → Bar like any other module (never in « More controls »). Every
  bar gets it once at start-up, first on the left as before; a Continuity
  bar with an empty middle moves its active window there.
- The « Window » bar menu becomes **« Windows »** (MLG-315): every open window,
  grouped by workspace (minimised ones last, the active one ticked), with
  its application icon; a click focuses it and goes to its workspace
  (a minimised window comes back to the shown one). The active window's
  commands stay on its title.
- The Continuity control center no longer overflows (MLG-321): labels are
  cut to their tile, the card takes its height again once the screen's
  brightness is known (DDC/CI answers after it opens), and a scroll bar, if
  any, sits beside the tiles.
- The clock marked with unread notifications opens them (MLG-323); without
  unread ones it keeps the calendar.
- **Windows and hyprdesk share their corners** (MLG-320): Hyprland's
  `decoration.rounding` defaults to 12 px when neither Effects nor the theme
  pack set it, and the Settings drawer uses the same radius.
- The launcher opens again with **SUPER + R** by default (MLG-319)
  (SUPER + Space since the macOS shortcuts); `keys.search` still changes it.
- Settings open again with **SUPER + SHIFT + D** by default (MLG-316) (SUPER + comma
  since the macOS shortcuts); `keys.settings` still changes it.
- Opening Settings no longer changes the bar (MLG-314): modules become draggable only
  on the Bar page, and without darker islands under the zones.
- **Continuity control center, laid out like macOS's** (MLG-318): a narrow card under
  its bar module instead of the full-width dashboard. Wi-Fi and Bluetooth
  in one tile (round toggle in the accent colour, network or device below;
  the title opens the details), a Silence tile, two square tiles (night
  light and screenshot when their widgets are there, otherwise lock and
  screens), Display and Sound bands with thick sliders (the speaker symbol
  mutes, the side button opens the sound card and its output choice), and
  the track playing. Calendar, notifications, system and network stay on
  their own cards and modules.
- The Sound page's mute button reads « Couper le son » / « Rétablir le
  son » (« Mute » / « Unmute »), so that « Couper » is the edit menu's Cut.
- **The dock is a plugin of its own** (MLG-305): Settings → Dock (Content,
  Appearance with its style — spacing, magnification, auto-hide, active
  marker, formerly in Theme → Components —, Pinned), turned off on its own
  (`hyprdesk config set plugin.dock off`). The Launcher plugin now only
  holds the application launcher (Applications, Shape, Shortcut) and is
  part of the core. An old `disabled_plugins = { "launcher" }` turns the
  dock off, as before.

### Fixed

- **The window border goes around the Hyprbars title bar** (MLG-326): it
  used to round itself under the bar, around the content only; hyprdesk now
  sets `bar_precedence_over_border` by default.
- **Hyprbars title bar in one colour on the active window** (MLG-325):
  Hyprland's optimised blur painted itself over a bar that is « part of the
  window », leaving a translucent rectangle inside a rim of the bar colour.
  hyprdesk turns `decoration.blur.new_optimizations` off while Hyprbars is
  loaded and gives the configured value back when it unloads.

- **Theme engine** (MLG-266): a pack's `[wallpaper]` was documented but
  silently ignored — it is now applied (per variant too) when the pack or its
  variant changes; `[fonts]`, `[icons]`, `[cursors]`, `[sounds]` took the
  farthest ancestor's instead of the closest, and `[fonts]` now merges field
  by field; an effect giving only `blur` (in a variant) reset the opacity to 1;
  the iced theme's success and warning colours were fixed, dark-theme colours.

- **`hyprdesk-theme-engine use --mode dry-run` no longer applies the theme**
  (MLG-258): it wrote the active theme before the dry run, and the daemon's
  watcher then applied the pack for real.
- **Inherited templates are read from the parent pack** (MLG-259): a pack
  with `[inherits]` looked for its parent's templates in its own folder (and
  lost a parent's `template` when it only redeclared `reload`); templates are
  now looked up from the pack to its ancestors, target declarations merge
  field by field.
- **`generate` covers every file and follows the active variant** (MLG-260):
  files outside `~/.config` (installed GTK theme) and gsettings values made it
  fail; it now names them after their XDG root and lists settings in
  `gsettings.txt`. `generate` and `apply` resolve the active theme's variant
  instead of the system colour scheme.

## [0.2.0] - 2026-11-22

The theme engine at the centre: one pack drives Hyprland, hyprdesk, GTK,
Qt, kitty and iced apps, applied in full from Settings (Hyprland options,
fonts, icons, cursors, sounds), readable on light themes too, with a new
Theme modules page, a LYRA shortcut, touchpad gestures and the advanced
settings reorganised (MLG-142 to MLG-255).

### Added

- **A theme's Hyprland options apply from Settings** (MLG-255). Picking a
  pack that declares `[hyprland.options]` (rounding, gaps, borders, blur,
  animations) now writes them at the top of `appearance.lua`; your own
  window and animation settings, written after, still win.
- **iced applications card** (MLG-252, Settings → Theme modules): hyprdesk
  and iced apps linked to `hyprdesk-theme-client` follow the theme palette
  live; the card shows it (and where `palette.json` lives), with no switch
  since there is nothing to turn off.
- **LYRA shortcut** (MLG-220): `SUPER + SHIFT + L` opens LYRA, the AI
  assistant, and closes it when shown, in every mode (`keys.assistant`,
  Settings → Shortcuts).
- **Touchpad gestures** (MLG-142, Settings → Keyboard and mouse → Gestures): fingers
  and direction bound to an action (switch workspace, special workspace,
  move, resize, close, fullscreen, floating, cursor zoom), written as
  `hl.gesture` in `inputs.lua`; the raw `gestures.*` options follow under
  advanced settings.
- **Per-device settings** (MLG-142, Keyboard and mouse → Devices, advanced): speed,
  acceleration, natural scrolling, left-handed, tap to click, keyboard layout
  or bound screen for one keyboard, mouse, touchpad or tablet (`hl.device`),
  chosen from the connected devices.
- **Layer and workspace rules** (MLG-144, Windows → Rules, advanced) next to window
  rules: the layer rules taken over from `hyprland.lua` were kept but could
  not be seen.
- **Hyprland plugins** (MLG-144, Hyprland configuration → Hyprland plugins,
  advanced): `hyprpm` repositories and plugins, on/off switch, loaded state,
  update and add a repository in a terminal, `hyprpm reload` added to
  startup when missing.
- **Permissions** (MLG-144, Hyprland configuration → Permissions, advanced):
  `ecosystem.enforce_permissions` with its `hl.permission` rules, the
  recommended ones (grim, the portal, hyprdesk, hyprpm) in one click, and a
  warning when enforcement would block every screen capture.
- `hl.gesture`, `hl.device` and `hl.permission` in `hyprland.lua` are taken
  over like options and rules; a gesture whose action is a Lua function stays
  there, and the page names its lines.

### Changed

- **App theming becomes Theme modules** (MLG-253): the Settings menu entry,
  the page and its texts now name what each card is, a module of the theme
  engine (`hyprdesk-theme-engine`) that carries the theme to an
  application. `hyprdesk settings theme-modules` opens the page; the old names
  still work.
- **Advanced settings reorganised and marked everywhere** (MLG-212). The *Effects*
  package becomes Animations → Desktop (desktop transitions, wallpaper
  crossfade; `hyprdesk settings effets` still leads there), Theme → Effects
  becomes Theme → Transparency, so only Animations → Effects keeps the name.
  Curves, Desktop, Transparency, window Groups, Environment, Hyprland plugins,
  Permissions and Devices are advanced tabs. Raw Hyprland options on a basic
  page (cursor, gestures) only show with advanced settings, in a cyan frame;
  the cyan dot also marks tabs and packages in the narrow drawer.
- Mouse page: the “No acceleration” warning pointed to an Advanced tab that
  no longer exists.
- **Advanced settings visible from the menu** (MLG-212): a cyan flask
  replaces the cyan dots; every package that holds advanced settings shows
  the flask and how many (advanced tabs and framed sections) while advanced
  settings are on; the “Advanced” badge carries it too.
- **More settings moved to advanced** (MLG-212): Desktop → Forced modes and
  Dependencies, Performance → hyprdesk engine, AI → Other AI services; fine
  effect settings (rounding shape, dimming strength, blur variant, size,
  passes, vibrancy, noise, shadow and glow range and falloff, wobble
  intensity); window border details (gaps between workspaces, ungrouped
  colours, resize on border, grab area, cursor on border, shadow colour);
  file index exclusions, file types and safety rescan; per-channel gamma,
  calibration file and reference image (Screen colours); adding a
  third-party plugin; each bar module's colour.

- **The theme engine holds the truth** (MLG-231 to MLG-235). The active pack
  and its variant are the reference; the `theme.*` settings mirror them.
  Picking or tweaking a theme in hyprdesk (Settings, wizard, `hyprdesk
  config set theme.*`) sets the matching engine pack, or a `perso` user pack;
  a theme set outside hyprdesk (`hyprdesk-theme-engine use`, LYRA) is copied
  back and every surface follows. The palette is published on every theme
  change, app themes (kitty, GTK, Thunar, Qt) use the active pack,
  Settings → Theme lists the engine packs (installed ones included), and
  `theme.preset` takes a pack name or id.
- **GTK apps follow the theme live, Brave and Firefox included** (MLG-237,
  MLG-248): the GTK target installs your base GTK theme recoloured to the
  pack (`~/.local/share/themes/hyprdesk-<pack>`) and selects it; open GTK 3
  apps reload it at once. Compiled themes such as Dracula-Pro were barely
  affected by redefined named colours. Unchecking GTK or uninstalling puts
  the base theme back; `--restore` also restores `gtk-theme`.
- **Qt colour scheme under a stable name** (MLG-247): `qt6ct/colors/hyprdesk.conf`,
  pick it once in qt6ct; the old per-pack files are removed.
- **Status colours follow the theme**: warning, success and danger (and the
  bar's warning and critical defaults) come from the active pack, readable
  on light themes too (they were fixed colours, 1.5:1 on *Clair*).
- `HYPRDESK_*` theme variables now reach launched applications (they are
  started by Hyprland, not by hyprdesk) and follow theme changes.
- The default theme is exactly the *Ardoise* pack: menus opacity 0.90
  (was 0.94).

### Removed

- **Thunar app theme** (MLG-251): the Thunar card in Settings → App
  theming is gone. The GTK 3 and 4 card already recolours the base GTK
  theme that Thunar follows, so the Thunar-only stylesheet only competed
  with it. The `@import` it left at the top of `gtk-3.0/gtk.css` and
  `hyprdesk-thunar.css` are removed on the next theme pass; `thunar` in
  `app_themes` is still accepted and ignored.

### Fixed

- **State colours on light themes** (MLG-250): success, warning, danger and
  secondary text no longer come out almost black on a light background
  (Clair: `#00821d`, `#a55f00`, `#d72730`, `#636877`). The contrast search
  now moves away from the background and stops at the closest readable
  shade, keeping hue and colour; dark themes are unchanged.
- **Bundled themes follow the theme engine in full** (MLG-254). Clair now
  sets GTK, Brave and Firefox to light mode instead of dark (the pack had no
  `default_variant`; a pack that declares none now takes it from its
  background). Picking a theme in Settings applies everything the pack
  declares (fonts, icons, cursors, sounds, its own targets), a pack with a
  different light or dark version offers it under its tile, and the seven
  packs declare complete metadata. `hyprdesk-theme-engine use` no longer
  copies a bundled pack to your folder; an unchanged copy left by an earlier
  version is ignored, so it no longer hides fixes or shows as “yours”.
- **Install, uninstall and restore with the theme engine** (MLG-230,
  MLG-243 to MLG-246):
  - `hyprdesk-theme-engine` is installed with the other dedicated binaries
    (from source in `~/.local/bin`, in `/usr/bin` by the packages), removed
    by the uninstaller and listed by `hyprdesk doctor`.
  - Uninstalling removes what the theme engine set (GTK 3 and 4 blocks and
    imports, kitty include, Thunar, Qt colours, Hyprland fragments) through
    the new `hyprdesk app-theme remove [--dry-run]`, keeping your own rules;
    GTK apps no longer keep hyprdesk's colours after an uninstall.
  - `--restore` also puts back `gtk-4.0/gtk.css`, Qt colour schemes and the
    GNOME settings the engine sets (colour scheme, accent, font, icons,
    cursor, sounds); an older original state gets them recorded at the next
    install, marked as late.
  - App themes again pass effective options to the engine (`~/` expanded,
    defaults applied): the kitty.conf include is set by default again, and
    an emptied reload command means none.
- **Theme engine** (MLG-221 to MLG-229):
  - `hyprdesk-theme-engine watch` no longer re-applies the theme in a loop
    (it woke on its own state files: about 250 reloads a second); it follows
    every user pack, starts without an active theme, and a second instance
    applies once and exits instead of taking over the socket.
  - The GTK reload sets `color-scheme` and `accent-color` with one
    `gsettings` call each; the accent becomes the nearest GNOME named
    colour (it was always failing).
  - The daemon's file options (`kitty.file`, `kitty.conf`, `thunar.file`,
    `thunar.conf`) and reload commands are honoured again, and a pack's
    declared reload runs for its own targets.
  - Packs can no longer run commands: Hyprland option values must fit on one
    line, variant options go through the allow-list, and rendered
    `hyprland`, `hyprlock` and `kitty` templates are checked line by line;
    generic outputs refuse autostart, services, scripts and launchers.
    Pack options now reach the persistent fragment; `--mode hot` no longer
    writes it.
  - Failed settings roll back to their own previous value, together with the
    files of the same target.
  - Symlinked dotfiles are written through instead of being replaced, and a
    symlinked `~/.config` no longer blocks every write.
  - Archive headers and the decompressed stream are bounded (a crafted
    `.hyprtheme` could ask for gigabytes).
  - `hyprdesk-theme-client` follows `palette.json` without polling.
  - Atomic writes use a per-process temporary file and reach the disk; the
    palette revision is computed under a lock.

## [0.1.9] - 2026-11-21

Safe to try and easy to undo: original state restored in one command, the
whole setup exported, every component back to its defaults, shortcut
conflicts asked instead of guessed, and a lock screen that survives hyprdesk
restarts (MLG-202 to MLG-210).

### Added

- **Back to exactly how it was before hyprdesk.** Before changing anything,
  `install.sh` saves the original state once in
  `~/.local/state/hyprdesk/origine/`: the user files it may touch
  (`~/.config/hypr`, kitty, GTK, default applications, user services), the
  system files (login screen, PAM, keyboard, polkit, GPU) and the packages
  already installed. `./scripts/uninstall.sh --restore` puts all of it back;
  whatever appeared since is moved to `~/hyprdesk-avant-restauration-<date>/`,
  never deleted, and packages installed for hyprdesk are offered for removal
  (`--remove-packages` skips the question). `--dry-run` shows the plan.
- **Shortcut conflicts asked, not guessed.** When a hyprdesk shortcut
  (`SUPER + D`…) is already bound in the Hyprland configuration, a window
  asks for each one: take it for hyprdesk, keep the existing binding (the
  hyprdesk one then only works with the desktop shown), or capture another
  combination. Answers are remembered; a new conflict, after an update or
  because another tool took the combination, is asked again. Installing or
  resetting the shortcuts clears the answers.
- **Setup wizard: backup and going back.** The wizard says when the
  configuration from before hyprdesk was saved, and **Undo the wizard**
  restores the configuration from before it was opened (also in History).
- **Guided tour, block by block.** The first time Settings opens, a tour
  presents each major theme in a few paragraphs while the drawer opens on
  it. Run it again from the **Guided tour** button or with `hyprdesk tour`.
- **Graphics card: one button to apply.** Settings → Performance → Graphics
  card now applies a new choice the right way: restart hyprdesk (a few
  seconds, windows stay open) or log out and back in when Hyprland's card
  changes, after a confirmation.
- **The export carries the whole hyprdesk and Hyprland setup** (MLG-203).
  `hyprdesk export` and Settings → Sharing now include `hyprland.lua`, every
  local file it `require`s and the scripts those files reference. They also
  include My themes, screen calibrations, icon positions and desktop modes,
  and every image the settings point to (icons, lock image, SmartUI icons).
  - On import, the Hyprland files are checked by Hyprland and count as code
    needing consent. `monitors.lua` stays opt-in (`--screens`), and home-folder
    paths are rewritten for the new machine.
  - The theme-only export adds the font, icon theme, app theming and lock
    image.

- **Default configuration for each component** (MLG-202). Every Settings
  page that keeps something in hyprdesk has a **Defaults…** button. On the
  command line, `hyprdesk config reset <component>|all [--yes]` previews the
  reset without `--yes`. A reset does three things:
  - settings go back to the starting desktop;
  - the Hyprland files the component holds are rewritten: `binds.lua`,
    `inputs.lua` (keyboard layout kept), `monitors.lua` and `autostart.lua`;
  - remembered state is forgotten.

  The full reset also clears kept notifications and sets custom themes and
  widgets aside. The reset no longer replaces `hyprdesk.lua`.

### Changed

- **Dedicated binaries for authentication.** The lock screen
  (`hyprdesk-lock`), the login screen (`hyprdesk-greeter`) and the polkit
  agent (`hyprdesk-polkit`) are now separate executables installed next to
  `hyprdesk`. Each one only links its own code: no daemon, no Lua, no file
  index (about 27–32 MB stripped instead of 69 MB). PAM and fprintd code
  lives in the `hyprdesk-auth` crate and translations in `hyprdesk-i18n`,
  neither of which depends on iced or Lua. `hyprdesk lock`,
  `hyprdesk greeter` and `hyprdesk polkit-ui` keep working: they hand over
  to the dedicated binary when it is installed, and run in-process
  otherwise. hypridle, the lid action, the polkit component and the greetd
  configuration launch the dedicated binaries directly; the greeter binary
  must pass the same root-ownership check as `hyprdesk`.

- **README rewritten as a short overview** (MLG-210). A new description,
  the major features up front, then installation and a quick start. Every
  detail moved to a user guide, [docs/GUIDE.md](docs/GUIDE.md) and
  [docs/GUIDE.fr.md](docs/GUIDE.fr.md).
- **The Panorama no longer shows its layout bar by default** (MLG-209). The
  1 | 1×2 | 1×3 | 2×2 | 3×3 | ✦ selector and the hide button are now off
  unless *Show strip screen controls* is on in the Panorama settings
  (`panorama.design.controls`). Split layouts now honour that setting too;
  before, they always showed the selector in the last zone's header.

### Fixed

- **Lock screen killed when hyprdesk restarts.** Started by hypridle, the
  lock screen lived in `hyprdesk.service`'s cgroup: an update or a restart of
  the service killed it, leaving Hyprland locked with no way to unlock (seen
  on wake-up). It now moves into its own systemd scope
  (`hyprdesk-lock-<pid>.scope`).
- **Lock screen waiting forever after a refused lock.** When Hyprland
  refused the lock (`ext_session_lock_v1.finished`), `hyprdesk lock` kept
  running without locking and held its instance lock, so the session could
  never be locked again. The event is now handled (vendored `sessionlockev`)
  and the process exits.

## [0.1.8] - 2026-11-21

Composable Panorama, `binds.lua` managed by hyprdesk, fingerprints from
Settings, and shortcut and screenshot fixes (MLG-160 to MLG-207).

### Changed

- **App theming on by default** (MLG-207). Every supported application
  (kitty, Thunar) takes hyprdesk's theme from the start; unchecking them all
  in Settings → App theming (`app_themes = {}`) is kept as “none”.
- **Default configuration: screenshots, SUPER + drag, 5 workspaces**
  (MLG-204). A new “Screenshots” essential binds Print (area),
  SUPER + Print (screen) and ALT + Print (window) to `hyprdesk capture`,
  wherever hyprland.lua has no screenshot binding of its own (hyprshot,
  grimblast or `hyprdesk capture`). With the desktop shown, moving and
  resizing windows with SUPER + mouse works again: the submap no longer
  suspends the hyprland.lua binding, hyprdesk binds it there too (same for
  the other essentials that make sense on the desktop). The starting desktop
  keeps 5 permanent workspaces.
- **`hyprdesk.lua` optional, sample fully commented** (MLG-199). A first
  installation writes the starting desktop (icons, widgets, bar) to
  `settings.lua`, adjustable in Settings, and a `hyprdesk.lua` with comments
  only. `hyprdesk.lua` is no longer recreated once `settings.lua` exists, and
  the configuration loads without it. `config reset` puts the starting
  desktop back instead of emptying the settings.
- **`binds.lua` managed by hyprdesk** (MLG-198). It is regenerated from what
  it really defines (variables replaced by their value, loops unrolled, one
  binding per line; section comments, commented-out code and disabled
  bindings kept; a binding whose action is a Lua function is kept as
  written). A hand-written file is copied once, then taken over at startup;
  Hyprland checks every write and everything is put back on a new error.
  `hyprdesk binds generate [--write]` previews or applies it.
- **Settings: English identifiers, consistent listing** (MLG-193). The
  bundled widgets are renamed `network`, `battery`, `keyboard`, `player`,
  `updates`, `nightlight`, `removable` and `voice` (were `reseau`,
  `batterie`, `clavier`, `lecteur`, `majs`, `nuit`, `supports`, `voix`);
  values `apps_style = sheet | spotlight`, `bar.style = continuity |
  convex`, `workspace_selector = thumbnails | icons`, `perf.profile =
  balanced | quality`; key `panorama.stream_mood` (was
  `flux_ambiance`). The old names are still read everywhere (hyprdesk.lua,
  settings.lua, replacement files `<old>.lua`, `hyprdesk config`) and
  rewritten on the next save. `hyprdesk config list` now shows booleans as
  `on | off`, describes every key (colours, effects, layouts, widgets,
  packages…), prints `—` for any unset value and lists each key once.

### Added

- **Fingerprints from Settings.** Settings → Lock enrolls and deletes your
  fingerprints with fprintd, without leaving hyprdesk.
- **Panorama: profiles for the shortcuts block** (MLG-172). The shortcuts
  block opens with a selector of four coloured icons — Video, Music, Remote,
  Settings (`panorama.design.shortcut_profile = video | audio | remote |
  system`). Video and Music bring their own composition: the menu on the
  left, the player at 16:9 over the full height (720p on a 2560×720 strip),
  then a playlist column of the services (video sources, or the new
  `music_sources`: Spotify, Deezer, YouTube Music, Qobuz, Radio France).
  Their keys are built in (±10 s, play, fullscreen, volume, mute, next;
  previous/next, output, microphone); Remote is a directional pad whose
  arrows, OK and Back go to the active window (`hyprdesk key <key>
  [mods]`); Settings keeps the personal shortcuts and composition.
  `hyprdesk panorama icons music` has SmartUI draw the music services.
- **Panorama background image: Browse…** opens the file chooser (portal).
- **Panorama: composable pages** (MLG-172). After the default view, the
  carousel holds pages, each with its own composition (`pages = { { name =
  "System", tiles = { … } } }`); by default a System page with six metric
  graphs. Settings → Panorama → Pages adds, renames, orders and removes
  pages; Compose edits any of them in the Composition tab. New tiles:
  `metric:<cpu|ram|disk|load|uptime|cpu_temp|gpu_temp>[:gauges|cards]`
  (one metric per tile) and `playlist`. Old widget-name pages still load.
  The default layout is now the composition.
- **Panorama metrics: colours, smooth curves, NVIDIA** (MLG-172). Each
  metric has its own colour (CPU cyan, RAM violet, GPU green…) in graphs
  and gauges; curves are smoothed (Catmull-Rom), with a soft glow and a dot
  on the latest value; they scroll continuously (30 fps while graphs are on
  screen) instead of jumping every second, and slide out on the left. NVIDIA cards (proprietary driver, absent from hwmon)
  are read through `nvidia-smi`: new `gpu` (load) and `vram` metrics, and
  `gpu_temp` falls back to it. The System page shows eight graphs.
- **LYRA voice** (MLG-172). A microphone button in the Panorama (next to
  the profiles) turns it on: hyprdesk listens for the word “Lyra” (Vosk,
  small French and English models, offline), sends the request to LYRA —
  its drawer opens — and reads the answer aloud with Piper (local neural
  voices, French siwis and English lessac), in the language of the request;
  listening pauses while LYRA speaks. Commands are approved on their own in
  voice mode; only dangerous ones (high risk) are asked aloud, and
  “yes”/“no” answers. Settings → Panorama → LYRA voice: on/off, the
  microphone (PipeWire, `panorama.voice_mic`), a voice test. Everything installs on first use in
  `~/.local/share/hyprdesk/voice` (~350 MB, no root): `hyprdesk voice setup
  | say [--en] <text> | status`; `panorama.voice` (on/off), `hyprdesk
  panorama voice on|off|toggle|ask <fr|en> <text>`.
- **Webradio “voice stream” bar, task manager, LYRA halo** (MLG-172).
  Radios (favourites, Radio Browser results, stream addresses) now play in
  mpv without a window; the player zone shows a “voice stream” bar — the
  sound scrolls in mirrored bars over a background that breathes with the
  bass, mids and highs (read from the PipeWire output monitor); three moods,
  Aurora, Sunset, Ocean (`panorama.flux_ambiance`). Play/Pause and Close
  drive mpv. Tapping a metric tile opens a task manager over the page:
  running programs sorted by CPU, memory, GPU, VRAM (`nvidia-smi pmon`) or
  disk, with “End” (second tap confirms, own programs only). LYRA voice:
  a dedicated wake detector for “Lyra”, automatic gain for distant
  microphones, the last heard sentence shown on the microphone button, and
  a conversation halo around the screens while LYRA listens, thinks or
  speaks (`panorama.voice_halo` screens | panorama | off,
  `panorama.voice_halo_color` lyra | accent). LYRA's context now covers
  every Panorama setting and command.
- **Panorama: Remote profile removed** (MLG-172). The shortcuts block keeps
  Video, Music and Settings; a configuration still on `remote` opens the
  Video profile. In a vertical strip the player is 16:9 across the width
  and the playlist takes the remaining height.
- **Panorama zone on an ordinary screen** (MLG-172). Settings → Screens →
  a screen → “Set a Panorama zone”: horizontal (top, bottom) or vertical
  (left, right) strip, 10 to 50 % of the screen. The Panorama sits in that
  strip with an exclusive zone, so windows — maximised ones too — stop
  before it, as on a second screen; the screen keeps its bar and desktop
  (`panorama.zone = { screen, side, size }`, CLI `panorama.zone
  DP-2:bottom:0.25 | none`). The task manager header is fixed, on a band,
  with the sorted column marked; rows alternate.
- **Panorama: no dead ends** (MLG-172). Page changes slide in from the
  side of the gesture; when a video or a radio plays out of sight (another
  page, the Settings profile), a “now playing” pill (title, ⏯, ✕) stays at
  the bottom; the Remote profile keeps the player zone and its keys go to
  the player; the task manager shows the metric on a third and the list on
  two thirds, and tapping the metric (or ✕) goes back
  (`hyprdesk panorama tasks <metric>|close`). A composition without a
  shortcuts block (Settings profile) shows a compact profile bar, so the
  other profiles stay one tap away; a radio playing when hyprdesk restarts
  (update) resumes on its own.
- **Panorama sources configured in Settings, only usable platforms
  shown** (MLG-172). Settings → Panorama → Sources edits the three lists
  (Video, Music, Webradio): add, order, remove, icons, default list. The
  Panorama only shows platforms that need no account and those you are
  signed in to — read from the browser profile (session cookie names,
  never values; Netflix, Spotify, Deezer, Prime Video, Max, Canal+…);
  platforms whose sign-in cannot be read stay visible. Each source can be
  forced (`show = "on" | "off"`), and “Sign in” opens it in the browser.
- **Playing a new media stops the previous one**: every MPRIS player still
  playing is paused before a Panorama source starts.
- **Panorama Music: Playlist and Webradio tabs, music on request through
  LYRA** (MLG-172). The Music column has two tabs: the music services, and
  Webradio — favourite radios (FIP, France Inter, franceinfo, France
  Culture, France Musique, Nova, TSF Jazz by default, ★ to keep a station)
  and the latest search in the free Radio Browser directory, or a web
  service opened in the player (Radio Garden, radio.net, TuneIn; Settings →
  Panorama, `panorama.webradio`, `panorama.radios`). LYRA plays on request:
  `hyprdesk panorama music <search>` opens the results page of the active
  platform, `hyprdesk panorama radio <name or genre>` plays the first
  matching station, `hyprdesk panorama open <url> [name]` opens an
  address. The profile buttons show their name below on hover.
- **Panorama: missing source icons are drawn automatically** (MLG-172).
  A video or music source without an icon gets one from SmartUI in the
  background (once per session and source), shown as soon as it is ready.
- **Panorama player: one source at a time, framing and memory per source**
  (MLG-172). Opening a source replaces the previous one, even while it is
  still opening (its window is closed when it shows up); tapping the playing
  source again does not relaunch it; a player window left by a previous
  hyprdesk is taken back or closed. Frame (services column) zooms and moves
  the window behind the player zone so only the video shows — the strip
  hides the rest — remembered per source (`zoom`, `dx`, `dy`). When a
  source starts playing, its video goes fullscreen inside the player zone
  on its own: hyprdesk finds the video (the region that moves between
  three captures of the window), then tries a double-click on it, the F
  key, and the usual places of the fullscreen button, checks each try, and
  remembers what worked for the source (`fs`); pages without a video
  (music) are left alone. `hyprdesk panorama fullscreen` does it on
  demand. The Fullscreen key goes to the player window. The Play key turns into Pause while
  playing; the page dots sit at the bottom centre, translucent without a
  recent touch.
- **Screens split into zones** (MLG-172). Wayland cannot cut one output into
  several logical monitors, so hyprdesk does it with a Hyprland Lua layout
  (`lua:hyprdesk-zones-…`): Settings → Screens → Screen → Zones offers 1,
  1×2, 1×3, 2×2 and 3×3, with zone 1 at 1/2 or 1/3 of the long side in 1×2
  and 1×3, and draws the split screen. Each window keeps its zone (the one
  under the mouse when it opens) and is tiled there; maximizing or going
  fullscreen (shortcut or the app itself) only covers its zone — the app
  believes it is fullscreen (client-only fullscreen state). SUPER + CTRL +
  SHIFT + arrow sends the window to the neighbouring zone. Saved as
  `hd.config({ screen_zones = { ["DP-2"] = "1x2:1/3" } })`, written to
  `layouts.lua` and applied live. The Panorama gets the same splits
  (`layout = "1x2" | "1x3" | "2x2" | "3x3"`, `first = "1/2" | "1/3"`, up to
  nine zones), and its Video zone keeps the browser window in its frame when
  it goes fullscreen.
- **Panorama: every layout keeps its own settings, gestures, SmartUI icons**
  (MLG-172). Each split (1×2, 1×3, 2×2, 3×3) keeps its zone widgets and
  zone-1 share (`zones = { ["2x2"] = { panes = {…} } }`, CLI
  `panorama.zones.<layout>.panes|first`); Settings → Panorama → Pages
  configures any layout without activating it. The composed view (split or
  free composition) is the first entry of a carousel followed by the pages:
  swipe sideways between them; swipe up opens a layout picker (one tile per
  layout with its diagram), swipe down hides the Panorama. Video sources are
  square tiles with an icon (initial until one exists); `hyprdesk panorama
  icons [sources|shortcuts|all] [--force]` — or the Settings button — has
  SmartUI (LYRA by default) draw the source and shortcut icons and writes
  them to the config. Sources and touch shortcuts can be reordered;
  `panorama.sources` is a CLI key; LYRA's context lists every Panorama key
  and command, so a plain request is enough.
- **Render card for Hyprland and hyprdesk** (MLG-171, Settings → Performance →
  Graphics card). Hyprland draws every frame on one card; when the screens
  are plugged into another one, every frame is copied across (an iGPU at
  full load for two 1440p 144 Hz screens on the discrete card). The tab
  lists the cards (driver, connected screens, the one rendering now, read
  from Hyprland's log), warns when the screens are on another card, and
  sets `hyprland_gpu` (PCI address): `env.lua` finds the card by that
  address at every Hyprland start (`cardN` numbers change, `by-path` names
  contain the `:` separator) and puts it first in `AQ_DRM_DEVICES`, the
  others after it; a missing card (vfio) falls back to the others. hyprdesk
  gets a new default, **Like Hyprland**: it renders on the same card
  (`HYPRDESK_WGPU_PCI`, adapter picked by PCI id in the vendored
  iced_wgpu), so its surfaces are never copied either. Also `hyprdesk
  config set hyprland_gpu <0000:01:00.0|auto>` and `gpu hyprland`.
- **Composable Panorama.** Configurable forest, telemetry, studio and applications
  compositions: freely positioned panes, background images, colors, fonts,
  transparent cards, analog/flip clocks, real gauges and history graphs,
  calendar, vinyl player and editable touch shortcuts. Settings, Lua and
  `panorama.design.*` CLI keys share validation and persistence; see
  [Panorama configuration](docs/PANORAMA.md).
- **Cinematic Panorama.** A theme-colored surface with a perspective floor,
  projected 3D crystal and orbiting lights, glass artwork and cards, magnetic
  controls, spring page transitions and swipe snap-back. Local animation stops
  when idle; `transition = "none"` disables motion. Isolated preview scenes
  cover media, system, clock, split layout, video sources and reduced motion.
- **Panorama for strip screens** (MLG-164). A very wide screen (aspect
  ratio of 2.5:1 or more, landscape or portrait: 2560×720, 1920×480,
  1280×400…), touch or not, gets a full-screen Panorama: pages you swipe
  with a finger or the mouse (media player over MPRIS, system, clock), page
  dots, and an idle clock after `sleep_minutes` without playback or touch.
  No screen model is hard-coded: hyprdesk offers the role once when such a
  screen shows up, ties the touch device to it (screen maker in the device
  name, a single candidate, or the screen where “Yes” is tapped) through
  `inputs.lua`, and remembers a refusal by EDID id, not by port. The
  Panorama surface never takes keyboard focus, the bar leaves that screen,
  and it can be tucked away behind a small handle to use the screen for
  windows. **Settings → Panorama** (in Bar and launcher), `hd.config({
  panorama = { screens, pages, declined, sleep_minutes } })`,
  `hyprdesk config set panorama.*` and `hyprdesk panorama
  [toggle|show|hide|page <n|id>|detect]`. A **preview** shows the Panorama
  in a floating window with the live player and system, to try it without
  a strip screen: Settings → Panorama → Preview (2560×720, 1920×480,
  1280×400, 720×2560 portrait) or `hyprdesk panorama preview [WxH]`; the
  window can be resized and the layout follows. **Split screen**: one page
  at a time, or the screen split into 2 or 3 zones (1 | 2 | 3 buttons on
  the screen, `layout = "split2"`), each zone with its own widget chosen
  with ◀ ▶ (media, system, clock, video). The new **Video** widget opens a
  service (Free TV, Netflix, Disney+, HBO Max, Canal+, Prime Video,
  YouTube; list editable in Settings → Panorama) in a Chromium browser app
  window with the usual profile, then floats, pins and places it exactly
  in its zone, borderless; the Panorama surface leaves a hole there
  (picture and input). Also in the preview window. `hyprdesk panorama
  layout <single|split2|split3>`, `hyprdesk panorama video <n|name|close>`,
  `hyprdesk config set panorama.layout|panes|browser`.
- **GPU switch in the bar** (MLG-162). On a passthrough machine, the new
  `gpu` widget shows the NVIDIA GPU reserved for VMs (VFIO, VM when a VM
  holds it, NVIDIA) and switches it live between vfio-pci and the nvidia
  driver (CUDA, NVENC, containers), without rebooting. Every switch asks
  whether it is for this session only or also for the next boots; a boot
  choice is kept in `/etc/hyprdesk/gpu-boot` and applied early by
  `hyprdesk-gpu.service`, the kernel command line is never edited. Two
  nvidia modes: compute only (nvidia_drm stays unloaded, so going back to
  vfio-pci works live) and display (DISPLAY in the bar: nvidia_drm is
  loaded and the screens plugged into the card light up, Hyprland picks
  them up live). In display mode Hyprland holds the card, so the menu only
  offers to change the driver for the next boot. nvidia is released with
  rmmod. Also `hyprdesk gpu [status | nvidia | display | vfio | toggle]
  [--boot | --next-boot]` (root part through pkexec) and `hd.gpus()` for
  plugins. Hidden without passthrough; added once to the bar.
- **Update progress in the bar** (MLG-161). While an update runs, the
  updates widget shows its progress instead of “…”: “42 %” in compact mode,
  a thin gauge and the percentage in detailed mode (option `detaille`,
  Settings → Plugins → Updates → Configure); the tooltip and the menu give
  the source, the phase (downloading, checking, installing, finishing) and
  the package in progress. `hyprdesk updates --upgrade` follows the package
  managers' output as it runs (pacman and yay / paru, apt, dnf, zypper,
  flatpak); the progress never goes backwards. Plugins can draw the same
  gauge by returning `progress` (0 to 1) from `render`.
- **One password per update** (MLG-160). An optional polkit rule
  (`/etc/polkit-1/rules.d/50-hyprdesk-updates.rules`) keeps the pkexec
  authorisation of package managers (pacman, apt-get, dnf, zypper) for
  5 minutes for the same process: the steps of one update, or yay / paru
  calling pkexec several times, ask for the password once. Any other
  command still asks; the password is kept nowhere. Off by default; switch
  in Settings → Plugins → Updates → Configure, in the widget menu, or
  `hyprdesk updates --polkit on|off`.

### Fixed

- **One screenshot at a time** (MLG-207). Pressing a capture shortcut again
  while an area is being selected (or the screen is frozen) no longer stacks
  a second capture on top: the extra press is ignored. The next capture is
  possible as soon as the selection ends, without waiting for the
  notification.
- **Shortcuts no longer written twice** (MLG-206). When the original lines of
  `binds.lua` (clipboard script, power menu, hyprshot…) were switched back
  on, they were replaced again by their hyprdesk equivalent, already
  written: SUPER + V, SUPER + W, SUPER + SHIFT + Q and SUPER + Print ended up
  twice and ran twice (a menu opened and closed at once). The original is now
  only switched off when its hyprdesk equivalent is there, and
  `hyprdesk binds sync` (run at startup) removes a hyprdesk shortcut written
  twice.
- **Conflict with a shortcut set at runtime** (MLG-206). A shortcut bound
  in Hyprland but written in no file was reported as coming from
  `hyprland.lua`. `hyprdesk keys` and Settings → Shortcuts now say it is
  found in neither `hyprland.lua` nor `binds.lua`, and that `hyprctl reload`
  removes it.

## [0.1.7] - 2026-11-18

Lock screen fix release (MLG-159).

### Fixed

- **The lock screen fits a Panorama strip** (MLG-164). On a short or very
  wide screen (2560×720, 1920×480…), the lock screen's column (clock, user,
  password) overflowed; it now lays out in a row — clock on the left, user
  and password on the right — with a smaller avatar and drift.
- **Identify markers leave with Settings** (MLG-147). Closing Settings left
  the Screens → Identify markers on every screen until their two-minute
  timeout; they are now removed with it.
- **The lock screen accepts the right password again** (MLG-159). Waking
  from sleep, the lock screen refused the correct password. Without
  `/etc/pam.d/hyprdesk-lock` (build from source: only the package installs
  it), hyprdesk fell back on the `hyprlock` PAM service, whose Arch file only
  declares `auth include login`. Since MLG-153, `pam_acct_mgmt` runs after
  authentication: with no account phase, Linux-PAM takes the one of `other`
  (`pam_deny`), hence a systematic refusal. hyprdesk now falls back on
  `hyprlock` only when its file declares an account phase (`account`,
  `-account`, `@include`), otherwise on `login`; `install.sh --from-source`
  installs `/etc/pam.d/hyprdesk-lock` like the package, never overwriting it.

## [0.1.6] - 2026-11-17

Performance and robustness release (MLG-126, MLG-156 to MLG-158).

### Added

- **The file index starts with hyprdesk** (MLG-157). New setting
  `index.autostart` (on by default; Settings → Index): the `:f` index daemon
  is launched a few seconds after the session opens, at low CPU and I/O
  priority, instead of on the first search. Off: started on the first `:f`,
  as before.

### Fixed

- **"The index daemon is not responding" on the first `:f`** (MLG-157). The
  daemon loaded its snapshot (hundreds of MB in memory) before opening its
  socket, and the launcher gave up after 3 s under memory pressure although
  the daemon went on starting. The socket now opens first, the snapshot loads
  in the background (also after the idle unload), and the launcher shows
  "Loading the index…" and asks again on its own. A single daemon per session
  (lock file), launches serialised, and its exit is reaped: no more zombie
  `hyprdesk` process.

- **Drawers open at once** (MLG-156). Opening the LYRA or Settings drawer
  could freeze the desktop for up to 4 s: iced re-read every system font for
  each new surface (its SVG cache is per renderer). The SVG font database is
  now shared by the whole process and loaded only for an SVG that contains
  text; icons no longer need it. Local copies `vendor/iced_wgpu` and
  `vendor/iced_tiny_skia`, checked by `scripts/verifier-vendor.sh`.

- **Tray icons no longer cost a fifth of a CPU core** (MLG-126). An
  application publishing only a 1024 × 1024 icon (ZCode, Electron) turned
  into four million D-Bus values, cloned and converted every 2 s. The
  StatusNotifierItem properties are now decoded straight into typed fields;
  a property of an unexpected type is read on its own and only it is
  ignored.

- **A replugged screen comes back to its workspace** (MLG-158). A dedicated
  screen (say HDMI-A-2 → workspace 6) unplugged then plugged back landed on
  a new workspace 11: Hyprland picks a workspace for an arriving screen
  before hyprdesk puts back its default rule, and takes the first free
  number not bound to another screen (7 to 10 being bound to the primary).
  After a screen is plugged in during the session, hyprdesk now brings it
  back to its default workspace; the windows of the stray workspace follow,
  and focus returns to the window that had it.

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
