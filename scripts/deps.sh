#!/usr/bin/env bash
# Prérequis de construction de hyprdesk : installe ce qui manque au lieu de
# s'arrêter sur une erreur de compilation.
#
#   scripts/deps.sh              vérifie et installe ce qui manque
#   scripts/deps.sh --dry-run    montre seulement ce qui serait fait
#   source scripts/deps.sh       (install.sh, package.sh) puis : hyprdesk_deps
#
# 1. Paquets système, selon la distribution (pacman, apt, dnf, zypper) :
#    compilateur C (Lua embarqué), pkg-config, PAM de développement (libpam.so
#    est lié à la compilation), xkbcommon et Wayland de développement (cherchés
#    par pkg-config à la compilation), et les bibliothèques chargées à l'exécution
#    (Wayland, xkbcommon, Vulkan, EGL, fontconfig, xdg-utils). Seuls les
#    paquets absents sont installés, avec sudo.
# 2. Rust : au moins la `rust-version` de Cargo.toml. Une toolchain trop
#    ancienne (celle de la distribution, ou rustup sur une vieille version)
#    n'est pas remplacée : rustup installe la dernière stable dans le dossier
#    personnel (~/.cargo, ~/.rustup), sans sudo, et la construction l'utilise.
#
# HYPRDESK_NO_DEPS=1 (ou --no-deps pour install.sh) saute cette étape.

# Messages : français ou anglais (UI de install.sh, sinon LANG).
hyprdesk_t() {
    local ui="${UI:-}"
    [[ -n "$ui" ]] || { [[ "${LANG:-}" == fr* ]] && ui=fr || ui=en; }
    if [[ "$ui" == en ]]; then printf '%s' "$2"; else printf '%s' "$1"; fi
}

# Version minimale de Rust : celle de Cargo.toml.
hyprdesk_rust_min() {
    sed -n 's/^rust-version = "\(.*\)"/\1/p' "${HYPRDESK_ROOT:-.}/Cargo.toml" | head -1
}

# Vrai si la version $1 est au moins $2.
hyprdesk_version_ge() {
    [ "$(printf '%s\n%s\n' "$2" "$1" | sort -V | head -1)" = "$2" ]
}

# Version de rustc utilisée par cargo (vide si aucune).
hyprdesk_rust_now() {
    rustc --version 2>/dev/null | awk '{print $2}' | sed 's/-.*//'
}

hyprdesk_pm() {
    for pm in pacman apt-get dnf zypper; do
        command -v "$pm" >/dev/null 2>&1 && { echo "$pm"; return; }
    done
}

# Paquets requis par gestionnaire.
hyprdesk_packages() {
    case "$1" in
        pacman) echo "base-devel pkgconf pam wayland libxkbcommon vulkan-icd-loader libglvnd fontconfig xdg-utils curl" ;;
        apt-get) echo "build-essential pkg-config libpam0g-dev libxkbcommon-dev libwayland-dev libwayland-client0 libwayland-egl1 libxkbcommon0 libvulkan1 libegl1 fontconfig xdg-utils curl ca-certificates" ;;
        dnf) echo "gcc make pkgconf-pkg-config pam-devel libxkbcommon-devel wayland-devel libwayland-client libwayland-egl libxkbcommon vulkan-loader libglvnd-egl fontconfig xdg-utils curl" ;;
        zypper) echo "gcc make pkg-config pam-devel libxkbcommon-devel wayland-devel libwayland-client0 libwayland-egl1 libxkbcommon0 libvulkan1 libEGL1 fontconfig xdg-utils curl" ;;
    esac
}

# Paquets absents parmi la liste.
hyprdesk_missing() {
    local pm="$1"; shift
    local p
    case "$pm" in
        pacman) pacman -T "$@" 2>/dev/null || true ;;
        apt-get)
            for p in "$@"; do
                dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q "install ok installed" || echo "$p"
            done ;;
        dnf|zypper)
            for p in "$@"; do
                rpm -q --whatprovides "$p" >/dev/null 2>&1 || echo "$p"
            done ;;
    esac
}

# Exécute en root : directement, ou par sudo.
hyprdesk_root() {
    if [ "$(id -u)" = 0 ]; then
        "$@"
    elif command -v sudo >/dev/null 2>&1; then
        sudo "$@"
    else
        echo "!! $(hyprdesk_t "sudo introuvable : lancer en root « $* »" "sudo not found: run as root “$*”")" >&2
        return 1
    fi
}

hyprdesk_install_packages() {
    local pm="$1"; shift
    case "$pm" in
        pacman) hyprdesk_root pacman -S --needed --noconfirm "$@" ;;
        apt-get) hyprdesk_root apt-get update -qq && hyprdesk_root env DEBIAN_FRONTEND=noninteractive apt-get install -y -qq "$@" >/dev/null ;;
        dnf) hyprdesk_root dnf install -y -q "$@" ;;
        zypper) hyprdesk_root zypper --non-interactive --quiet install "$@" ;;
    esac
}

