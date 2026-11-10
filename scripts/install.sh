#!/usr/bin/env bash
# Installation de hyprdesk sur la machine locale :
#   1. build release + binaire dans ~/.local/bin
#   2. police Mindlog dans le dossier de polices utilisateur
#   3. config d'exemple ~/.config/hypr/hyprdesk.lua si absente — jamais écrasée
#   4. définitions LuaLS à côté (autocomplétion)
#   5. unit systemd user (WAYLAND_DISPLAY détecté) + activation et redémarrage
# Ne touche PAS à hyprland.lua : les lignes à ajouter sont affichées à la fin.
set -euo pipefail
cd "$(dirname "$0")/.."

BIN_DIR="$HOME/.local/bin"
UNIT_DIR="$HOME/.config/systemd/user"
FONT_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/fonts/hyprdesk/Mindlog"

echo "== build release"
cargo build --release --locked

echo "== police Mindlog -> $FONT_DIR"
mkdir -p "$FONT_DIR"
install -m644 assets/fonts/mindlog/*.ttf assets/fonts/mindlog/OFL.txt "$FONT_DIR/"
if command -v fc-cache >/dev/null 2>&1; then
    fc-cache -f "$FONT_DIR"
fi

echo "== binaire -> $BIN_DIR"
mkdir -p "$BIN_DIR"
HYPRDESK_INSTALL_STAGE=$(mktemp "$BIN_DIR/.hyprdesk-install.XXXXXX")
trap 'rm -f -- "$HYPRDESK_INSTALL_STAGE"' EXIT
install -m755 target/release/hyprdesk "$HYPRDESK_INSTALL_STAGE"
mv -f -- "$HYPRDESK_INSTALL_STAGE" "$BIN_DIR/hyprdesk"
trap - EXIT

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
systemctl --user enable hyprdesk.service
systemctl --user restart hyprdesk.service
sleep 1
systemctl --user --no-pager --lines=0 status hyprdesk.service || true

echo
echo "=================================================================="
echo " hyprdesk installé — à ajouter dans ~/.config/hypr/hyprland.lua :"
echo "=================================================================="
"$BIN_DIR/hyprdesk" snippet | grep -v '^hl.exec_cmd("hyprdesk")'
echo
echo "Raccourcis par défaut : SUPER + D (bureau), SUPER + SHIFT + D (réglages)"
echo "(installés par hyprdesk, modifiables dans Réglages → Bureau)."
echo
echo "hyprdesk dessine le fond d'écran : retirez hyprpaper de l'autostart."
