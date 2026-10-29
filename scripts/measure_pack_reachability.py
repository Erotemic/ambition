#!/usr/bin/env python3
"""How much of the shared sprite pack can any consumer in the workspace reach?

`assets/sprite_packs/<tier>/` holds an "ultrapack": every sprite target packed
into shared atlas pages, generated at four quality tiers. It is the largest
single body of generated art in the tree.

⛔⛔ THE PACK ROAD HAS EXACTLY ONE PRODUCTION CONSUMER, AND IT IS FOR PROPS.
`build_prop_sprite_asset_packed` (character_sprites/assets.rs) is called from
one place — the prop sheet loader (`load_prop_sheet` in the monolith's
`game_assets`) — and only for a prop sheet row of a pack that states
`packed: true` (`data/prop_sheets.ron`). Characters never take it: `load_character_sprites_in` goes to the per-target `*_spritesheet.ron`
every time. So a packed target that no prop row opts into is art that is
generated, stored and shipped, and that nothing can ask for.

⭐ REACHABILITY IS A SOURCE FACT; MEGABYTES ARE A MACHINE FACT. The opt-in
rows are committed pack data and the single call site is committed Rust; they
read the same on every checkout. The page sizes are gitignored generated output — a box that never ran
regen has no packs at all, and a worktree SYMLINKS the main checkout's copies,
so a second worktree agreeing is not a second source.

⚠ THIS IS NOT A CLAIM THAT THE PACK IS WRONG. Packing every target is what a
packer should do; the finding is that ADOPTION never followed. Whether to
narrow the generator, adopt the pack for characters, or drop the tiers nobody
reads is a decision, not a measurement — this script only says what the reach
is today.

Usage:
    scripts/measure_pack_reachability.py
    scripts/measure_pack_reachability.py --json
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
from lib.content_libraries import runtime_files
TIERS = ["full", "half", "quarter", "potato"]

# A `prop_sheets` row: `"kind": ( fields ),` where a field value can hold one
# level of parentheses (`sheet: Some("target")`).
ROW_RE = re.compile(r'"([a-z0-9_]+)"\s*:\s*\(((?:[^()]|\([^()]*\))*)\)')
SHEET_RE = re.compile(r'sheet\s*:\s*Some\("([a-z0-9_]+)"\)')
PACKED_RE = re.compile(r'packed\s*:\s*true')


def prop_sheet_files() -> list[Path]:
    """Each game's prop sheet rows."""
    return sorted(REPO.glob("game/*/assets/data/prop_sheets.ron"))


def read_prop_rows(files: list[Path] | None = None) -> str:
    """The text of every prop sheet file, one after the other."""
    return "\n".join(path.read_text() for path in (prop_sheet_files() if files is None else files))


def opted_in_targets(source: str) -> set[str]:
    """Targets a prop row asks the pack for: the `sheet` of each row that
    states `packed: true`, or its kind when it names no sheet.

    ⛔ AN EMPTY RESULT IS A PARSER FAILURE, NOT AN ANSWER. If the table's shape
    changes, matching nothing would report the whole pack unreachable — a
    dramatic finding produced entirely by a broken regex. `main` refuses it.
    """
    text = "\n".join(line.split("//", 1)[0] for line in source.splitlines())
    targets = set()
    for kind, body in ROW_RE.findall(text):
        if PACKED_RE.search(body):
            sheet = SHEET_RE.search(body)
            targets.add(sheet.group(1) if sheet else kind)
    return targets


def pages_used_by(catalog: dict, targets: set[str]) -> set[int]:
    """Page indices those targets' frames address, plus page 0.

    Page 0 is always counted: the catalog registers `ultrapack_0.png` per tier
    as the profile-gated entry every consumer resolves before its siblings
    (`extend_with_sprite_pack_entries`), so it loads even when the consumer's
    own frames live elsewhere.
    """
    used = {0}
    for name in targets:
        for frames in catalog.get("targets", {}).get(name, {}).values():
            for frame in frames:
                used.add(frame["page"])
    return used


def measure(targets: set[str]) -> dict:
    tiers = []
    files = runtime_files(REPO)
    for tier in TIERS:
        catalog_path = files.get(f"sprite_packs/{tier}/ultrapack.json")
        if catalog_path is None:
            continue
        catalog = json.loads(catalog_path.read_text())
        used = pages_used_by(catalog, targets)
        rows = []
        for index, name in enumerate(catalog["pages"]):
            page = files.get(f"sprite_packs/{tier}/{Path(name).name}")
            if page is not None:
                rows.append((index, page.stat().st_size))
        tiers.append(
            {
                "tier": tier,
                "packed_targets": len(catalog.get("targets", {})),
                "pages": len(rows),
                "bytes": sum(size for _, size in rows),
                "reachable_pages": len([1 for index, _ in rows if index in used]),
                "reachable_bytes": sum(s for index, s in rows if index in used),
            }
        )
    return {"tiers": tiers, "opted_in": sorted(targets)}


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args(argv)

    if not any(name.startswith("sprite_packs/") for name in runtime_files(REPO)):
        print(
            "NO `assets/sprite_packs/` IN THIS CHECKOUT. The packs are gitignored\n"
            "generated output; run the sprite regen before reading this as 0 MB.\n"
            "⛔ Absent is not zero."
        )
        return 2
    files = prop_sheet_files()
    if not files:
        print("NO game/*/assets/data/prop_sheets.ron — cannot read the opt-in rows.")
        return 2

    targets = opted_in_targets(read_prop_rows(files))
    if not targets:
        print(
            "NO PACK OPT-IN ROWS PARSED from the prop sheet files. Either the\n"
            "rows' shape changed or the regex is stale. Refusing to report the\n"
            "pack unreachable on a parser failure — that would be a huge finding\n"
            "manufactured by a broken match."
        )
        return 2

    out = measure(targets)
    if args.json:
        print(json.dumps(out, indent=2))
        return 0

    packed = max((t["packed_targets"] for t in out["tiers"]), default=0)
    print(
        f"⭐ {len(targets)} target(s) opt into the pack — {', '.join(out['opted_in'])} —\n"
        f"   against {packed} target(s) the pack contains. Characters never take this\n"
        f"   road; `build_prop_sprite_asset_packed` has one call site, for props.\n"
    )
    print(f"{'tier':<9} {'pages':>6} {'MB':>8} {'reach pages':>12} {'reach MB':>9}")
    total = reach = 0
    for row in out["tiers"]:
        total += row["bytes"]
        reach += row["reachable_bytes"]
        print(
            f"{row['tier']:<9} {row['pages']:>6} {row['bytes'] / 1e6:>7.1f} "
            f"{row['reachable_pages']:>12} {row['reachable_bytes'] / 1e6:>8.1f}"
        )
    if total:
        print(
            f"\n{total / 1e6:.1f} MB of pack pages on this machine; {reach / 1e6:.1f} MB "
            f"sits on a page a consumer can reach.\n"
            f"{(total - reach) / 1e6:.1f} MB ({100 * (total - reach) / total:.1f}%) is "
            f"unreachable through every consumer in the workspace.\n"
            "⚠ Megabytes are THIS MACHINE's generated output. The reachability above "
            "is a source fact and holds on any checkout."
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
