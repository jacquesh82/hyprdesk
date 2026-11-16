#!/usr/bin/env bash
# shellcheck disable=SC2034,SC2005,SC2153
# (SC2034 : HYPRDESK_ROOT et HD_STEP_TOTAL lus par deps.sh et ui.sh sourcés ;
#  SC2153 : LYRA_REF et MYMFA_REF viennent de versions.sh sourcé ;
#  SC2005 : t() écrit par printf %s sans saut de ligne — le echo l'ajoute.)
# Mise à jour de hyprdesk : relève ce qui a changé, puis l'applique.
#
#   ./scripts/update.sh            relevé, puis mise à jour après confirmation
#   ./scripts/update.sh --check    relevé seulement (code 10 : mise à jour disponible)
#   ./scripts/update.sh --yes      sans question
#   --verbose                      sortie complète des commandes
#
# Relevé :
#   - hyprdesk : la release de la version du dépôt distant (Cargo.toml de la
#     branche suivie, lu comme donnée ; paquet précompilé et vérifié, installé
#     par install.sh) comparée à la version installée ; le dépôt cloné est
#     tiré au passage (scripts, ressources, traductions). Installé depuis le dépôt
#     en l'état (install_nightly_build.sh) ? la release ne sera pas plus
#     récente que votre binaire : mettez à jour avec le même script ;
#   - Hyprbars (hyprpm) : recompilé quand la version de Hyprland change ;
#   - LYRA et myMFA, s'ils ont été installés par hyprdesk : leurs sources
#     (~/.local/share/hyprdesk/src) qui ne sont pas à la release attendue par
#     la version de hyprdesk tirée (scripts/versions.sh).
# Application : git pull --ff-only ; install.sh avec les options de la
# dernière installation (install-options) ; LYRA et myMFA recompilés s'ils
# ont changé ; copie du binaire de l'écran de connexion rafraîchie ; bilan.
# Un dépôt modifié localement n'est pas tiré (rien n'est écrasé).
# Rien du dépôt distant n'est exécuté avant la confirmation (le relevé, et
# --check, le lisent comme données). HYPRDESK_REQUIRE_SIGNATURE=1 : le commit
# tiré doit porter une signature valide (git verify-commit), sinon rien n'est
# tiré ni lancé.
set -Eeuo pipefail
cd "$(dirname "$0")/.."

CHECK=0
YES=0
while [ $# -gt 0 ]; do
    case "$1" in
        --check) CHECK=1 ;;
        --yes|-y) YES=1 ;;
        --verbose) export HYPRDESK_VERBOSE=1 ;;
        -h|--help) sed -n '2,19p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "argument inconnu / unknown argument : $1 (--help)" >&2; exit 2 ;;
    esac
    shift
done

UI=fr
[[ "${LANG:-}" == fr* ]] || UI=en
export UI
HYPRDESK_ROOT="$PWD"
# shellcheck source=deps.sh disable=SC1091
source scripts/deps.sh
t() { hyprdesk_t "$@"; }

STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/hyprdesk"
SRC_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/hyprdesk/src"
LYRA_SRC="$SRC_DIR/mindlog.hyprland_tools"
MYMFA_SRC="$SRC_DIR/mindlog.mymfa.clients"
BIN_DIR="$HOME/.local/bin"
hd_log_init

hd_banner "hyprdesk · $(t "mise à jour" "update")" \
    "$(t "Relevé de ce qui a changé, puis mise à jour." "What changed, then the update.")" \
    "$(t "Journal complet : $HD_LOG" "Full log: $HD_LOG")"

if ! git rev-parse --git-dir >/dev/null 2>&1; then
    hd_die_line "$(t "pas un dépôt git : la mise à jour suit le dépôt cloné (git clone …)" "not a git repository: updates follow the cloned repository (git clone …)")"
    exit 1
fi