# Version de Hyprland installée (binaire) et de la session en cours.
hyprdesk_hyprland_installed() {
    command -v Hyprland >/dev/null 2>&1 \
        && Hyprland --version 2>/dev/null | head -1 | awk '{print $2}' | sed 's/^v//'
}
hyprdesk_hyprland_running() {
    [ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ] && command -v hyprctl >/dev/null 2>&1 \
        && hyprctl -j version 2>/dev/null | sed -n 's/.*"tag": *"v\{0,1\}\([0-9.]*\).*/\1/p' | head -1
}

# Installe ou met à jour Hyprland par la distribution (Arch : mise à jour
# complète, pacman n'acceptant pas de mise à jour partielle).
hyprdesk_upgrade_hyprland() {
    case "${1:-$(hyprdesk_pm)}" in
        pacman) hyprdesk_root pacman -Syu --needed --noconfirm hyprland ;;
        apt-get) hyprdesk_root apt-get update -qq && hyprdesk_root env DEBIAN_FRONTEND=noninteractive apt-get install -y -qq hyprland >/dev/null ;;
        dnf) hyprdesk_root dnf install -y -q hyprland && hyprdesk_root dnf upgrade -y -q hyprland ;;
        zypper) hyprdesk_root zypper --non-interactive --quiet install hyprland ;;
        *) return 1 ;;
    esac
}

# Contrôles préalables, AVANT toute modification : critères de base. Un échec
# arrête l'installation avec la marche à suivre ; code 0 si tout va bien,
# 1 sinon, 3 si seul Hyprland est trop ancien (l'assistant propose la mise à jour).
hyprdesk_preflight() {
    local fails=0 only_hypr=1 hmin="${HYPRDESK_HYPRLAND_MIN:-0.57}" v run free
    ok() { echo "  ✓ $1"; }
    ko() { echo "  ✗ $1"; [ -n "${2:-}" ] && echo "      → $2"; fails=$((fails + 1)); [ "${3:-}" = hypr ] || only_hypr=0; }
    warn() { echo "  • $1"; }
    echo "== $(hyprdesk_t "contrôles préalables" "preflight checks")"
    if [ "$(id -u)" = 0 ]; then
        ko "$(hyprdesk_t "lancé en root" "running as root")" \
           "$(hyprdesk_t "lancer en utilisateur : sudo sera demandé quand il le faut" "run as your user: sudo is asked when needed")"
    else
        ok "$(hyprdesk_t "utilisateur" "user"): $(id -un)"
    fi
    case "$(uname -m)" in
        x86_64|aarch64) ok "$(hyprdesk_t "architecture" "architecture"): $(uname -m)" ;;
        *) ko "$(hyprdesk_t "architecture non prise en charge" "unsupported architecture"): $(uname -m)" ;;
    esac
    if systemctl --user show-environment >/dev/null 2>&1; then
        ok "$(hyprdesk_t "systemd utilisateur" "systemd user manager")"
    else
        ko "$(hyprdesk_t "systemd utilisateur injoignable" "systemd user manager unreachable")" \
           "$(hyprdesk_t "lancer depuis une session ouverte (pas su/sudo -i)" "run from a logged-in session (not su/sudo -i)")"
    fi
    v="$(hyprdesk_hyprland_installed || true)"
    if [ -z "$v" ]; then
        ko "$(hyprdesk_t "Hyprland absent" "Hyprland not installed")" \
           "$(hyprdesk_t "installer Hyprland $hmin ou plus (paquet hyprland)" "install Hyprland $hmin or newer (hyprland package)")" hypr
    elif ! hyprdesk_version_ge "$v" "$hmin"; then
        ko "$(hyprdesk_t "Hyprland $v installé : $hmin ou plus requis" "Hyprland $v installed: $hmin or newer required")" \
           "$(hyprdesk_t "mettre à jour (Arch : sudo pacman -Syu), se reconnecter, relancer l'installation" "update it (Arch: sudo pacman -Syu), log out and back in, run the install again")" hypr
    else
        ok "Hyprland $v (≥ $hmin)"
    fi
    run="$(hyprdesk_hyprland_running || true)"
    if [ -z "$run" ]; then
        warn "$(hyprdesk_t "hors d'une session Hyprland : hyprdesk démarrera à la prochaine connexion" "outside a Hyprland session: hyprdesk starts at next login")"
    elif ! hyprdesk_version_ge "$run" "$hmin"; then
        ko "$(hyprdesk_t "session en Hyprland $run : $hmin ou plus requis" "session runs Hyprland $run: $hmin or newer required")" \
           "$(hyprdesk_t "se reconnecter pour passer à la version installée, puis relancer" "log out and back in to run the installed version, then run again")" hypr
    else
        ok "$(hyprdesk_t "session Hyprland" "Hyprland session") $run"
    fi
    if [ "${HYPRDESK_NO_DEPS:-0}" != 1 ] && [ -z "$(hyprdesk_pm)" ]; then
        ko "$(hyprdesk_t "gestionnaire de paquets non reconnu (pacman, apt, dnf, zypper)" "unknown package manager (pacman, apt, dnf, zypper)")" \
           "$(hyprdesk_t "installer les prérequis à la main, puis relancer avec --no-deps" "install the prerequisites by hand, then run with --no-deps")"
    fi
    free="$(df -Pk "${HYPRDESK_ROOT:-.}" 2>/dev/null | awk 'NR == 2 {print int($4 / 1048576)}')"
    if [ -n "$free" ] && [ "$free" -lt 4 ]; then
        ko "$(hyprdesk_t "espace libre : $free Gio (4 Gio requis pour la compilation)" "free space: $free GiB (4 GiB needed to build)")"
    else
        ok "$(hyprdesk_t "espace libre" "free space"): ${free:-?} $(hyprdesk_t "Gio" "GiB")"
    fi
    if command -v curl >/dev/null 2>&1; then
        if curl -sfSo /dev/null --max-time 8 https://index.crates.io/config.json \
            && curl -sfSo /dev/null --max-time 8 https://github.com; then
            ok "$(hyprdesk_t "réseau (crates.io, GitHub)" "network (crates.io, GitHub)")"
        else
            ko "$(hyprdesk_t "crates.io ou GitHub injoignable" "crates.io or GitHub unreachable")" \
               "$(hyprdesk_t "la compilation télécharge ses dépendances : vérifier la connexion" "the build downloads its dependencies: check the connection")"
        fi
    fi
    if [ "$fails" -gt 0 ]; then
        echo "!! $(hyprdesk_t "installation arrêtée avant toute modification : $fails critère(s) non rempli(s)" "install stopped before changing anything: $fails check(s) failed")" >&2
        [ "$only_hypr" = 1 ] && return 3
        return 1
    fi
    return 0
}

