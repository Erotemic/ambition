#!/usr/bin/env python3
"""Which authoring surface decides each fighter's strike box, and has it ever been tuned?

⛔⛤ **THE MOVE TABLE IS NOT THE ANSWER FOR MOST FIGHTERS.** MEASURED 2026-09-12:
six fighters play the `half_extents` authored in `data/movesets/*.ron` EXACTLY;
ten do not, because they have a rendered sprite stage whose spec DERIVES the
hitbox from a bone segment (`"strike": {"base": "near_arm_l", "tip":
"near_arm_hand"}`). medic's forward tilt hitbox is her forearm, extended 8% — the
"3.8 px tall against a 48 px body" that had been read as a stingy box is the
thickness of her arm, and no edit to the move table can change it.

⇒ For those ten the generosity surface is `hitbox.inflate` in the `.spec.json`,
and this script answers the two questions that follow: which specs have never
carried one, and which recorded moves they decide.

    moveset_takes --characters grid --out takes.json     # ~30 min, every verb
    python3 scripts/measure_hitbox_authoring_coverage.py takes.json

⚠ **A SPEC IS NOT A MOVE.** `fighting_brawler_v1` is shared by four characters, so
one edit moves up to eight recorded moves at once — and a shared spec is a shared
RULE (which bones, how far to extend) applied to each character's own skeleton,
not a shared box. The `--coupling` table is the one to read before editing.

⚠ `inflate` IS ABSOLUTE PIXELS (`_grow_hull` pushes every vertex that far from the
hull's centre); `extend` is a multiplier on the bone's length. And an edit reaches
the game only after the sprite is RE-PUBLISHED — the runtime reads published sheet
metadata, not the spec.
"""

from __future__ import annotations

import collections
import importlib.util
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
STAGES = REPO / "tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/motion/humanoid"
TABLES = REPO / "game/ambition_content/assets/data/movesets"

# ⛔ DERIVED FROM THE TREE, NOT HAND-LISTED, for the character->table half. The
# stage->character half has no machine-readable source yet and is stated here;
# `--check` refuses a stage this map does not name, so a new stage is an error
# rather than a silently missing row.
STAGE_CHARACTERS = {
    "medic_triage_v1": ["medic"],
    "officer_brawler_v1": ["officer"],
    "performer_stage_v1": ["performer"],
    "projectile_beast_v1": ["projectile_polygon"],
    "fighting_brawler_v1": ["npc_alice", "npc_bob", "npc_carl_stargan", "npc_emmy_noether"],
    "author_pen_v1": ["author"],
    "fighting_polygon_v1": ["pointed_polygon", "pugnacious_polygon"],
}


def clip_of_move() -> dict[str, str]:
    """Move id -> the clip it plays, read from the authored tables."""
    out: dict[str, str] = {}
    for path in sorted(TABLES.glob("*.ron")):
        text = path.read_text()
        for match in re.finditer(
            r'id: "([^"]+)",\n\s+display_name: [^\n]*\n\s+clip: \(\n\s+clip: "([^"]+)"', text
        ):
            out[match.group(1)] = match.group(2)
    return out


def specs() -> dict[str, dict[str, dict]]:
    """Stage -> clip -> the spec's `hitbox` block, for BONE-DERIVED specs only."""
    out: dict[str, dict[str, dict]] = {}
    for stage in sorted(p for p in STAGES.iterdir() if p.is_dir()):
        rows: dict[str, dict] = {}
        for path in sorted((stage / "specs").glob("*.spec.json")):
            spec = json.loads(path.read_text())
            if "strike" not in spec:
                continue
            clip = spec.get("clip", path.stem.replace(".spec", ""))
            rows[clip] = spec.get("hitbox") or {}
        out[stage.name] = rows
    return out


def _ratios(take_path: pathlib.Path):
    loader = importlib.util.spec_from_file_location(
        "measure_strike_area_over_body", REPO / "scripts" / "measure_strike_area_over_body.py"
    )
    module = importlib.util.module_from_spec(loader)
    loader.loader.exec_module(module)
    return module.per_move(json.loads(take_path.read_text()))


def main() -> int:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 1:
        print(__doc__)
        return 2
    rows = _ratios(pathlib.Path(args[0]))
    clips = clip_of_move()
    stages = specs()

    unknown = sorted(set(stages) - set(STAGE_CHARACTERS))
    if unknown:
        raise SystemExit(
            f"sprite stage(s) {unknown} are not named in STAGE_CHARACTERS. ⇒ REFUSING "
            "TO REPORT: their moves would be silently missing from every table below, "
            "which reads as 'those moves are fine'."
        )

    print(f"{'stage':<24} {'specs':>5} {'bone-derived':>13} {'with inflate':>13}")
    for stage, entries in sorted(stages.items()):
        total = len(list((STAGES / stage / 'specs').glob('*.spec.json')))
        inflated = sum(1 for h in entries.values() if h.get("inflate"))
        print(f"  {stage:<22} {total:>5} {len(entries):>13} {inflated:>13}")

    joined: list[tuple[float, str, str, str, str]] = []
    per_spec: dict[str, list[tuple[float, str, str]]] = collections.defaultdict(list)
    for stage, entries in stages.items():
        for character in STAGE_CHARACTERS[stage]:
            for (who, move), (ratio, _strike, _body) in rows.items():
                if who != character:
                    continue
                clip = clips.get(move)
                if clip is None or clip not in entries:
                    continue
                per_spec[f"{stage}/{clip}"].append((ratio, character, move))
                if not entries[clip].get("inflate"):
                    joined.append((ratio, character, move, stage, clip))
    joined.sort()

    # ⛔ THE ANTI-VACUITY FLOOR. An empty join reads as "every move is tuned".
    if not joined:
        raise SystemExit(
            "the take and the specs joined on NOTHING. ⇒ REFUSING TO REPORT: an "
            "empty table here reads as 'no move is missing an inflate' rather "
            "than 'the recording and the specs do not name the same clips'."
        )

    thin = sum(1 for row in joined if row[0] < 1.0)
    print(f"\n{len(joined)} recorded move(s) are bone-derived with NO inflate; {thin} play THIN.\n")
    for ratio, character, move, stage, clip in joined[:20]:
        print(f"  {ratio:5.2f}  {character + ' ' + move:<44} {stage}/specs/{clip}.spec.json")

    shared = {k: v for k, v in per_spec.items() if len(v) > 1}
    print(f"\n{len(shared)} of {len(per_spec)} specs decide MORE THAN ONE recorded move:\n")
    for key, entries in sorted(shared.items(), key=lambda kv: -len(kv[1]))[:8]:
        spread = " · ".join(f"{c} {r:.2f}" for r, c, _ in sorted(entries)[:4])
        print(f"  {key:<42} {len(entries)} moves: {spread}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
