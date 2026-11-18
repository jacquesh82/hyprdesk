# shellcheck shell=bash
# Le paquet précompilé d'une release de hyprdesk (construit par le job
# packages de .github/workflows/ci.yml sur un tag v*) et sa vérification.
# Sourcé par scripts/install.sh et scripts/update.sh.
#
#   release_version           → « 0.1.7 », la version de ce dépôt (Cargo.toml) :
#                               celle qu'on installe, pas la dernière publiée —
#                               les scripts relus et le paquet vont ensemble
#   release_latest            → la dernière release publiée (information)
#   release_pkg_url <v> <pm>  → URL du paquet pour ce gestionnaire (vide : rien
#                               pour cette architecture ou distribution)
#   release_verify <v> <fichier>
#                             → vérifie le paquet téléchargé (MLG-151) :
#                               empreinte dans SHA256SUMS de la release,
#                               toujours ; signature de SHA256SUMS (cosign,
#                               certificat du workflow ci.yml sur le tag v<v>)
#                               quand cosign est installé, obligatoire avec
#                               HYPRDESK_REQUIRE_SIGNATURE=1. Code 1 : refusé
#                               (à appeler AVANT toute élévation).
#   release_pkg_install <fichier> <pm> → installe le paquet vérifié
#
# Les paquets sont x86_64 (amd64) et aarch64 (arm64), comme scripts/package.sh.

RELEASE_REPO=jacquesh82/hyprdesk

release_version() {
    sed -n 's/^version = "\([0-9A-Za-z.+-]*\)"$/\1/p' "${HYPRDESK_ROOT:-.}/Cargo.toml" | head -1
}

release_latest() {
    curl -fsSL --proto '=https' --max-time 20 "https://api.github.com/repos/$RELEASE_REPO/releases/latest" \
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
    curl -fsSL --proto '=https' --max-time 30 "https://api.github.com/repos/$RELEASE_REPO/releases/tags/v${version}" 2>/dev/null \
        | grep -o '"browser_download_url": *"[^"]*"' | cut -d'"' -f4 \
        | grep -E "^https://github\\.com/$RELEASE_REPO/releases/download/v${version}/" \
        | grep -E "$pat" | head -1
}

# Message d'un refus, sur la sortie d'erreur (hd_run le garde au journal).
release_refuse() { echo "paquet refusé : $*" >&2; return 1; }

release_verify() {
    local version="$1" file="$2" dir name base want got
    dir="$(dirname "$file")"
    name="$(basename "$file")"
    base="https://github.com/$RELEASE_REPO/releases/download/v${version}"
    curl -fsSL --proto '=https' --max-time 60 -o "$dir/SHA256SUMS" "$base/SHA256SUMS" \
        || release_refuse "SHA256SUMS absent de la release v$version (release antérieure à la vérification ? --from-source compile le dépôt)" || return 1
    # Une ligne « <empreinte>  <nom> » exactement, pour ce fichier.
    want="$(awk -v n="$name" '$2 == n || $2 == "*" n { print $1 }' "$dir/SHA256SUMS" | head -1)"
    [[ "$want" =~ ^[0-9a-f]{64}$ ]] || release_refuse "$name absent de SHA256SUMS" || return 1
    got="$(sha256sum "$file" | awk '{print $1}')"
    [[ "$got" == "$want" ]] || release_refuse "empreinte de $name : $got, attendue $want" || return 1
    echo "empreinte vérifiée : $name ($got)"
    if command -v cosign >/dev/null 2>&1; then
        curl -fsSL --proto '=https' --max-time 60 -o "$dir/SHA256SUMS.sigstore.json" "$base/SHA256SUMS.sigstore.json" \
            || release_refuse "signature SHA256SUMS.sigstore.json absente de la release v$version" || return 1
        cosign verify-blob \
            --bundle "$dir/SHA256SUMS.sigstore.json" \
            --certificate-identity "https://github.com/$RELEASE_REPO/.github/workflows/ci.yml@refs/tags/v${version}" \
            --certificate-oidc-issuer https://token.actions.githubusercontent.com \
            "$dir/SHA256SUMS" >&2 \
            || release_refuse "signature de SHA256SUMS invalide (ni ce dépôt, ni ce tag)" || return 1
        echo "signature vérifiée : SHA256SUMS signé par le workflow ci.yml du tag v$version"
    elif [[ "${HYPRDESK_REQUIRE_SIGNATURE:-0}" == 1 ]]; then
        release_refuse "HYPRDESK_REQUIRE_SIGNATURE=1 et cosign absent : la signature ne peut pas être vérifiée"
        return 1
    else
        echo "signature non vérifiée : cosign absent (empreinte seule ; HYPRDESK_REQUIRE_SIGNATURE=1 l'exige)"
    fi
}

release_pkg_install() {
    local file="$1" pm="$2"
    case "$pm" in
        pacman) hyprdesk_root pacman -U --noconfirm "$file" ;;
        apt-get) hyprdesk_root apt-get install -y -q "$file" ;;
        dnf) hyprdesk_root dnf install -y "$file" ;;
        # Paquet rpm non signé (pas de clé de signature nfpm) : zypper le
        # refuserait. Appelé seulement après release_verify, qui a contrôlé
        # l'empreinte (et la signature de SHA256SUMS si cosign est là).
        zypper) hyprdesk_root zypper --non-interactive install --allow-unsigned-rpm "$file" ;;
        *) return 1 ;;
    esac
}
