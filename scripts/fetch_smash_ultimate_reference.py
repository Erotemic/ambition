#!/usr/bin/env python3
"""Fetch Smash Ultimate's per-fighter movement and evade data into one CSV.

The table is the reference ``docs/related-work/smash-ultimate-fighter-reference.md``
reads from. It is a copy of two community sources, attributed in that page:

* Ultimate Frame Data's roster statistics page (``ultimateframedata.com/stats.php``);
* SmashWiki's Ultimate rolling-distance tables (``ssbwiki.com/Roll``) and
  traction and air friction tables (``ssbwiki.com/Traction``,
  ``ssbwiki.com/Air_friction``).

Values are kept in Ultimate's own units (distance units, frames at 60 Hz) and
spelled exactly as the source spells them, so a re-fetch diffs cleanly. Nothing
here is converted to Ambition's scale: that is the reader's job, and its factor
is stated in the page.

Usage::

    python3 scripts/fetch_smash_ultimate_reference.py            # fetch + write
    python3 scripts/fetch_smash_ultimate_reference.py --html F    # parse a saved stats page
"""
import argparse
import csv
import html
import re
import sys
import urllib.request
from pathlib import Path

UFD_URL = "https://ultimateframedata.com/stats.php"
ROLL_URL = "https://www.ssbwiki.com/index.php?title=Roll&action=raw"
TRACTION_URL = "https://www.ssbwiki.com/index.php?title=Traction&action=raw"
AIR_FRICTION_URL = "https://www.ssbwiki.com/index.php?title=Air_friction&action=raw"
OUT = Path(__file__).resolve().parent.parent / "docs/related-work/data/smash_ultimate_fighter_stats.csv"

# The roster is the "Dash and Run Speed" table's: one row per fighter. Other
# tables spell some fighters differently, or merge echoes ("Marth/Lucina"); a
# spelling is matched on letters and digits only, through these keys.
ROSTER_TABLE = "Dash and Run Speed"
ALIASES = {
    "dedede": "kingdedede",
    "dk": "donkeykong",
    "rosalinaandluma": "rosalinaluma",
    "rosalina": "rosalinaluma",
    "iceclimbers": "iceclimbersleader",
    "popo": "iceclimbersleader",
    "nana": "iceclimberspartner",
    "pokemontrainercharizard": "charizard",
    "pokemontrainerivysaur": "ivysaur",
    "pokemontrainersquirtle": "squirtle",
    "ptcharizard": "charizard",
    "ptivysaur": "ivysaur",
    "ptsquirtle": "squirtle",
    "rosanoluma": None,
    "pokemontrainerall": None,
}

# section title -> {source column -> csv column}; unnamed columns are dropped.
UFD_TABLES = {
    "Walk Speed": {"Walk Speed": "walk_speed"},
    "Dash and Run Speed": {
        "Initial Dash": "initial_dash",
        "Run Speed": "run_speed",
        "Dash Frames": "dash_frames",
        "Pivot Dash Frames": "pivot_dash_frames",
    },
    "Dash Turnaround": {
        "Normal (Buffer Window)": "dash_turn_normal",
        "Slow (Full Dash)": "dash_turn_slow",
    },
    "Air Speed": {"Air speed": "air_speed"},
    "Air Acceleration": {"Base": "air_accel_base", "Additional": "air_accel_add", "Max": "air_accel_max"},
    "Gravity": {"Gravity": "gravity"},
    "Fall Speed": {"Regular Fall": "fall_speed", "Fast Fall": "fast_fall_speed"},
    "Weight": {"Weight": "weight"},
    "Jump Height": {"Full Hop": "full_hop", "Short Hop": "short_hop", "Air Jump": "air_jump"},
    "Jump Durations": {
        "Short Hop": "sh_frames",
        "Full Hop": "fh_frames",
        "SH Fast Fall": "sh_ff_frames",
        "FH Fast Fall": "fh_ff_frames",
    },
    "Landing": {"Hard Land": "hard_landing_lag"},
    "Spot Dodges": {
        "Total Frames With/Without Cancel": "spot_dodge_total",
        "Intangible Start Frame": "spot_dodge_int_start",
        "Intangible Last Frame": "spot_dodge_int_last",
    },
    "Forward Rolls": {"Startup": "froll_startup", "Intangible Until": "froll_int_until", "Total": "froll_total"},
    "Backward Rolls": {"Startup": "broll_startup", "Intangible Until": "broll_int_until", "Total": "broll_total"},
    "Neutral Air Dodges": {"Startup": "nairdodge_startup", "Intangible Until": "nairdodge_int_until", "Total": "nairdodge_total"},
    "Grab Range": {"Grab Range": "grab_range"},
    "Out of Shield": {"Grab": "oos_grab_frame"},
}
ROLL_SECTIONS = {"Forward rolls": "froll_distance", "Back rolls": "broll_distance"}


def fetch(url):
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (ambition reference fetch)"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return r.read().decode("utf-8")


def text(cell):
    return " ".join(html.unescape(re.sub(r"<[^>]+>", " ", cell)).split())


def key(raw):
    k = re.sub(r"[^a-z0-9]", "", raw.lower())
    return ALIASES.get(k, k)


