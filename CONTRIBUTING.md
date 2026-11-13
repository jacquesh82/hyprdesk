# Contributing to hyprdesk

This page covers the practical parts: getting a build, the verification
loop, and what a pull request is checked against. For how the daemon works,
read [docs/ADVANCED.md](docs/ADVANCED.md) first.

## Prerequisites

- **Rust 1.99 or newer** (edition 2024).
- Build dependencies of your distribution (pkg-config, Wayland, xkbcommon,
  fontconfig, EGL…). `scripts/deps.sh --dry-run` lists what is missing on
  your machine; the [README](README.md#installation) installs everything.
- **Hyprland 0.57 or newer is needed to run the daemon, not to build it.**
  `cargo test` and `cargo build` work outside any graphical session.

## Build and test

```bash
cargo test --locked
```

Always pass `--locked`: `Cargo.lock` pins `winit-core` to `0.31.0-beta.2`
because beta.4 breaks a dependency of `iced_layershell`.

The full verification loop, without starting the session daemon:

```bash
bash scripts/verify.sh
```

It runs, in order:

1. `cargo fmt --all -- --check`
2. `cargo clippy --locked --all-targets -- -D warnings`
3. `cargo test --locked`
4. `cargo build --locked`
5. If `dbus-run-session` is available, two tests that need a session bus
   (tray watcher, notification server) under an isolated D-Bus. Set
   `HYPRDESK_SKIP_DBUS=1` to skip them when the tool is missing.

`bash scripts/verify.sh --visual` adds a screenshot of every view at three
widths under `xvfb-run` (needs Xvfb). To try the interface without touching
your session, `cargo run --locked -- preview` runs the real views on demo
data: no IPC, no persistence, no launches.

The CI runs `bash scripts/verify.sh` on every push and pull request.

## Commit messages

- Written in **French**.
- **One concern per commit.**
- Prefixed with `hyprdesk : ` for the daemon itself, or with the component
  when the change belongs to one: `myMFA : …`, `README : …`.

## Translations

French is the source language and the key at the same time, as in gettext:
`tr("Afficher la barre")` displays “Show bar” in English, and the French
text itself otherwise. The English catalog lives in
[locales/en.toml](locales/en.toml) and is embedded in the binary: there is
no runtime translation file.

- The `catalogue_complet` test in [src/i18n.rs](src/i18n.rs) **fails when a
  marked string has no English entry**. Strings are collected from Rust and
  Lua sources, so a new label breaks the build until it is translated.
- To get the missing entries as a starting point:
  `HYPRDESK_I18N_TODO=/tmp/todo.toml cargo test --locked catalogue_complet`
  writes every missing key with an empty value (the test still fails).
  Translate the values and append them to `locales/en.toml`.
- Help lines, generated Lua comments and the bundled examples have their own
  tests in the same file; they fail on untranslated text too.

Lua plugins translate with `hd.t("texte")` — see
[docs/PLUGINS.md](docs/PLUGINS.md), § 9.

## Plugins and widgets

Settings bundles, widget plugins and the launcher follow one design,
documented in [docs/PLUGINS.md](docs/PLUGINS.md). To write a widget plugin,
read [docs/WRITING_PLUGINS.md](docs/WRITING_PLUGINS.md) first: it walks
through the API (`hd.widget`, `hd.t`, `hd.exec`), the sandbox limits and the
test story.

## Pull requests

1. Fork the repository and work on a **branch** (one concern per PR).
2. Open the PR against `main` and describe **what changes and why**, with
   screenshots when the visible interface moves.
3. `bash scripts/verify.sh` must pass. If your change touches the English
   catalog or the Lua examples, say so in the description.
4. New behaviour needs a test; visible changes need a line in the README or
   in `docs/`.

## Licensing

By contributing, you agree that your contributions are licensed under the
MIT License, like the rest of hyprdesk.
