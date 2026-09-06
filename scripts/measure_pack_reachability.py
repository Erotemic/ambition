#!/usr/bin/env python3
"""How much of the shared sprite pack can any consumer in the workspace reach?

`assets/sprite_packs/<tier>/` holds an "ultrapack": every sprite target packed
into shared atlas pages, generated at four quality tiers. It is the largest
single body of generated art in the tree.

⛔⛔ THE PACK ROAD HAS EXACTLY ONE PRODUCTION CONSUMER, AND IT IS FOR PROPS.
`build_prop_sprite_asset_packed` (character_sprites/assets.rs) is called from
one place — the intro prop loop in `game/ambition_content/src/intro/plugin.rs`
— and only for rows whose 4th tuple element is `Some(target)`. Characters never
take it: `load_character_sprites_in` goes to the per-target `*_spritesheet.ron`
every time. So a packed target that no prop row opts into is art that is
generated, stored and shipped, and that nothing can ask for.

⭐ REACHABILITY IS A SOURCE FACT; MEGABYTES ARE A MACHINE FACT. The opt-in
table and the single call site are committed Rust and read the same on every
checkout. The page sizes are gitignored generated output — a box that never ran
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
PACKS = REPO / "crates/ambition_platformer2d_actor_monolith/assets/sprite_packs"
PROP_ROWS = REPO / "game/ambition_content/src/intro/sprites.rs"
TIERS = ["full", "half", "quarter", "potato"]

# The 4th element of an `intro_prop_sprite_rows()` tuple: `Some("target"),` on
# its own line, closing the tuple. `None,` rows use the per-target sheet.
OPT_IN_RE = re.compile(r'Some\("([a-z0-9_]+)"\),\s*\)')


def opted_in_targets(source: str) -> set[str]:
    """Targets a prop row asks the pack for.

    ⛔ AN EMPTY RESULT IS A PARSER FAILURE, NOT AN ANSWER. If the table's shape
    changes, matching nothing would report the whole pack unreachable — a
    dramatic finding produced entirely by a broken regex. `main` refuses it.
    """
    return set(OPT_IN_RE.findall(source))


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
    for tier in TIERS:
        catalog_path = PACKS / tier / "ultrapack.json"
        if not catalog_path.exists():
            continue
        catalog = json.loads(catalog_path.read_text())
        used = pages_used_by(catalog, targets)
        rows = []
        for index, name in enumerate(catalog["pages"]):
            page = PACKS / tier / Path(name).name
            if page.exists():
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

    if not PACKS.is_dir():
        print(
            "NO `assets/sprite_packs/` IN THIS CHECKOUT. The packs are gitignored\n"
            "generated output; run the sprite regen before reading this as 0 MB.\n"
            "⛔ Absent is not zero."
        )
        return 2
    if not PROP_ROWS.exists():
        print(f"NO {PROP_ROWS.relative_to(REPO)} — cannot read the opt-in table.")
        return 2

    targets = opted_in_targets(PROP_ROWS.read_text())
    if not targets:
        print(
            "NO PACK OPT-IN ROWS PARSED from intro_prop_sprite_rows(). Either the\n"
            "table's shape changed or the regex is stale. Refusing to report the\n"
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
