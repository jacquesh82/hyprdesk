#!/bin/sh
# Après installation du paquet hyprdesk Arch construit par nfpm (scripts/package.sh) ;
# même message que hyprdesk.install du PKGBUILD. Pas de fc-cache : hook de fontconfig.
cat <<'MSG'
hyprdesk installé. Pour l'activer dans votre session Hyprland :
  systemctl --user import-environment WAYLAND_DISPLAY HYPRLAND_INSTANCE_SIGNATURE
  systemctl --user enable --now hyprdesk.service
Lignes à ajouter à hyprland.lua : hyprdesk snippet
Dépendances facultatives (veille, réseau, captures…) : hyprdesk doctor
MSG
