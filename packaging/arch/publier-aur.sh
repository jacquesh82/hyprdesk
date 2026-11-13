#!/usr/bin/env bash
# Publie le PKGBUILD sur l'AUR (https://aur.archlinux.org/packages/hyprdesk).
#
#   packaging/arch/publier-aur.sh          prépare le commit AUR, sans l'envoyer
#   packaging/arch/publier-aur.sh --push   puis l'envoie
#
# Il faut : une clé SSH déclarée sur le compte AUR (My Account → SSH Public
# Key), le tag v$pkgver poussé sur GitHub, et le sha256 renseigné (job release
# de ci.yml) : un PKGBUILD à 'SKIP' est refusé ici.
set -euo pipefail
ICI="$(cd "$(dirname "$0")" && pwd)"
PUSH=0
[[ "${1:-}" == --push ]] && PUSH=1
DEST="${AUR_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/hyprdesk-aur}"

cd "$ICI"
# shellcheck disable=SC1091
source <(grep -E '^(pkgname|pkgver|pkgrel)=' PKGBUILD)
grep -q "^sha256sums=('SKIP')" PKGBUILD && {
    echo "sha256sums vaut 'SKIP' : poussez le tag v$pkgver, laissez le job release le renseigner, puis git pull." >&2
    exit 1
}
curl -fsIL -o /dev/null "https://github.com/jacquesh82/hyprdesk/archive/refs/tags/v$pkgver.tar.gz" || {
    echo "tag v$pkgver introuvable sur GitHub : poussez-le d'abord (git push origin v$pkgver)." >&2
    exit 1
}
makepkg --printsrcinfo > .SRCINFO
# Contrôle : le tarball du tag correspond au sha256 du PKGBUILD.
TMP_SRC="$(mktemp -d)"
trap 'rm -rf "$TMP_SRC"' EXIT
SRCDEST="$TMP_SRC" makepkg --verifysource -f >/dev/null

if [[ -d "$DEST/.git" ]]; then
    git -C "$DEST" pull -q --ff-only
else
    git clone -q ssh://aur@aur.archlinux.org/hyprdesk.git "$DEST"
fi
cp PKGBUILD hyprdesk.install .SRCINFO "$DEST/"
cd "$DEST"
git add PKGBUILD hyprdesk.install .SRCINFO
if git diff --cached --quiet; then
    echo "AUR déjà à jour ($pkgver-$pkgrel)."
    exit 0
fi
git commit -q -m "hyprdesk $pkgver-$pkgrel"
git --no-pager show --stat HEAD
if [[ "$PUSH" == 1 ]]; then
    git push -q origin HEAD:master
    echo "publié : https://aur.archlinux.org/packages/hyprdesk"
else
    echo "commit prêt dans $DEST ; relancez avec --push pour l'envoyer."
fi
