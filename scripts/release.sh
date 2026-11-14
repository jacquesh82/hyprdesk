# shellcheck shell=bash
# La dernière release publiée de hyprdesk et son paquet précompilé
# (construit par le job release de .github/workflows/ci.yml sur un tag v*).
# Sourcé par scripts/install.sh et scripts/update.sh.
#
#   release_latest            → « 0.1.1 » (vide : injoignable, pas de release)
#   release_pkg_url <v> <pm>  → URL du paquet pour ce gestionnaire (vide : rien
#                               pour cette architecture ou distribution)
#   release_pkg_install <fichier> <pm> → installe le paquet téléchargé
#
# Les paquets sont x86_64 (amd64) et aarch64 (arm64), comme scripts/package.sh.

release_latest() {
    curl -fsSL --max-time 20 "https://api.github.com/repos/jacquesh82/hyprdesk/releases/latest" \
        | sed -n 's/^.*"tag_name": *"v\([^"]*\)".*/\1/p' | head -1
}

release_arch() {
    case "$(uname -m)" in
        x86_64) echo x86_64 ;;
        aarch64) echo aarch64 ;;
    esac
}

release_pkg_url() {
    local version="$1" pm="$2" arch debarch pat
    arch="$(release_arch)" || return 1
    case "$arch" in
        x86_64) debarch=amd64 ;;
        aarch64) debarch=arm64 ;;
        *) return 1 ;;
    esac
    case "$pm" in
        pacman) pat="hyprdesk-${version}-[0-9]+-${arch}\\.pkg\\.tar\\.zst$" ;;
        apt-get) pat="hyprdesk_${version}-[0-9]+_${debarch}\\.deb$" ;;
        dnf) pat="hyprdesk-${version}-[0-9]+\\.${arch}\\.rpm$" ;;
        zypper) pat="hyprdesk-${version}-[0-9]+\\.${arch}\\.rpm$" ;;
        *) return 1 ;;
    esac
    curl -fsSL --max-time 30 "https://api.github.com/repos/jacquesh82/hyprdesk/releases/tags/v${version}" 2>/dev/null \
        | grep -o '"browser_download_url": *"[^"]*"' | cut -d'"' -f4 \
        | grep -E "$pat" | head -1
}

release_pkg_install() {
    local file="$1" pm="$2"
    case "$pm" in
        pacman) hyprdesk_root pacman -U --noconfirm "$file" ;;
        apt-get) hyprdesk_root apt-get install -y -q "$file" ;;
        dnf) hyprdesk_root dnf install -y "$file" ;;
        zypper) hyprdesk_root zypper --non-interactive install --allow-unsigned-rpm "$file" ;;
        *) return 1 ;;
    esac
}
