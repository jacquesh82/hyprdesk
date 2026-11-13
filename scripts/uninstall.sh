#!/usr/bin/env bash
# shellcheck disable=SC1111
# (SC1111 : “ ” typographiques des traductions anglaises.)
# Désinstallation de hyprdesk : défait ce que scripts/install.sh a posé.
#
#   ./scripts/uninstall.sh             récapitulatif, confirmation, puis désinstallation
#   ./scripts/uninstall.sh --dry-run   ce qui serait fait, sans rien toucher
#
#   --purge        efface aussi la configuration de hyprdesk (hyprdesk.lua,
#                  hyprdesk/, fonds d'écran importés, état, cache) ; sans elle,
#                  une réinstallation retrouve vos réglages
#   --lyra         désinstalle aussi LYRA, l'assistante IA (hyprpanel)
#   --mymfa        désinstalle aussi myMFA, le trousseau ; gnome-keyring
#                  reprend la main (vos secrets restent dans le coffre myMFA)
#   --yes          sans confirmation (obligatoire sans terminal)
#   --dry-run      liste les actions sans rien modifier
#   --lang fr|en   langue des messages (sans : celle du système)
#
# Ce qui est défait : l'écran de connexion (si hyprdesk l'avait remplacé : le
# gestionnaire d'avant revient), le service systemd utilisateur, le binaire et
# la police, les lignes posées dans hyprland.lua et binds.lua (démarrage,
# SUPER + L rendu à hyprlock, raccourcis qui appellent hyprdesk mis en
# commentaire, services « gérés par hyprdesk » remis en marche), le thème des
# applications (kitty, Thunar). Copie de chaque fichier modifié :
# <fichier>.avant-desinstallation-hyprdesk.bak ; retour arrière si Hyprland
# signale une nouvelle erreur au rechargement.
# Restent en place : inputs.lua, monitors.lua, binds.lua et autostart.lua, que
# hyprland.lua charge (configuration Hyprland ordinaire, à vous).
set -Eeuo pipefail
cd "$(dirname "$0")/.."

PURGE=0
LYRA=0
MYMFA=0
YES=0
DRY=0
while [ $# -gt 0 ]; do
    case "$1" in
        --purge) PURGE=1 ;;
        --lyra) LYRA=1 ;;
        --mymfa) MYMFA=1 ;;
        --yes|-y) YES=1 ;;
        --dry-run|-n) DRY=1 ;;
        --lang) case "${2:-}" in fr|en) UI="$2" ;; *) echo "--lang fr|en" >&2; exit 2 ;; esac; shift ;;
        -h|--help) sed -n '4,28p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "argument inconnu / unknown argument : $1 (--help)" >&2; exit 2 ;;
    esac
    shift
done
[[ -n "${UI:-}" ]] || { [[ "${LANG:-}" == fr* ]] && UI=fr || UI=en; }
export UI

# shellcheck source=ui.sh disable=SC1091
source scripts/ui.sh
t() { hyprdesk_t "$@"; }

CFG="${XDG_CONFIG_HOME:-$HOME/.config}"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
STATE="${XDG_STATE_HOME:-$HOME/.local/state}/hyprdesk"
CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/hyprdesk"
BIN_DIR="$HOME/.local/bin"
UNIT_DIR="$CFG/systemd/user"
HYPR_DIR="$CFG/hypr"
HYPR_CONF="$HYPR_DIR/hyprland.lua"
BAK_SUFFIX=".avant-desinstallation-hyprdesk.bak"
BIN="$(command -v hyprdesk 2>/dev/null || echo "$BIN_DIR/hyprdesk")"
IN_SESSION=0
[[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]] && command -v hyprctl >/dev/null 2>&1 && IN_SESSION=1

TODO=()
# do_ <description> <commande…> : affichée, exécutée hors --dry-run.
do_() {
    local what="$1"; shift
    if [[ "$DRY" == 1 ]]; then
        hd_info "$(t "ferait :" "would:") $what"
        return 0
    fi
    if "$@" >/dev/null 2>&1; then
        hd_ok "$what"
    else
        hd_warn "$what — $(t "échec" "failed")"
        return 1
    fi
}
rm_path() { # rm_path <chemin…> : seulement ce qui existe
    local p
    for p in "$@"; do
        [[ -e "$p" || -L "$p" ]] || continue
        do_ "$(t "supprimé :" "removed:") ${p/#$HOME/\~}" rm -rf -- "$p" || true
    done
}
backup_once() {
    [[ -f "$1" && ! -f "$1$BAK_SUFFIX" ]] && cp -p "$1" "$1$BAK_SUFFIX"
    return 0
}

