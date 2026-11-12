#!/usr/bin/env bash
# Habillage des scripts d'installation : couleurs, icônes, étapes numérotées,
# attente animée des commandes longues, erreurs et pistes de correction.
# Sourcé par deps.sh, install.sh et installer.sh.
#
#   HYPRDESK_COLOR=0|1   sans / avec couleurs (sinon : selon le terminal et NO_COLOR)
#   HYPRDESK_ASCII=1     icônes ASCII (terminal sans UTF-8 ni emoji)
#   HYPRDESK_VERBOSE=1   sortie complète des commandes longues (install.sh --verbose)
#
# Les sorties complètes des commandes longues vont dans un journal ($HD_LOG,
# ~/.local/state/hyprdesk/install-<date>.log) ; l'écran n'en montre que la
# dernière ligne pendant l'attente, et les dernières lignes en cas d'échec.

[[ -n "${HD_UI_LOADED:-}" ]] && return 0
HD_UI_LOADED=1

# Messages : français ou anglais ($UI, sinon LANG).
hyprdesk_t() {
    local ui="${UI:-}"
    [[ -n "$ui" ]] || { [[ "${LANG:-}" == fr* ]] && ui=fr || ui=en; }
    if [[ "$ui" == en ]]; then printf '%s' "$2"; else printf '%s' "$1"; fi
}

# -- Couleurs et icônes -----------------------------------------------------------

# hd_colors on|off|auto
hd_colors() {
    local mode="${1:-auto}"
    if [[ "$mode" == auto ]]; then
        if [[ -n "${HYPRDESK_COLOR:-}" ]]; then
            [[ "$HYPRDESK_COLOR" == 1 ]] && mode=on || mode=off
        elif [[ -n "${NO_COLOR:-}" || "${TERM:-dumb}" == dumb ]] || ! [[ -t 1 ]]; then
            mode=off
        else
            mode=on
        fi
    fi
    if [[ "$mode" == on ]]; then
        local deep=0
        [[ "$(tput colors 2>/dev/null || echo 8)" -ge 256 ]] && deep=1
        C_RESET=$'\e[0m' C_BOLD=$'\e[1m' C_DIM=$'\e[2m' C_ITAL=$'\e[3m'
        if [[ "$deep" == 1 ]]; then
            C_RED=$'\e[38;5;203m' C_GREEN=$'\e[38;5;114m' C_YELLOW=$'\e[38;5;221m'
            C_BLUE=$'\e[38;5;111m' C_CYAN=$'\e[38;5;117m' C_ACCENT=$'\e[38;5;141m'
            C_GREY=$'\e[38;5;245m'
        else
            C_RED=$'\e[91m' C_GREEN=$'\e[92m' C_YELLOW=$'\e[93m'
            C_BLUE=$'\e[94m' C_CYAN=$'\e[96m' C_ACCENT=$'\e[95m' C_GREY=$'\e[90m'
        fi
        HD_COLOR_ON=1
    else
        C_RESET='' C_BOLD='' C_DIM='' C_ITAL='' C_RED='' C_GREEN='' C_YELLOW=''
        C_BLUE='' C_CYAN='' C_ACCENT='' C_GREY=''
        HD_COLOR_ON=0
    fi
}

