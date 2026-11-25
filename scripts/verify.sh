#!/usr/bin/env bash
# Boucle de vérification reproductible, sans démarrer le démon de la session.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --locked --workspace
if command -v dbus-run-session >/dev/null && [[ "${HYPRDESK_SKIP_DBUS:-0}" != "1" ]]; then
    dbus-run-session -- cargo test --locked tray::tests::native_watcher_lifecycle -- --ignored --test-threads=1
    dbus-run-session -- cargo test --locked notifications::tests::serveur_sur_le_bus -- --ignored --test-threads=1
fi
if [[ "${1:-}" == "--visual" ]]; then
    output="${2:-/tmp/hyprdesk-visual}"
    for width in 1440 900 640; do
        xvfb-run -a -s '-screen 0 3000x2000x24' env -u WAYLAND_DISPLAY \
            WINIT_UNIX_BACKEND=x11 WGPU_BACKEND=vulkan HYPRDESK_PREVIEW_WIDTH="$width" \
            target/debug/hyprdesk preview "$output/$width"
        for scene in desktop typography icons wallpaper widgets bar effects volume calendar workspaces screens launcher apps assistant plugins share launcher-idle launcher-hover launcher-top launcher-left launcher-right launcher-popup chat theme bureau launcher-opening launcher-menu-opening launcher-launching launcher-bright launcher-dark launcher-menu-long layouts tour veille osd mode-launcher mode-hidden animations animations-effets polkit notifications notifications-resume probleme recherche presse-papiers reglages son energie performance date-heure reseau bluetooth menu-icone proprietes menu-dock-long fenetres connexion luminosite plugin-config applications lanceur-central clavier-souris ecran-connexion mots-de-passe ia-services smartui a-propos raccourcis raccourcis-essentiels bar-idle bar-hover bar-pressed bar-workspace-switch bar-controls bar-notifications bar-notifications-clavier acces-clavier bar-menu-switch bar-menu-closing bar-menu-shell bar-menu-help bar-menu-workspaces bar-menu-window bar-bottom bar-overflow bar-still bar-graphite-idle bar-graphite-pressed bar-graphite-switch bar-graphite-bottom bar-graphite-urgent bar-graphite-edit bar-island-idle bar-island-hover bar-island-bottom bar-island-still bar-island-pressed bar-island-switch bar-island-notif bar-island-activity bar-island-urgent bar-convex-idle bar-convex-pressed bar-convex-switch bar-convex-urgent bar-convex-controls bar-convex-calendar bar-convex-opening bar-convex-bottom bar-convex-still bar-convex-merge bar-convex-settings bar-convex-launcher bar-convex-sides bar-convex-dock-shown bar-convex-notification controls-desktop overview menu-fenetre menu-espace bar-menu-notification bar-menu-module bar-selector-relief bar-selector-rail bar-selector-miniatures bar-selector-icones bar-selector-notif reglages-valeur-saisie reglages-notifications forcer-quitter forcer-quitter-confirmation reglages-echelle widgets-tailles widgets-petits; do
            test -s "$output/$width/$scene.png"
        done
    done
fi