greeter_ours() {
    [[ -L /etc/systemd/system/display-manager.service ]] \
        && [[ "$(basename "$(readlink -f /etc/systemd/system/display-manager.service)")" == greetd.service ]] \
        && [[ -x "$BIN" ]] && "$BIN" greeter setup status 2>/dev/null | grep -q hyprdesk
}
lyra_bins() {
    local b
    for b in hyprpanel-chat hyprpanel; do [[ -x "$BIN_DIR/$b" ]] && echo "$BIN_DIR/$b"; done
    return 0
}

# -- Récapitulatif -----------------------------------------------------------------------
BANNER=("$(t "Défait ce que scripts/install.sh a posé." "Undoes what scripts/install.sh set up.")")
[[ "$DRY" == 1 ]] && BANNER+=("$(t "Essai à blanc : rien ne sera modifié." "Dry run: nothing will change.")")
hd_banner "hyprdesk · $(t "désinstallation" "uninstall")" "${BANNER[@]}"
hd_section "📋" "Ce qui sera fait" "What will happen"
greeter_ours && hd_info "$(t "écran de connexion : retour au gestionnaire d'avant (sudo)" "login screen: back to the previous display manager (sudo)")"
hd_info "$(t "service hyprdesk arrêté et retiré ; binaire et police supprimés" "hyprdesk service stopped and removed; binary and font deleted")"
hd_info "$(t "hyprland.lua et binds.lua nettoyés (copie de sauvegarde d'abord)" "hyprland.lua and binds.lua cleaned up (backup copy first)")"
hd_info "$(t "thème des applications retiré (kitty, Thunar)" "application theme removed (kitty, Thunar)")"
if [[ "$PURGE" == 1 ]]; then
    hd_info "$(t "configuration de hyprdesk effacée (--purge)" "hyprdesk configuration deleted (--purge)")"
else
    hd_note "$(t "configuration gardée (hyprdesk.lua, hyprdesk/) : --purge pour l'effacer" "configuration kept (hyprdesk.lua, hyprdesk/): --purge to delete it")"
fi
[[ "$LYRA" == 1 ]] && hd_info "$(t "LYRA désinstallée (--lyra)" "LYRA uninstalled (--lyra)")"
[[ "$LYRA" == 0 && -n "$(lyra_bins)" ]] && hd_note "$(t "LYRA gardée : --lyra pour la retirer aussi" "LYRA kept: --lyra to remove her too")"
[[ "$MYMFA" == 1 ]] && hd_info "$(t "myMFA désinstallé, gnome-keyring rétabli (--mymfa)" "myMFA uninstalled, gnome-keyring restored (--mymfa)")"
[[ "$MYMFA" == 0 && -x "$BIN_DIR/mymfa" ]] && hd_note "$(t "myMFA gardé : --mymfa pour le retirer aussi" "myMFA kept: --mymfa to remove it too")"

if [[ "$DRY" == 0 && "$YES" == 0 ]]; then
    if [[ -t 0 ]]; then
        printf '\n  %s?%s ' "$C_ACCENT$C_BOLD" "$C_RESET"
        read -r -p "$(t "Désinstaller hyprdesk ? [o/N] " "Uninstall hyprdesk? [y/N] ")" reply
        [[ "$reply" =~ ^[oOyY] ]] || { hd_info "$(t "rien n'a été modifié" "nothing was changed")"; exit 0; }
    else
        hd_die_line "$(t "sans terminal : relancez avec --yes" "no terminal: run again with --yes")"
        exit 2
    fi
fi

# -- 1. Écran de connexion (avant le binaire, qui sait le défaire) ----------------------
hd_section "🖥" "Écran de connexion" "Login screen"
if greeter_ours; then
    if [[ "$DRY" == 1 ]]; then
        hd_info "$(t "ferait :" "would:") sudo $BIN greeter setup restore"
    elif hd_sudo_ready "$(t "rendre l'écran de connexion d'avant" "restore the previous login screen")" \
        && sudo "$BIN" greeter setup restore; then
        hd_ok "$(t "gestionnaire de connexion d'avant rétabli" "previous display manager restored")"
        TODO+=("$(t "redémarrer pour retrouver l'écran de connexion d'avant : systemctl reboot" "reboot to get the previous login screen back: systemctl reboot")")
    else
        hd_ko "$(t "écran de connexion non rétabli : désinstallation arrêtée (rien d'autre n'a changé)" "login screen not restored: uninstall stopped (nothing else changed)")"
        hd_cmd "sudo $BIN greeter setup restore"
        exit 1
    fi
else
    hd_ok "$(t "pas celui de hyprdesk : rien à faire" "not the hyprdesk one: nothing to do")"
fi

# -- 2. myMFA et LYRA (demandés) ----------------------------------------------------------
if [[ "$MYMFA" == 1 ]]; then
    hd_section "🔑" "myMFA" "myMFA"
    # gnome-keyring reprend le Secret Service (comme Réglages → Mots de passe).
    do_ "$(t "gnome-keyring démasqué" "gnome-keyring unmasked")" \
        systemctl --user unmask gnome-keyring-daemon.service gnome-keyring-daemon.socket || true
    do_ "$(t "mymfa.service arrêté et désactivé" "mymfa.service stopped and disabled")" \
        systemctl --user disable --now mymfa.service || true
    rm_path "$BIN_DIR/mymfa" "$UNIT_DIR/mymfa.service" "$UNIT_DIR/mymfa.service.d" \
        "$DATA/dbus-1/services/today.mindlog.MyMfa1.service" "$DATA/hyprdesk/src/mindlog.mymfa.clients"
    [[ "$DRY" == 1 ]] || systemctl --user start gnome-keyring-daemon.socket >/dev/null 2>&1 || true
    for d in /usr/lib/security /usr/lib64/security "/usr/lib/$(uname -m)-linux-gnu/security"; do
        [[ -f "$d/pam_mymfa.so" ]] && TODO+=("$(t "module PAM de myMFA (retirer d'abord ses lignes de /etc/pam.d) : sudo rm $d/pam_mymfa.so" "myMFA PAM module (remove its lines from /etc/pam.d first): sudo rm $d/pam_mymfa.so")")
    done
    hd_note "$(t "le coffre de myMFA n'est pas effacé : vos secrets y restent" "the myMFA vault is not deleted: your secrets stay in it")"
fi
if [[ "$LYRA" == 1 ]]; then
    hd_section "🤖" "LYRA" "LYRA"
    do_ "$(t "hyprpanel.service arrêté et désactivé" "hyprpanel.service stopped and disabled")" \
        systemctl --user disable --now hyprpanel.service || true
    # shellcheck disable=SC2046
    rm_path $(lyra_bins) "$UNIT_DIR/hyprpanel.service" "$DATA/hyprdesk/src/mindlog.hyprland_tools"
fi

# -- 3. Service et démon ------------------------------------------------------------------
hd_section "⚙" "Service" "Service"
if [[ "$IN_SESSION" == 1 && -x "$BIN" && "$DRY" == 0 ]] && "$BIN" status >/dev/null 2>&1; then
    # Fenêtres rendues au tiling avant l'arrêt (mode « pas de bureau »).
    "$BIN" hide >/dev/null 2>&1 || true
    sleep 0.5
fi
do_ "$(t "hyprdesk.service arrêté et désactivé" "hyprdesk.service stopped and disabled")" \
    systemctl --user disable --now hyprdesk.service || true
rm_path "$UNIT_DIR/hyprdesk.service" "$UNIT_DIR/hyprdesk.service.d"
[[ "$DRY" == 1 ]] || systemctl --user daemon-reload >/dev/null 2>&1 || true

# -- 4. hyprland.lua et binds.lua ---------------------------------------------------------
hd_section "📝" "Configuration Hyprland" "Hyprland configuration"
HAS_HYPRLOCK=0
command -v hyprlock >/dev/null 2>&1 && HAS_HYPRLOCK=1
# Nettoie un fichier Lua de Hyprland ; la sortie va sur stdout.
clean_lua() {
    awk -v hyprlock="$HAS_HYPRLOCK" '
        # Démarrage posé par install.sh : le commentaire et sa suite, puis la ligne.
        /-- hyprdesk : démarrage à la connexion/ { skip = 1; next }
        skip && /^[[:space:]]*--/ { next }
        { skip = 0 }
        /systemctl --user start hyprdesk\.service/ { next }
        /^[[:space:]]*-- hyprdesk : verrouillage \(écrit par scripts\/install\.sh\)/ { next }
        # Services mis en commentaire par hyprdesk : remis en marche.
        /^[[:space:]]*-- hl\.exec_cmd\(.*\)[[:space:]]*--.*(géré par hyprdesk|retiré : hyprdesk)/ {
            match($0, /^[[:space:]]*/); ind = substr($0, 1, RLENGTH)
            line = substr($0, RLENGTH + 1); sub(/^-- /, "", line)
            sub(/\)[[:space:]]*--.*$/, ")", line)
            print ind line; next
        }
        # SUPER + L : rendu à hyprlock s il est là.
        /^[[:space:]]*hl\.bind\(.*exec_cmd\("[^"]*hyprdesk lock"\)/ && hyprlock == 1 {
            sub(/exec_cmd\("[^"]*hyprdesk lock"\)/, "exec_cmd(\"hyprlock\")")
            sub(/description = "hyprdesk : verrouiller"/, "description = \"Verrouiller (hyprlock)\"")
            print; next
        }
        # Raccourcis et lancements qui appellent hyprdesk, sur une ligne : en commentaire.
        /^[[:space:]]*hl\.(bind|exec_cmd)\(.*"[^"]*hyprdesk( [^"]*)?"/ && /\)[[:space:]]*(--.*)?$/ {
            match($0, /^[[:space:]]*/)
            print substr($0, 1, RLENGTH) "-- " substr($0, RLENGTH + 1) " -- hyprdesk désinstallé"; next
        }
        { print }
    ' "$1" | awk '
        # Bloc de démarrage resté vide : retiré.
        { lines[NR] = $0 }
        END {
            for (i = 1; i <= NR; i++) {
                if (lines[i] ~ /^hl\.on\("hyprland\.start", function\(\)[[:space:]]*$/ && lines[i + 1] ~ /^end\)[[:space:]]*$/) { i++; continue }
                print lines[i]
            }
        }'
}
config_errors() { [[ "$IN_SESSION" == 1 ]] && hyprctl configerrors 2>/dev/null | grep -v '^[[:space:]]*$' || true; }
EDITED=()
ERRORS_BEFORE="$(config_errors)"
for f in "$HYPR_CONF" "$HYPR_DIR/binds.lua" "$HYPR_DIR/autostart.lua"; do
    [[ -f "$f" ]] || continue
    new="$(clean_lua "$f")"
    if [[ "$new" == "$(cat "$f")" ]]; then
        hd_ok "${f/#$HOME/\~} : $(t "rien à retirer" "nothing to remove")"
        continue
    fi
    if [[ "$DRY" == 1 ]]; then
        hd_info "$(t "ferait :" "would:") ${f/#$HOME/\~}"
        diff <(cat "$f") <(printf '%s\n' "$new") | sed -n 's/^< /      - /p; s/^> /      + /p' | head -20 || true
        continue
    fi
    backup_once "$f"
    printf '%s\n' "$new" > "$f"
    EDITED+=("$f")
    hd_ok "${f/#$HOME/\~} $(t "nettoyé (copie :" "cleaned up (backup:") ${f##*/}$BAK_SUFFIX)"
done
if ((${#EDITED[@]} > 0)) && [[ "$IN_SESSION" == 1 ]]; then
    hyprctl reload >/dev/null 2>&1 || true
    sleep 0.5
    if [[ "$(config_errors)" != "$ERRORS_BEFORE" ]]; then
        # Une erreur nouvelle : les fichiers d'avant reviennent.
        for f in "${EDITED[@]}"; do cp -p "$f$BAK_SUFFIX" "$f"; done
        hyprctl reload >/dev/null 2>&1 || true
        hd_warn "$(t "Hyprland signalait une erreur : fichiers remis comme avant" "Hyprland reported an error: files put back as they were")"
        TODO+=("$(t "nettoyer hyprland.lua à la main (lignes « hyprdesk »)" "clean up hyprland.lua by hand (“hyprdesk” lines)")")
    else
        hd_ok "$(t "Hyprland rechargé, sans nouvelle erreur" "Hyprland reloaded, no new error")"
    fi
fi
if [[ -f "$STATE/install-options" ]] && grep -qx -- --remove-hyprpaper "$STATE/install-options"; then
    TODO+=("$(t "fond d'écran : hyprpaper avait été retiré à l'installation" "wallpaper: hyprpaper was removed at install time") — systemctl --user enable --now hyprpaper.service")
fi

# -- 5. Thème des applications ------------------------------------------------------------
hd_section "🎨" "Thème des applications" "Application theme"
strip_lines() { # strip_lines <fichier> <motif awk> : lignes de hyprdesk retirées
    local f="$1" re="$2"
    [[ -f "$f" ]] && grep -Eq "$re" "$f" || return 0
    if [[ "$DRY" == 1 ]]; then hd_info "$(t "ferait :" "would:") ${f/#$HOME/\~}"; return 0; fi
    backup_once "$f"
    grep -Ev "$re" "$f" > "$f.hyprdesk-tmp" || true
    mv -f "$f.hyprdesk-tmp" "$f"
    hd_ok "${f/#$HOME/\~} $(t "nettoyé" "cleaned up")"
}
strip_lines "$CFG/kitty/kitty.conf" '^# Thème de hyprdesk \(|^include .*hyprdesk-theme\.conf'
strip_lines "$CFG/gtk-3.0/gtk.css" '^/\* Thème de Thunar par hyprdesk|^@import url\(".*hyprdesk-thunar\.css"\);'
rm_path "$CFG/kitty/hyprdesk-theme.conf" "$CFG/gtk-3.0/hyprdesk-thunar.css"

# -- 6. Fichiers ---------------------------------------------------------------------------
hd_section "🗑" "Fichiers" "Files"
rm_path "$BIN_DIR/hyprdesk" "$DATA/fonts/hyprdesk" "$CFG/hyprpanel/context.d/hyprdesk.md"
[[ "$DRY" == 1 ]] || { command -v fc-cache >/dev/null 2>&1 && fc-cache -f >/dev/null 2>&1; } || true
if [[ "$PURGE" == 1 ]]; then
    rm_path "$HYPR_DIR/hyprdesk.lua" "$HYPR_DIR/hyprdesk.meta.lua" "$HYPR_DIR/hyprdesk" \
        "$DATA/hyprdesk" "$STATE" "$CACHE"
else
    rm_path "$CACHE"
    hd_note "$(t "gardés pour une réinstallation :" "kept for a reinstall:") ~/.config/hypr/hyprdesk.lua, ~/.config/hypr/hyprdesk/, ${DATA/#$HOME/\~}/hyprdesk"
fi

# -- Bilan ---------------------------------------------------------------------------------
hd_section "📝" "Ce qu'il reste à faire" "What is left to do"
TODO+=("$(t "restent à vous, chargés par hyprland.lua : inputs.lua, monitors.lua, binds.lua, autostart.lua" "yours to keep, loaded by hyprland.lua: inputs.lua, monitors.lua, binds.lua, autostart.lua")")
i=1
for item in "${TODO[@]}"; do
    printf '  %s%d.%s %s\n' "$C_ACCENT$C_BOLD" "$i" "$C_RESET" "$item"
    i=$((i + 1))
done
echo
if [[ "$DRY" == 1 ]]; then
    hd_panel "$C_CYAN" "$C_CYAN$C_BOLD$I_INFO $(t "essai à blanc : rien n'a été modifié" "dry run: nothing was changed")$C_RESET" \
        "$(t "relancez sans --dry-run pour désinstaller" "run again without --dry-run to uninstall")"
else
    hd_panel "$C_GREEN" "$C_GREEN$C_BOLD$I_OK $(t "hyprdesk est désinstallé" "hyprdesk is uninstalled")$C_RESET" \
        "$(t "pour revenir : ./scripts/install.sh" "to come back: ./scripts/install.sh")"
fi
echo