# -- Relevé : hyprdesk --------------------------------------------------------------------
HD_STEP_TOTAL=0
hd_step "🔎" "Ce qui a changé" "What changed"
PENDING=()   # « libellé » des mises à jour à faire
UP="$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null || echo origin/main)"
if ! hd_run "$(t "dépôt de hyprdesk" "hyprdesk repository") ($UP)" git fetch -q "${UP%%/*}"; then
    hd_warn "$(t "dépôt distant injoignable : relevé local seulement" "remote unreachable: local check only")"
fi
HEAD_C="$(git rev-parse HEAD)"
UP_C="$(git rev-parse "$UP" 2>/dev/null || echo "$HEAD_C")"
BEHIND="$(git rev-list --count "HEAD..$UP_C" 2>/dev/null || echo 0)"
AHEAD="$(git rev-list --count "$UP_C..HEAD" 2>/dev/null || echo 0)"
DIRTY="$(git status --porcelain --untracked-files=no)"

# La release publiée pilote la mise à jour de hyprdesk : son paquet
# précompilé est installé par install.sh (scripts/release.sh).
# shellcheck source=release.sh disable=SC1091
source scripts/release.sh
# La version qu'installera install.sh après le git pull : celle du
# Cargo.toml distant (lu comme donnée, validé), sinon du dépôt local.
REL="$(git show "$UP_C:Cargo.toml" 2>/dev/null | sed -n 's/^version = "\([0-9A-Za-z.+-]*\)"$/\1/p' | head -1)"
[[ -n "$REL" ]] || REL="$(release_version)"
# Binaire de la session : compilé (~/.local/bin) ou du paquet (/usr/bin).
INSTALLED_BIN="$BIN_DIR/hyprdesk"
[[ -x "$INSTALLED_BIN" ]] || INSTALLED_BIN=/usr/bin/hyprdesk
INSTALLED_V="$("$INSTALLED_BIN" --version 2>/dev/null | sed -n 's/^hyprdesk //p')"
if [[ -z "$INSTALLED_V" ]]; then
    hd_warn "$(t "version installée inconnue : installation proposée" "installed version unknown: install offered")"
    PENDING+=("hyprdesk")
elif [[ "$REL" != "$INSTALLED_V" ]]; then
    hd_info "$(t "hyprdesk : release v$REL disponible (installée : v$INSTALLED_V)" \
                 "hyprdesk: release v$REL available (installed: v$INSTALLED_V)")"
    PENDING+=("hyprdesk")
else
    hd_ok "$(t "hyprdesk à jour" "hyprdesk up to date") (v$INSTALLED_V)"
fi
if ((BEHIND > 0)); then
    hd_info "$(t "dépôt : $BEHIND commit(s) à tirer de $UP (scripts et ressources)" \
                 "repository: $BEHIND commit(s) to pull from $UP (scripts and assets)")"
fi
((AHEAD > 0)) && hd_note "$(t "$AHEAD commit(s) locaux pas encore poussés : gardés" "$AHEAD local commit(s) not pushed yet: kept")"
if [[ -n "$DIRTY" ]] && ((BEHIND > 0)); then
    hd_warn "$(t "modifications locales non commitées : le dépôt ne sera pas tiré" "uncommitted local changes: the repository will not be pulled")"
    hd_hint "$(t "les commiter ou les mettre de côté (git stash), puis relancer" "commit them or set them aside (git stash), then run again")"
fi

