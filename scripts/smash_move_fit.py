#!/usr/bin/env python3
"""Fit one Ambition move's launch to its Ultimate anchor, relative to a calibrated move.

A fighter's smash attacks are calibrated against Ultimate's KO percents in
Ambition's own launch law (``ambition_entity_catalog::launch::launch_speed``:
``speed = base_scale*base + percent_scale*growth*percent/weight``). Another move
of the same fighter is placed by the ratio Ultimate's own knockback formula
gives between the anchor fighter's two moves, at every victim percent:

    target(p) = ours(reference, p) * KB_ultimate(move, p) / KB_ultimate(reference, p)

and fitted as a line through p = 0 and p = ``--fit-pct``. Ultimate's knockback
is linear in percent for a fixed hit, so the fit is exact wherever our
reference is proportional to its anchor.

Ultimate's numbers come from ``docs/related-work/data/smash_ultimate_move_hitboxes.csv``
(``scripts/fetch_smash_ultimate_moves.py``). Each move is represented by its
strongest hitbox at 100% on a weight-100 victim, which is the clean hit of a
sweetspotted move and the launcher of a multi-hit one, evaluated at the percent
the move's linking hits carried the victim to (``linking_damage``).
Fixed-knockback rows are not launchers.

Ambition's numbers are read from the moveset RON named on the command line:
the strongest volume of the move bound to ``--ref-verb``.

Usage::

    python3 scripts/smash_move_fit.py --moveset goblin --anchor "Diddy Kong" \\
        --verb attack_up --anchor-move Up_tilt --ref-verb smash_up --ref-anchor-move Up_smash
"""
import argparse
import csv
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TABLE = ROOT / "docs/related-work/data/smash_ultimate_move_hitboxes.csv"
MOVESETS = ROOT / "game/ambition_content/assets/data/movesets"
ROOM_RULES = ROOT / "game/ambition_demo_smash/assets/data/room_rules.ron"


def kb_ultimate(row, pct, weight=100.0):
    """Ultimate's knockback for one hitbox on a victim at `pct` before the hit."""
    d = float(row["damage"])
    p = pct + d
    return ((p / 10 + p * d / 20) * 200 / (weight + 100) * 1.4 + 18) * float(row["ks"]) / 100 + float(row["bk"])


def strongest(rows):
    live = [r for r in rows if r["ks"] and r["bk"] and float(r.get("fkv") or 0) == 0]
    return max(live, key=lambda r: kb_ultimate(r, 100.0)) if live else None


def linking_damage(rows):
    """The damage a multi-hit move deals before its launcher connects.

    A page lists a move's hits in order, each hit's hitboxes numbered from id 0,
    so an id that restarts at 0 starts the next group. A group whose every row
    is SET knockback (`fkv` > 0) is a linking hit that carries the victim into
    the launcher; its damage is added once. A group of growing rows is the
    launcher, or another version of it (a clean and a late hit), so it adds
    nothing.
    """
    groups, current = [], []
    for r in rows:
        if r["id"] == "0" and current:
            groups.append(current)
            current = []
        current.append(r)
    if current:
        groups.append(current)
    return sum(
        max(float(r["damage"]) for r in g)
        for g in groups
        if all(float(r.get("fkv") or 0) > 0 for r in g)
    )


def ultimate_move(fighter, move):
    """(launcher hitbox, damage the linking hits add before it)."""
    with TABLE.open() as fh:
        rows = [r for r in csv.DictReader(fh) if r["fighter"] == fighter and r["move"] == move]
    best = strongest(rows)
    if best is None:
        raise SystemExit(f"no knockback rows for {fighter} {move} in {TABLE.name}")
    return best, linking_damage(rows)


def rule(name):
    m = re.search(rf"{name}:\s*(?:Some\()?([\d.]+)", ROOM_RULES.read_text())
    return float(m.group(1)) if m else 1.0


def move_block(text, verb, move_id=None):
    if move_id is None:
        m = re.search(rf'"{verb}"\s*:\s*"([^"]+)"', text)
        if not m:
            raise SystemExit(f"no verb {verb}")
        move_id = m.group(1)
    start = text.find(f'id: "{move_id}"')
    if start < 0:
        raise SystemExit(f"no move {move_id}")
    nxt = re.search(r"\n\s*\(\n\s*id: \"", text[start + 10:])
    end = start + 10 + nxt.start() if nxt else len(text)
    return move_id, start, end


