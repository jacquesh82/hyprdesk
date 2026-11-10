#!/usr/bin/env bash
# Installation de hyprdesk sur la machine locale :
#   1. build release + binaire dans ~/.local/bin
#   2. config d'exemple ~/.config/hypr/hyprdesk.lua si absente — jamais écrasée
#   3. définitions LuaLS à côté (autocomplétion)
#   4. unit systemd user (WAYLAND_DISPLAY détecté) + enable --now
# Ne touche PAS à hyprland.lua : les lignes à ajouter sont affichées à la fin.
set -euo pipefail
cd "$(dirname "$0")/.."

BIN_DIR="$HOME/.local/bin"
UNIT_DIR="$HOME/.config/systemd/user"

echo "== build release"
cargo build --release --locked

echo "== binaire -> $BIN_DIR"
install -Dm755 target/release/hyprdesk "$BIN_DIR/hyprdesk"

echo "== configuration"
"$BIN_DIR/hyprdesk" init
"$BIN_DIR/hyprdesk" stubs
"$BIN_DIR/hyprdesk" check

echo "== unit systemd user"
WL_DISPLAY="${WAYLAND_DISPLAY:-wayland-1}"
mkdir -p "$UNIT_DIR"
sed "/^\[Service\]/a Environment=WAYLAND_DISPLAY=$WL_DISPLAY" \
    assets/hyprdesk.service > "$UNIT_DIR/hyprdesk.service"
systemctl --user daemon-reload
systemctl --user enable --now hyprdesk.service
sleep 1
systemctl --user --no-pager --lines=0 status hyprdesk.service || true

echo
echo "=================================================================="
echo " hyprdesk installé — à ajouter dans ~/.config/hypr/hyprland.lua :"
echo "=================================================================="
"$BIN_DIR/hyprdesk" snippet | grep -v '^hl.exec_cmd("hyprdesk")'
echo
echo "hyprdesk dessine le fond d'écran : retirez hyprpaper de l'autostart."
