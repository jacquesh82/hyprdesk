# hyprdesk : écoute de LYRA (mot d'activation « Lyra »), par Vosk, hors ligne.
#
# Lancé par hyprdesk : python lyra_voice.py <modèle-fr> <modèle-en> [micro]
# Micro : pw-record (PipeWire), 16 kHz mono. Écrit une ligne JSON par
# événement sur la sortie standard :
#   {"event": "ready"}
#   {"event": "wake"}                                   mot entendu
#   {"event": "text", "lang": "fr", "text": "mets FIP"}  demande
#   {"event": "error", "message": "..."}
# Une ligne « pause » / « resume » sur l'entrée standard suspend l'écoute
# (pendant que LYRA parle).

import array
import json
import os
import select
import subprocess
import sys
import time

try:
    import vosk
except Exception as e:  # pragma: no cover
    print(json.dumps({"event": "error", "message": f"vosk : {e}"}), flush=True)
    sys.exit(1)

RATE = 16000
# Mot d'activation et ce que les petits modèles en entendent.
WAKE = {"lyra", "lira", "lyre", "lire", "lyrat", "laïra", "lira's", "lyre's", "lara", "laira"}
# Fenêtre pour une demande après « Lyra » seul (secondes).
FOLLOW = 7.0


def emit(**kw):
    print(json.dumps(kw, ensure_ascii=False), flush=True)


def after_wake(words):
    """Mots qui suivent le mot d'activation ; None s'il n'y est pas."""
    for i, w in enumerate(words):
        if w["word"].lower().strip(".,!?") in WAKE:
            return words[i + 1 :]
    return None


def conf(words):
    return sum(w.get("conf", 0.0) for w in words) / len(words) if words else 0.0


class Gain:
    """Gain automatique : un micro lointain (webcam) parle trop bas pour les
    petits modèles ; on vise une crête vers 12 000, gain 1 à 12."""

    def __init__(self):
        self.peak = 2000.0

    def apply(self, data):
        a = array.array("h", data)
        top = max((abs(x) for x in a), default=0)
        # Crête lissée : monte vite, redescend lentement.
        self.peak = max(top, self.peak * 0.995, 300.0)
        g = min(12.0, max(1.0, 12000.0 / self.peak))
        if g > 1.05:
            a = array.array("h", (max(-32768, min(32767, int(x * g))) for x in a))
        return a.tobytes()


def main():
    vosk.SetLogLevel(-1)
    paths = {"fr": sys.argv[1], "en": sys.argv[2]}
    # Micro choisi (nœud PipeWire) ; sans : celui par défaut.
    target = sys.argv[3] if len(sys.argv) > 3 else None
    models = {lang: vosk.Model(path) for lang, path in paths.items()}
    recs = {}
    for lang, model in models.items():
        r = vosk.KaldiRecognizer(model, RATE)
        r.SetWords(True)
        recs[lang] = r
    # Réveil : un détecteur limité au mot (« Lyra » n'est pas dans le
    # vocabulaire ; « lira » s'en approche et y est).
    wake_rec = vosk.KaldiRecognizer(models["fr"], RATE, json.dumps(["lira", "lyre", "[unk]"]))
    rec = ["pw-record", "--raw", "--rate", str(RATE), "--channels", "1", "--format", "s16"]
    if target:
        rec += ["--target", target]
    # Essai : un fichier brut 16 kHz mono s16 à la place du micro.
    test = os.environ.get("LYRA_VOICE_INPUT")
    if test:
        rec = ["cat", test]
    else:
        rec += ["-"]
    mic = subprocess.Popen(
        rec,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    emit(event="ready")
    gain = Gain()
    paused = False
    armed_until = 0.0  # « Lyra » entendu : la phrase en cours ou suivante est la demande
    finals = {}  # langue -> mots de la dernière phrase finie
    first_final = 0.0
    while True:
        # Commandes de hyprdesk (pause pendant que LYRA parle).
        while select.select([sys.stdin], [], [], 0)[0]:
            line = sys.stdin.readline()
            if not line:
                mic.kill()
                return
            cmd = line.strip()
            if cmd == "pause":
                paused = True
            elif cmd == "resume":
                paused = False
                for r in list(recs.values()) + [wake_rec]:
                    r.Reset()
                finals.clear()
            elif cmd == "arm":
                # Question posée par LYRA : la phrase suivante est la réponse.
                armed_until = time.monotonic() + FOLLOW
        data = mic.stdout.read(4000)
        if not data:
            emit(event="error", message="micro indisponible (pw-record)")
            return
        if paused:
            continue
        data = gain.apply(data)
        now = time.monotonic()
        # Mot d'activation.
        if wake_rec.AcceptWaveform(data):
            heard = json.loads(wake_rec.Result()).get("text", "")
        else:
            heard = json.loads(wake_rec.PartialResult()).get("partial", "")
        if now >= armed_until and any(w in ("lira", "lyre") for w in heard.split()):
            armed_until = now + FOLLOW
            wake_rec.Reset()
            emit(event="wake")
        for lang, r in recs.items():
            if r.AcceptWaveform(data):
                words = json.loads(r.Result()).get("result", [])
                if words:
                    finals[lang] = words
                    if len(finals) == 1:
                        first_final = now
        # Les deux modèles ont fini la phrase (ou l'un d'eux depuis 1,2 s).
        if finals and (len(finals) == len(recs) or now - first_final > 1.2):
            armed = time.monotonic() < armed_until
            best = None
            for lang, words in finals.items():
                rest = after_wake(words)
                if rest is None and armed:
                    rest = words
                score = conf(rest if rest else words)
                if best is None or score > best[0]:
                    best = (score, lang, rest, words)
            finals.clear()
            _, lang, rest, words = best
            # Diagnostic : ce que le micro a entendu (affiché, jamais gardé).
            emit(event="heard", text=" ".join(w["word"] for w in words))
            if rest is None:
                continue
            text = " ".join(w["word"] for w in rest).strip()
            if text:
                armed_until = 0.0
                emit(event="text", lang=lang, text=text)
            else:
                armed_until = time.monotonic() + FOLLOW
                emit(event="wake")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        pass