# -- Relevé : LYRA et myMFA (sources gardées par install.sh) -----------------------------
# Leurs releases : celles de la version de hyprdesk qui sera installée
# (scripts/versions.sh du dépôt distant quand il sera tiré), sinon de la
# version locale. HYPRDESK_LYRA_REF et HYPRDESK_MYMFA_REF passent outre.
# shellcheck source=versions.sh disable=SC1091
source scripts/versions.sh
if ((BEHIND > 0)) && [[ -z "$DIRTY" ]]; then
    # versions.sh distant lu comme données (jamais sourcé, MLG-151) : les
    # valeurs reconnues et validées seulement, les variables HYPRDESK_* de
    # l'environnement passent toujours outre.
    UP_VERSIONS="$(git show "$UP_C:scripts/versions.sh" 2>/dev/null || true)"
    v="$(hyprdesk_versions_get "$UP_VERSIONS" LYRA_REF)";     [[ -n "$v" && -z "${HYPRDESK_LYRA_REF:-}" ]] && LYRA_REF="$v"
    v="$(hyprdesk_versions_get "$UP_VERSIONS" LYRA_COMMIT)";  [[ "$v" =~ ^[0-9a-f]{40}$ && -z "${HYPRDESK_LYRA_REF:-}${HYPRDESK_LYRA_COMMIT:-}" ]] && LYRA_COMMIT="$v"
    v="$(hyprdesk_versions_get "$UP_VERSIONS" MYMFA_REF)";    [[ -n "$v" && -z "${HYPRDESK_MYMFA_REF:-}" ]] && MYMFA_REF="$v"
    v="$(hyprdesk_versions_get "$UP_VERSIONS" MYMFA_COMMIT)"; [[ "$v" =~ ^[0-9a-f]{40}$ && -z "${HYPRDESK_MYMFA_REF:-}${HYPRDESK_MYMFA_COMMIT:-}" ]] && MYMFA_COMMIT="$v"
    unset v UP_VERSIONS
fi
# Le commit d'un tag annoté (FETCH_HEAD désigne l'objet tag) ou d'une branche.
fetched() { git -C "$1" rev-parse 'FETCH_HEAD^{commit}'; }
LYRA_NEW=""
# Commit récupéré conforme à celui épinglé par versions.sh (vide : référence
# passée à la main, non vérifiée) ; sinon la mise à jour est refusée.
pinned() { [[ -z "$2" || "$(fetched "$1")" == "$2" ]]; }
if [[ -d "$LYRA_SRC/.git" ]]; then
    # Le tag de versions.sh seulement : plus de repli sur main (MLG-154).
    REF="$LYRA_REF"
    if git -C "$LYRA_SRC" fetch -q --depth 1 origin "$REF" >/dev/null 2>&1; then
        if ! pinned "$LYRA_SRC" "$LYRA_COMMIT"; then
            hd_ko "LYRA : $(t "le tag $REF désigne $(fetched "$LYRA_SRC"), attendu $LYRA_COMMIT : mise à jour refusée" "tag $REF points to $(fetched "$LYRA_SRC"), expected $LYRA_COMMIT: update refused")"
        elif [[ "$(git -C "$LYRA_SRC" rev-parse HEAD)" != "$(fetched "$LYRA_SRC")" ]]; then
            LYRA_NEW="$(git -C "$LYRA_SRC" log -1 --format='%h %s' FETCH_HEAD)"
            hd_info "LYRA : $(t "nouvelle version" "new version") ($REF) — $LYRA_NEW"
            PENDING+=("LYRA")
        else
            hd_ok "LYRA $(t "à jour" "up to date")"
        fi
    else
        hd_warn "LYRA : $(t "dépôt injoignable" "repository unreachable")"
    fi
elif command -v hyprpanel-chat >/dev/null 2>&1; then
    hd_note "LYRA : $(t "installée hors de hyprdesk, non suivie ici" "installed outside hyprdesk, not tracked here")"
fi
MYMFA_NEW=""
if [[ -d "$MYMFA_SRC/.git" ]]; then
    if GIT_SSH_COMMAND="ssh -o BatchMode=yes -o ConnectTimeout=8" \
        git -C "$MYMFA_SRC" fetch -q --depth 1 origin "$MYMFA_REF" >/dev/null 2>&1; then
        if ! pinned "$MYMFA_SRC" "$MYMFA_COMMIT"; then
            hd_ko "myMFA : $(t "le tag $MYMFA_REF désigne $(fetched "$MYMFA_SRC"), attendu $MYMFA_COMMIT : mise à jour refusée" "tag $MYMFA_REF points to $(fetched "$MYMFA_SRC"), expected $MYMFA_COMMIT: update refused")"
        elif [[ "$(git -C "$MYMFA_SRC" rev-parse HEAD)" != "$(fetched "$MYMFA_SRC")" ]]; then
            MYMFA_NEW="$(git -C "$MYMFA_SRC" log -1 --format='%h %s' FETCH_HEAD)"
            hd_info "myMFA : $(t "nouvelle version" "new version") ($MYMFA_REF) — $MYMFA_NEW"
            PENDING+=("myMFA")
        else
            hd_ok "myMFA $(t "à jour" "up to date")"
        fi
    else
        hd_warn "myMFA : $(t "dépôt injoignable" "repository unreachable")"
    fi
