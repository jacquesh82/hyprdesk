#!/bin/sh
# Après installation du paquet hyprdesk Arch construit par nfpm (scripts/package.sh) ;
# même message que hyprdesk.install du PKGBUILD. Pas de fc-cache : hook de fontconfig.
cat <<'MSG'
hyprdesk is installed. To start it in your Hyprland session:
  systemctl --user import-environment WAYLAND_DISPLAY HYPRLAND_INSTANCE_SIGNATURE
  systemctl --user enable --now hyprdesk.service
Suggested hyprland.lua lines (start at login, SUPER + D): hyprdesk snippet
Optional dependencies (idle, network, screenshots…): hyprdesk doctor
Login screen, opt-in (needs greetd and cage): sudo hyprdesk greeter setup enable
MSG
