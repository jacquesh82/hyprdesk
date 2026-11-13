#!/usr/bin/env bash
# Démo animée du README (MLG-115), filmée dans la vraie session :
# connexion sur l'écran greetd (simulée), les trois modes, retour au mode 1
# et deux images ouvertes, modes 2 et 3 avec les fenêtres rangées, visite
# des Réglages, une question à LYRA. Interface en anglais le temps de la
# prise ; tout est remis en place à la fin.
#
#   OUT=/tmp/demo scripts/demo.sh             # → $OUT/raw.mp4, demo.mp4, demo.gif
#
# Variables : WS (workspace vide, 3), IMG1 et IMG2 (images), DEMO_USER
# (utilisateur affiché, alex), HYPRDESK_BIN (binaire du greeter, celui de
# target/debug s'il existe). Prérequis : hyprdesk en marche, wf-recorder,
# wtype, mpv, ffmpeg ; session déverrouillée, ne pas toucher au clavier.
# Pièges : un workspace sans fenêtre pour le premier passage des modes
# (sinon les fenêtres cachent les icônes) ; mpv, qui réajuste l'image quand
# la fenêtre change de taille.
set -euo pipefail
REPO="$(cd "$(dirname "$0")/.." && pwd)"
DEV="${HYPRDESK_BIN:-$REPO/target/debug/hyprdesk}"
[[ -x "$DEV" ]] || DEV="$(command -v hyprdesk)"
OUT="${OUT:?OUT=dossier de sortie}"
WS="${WS:-3}"
IMG1="${IMG1:?IMG1=première image}"
IMG2="${IMG2:?IMG2=seconde image}"
DEMO_USER="${DEMO_USER:-alex}"
mkdir -p "$OUT"
cd "$REPO"
for t in wf-recorder wtype mpv ffmpeg hyprctl hyprdesk; do
    command -v "$t" >/dev/null || { echo "$t absent" >&2; exit 1; }
done

pgrep -f 'hyprdesk lock$' >/dev/null && { echo "session verrouillée" >&2; exit 1; }
BACK="$(hyprctl activeworkspace -j | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["id"])')"
LANG_BEFORE="$(hyprdesk config get language)"
N="$(hyprctl workspaces -j | python3 -I -c "import json,sys; print(sum(w['windows'] for w in json.load(sys.stdin) if w['id']==$WS))")"
[[ "$N" == 0 ]] || { echo "workspace $WS non vide" >&2; exit 1; }

