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
            case "$pm" in
                pacman) hyprdesk_root pacman -Syu --needed --noconfirm hyprland ;;
                apt-get) hyprdesk_root apt-get update -qq && hyprdesk_root env DEBIAN_FRONTEND=noninteractive apt-get install -y -qq hyprland >/dev/null ;;
                dnf) hyprdesk_root dnf install -y -q hyprland && hyprdesk_root dnf upgrade -y -q hyprland ;;
                zypper) hyprdesk_root zypper --non-interactive --quiet install hyprland ;;
            esac || true
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
