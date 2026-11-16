#!/usr/bin/env bash
# Cohérence du paquet AUR (MLG-151) : packaging/arch/PKGBUILD et .SRCINFO
# décrivent la même version que Cargo.toml, la même archive (tag v$pkgver) et
# la même empreinte.
#
#   scripts/verifier-arch.sh                 contrôle (code 1 si incohérent)
#   scripts/verifier-arch.sh --corriger      réécrit dans .SRCINFO les lignes
#                                            pkgver, pkgrel, source et sha256sums
#                                            depuis le PKGBUILD
#   scripts/verifier-arch.sh --tag vX.Y.Z    contrôle aussi que le tag est la
#                                            version de Cargo.toml (job release)
#
# makepkg --printsrcinfo (packaging/arch/publier-aur.sh) reste la référence au
# moment de publier sur l'AUR ; ce script tient .SRCINFO juste entre-temps,
# sans makepkg (absent des runners Ubuntu de la CI).
set -euo pipefail
cd "$(dirname "$0")/.."

FIX=0
TAG=""
while [ $# -gt 0 ]; do
    case "$1" in
        --corriger) FIX=1 ;;
        --tag) TAG="${2:?--tag vX.Y.Z}"; shift ;;
        *) echo "argument inconnu : $1 (--corriger, --tag vX.Y.Z)" >&2; exit 2 ;;
    esac
    shift
done

PKGBUILD=packaging/arch/PKGBUILD
SRCINFO=packaging/arch/.SRCINFO
cargo_v="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
pkgver="$(sed -n 's/^pkgver=\(.*\)$/\1/p' "$PKGBUILD")"
pkgrel="$(sed -n 's/^pkgrel=\(.*\)$/\1/p' "$PKGBUILD")"
sha="$(sed -n "s/^sha256sums=('\([0-9a-f]\{64\}\|SKIP\)')$/\1/p" "$PKGBUILD")"
source_line="hyprdesk-$pkgver.tar.gz::https://github.com/jacquesh82/hyprdesk/archive/refs/tags/v$pkgver.tar.gz"

if [[ "$FIX" == 1 ]]; then
    [[ -n "$pkgver" && -n "$pkgrel" && -n "$sha" ]] || { echo "PKGBUILD : pkgver, pkgrel ou sha256sums illisible" >&2; exit 1; }
    sed -i -e "s|^\tpkgver = .*|\tpkgver = $pkgver|" \
           -e "s|^\tpkgrel = .*|\tpkgrel = $pkgrel|" \
           -e "s|^\tsource = .*|\tsource = $source_line|" \
           -e "s|^\tsha256sums = .*|\tsha256sums = $sha|" "$SRCINFO"
fi

rc=0
fail() { echo "incohérence : $*" >&2; rc=1; }
[[ "$pkgver" == "$cargo_v" ]] || fail "PKGBUILD pkgver=$pkgver, Cargo.toml version=$cargo_v"
[[ -n "$sha" ]] || fail "PKGBUILD : sha256sums absent ou mal formé"
grep -qxF "	pkgver = $pkgver" "$SRCINFO" || fail ".SRCINFO : pkgver différent de $pkgver"
grep -qxF "	pkgrel = $pkgrel" "$SRCINFO" || fail ".SRCINFO : pkgrel différent de $pkgrel"
grep -qxF "	source = $source_line" "$SRCINFO" || fail ".SRCINFO : source ne vise pas l'archive v$pkgver"
grep -qxF "	sha256sums = $sha" "$SRCINFO" || fail ".SRCINFO : sha256sums différent du PKGBUILD"
if [[ -n "$TAG" ]]; then
    [[ "$TAG" == "v$cargo_v" ]] || fail "tag $TAG, Cargo.toml version $cargo_v"
fi
[[ "$rc" == 0 ]] && echo "arch : PKGBUILD et .SRCINFO cohérents (hyprdesk $pkgver-$pkgrel)"
exit "$rc"
