#!/usr/bin/env python3
"""Write a fighter's Smash-stage move clocks (`move_timing`) into its facet.

The feel audit (docs/planning/demos/smash-feel-audit-2026-10-29.md, R3)
measured one frame-data shape across the roster: aerials of 17-29 frames with
no landing lag on most fighters, tilts a few frames longer than the jab, and
`3/8/24` dash attacks everywhere. Ultimate's are 32-56 frame aerials with
landing lag, tilts at twice the jab's recovery, and a fast up smash.

A move's frames belong to the game it is played in (the same character's
home game keeps its own pace), so this does not touch the moveset files. It
writes each fighter's `smash_fighter` facet
(`game/ambition_content/assets/data/fighters/<id>.ron`) a `move_timing` block:
per Smash-verb move, the first strike, the last strike, the total and the
landing lag from an archetype template (frames at 60 Hz, authored as seconds),
plus the swing cue the move's class sounds with. The match applies it at
preparation (`ambition_characters::move_damage::move_timing_over`), a
piecewise-linear map of the move's own clock, so hit volumes, events and the
clip keep their phase of the swing.

    python3 scripts/smash_retime.py                 # report every roster fighter
    python3 scripts/smash_retime.py --apply         # write the facets
    python3 scripts/smash_retime.py --fighter goblin --apply

The archetype of each fighter is in ARCHETYPES below, from the Ultimate anchor
its `smash_fighter` facet names (`scripts/smash_fighter_shape.py`). Templates
are first cuts to play, not rulings: frames from Ultimate Frame Data's Mario
(mid), Sheik/Fox (fast), King Dedede/Bowser (heavy), Marth (sword), Captain
Falcon (falcon) and a Yoshi/Samus/Toon Link blend (zoner), rounded; Ultimate
is a reference, not a copy (Jon, 2026-10-29).
"""
from __future__ import annotations

import argparse
import glob
import os
import re
import sys

HZ = 60.0
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# verb -> (first active, last active, total, landing lag or None), in frames.
TEMPLATES = {
    "mid": {
        "attack": (3, 5, 17, None),
        "attack_forward": (5, 8, 29, None),
        "attack_up": (5, 11, 23, None),
        "attack_down": (5, 7, 25, None),
        "attack_dash": (6, 14, 38, None),
        "smash_forward": (15, 18, 46, None),
        "smash_up": (9, 14, 39, None),
        "smash_down": (5, 12, 43, None),
        "attack_air": (3, 24, 42, 6),
        "attack_air_forward": (12, 15, 48, 14),
        "attack_air_back": (6, 12, 33, 6),
        "attack_air_up": (4, 8, 32, 6),
        "attack_air_down": (5, 16, 44, 12),
        "grab": (6, 7, 34, None),
        "grab_dash": (9, 10, 42, None),
    },
    "fast": {
        "attack": (2, 4, 15, None),
        "attack_forward": (5, 8, 24, None),
        "attack_up": (4, 9, 21, None),
        "attack_down": (5, 7, 22, None),
        "attack_dash": (5, 12, 34, None),
        "smash_forward": (13, 16, 42, None),
        "smash_up": (8, 13, 36, None),
        "smash_down": (6, 11, 40, None),
        "attack_air": (3, 22, 38, 5),
        "attack_air_forward": (8, 11, 40, 9),
        "attack_air_back": (6, 10, 30, 6),
        "attack_air_up": (4, 8, 28, 5),
        "attack_air_down": (6, 14, 40, 10),
        "grab": (6, 7, 32, None),
        "grab_dash": (9, 10, 40, None),
    },
    "heavy": {
        "attack": (5, 7, 20, None),
        "attack_forward": (9, 12, 36, None),
        "attack_up": (8, 13, 32, None),
        "attack_down": (7, 10, 30, None),
        "attack_dash": (9, 16, 44, None),
        "smash_forward": (18, 21, 54, None),
        "smash_up": (12, 17, 48, None),
        "smash_down": (8, 14, 50, None),
        "attack_air": (6, 26, 48, 9),
        "attack_air_forward": (14, 17, 54, 16),
        "attack_air_back": (9, 13, 40, 10),
        "attack_air_up": (7, 11, 38, 8),
        "attack_air_down": (10, 20, 52, 18),
        "grab": (7, 8, 38, None),
        "grab_dash": (11, 12, 48, None),
    },
    "sword": {
        # Marth / Lucina: a swordsman's disjoint, slower tilts, a long nair.
        "attack": (5, 6, 22, None),
        "attack_forward": (8, 10, 32, None),
        "attack_up": (6, 12, 30, None),
        "attack_down": (7, 8, 22, None),
        "attack_dash": (12, 15, 42, None),
        "smash_forward": (10, 13, 47, None),
        "smash_up": (13, 16, 50, None),
        "smash_down": (6, 20, 60, None),
        "attack_air": (6, 18, 46, 7),
        "attack_air_forward": (6, 9, 34, 9),
        "attack_air_back": (7, 10, 44, 11),
        "attack_air_up": (5, 8, 32, 8),
        "attack_air_down": (9, 12, 50, 15),
        "grab": (6, 7, 34, None),
        "grab_dash": (9, 10, 40, None),
    },
    "falcon": {
        # Captain Falcon: a quick jab, slow committed smashes, the knee.
        "attack": (3, 4, 17, None),
        "attack_forward": (9, 11, 30, None),
        "attack_up": (17, 19, 37, None),
        "attack_down": (10, 12, 27, None),
        "attack_dash": (7, 15, 37, None),
        "smash_forward": (19, 21, 54, None),
        "smash_up": (22, 26, 57, None),
        "smash_down": (19, 23, 57, None),
        "attack_air": (7, 22, 42, 7),
        "attack_air_forward": (14, 16, 39, 11),
        "attack_air_back": (10, 13, 32, 7),
        "attack_air_up": (7, 10, 33, 6),
        "attack_air_down": (16, 18, 44, 12),
        "grab": (6, 7, 34, None),
        "grab_dash": (9, 10, 42, None),
    },
    "zoner": {
        # Yoshi's body with Samus's and Toon Link's moves: middling frames,
        # a lingering nair, slow grabs.
        "attack": (3, 5, 18, None),
        "attack_forward": (5, 8, 27, None),
        "attack_up": (7, 10, 26, None),
        "attack_down": (6, 8, 28, None),
        "attack_dash": (8, 14, 40, None),
        "smash_forward": (14, 17, 47, None),
        "smash_up": (11, 15, 44, None),
        "smash_down": (7, 13, 46, None),
        "attack_air": (3, 20, 41, 6),
        "attack_air_forward": (11, 14, 44, 12),
        "attack_air_back": (7, 10, 36, 8),
        "attack_air_up": (5, 9, 34, 6),
        "attack_air_down": (8, 16, 48, 14),
        "grab": (9, 10, 44, None),
        "grab_dash": (11, 12, 48, None),
    },

}

