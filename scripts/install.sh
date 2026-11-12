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
#
# Étapes : prérequis (scripts/deps.sh), compilation, binaire dans ~/.local/bin,
# police, configuration, service systemd utilisateur, langue, clavier,
# démarrage à la connexion (hyprland.lua, sans dépendre de
# graphical-session.target), SUPER + L, hyprpaper, écran de connexion, puis
# un bilan vérifié point par point (code de sortie non nul si l'essentiel manque).
# Chaque modification de hyprland.lua est précédée d'une copie de sauvegarde.
set -euo pipefail
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
        -h|--help) sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
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
t() { if [[ "$UI" == en ]]; then printf '%s' "$2"; else printf '%s' "$1"; fi; }
step() { echo "== $(t "$1" "$2")"; }
note() { echo "   $(t "$1" "$2")"; }

HYPRDESK_ROOT="$PWD"
# shellcheck source=deps.sh
source scripts/deps.sh
hyprdesk_deps

BIN_DIR="$HOME/.local/bin"
BIN="$BIN_DIR/hyprdesk"
UNIT_DIR="$HOME/.config/systemd/user"
FONT_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/fonts/hyprdesk/Mindlog"
HYPR_CONF="${XDG_CONFIG_HOME:-$HOME/.config}/hypr/hyprland.lua"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/hypr"

# Copie de sauvegarde de hyprland.lua, une fois par étape.
backup() { cp -p "$HYPR_CONF" "$HYPR_CONF.avant-hyprdesk-$1.bak"; }

step "compilation" "build"
cargo build --release --locked