def resolve(cells, roster):
    """Fold every table's spelling onto the roster's; report what does not fold."""
    by_key = {key(n): n for n in roster}
    rows = {n: {} for n in roster}
    dropped = set()
    for raw, col, value in cells:
        hits = [by_key.get(key(part)) for part in raw.split("/")]
        if not all(hits):
            dropped.add(raw)
            continue
        for who in hits:
            # A merged row ("Marth/Lucina") fills a gap; it never overwrites a
            # fighter's own row.
            if not rows[who].get(col) or "/" not in raw:
                rows[who][col] = value
    return rows, sorted(dropped)


def parse_ufd(page, cells, roster):
    for section in re.split(r"<h2>", page)[1:]:
        title = section.split("<")[0].strip()
        wanted = UFD_TABLES.get(title)
        if not wanted:
            continue
        table = re.search(r"<table.*?</table>", section, re.S)
        if not table:
            sys.exit(f"stats page: no table under {title!r}")
        table = table.group(0)
        heads = [text(h) for h in re.findall(r"<th[^>]*>(.*?)</th>", table, re.S)]
        missing = set(wanted) - set(heads)
        if missing:
            sys.exit(f"stats page: {title!r} lost columns {sorted(missing)} (has {heads})")
        for tr in re.findall(r"<tr[^>]*>(.*?)</tr>", table, re.S):
            row = [text(c) for c in re.findall(r"<td[^>]*>(.*?)</td>", tr, re.S)]
            if len(row) < len(heads) - 1:
                continue
            record = dict(zip(heads, row))
            who = record.get("Character", "").strip()
            if not who:
                continue
            if title == ROSTER_TABLE:
                roster.append(who)
            for src, col in wanted.items():
                cells.append((who, col, record[src]))


def parse_rolls(wiki, cells):
    start = wiki.index("==Rolling distances in ''Ultimate''==")
    body = wiki[start:]
    for heading, col in ROLL_SECTIONS.items():
        i = body.index(f"==={heading}===")
        j = body.find("|}", i)
        for m in re.finditer(r"\{\{CharHead\|([^|}]+)\|SSBU(?:\|[^}]*)?\}\}\|\|(?:\{\{rollover\|)?([0-9.]+)", body[i:j]):
            cells.append((m.group(1), col, m.group(2)))


def parse_wiki_table(wiki, columns, what):
    """The first table of a SmashWiki page's Ultimate section, one row per
    fighter. Tied rows share a value through `rowspan`, cell by cell, so each
    column carries its own span."""
    body = wiki[wiki.index("===''[[Super Smash Bros. Ultimate]]''===") :]
    body = body[: body.index("|}")]
    carry, left = [None] * len(columns), [0] * len(columns)
    cells = []
    for row in body.split("|-")[1:]:
        who = re.search(r"\{\{CharHead\|([^|}]+)\|SSBU(?:\|[^}]*)?\}\}", row)
        if not who:
            continue
        given = iter(row[who.end() :].split("||")[1:])
        for c, col in enumerate(columns):
            if not left[c]:
                cell = next(given, None)
                own = cell and re.match(r"\s*(?:rowspan=(\d+)\|)?(?:\{\{rollover\|)?([0-9.]+)", cell)
                if not own:
                    sys.exit(f"{what} table: no {col} for {who.group(1)!r}")
                carry[c], left[c] = own.group(2), int(own.group(1) or 1)
            cells.append((who.group(1), col, carry[c]))
            left[c] -= 1
    return cells


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--html", type=Path, help="a saved copy of the stats page")
    ap.add_argument("--roll-wiki", type=Path, help="a saved copy of the Roll page's wikitext")
    ap.add_argument("--traction-wiki", type=Path, help="a saved copy of the Traction page's wikitext")
    ap.add_argument("--air-friction-wiki", type=Path, help="a saved copy of the Air friction page's wikitext")
    ap.add_argument("--out", type=Path, default=OUT)
    args = ap.parse_args()

    page = args.html.read_text(encoding="utf-8") if args.html else fetch(UFD_URL)
    wiki = args.roll_wiki.read_text(encoding="utf-8") if args.roll_wiki else fetch(ROLL_URL)
    traction = args.traction_wiki.read_text(encoding="utf-8") if args.traction_wiki else fetch(TRACTION_URL)
    friction = args.air_friction_wiki.read_text(encoding="utf-8") if args.air_friction_wiki else fetch(AIR_FRICTION_URL)
    cells, roster = [], []
    parse_ufd(page, cells, roster)
    parse_rolls(wiki, cells)
    cells += parse_wiki_table(traction, ["traction"], "traction")
    cells += parse_wiki_table(friction, ["air_friction", "falling_friction"], "air friction")
    rows, dropped = resolve(cells, roster)

    columns = ["character"] + [c for t in UFD_TABLES.values() for c in t.values()] + list(ROLL_SECTIONS.values()) + ["traction", "air_friction", "falling_friction"]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    tmp = args.out.with_suffix(".tmp")
    with tmp.open("w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, columns, extrasaction="raise", restval="")
        w.writeheader()
        for who in sorted(rows):
            w.writerow({"character": who, **rows[who]})
    tmp.replace(args.out)
    print(f"wrote {len(rows)} fighters x {len(columns) - 1} columns -> {args.out}")
    if dropped:
        print("rows naming no roster fighter (left out):", "; ".join(dropped))


if __name__ == "__main__":
    main()