ARCHETYPES = {
    "player_robot_v3": "mid",
    "sanic": "fast",
    "npc_pirate_admiral": "heavy",  # King Dedede
    "npc_ninja_shadow_oni_leader": "fast",  # Sheik
    "npc_alice": "mid",  # Palutena
    "npc_bob": "mid",  # Steve
    "npc_oiler": "mid",  # Samus
    "perfect_cellular_automaton": "mid",  # Mewtwo
    "goblin": "fast",  # Diddy Kong
    "npc_emmy_noether": "mid",  # Zelda
    "npc_carl_stargan": "mid",  # Rosalina
    "special_patent_clerk": "mid",  # Dr. Mario
    "npc_gnu_ton": "fast",  # Marth, a light fencer
    "npc_ramen_nujan": "mid",  # Robin
    "pointed_polygon": "sword",  # Marth / Lucina (Jon, 2026-10-29)
    "projectile_polygon": "zoner",  # Yoshi body, Samus / Toon Link moves (Jon, 2026-10-29)
    "pugnacious_polygon": "falcon",  # Captain Falcon (Jon, 2026-10-29)
}

AERIALS = {"attack_air", "attack_air_forward", "attack_air_back", "attack_air_up", "attack_air_down"}
SWINGS = {
    "attack": "swing.light",
    "attack_forward": "swing.light",
    "attack_up": "swing.light",
    "attack_down": "swing.light",
    "attack_dash": "swing.light",
    "smash_forward": "swing.heavy",
    "smash_up": "swing.heavy",
    "smash_down": "swing.heavy",
    "attack_air": "swing.aerial",
    "attack_air_forward": "swing.aerial",
    "attack_air_back": "swing.aerial",
    "attack_air_up": "swing.aerial",
    "attack_air_down": "swing.aerial",
}
FACET_BLOCK = re.compile(r"\n    // SMASH-STAGE MOVE CLOCKS.*?\n    move_timing: \{.*?\n    \},\n(?:    // SMASH-STAGE BASE KNOCKBACK.*?\n    move_knockback: \{.*?\n    \},\n)?", re.S)
# The factor a move's BASE knockback is scaled by on the stage. Measured
# 2026-10-29 (`ko_envelope feel pointed_polygon pugnacious_polygon`): with
# Ultimate's hitstun law the tilts and aerials launched 120-290 px/s at 0%,
# a stun shorter than the swing, so no low-percent hit led anywhere. A
# quarter harder moves the floor of the launch and barely the KO percent
# (growth is untouched). Smashes and jabs keep their own.
# Second pass, same measurement: at 1.25 the forward tilt was still -11 at
# 0%. Up tilt keeps its own base (it was authored as the juggle launcher and
# is +14 at 0% already); the back and up airs are launchers too. First cut
# for the play pass; the KO percents of these moves barely move.
KNOCKBACK = {
    "attack_forward": 1.6,
    "attack_down": 1.6,
    "attack_dash": 1.5,
    "attack_air": 1.5,
    "attack_air_forward": 1.5,
    "attack_air_back": 1.3,
    "attack_air_up": 1.3,
    "attack_air_down": 1.4,
}