elif command -v mymfa >/dev/null 2>&1; then
    hd_note "myMFA : $(t "installé hors de hyprdesk, non suivi ici" "installed outside hyprdesk, not tracked here")"
fi

# Hyprbars compilé par hyprpm : à refaire quand la version de Hyprland qui
# tourne n'est plus celle de sa compilation (sinon Hyprland refuse le plugin).
HYPRBARS_NEW=""
if [[ -d "/var/cache/hyprpm/$USER/hyprland-plugins" ]] && command -v hyprpm >/dev/null 2>&1; then
    HB_RUN="$(hyprdesk_hyprland_running || true)"
    HB_BUILT="$(cat "$STATE_DIR/hyprbars-hyprland" 2>/dev/null || true)"
    if [[ -n "$HB_RUN" && "$HB_RUN" != "$HB_BUILT" ]]; then
        HYPRBARS_NEW="$HB_RUN"
        hd_info "Hyprbars : $(t "à recompiler pour Hyprland" "to rebuild for Hyprland") $HB_RUN${HB_BUILT:+ ($(t "compilé pour" "built for") $HB_BUILT)}"
        PENDING+=("Hyprbars")
    else
        hd_ok "Hyprbars $(t "à jour" "up to date")"
    fi
fi

# Écran de connexion de hyprdesk installé depuis une compilation : il lance
# sa propre copie root du binaire (/usr/local/bin). Avec le paquet, greetd
# lance /usr/bin/hyprdesk, mis à jour par le paquet lui-même.
greeter_stale() {
    [[ -x /usr/local/bin/hyprdesk ]] && [[ -x "$BIN_DIR/hyprdesk" ]] \
        && "$BIN_DIR/hyprdesk" greeter setup status 2>/dev/null | grep -q hyprdesk \
        && ! cmp -s /usr/local/bin/hyprdesk "$BIN_DIR/hyprdesk"
}
if greeter_stale; then
    hd_info "$(t "écran de connexion : sa copie de hyprdesk est en retard" "login screen: its copy of hyprdesk is behind")"
    PENDING+=("$(t "écran-de-connexion" "login-screen")")
fi