hyprdesk_deps() {
    local dry=0
    [ "${1:-}" = "--dry-run" ] && dry=1
    if [ "${HYPRDESK_NO_DEPS:-0}" = 1 ]; then
        echo "== $(hyprdesk_t "prérequis : non vérifiés (HYPRDESK_NO_DEPS=1)" "prerequisites: not checked (HYPRDESK_NO_DEPS=1)")"
        return 0
    fi
    echo "== $(hyprdesk_t "prérequis de construction" "build prerequisites")"

    # -- Paquets système ------------------------------------------------------
    local pm missing
    pm="$(hyprdesk_pm)"
    if [ -z "$pm" ]; then
        echo "   $(hyprdesk_t "gestionnaire de paquets non reconnu (pacman, apt, dnf, zypper) :" "unknown package manager (pacman, apt, dnf, zypper):")"
        echo "   $(hyprdesk_t "il faut un compilateur C, pkg-config, PAM (développement), Wayland," "needed: a C compiler, pkg-config, PAM (development), Wayland,")"
        echo "   $(hyprdesk_t "xkbcommon, Vulkan, EGL, fontconfig et xdg-utils." "xkbcommon, Vulkan, EGL, fontconfig and xdg-utils.")"
    else
        # shellcheck disable=SC2046
        missing="$(hyprdesk_missing "$pm" $(hyprdesk_packages "$pm") | tr '\n' ' ' | sed 's/ *$//')"
        if [ -z "$missing" ]; then
            echo "   $(hyprdesk_t "paquets système : présents ($pm)" "system packages: present ($pm)")"
        elif [ "$dry" = 1 ]; then
            echo "   $(hyprdesk_t "à installer ($pm) : $missing" "to install ($pm): $missing")"
        else
            echo "   $(hyprdesk_t "installation ($pm) : $missing" "installing ($pm): $missing")"
            # shellcheck disable=SC2086
            hyprdesk_install_packages "$pm" $missing || {
                echo "!! $(hyprdesk_t "installation des paquets impossible : $missing" "could not install packages: $missing")" >&2
                return 1
            }
        fi
    fi

    # -- Hyprland -------------------------------------------------------------------
    # 0.57 au moins (API Lua) : en 0.56, hyprdesk le ferait planter en posant ses
    # raccourcis. Mis à jour par la distribution ; la session en cours garde
    # l'ancien jusqu'à la reconnexion.
    local hmin="${HYPRDESK_HYPRLAND_MIN:-0.57}" hnow=""
    command -v Hyprland >/dev/null 2>&1 \
        && hnow="$(Hyprland --version 2>/dev/null | head -1 | awk '{print $2}' | sed 's/^v//')"
    if [ -n "$hnow" ] && hyprdesk_version_ge "$hnow" "$hmin"; then
        echo "   Hyprland $hnow (≥ $hmin)"
    elif [ -n "$pm" ]; then
        echo "   $(hyprdesk_t "Hyprland ${hnow:-absent} : $hmin ou plus est requis" "Hyprland ${hnow:-missing}: $hmin or newer is required")"
        if [ "$dry" = 1 ]; then
            case "$pm" in
                pacman) echo "   $(hyprdesk_t "mettrait le système à jour (pacman -Syu hyprland : Arch n'accepte pas de mise à jour partielle)" "would update the system (pacman -Syu hyprland: Arch does not support partial upgrades)")" ;;
                *) echo "   $(hyprdesk_t "mettrait hyprland à jour ($pm)" "would update hyprland ($pm)")" ;;
            esac
        else
            hyprdesk_upgrade_hyprland "$pm" || true
            hnow="$(Hyprland --version 2>/dev/null | head -1 | awk '{print $2}' | sed 's/^v//')"
            if [ -n "$hnow" ] && hyprdesk_version_ge "$hnow" "$hmin"; then
                echo "   $(hyprdesk_t "Hyprland $hnow installé : reconnectez-vous pour l'utiliser" "Hyprland $hnow installed: log out and back in to use it")"
            else
                echo "!! $(hyprdesk_t "Hyprland ${hnow:-absent} : la distribution ne fournit pas encore $hmin ; hyprdesk refusera de démarrer" "Hyprland ${hnow:-missing}: the distribution does not ship $hmin yet; hyprdesk will refuse to start")" >&2
            fi
        fi
    fi

    # -- Rust ---------------------------------------------------------------------
    local min now
    min="$(hyprdesk_rust_min)"
    now="$(hyprdesk_rust_now)"
    if [ -n "$now" ] && hyprdesk_version_ge "$now" "$min"; then
        echo "   Rust $now (≥ $min)"
        return 0
    fi
    echo "   $(hyprdesk_t "Rust ${now:-absente} : $min ou plus est requise" "Rust ${now:-missing}: $min or newer is required")"
    if [ "$dry" = 1 ]; then
        if command -v rustup >/dev/null 2>&1; then
            echo "   $(hyprdesk_t "rustup : installerait la toolchain stable et l'utiliserait" "rustup: would install and use the stable toolchain")"
        else
            echo "   $(hyprdesk_t "installerait rustup dans ~/.cargo (sans sudo), toolchain stable" "would install rustup in ~/.cargo (no sudo), stable toolchain")"
        fi
        return 0
    fi
    if ! command -v rustup >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/rustup" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
    if ! command -v rustup >/dev/null 2>&1; then
        echo "   $(hyprdesk_t "installation de rustup dans ~/.cargo (https://sh.rustup.rs)" "installing rustup in ~/.cargo (https://sh.rustup.rs)")"
        # Rust de la distribution déjà là : rustup s'installe à côté, sans la retirer.
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
            | RUSTUP_INIT_SKIP_PATH_CHECK=yes sh -s -- -y -q --profile minimal --default-toolchain stable --no-modify-path >/dev/null || {
            echo "!! $(hyprdesk_t "installation de rustup impossible" "could not install rustup")" >&2
            return 1
        }
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        rustup -q toolchain install stable --profile minimal || return 1
    fi
    # Cette construction utilise la stable de rustup, même si la toolchain par
    # défaut, ou la Rust de la distribution (/usr/bin), reste plus ancienne.
    export PATH="$(dirname "$(command -v rustup)"):$PATH"
    export RUSTUP_TOOLCHAIN=stable
    now="$(hyprdesk_rust_now)"
    if [ -z "$now" ] || ! hyprdesk_version_ge "$now" "$min"; then
        echo "!! $(hyprdesk_t "Rust ${now:-absente} après installation, $min requise" "Rust ${now:-missing} after installing, $min required")" >&2
        return 1
    fi
    echo "   $(hyprdesk_t "Rust $now (stable de rustup)" "Rust $now (rustup stable)")"
    case ":$PATH:" in
        *":$HOME/.cargo/bin:"*) ;;
        *) echo "   $(hyprdesk_t "(ajoutez ~/.cargo/bin à votre PATH pour les prochaines constructions)" "(add ~/.cargo/bin to your PATH for future builds)")" ;;
    esac
}

# Lancé directement (et non sourcé) : vérifie et installe.
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    set -euo pipefail
    HYPRDESK_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
    hyprdesk_deps "${1:-}"
fi
