#!/usr/bin/env python3
"""Which crates does the FEATURELESS facade profile pull in, and what would
cutting each one actually remove?

⭐⭐ **A9 HAS BEEN HELD FOR THIS EXACT QUESTION.** The packet says so in its own
words: *"The 49 crates that remain have been traced to their activating parents;
what has not been established is which of them a minimum profile has a right to
expect."* A list of 49 names does not let anybody rule on that. What does is the
COST OF EACH CUT: if `ambition_audio` left the closure, how many other crates
would leave with it, and which ones are shared with everything else and so cannot
leave at all?

⛔⛔ **THE FEATURE-RESOLVED TREE, NEVER THE MANIFEST WALK.** A manifest walk
counts OPTIONAL edges, so it reports crates a featureless build does not link —
`the-featureless-facade-links-none-of-these` exists because of that difference.
This asks cargo, with the same flags that contract uses.

⇒ EXCLUSIVE SUBTREE is the number to read. A crate whose exclusive subtree is
just itself is one edge away from leaving; a crate with a large exclusive subtree
is a whole region. A crate that is NOT exclusive to anything is load-bearing for
several parents at once and cutting one parent does nothing.

⚠ IT RULES ON NOTHING. "Has a right to expect" is Jon's call; this says what
each answer would cost.

    python3 scripts/measure_minimum_profile_closure.py
    python3 scripts/measure_minimum_profile_closure.py --root ambition_platformer2d
"""
from __future__ import annotations

import argparse
import collections
import os
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
CARGO = pathlib.Path.home() / ".cargo" / "bin" / "cargo"
# ⛔⛤ THE PREFIX IS BOX-DRAWING, NOT ASCII. My first version matched `[ |`+-]*`
# and every line failed, which the anti-vacuity floor below caught by reporting a
# closure of ZERO — an ASCII assumption about a tool's output is a guess, and the
# floor is what turns a guess into a refusal instead of an empty table.
LINE = re.compile(r"^([\u2502\u251c\u2514\u2500 ]*)([A-Za-z0-9_-]+) v")


def tree(root: str) -> list[str]:
    """`cargo tree` for the featureless facade profile, as raw lines."""
    cargo = str(CARGO) if CARGO.exists() else "cargo"
    out = subprocess.run(
        [
            cargo,
            "tree",
            "-e",
            "normal",
            "--no-default-features",
            "-p",
            root,
        ],
        cwd=REPO,
        capture_output=True,
        text=True,
        env={**os.environ, "CARGO_TERM_COLOR": "never"},
    )
    if out.returncode != 0:
        raise SystemExit(f"cargo tree failed:\n{out.stderr}")
    return out.stdout.splitlines()


def edges(lines: list[str], prefix: str) -> tuple[set[str], set[tuple[str, str]]]:
    """Every `prefix`-named crate in the tree, and every parent->child edge
    between two of them.

    ⛔ THE STACK SPANS NON-MATCHING CRATES. A third-party crate between two of
    ours is not an edge to record, but it IS a level of depth: collapsing it
    would attach a grandchild to the wrong parent. The stack keeps every level
    and only the `prefix` ones are emitted.
    """
    nodes: set[str] = set()
    found: set[tuple[str, str]] = set()
    stack: list[tuple[int, str | None]] = []
    for line in lines:
        m = LINE.match(line)
        if not m:
            continue
        depth = len(m.group(1)) // 4
        name = m.group(2)
        mine = name.startswith(prefix)
        while len(stack) > depth:
            stack.pop()
        if len(stack) < depth:
            # A depth jump means the line shape changed under us.
            raise SystemExit(f"unexpected depth {depth} after {len(stack)}: {line!r}")
        parent = next((n for _, n in reversed(stack) if n is not None), None)
        if mine:
            nodes.add(name)
            if parent is not None and parent != name:
                found.add((parent, name))
        stack.append((depth, name if mine else None))
    return nodes, found


def reachable(root: str, out: dict[str, set[str]], without: str | None) -> set[str]:
    """Everything reachable from `root`, with `without` (and its edges) cut."""
    seen: set[str] = set()
    stack = [root]
    while stack:
        here = stack.pop()
        for child in out.get(here, ()):
            if child == without or child in seen:
                continue
            seen.add(child)
            stack.append(child)
    return seen


