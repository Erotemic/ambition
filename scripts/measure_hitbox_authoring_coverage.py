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

⚠ **CORRECTED BY `d59e982f7`: THE OVERRIDE IS PER CLIP, NOT PER CHARACTER.** The
paragraph above says ten fighters' move tables are dead; measured, the table is a
FALLBACK rather than a dead letter. A clip WITH a bone-derived spec ignores it; a
clip WITHOUT one still plays it — `npc_carl_stargan` is the clean demonstration,
4 of 17 recorded moves matching the authored box exactly and all four on clips
with no `strike` block. ⇒ "author in the `.spec.json`" is the WRONG instruction
for such a move. Everything this script reports is already per-clip, because
`specs()` keeps only specs carrying `strike`.

⛤ **AND `--clock` NEEDS NO RECORDING AT ALL.** Area is a MEASURED quantity, so the
default mode needs a take. `extend` and `inflate` are AUTHORED and the clock is
derived by the exporter, so joining generosity to the ACTIVE window is static and
costs milliseconds:

    moveset_export --out bundle.json
    python3 scripts/measure_hitbox_authoring_coverage.py --clock bundle.json

⛔⛤ **AND IT PRINTS THE PER-CHARACTER SPLIT WHETHER YOU WANT IT OR NOT.** MEASURED
2026-09-12: 7 of the 9 long-active bone-derived moves carry an `inflate`, which
reads as "the long moves are also the generous ones" — and ALL SEVEN ARE THE
PERFORMER'S. Excluding that one fighter the relationship does not weaken, it
inverts to nothing. A pooled band here IS a single fighter unless you look.

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
import statistics
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


#: ⛔ A BAND WHERE ONE CHARACTER HOLDS MORE THAN THIS IS THAT CHARACTER'S NUMBER,
#: NOT THE ROSTER'S. A bare majority, deliberately: the failure this catches is
#: not a subtle skew, it is 7 of 9 rows belonging to one fighter.
DOMINANCE = 0.5

#: ⚠ A BAND THINNER THAN THIS SUPPORTS NO CLAIM IN EITHER DIRECTION. The
#: performer-excluded 10f+ band is n=2; "0 of 2 carry an inflate" is not evidence
#: that the roster has no such relationship, and printing it bare invites exactly
#: that reading.
THIN_BAND = 5

BANDS = ("2-5f", "6-9f", "10f+")


def refuse_unnamed_stages(stages: dict) -> None:
    """⛔ A new sprite stage must be an ERROR, not a silent absence."""
    unknown = sorted(set(stages) - set(STAGE_CHARACTERS))
    if unknown:
        raise SystemExit(
            f"sprite stage(s) {unknown} are not named in STAGE_CHARACTERS. ⇒ REFUSING "
            "TO REPORT: their moves would be silently missing from every table below, "
            "which reads as 'those moves are fine'."
        )


def band_of(active_f: float) -> str:
    """⛔ THE TOP BAND IS `>= 10`, NOT `> 9`, AND THE DIFFERENCE IS SIX MOVES.
    Six authored moves sit at exactly 9.6f, five of them the performer's; counted
    with `> 9` the census reports 24 long moves where the committed figure is 18.
    The edge is load-bearing."""
    if active_f <= 5:
        return "2-5f"
    return "6-9f" if active_f < 10 else "10f+"


def clock_join(bundle_path: pathlib.Path) -> list[dict]:
    """Bone-derived moves with a live window, joined to the knobs that decide them.

    ⛤ NO RECORDING. `extend` and `inflate` are authored in the `.spec.json` and
    `active_f` is derived by the exporter, so every input here is a file already
    in the tree.
    """
    bundle = json.loads(bundle_path.read_text())
    clips = clip_of_move()
    stages = specs()
    refuse_unnamed_stages(stages)
    active: dict[tuple[str, str], float] = {}
    for character in bundle.get("characters", []):
        for move in character.get("moves", []):
            frames = (move.get("derived") or {}).get("active_f")
            # ⛔ A MOVE WITH NO LIVE WINDOW IS A DIFFERENT KIND OF MOVE, not a
            # slow one; folding it in describes the utility moves.
            if frames:
                active[(character.get("id"), str(move.get("id")))] = frames
    rows: list[dict] = []
    for stage, entries in stages.items():
        for character in STAGE_CHARACTERS[stage]:
            for (who, move), frames in active.items():
                if who != character:
                    continue
                clip = clips.get(move)
                # ⚠ PER CLIP, NOT PER CHARACTER (`d59e982f7`): a clip with no
                # bone-derived spec is decided by the move table and does not
                # belong in a census of what the SPEC decides.
                if clip is None or clip not in entries:
                    continue
                hitbox = entries[clip]
                rows.append(
                    {
                        "active_f": frames,
                        "extend": hitbox.get("extend") or 1.0,
                        "inflate": hitbox.get("inflate") or 0.0,
                        "character": character,
                        "move": move,
                        "stage": stage,
                        "clip": clip,
                    }
                )
    return rows


