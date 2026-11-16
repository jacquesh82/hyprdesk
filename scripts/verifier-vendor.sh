#!/usr/bin/env bash
# Vérifie que les copies modifiées de vendor/ (layershellev, iced_layershell)
# sont exactement l'archive crates.io d'origine plus vendor/<crate>/patches/ :
#
#   1. l'archive <crate>-<version>.crate est téléchargée depuis crates.io et
#      son empreinte comparée à celle consignée dans vendor/<crate>/HYPRDESK.md ;
#   2. les correctifs y sont appliqués dans un dossier temporaire ;
#   3. le résultat est comparé à la copie du dépôt : une modification de
#      vendor/ non reportée dans patches/ (ou l'inverse) fait échouer.
#
#   scripts/verifier-vendor.sh                 les deux copies
#   scripts/verifier-vendor.sh --regenerer     réécrit patches/hyprdesk.patch
#                                              depuis la copie (après une
#                                              modification voulue de vendor/)
#
# Lancé par la CI (job « vendor », MLG-151).
set -euo pipefail
cd "$(dirname "$0")/.."

REGEN=0
[[ "${1:-}" == --regenerer ]] && REGEN=1
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
rc=0
for dir in vendor/*/; do
    dir="${dir%/}"
    crate="$(basename "$dir")"
    [[ -f "$dir/HYPRDESK.md" ]] || continue
    version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$dir/Cargo.toml" | head -1)"
    # shellcheck disable=SC2016 # accents graves de Markdown, pas une substitution
    expected="$(sed -n 's/^- Empreinte (sha256) de l.archive : `\([0-9a-f]\{64\}\)`.*/\1/p' "$dir/HYPRDESK.md" | head -1)"
    if [[ -z "$version" || -z "$expected" ]]; then
        echo "$crate : version ou empreinte d'origine absente de $dir/HYPRDESK.md" >&2
        rc=1
        continue
    fi
    curl -fsSL --proto '=https' --max-time 60 -o "$TMP/$crate.crate" \
        "https://static.crates.io/crates/$crate/$crate-$version.crate"
    got="$(sha256sum "$TMP/$crate.crate" | awk '{print $1}')"
    if [[ "$got" != "$expected" ]]; then
        echo "$crate $version : empreinte de l'archive $got, attendue $expected" >&2
        rc=1
        continue
    fi
    tar xzf "$TMP/$crate.crate" -C "$TMP"
    orig="$TMP/$crate-$version"
    rm -f "$orig/Cargo.lock" "$orig/.cargo_vcs_info.json" "$orig/.cargo-ok"
    # La copie du dépôt, sans ce que hyprdesk y ajoute pour la traçabilité.
    mod="$TMP/mod-$crate"
    cp -r "$dir" "$mod"
    rm -rf "$mod/patches" "$mod/HYPRDESK.md" "$mod/LICENSE"
    if [[ "$REGEN" == 1 ]]; then
        mkdir -p "$dir/patches"
        (cd "$TMP" && diff -ruN "$crate-$version" "mod-$crate" || true) \
            | grep -v '^diff -ruN' \
            | sed -E -e "s#^--- $crate-$version/([^	]*)	.*#--- a/\\1#" \
                     -e "s#^\\+\\+\\+ mod-$crate/([^	]*)	.*#+++ b/\\1#" \
            > "$dir/patches/hyprdesk.patch"
        echo "$crate : $dir/patches/hyprdesk.patch réécrit"
        continue
    fi
    for p in "$dir"/patches/*.patch; do
        [[ -f "$p" ]] || continue
        patch -s -p1 -d "$orig" < "$p"
    done
    if diff -r "$orig" "$mod" >"$TMP/$crate.diff"; then
        echo "$crate $version : archive vérifiée, copie = archive + correctifs"
    else
        echo "$crate $version : la copie de $dir s'écarte de l'archive + correctifs :" >&2
        head -40 "$TMP/$crate.diff" >&2
        echo "(modification voulue : scripts/verifier-vendor.sh --regenerer, puis la décrire dans HYPRDESK.md)" >&2
        rc=1
    fi
done
exit "$rc"