if ((${#PENDING[@]} == 0)); then
    hd_panel "$C_GREEN" "$C_GREEN$C_BOLD$I_OK $(t "Tout est à jour." "Everything is up to date.")$C_RESET"
    echo
    exit 0
fi
if ((CHECK)); then
    hd_panel "$C_YELLOW" "$C_YELLOW$C_BOLD$I_WARN $(t "Mise à jour disponible :" "Update available:") ${PENDING[*]}$C_RESET" \
        "$(t "pour l'appliquer :" "to apply it:") $C_BOLD./scripts/update.sh$C_RESET"
    echo
    exit 10
fi
if ((!YES)); then
    if [[ ! -t 0 ]]; then
        hd_die_line "$(t "sans terminal : relancer avec --yes pour appliquer" "no terminal: run again with --yes to apply")"
        exit 2
    fi
    printf '\n  %s?%s ' "$C_ACCENT$C_BOLD" "$C_RESET"
    read -r -p "$(t "Mettre à jour (${PENDING[*]}) ? [O/n] " "Update (${PENDING[*]})? [Y/n] ")" a
    [[ -z "$a" || "$a" =~ ^[oOyY] ]] || { hd_info "$(t "rien n'a été modifié" "nothing was changed")"; exit 0; }
fi

# -- Application -----------------------------------------------------------------------------
DONE=()
if ((BEHIND > 0)) && [[ -z "$DIRTY" ]] && [[ "${HYPRDESK_REQUIRE_SIGNATURE:-0}" == 1 ]]; then
    # Rien n'est tiré ni lancé si le commit distant n'est pas signé par une
    # clé de confiance (gpg.ssh.allowedSignersFile ou trousseau GnuPG).
    if ! git verify-commit "$UP_C" >>"$HD_LOG" 2>&1; then
        hd_die_line "$(t "commit $UP_C non signé ou signature non reconnue (HYPRDESK_REQUIRE_SIGNATURE=1) : rien n'est tiré" "commit $UP_C unsigned or signature not trusted (HYPRDESK_REQUIRE_SIGNATURE=1): nothing pulled")"
        exit 1
    fi
    hd_ok "$(t "signature du commit $UP_C vérifiée" "commit $UP_C signature verified")"
fi
if ((BEHIND > 0)) && [[ -z "$DIRTY" ]]; then
    hd_step "⬇" "Récupération" "Pull"
    hd_run "git pull --ff-only" git pull -q --ff-only "${UP%%/*}" "${UP#*/}"
fi
if [[ " ${PENDING[*]} " == *" hyprdesk "* ]]; then
    # Les options de la dernière installation : mêmes choix, sans repasser
    # par l'écran de connexion (déjà en place ou refusé) ni les questions.
    OPTS=()
    if [[ -f "$STATE_DIR/install-options" ]]; then
        while IFS= read -r o; do
            [[ -z "$o" || "$o" == --greeter ]] && continue
            OPTS+=("$o")
        done < "$STATE_DIR/install-options"
    fi
    [[ " ${OPTS[*]:-} " == *" --no-greeter "* ]] || OPTS+=(--no-greeter)
    [[ " ${OPTS[*]:-} " == *" --optional-deps "* ]] || OPTS+=(--optional-deps none)
    hd_note "scripts/install.sh ${OPTS[*]}"
    # Le script relancé : sa version tirée à l'instant.
    if scripts/install.sh "${OPTS[@]}"; then
        DONE+=("hyprdesk $(git rev-parse --short HEAD)")
    else
        hd_ko "$(t "installation incomplète : voir le bilan ci-dessus" "install incomplete: see the report above")"
    fi
fi
HD_STEP_TOTAL=0
if [[ -n "$LYRA_NEW" ]]; then
    hd_step "🤖" "LYRA" "LYRA"
    if hd_run "$(t "sources" "sources")" git -C "$LYRA_SRC" checkout -q FETCH_HEAD \
        && hd_run "$(t "compilation de LYRA" "building LYRA")" \
            cargo build --release --locked --manifest-path "$LYRA_SRC/Cargo.toml" -p hyprpanel-chat -p hyprpanel-server \
        && install -m755 "$LYRA_SRC/target/release/hyprpanel-chat" "$LYRA_SRC/target/release/hyprpanel" "$BIN_DIR/"; then
        systemctl --user try-restart hyprpanel.service 2>/dev/null || true
        hd_ok "LYRA : $LYRA_NEW"
        DONE+=("LYRA")
    else
        hd_warn "$(t "LYRA non mise à jour (voir le journal)" "LYRA not updated (see the log)")"
    fi
fi
if [[ -n "$MYMFA_NEW" ]]; then
    hd_step "🔑" "myMFA" "myMFA"
    if hd_run "$(t "sources" "sources")" git -C "$MYMFA_SRC" checkout -q FETCH_HEAD \
        && hd_run "$(t "compilation et installation de myMFA" "building and installing myMFA")" \
            bash "$MYMFA_SRC/linux/scripts/installer.sh"; then
        hd_ok "myMFA : $MYMFA_NEW"
        DONE+=("myMFA")
        PAM_LIB="$MYMFA_SRC/linux/target/release/libpam_mymfa.so"
        for d in /usr/lib/security /usr/lib64/security "/usr/lib/$(uname -m)-linux-gnu/security" "/lib/$(uname -m)-linux-gnu/security"; do
            if [[ -f "$d/pam_mymfa.so" ]] && ! cmp -s "$PAM_LIB" "$d/pam_mymfa.so"; then
                # « || true » : l'échec de la mise à jour du module n'arrête pas
                # la suite ; l'ancien module reste en place.
                # shellcheck disable=SC2015
                hd_sudo_ready "$(t "mettre à jour le module PAM de myMFA" "update the myMFA PAM module")" \
                    && hd_run "$(t "module PAM" "PAM module")" hyprdesk_root install -D -o root -g root -m0755 "$PAM_LIB" "$d/pam_mymfa.so" || true
            fi
        done
    else
        hd_warn "$(t "myMFA non mis à jour (voir le journal)" "myMFA not updated (see the log)")"
    fi
fi
if [[ -n "$HYPRBARS_NEW" ]]; then
    hd_step "🪟" "Hyprbars" "Hyprbars"
    # Questions de hyprpm posées à l'utilisateur (plus de « yes | », MLG-154) :
    # au premier plan, entrée reliée au terminal ; sans terminal, sautée.
    hb_update() {
        hd_info "$(t "en-têtes et plugins (hyprpm update)" "headers and plugins (hyprpm update)")"
        printf '\n### %s — hyprpm update\n' "$(date +%T)" >>"$HD_LOG"
        hyprpm update > >(tee -a "$HD_LOG") 2>&1
    }
    if [[ ! -t 0 ]]; then
        hd_warn "$(t "sans terminal : hyprpm pose des questions, Hyprbars non recompilé (hyprpm update au terminal)" "no terminal: hyprpm asks questions, Hyprbars not rebuilt (hyprpm update in a terminal)")"
    elif hd_sudo_ready "$(t "recompiler les plugins de Hyprland (hyprpm)" "rebuild the Hyprland plugins (hyprpm)")" \
        && hb_update; then
        printf '%s\n' "$HYPRBARS_NEW" > "$STATE_DIR/hyprbars-hyprland"
        hd_ok "Hyprbars : Hyprland $HYPRBARS_NEW"
        DONE+=("Hyprbars")
    else
        hd_warn "$(t "Hyprbars non recompilé (voir le journal) : hyprpm update" "Hyprbars not rebuilt (see the log): hyprpm update")"
    fi
fi
# Écran de connexion : copie du binaire rafraîchie (après la réinstallation).
if greeter_stale; then
    hd_step "🚪" "Écran de connexion" "Login screen"
    # Le binaire compilé est d'abord installé en root, puis c'est cette
    # copie root qui est lancée : jamais ~/.local/bin en root (MLG-152).
    if hd_sudo_ready "$(t "mettre à jour l'écran de connexion" "update the login screen")" \
        && hd_run "$(t "copie root du binaire" "root copy of the binary") (/usr/local/bin/hyprdesk)" hyprdesk_root_install_build "$BIN_DIR/hyprdesk" \
        && hd_run "$(t "profil de l'écran de connexion" "login screen profile")" hyprdesk_root /usr/local/bin/hyprdesk greeter setup install; then
        DONE+=("$(t "écran de connexion" "login screen")")
    else
        hd_warn "$(t "écran de connexion non mis à jour : sudo hyprdesk greeter setup install" "login screen not updated: sudo hyprdesk greeter setup install")"
    fi
fi

if ((${#DONE[@]} > 0)); then
    # sed (plutôt que ${var//…}) : joint par la virgule IFS puis « , » -> « ,  ».
    # shellcheck disable=SC2001
    hd_panel "$C_GREEN" "$C_GREEN$C_BOLD$I_OK $(t "Mis à jour :" "Updated:") $(IFS=,; echo "${DONE[*]}" | sed 's/,/, /g')$C_RESET"
else
    hd_panel "$C_YELLOW" "$C_YELLOW$C_BOLD$I_WARN $(t "Rien n'a pu être mis à jour : voir ci-dessus" "Nothing could be updated: see above")$C_RESET"
fi
echo
