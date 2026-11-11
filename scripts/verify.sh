#!/usr/bin/env bash
# Boucle de vérification reproductible, sans démarrer le démon de la session.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
if command -v dbus-run-session >/dev/null && [[ "${HYPRDESK_SKIP_DBUS:-0}" != "1" ]]; then
    dbus-run-session -- cargo test --locked tray::tests::native_watcher_lifecycle -- --ignored --test-threads=1
    dbus-run-session -- cargo test --locked notifications::tests::serveur_sur_le_bus -- --ignored --test-threads=1
fi
if [[ "${1:-}" == "--visual" ]]; then
    output="${2:-/tmp/hyprdesk-visual}"
    for width in 1440 900 640; do
        xvfb-run -a -s '-screen 0 1600x1000x24' env -u WAYLAND_DISPLAY \
            WINIT_UNIX_BACKEND=x11 WGPU_BACKEND=vulkan HYPRDESK_PREVIEW_WIDTH="$width" \
            target/debug/hyprdesk preview "$output/$width"
        for scene in desktop typography icons wallpaper widgets bar effects volume calendar workspaces screens launcher apps assistant plugins share launcher-idle launcher-hover launcher-top launcher-left launcher-right launcher-popup chat theme bureau launcher-opening launcher-menu-opening launcher-launching launcher-bright launcher-dark launcher-menu-long layouts tour veille osd animations animations-effets polkit notifications probleme recherche presse-papiers son energie date-heure; do
            test -s "$output/$width/$scene.png"
        done
    done
fi
