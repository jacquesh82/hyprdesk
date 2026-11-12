# Assistant d'installation de hyprdesk, sourcé par scripts/install.sh lancé
# sans option au terminal (on ne lance pas ce fichier soi-même).
#
# 5 étapes : langue, contrôles préalables, clavier, options, récapitulatif ;
# puis install.sh est relancé avec les options choisies : ses étapes
# numérotées, le bilan et ce qu'il reste à faire.
#
# Interface : whiptail s'il est là (paquet libnewt / whiptail / newt), aux
# couleurs de hyprdesk ; sinon questions au terminal, en couleur. Un critère
# de base manquant arrête tout juste après la langue. Rien n'est modifié avant
# la confirmation finale.
#   HYPRDESK_INSTALLER_TUI=0   questions au terminal même avec whiptail
#   HYPRDESK_COLOR=0 / NO_COLOR, HYPRDESK_ASCII=1   voir scripts/ui.sh
UI=fr
[[ "${LANG:-}" == fr* ]] || UI=en
export UI
HYPRDESK_ROOT="$PWD"
# shellcheck source=deps.sh
source scripts/deps.sh
t() { hyprdesk_t "$@"; }

TUI=0
command -v whiptail >/dev/null 2>&1 && [[ "${HYPRDESK_INSTALLER_TUI:-1}" != 0 ]] && TUI=1
TITLE=" hyprdesk "
WIZ_N=0
WIZ_TOTAL=5
WIZ_NAME=""

# Couleurs de whiptail : fond sombre, accent violet de hyprdesk.
export NEWT_COLORS='
root=white,black
roottext=brightmagenta,black
window=white,black
shadow=black,black
border=magenta,black
title=brightmagenta,black
textbox=white,black
acttextbox=white,black
label=white,black
button=black,magenta
actbutton=white,magenta
compactbutton=brightmagenta,black
listbox=white,black
actlistbox=black,magenta
sellistbox=brightmagenta,black
actsellistbox=black,brightmagenta
checkbox=white,black
actcheckbox=black,magenta
entry=white,black
disentry=gray,black
helpline=black,magenta
emptyscale=,black
fullscale=,magenta
'

# wiz <titre fr> <titre en> : passe à l'étape suivante de l'assistant.
wiz() {
    WIZ_N=$((WIZ_N + 1))
    WIZ_NAME="$(t "$1" "$2")"
    if [[ "$TUI" == 0 ]]; then
        printf '\n%s%s %d/%d%s %s  %s%s%s\n' "$C_ACCENT$C_BOLD" "$I_STEP" "$WIZ_N" "$WIZ_TOTAL" "$C_RESET" \
            "$(hd_progress "$WIZ_N" "$WIZ_TOTAL")" "$C_BOLD" "$WIZ_NAME" "$C_RESET"
    fi
}
# Bandeau du haut de l'écran whiptail : où l'on en est.
backtitle() {
    local bar="" i
    for ((i = 1; i <= WIZ_TOTAL; i++)); do
        if ((i <= WIZ_N)); then bar+="$I_FULL"; else bar+="$I_EMPTY"; fi
    done
    printf ' hyprdesk · %s   %s  %s %d/%d : %s' "$(t "assistant d'installation" "setup assistant")" "$bar" \
        "$(t "étape" "step")" "$WIZ_N" "$WIZ_TOTAL" "$WIZ_NAME"
}
wt() { whiptail --backtitle "$(backtitle)" --title "$TITLE" "$@"; }
# Dimensions des boîtes : celles demandées, réduites à la taille du terminal.
wh() { local l; l="$(tput lines 2>/dev/null || echo 24)"; ((l - 2 < $1)) && echo $((l - 2)) || echo "$1"; }
ww() { local c; c="$(tput cols 2>/dev/null || echo 80)"; ((c - 4 < $1)) && echo $((c - 4)) || echo "$1"; }

# -- Briques d'interface (whiptail, ou terminal) --------------------------------

