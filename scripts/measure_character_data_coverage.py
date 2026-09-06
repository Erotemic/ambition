#!/usr/bin/env python3
"""Two per-character coverage questions Jon's reports keep asking.

Both live in `JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`, both have been re-counted
by hand three times, and both are one grep away from a wrong answer.

  KNOCKDOWN ROWS  which sheets carry all four of `knockdown` / `getup` / `tech`
                  / `getup_attack`. A sheet without them does not break — it
                  falls `knockdown → prone → land_hard → hit → idle`, so a
                  knocked-down character plays its HIT pose while lying there.
                  That is why it reads as wrong rather than as missing.

  STANDING HEIGHT which catalog rows author `standing_height`, the shared unit
                  that makes a character measure a real world height instead of
                  falling to `body_kind`'s default of 48.0 — the player robot's
                  own height, which is why an adult pirate stands exactly as
                  tall as a chibi robot.

⛔⛔ THE MANIFESTS COME IN TWO SHAPES AND A REGEX FOR ONE SEES NONE OF THE OTHER.
`noether_spritesheet.ron` lists animations as `body_metrics.animations` MAP KEYS
(`"knockdown": (...)`); `officer_spritesheet.ron` names them in ROW FIELDS
(`animation: "knockdown"`). Matching only the first form reported 4 sheets where
13 carry the rows, and called `officer` — a file that plainly contains all four
words — empty. Measured 2026-09-02. Both forms are matched here.

⚠ AND COUNT ROWS, NOT LINES. `grep -c standing_height` counts lines and picks up
comments; this splits the catalog on its 8-space id headers and reads each row's
own `Some(...)`.

Usage:
    scripts/measure_character_data_coverage.py
    scripts/measure_character_data_coverage.py --json
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SHEETS = REPO / "crates/ambition_platformer2d_actor_monolith/assets/sprites"
CATALOG = REPO / "game/ambition_content/assets/data/character_catalog.ron"
KNOCKDOWN_ROWS = ("knockdown", "getup", "tech", "getup_attack")
DEFAULT_HEIGHT = 48.0

ROW_HEADER = re.compile(r'\n(?=        "[a-z_0-9]+": \()')
ROW_ID = re.compile(r'\s*"([a-z_0-9]+)": \(')
HEIGHT = re.compile(r"standing_height:\s*Some\(([0-9.]+)\)")


def sheet_has_row(text: str, row: str) -> bool:
    """Both manifest shapes — a map key AND a row field. See the module note."""
    return bool(
        re.search(rf'"{row}"\s*:', text) or re.search(rf'animation:\s*"{row}"', text)
    )


def knockdown_coverage() -> dict[str, list[str]]:
    """sheet target -> the knockdown rows it carries."""
    out: dict[str, list[str]] = {}
    for manifest in sorted(SHEETS.rglob("*_spritesheet.ron")):
        text = manifest.read_text(errors="ignore")
        name = manifest.name[: -len("_spritesheet.ron")]
        out[name] = [row for row in KNOCKDOWN_ROWS if sheet_has_row(text, row)]
    return out


def authored_heights() -> tuple[dict[str, float], list[str]]:
    """(rows that author a height, rows that do not) — per ROW, not per line."""
    if not CATALOG.is_file():
        return {}, []
    authored: dict[str, float] = {}
    silent: list[str] = []
    for chunk in ROW_HEADER.split(CATALOG.read_text(errors="ignore")):
        found = ROW_ID.match(chunk)
        if not found:
            continue
        # ⛔ COMMENTS ARE NOT AUTHORSHIP. A row carrying
        # `// standing_height: Some(99.0),` authors nothing, and counting it
        # would report the mechanism as adopted where somebody only wrote down
        # the idea. Caught by the fixture test, not by reading.
        live = "\n".join(
            line for line in chunk.splitlines() if not line.lstrip().startswith("//")
        )
        height = HEIGHT.search(live)
        if height:
            authored[found.group(1)] = float(height.group(1))
        else:
            silent.append(found.group(1))
    return authored, silent


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args(argv)

    if not SHEETS.is_dir():
        print(
            "NO SPRITE TREE IN THIS CHECKOUT. `assets/sprites` is gitignored\n"
            "generated output; run the regen before reading this as zero coverage.\n"
            "⛔ Absent is not zero."
        )
        return 2

    rows = knockdown_coverage()
    full = sorted(n for n, got in rows.items() if len(got) == len(KNOCKDOWN_ROWS))
    partial = {n: got for n, got in rows.items() if got and len(got) < len(KNOCKDOWN_ROWS)}
    authored, silent = authored_heights()

    if not rows:
        print("⛔ NO SHEETS SCANNED — the manifest glob found nothing.")
        return 2

    if args.json:
        print(json.dumps(
            {"sheets": len(rows), "knockdown_all_four": full,
             "knockdown_partial": partial,
             "standing_height": authored,
             "standing_height_missing": len(silent)}, indent=2, sort_keys=True))
        return 0

    print(f"{len(rows)} sheets scanned\n")
    print(f"carry ALL FOUR knockdown rows: {len(full)}")
    print("   " + ", ".join(full))
    if partial:
        print(f"carry SOME: {len(partial)}")
        for name, got in sorted(partial.items()):
            print(f"   {name:<28} {', '.join(got)}")
    print(
        "\n⚠ A sheet without them falls `knockdown → prone → land_hard → hit → "
        "idle`,\n  so the character plays its HIT pose while lying down — wrong "
        "rather than missing."
    )

    total = len(authored) + len(silent)
    print(f"\ncatalog rows authoring `standing_height`: {len(authored)} of {total}")
    if authored:
        lo = min(authored.values())
        hi = max(authored.values())
        print(f"   values {lo}–{hi} against the {DEFAULT_HEIGHT} default")
        pirates = {n: v for n, v in authored.items() if "pirate" in n}
        if pirates:
            print(f"   pirates with a height: {pirates}")
    quiet_pirates = [n for n in silent if "pirate" in n]
    if quiet_pirates:
        print(
            f"   ⛔ {len(quiet_pirates)} pirate row(s) still fall to "
            f"{DEFAULT_HEIGHT} — the player robot's own height:"
        )
        print("      " + ", ".join(sorted(quiet_pirates)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