def report_minimum(args) -> int:
    """What a named capability set costs, as its own featureless closure.

    ⭐⭐ **THIS IS THE INSTRUMENT A RULING NEEDS, AND SUBTRACTION IS NOT.** A
    census that asks "what does cutting X remove" reports NOTHING for almost
    every X here, because the facade names 42 crates directly AND the hubs reach
    the same 42 from underneath — each crate is held by two roads, so cutting
    either alone changes the profile not at all. MEASURED: cutting `provider`,
    `host`, `runtime`, `actor_monolith` and `sim_view` together takes 49 to 44.
    ⇒ Ask what a profile COSTS, not what a cut saves.
    """
    if not args.minimum:
        raise SystemExit("--minimum needs at least one crate to price")
    closure: set[str] = set()
    per: list[tuple[int, str]] = []
    for crate in args.minimum:
        nodes, _ = edges(tree(crate), args.prefix)
        per.append((len(nodes), crate))
        closure |= nodes
    whole, _ = edges(tree(args.root), args.prefix)
    print(f"the capability set costs {len(closure)} {args.prefix}* crate(s)")
    print(f"  against {len(whole)} for the featureless {args.root} profile\n")
    for size, crate in sorted(per, reverse=True):
        print(f"  {size:>3}  {crate}")
    print("\nIN THE SET\n  " + "\n  ".join(sorted(closure)))
    absent = sorted(whole - closure)
    print(f"\nNOT IN IT ({len(absent)} of the current profile)\n  " + "\n  ".join(absent))
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--root", default="ambition_platformer2d")
    ap.add_argument(
        "--minimum",
        nargs="*",
        default=None,
        metavar="CRATE",
        help="report the closure of THIS capability set instead of the root's — "
        "the instrument for ruling on what a minimum profile costs",
    )
    ap.add_argument("--prefix", default="ambition")
    args = ap.parse_args()

    if args.minimum is not None:
        return report_minimum(args)

    nodes, found = edges(tree(args.root), args.prefix)
    if args.root not in nodes:
        raise SystemExit(f"{args.root} is not in its own tree — the parse is wrong")

    out: dict[str, set[str]] = collections.defaultdict(set)
    incoming: dict[str, set[str]] = collections.defaultdict(set)
    for parent, child in found:
        out[parent].add(child)
        incoming[child].add(parent)

    whole = reachable(args.root, out, None)
    # ⛔ THE FLOOR. An empty or near-empty closure would make every "cutting X
    # removes 0" below trivially true, which is this repository's most repeated
    # instrument failure.
    if len(whole) < 20:
        raise SystemExit(
            f"only {len(whole)} crate(s) reachable from {args.root}: the tree "
            "parse is not seeing the closure, so every cut below would cost nothing"
        )

    rows = []
    for crate in sorted(whole):
        left = whole - reachable(args.root, out, crate) - {crate}
        rows.append((len(left) + 1, crate, sorted(left), sorted(incoming[crate])))
    rows.sort(key=lambda r: (-r[0], r[1]))

    print(f"{len(whole)} {args.prefix}* crate(s) in the featureless closure of {args.root}\n")
    print(f"{'cut cost':>8}  {'crate':<44} leaves with it")
    print(f"{'-' * 8}  {'-' * 44} {'-' * 30}")
    for cost, crate, left, _ in rows:
        tail = ", ".join(c.replace("ambition_", "") for c in left) if left else "—"
        print(f"{cost:>8}  {crate:<44} {tail}")

    print("\nDIRECT PARENTS (who activates each)\n")
    for _, crate, _, parents in sorted(rows, key=lambda r: r[1]):
        who = ", ".join(p.replace("ambition_", "") for p in parents) or "(the root)"
        print(f"  {crate:<44} {who}")

    solo = [c for cost, c, _, _ in rows if cost == 1]
    print(
        f"\n{len(solo)} crate(s) are ONE EDGE from leaving (cut cost 1) and "
        f"{len(rows) - len(solo)} carry a region with them."
    )

    # ── what the ROOT's own manifest decides ────────────────────────────────
    direct = out.get(args.root, set())
    # ⛔⛤ **ONE EDGE CUT, NOT ONE CRATE REMOVED — AND MY FIRST VERSION WAS
    # UNFALSIFIABLE.** It unioned `reachable(child) | {child}` over the direct
    # children and subtracted: every child is in its own set, so the difference
    # was ALWAYS empty and "0 crates depend on this edge alone" printed no matter
    # what the tree said. A poison that removed every non-root edge still printed
    # 0, which is what exposed it. ⇒ Ask the real question per child: with the
    # root's OWN edge to it cut, is it still reachable from the root?
    only_root = []
    for child in sorted(direct):
        trimmed = {p: set(cs) for p, cs in out.items()}
        trimmed[args.root] = trimmed[args.root] - {child}
        if child not in reachable(args.root, trimmed, None):
            only_root.append(child)
    print(
        f"\n{len(direct)} of the {len(whole)} are named DIRECTLY by "
        f"{args.root}; {len(only_root)} are in the closure ONLY because of "
        "that edge."
    )
    for child in only_root:
        print(f"      · {child}")
    if not only_root:
        print(
            "  ⇒ EVERY direct edge is REDUNDANT with a path from underneath, so "
            "no edit to this manifest alone\n     can shrink the profile. The "
            "closure is decided by whichever child already pulls the rest."
        )
    else:
        print(
            f"  ⇒ The other {len(direct) - len(only_root)} direct edges are "
            "REDUNDANT with a path from underneath: removing one\n     from this "
            "manifest changes the profile not at all."
        )

    # ── each direct child's OWN featureless closure ─────────────────────────
    #  THE NUMBER A RULING NEEDS. "Which crates may a minimum expect" is
    # unanswerable while one unconditional edge already brings the whole set;
    # what a ruling can act on is how large each entry point is on its own.
    print("\nWHAT EACH DIRECT CHILD COSTS ON ITS OWN (its own featureless closure)\n")
    sizes = []
    for child in sorted(direct):
        try:
            nodes, _ = edges(tree(child), args.prefix)
        except SystemExit as refusal:
            print(f"  {child:<44} ? ({refusal})")
            continue
        sizes.append((len(nodes), child))
    sizes.sort(reverse=True)
    for size, child in sizes:
        share = 100.0 * size / (len(whole) + 1)
        print(f"  {size:>3} crate(s)  {share:5.1f}% of the profile   {child}")
    if sizes:
        biggest, who = sizes[0]
        print(
            f"\n⇒ {who} alone accounts for {biggest} of the "
            f"{len(whole) + 1}-crate profile. A minimum profile's real question "
            "is whether THAT edge\n  belongs in it, not which leaves to prune."
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
