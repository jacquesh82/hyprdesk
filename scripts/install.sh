#!/usr/bin/env bash
# Installation de hyprdesk sur la machine locale (assistant : scripts/installer.sh).
#
#   --lang fr|en|auto        langue de hyprdesk et de ces messages (sans : la langue
#                            de hyprdesk ne change pas, messages dans celle du système)
#   --keyboard L[:V[:O]]     clavier de la session (inputs.lua), ex. fr:azerty
#   --system-keyboard        le même clavier pour le système (localectl : GDM, TTY…)
#   --no-deps                ne pas installer les prérequis manquants
#   --no-autostart           ne pas ajouter le démarrage à la connexion
#   --no-lock-key            ne pas poser SUPER + L -> hyprdesk lock
#   --remove-hyprpaper       retirer hyprpaper de l'autostart (hyprdesk dessine le fond)
#   --greeter | --no-greeter remplacer GDM (SDDM…) par l'écran de hyprdesk, ou non
#                            (sans l'un ni l'autre : question posée)
#   --verbose                sortie complète des commandes (sinon : journal)
#
# Étapes numérotées : contrôles préalables (arrêt avant toute modification si un
# critère de base manque : root, architecture, systemd utilisateur, Hyprland 0.57+,
# gestionnaire de paquets, espace disque, réseau), prérequis (scripts/deps.sh),
# compilation, police, binaire dans ~/.local/bin, configuration, langue,
# clavier, dépendances facultatives, service systemd utilisateur, démarrage à
# la connexion (hyprland.lua, sans dépendre de graphical-session.target),
# SUPER + L, hyprpaper, écran de connexion ; puis un bilan vérifié point par
# point et la liste de ce qu'il reste à faire (code de sortie non nul si
# l'essentiel manque). Sorties complètes : ~/.local/state/hyprdesk/install-*.log.
# Chaque modification de hyprland.lua est précédée d'une copie de sauvegarde.
# Couleurs : HYPRDESK_COLOR=0|1, NO_COLOR ; icônes ASCII : HYPRDESK_ASCII=1.
set -Eeuo pipefail
cd "$(dirname "$0")/.."

LANG_CHOICE=""
KEYBOARD=""
SYSTEM_KEYBOARD=0
AUTOSTART=1
LOCK_KEY=1
REMOVE_HYPRPAPER=0
GREETER=ask
while [ $# -gt 0 ]; do
    case "$1" in
        --lang) LANG_CHOICE="${2:?--lang fr|en|auto}"; shift ;;
        --keyboard) KEYBOARD="${2:?--keyboard disposition[:variante[:options]]}"; shift ;;
        --system-keyboard) SYSTEM_KEYBOARD=1 ;;
        --no-deps) export HYPRDESK_NO_DEPS=1 ;;
        --no-autostart) AUTOSTART=0 ;;
        --no-lock-key) LOCK_KEY=0 ;;
        --remove-hyprpaper) REMOVE_HYPRPAPER=1 ;;
        --greeter) GREETER=yes ;;
        --no-greeter) GREETER=no ;;
        --verbose) export HYPRDESK_VERBOSE=1 ;;
        -h|--help) sed -n '2,27p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "argument inconnu / unknown argument : $1 (--help)" >&2; exit 2 ;;
    esac
    shift
done
case "$LANG_CHOICE" in ""|fr|en|auto) ;; *) echo "--lang : fr, en ou auto" >&2; exit 2 ;; esac

# Langue des messages : celle choisie, sinon celle du système.
UI=fr
if [[ "$LANG_CHOICE" == en ]] || { [[ "$LANG_CHOICE" != fr ]] && [[ "${LANG:-}" != fr* ]]; }; then
    UI=en
fi
export UI

HYPRDESK_ROOT="$PWD"
# shellcheck source=deps.sh
source scripts/deps.sh
t() { hyprdesk_t "$@"; }

BIN_DIR="$HOME/.local/bin"
BIN="$BIN_DIR/hyprdesk"
UNIT_DIR="$HOME/.config/systemd/user"
FONT_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/fonts/hyprdesk/Mindlog"
HYPR_CONF="${XDG_CONFIG_HOME:-$HOME/.config}/hypr/hyprland.lua"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/hypr"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"

# Copie de sauvegarde de hyprland.lua, une fois par étape.
BACKUPS=()
backup() {
    cp -p "$HYPR_CONF" "$HYPR_CONF.avant-hyprdesk-$1.bak"
    BACKUPS+=("$HYPR_CONF.avant-hyprdesk-$1.bak")
}

