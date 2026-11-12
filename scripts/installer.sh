#!/usr/bin/env bash
# Assistant d'installation de hyprdesk : langue, clavier, options à activer,
# récapitulatif, puis installation (scripts/install.sh) et bilan vérifié.
#
#   scripts/installer.sh
#
# Interface : whiptail s'il est là (paquet libnewt / whiptail / newt), sinon
# questions au terminal. Rien n'est modifié avant la confirmation finale.
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ ! -t 0 ]]; then
    echo "installer.sh est interactif ; sans terminal : scripts/install.sh --help" >&2
    exit 2
fi
TUI=0
command -v whiptail >/dev/null 2>&1 && [[ "${HYPRDESK_INSTALLER_TUI:-1}" != 0 ]] && TUI=1
UI=fr
[[ "${LANG:-}" == fr* ]] || UI=en
t() { if [[ "$UI" == en ]]; then printf '%s' "$2"; else printf '%s' "$1"; fi; }
TITLE="hyprdesk"

# -- Briques d'interface (whiptail, ou terminal) --------------------------------

# menu <texte> <défaut> <clé> <libellé> [<clé> <libellé>…] : la clé choisie.
menu() {
    local text="$1" def="$2"; shift 2
    if [[ "$TUI" == 1 ]]; then
        whiptail --title "$TITLE" --default-item "$def" --menu "$text" 22 74 12 "$@" 3>&1 1>&2 2>&3
        return
    fi
    local keys=() i=1 n
    echo >&2
    echo "$text" >&2
    while [ $# -gt 0 ]; do
        keys+=("$1")
        printf '  %2d) %s%s\n' "$i" "$2" "$([[ "$1" == "$def" ]] && echo " *")" >&2
        i=$((i + 1)); shift 2
    done
    read -r -p "> " n
    if [[ -z "$n" ]]; then echo "$def"; return; fi
    [[ "$n" =~ ^[0-9]+$ ]] && (( n >= 1 && n <= ${#keys[@]} )) && { echo "${keys[$((n - 1))]}"; return; }
    echo "$def"
}

# input <texte> <défaut>
input() {
    if [[ "$TUI" == 1 ]]; then
        whiptail --title "$TITLE" --inputbox "$1" 10 74 "$2" 3>&1 1>&2 2>&3
        return
    fi
    local v
    read -r -p "$1 [$2] " v
    echo "${v:-$2}"
}

# checklist <texte> <clé> <libellé> <on|off> […] : clés cochées, séparées par des espaces.
checklist() {
    local text="$1"; shift
    if [[ "$TUI" == 1 ]]; then
        whiptail --title "$TITLE" --separate-output --checklist "$text" 22 78 10 "$@" 3>&1 1>&2 2>&3 | tr '\n' ' '
        return
    fi
    local out=() a
    echo >&2
    echo "$text" >&2
    while [ $# -gt 0 ]; do
        local def="n"; [[ "$3" == on ]] && def="o"
        read -r -p "  $2 ? [$( [[ "$def" == o ]] && t "O/n" "Y/n" || t "o/N" "y/N")] " a
        a="${a:-$def}"
        [[ "$a" =~ ^[oOyY] ]] && out+=("$1")
        shift 3
    done
    echo "${out[*]:-}"
}

# yesno <texte> : vrai pour oui.
yesno() {
    if [[ "$TUI" == 1 ]]; then
        whiptail --title "$TITLE" --yesno "$1" 22 78
        return
    fi
    local a
    echo >&2
    echo "$1" >&2
    read -r -p "$(t "Continuer ? [o/N] " "Continue? [y/N] ")" a
    [[ "$a" =~ ^[oOyY] ]]
}

# -- 1. Langue -----------------------------------------------------------------------
LANG_CHOICE="$(menu "Langue / Language" "$UI" \
    fr "Français" \
    en "English" \
    auto "Langue du système / System language")" || exit 1
case "$LANG_CHOICE" in fr) UI=fr ;; en) UI=en ;; esac

# -- 2. Clavier ---------------------------------------------------------------------
# Celui en vigueur : Hyprland s'il répond, sinon localectl.
CURRENT=""
if command -v hyprctl >/dev/null 2>&1 && [[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]]; then
    L="$(hyprctl -j getoption input:kb_layout 2>/dev/null | sed -n 's/.*"str": *"\([^"]*\)".*/\1/p')"
    V="$(hyprctl -j getoption input:kb_variant 2>/dev/null | sed -n 's/.*"str": *"\([^"]*\)".*/\1/p')"
    [[ -n "$L" && "$L" != "[[EMPTY]]" ]] && CURRENT="$L${V:+:$V}"
    CURRENT="${CURRENT//\[\[EMPTY\]\]/}"
fi
if [[ -z "$CURRENT" ]] && command -v localectl >/dev/null 2>&1; then
    L="$(localectl status 2>/dev/null | sed -n 's/.*X11 Layout: *//p')"
    V="$(localectl status 2>/dev/null | sed -n 's/.*X11 Variant: *//p')"
    [[ -n "$L" ]] && CURRENT="$L${V:+:$V}"
fi
CURRENT="${CURRENT:-us}"
# Dispositions proposées ; l'actuelle en tête, sans doublon.
KB_ITEMS=("$CURRENT" "$(t "Actuel" "Current") : $CURRENT")
add_kb() { [[ "$1" == "$CURRENT" ]] || KB_ITEMS+=("$1" "$2"); }
add_kb fr "Français (AZERTY)"
add_kb fr:oss "Français (AZERTY, variante OSS)"
add_kb fr:bepo "Français (BÉPO)"
add_kb be "Belge (AZERTY)"
add_kb ch:fr "Suisse romand"
add_kb ca:fr "Canadien français"
add_kb us "English (US, QWERTY)"
add_kb us:intl "English (US, international)"
add_kb gb "English (UK)"
add_kb de "Deutsch (QWERTZ)"
add_kb es "Español"
add_kb it "Italiano"
KB_ITEMS+=(other "$(t "Autre…" "Other…")")
KEYBOARD="$(menu "$(t "Clavier de la session (et de l'écran de connexion)" "Session keyboard (and login screen)")" "$CURRENT" "${KB_ITEMS[@]}")" || exit 1
if [[ "$KEYBOARD" == other ]]; then
    KEYBOARD="$(input "$(t "Disposition[:variante] xkb (ex. pt, de:nodeadkeys)" "xkb layout[:variant] (e.g. pt, de:nodeadkeys)")" "$CURRENT")" || exit 1
fi

# -- 3. Options ----------------------------------------------------------------------
HYPR_CONF="${XDG_CONFIG_HOME:-$HOME/.config}/hypr/hyprland.lua"
HAS_HYPRPAPER=off
{ grep -qE '^[[:space:]]*hl\.exec_cmd\(.*hyprpaper' "$HYPR_CONF" 2>/dev/null \
    || systemctl --user is-enabled hyprpaper.service >/dev/null 2>&1; } && HAS_HYPRPAPER=on
DM=""
[[ -L /etc/systemd/system/display-manager.service ]] \
    && DM="$(basename "$(readlink -f /etc/systemd/system/display-manager.service)" .service)"
GREETER_ITEM=(greeter "$(t "Remplacer ${DM:-le gestionnaire de connexion} par l'écran de hyprdesk" "Replace ${DM:-the display manager} with the hyprdesk login screen")" off)
case "$DM" in gdm) DM=GDM ;; sddm) DM=SDDM ;; lightdm) DM=LightDM ;; esac
GREETER_ITEM[1]="$(t "Remplacer ${DM:-le gestionnaire de connexion} par l'écran de hyprdesk" "Replace ${DM:-the display manager} with the hyprdesk login screen")"
# Écran de hyprdesk déjà en place : rien à proposer.
if [[ "$DM" == greetd ]] && command -v hyprdesk >/dev/null 2>&1 \
    && hyprdesk greeter setup status 2>/dev/null | grep -q hyprdesk; then
    GREETER_ITEM=()
fi
OPTIONS="$(checklist "$(t "Options à activer (espace : cocher)" "Options to enable (space: toggle)")" \
    deps "$(t "Installer les prérequis manquants (paquets, Rust récente, Hyprland 0.57+)" "Install missing prerequisites (packages, recent Rust, Hyprland 0.57+)")" on \
    autostart "$(t "Démarrer hyprdesk à la connexion" "Start hyprdesk at login")" on \
    lockkey "$(t "SUPER + L verrouille avec hyprdesk" "SUPER + L locks with hyprdesk")" on \
    hyprpaper "$(t "Retirer hyprpaper (hyprdesk dessine le fond)" "Remove hyprpaper (hyprdesk draws the wallpaper)")" "$HAS_HYPRPAPER" \
    syskb "$(t "Même clavier pour le système (TTY, autre écran de connexion)" "Same keyboard system-wide (TTY, other login screens)")" off \
    "${GREETER_ITEM[@]}")" || exit 1
has() { [[ " $OPTIONS " == *" $1 "* ]]; }

# -- 4. Récapitulatif ------------------------------------------------------------
ARGS=(--lang "$LANG_CHOICE" --keyboard "$KEYBOARD")
yes_no() { has "$1" && t "oui" "yes" || t "non" "no"; }
has deps || ARGS+=(--no-deps)
has autostart || ARGS+=(--no-autostart)
has lockkey || ARGS+=(--no-lock-key)
has hyprpaper && ARGS+=(--remove-hyprpaper)
has syskb && ARGS+=(--system-keyboard)
if has greeter; then ARGS+=(--greeter); else ARGS+=(--no-greeter); fi
# Colonnes alignées en caractères (printf compte des octets : accents).
row() { local n=$((28 - ${#1})); ((n < 1)) && n=1; printf '  %s%*s%s\n' "$1" "$n" "" "$2"; }
SUMMARY="$(t "Récapitulatif" "Summary")

$(row "$(t "Langue" "Language")" "$LANG_CHOICE")
$(row "$(t "Clavier" "Keyboard")" "$KEYBOARD")
$(row "$(t "Prérequis manquants" "Missing prerequisites")" "$(yes_no deps)")
$(row "$(t "Démarrage à la connexion" "Start at login")" "$(yes_no autostart)")
$(row "SUPER + L" "$(yes_no lockkey)")
$(row "$(t "Retirer hyprpaper" "Remove hyprpaper")" "$(yes_no hyprpaper)")
$(row "$(t "Clavier du système" "System keyboard")" "$(yes_no syskb)")
$(row "$(t "Écran de connexion" "Login screen")" "$( [[ ${#GREETER_ITEM[@]} -eq 0 ]] && t "déjà en place" "already set" || yes_no greeter)")

$(t "hyprland.lua est sauvegardé avant chaque modification. sudo sera demandé
pour les paquets, le clavier du système et l'écran de connexion." "hyprland.lua is backed up before each change. sudo is asked for packages,
the system keyboard and the login screen.")"
yesno "$SUMMARY" || { echo "$(t "Rien n'a été modifié." "Nothing was changed.")"; exit 0; }

# -- 5. Installation et bilan ------------------------------------------------------
clear 2>/dev/null || true
echo "scripts/install.sh ${ARGS[*]}"
exec scripts/install.sh "${ARGS[@]}"