step "police Mindlog -> $FONT_DIR" "Mindlog font -> $FONT_DIR"
mkdir -p "$FONT_DIR"
install -m644 assets/fonts/mindlog/*.ttf assets/fonts/mindlog/OFL.txt "$FONT_DIR/"
if command -v fc-cache >/dev/null 2>&1; then
    fc-cache -f "$FONT_DIR"
fi

step "binaire -> $BIN_DIR" "binary -> $BIN_DIR"
mkdir -p "$BIN_DIR"
HYPRDESK_INSTALL_STAGE=$(mktemp "$BIN_DIR/.hyprdesk-install.XXXXXX")
trap 'rm -f -- "$HYPRDESK_INSTALL_STAGE"' EXIT
install -m755 target/release/hyprdesk "$HYPRDESK_INSTALL_STAGE"
mv -f -- "$HYPRDESK_INSTALL_STAGE" "$BIN"
trap - EXIT

step "configuration" "configuration"
"$BIN" init
"$BIN" stubs
"$BIN" check

if [[ -n "$LANG_CHOICE" ]]; then
    step "langue" "language"
    "$BIN" config set language "$LANG_CHOICE" >/dev/null
    note "hyprdesk : $LANG_CHOICE" "hyprdesk: $LANG_CHOICE"
fi

if [[ -n "$KEYBOARD" ]]; then
    step "clavier" "keyboard"
    IFS=: read -r KB_LAYOUT KB_VARIANT KB_OPTIONS <<< "$KEYBOARD"
    "$BIN" keyboard "$KB_LAYOUT" "${KB_VARIANT:-}" "${KB_OPTIONS:-}"
    # inputs.lua n'agit que si hyprland.lua le charge.
    if [[ -f "$HYPR_CONF" ]] && [[ -f "$CONFIG_DIR/inputs.lua" ]] && ! grep -qE '^[[:space:]]*require\("inputs"\)' "$HYPR_CONF"; then
        backup clavier
        printf '\n-- hyprdesk : clavier et souris (Réglages → Clavier et souris ; écrit par scripts/install.sh)\nrequire("inputs")\n' >> "$HYPR_CONF"
        note "require(\"inputs\") ajouté à $HYPR_CONF" "require(\"inputs\") added to $HYPR_CONF"
    fi
    if [[ "$SYSTEM_KEYBOARD" == 1 ]]; then
        if command -v localectl >/dev/null 2>&1; then
            hyprdesk_root localectl set-x11-keymap "$KB_LAYOUT" "" "${KB_VARIANT:-}" "${KB_OPTIONS:-}" \
                && note "clavier du système : $KB_LAYOUT ${KB_VARIANT:-}" "system keyboard: $KB_LAYOUT ${KB_VARIANT:-}" \
                || note "clavier du système non modifié (localectl)" "system keyboard unchanged (localectl)"
        else
            note "localectl absent : clavier du système non modifié" "no localectl: system keyboard unchanged"
        fi
    fi
fi

step "dépendances externes" "external dependencies"
# Informatif : une dépendance absente rend seulement sa fonction inopérante.
"$BIN" doctor || true

step "service systemd utilisateur" "systemd user service"
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
if [[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]]; then
    systemctl --user restart hyprdesk.service
    for _ in $(seq 1 20); do
        "$BIN" status >/dev/null 2>&1 && break
        sleep 0.5
    done
else
    note "hors d'une session Hyprland : le service démarrera à la prochaine connexion" \
         "outside a Hyprland session: the service starts at next login"
fi

START_MARK="hyprdesk : démarrage à la connexion"
START_CMD="hl.exec_cmd(\"sh -c 'systemctl --user import-environment WAYLAND_DISPLAY HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP; systemctl --user start hyprdesk.service'\")"
if [[ "$AUTOSTART" == 1 ]]; then
    step "démarrage à la connexion" "start at login"
    if [[ ! -f "$HYPR_CONF" ]]; then
        note "$HYPR_CONF absent : ajoutez dans hl.on(\"hyprland.start\", …) : $START_CMD" \
             "$HYPR_CONF missing: add inside hl.on(\"hyprland.start\", …): $START_CMD"
    elif grep -q "$START_MARK" "$HYPR_CONF"; then
        note "déjà en place" "already set"
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
        note "ajouté à $HYPR_CONF" "added to $HYPR_CONF"
    fi
fi

LOCK_CMD="$BIN lock"
# Raccourci SUPER + L : « mainMod .. " + L" » ou « "SUPER + L" ».
LOCK_KEY_RE='hl\.bind\((mainMod[[:space:]]*\.\.[[:space:]]*"[[:space:]]*\+[[:space:]]*L"|"SUPER[[:space:]]*\+[[:space:]]*L")'
if [[ "$LOCK_KEY" == 1 ]]; then
    step "raccourci de verrouillage (SUPER + L)" "lock shortcut (SUPER + L)"
    if [[ ! -f "$HYPR_CONF" ]]; then
        note "$HYPR_CONF absent : ajoutez hl.bind(\"SUPER + L\", hl.dsp.exec_cmd(\"$LOCK_CMD\"))" \
             "$HYPR_CONF missing: add hl.bind(\"SUPER + L\", hl.dsp.exec_cmd(\"$LOCK_CMD\"))"
    elif grep -Eq "$LOCK_KEY_RE.*hyprdesk lock" "$HYPR_CONF"; then
        note "déjà en place" "already set"
    elif grep -Eq "$LOCK_KEY_RE.*exec_cmd\(\"hyprlock\"\)" "$HYPR_CONF"; then
        backup lock
        sed -E -i "/$LOCK_KEY_RE/ s#exec_cmd\(\"hyprlock\"\)#exec_cmd(\"$LOCK_CMD\")#" "$HYPR_CONF"
        note "hyprlock remplacé par hyprdesk lock" "hyprlock replaced by hyprdesk lock"
    elif grep -Eq "$LOCK_KEY_RE" "$HYPR_CONF"; then
        note "SUPER + L sert déjà à autre chose : laissé tel quel" "SUPER + L is already used: left as is"
    else
        backup lock
        printf '\n-- hyprdesk : verrouillage (écrit par scripts/install.sh)\nhl.bind("SUPER + L", hl.dsp.exec_cmd("%s"), { description = "hyprdesk : verrouiller" })\n' \
            "$LOCK_CMD" >> "$HYPR_CONF"
        note "ajouté à $HYPR_CONF" "added to $HYPR_CONF"
    fi
fi

if [[ "$REMOVE_HYPRPAPER" == 1 ]]; then
    step "hyprpaper retiré de l'autostart" "hyprpaper removed from autostart"
    if [[ -f "$HYPR_CONF" ]] && grep -qE '^[[:space:]]*hl\.exec_cmd\(.*hyprpaper' "$HYPR_CONF"; then
        backup hyprpaper
        sed -E -i 's/^([[:space:]]*)(hl\.exec_cmd\(.*hyprpaper.*)$/\1-- \2 -- retiré : hyprdesk dessine le fond d'\''écran/' "$HYPR_CONF"
        note "lignes hyprpaper mises en commentaire" "hyprpaper lines commented out"
    fi
    if systemctl --user is-enabled hyprpaper.service >/dev/null 2>&1; then
        systemctl --user disable --now hyprpaper.service >/dev/null 2>&1 || true
        note "hyprpaper.service désactivé" "hyprpaper.service disabled"
    fi
fi

step "écran de connexion" "login screen"
DM_UNIT=""
if [[ -L /etc/systemd/system/display-manager.service ]]; then
    DM_UNIT="$(basename "$(readlink -f /etc/systemd/system/display-manager.service)")"
fi
greeter_ours() {
    [[ "$DM_UNIT" == "greetd.service" ]] && "$BIN" greeter setup status 2>/dev/null | grep -q "hyprdesk"
}
if greeter_ours; then
    note "déjà l'écran de hyprdesk (greetd)" "already the hyprdesk login screen (greetd)"
    GREETER=no
elif [[ "$GREETER" == ask ]]; then
    if [[ -t 0 ]]; then
        read -r -p "   $(t "Remplacer ${DM_UNIT:-le gestionnaire actuel} par l'écran de connexion de hyprdesk ? [o/N] " "Replace ${DM_UNIT:-the current display manager} with the hyprdesk login screen? [y/N] ")" reply
        [[ "$reply" =~ ^[oOyY] ]] && GREETER=yes || GREETER=no
    else
        note "non interactif : relancez avec --greeter pour le mettre en place" "non-interactive: run again with --greeter to set it up"
        GREETER=no
    fi
fi
if [[ "$GREETER" == yes ]]; then
    # Langue de l'écran de connexion : celle choisie, si la locale existe.
    GREETER_LANG="${LANG:-}"
    case "$LANG_CHOICE" in
        fr) locale -a 2>/dev/null | grep -qi '^fr_FR\.utf-\?8$' && GREETER_LANG=fr_FR.UTF-8 ;;
        en) locale -a 2>/dev/null | grep -qi '^en_US\.utf-\?8$' && GREETER_LANG=en_US.UTF-8 ;;
    esac
    PROFILE="$(mktemp --suffix=.json)"
    LANG="$GREETER_LANG" "$BIN" greeter setup profile "$PROFILE"
    chmod 644 "$PROFILE"
    if hyprdesk_root "$BIN" greeter setup packages \
        && hyprdesk_root "$BIN" greeter setup install "$PROFILE" \
        && hyprdesk_root "$BIN" greeter setup enable; then
        DM_UNIT="greetd.service"
        note "activé : il s'affichera au prochain démarrage (retour : sudo hyprdesk greeter setup restore)" \
             "enabled: shown at next boot (undo: sudo hyprdesk greeter setup restore)"
    else
        echo "!! $(t "écran de connexion non mis en place ; rien n'est basculé tant que « enable » n'a pas réussi" \
                     "login screen not set up; nothing is switched until “enable” succeeds")" >&2
    fi
    rm -f "$PROFILE"
fi

# -- Bilan vérifié ----------------------------------------------------------------
echo
echo "=================================================================="
echo " $(t "Bilan de l'installation" "Installation report")"
echo "=================================================================="
FAILED=0
check() { # check <essentiel 0|1> <libellé fr> <libellé en> <commande…>
    local essential="$1" fr="$2" en="$3"; shift 3
    if "$@" >/dev/null 2>&1; then
        echo "  ✓ $(t "$fr" "$en")"
    else
        echo "  ✗ $(t "$fr" "$en")"
        [[ "$essential" == 1 ]] && FAILED=1
    fi
    return 0
}
check 1 "binaire $BIN ($("$BIN" --version 2>/dev/null))" "binary $BIN ($("$BIN" --version 2>/dev/null))" test -x "$BIN"
check 1 "configuration valide" "valid configuration" "$BIN" check
check 1 "service activé à la connexion" "service enabled at login" systemctl --user is-enabled hyprdesk.service
if [[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]]; then
    check 1 "service en marche (hyprdesk répond)" "service running (hyprdesk answers)" "$BIN" status
    check 1 "Hyprland sans erreur de configuration" "Hyprland config without errors" \
        bash -c '[[ -z "$(hyprctl configerrors 2>/dev/null | grep -v "^$")" ]]'
fi
[[ "$AUTOSTART" == 1 ]] && check 1 "démarrage à la connexion dans hyprland.lua" "start at login in hyprland.lua" grep -q "$START_MARK" "$HYPR_CONF"
[[ "$LOCK_KEY" == 1 ]] && check 0 "SUPER + L verrouille avec hyprdesk" "SUPER + L locks with hyprdesk" grep -Eq "$LOCK_KEY_RE.*hyprdesk lock" "$HYPR_CONF"
[[ -n "$LANG_CHOICE" ]] && check 0 "langue : $LANG_CHOICE" "language: $LANG_CHOICE" bash -c "[[ \"\$('$BIN' config get language)\" == '$LANG_CHOICE' ]]"
if [[ -n "$KEYBOARD" ]]; then
    check 0 "clavier : $KB_LAYOUT ${KB_VARIANT:-}" "keyboard: $KB_LAYOUT ${KB_VARIANT:-}" \
        bash -c "grep -q 'kb_layout = \"$KB_LAYOUT\"' '$CONFIG_DIR/inputs.lua' && grep -qE '^[[:space:]]*require\\(\"inputs\"\\)' '$HYPR_CONF'"
fi
[[ "$REMOVE_HYPRPAPER" == 1 ]] && check 0 "hyprpaper retiré de l'autostart" "hyprpaper removed from autostart" \
    bash -c "! grep -qE '^[[:space:]]*hl\\.exec_cmd\\(.*hyprpaper' '$HYPR_CONF'"
if [[ "$GREETER" == yes ]] || greeter_ours; then
    check 0 "écran de connexion de hyprdesk (greetd)" "hyprdesk login screen (greetd)" greeter_ours
fi
DOCTOR="$("$BIN" doctor 2>/dev/null | grep -Eo '[0-9]+/[0-9]+' | tail -1)"
echo "  • $(t "dépendances facultatives : ${DOCTOR:-?} présentes (hyprdesk doctor)" "optional dependencies: ${DOCTOR:-?} present (hyprdesk doctor)")"
echo
echo "$(t "Raccourcis : SUPER + D (bureau), SUPER + SHIFT + D (réglages), SUPER + R (lanceur)." \
          "Shortcuts: SUPER + D (desktop), SUPER + SHIFT + D (settings), SUPER + R (launcher).")"
echo "$(t "Lignes facultatives pour hyprland.lua : hyprdesk snippet" "Optional hyprland.lua lines: hyprdesk snippet")"
echo "$(t "Ensuite : fermez la session et reconnectez-vous (ou redémarrez)." \
          "Next: log out and back in (or reboot).")"
if [[ "$FAILED" == 1 ]]; then
    echo "!! $(t "un point essentiel a échoué : journalctl --user -u hyprdesk -n 50" \
                 "an essential check failed: journalctl --user -u hyprdesk -n 50")" >&2
    exit 1
fi