hd_icons() {
    HD_ASCII="${HYPRDESK_ASCII:-}"
    if [[ -z "$HD_ASCII" ]]; then
        HD_ASCII=1
        # UTF-8 annoncé par la locale, même non générée (LANG=fr_FR.UTF-8 sans
        # fr_FR dans locale -a : charmap répond ANSI, le terminal reste UTF-8).
        if [[ "$(locale charmap 2>/dev/null)" == UTF-8 ]] \
            || [[ "${LC_ALL:-${LC_CTYPE:-${LANG:-}}}" =~ [Uu][Tt][Ff]-?8 ]]; then
            HD_ASCII=0
        fi
    fi
    if [[ "$HD_ASCII" == 1 ]]; then
        I_OK='[ok]' I_KO='[x]' I_WARN='[!]' I_INFO='[i]' I_HINT='->' I_DOT='*'
        I_STEP='>>' I_BAR='|' I_FULL='#' I_EMPTY='.' HD_SPIN=('|' '/' '-' '\')
    else
        I_OK='✔' I_KO='✖' I_WARN='⚠' I_INFO='ℹ' I_HINT='➜' I_DOT='•'
        # Animation en tableau : sans locale UTF-8 générée, bash découperait
        # une chaîne octet par octet.
        I_STEP='▶' I_BAR='┃' I_FULL='■' I_EMPTY='□' HD_SPIN=(⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏)
    fi
}

# hd_icon <emoji> <ascii> : l'icône d'une étape.
hd_icon() { [[ "$HD_ASCII" == 1 ]] && printf '%s' "$2" || printf '%s' "$1"; }

# hd_plain : sans couleurs (texte capturé pour whiptail, journal).
hd_plain() { hd_colors off; }

hd_colors auto
hd_icons

hd_cols() {
    local c
    c="$(tput cols 2>/dev/null || echo 80)"
    [[ "$c" =~ ^[0-9]+$ ]] || c=80
    ((c > 80)) && c=80
    ((c < 40)) && c=40
    echo "$c"
}

# Longueur en caractères, quelle que soit la locale (sans locale UTF-8
# générée, ${#x} compte des octets) : octets moins octets de continuation.
hd_strlen() {
    local b c
    b="$(printf '%s' "$1" | LC_ALL=C wc -c)"
    c="$(printf '%s' "$1" | LC_ALL=C tr -cd '\200-\277' | LC_ALL=C wc -c)"
    echo $((b - c))
}

hd_rep() { local s="" i; for ((i = 0; i < $2; i++)); do s+="$1"; done; printf '%s' "$s"; }

# -- Messages -----------------------------------------------------------------------

hd_ok()   { printf '  %s%s%s %s\n' "$C_GREEN" "$I_OK" "$C_RESET" "$*"; }
hd_ko()   { printf '  %s%s%s %s%s%s\n' "$C_RED" "$I_KO" "$C_RESET" "$C_BOLD" "$*" "$C_RESET"; }
hd_warn() { printf '  %s%s%s %s\n' "$C_YELLOW" "$I_WARN" "$C_RESET" "$*"; }
hd_info() { printf '  %s%s%s %s\n' "$C_CYAN" "$I_INFO" "$C_RESET" "$*"; }
hd_note() { printf '    %s%s%s\n' "$C_GREY" "$*" "$C_RESET"; }
# Piste de correction, sous un ✖ ou un ⚠.
hd_hint() { printf '      %s%s%s %s\n' "$C_YELLOW" "$I_HINT" "$C_RESET" "$*"; }
# Commande à copier.
hd_cmd()  { printf '      %s$%s %s%s%s\n' "$C_GREY" "$C_RESET" "$C_BOLD" "$*" "$C_RESET"; }
# Message d'erreur sur la sortie d'erreur.
hd_die_line() { printf '%s%s %s%s\n' "$C_RED$C_BOLD" "$I_KO" "$*" "$C_RESET" >&2; }

# hd_banner <titre> [sous-titre…] : bandeau d'ouverture.
hd_banner() {
    local w title="$1"; shift
    w=$(($(hd_cols) - 4))
    echo
    printf '  %s%s%s\n' "$C_ACCENT" "$(hd_rep '━' "$w")" "$C_RESET"
    printf '  %s%s  %s%s\n' "$C_ACCENT$C_BOLD" "$(hd_icon '◆' '*')" "$title" "$C_RESET"
    local l
    for l in "$@"; do printf '     %s%s%s\n' "$C_GREY" "$l" "$C_RESET"; done
    printf '  %s%s%s\n' "$C_ACCENT" "$(hd_rep '━' "$w")" "$C_RESET"
}

# hd_progress <n> <total> : ■■■□□□
hd_progress() {
    local n="$1" total="$2" width=12 full
    ((total > 0)) || return 0
    full=$((n * width / total))
    printf '%s%s%s%s%s' "$C_ACCENT" "$(hd_rep "$I_FULL" "$full")" "$C_GREY" "$(hd_rep "$I_EMPTY" $((width - full)))" "$C_RESET"
}

# hd_step <emoji> <titre fr> <titre en> [<détail fr> <détail en>]
# Étape numérotée si HD_STEP_TOTAL est posé ; HD_HINT (piste de correction en
# cas d'échec) est remis à zéro, à reposer juste après si besoin.
HD_STEP_N=0
HD_STEP_TOTAL="${HD_STEP_TOTAL:-0}"
HD_STEP_NAME=""
HD_HINT=""
hd_step() {
    local icon="$1" title counter=""
    title="$(hyprdesk_t "$2" "$3")"
    HD_STEP_N=$((HD_STEP_N + 1))
    HD_STEP_NAME="$title"
    HD_HINT=""
    if ((HD_STEP_TOTAL > 0)); then
        counter="$(printf '%2d/%d' "$HD_STEP_N" "$HD_STEP_TOTAL")"
        printf '\n%s%s %s%s %s  %s%s %s%s\n' "$C_ACCENT$C_BOLD" "$I_STEP" "$counter" "$C_RESET" \
            "$(hd_progress "$HD_STEP_N" "$HD_STEP_TOTAL")" "$C_BOLD" "$(hd_icon "$icon" '')" "$title" "$C_RESET"
    else
        printf '\n%s%s %s%s %s%s\n' "$C_ACCENT$C_BOLD" "$I_STEP" "$C_RESET$C_BOLD" "$(hd_icon "$icon" '')" "$title" "$C_RESET"
    fi
    if [[ -n "${4:-}" ]]; then
        printf '   %s%s%s\n' "$C_GREY$C_ITAL" "$(hyprdesk_t "$4" "$5")" "$C_RESET"
    fi
}

# hd_section <emoji> <titre fr> <titre en> : titre sans numéro (bilan…).
hd_section() {
    local w
    w=$(($(hd_cols) - 4))
    printf '\n  %s%s%s\n' "$C_GREY" "$(hd_rep '─' "$w")" "$C_RESET"
    printf '  %s%s %s%s\n' "$C_BOLD" "$(hd_icon "$1" "$I_STEP")" "$(hyprdesk_t "$2" "$3")" "$C_RESET"
}

# hd_panel <couleur> <ligne…> : encadré à barre verticale (verdict, échec).
hd_panel() {
    local color="$1" l; shift
    echo
    for l in "$@"; do printf '  %s%s%s %s\n' "$color" "$I_BAR" "$C_RESET" "$l"; done
}

# -- Durées, journal, attente -------------------------------------------------------

hd_duration() {
    local s="$1"
    if ((s >= 60)); then printf '%d min %02d s' $((s / 60)) $((s % 60)); else printf '%d s' "$s"; fi
}

hd_log_init() {
    if [[ -z "${HD_LOG:-}" ]]; then
        local dir="${XDG_STATE_HOME:-$HOME/.local/state}/hyprdesk"
        mkdir -p "$dir"
        HD_LOG="$dir/install-$(date +%Y%m%d-%H%M%S).log"
    fi
    touch "$HD_LOG"
    export HD_LOG
}

# hd_log_tail_line <largeur> [<octet de départ>] : dernière ligne lisible du
# journal écrite depuis ce point (sans couleurs ni retours chariot).
hd_log_tail_line() {
    tail -c "+$((${2:-0} + 1))" "$HD_LOG" 2>/dev/null | tail -c 4000 | tr '\r' '\n' | sed 's/\x1b\[[0-9;?]*[A-Za-z]//g' \
        | grep -v '^[[:space:]]*$' | tail -1 | sed 's/^[[:space:]]*//' | cut -c "1-$1" || true
}

# hd_run <libellé> <commande…> : lance la commande, sortie dans le journal,
# attente animée avec la dernière ligne produite et le temps écoulé ; ✔ ou ✖
# avec les dernières lignes du journal. Rend le code de la commande.
hd_run() {
    local label="$1"; shift
    local start=$SECONDS rc=0
    hd_log_init
    printf '\n### %s — %s\n$ %s\n' "$(date +%T)" "$label" "$*" >>"$HD_LOG"
    local from
    from="$(stat -c %s "$HD_LOG" 2>/dev/null || echo 0)"
    if [[ "${HYPRDESK_VERBOSE:-0}" == 1 ]] || ! [[ -t 1 ]]; then
        hd_info "$label"
        "$@" > >(tee -a "$HD_LOG") 2>&1 </dev/null || rc=$?
    else
        # Sans le piège d'erreur hérité (set -E) : l'échec est rendu à l'appelant.
        (trap - ERR; "$@") >>"$HD_LOG" 2>&1 </dev/null &
        local pid=$! i=0 n="${#HD_SPIN[@]}" w last
        w=$(($(hd_cols) - ${#label} - 20))
        ((w < 10)) && w=10
        printf '\e[?25l'
        while kill -0 "$pid" 2>/dev/null; do
            last="$(hd_log_tail_line "$w" "$from")"
            printf '\r\e[K  %s%s%s %s %s%s%s %s%s%s' "$C_ACCENT" "${HD_SPIN[i % n]}" "$C_RESET" "$label" \
                "$C_GREY" "$(hd_duration $((SECONDS - start)))" "$C_RESET" "$C_DIM" "$last" "$C_RESET"
            i=$((i + 1))
            sleep 0.2
        done
        wait "$pid" || rc=$?
        printf '\r\e[K\e[?25h'
    fi
    local took=$((SECONDS - start)) dur=""
    ((took >= 3)) && dur=" $C_GREY($(hd_duration "$took"))$C_RESET"
    if [[ "$rc" == 0 ]]; then
        printf '  %s%s%s %s%s\n' "$C_GREEN" "$I_OK" "$C_RESET" "$label" "$dur"
    else
        printf '  %s%s%s %s%s%s%s\n' "$C_RED" "$I_KO" "$C_RESET" "$C_BOLD" "$label" "$C_RESET" "$dur"
        if [[ "${HYPRDESK_VERBOSE:-0}" != 1 ]] && [[ -t 1 ]]; then
            printf '    %s%s%s\n' "$C_GREY" "$(hyprdesk_t "dernières lignes :" "last lines:")" "$C_RESET"
            tail -c "+$((from + 1))" "$HD_LOG" | tail -n 15 | sed 's/\x1b\[[0-9;?]*[A-Za-z]//g' | while IFS= read -r l; do
                printf '    %s│%s %s\n' "$C_RED" "$C_RESET" "$l"
            done
        fi
        hd_note "$(hyprdesk_t "journal complet : $HD_LOG" "full log: $HD_LOG")"
    fi
    return "$rc"
}

# hd_sudo_ready <raison> : demande le mot de passe sudo avant une attente animée
# (sinon l'invite se mélangerait à l'animation). Vrai si root ou sudo accepté.
hd_sudo_ready() {
    [[ "$(id -u)" == 0 ]] && return 0
    command -v sudo >/dev/null 2>&1 || return 0
    sudo -n true 2>/dev/null && return 0
    printf '  %s%s%s %s\n' "$C_YELLOW" "$(hd_icon '🔑' '[sudo]')" "$C_RESET" \
        "$(hyprdesk_t "sudo : mot de passe demandé pour $1" "sudo: password needed to $1")"
    sudo -v
}