# Nombre d'étapes, selon les options.
HD_STEP_TOTAL=10
[[ -n "$LANG_CHOICE" ]] && HD_STEP_TOTAL=$((HD_STEP_TOTAL + 1))
[[ -n "$KEYBOARD" ]] && HD_STEP_TOTAL=$((HD_STEP_TOTAL + 1))
[[ "$AUTOSTART" == 1 ]] && HD_STEP_TOTAL=$((HD_STEP_TOTAL + 1))
[[ "$LOCK_KEY" == 1 ]] && HD_STEP_TOTAL=$((HD_STEP_TOTAL + 1))
[[ "$REMOVE_HYPRPAPER" == 1 ]] && HD_STEP_TOTAL=$((HD_STEP_TOTAL + 1))

hd_log_init
hd_banner "hyprdesk ${VERSION:+$VERSION }· $(t "installation" "installation")" \
    "$(t "$HD_STEP_TOTAL étapes, puis un bilan et ce qu'il reste à faire." "$HD_STEP_TOTAL steps, then a report and what is left to do.")" \
    "$(t "Journal complet : $HD_LOG" "Full log: $HD_LOG")"

# Arrêt sur erreur : quelle étape, pourquoi, que faire.
on_error() {
    local rc=$? line="$1"
    trap - ERR
    set +e
    printf '\e[?25h'
    local lines=(
        "$C_RED$C_BOLD$I_KO $(t "Installation arrêtée" "Install stopped") — $(t "étape" "step") $HD_STEP_N/$HD_STEP_TOTAL : $HD_STEP_NAME$C_RESET"
        "$C_GREY$(t "install.sh, ligne $line, code $rc" "install.sh, line $line, code $rc")$C_RESET"
    )
    [[ -n "$HD_HINT" ]] && lines+=("$C_YELLOW$I_HINT$C_RESET $HD_HINT")
    lines+=(
        "$C_YELLOW$I_HINT$C_RESET $(t "journal complet :" "full log:") $C_BOLD$HD_LOG$C_RESET"
        "$C_YELLOW$I_HINT$C_RESET $(t "après correction, relancez : les étapes déjà faites sont refaites sans dommage" "once fixed, run again: steps already done are safely redone")"
        "   $C_BOLD./scripts/installer.sh$C_RESET"
    )
    ((${#BACKUPS[@]} > 0)) && lines+=("$C_GREY$(t "hyprland.lua sauvegardé avant modification :" "hyprland.lua backed up before changes:") ${BACKUPS[*]}$C_RESET")
    hd_panel "$C_RED" "${lines[@]}" >&2
    exit "$rc"
}
trap 'on_error $LINENO' ERR
trap 'printf "\e[?25h\n"; hd_die_line "$(t "interrompu (Ctrl+C) à l étape $HD_STEP_N/$HD_STEP_TOTAL : $HD_STEP_NAME" "interrupted (Ctrl+C) at step $HD_STEP_N/$HD_STEP_TOTAL: $HD_STEP_NAME")"; exit 130' INT

# -- Contrôles préalables : rien n'est modifié si un critère de base manque ---------
PREFLIGHT=0
hyprdesk_preflight || PREFLIGHT=$?
[[ "$PREFLIGHT" == 0 ]] || { trap - ERR; exit 1; }

# -- Prérequis -------------------------------------------------------------------------
hyprdesk_deps

# -- Compilation -----------------------------------------------------------------------
hd_step "🦀" "Compilation" "Build" \
    "cargo build --release : 3 à 10 min la première fois, quelques secondes ensuite." \
    "cargo build --release: 3 to 10 min the first time, a few seconds afterwards."
HD_HINT="$(t "lisez l'erreur ci-dessus ; une bibliothèque de développement absente se règle en relançant sans --no-deps" \
             "read the error above; a missing development library is fixed by running again without --no-deps")"
hd_run "$(t "compilation de hyprdesk" "building hyprdesk")" cargo build --release --locked

# -- Police ----------------------------------------------------------------------------
hd_step "🔤" "Police Mindlog" "Mindlog font" \
    "Les icônes de la barre et des Réglages ; dans $FONT_DIR." "Bar and Settings icons; into $FONT_DIR."
mkdir -p "$FONT_DIR"
install -m644 assets/fonts/mindlog/*.ttf assets/fonts/mindlog/OFL.txt "$FONT_DIR/"
hd_ok "$(t "police copiée" "font copied")"
if command -v fc-cache >/dev/null 2>&1; then
    hd_run "$(t "cache des polices" "font cache")" fc-cache -f "$FONT_DIR"
fi

# -- Binaire ---------------------------------------------------------------------------
hd_step "📥" "Binaire" "Binary" "$BIN" "$BIN"
HD_HINT="$(t "vérifiez que $BIN_DIR est accessible en écriture" "check that $BIN_DIR is writable")"
mkdir -p "$BIN_DIR"
HYPRDESK_INSTALL_STAGE=$(mktemp "$BIN_DIR/.hyprdesk-install.XXXXXX")
trap 'rm -f -- "$HYPRDESK_INSTALL_STAGE"' EXIT
install -m755 target/release/hyprdesk "$HYPRDESK_INSTALL_STAGE"
mv -f -- "$HYPRDESK_INSTALL_STAGE" "$BIN"
trap - EXIT
hd_ok "$("$BIN" --version 2>/dev/null || echo hyprdesk) → $BIN"

# -- Configuration ---------------------------------------------------------------------
hd_step "🛠" "Configuration" "Configuration" \
    "~/.config/hypr/hyprdesk.lua créé s'il manque ; une configuration existante est gardée." \
    "~/.config/hypr/hyprdesk.lua created if missing; an existing configuration is kept."
HD_HINT="$(t "hyprdesk check signale une erreur dans ~/.config/hypr/hyprdesk.lua : corrigez la ligne indiquée dans le journal" \
             "hyprdesk check reports an error in ~/.config/hypr/hyprdesk.lua: fix the line shown in the log")"
hd_run "$(t "configuration de départ" "starter configuration")" "$BIN" init
hd_run "$(t "définitions pour l'éditeur (stubs Lua)" "editor definitions (Lua stubs)")" "$BIN" stubs
hd_run "$(t "configuration vérifiée" "configuration checked")" "$BIN" check

if [[ -n "$LANG_CHOICE" ]]; then
    hd_step "🌍" "Langue" "Language"
    "$BIN" config set language "$LANG_CHOICE" >>"$HD_LOG" 2>&1
    hd_ok "$(t "hyprdesk en" "hyprdesk in") : $LANG_CHOICE"
fi

if [[ -n "$KEYBOARD" ]]; then
    hd_step "⌨" "Clavier" "Keyboard" \
        "inputs.lua, chargé par hyprland.lua." "inputs.lua, loaded by hyprland.lua."
    HD_HINT="$(t "disposition ou variante inconnue : localectl list-x11-keymap-layouts (et -variants <disposition>)" \
                 "unknown layout or variant: localectl list-x11-keymap-layouts (and -variants <layout>)")"
    IFS=: read -r KB_LAYOUT KB_VARIANT KB_OPTIONS <<< "$KEYBOARD"
    hd_run "$(t "clavier" "keyboard") $KB_LAYOUT${KB_VARIANT:+ ($KB_VARIANT)}" "$BIN" keyboard "$KB_LAYOUT" "${KB_VARIANT:-}" "${KB_OPTIONS:-}"
    # inputs.lua n'agit que si hyprland.lua le charge.
    if [[ -f "$HYPR_CONF" ]] && [[ -f "$CONFIG_DIR/inputs.lua" ]] && ! grep -qE '^[[:space:]]*require\("inputs"\)' "$HYPR_CONF"; then
        backup clavier
        printf '\n-- hyprdesk : clavier et souris (Réglages → Clavier et souris ; écrit par scripts/install.sh)\nrequire("inputs")\n' >> "$HYPR_CONF"
        hd_ok "$(t "require(\"inputs\") ajouté à hyprland.lua" "require(\"inputs\") added to hyprland.lua")"
    fi
    if [[ "$SYSTEM_KEYBOARD" == 1 ]]; then
        if command -v localectl >/dev/null 2>&1; then
            if hd_sudo_ready "$(t "changer le clavier du système" "change the system keyboard")" \
                && hyprdesk_root localectl set-x11-keymap "$KB_LAYOUT" "" "${KB_VARIANT:-}" "${KB_OPTIONS:-}" >>"$HD_LOG" 2>&1; then
                hd_ok "$(t "clavier du système" "system keyboard") : $KB_LAYOUT ${KB_VARIANT:-}"
            else
                hd_warn "$(t "clavier du système non modifié (localectl)" "system keyboard unchanged (localectl)")"
            fi
        else
            hd_warn "$(t "localectl absent : clavier du système non modifié" "no localectl: system keyboard unchanged")"
        fi
    fi
fi

# -- Dépendances facultatives ----------------------------------------------------------
hd_step "🩺" "Dépendances facultatives" "Optional dependencies" \
    "Une absente rend seulement sa fonction inopérante (détail : hyprdesk doctor)." \
    "A missing one only disables its feature (details: hyprdesk doctor)."
DOCTOR_OUT="$("$BIN" doctor 2>/dev/null || true)"
printf '%s\n' "$DOCTOR_OUT" >>"$HD_LOG"
DOCTOR="$(grep -Eo '[0-9]+/[0-9]+' <<< "$DOCTOR_OUT" | tail -1 || true)"
OPT_PKGS=()
while IFS= read -r l; do
    [[ -n "$l" ]] || continue
    name="$(awk '{print $2}' <<< "$l")"
    what="$(sed -E 's/^[[:space:]]*✗[[:space:]]+[^[:space:]]+[[:space:]]+//; s/[[:space:]]*\((paquet|package)[[:space:]]*:[^)]*\)//' <<< "$l")"
    pkg="$(sed -nE 's/.*\((paquet|package)[[:space:]]*:[[:space:]]*([^)]+)\).*/\2/p' <<< "$l")"
    hd_warn "$C_BOLD$name$C_RESET — $what"
    if [[ -n "$pkg" ]]; then OPT_PKGS+=("$pkg"); fi
done < <(grep '✗' <<< "$DOCTOR_OUT" || true)
hd_ok "${DOCTOR:-?} $(t "présentes" "present")"

# -- Service ---------------------------------------------------------------------------
hd_step "🔁" "Service systemd utilisateur" "systemd user service" \
    "hyprdesk.service, relancé s'il s'arrête." "hyprdesk.service, restarted if it stops."
HD_HINT="$(t "le service ne démarre pas : journalctl --user -u hyprdesk -n 50" "the service does not start: journalctl --user -u hyprdesk -n 50")"
mkdir -p "$UNIT_DIR"
# Pas de WAYLAND_DISPLAY figé : il change d'un poste et d'une session à
# l'autre ; Hyprland le transmet à systemd au démarrage.
install -m644 assets/hyprdesk.service "$UNIT_DIR/hyprdesk.service"
if [[ -n "${WAYLAND_DISPLAY:-}" ]]; then
    systemctl --user import-environment WAYLAND_DISPLAY HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP 2>/dev/null || true
fi
# Surcharges locales (relance, environnement…) : jamais écrasées ici.
mkdir -p "$UNIT_DIR/hyprdesk.service.d"
systemctl --user daemon-reload
systemctl --user enable hyprdesk.service >/dev/null 2>&1
hd_ok "$(t "service installé et activé" "service installed and enabled")"
if [[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]]; then
    wait_ready() {
        systemctl --user restart hyprdesk.service
        for _ in $(seq 1 20); do
            "$BIN" status >/dev/null 2>&1 && return 0
            sleep 0.5
        done
        return 1
    }
    hd_run "$(t "démarrage de hyprdesk" "starting hyprdesk")" wait_ready \
        || hd_warn "$(t "hyprdesk ne répond pas encore : voir le bilan" "hyprdesk does not answer yet: see the report")"
else
    hd_info "$(t "hors d'une session Hyprland : le service démarrera à la prochaine connexion" \
                 "outside a Hyprland session: the service starts at next login")"
fi

START_MARK="hyprdesk : démarrage à la connexion"
START_CMD="hl.exec_cmd(\"sh -c 'systemctl --user import-environment WAYLAND_DISPLAY HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP; systemctl --user start hyprdesk.service'\")"
if [[ "$AUTOSTART" == 1 ]]; then
    hd_step "🚀" "Démarrage à la connexion" "Start at login" \
        "Une ligne dans hyprland.lua, sans dépendre de graphical-session.target." \
        "One line in hyprland.lua, without depending on graphical-session.target."
    if [[ ! -f "$HYPR_CONF" ]]; then
        hd_warn "$(t "hyprland.lua absent" "hyprland.lua missing")"
        hd_hint "$(t "ajoutez dans hl.on(\"hyprland.start\", …) :" "add inside hl.on(\"hyprland.start\", …):")"
        hd_cmd "$START_CMD"
    elif grep -q "$START_MARK" "$HYPR_CONF"; then
        hd_ok "$(t "déjà en place" "already set")"
    else
        backup demarrage
        if grep -qE '^hl\.on\("hyprland\.start", function\(\)' "$HYPR_CONF"; then
            # Bloc de démarrage existant (celui du modèle de Hyprland) : la ligne y entre en tête.
            START_LINE="    -- $START_MARK (écrit par scripts/install.sh) : environnement
    -- transmis à systemd, puis le service, sans attendre graphical-session.target.
    $START_CMD"
            awk -v line="$START_LINE" '
                !done && /^hl\.on\("hyprland\.start", function\(\)/ { print; print line; done = 1; next }
                { print }' "$HYPR_CONF" > "$HYPR_CONF.hyprdesk-tmp" && mv -f "$HYPR_CONF.hyprdesk-tmp" "$HYPR_CONF"
        else
            cat >> "$HYPR_CONF" <<LUA

-- $START_MARK (écrit par scripts/install.sh) : environnement de la session
-- transmis à systemd, puis le service, sans attendre graphical-session.target,
-- que toutes les distributions n'activent pas sous Hyprland.
hl.on("hyprland.start", function()
    $START_CMD
end)
LUA
        fi
        hd_ok "$(t "ajouté à hyprland.lua" "added to hyprland.lua")"
    fi
fi

LOCK_CMD="$BIN lock"
# Raccourci SUPER + L : « mainMod .. " + L" » ou « "SUPER + L" ».
LOCK_KEY_RE='hl\.bind\((mainMod[[:space:]]*\.\.[[:space:]]*"[[:space:]]*\+[[:space:]]*L"|"SUPER[[:space:]]*\+[[:space:]]*L")'
if [[ "$LOCK_KEY" == 1 ]]; then
    hd_step "🔐" "Raccourci de verrouillage (SUPER + L)" "Lock shortcut (SUPER + L)"
    if [[ ! -f "$HYPR_CONF" ]]; then
        hd_warn "$(t "hyprland.lua absent ; à ajouter :" "hyprland.lua missing; to add:")"
        hd_cmd "hl.bind(\"SUPER + L\", hl.dsp.exec_cmd(\"$LOCK_CMD\"))"
    elif grep -Eq "$LOCK_KEY_RE.*hyprdesk lock" "$HYPR_CONF"; then
        hd_ok "$(t "déjà en place" "already set")"
    elif grep -Eq "$LOCK_KEY_RE.*exec_cmd\(\"hyprlock\"\)" "$HYPR_CONF"; then
        backup lock
        sed -E -i "/$LOCK_KEY_RE/ s#exec_cmd\(\"hyprlock\"\)#exec_cmd(\"$LOCK_CMD\")#" "$HYPR_CONF"
        hd_ok "$(t "hyprlock remplacé par hyprdesk lock" "hyprlock replaced by hyprdesk lock")"
    elif grep -Eq "$LOCK_KEY_RE" "$HYPR_CONF"; then
        hd_warn "$(t "SUPER + L sert déjà à autre chose : laissé tel quel" "SUPER + L is already used: left as is")"
    else
        backup lock
        printf '\n-- hyprdesk : verrouillage (écrit par scripts/install.sh)\nhl.bind("SUPER + L", hl.dsp.exec_cmd("%s"), { description = "hyprdesk : verrouiller" })\n' \
            "$LOCK_CMD" >> "$HYPR_CONF"
        hd_ok "$(t "ajouté à hyprland.lua" "added to hyprland.lua")"
    fi
fi

if [[ "$REMOVE_HYPRPAPER" == 1 ]]; then
    hd_step "🖼" "hyprpaper retiré de l'autostart" "hyprpaper removed from autostart" \
        "hyprdesk dessine lui-même le fond d'écran." "hyprdesk draws the wallpaper itself."
    DONE_PAPER=0
    if [[ -f "$HYPR_CONF" ]] && grep -qE '^[[:space:]]*hl\.exec_cmd\(.*hyprpaper' "$HYPR_CONF"; then
        backup hyprpaper
        sed -E -i 's/^([[:space:]]*)(hl\.exec_cmd\(.*hyprpaper.*)$/\1-- \2 -- retiré : hyprdesk dessine le fond d'\''écran/' "$HYPR_CONF"
        hd_ok "$(t "lignes hyprpaper mises en commentaire" "hyprpaper lines commented out")"
        DONE_PAPER=1
    fi
    if systemctl --user is-enabled hyprpaper.service >/dev/null 2>&1; then
        systemctl --user disable --now hyprpaper.service >/dev/null 2>&1 || true
        hd_ok "$(t "hyprpaper.service désactivé" "hyprpaper.service disabled")"
        DONE_PAPER=1
    fi
    [[ "$DONE_PAPER" == 1 ]] || hd_ok "$(t "hyprpaper n'était pas lancé" "hyprpaper was not started")"
fi

# -- Écran de connexion ----------------------------------------------------------------
hd_step "🚪" "Écran de connexion" "Login screen" \
    "Facultatif : l'écran de hyprdesk (greetd) à la place de GDM, SDDM…" \
    "Optional: the hyprdesk screen (greetd) instead of GDM, SDDM…"
DM_UNIT=""
if [[ -L /etc/systemd/system/display-manager.service ]]; then
    DM_UNIT="$(basename "$(readlink -f /etc/systemd/system/display-manager.service)")"
fi
greeter_ours() {
    [[ "$DM_UNIT" == "greetd.service" ]] && "$BIN" greeter setup status 2>/dev/null | grep -q "hyprdesk"
}
GREETER_NEW=0
if greeter_ours; then
    hd_ok "$(t "déjà l'écran de hyprdesk (greetd)" "already the hyprdesk login screen (greetd)")"
    GREETER=no
elif [[ "$GREETER" == ask ]]; then
    if [[ -t 0 ]]; then
        printf '  %s?%s ' "$C_ACCENT$C_BOLD" "$C_RESET"
        read -r -p "$(t "Remplacer ${DM_UNIT:-le gestionnaire actuel} par l'écran de connexion de hyprdesk ? [o/N] " "Replace ${DM_UNIT:-the current display manager} with the hyprdesk login screen? [y/N] ")" reply
        [[ "$reply" =~ ^[oOyY] ]] && GREETER=yes || GREETER=no
    else
        hd_info "$(t "non interactif : relancez avec --greeter pour le mettre en place" "non-interactive: run again with --greeter to set it up")"
        GREETER=no
    fi
fi
if [[ "$GREETER" == no ]] && ! greeter_ours; then
    hd_info "$(t "inchangé" "unchanged")${DM_UNIT:+ ($DM_UNIT)}"
fi
if [[ "$GREETER" == yes ]]; then
    # Langue de l'écran de connexion : celle choisie, si la locale existe.
    GREETER_LANG="${LANG:-}"
    case "$LANG_CHOICE" in
        fr) locale -a 2>/dev/null | grep -qi '^fr_FR\.utf-\?8$' && GREETER_LANG=fr_FR.UTF-8 ;;
        en) locale -a 2>/dev/null | grep -qi '^en_US\.utf-\?8$' && GREETER_LANG=en_US.UTF-8 ;;
    esac
    PROFILE="$(mktemp --suffix=.json)"
    LANG="$GREETER_LANG" "$BIN" greeter setup profile "$PROFILE" >>"$HD_LOG" 2>&1
    chmod 644 "$PROFILE"
    if hd_sudo_ready "$(t "installer l'écran de connexion" "install the login screen")" \
        && hd_run "$(t "paquets greetd et cage" "greetd and cage packages")" hyprdesk_root "$BIN" greeter setup packages \
        && hd_run "$(t "configuration de greetd" "greetd configuration")" hyprdesk_root "$BIN" greeter setup install "$PROFILE" \
        && hd_run "$(t "bascule vers greetd" "switch to greetd")" hyprdesk_root "$BIN" greeter setup enable; then
        DM_UNIT="greetd.service"
        GREETER_NEW=1
        hd_ok "$(t "activé : il s'affichera au prochain démarrage" "enabled: shown at next boot")"
        hd_note "$(t "retour en arrière : sudo hyprdesk greeter setup restore" "undo: sudo hyprdesk greeter setup restore")"
    else
        hd_warn "$(t "écran de connexion non mis en place ; rien n'est basculé tant que « enable » n'a pas réussi" \
                     "login screen not set up; nothing is switched until “enable” succeeds")"
    fi
    rm -f "$PROFILE"
fi

# == Bilan vérifié et ce qu'il reste à faire ==========================================
trap - ERR
set +e
FAILED=0
TODO=()     # « texte|commande »
OPTIONAL=() # idem, facultatif
todo() { TODO+=("$1|${2:-}"); }
optional() { OPTIONAL+=("$1|${2:-}"); }

hd_section "📋" "Bilan" "Report"
# check <essentiel 0|1> <libellé fr> <libellé en> <correction fr> <correction en> <commande…>
check() {
    local essential="$1" fr="$2" en="$3" fix_fr="$4" fix_en="$5"; shift 5
    if "$@" >/dev/null 2>&1; then
        hd_ok "$(t "$fr" "$en")"
    else
        if [[ "$essential" == 1 ]]; then
            hd_ko "$(t "$fr" "$en")"
            FAILED=$((FAILED + 1))
        else
            hd_warn "$(t "$fr" "$en")"
        fi
        if [[ -n "$fix_fr" ]]; then
            hd_hint "$(t "$fix_fr" "$fix_en")"
            [[ "$essential" == 1 ]] && todo "$(t "$fix_fr" "$fix_en")"
        fi
    fi
    return 0
}
JOURNAL="journalctl --user -u hyprdesk -n 50"
check 1 "binaire $BIN ($("$BIN" --version 2>/dev/null))" "binary $BIN ($("$BIN" --version 2>/dev/null))" \
    "relancez l'installation" "run the install again" test -x "$BIN"
check 1 "configuration valide" "valid configuration" \
    "corrigez ~/.config/hypr/hyprdesk.lua (hyprdesk check dit où)" "fix ~/.config/hypr/hyprdesk.lua (hyprdesk check says where)" "$BIN" check
check 1 "service activé à la connexion" "service enabled at login" \
    "activez-le : systemctl --user enable hyprdesk" "enable it: systemctl --user enable hyprdesk" \
    systemctl --user is-enabled hyprdesk.service
IN_SESSION=0
[[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]] && IN_SESSION=1
HYPR_RUN=""
if [[ "$IN_SESSION" == 1 ]]; then
    # Hyprland de la session en cours : hyprdesk refuse de démarrer sous 0.57.
    HYPR_RUN="$(hyprdesk_hyprland_running || true)"
    check 1 "Hyprland ${HYPR_RUN:-?} en cours (0.57 ou plus)" "running Hyprland ${HYPR_RUN:-?} (0.57 or newer)" \
        "mettez Hyprland à jour, puis reconnectez-vous : hyprdesk démarrera" "update Hyprland, then log out and back in: hyprdesk will start" \
        hyprdesk_version_ge "${HYPR_RUN:-0}" 0.57
    check 1 "hyprdesk en marche (il répond)" "hyprdesk running (it answers)" \
        "voir pourquoi : $JOURNAL" "see why: $JOURNAL" "$BIN" status
    check 1 "Hyprland sans erreur de configuration" "Hyprland config without errors" \
        "erreurs signalées par : hyprctl configerrors" "errors listed by: hyprctl configerrors" \
        bash -c '[[ -z "$(hyprctl configerrors 2>/dev/null | grep -v "^$")" ]]'
fi
if [[ "$AUTOSTART" == 1 ]]; then
    check 1 "démarrage à la connexion dans hyprland.lua" "start at login in hyprland.lua" \
        "ajoutez la ligne de démarrage (voir l'étape « Démarrage à la connexion »)" "add the start line (see the “Start at login” step)" \
        grep -q "$START_MARK" "$HYPR_CONF"
fi
if [[ "$LOCK_KEY" == 1 ]]; then
    check 0 "SUPER + L verrouille avec hyprdesk" "SUPER + L locks with hyprdesk" \
        "SUPER + L déjà pris : liez hyprdesk lock à une autre touche dans hyprland.lua" "SUPER + L already taken: bind hyprdesk lock to another key in hyprland.lua" \
        grep -Eq "$LOCK_KEY_RE.*hyprdesk lock" "$HYPR_CONF"
fi
if [[ -n "$LANG_CHOICE" ]]; then
    check 0 "langue : $LANG_CHOICE" "language: $LANG_CHOICE" "" "" \
        bash -c "[[ \"\$('$BIN' config get language)\" == '$LANG_CHOICE' ]]"
fi
if [[ -n "$KEYBOARD" ]]; then
    check 0 "clavier : $KB_LAYOUT ${KB_VARIANT:-}" "keyboard: $KB_LAYOUT ${KB_VARIANT:-}" \
        "Réglages → Clavier et souris" "Settings → Keyboard and mouse" \
        bash -c "grep -q 'kb_layout = \"$KB_LAYOUT\"' '$CONFIG_DIR/inputs.lua' && grep -qE '^[[:space:]]*require\\(\"inputs\"\\)' '$HYPR_CONF'"
fi
if [[ "$REMOVE_HYPRPAPER" == 1 ]]; then
    check 0 "hyprpaper retiré de l'autostart" "hyprpaper removed from autostart" "" "" \
        bash -c "! grep -qE '^[[:space:]]*hl\\.exec_cmd\\(.*hyprpaper' '$HYPR_CONF'"
fi
if [[ "$GREETER" == yes ]] || greeter_ours; then
    check 0 "écran de connexion de hyprdesk (greetd)" "hyprdesk login screen (greetd)" \
        "réessayez : sudo hyprdesk greeter setup enable" "try again: sudo hyprdesk greeter setup enable" greeter_ours
fi
hd_info "$(t "dépendances facultatives : ${DOCTOR:-?} présentes" "optional dependencies: ${DOCTOR:-?} present")"
if ((${#BACKUPS[@]} > 0)); then
    hd_note "$(t "sauvegardes de hyprland.lua :" "hyprland.lua backups:") ${BACKUPS[*]##*/}"
fi

# Ce qu'il reste à faire pour que hyprdesk tourne.
HYPR_INST="$(hyprdesk_hyprland_installed || true)"
if [[ "$IN_SESSION" == 0 ]]; then
    todo "$(t "ouvrez une session Hyprland : hyprdesk s'y lance tout seul" "log in to a Hyprland session: hyprdesk starts by itself")"
elif [[ -n "$HYPR_INST" && -n "$HYPR_RUN" && "$HYPR_INST" != "$HYPR_RUN" ]]; then
    todo "$(t "reconnectez-vous : la session tourne en Hyprland $HYPR_RUN, $HYPR_INST est installé" \
              "log out and back in: the session runs Hyprland $HYPR_RUN, $HYPR_INST is installed")"
fi
if [[ "$GREETER_NEW" == 1 ]]; then
    todo "$(t "redémarrez pour voir l'écran de connexion de hyprdesk" "reboot to see the hyprdesk login screen")" "systemctl reboot"
fi
case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *)
        case "${SHELL##*/}" in
            fish) PATH_CMD="fish_add_path ~/.local/bin" ;;
            zsh) PATH_CMD="echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc" ;;
            *) PATH_CMD="echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc" ;;
        esac
        optional "$(t "~/.local/bin dans le PATH, pour taper hyprdesk dans un terminal" "~/.local/bin in PATH, to type hyprdesk in a terminal")" "$PATH_CMD" ;;
esac
if ((${#OPT_PKGS[@]} > 0)); then
    optional "$(t "dépendances facultatives absentes (voir l'étape Dépendances facultatives)" "missing optional dependencies (see the Optional dependencies step)")" \
        "$(hyprdesk_install_hint) ${OPT_PKGS[*]}"
fi
if [[ "$GREETER" == no ]] && ! greeter_ours; then
    optional "$(t "écran de connexion de hyprdesk" "hyprdesk login screen")" "./scripts/install.sh --greeter"
fi
optional "$(t "lignes conseillées pour hyprland.lua (raccourcis, règles)" "suggested hyprland.lua lines (shortcuts, rules)")" "hyprdesk snippet"

print_list() { # print_list <élément…>
    local i=1 item text cmd
    for item in "$@"; do
        text="${item%%|*}" cmd="${item#*|}"
        printf '  %s%d.%s %s\n' "$C_ACCENT$C_BOLD" "$i" "$C_RESET" "$text"
        [[ -n "$cmd" ]] && hd_cmd "$cmd"
        i=$((i + 1))
    done
}
hd_section "📝" "Ce qu'il reste à faire" "What is left to do"
if ((${#TODO[@]} > 0)); then
    print_list "${TODO[@]}"
else
    hd_ok "$(t "rien : hyprdesk est prêt" "nothing: hyprdesk is ready")"
fi
if ((${#OPTIONAL[@]} > 0)); then
    printf '\n  %s%s%s\n' "$C_GREY" "$(t "Facultatif :" "Optional:")" "$C_RESET"
    print_list "${OPTIONAL[@]}"
fi

hd_section "⌨" "Premiers pas" "First steps"
key() { printf '  %s%-18s%s %s\n' "$C_ACCENT$C_BOLD" "$1" "$C_RESET" "$2"; }
key "SUPER + D" "$(t "bureau (widgets, fond)" "desktop (widgets, wallpaper)")"
key "SUPER + R" "$(t "lanceur d'applications" "application launcher")"
key "SUPER + SHIFT + D" "$(t "Réglages" "Settings")"
[[ "$LOCK_KEY" == 1 ]] && key "SUPER + L" "$(t "verrouiller" "lock")"

# Verdict.
if ((FAILED > 0)); then
    hd_panel "$C_RED" \
        "$C_RED$C_BOLD$I_KO $(t "hyprdesk n'est pas encore fonctionnel : $FAILED point(s) essentiel(s) en échec" "hyprdesk is not working yet: $FAILED essential check(s) failed")$C_RESET" \
        "$(t "suivez « Ce qu'il reste à faire » ci-dessus ; journal :" "follow “What is left to do” above; log:") $HD_LOG"
    echo
    exit 1
elif ((${#TODO[@]} > 0)); then
    hd_panel "$C_YELLOW" \
        "$C_YELLOW$C_BOLD$I_WARN $(t "hyprdesk est installé : encore ${#TODO[@]} action(s) pour qu'il tourne" "hyprdesk is installed: ${#TODO[@]} action(s) left before it runs")$C_RESET" \
        "$(t "voir « Ce qu'il reste à faire » ci-dessus" "see “What is left to do” above")"
else
    hd_panel "$C_GREEN" \
        "$C_GREEN$C_BOLD$I_OK $(t "hyprdesk est installé et fonctionne" "hyprdesk is installed and running")$C_RESET" \
        "$(t "Réglages : SUPER + SHIFT + D, ou le bouton du dock" "Settings: SUPER + SHIFT + D, or the dock button")"
fi
echo
exit 0
