#!/usr/bin/env bash
# Paquets de hyprdesk : .deb (Debian, Ubuntu), .rpm (Fedora, openSUSE) et
# .pkg.tar.zst (Arch Linux), dans dist/.
#
#   scripts/package.sh                 les trois
#   scripts/package.sh deb rpm         seulement ceux-là
#   scripts/package.sh --release 2     itération du paquet (défaut : 1)
#   scripts/package.sh --no-build      réutilise target/release/hyprdesk
#   MAINTAINER="Nom <mail>" scripts/package.sh   (défaut : Jacques HULLU)
#
# Un seul outil, nfpm, produit les trois formats depuis packaging/nfpm.yaml.in :
# celui du PATH s'il est installé, sinon son image Docker (version épinglée).
#
# Le binaire est construit ici, donc lié à la glibc de cette machine : la
# version minimale requise est relevée dans ses symboles et inscrite dans les
# dépendances, pour que apt ou dnf refusent proprement un système trop ancien.
# Pour viser une distribution plus ancienne, lancer ce script dans un conteneur
# de cette distribution (avec Rust).
set -euo pipefail
cd "$(dirname "$0")/.."

NFPM_IMAGE="goreleaser/nfpm:v2.48.0"
FORMATS=()
RELEASE=1
BUILD=1
while [ $# -gt 0 ]; do
    case "$1" in
        deb|rpm) FORMATS+=("$1") ;;
        arch|archlinux|pacman) FORMATS+=(archlinux) ;;
        --release) RELEASE="${2:?--release attend un nombre}"; shift ;;
        --no-build) BUILD=0 ;;
        -h|--help) sed -n '2,18p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "argument inconnu : $1 (deb, rpm, arch, --release N, --no-build)" >&2; exit 2 ;;
    esac
    shift
done
[ ${#FORMATS[@]} -gt 0 ] || FORMATS=(deb rpm archlinux)
case "$RELEASE" in ''|*[!0-9]*) echo "--release : nombre entier attendu" >&2; exit 2 ;; esac

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
[ -n "$VERSION" ] || { echo "version introuvable dans Cargo.toml" >&2; exit 1; }
case "$(uname -m)" in
    x86_64) ARCH=amd64 ;;
    aarch64) ARCH=arm64 ;;
    *) echo "architecture non prise en charge : $(uname -m)" >&2; exit 1 ;;
esac
MAINTAINER="${MAINTAINER:-Jacques HULLU <jacquesh82@users.noreply.github.com>}"

if [ "$BUILD" = 1 ]; then
    # Prérequis manquants (paquets, Rust assez récente) : installés d'abord.
    # HYPRDESK_ROOT est lu par deps.sh, sourcé juste après.
    # shellcheck disable=SC2034
    HYPRDESK_ROOT="$PWD"
    # shellcheck source=deps.sh disable=SC1091
    source scripts/deps.sh
    hyprdesk_deps
    echo "== build release"
    cargo build --release --locked
fi
BIN="${CARGO_TARGET_DIR:-target}/release/hyprdesk"
[ -x "$BIN" ] || { echo "$BIN absent : lancer sans --no-build" >&2; exit 1; }

# Version minimale de la glibc : la plus haute version de symbole GLIBC_x.y requise.
GLIBC="$( (objdump -T "$BIN" 2>/dev/null || strings "$BIN") \
    | grep -o 'GLIBC_[0-9]\+\(\.[0-9]\+\)\+' | sed 's/GLIBC_//' | sort -uV | tail -1)"
[ -n "$GLIBC" ] || { echo "version de glibc requise introuvable dans $BIN" >&2; exit 1; }

DIST=dist
STAGE="$DIST/stage"
rm -rf "$STAGE"
mkdir -p "$STAGE/usr/bin" "$STAGE/usr/lib/systemd/user"
install -m755 "$BIN" "$STAGE/usr/bin/hyprdesk"
strip --strip-unneeded "$STAGE/usr/bin/hyprdesk" 2>/dev/null || true
sed -e 's#%h/\.local/bin/hyprdesk#/usr/bin/hyprdesk#' \
    -e 's#déposée par scripts/install.sh#installée par le paquet hyprdesk#' \
    -e 's#(qui précise WAYLAND_DISPLAY détecté)\.#(environnement de session : systemctl --user import-environment).#' \
    assets/hyprdesk.service > "$STAGE/usr/lib/systemd/user/hyprdesk.service"
grep -q '^ExecStart=/usr/bin/hyprdesk ' "$STAGE/usr/lib/systemd/user/hyprdesk.service" \
    || { echo "unit systemd : ExecStart non réécrit" >&2; exit 1; }

sed -e "s#@VERSION@#$VERSION#" -e "s#@RELEASE@#$RELEASE#" -e "s#@ARCH@#$ARCH#" \
    -e "s#@GLIBC@#$GLIBC#g" -e "s#@STAGE@#$STAGE#g" -e "s#@MAINTAINER@#$MAINTAINER#" \
    packaging/nfpm.yaml.in > "$DIST/nfpm.yaml"
if grep -v '^\s*#' "$DIST/nfpm.yaml" | grep -q '@[A-Z]\+@'; then
    echo "marqueur non remplacé dans $DIST/nfpm.yaml" >&2
    exit 1
fi

nfpm_run() {
    if command -v nfpm >/dev/null 2>&1; then
        nfpm "$@"
    elif command -v docker >/dev/null 2>&1; then
        docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/src" -w /src "$NFPM_IMAGE" "$@"
    else
        echo "nfpm introuvable : l'installer (https://nfpm.goreleaser.com) ou installer Docker" >&2
        exit 1
    fi
}

echo "== hyprdesk $VERSION-$RELEASE ($ARCH, glibc >= $GLIBC)"
for f in "${FORMATS[@]}"; do
    nfpm_run package --config "$DIST/nfpm.yaml" --packager "$f" --target "$DIST/"
done
rm -rf "$STAGE"
ls -l "$DIST"/hyprdesk[-_]* 2>/dev/null || true
