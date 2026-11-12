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
        echo "!! sudo introuvable : lancer en root « $* »" >&2
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
        echo "== prérequis : non vérifiés (HYPRDESK_NO_DEPS=1)"
        return 0
    fi
    echo "== prérequis de construction"

    # -- Paquets système ------------------------------------------------------
    local pm missing
    pm="$(hyprdesk_pm)"
    if [ -z "$pm" ]; then
        echo "   gestionnaire de paquets non reconnu (pacman, apt, dnf, zypper) :"
        echo "   il faut un compilateur C, pkg-config, PAM (développement), Wayland,"
        echo "   xkbcommon, Vulkan, EGL, fontconfig et xdg-utils."
    else
        # shellcheck disable=SC2046
        missing="$(hyprdesk_missing "$pm" $(hyprdesk_packages "$pm") | tr '\n' ' ' | sed 's/ *$//')"
        if [ -z "$missing" ]; then
            echo "   paquets système : présents ($pm)"
        elif [ "$dry" = 1 ]; then
            echo "   à installer ($pm) : $missing"
        else
            echo "   installation ($pm) : $missing"
            # shellcheck disable=SC2086
            hyprdesk_install_packages "$pm" $missing || {
                echo "!! installation des paquets impossible : $missing" >&2
                return 1
            }
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
    echo "   Rust ${now:-absente} : $min ou plus est requise"
    if [ "$dry" = 1 ]; then
        if command -v rustup >/dev/null 2>&1; then
            echo "   rustup : installerait la toolchain stable et l'utiliserait"
        else
            echo "   installerait rustup dans ~/.cargo (sans sudo), toolchain stable"
        fi
        return 0
    fi
    if ! command -v rustup >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/rustup" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
    if ! command -v rustup >/dev/null 2>&1; then
        echo "   installation de rustup dans ~/.cargo (https://sh.rustup.rs)"
        # Rust de la distribution déjà là : rustup s'installe à côté, sans la retirer.
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
            | RUSTUP_INIT_SKIP_PATH_CHECK=yes sh -s -- -y -q --profile minimal --default-toolchain stable --no-modify-path >/dev/null || {
            echo "!! installation de rustup impossible" >&2
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
        echo "!! Rust ${now:-absente} après installation, $min requise" >&2
        return 1
    fi
    echo "   Rust $now (stable de rustup)"
    case ":$PATH:" in
        *":$HOME/.cargo/bin:"*) ;;
        *) echo "   (ajoutez ~/.cargo/bin à votre PATH pour les prochaines constructions)" ;;
    esac
}

# Lancé directement (et non sourcé) : vérifie et installe.
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    set -euo pipefail
    HYPRDESK_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
    hyprdesk_deps "${1:-}"
fi