MOVE_HEAD = re.compile(r'\(\s*id:\s*"([a-z_0-9]+)",\s*display_name')
VERBS = re.compile(r'verbs:\s*\{(.*?)\}', re.S)
WINDOW = re.compile(r"start_s:\s*([0-9.]+),\s*end_s:\s*([0-9.]+),\s*tag:\s*([A-Za-z]+)")


def verbs_of(text: str) -> dict[str, str]:
    m = VERBS.search(text)
    return dict(re.findall(r'"([a-z_]+)":\s*"([a-z_0-9]+)"', m.group(1))) if m else {}


def moveset_files() -> dict[str, str]:
    """character id -> moveset file that defines it."""
    out = {}
    for path in glob.glob(os.path.join(ROOT, "game/*/assets/data/movesets/*.ron")):
        text = open(path).read()
        for m in re.finditer(r'id:\s*"([a-z_0-9]+)",\s*\n\s*contracts', text):
            out[m.group(1)] = path
    return out


def move_span(text: str, move_id: str) -> tuple[int, int] | None:
    m = re.search(r'\(\s*id:\s*"%s",\s*display_name' % re.escape(move_id), text)
    if not m:
        return None
    rest = text[m.start() + 10 :]
    nxt = MOVE_HEAD.search(rest)
    end = m.start() + 10 + nxt.start() if nxt else len(text)
    return m.start(), end


def frames(x: float) -> int:
    return int(round(x * HZ))


def fmt(seconds: float) -> str:
    s = f"{seconds:.4f}".rstrip("0")
    return s + "0" if s.endswith(".") else s


def retime(body: str, template: tuple[int, int, int, int | None], aerial: bool) -> tuple[str, str, str]:
    """Return (new body, before summary, after summary)."""
    hit, last, total, lag = template
    duration = float(re.search(r"duration_s:\s*([0-9.]+)", body).group(1))
    active = [(float(a), float(b)) for a, b, tag in WINDOW.findall(body) if tag == "Active"]
    if not active:
        return body, "no active window", "unchanged"
    a0, a1 = active[0][0], active[-1][1]
    before = f"{frames(a0)}/{frames(a1)}/{frames(duration)}"
    new_hit, new_last, new_total = hit / HZ, last / HZ, total / HZ

    def f(t: float) -> float:
        if t <= a0:
            return new_hit * (t / a0) if a0 > 0 else 0.0
        if t <= a1:
            return new_hit + (new_last - new_hit) * ((t - a0) / (a1 - a0) if a1 > a0 else 1.0)
        tail = duration - a1
        return new_last + (new_total - new_last) * ((t - a1) / tail if tail > 0 else 1.0)

    def sub_time(m: re.Match) -> str:
        return f"{m.group(1)}{fmt(min(f(float(m.group(2))), new_total))}"

    body = re.sub(r"(\bstart_s:\s*)([0-9.]+)", sub_time, body)
    body = re.sub(r"(\bend_s:\s*)([0-9.]+)", sub_time, body)
    body = re.sub(r"(\bat_s:\s*)([0-9.]+)", sub_time, body)
    body = re.sub(r"(\bautocancel_after_s:\s*Some\()([0-9.]+)", sub_time, body)
    body = re.sub(r"(\bduration_s:\s*)([0-9.]+)", lambda m: f"{m.group(1)}{fmt(new_total)}", body)
    if aerial and lag is not None:
        body = re.sub(r"landing_lag_s:\s*(Some\([0-9.]+\)|None)", f"landing_lag_s: Some({fmt(lag / HZ)})", body, count=1)
        # Landing in the last two fifths of the move is clean: the swing is over.
        autocancel = new_last + 0.6 * (new_total - new_last)
        body = re.sub(
            r"autocancel_after_s:\s*(Some\([0-9.]+\)|None)",
            f"autocancel_after_s: Some({fmt(autocancel)})",
            body,
            count=1,
        )
    after = f"{hit}/{last}/{total}" + (f" L{lag}" if aerial and lag is not None else "")
    return body, before, after