def volumes(text, start, end):
    pat = re.compile(r"knockback:\s*([\d.]+)(,\s*)knockback_growth:\s*Some\(([\d.]+)\)")
    return [(m.start(), m.end(), float(m.group(1)), float(m.group(3)), m.group(2)) for m in pat.finditer(text, start, end)]


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--moveset", required=True)
    ap.add_argument("--anchor", required=True)
    ap.add_argument("--verb", required=True)
    ap.add_argument("--anchor-move", required=True)
    ap.add_argument("--ref-verb", required=True)
    ap.add_argument("--ref-anchor-move", required=True)
    ap.add_argument("--move-id", help="the move to fit, where the verb's first binding is not it "
                    "(a fighter facet's `verbs` name Smash moves the home verbs do not)")
    ap.add_argument("--fit-pct", type=float, default=150.0)
    ap.add_argument("--write", action="store_true", help="rewrite the move's volumes in the RON")
    args = ap.parse_args()

    base_scale = rule("base_knockback_scale")
    pct_scale = rule("victim_percent_knockback_scale")
    path = MOVESETS / f"{args.moveset}.ron"
    text = path.read_text()

    ref_id, rs, re_ = move_block(text, args.ref_verb)
    ref_vols = volumes(text, rs, re_)
    _, _, ref_base, ref_growth, _ = max(ref_vols, key=lambda v: base_scale * v[2] + pct_scale * v[3] * 100)

    def ours(base, growth, p):
        return base_scale * base + pct_scale * growth * p

    (u_move, move_link), (u_ref, ref_link) = (
        ultimate_move(args.anchor, args.anchor_move),
        ultimate_move(args.anchor, args.ref_anchor_move),
    )

    def ratio(p):
        # Each launcher lands at the percent its own linking hits carried the victim to.
        return kb_ultimate(u_move, p + move_link) / kb_ultimate(u_ref, p + ref_link)

    target = {p: ours(ref_base, ref_growth, p) * ratio(p) for p in (0.0, args.fit_pct)}
    new_base = target[0.0] / base_scale
    new_growth = (target[args.fit_pct] - target[0.0]) / (pct_scale * args.fit_pct)

    move_id, ms, me = move_block(text, args.verb, args.move_id)
    vols = volumes(text, ms, me)
    _, _, old_base, old_growth, _ = max(vols, key=lambda v: base_scale * v[2] + pct_scale * v[3] * 100)
    print(f"{args.moveset} {args.verb}={move_id} vs {args.ref_verb}={ref_id}  anchor {args.anchor}")
    print(f"  ultimate {args.anchor_move}: d{u_move['damage']} bk{u_move['bk']} ks{u_move['ks']}  "
          f"{args.ref_anchor_move}: d{u_ref['damage']} bk{u_ref['bk']} ks{u_ref['ks']}"
          f"  (linking damage {move_link:g} / {ref_link:g})")
    for p in (0, 50, 100, 150, 200):
        print(f"  {p:3d}%  ref {ours(ref_base, ref_growth, p):6.0f}  old {ours(old_base, old_growth, p):6.0f}"
              f"  new {ours(new_base, new_growth, p):6.0f}  (ultimate ratio {ratio(p):.2f})")
    print(f"  strongest volume: knockback {old_base:g} -> {new_base:.1f}, growth {old_growth:g} -> {new_growth:.2f}")
    if args.write:
        # Every volume of the move keeps its proportion to the strongest one.
        kb_k, g_k = new_base / old_base, new_growth / old_growth
        out, last = [], 0
        for s, e, b, g, sep in vols:
            out.append(text[last:s])
            out.append(f"knockback: {b * kb_k:.1f}{sep}knockback_growth: Some({g * g_k:.2f})")
            last = e
        out.append(text[last:])
        path.write_text("".join(out))
        print(f"  wrote {len(vols)} volume(s) of {move_id} in {path.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
