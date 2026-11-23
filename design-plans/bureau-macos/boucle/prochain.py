#!/usr/bin/env python3
"""Next findings of a lane, read from the loop ledger.

Usage (from the main checkout):
    python3 -I .zcode/hig-boucle/prochain.py <A|B|C> [nombre]   next findings of the lane
    python3 -I .zcode/hig-boucle/prochain.py <A|B|C> --tous      every pending finding of the lane
    python3 -I .zcode/hig-boucle/prochain.py --bilan             counts for every lane
    python3 -I .zcode/hig-boucle/prochain.py --regles            the lane rules

This file is the single source of truth for which lane owns a finding.
"""
import collections
import os
import re
import sys

LEDGER = "~/.cache/hyprdesk-wt-hig-{lane}/design-plans/bureau-macos/boucle/avancement.md"
PREFIX = {"SYS": "A", "PAT": "A", "STA": "A", "FON": "A", "ACC": "A",
          "SEL": "B", "NAV": "B", "TYP": "B", "LAY": "B", "PRES": "B",
          "MENU": "C", "ENT": "C", "ICO": "C", "DAT": "C"}
EXTRA_A = {"LAY-02", "LAY-03", "LAY-04", "ICO-03", "ICO-07", "TYP-19"}
EXTRA_B = {"PAT-09", "PAT-13", "PAT-16", "ACC-10",
           "DAT-01", "DAT-02", "DAT-03", "DAT-05", "DAT-06", "DAT-07", "DAT-09", "DAT-10",
           "DAT-11", "DAT-13", "DAT-14", "DAT-15", "DAT-18", "DAT-20"}
EXTRA_C = {"PAT-12", "PAT-14", "TYP-17", "DAT-04", "DAT-08", "DAT-16", "DAT-17", "DAT-19"}
PENDING = ("en cours", "à reprendre", "à faire")
ORDER = {"en cours": 0, "à reprendre": 1, "à faire": 2}
ROW = re.compile(r"^\|\s*([A-Z]{2,5}-\d+|[A-Z]+-[A-Z0-9]+)\s*\|(.*)\|\s*$")


def lane_of(ident):
    if ident in EXTRA_A:
        return "A"
    if ident in EXTRA_B:
        return "B"
    if ident in EXTRA_C:
        return "C"
    return PREFIX.get(ident.split("-")[0], "A")


def rows(lane):
    path = os.path.expanduser(LEDGER.format(lane=lane))
    phase, out = "?", []
    for line in open(path, encoding="utf-8"):
        if line.startswith("## "):
            phase = line[3:].strip()
            continue
        m = ROW.match(line)
        if not m:
            continue
        cells = [c.strip() for c in re.split(r"(?<!\\)\|", m.group(2))]
        if len(cells) < 6:
            continue
        out.append((m.group(1), lane_of(m.group(1)), cells[0], cells[1], cells[2], cells[4], phase))
    return out


def short(text, n):
    return text if len(text) <= n else text[:n - 1] + "…"


def main():
    args = sys.argv[1:]
    if "--regles" in args:
        print("Couloir A (notifications, shell, matière, accessibilité) : " + ", ".join(sorted(p for p in PREFIX if PREFIX[p] == "A")) + " ; plus " + ", ".join(sorted(EXTRA_A)))
        print("Couloir B (réglages, saisie, écriture, présentation) : " + ", ".join(sorted(p for p in PREFIX if PREFIX[p] == "B")) + " ; plus " + ", ".join(sorted(EXTRA_B)))
        print("Couloir C (menus, icônes, entrées, bureau) : " + ", ".join(sorted(p for p in PREFIX if PREFIX[p] == "C")) + " ; plus " + ", ".join(sorted(EXTRA_C)))
        return
    if "--bilan" in args:
        every = rows("A")  # un seul registre suffit : les trois couloirs portent le meme
        for lane in "ABC":
            count = collections.Counter(r[5] for r in every if r[1] == lane)
            print(f"couloir {lane} : " + ", ".join(f"{k} {v}" for k, v in sorted(count.items())))
        left = [r for r in every if r[5] in PENDING]
        print("reste à traiter, tous couloirs :", len(left), "| en cours :", sum(1 for r in left if r[5] == "en cours"))
        return
    if not args or args[0] not in ("A", "B", "C"):
        print(__doc__)
        sys.exit(2)
    lane = args[0]
    mine = [r for r in rows(lane) if r[1] == lane]
    count = collections.Counter(r[5] for r in mine)
    print(f"couloir {lane} : " + ", ".join(f"{k} {count[k]}" for k in
          ("à faire", "à reprendre", "en cours", "fait", "bloqué", "sans objet") if count[k]))
    pending = sorted((r for r in mine if r[5] in PENDING), key=lambda r: (ORDER[r[5]],))
    if "--tous" in args:
        print("en attente :", " ".join(f"{r[0]}({r[5]})" if r[5] != "à faire" else r[0] for r in pending))
        return
    number = int(args[1]) if len(args) > 1 and args[1].isdigit() else 6
    for ident, _, objet, nature, effort, statut, phase in pending[:number]:
        print(f"{ident:9s} {statut:11s} {effort} {nature:7s} {short(phase.split('—')[0].strip(), 9):9s} {short(objet, 78)}")


if __name__ == "__main__":
    main()
