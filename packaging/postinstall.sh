#!/bin/sh
# Après installation du paquet hyprdesk (deb, rpm, archlinux).
# Rien n'est activé d'office : le service est utilisateur, chacun l'active.
if command -v fc-cache >/dev/null 2>&1; then
    fc-cache -f /usr/share/fonts/hyprdesk >/dev/null 2>&1 || true
fi
# Pile PAM du verrouillage (MLG-153) : le paquet deb la pose lui-même
# (fichier de configuration) ; pour le rpm, commun à Fedora (system-auth) et
# openSUSE (common-auth), elle est choisie ici d'après la distribution,
# jamais écrasée si elle existe déjà.
if [ ! -e /etc/pam.d/hyprdesk-lock ]; then
    pam_src=""
    if [ -f /etc/pam.d/system-auth ]; then
        pam_src=/usr/share/hyprdesk/pam/hyprdesk-lock.system-auth
    elif [ -f /etc/pam.d/common-auth ] && [ -f /etc/pam.d/common-account ]; then
        pam_src=/usr/share/hyprdesk/pam/hyprdesk-lock.common-auth
    fi
    if [ -n "$pam_src" ] && [ -f "$pam_src" ]; then
        install -m0644 "$pam_src" /etc/pam.d/hyprdesk-lock || true
    fi
fi
cat <<'EOF'
hyprdesk installé. Pour l'activer dans votre session Hyprland :
  systemctl --user import-environment WAYLAND_DISPLAY HYPRLAND_INSTANCE_SIGNATURE
  systemctl --user enable --now hyprdesk.service
Lignes à ajouter à hyprland.lua : hyprdesk snippet
Dépendances facultatives (veille, réseau, captures…) : hyprdesk doctor
EOF
exit 0
