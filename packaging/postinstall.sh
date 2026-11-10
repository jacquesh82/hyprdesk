#!/bin/sh
# Après installation du paquet hyprdesk (deb, rpm, archlinux).
# Rien n'est activé d'office : le service est utilisateur, chacun l'active.
if command -v fc-cache >/dev/null 2>&1; then
    fc-cache -f /usr/share/fonts/hyprdesk >/dev/null 2>&1 || true
fi
cat <<'EOF'
hyprdesk installé. Pour l'activer dans votre session Hyprland :
  systemctl --user import-environment WAYLAND_DISPLAY HYPRLAND_INSTANCE_SIGNATURE
  systemctl --user enable --now hyprdesk.service
Lignes à ajouter à hyprland.lua : hyprdesk snippet
Dépendances facultatives (veille, réseau, captures…) : hyprdesk doctor
EOF
exit 0
