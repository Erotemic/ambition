#!/usr/bin/env python3
"""Fetch Smash Ultimate's per-hitbox knockback data for chosen fighters' moves.

The table is a copy of SmashWiki's move pages (``ssbwiki.com/<Fighter>_(SSBU)/<Move>``),
whose "Hitboxes" section lists one ``UltimateHitboxTableRow`` per hitbox. It is
attributed in ``docs/related-work/smash-ultimate-fighter-reference.md``.

Values are kept in Ultimate's own terms and spelled as the source spells them:
``damage`` in percent, ``angle`` in degrees (361 is the Sakurai angle), ``bk``
base knockback, ``ks`` knockback scaling (growth, in percent), ``fkv`` fixed
knockback (0 = none). ``section`` is the nearest heading above the row, which
is how a page tells a multi-hit move's hits apart.

The default fighters are the anchors Ambition's fighter facets were shaped
from, plus every fighter named with ``--fighter``. The default moves are the
ground and air normals.

Usage::

    python3 scripts/fetch_smash_ultimate_moves.py
    python3 scripts/fetch_smash_ultimate_moves.py --fighter "Ganondorf" --move Up_tilt
"""
import argparse
import csv
import re
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "docs/related-work/data/smash_ultimate_move_hitboxes.csv"
PAGE = "https://www.ssbwiki.com/index.php?title={title}&action=raw"

ANCHORS = [
    "Mario", "Luigi", "Mewtwo", "King Dedede", "Sheik", "Samus", "Diddy Kong",
    "Zelda", "Rosalina & Luma", "Dr. Mario", "Marth", "Robin", "Charizard",
    "Little Mac", "Steve", "Lucina", "Palutena",
]
MOVES = [
    "Forward_tilt", "Up_tilt", "Down_tilt", "Dash_attack",
    "Forward_smash", "Up_smash", "Down_smash",
    "Neutral_aerial", "Forward_aerial", "Back_aerial", "Up_aerial", "Down_aerial",
]
FIELDS = ["damage", "angle", "bk", "ks", "fkv"]
COLUMNS = ["fighter", "move", "section", "id"] + FIELDS


def fetch(title):
    url = PAGE.format(title=urllib.parse.quote(title, safe="/()_"))
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (ambition reference fetch)"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return r.read().decode("utf-8")


def follow_redirect(text):
    m = re.match(r"\s*#REDIRECT\s*\[\[([^\]|#]+)", text, re.I)
    return m.group(1).replace(" ", "_") if m else None


def plain(value):
    """A cell's number. A template's first argument is the base value: a charged
    smash spells `{{ChargedSmashDmgSSBU|6.0}}`, a damage multiplier
    `{{rollover|6.3%|7.4088% with ...|y}}`."""
    m = re.fullmatch(r"\{\{[^|}]*\|([^|}]*)(\|[^}]*)?\}\}", value)
    return (m.group(1) if m else value).strip().rstrip("%")


def rows_of(fighter, move, text):
    """Each hitbox row of a page, with the heading it sits under."""
    out = []
    section = ""
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        heading = re.match(r"^(=+)\s*(.*?)\s*\1\s*$", line)
        if heading:
            # The "Hitboxes" heading opens the table; a heading below it names a hit.
            section = "" if heading.group(2) == "Hitboxes" else heading.group(2)
        if line.startswith("{{UltimateHitboxTableRow"):
            fields = {}
            i += 1
            while i < len(lines) and not lines[i].startswith("}}"):
                m = re.match(r"^\|(\w+)=(.*)$", lines[i].strip())
                if m:
                    fields[m.group(1)] = m.group(2).strip()
                i += 1
            row = {"fighter": fighter, "move": move, "section": section, "id": fields.get("id", "")}
            for f in FIELDS:
                row[f] = plain(fields.get(f, ""))
            out.append(row)
        i += 1
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--fighter", action="append", default=[], help="add a fighter (SmashWiki spelling)")
    ap.add_argument("--move", action="append", default=[], help="only these moves (SmashWiki page names)")
    ap.add_argument("--delay", type=float, default=1.0, help="seconds between requests")
    args = ap.parse_args()
    fighters = ANCHORS + [f for f in args.fighter if f not in ANCHORS]
    moves = args.move or MOVES
    rows = []
    for fighter in fighters:
        for move in moves:
            title = f"{fighter.replace(' ', '_')}_(SSBU)/{move}"
            try:
                text = fetch(title)
                target = follow_redirect(text)
                if target:
                    time.sleep(args.delay)
                    text = fetch(target)
            except Exception as err:  # a missing page is a gap in the table, not a failure
                print(f"skip {title}: {err}", file=sys.stderr)
                continue
            got = rows_of(fighter, move, text)
            if not got:
                print(f"no hitbox rows: {title}", file=sys.stderr)
            rows.extend(got)
            time.sleep(args.delay)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with OUT.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=COLUMNS)
        w.writeheader()
        w.writerows(rows)
    print(f"wrote {len(rows)} hitbox rows -> {OUT}")


if __name__ == "__main__":
    main()
