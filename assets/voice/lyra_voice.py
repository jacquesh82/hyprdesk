# hyprdesk : écoute de LYRA (mot d'activation « Lyra »), par Vosk, hors ligne.
#
# Lancé par hyprdesk : python lyra_voice.py <modèle-fr> <modèle-en>
# Micro : pw-record (PipeWire), 16 kHz mono. Écrit une ligne JSON par
# événement sur la sortie standard :
#   {"event": "ready"}
#   {"event": "wake"}                                   mot entendu
#   {"event": "text", "lang": "fr", "text": "mets FIP"}  demande
#   {"event": "error", "message": "..."}
# Une ligne « pause » / « resume » sur l'entrée standard suspend l'écoute
# (pendant que LYRA parle).

import json
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


def main():
    vosk.SetLogLevel(-1)
    paths = {"fr": sys.argv[1], "en": sys.argv[2]}
    recs = {}
    for lang, path in paths.items():
        r = vosk.KaldiRecognizer(vosk.Model(path), RATE)
        r.SetWords(True)
        recs[lang] = r
    mic = subprocess.Popen(
        ["pw-record", "--raw", "--rate", str(RATE), "--channels", "1", "--format", "s16", "-"],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    emit(event="ready")
    paused = False
    armed_until = 0.0  # « Lyra » entendu seul : la phrase suivante est la demande
    finals = {}  # langue -> mots de la dernière phrase finie
    first_final = 0.0
    woke = False
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
                for r in recs.values():
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
        for lang, r in recs.items():
            if r.AcceptWaveform(data):
                words = json.loads(r.Result()).get("result", [])
                finals[lang] = words
                if len(finals) == 1:
                    first_final = time.monotonic()
            elif not woke:
                partial = json.loads(r.PartialResult()).get("partial", "")
                if any(w.strip(".,!?") in WAKE for w in partial.lower().split()):
                    woke = True
                    emit(event="wake")
        # Les deux modèles ont fini la phrase (ou l'un d'eux depuis 1,2 s).
        if finals and (len(finals) == len(recs) or time.monotonic() - first_final > 1.2):
            best = None
            for lang, words in finals.items():
                rest = after_wake(words)
                if rest is None and time.monotonic() < armed_until:
                    rest = words
                if rest is None:
                    continue
                score = conf(rest) if rest else 0.0
                if best is None or score > best[0]:
                    best = (score, lang, rest)
            finals.clear()
            woke = False
            if best is None:
                continue
            _, lang, rest = best
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