PIDS=()
REC=""
GREETER=""
cleanup() {
    [[ -n "$REC" ]] && kill -INT "$REC" 2>/dev/null || true
    [[ -n "$GREETER" ]] && kill "$GREETER" 2>/dev/null || true
    hyprdesk hide >/dev/null 2>&1 || true
    ((${#PIDS[@]})) && kill "${PIDS[@]}" 2>/dev/null || true
    hyprdesk config set language "$LANG_BEFORE" >/dev/null 2>&1 || true
    hyprctl dispatch "hl.dsp.focus({ workspace = $BACK })" >/dev/null
}
trap cleanup EXIT

dsp() { hyprctl dispatch "$1" >/dev/null; }
cursor() { dsp "hl.dsp.cursor.move({ x = $1, y = $2 })"; }
# client_pid <pid> → "adresse x y w h" de la fenêtre de ce processus.
client_pid() {
    hyprctl clients -j | python3 -I -c "
import json,sys
for c in json.load(sys.stdin):
    if c['pid']==int(sys.argv[1]):
        print(c['address'], *c['at'], *c['size'])" "$1"
}
# Glisse une fenêtre (par le haut) de (dx, dy), curseur compris.
drag() { # drag <pid> <dx> <dy>
    local ADDR X Y W H
    read -r ADDR X Y W H < <(client_pid "$1")
    local CX=$((X + W / 2)) CY=$((Y + 20))
    cursor "$CX" "$CY"; sleep 0.4
    for i in $(seq 1 16); do
        local DX=$(($2 * i / 16)) DY=$(($3 * i / 16))
        dsp "hl.dsp.window.move({ x = $((X + DX)), y = $((Y + DY)), relative = false, window = \"address:$ADDR\" })"
        cursor $((CX + DX)) $((CY + DY))
        sleep 0.05
    done
    sleep 0.5
}
# Glisse une fenêtre vers (x, y) en la mettant à la taille w × h.
drag_to() { # drag_to <pid> <x> <y> <w> <h>
    local ADDR X Y W H
    read -r ADDR X Y W H < <(client_pid "$1")
    for i in $(seq 1 18); do
        local NX=$((X + ($2 - X) * i / 18)) NY=$((Y + ($3 - Y) * i / 18))
        local NW=$((W + ($4 - W) * i / 18)) NH=$((H + ($5 - H) * i / 18))
        dsp "hl.dsp.window.resize({ x = $NW, y = $NH, window = \"address:$ADDR\" })"
        dsp "hl.dsp.window.move({ x = $NX, y = $NY, relative = false, window = \"address:$ADDR\" })"
        cursor $((NX + NW / 2)) $((NY + 20))
        sleep 0.045
    done
    sleep 0.4
}
# Frappe seulement si l'écran de connexion de démo tient le clavier.
type_greeter() {
    hyprctl layers -j | grep -q '"namespace": *"hyprdesk-greeter"' \
        || { echo "écran de connexion absent : pas de frappe" >&2; return 0; }
    wtype -d 110 "$@"
}

hyprdesk config set language en >/dev/null
dsp "hl.dsp.focus({ workspace = $WS })"
sleep 0.5
MON="$(hyprctl activeworkspace -j | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["monitor"])')"
MW="$(hyprctl monitors -j | python3 -I -c "import json,sys; print([m['width'] for m in json.load(sys.stdin) if m['name']=='$MON'][0])")"
hyprdesk hide >/dev/null; sleep 0.5
cursor $((MW / 2 + 400)) 900

# Écran de connexion (démo filmée : utilisateur fictif, validation = ouverture).
USER="$DEMO_USER" HYPRDESK_GREETER_FILM=1 "$DEV" greeter --demo >/dev/null 2>&1 & GREETER=$!
sleep 2.5

wf-recorder -o "$MON" -r 30 -c libx264 -p preset=veryfast -p crf=18 -f "$OUT/raw.mp4" >/dev/null 2>&1 &
REC=$!
sleep 2

# Connexion : Entrée sur l'utilisateur, le mot de passe, Entrée.
type_greeter -k Return; sleep 1.2
type_greeter "correct-horse"; sleep 0.6
type_greeter -k Return; sleep 2.6
GREETER=""

# Les trois modes : le fond seul, le dock, le bureau complet (icônes,
# widgets), le curseur passe sur les icônes.
hyprdesk cycle; sleep 2.2
hyprdesk cycle; sleep 1
for y in 140 260 380 260; do cursor 90 "$y"; sleep 0.45; done
sleep 1.2
# Retour au mode 1, deux images (en mosaïque).
hyprdesk cycle; sleep 1.2
VIEW=(mpv --keep-open=yes --image-display-duration=inf --no-osc --osd-level=0 --no-input-default-bindings --autofit=50%)
"${VIEW[@]}" --title="$(basename "$IMG1")" "$IMG1" >/dev/null 2>&1 & PIDS+=($!); I1=$!; sleep 1.2
"${VIEW[@]}" --title="$(basename "$IMG2")" "$IMG2" >/dev/null 2>&1 & PIDS+=($!); I2=$!; sleep 2.4
# Mode 2 (dock), puis mode 3 : les images flottent sur le bureau, on les range.
hyprdesk cycle; sleep 2.2
hyprdesk cycle; sleep 1.6
drag_to "$I1" 470 140 960 540
drag_to "$I2" 1250 760 960 540
sleep 1.4

# Visite des Réglages.
for p in wallpaper widgets keys screens effects; do
    hyprdesk settings "$p"; sleep 2.3
done

# LYRA (ferme les Réglages), réponse en direct.
hyprdesk ask "Demo" "Which key brings up the desktop, and how do I add my own widget? Two short sentences."
sleep 9
hyprdesk assistant; sleep 1.2

# Retour au tiling.
hyprdesk hide; sleep 2.4
kill -INT "$REC"; wait "$REC" 2>/dev/null || true
REC=""

# README : mp4 1920 px et GIF 960 px (12 images/s, palette de 128 couleurs).
ffmpeg -v error -y -i "$OUT/raw.mp4" -vf "scale=1920:-2" -c:v libx264 -crf 26 -preset slow \
    -pix_fmt yuv420p -movflags +faststart -an "$OUT/demo.mp4"
ffmpeg -v error -y -i "$OUT/raw.mp4" -vf "fps=12,scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle" \
    "$OUT/demo.gif"
echo "ok : $OUT/demo.gif, $OUT/demo.mp4 (à copier dans assets/screenshots/hyprdesk-demo.*)"