def facet_block(rows: list[tuple[str, str, tuple[int, int, int, int | None]]]) -> str:
    lines = [
        "",
        "    // SMASH-STAGE MOVE CLOCKS: first strike, last strike, total and landing",
        "    // lag, in seconds, and the swing cue. Written by",
        "    // `scripts/smash_retime.py` from an archetype template; the moveset keeps",
        "    // the home game's pace. Frames at 60 Hz are in the trailing comment.",
        "    move_timing: {",
    ]
    for verb, move_id, (hit, last, total, lag) in rows:
        parts = [f"hit_s: {fmt(hit / HZ)}", f"last_s: {fmt(last / HZ)}", f"total_s: {fmt(total / HZ)}"]
        if verb in AERIALS and lag is not None:
            parts.append(f"landing_lag_s: Some({fmt(lag / HZ)})")
        swing = SWINGS.get(verb)
        if swing:
            parts.append(f'swing: Some("{swing}")')
        frames_note = f"{hit}/{last}/{total}" + (f" L{lag}" if verb in AERIALS and lag is not None else "")
        lines.append(f'        // {verb}: {frames_note}')
        lines.append(f'        "{move_id}": ({", ".join(parts)}),')
    lines += ["    },"]
    kb = [(verb, move_id) for verb, move_id, _ in rows if verb in KNOCKBACK]
    if kb:
        lines += [
            "    // SMASH-STAGE BASE KNOCKBACK: the factor each move's base launch is",
            "    // scaled by here, so a tilt or an aerial at 0% stuns past its own",
            "    // swing. Written by `scripts/smash_retime.py`.",
            "    move_knockback: {",
        ]
        for verb, move_id in kb:
            lines.append(f'        "{move_id}": {KNOCKBACK[verb]},')
        lines += ["    },"]
    lines += [""]
    return "\n".join(lines)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--apply", action="store_true", help="write the facet files")
    ap.add_argument("--fighter", action="append", help="only these character ids (repeatable)")
    args = ap.parse_args()
    files = moveset_files()
    wanted = args.fighter or list(ARCHETYPES)
    written = 0
    for cid in wanted:
        arche = ARCHETYPES.get(cid)
        path = files.get(cid)
        facet = os.path.join(ROOT, f"game/ambition_content/assets/data/fighters/{cid}.ron")
        if not arche or not path:
            print(f"{cid}: no archetype or moveset, skipped", file=sys.stderr)
            continue
        if not os.path.exists(facet):
            print(f"{cid}: no smash_fighter facet, skipped", file=sys.stderr)
            continue
        text = open(path).read()
        facet_text = open(facet).read()
        facet_verbs = verbs_of(facet_text)
        own = re.search(r'id:\s*"%s",\s*\n\s*contracts(.*?)moves:' % re.escape(cid), text, re.S)
        own_verbs = verbs_of(own.group(0)) if own else {}
        print(f"== {cid} ({arche})")
        rows: list[tuple[str, str, tuple[int, int, int, int | None]]] = []
        done: set[str] = set()
        for verb, template in TEMPLATES[arche].items():
            move_id = facet_verbs.get(verb) or own_verbs.get(verb)
            if not move_id or move_id in done:
                continue
            span = move_span(text, move_id)
            if not span:
                print(f"  {verb:20s} -> {move_id}: not in file", file=sys.stderr)
                continue
            s, e = span
            _, before, after = retime(text[s:e], template, verb in AERIALS)
            print(f"  {verb:20s} {move_id:28s} {before:>10s} -> {after}")
            rows.append((verb, move_id, template))
            done.add(move_id)
        if not args.apply or not rows:
            continue
        block = facet_block(rows)
        if FACET_BLOCK.search(facet_text):
            facet_text = FACET_BLOCK.sub(lambda _m: block, facet_text, count=1)
        else:
            close = facet_text.rstrip().rfind(")")
            facet_text = facet_text[:close].rstrip("\n") + "\n" + block + ")\n"
        tmp = facet + ".tmp"
        with open(tmp, "w") as fh:
            fh.write(facet_text)
        os.replace(tmp, facet)
        written += 1
    if args.apply:
        print(f"wrote {written} facet(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