def clock_report(bundle_path: pathlib.Path) -> int:
    rows = clock_join(bundle_path)
    # ⛔ THE ANTI-VACUITY FLOOR, same family as the one in `main`.
    if not rows:
        raise SystemExit(
            "the bundle and the specs joined on NOTHING. ⇒ REFUSING TO REPORT: an "
            "empty table here reads as 'generosity does not track the clock' "
            "rather than 'the export and the specs do not name the same clips'."
        )
    print(
        f"{len(rows)} bone-derived move(s) with a live window, joined to the "
        "spec that decides each one\n"
    )
    grouped: dict[str, list[dict]] = collections.defaultdict(list)
    for row in rows:
        grouped[band_of(row["active_f"])].append(row)
    print(f"{'band':<7}{'n':>4}{'median extend':>15}{'max extend':>12}{'with inflate':>14}")
    for name in BANDS:
        members = grouped.get(name) or []
        if not members:
            continue
        extends = [r["extend"] for r in members]
        inflated = [r for r in members if r["inflate"]]
        print(
            f"{name:<7}{len(members):>4}{statistics.median(extends):>15.3f}"
            f"{max(extends):>12.3f}{len(inflated):>14}"
        )

    # ⛔⛤ THE SPLIT IS NOT OPTIONAL, AND THAT IS THE POINT OF THIS MODE. Pooled,
    # this table says the long-active moves are the generous ones. One fighter
    # owns that finding. Printing the pooled row alone has produced a wrong
    # roster claim three times in this repository.
    print("\nper character, because a pooled band is one fighter until you look:")
    for name in BANDS:
        members = grouped.get(name) or []
        if not members:
            continue
        counted = collections.Counter(r["character"] for r in members)
        who, held = counted.most_common(1)[0]
        spread = " · ".join(f"{c} {n}" for c, n in counted.most_common())
        print(f"  {name:<7} {spread}")
        if held / len(members) > DOMINANCE:
            rest = [r for r in members if r["character"] != who]
            print(
                f"  ⛔ {name}: {who} holds {held} of {len(members)} "
                f"({held / len(members) * 100:.0f}%) — this band is that "
                "fighter's number, not the roster's."
            )
            if len(rest) < THIN_BAND:
                print(
                    f"     without {who}: n={len(rest)} ⇒ TOO THIN TO SUPPORT A "
                    "CLAIM IN EITHER DIRECTION."
                )
            else:
                print(
                    f"     without {who}: n={len(rest)}, "
                    f"{sum(1 for r in rest if r['inflate'])} with inflate"
                )

    long_rows = sorted(
        (r for r in rows if r["active_f"] >= 10),
        key=lambda r: -r["active_f"],
    )
    # ⛔ NAMED, NOT COUNTED. A count that holds while its members change has
    # burned this repository before.
    print(f"\n{len(long_rows)} move(s) run ACTIVE at 10 frames or more, named:")
    for row in long_rows:
        print(
            f"  {row['active_f']:5.1f}f  extend={row['extend']:<5} "
            f"inflate={row['inflate']:<6} {row['character'] + ' ' + row['move']:<44}"
            f" {row['stage']}/specs/{row['clip']}.spec.json"
        )
    return 0


def main() -> int:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 1:
        print(__doc__)
        return 2
    # ⛤ `--clock` takes a `moveset_export` BUNDLE, not a recording.
    if "--clock" in sys.argv[1:]:
        return clock_report(pathlib.Path(args[0]))
    rows = _ratios(pathlib.Path(args[0]))
    clips = clip_of_move()
    stages = specs()
    refuse_unnamed_stages(stages)

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