# menu <texte> <défaut> <clé> <libellé> [<clé> <libellé>…] : la clé choisie.
menu() {
    local text="$1" def="$2"; shift 2
    if [[ "$TUI" == 1 ]]; then
        wt --default-item "$def" --ok-button "$(t "Suivant" "Next")" --cancel-button "$(t "Quitter" "Quit")" \
            --menu "$text" "$(wh 22)" "$(ww 74)" 12 "$@" 3>&1 1>&2 2>&3
        return
    fi
    local keys=() i=1 n mark
    printf '   %s\n' "$text" >&2
    while [ $# -gt 0 ]; do
        keys+=("$1")
        mark=""
        [[ "$1" == "$def" ]] && mark=" $C_GREEN$(t "(par défaut)" "(default)")$C_RESET"
        printf '   %s%2d%s  %s%s\n' "$C_ACCENT$C_BOLD" "$i" "$C_RESET" "$2" "$mark" >&2
        i=$((i + 1)); shift 2
    done
    read -r -p "   $C_ACCENT$C_BOLD?$C_RESET $(t "numéro (Entrée : défaut)" "number (Enter: default)") " n
    if [[ -z "$n" ]]; then echo "$def"; return; fi
    [[ "$n" =~ ^[0-9]+$ ]] && (( n >= 1 && n <= ${#keys[@]} )) && { echo "${keys[$((n - 1))]}"; return; }
    echo "$def"
}

# input <texte> <défaut>
input() {
    if [[ "$TUI" == 1 ]]; then
        wt --inputbox "$1" "$(wh 10)" "$(ww 74)" "$2" 3>&1 1>&2 2>&3
        return
    fi
    local v
    read -r -p "   $C_ACCENT$C_BOLD?$C_RESET $1 [$2] " v
    echo "${v:-$2}"
}

# checklist <texte> <clé> <libellé> <on|off> […] : clés cochées, séparées par des espaces.
checklist() {
    local text="$1"; shift
    if [[ "$TUI" == 1 ]]; then
        wt --separate-output --ok-button "$(t "Suivant" "Next")" --cancel-button "$(t "Quitter" "Quit")" \
            --checklist "$text" "$(wh 22)" "$(ww 78)" 10 "$@" 3>&1 1>&2 2>&3 | tr '\n' ' '
        return
    fi
    local out=() a box
    printf '   %s\n' "$(t "Répondez o (oui) ou n (non) ; Entrée : la valeur en majuscule." "Answer y (yes) or n (no); Enter: the capitalised value.")" >&2
    while [ $# -gt 0 ]; do
        local def="n"; [[ "$3" == on ]] && def="o"
        read -r -p "   $C_ACCENT$I_DOT$C_RESET $2 ? [$( [[ "$def" == o ]] && t "O/n" "Y/n" || t "o/N" "y/N")] " a
        a="${a:-$def}"
        if [[ "$a" =~ ^[oOyY] ]]; then out+=("$1"); box="$C_GREEN$I_OK$C_RESET"; else box="$C_GREY—$C_RESET"; fi
        # La réponse remplace la question : ✔ choisi, — non.
        printf '\e[1A\r\e[K   %s %s\n' "$box" "$2" >&2
        shift 3
    done
    echo "${out[*]:-}"
}

# yesno <texte> [<oui> <non>] : vrai pour oui.
yesno() {
    if [[ "$TUI" == 1 ]]; then
        wt --yes-button "${2:-$(t "Oui" "Yes")}" --no-button "${3:-$(t "Non" "No")}" --yesno "$1" "$(wh 26)" "$(ww 80)"
        return
    fi
    local a
    printf '%s\n' "$1" | sed 's/^/   /' >&2
    read -r -p "   $C_ACCENT$C_BOLD?$C_RESET ${2:-$(t "Continuer" "Continue")} ? [$(t "o/N" "y/N")] " a
    [[ "$a" =~ ^[oOyY] ]]
}

# message <texte> [<bouton>] : à lire, puis Entrée.
message() {
    if [[ "$TUI" == 1 ]]; then
        wt --ok-button "${2:-$(t "Suivant" "Next")}" --msgbox "$1" "$(wh 26)" "$(ww 80)"
        return
    fi
    printf '%s\n' "$1" | sed 's/^/   /' >&2
}

quit() {
    [[ "$TUI" == 1 ]] && clear 2>/dev/null
    echo
    hd_info "$(t "Assistant quitté : rien n'a été modifié." "Assistant closed: nothing was changed.")"
    exit 1
}

# -- 1. Langue -----------------------------------------------------------------------
if [[ "$TUI" == 0 ]]; then
    hd_banner "hyprdesk · $(t "assistant d'installation" "setup assistant")" \
        "$(t "5 questions, puis l'installation. Rien n'est modifié avant la confirmation." \
             "5 questions, then the install. Nothing is changed before you confirm.")"
fi
wiz "Langue" "Language"
LANG_CHOICE="$(menu "Langue / Language" "$UI" \
    fr "Français" \
    en "English" \
    auto "Langue du système / System language")" || quit
case "$LANG_CHOICE" in fr) UI=fr ;; en) UI=en ;; esac
WIZ_NAME="$(t "Langue" "Language")"

# -- 2. Contrôles préalables : rien ne va plus loin si un critère de base manque ------
wiz "Contrôles préalables" "Preflight checks"
WELCOME="$(t "Bienvenue ! Cet assistant installe hyprdesk, le bureau pour Hyprland." "Welcome! This assistant installs hyprdesk, the desktop for Hyprland.")

$(t "  1. Langue                  ✔
  2. Contrôles préalables    ← maintenant
  3. Clavier
  4. Options
  5. Récapitulatif, puis installation (5 à 10 min la première fois)" \
"  1. Language                ✔
  2. Preflight checks        ← now
  3. Keyboard
  4. Options
  5. Summary, then install (5 to 10 min the first time)")

$(t "Rien n'est modifié avant votre confirmation à l'étape 5. À la fin, un bilan
dit ce qui marche et ce qu'il reste à faire." "Nothing is changed before you confirm at step 5. At the end, a report
says what works and what is left to do.")"
PREFLIGHT=0
REPORT=""
if [[ "$TUI" == 1 ]]; then
    message "$WELCOME

$(t "Suivant : contrôles préalables (quelques secondes)." "Next: preflight checks (a few seconds).")" || quit
    # Texte sans couleurs pour whiptail.
    REPORT="$(hd_plain; HD_PREFLIGHT_HEADER=0 hyprdesk_preflight 2>&1)" || PREFLIGHT=$?
else
    printf '%s\n' "$WELCOME" | sed 's/^/   /'
    echo
    HD_PREFLIGHT_HEADER=0 hyprdesk_preflight || PREFLIGHT=$?
fi
if [[ "$PREFLIGHT" == 3 ]] && [[ -n "$(hyprdesk_pm)" ]]; then
    # Seul Hyprland est trop ancien : proposé ici, puis reconnexion.
    if yesno "$REPORT

$(t "Seul Hyprland bloque. Le mettre à jour maintenant ?
Ensuite : se reconnecter, puis relancer l'assistant." "Only Hyprland is blocking. Update it now?
Then: log out and back in, and run the assistant again.")" "$(t "Mettre à jour" "Update")" "$(t "Quitter" "Quit")"; then
        [[ "$TUI" == 1 ]] && clear 2>/dev/null
        hd_step "⬆" "Mise à jour de Hyprland" "Updating Hyprland"
        hd_sudo_ready "$(t "mettre Hyprland à jour" "update Hyprland")" || exit 1
        hd_run "$(t "mise à jour de Hyprland" "updating Hyprland")" hyprdesk_upgrade_hyprland || exit 1
        hd_ok "Hyprland $(hyprdesk_hyprland_installed || echo '?')"
        hd_panel "$C_YELLOW" "$C_YELLOW$C_BOLD$I_WARN $(t "Il reste à faire :" "Left to do:")$C_RESET" \
            "1. $(t "fermez la session et reconnectez-vous" "log out and back in")" \
            "2. $(t "relancez" "run again") $C_BOLD./scripts/install.sh$C_RESET"
        echo
    fi
    exit 1
elif [[ "$PREFLIGHT" != 0 ]]; then
    if [[ "$TUI" == 1 ]]; then
        message "$REPORT" "$(t "Quitter" "Quit")" || true
        clear 2>/dev/null
        # Le même bilan au terminal, en couleur, pour le garder sous les yeux.
        hyprdesk_preflight || true
    fi
    exit 1
fi
if [[ "$TUI" == 1 ]]; then
    message "$REPORT

$(t "Tout est en ordre. Suivant : le clavier." "All good. Next: the keyboard.")" || quit
fi

# -- 3. Clavier ---------------------------------------------------------------------
wiz "Clavier" "Keyboard"
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
KEYBOARD="$(menu "$(t "Clavier de la session (et de l'écran de connexion)" "Session keyboard (and login screen)")" "$CURRENT" "${KB_ITEMS[@]}")" || quit
if [[ "$KEYBOARD" == other ]]; then
    KEYBOARD="$(input "$(t "Disposition[:variante] xkb (ex. pt, de:nodeadkeys)" "xkb layout[:variant] (e.g. pt, de:nodeadkeys)")" "$CURRENT")" || quit
fi

# -- 4. Options ----------------------------------------------------------------------
wiz "Options" "Options"
HYPR_CONF="${XDG_CONFIG_HOME:-$HOME/.config}/hypr/hyprland.lua"
HAS_HYPRPAPER=off
{ grep -qE '^[[:space:]]*hl\.exec_cmd\(.*hyprpaper' "$HYPR_CONF" 2>/dev/null \
    || systemctl --user is-enabled hyprpaper.service >/dev/null 2>&1; } && HAS_HYPRPAPER=on
DM=""
[[ -L /etc/systemd/system/display-manager.service ]] \
    && DM="$(basename "$(readlink -f /etc/systemd/system/display-manager.service)" .service)"
GREETER_ITEM=(greeter "" off)
GREETER_STATE=""   # récapitulatif quand l'option n'est pas proposée
PM="$(hyprdesk_pm)"
# Écran de hyprdesk déjà en place : rien à proposer.
if [[ "$DM" == greetd ]] && command -v hyprdesk >/dev/null 2>&1 \
    && hyprdesk greeter setup status 2>/dev/null | grep -q hyprdesk; then
    GREETER_ITEM=()
    GREETER_STATE="$(t "déjà en place" "already set")"
else
    case "$DM" in gdm) DM=GDM ;; sddm) DM=SDDM ;; lightdm) DM=LightDM ;; esac
    GREETER_ITEM[1]="$(t "Remplacer ${DM:-le gestionnaire de connexion} par l'écran de hyprdesk" "Replace ${DM:-the display manager} with the hyprdesk login screen")"
    # Il lui faut greetd et cage : présents, sinon installables depuis les dépôts.
    if ! { command -v greetd >/dev/null 2>&1 && command -v cage >/dev/null 2>&1; }; then
        if [[ -n "$PM" ]] && hyprdesk_pkg_available "$PM" greetd && hyprdesk_pkg_available "$PM" cage; then
            GREETER_ITEM[1]+="$(t " (installe greetd et cage)" " (installs greetd and cage)")"
        else
            GREETER_ITEM=()
            GREETER_STATE="$(t "non proposé (greetd ou cage absent des dépôts)" "not offered (greetd or cage not in the repositories)")"
        fi
    fi
fi
# LYRA : proposée (cochée) si elle n'est pas déjà là.
LYRA_ITEM=(lyra "$(t "Installer LYRA, l'assistante IA (compilée depuis GitHub, 3 à 8 min)" "Install LYRA, the AI assistant (built from GitHub, 3 to 8 min)")" on)
LYRA_STATE=""
if command -v hyprpanel-chat >/dev/null 2>&1 || [[ -x "$HOME/.local/bin/hyprpanel-chat" ]]; then
    LYRA_ITEM=()
    LYRA_STATE="$(t "déjà installée" "already installed")"
fi
# myMFA : proposé (coché) s'il n'est pas déjà là.
MYMFA_ITEM=(mymfa "$(t "Installer myMFA, le trousseau (mots de passe, codes 2FA ; dépôt privé, accès SSH)" "Install myMFA, the keyring (passwords, 2FA codes; private repository, SSH access)")" on)
MYMFA_STATE=""
if command -v mymfa >/dev/null 2>&1 || [[ -x "$HOME/.local/bin/mymfa" ]]; then
    MYMFA_ITEM=()
    MYMFA_STATE="$(t "déjà installé" "already installed")"
fi
OPTIONS="$(checklist "$(t "Options à activer (espace : cocher, Entrée : suivant)" "Options to enable (space: toggle, Enter: next)")" \
    "${LYRA_ITEM[@]}" \
    "${MYMFA_ITEM[@]}" \
    deps "$(t "Installer les prérequis manquants (paquets, Rust récente)" "Install missing prerequisites (packages, recent Rust)")" on \
    autostart "$(t "Démarrer hyprdesk à la connexion" "Start hyprdesk at login")" on \
    lockkey "$(t "SUPER + L verrouille avec hyprdesk" "SUPER + L locks with hyprdesk")" on \
    hyprpaper "$(t "Retirer hyprpaper (hyprdesk dessine le fond)" "Remove hyprpaper (hyprdesk draws the wallpaper)")" "$HAS_HYPRPAPER" \
    syskb "$(t "Même clavier pour le système (TTY, autre écran de connexion)" "Same keyboard system-wide (TTY, other login screens)")" off \
    "${GREETER_ITEM[@]}")" || quit
has() { [[ " $OPTIONS " == *" $1 "* ]]; }

# -- 5. Récapitulatif ------------------------------------------------------------
wiz "Récapitulatif" "Summary"
ARGS=(--lang "$LANG_CHOICE" --keyboard "$KEYBOARD")
yes_no() { if has "$1"; then t "oui" "yes"; else t "non" "no"; fi; }
has deps || ARGS+=(--no-deps)
has autostart || ARGS+=(--no-autostart)
has lockkey || ARGS+=(--no-lock-key)
has hyprpaper && ARGS+=(--remove-hyprpaper)
has syskb && ARGS+=(--system-keyboard)
if has greeter; then ARGS+=(--greeter); else ARGS+=(--no-greeter); fi
[[ ${#LYRA_ITEM[@]} -eq 0 ]] || has lyra || ARGS+=(--no-lyra)
[[ ${#MYMFA_ITEM[@]} -eq 0 ]] || has mymfa || ARGS+=(--no-mymfa)
# Colonnes alignées en caractères (printf et ${#x} peuvent compter des octets).
row() { local n=$((28 - $(hd_strlen "$1"))); ((n < 1)) && n=1; printf '  %s%*s%s\n' "$1" "$n" "" "$2"; }
SUMMARY="$(t "Contrôles préalables : tout est en ordre ✔" "Preflight checks: all good ✔")

$(row "$(t "Langue" "Language")" "$LANG_CHOICE")
$(row "LYRA" "$( [[ ${#LYRA_ITEM[@]} -eq 0 ]] && echo "$LYRA_STATE" || yes_no lyra)")
$(row "myMFA" "$( [[ ${#MYMFA_ITEM[@]} -eq 0 ]] && echo "$MYMFA_STATE" || yes_no mymfa)")
$(row "$(t "Clavier" "Keyboard")" "$KEYBOARD")
$(row "$(t "Prérequis manquants" "Missing prerequisites")" "$(yes_no deps)")
$(row "$(t "Démarrage à la connexion" "Start at login")" "$(yes_no autostart)")
$(row "SUPER + L" "$(yes_no lockkey)")
$(row "$(t "Retirer hyprpaper" "Remove hyprpaper")" "$(yes_no hyprpaper)")
$(row "$(t "Clavier du système" "System keyboard")" "$(yes_no syskb)")
$(row "$(t "Écran de connexion" "Login screen")" "$( [[ ${#GREETER_ITEM[@]} -eq 0 ]] && echo "$GREETER_STATE" || yes_no greeter)")

$(t "Ensuite : compilation et installation en étapes numérotées (5 à 10 min la
première fois). L'étape « Dépendances facultatives » propose d'installer celles
qui manquent (veille, captures, Bluetooth…), puis un bilan dit ce qu'il reste
à faire. hyprland.lua est
sauvegardé avant chaque modification ; sudo est demandé pour les paquets, le
clavier du système et l'écran de connexion." "Then: build and install in numbered steps (5 to 10 min the first time).
The “Optional dependencies” step offers to install the missing ones (idle,
screenshots, Bluetooth…), then a report says what is left to do. hyprland.lua is backed up before each
change; sudo is asked for packages, the system keyboard and the login screen.")"
yesno "$SUMMARY" "$(t "Installer" "Install")" "$(t "Annuler" "Cancel")" || quit

# -- Installation et bilan ------------------------------------------------------
[[ "$TUI" == 1 ]] && clear 2>/dev/null
hd_note "scripts/install.sh ${ARGS[*]}"
exec scripts/install.sh "${ARGS[@]}"
