#!/usr/bin/env python3
"""WHY is each crate in the A9 minimum profile's closure — which parent puts it there?

⭐⭐ **A9's OPEN QUESTION IS "WHICH OF THE 49 A MINIMUM PROFILE HAS A RIGHT TO
EXPECT", AND A COUNT CANNOT ANSWER IT.** The row establishes that the featureless
facade reaches 49 workspace crates and that every remaining one has two or more
parents, so no single-edge decrement exists. That is a statement about the graph.
**What nobody has recorded is the SHAPE of each crate's parentage**, which is the
difference between "the consumer asked for this" and "a hub it could not decline
brought it".

⛔⛔ **THIS IS A CLASSIFICATION, NOT A CARVE LIST, AND THE ROW SAYS WHY IN ITS OWN
WORDS:** *"A closure measurement cannot say which ownership change is
semantically correct — the `actor_spawn` carve is this repository's own receipt
for that, where a green SCC number sat beside a live view the extraction had taken
with it."* ⇒ Nothing here licenses moving a crate. It says which crates are
reached ONLY through the hubs, so that a later ownership question has a subject.

⚠ **IT MEASURES COMPILER REACHABILITY ONLY — the first of the three axes the row
names** (*"Separate compiler reachability, runtime installation and public-import
ergonomics"*). A crate can be linked and install nothing. That second axis needs
the fixture to RUN and is not what this asks.

    python3 scripts/measure_minimum_profile_parentage.py
    python3 scripts/measure_minimum_profile_parentage.py --json
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
CARGO = str(pathlib.Path.home() / ".cargo" / "bin" / "cargo")
FACADE = "ambition_platformer2d"

# The crates the row identifies as PARENTS OF NEARLY EVERYTHING. Named here
# rather than derived so the classification is auditable: the row says
# "`ambition_platformer2d_runtime` and `ambition_platformer2d_actor_monolith` are
# parents of nearly everything, and `ambition_platformer2d_core` has 33 parents".
# `provider`, `host` and `sim_view` join them because the facade's own composition
# names them and a consumer selecting no capability cannot decline any of the six.
HUBS = {
    "ambition_platformer2d_runtime",
    "ambition_platformer2d_actor_monolith",
    "ambition_platformer2d_core",
    "ambition_platformer2d_provider",
    "ambition_platformer2d_host",
    "ambition_sim_view",
}

NAME = re.compile(r"(ambition_[a-z0-9_]+) v")
DECL = re.compile(r"^(ambition_[a-z0-9_]+)\s*=")


def facade_declared() -> tuple[set[str], set[str]]:
    """(non-optional, optional) `ambition_*` deps declared by the facade itself."""
    text = (REPO / "crates" / FACADE / "Cargo.toml").read_text()
    body = text.split("[dependencies]", 1)[1].split("\n[", 1)[0]
    opt, req = set(), set()
    for line in body.splitlines():
        m = DECL.match(line.strip())
        if m:
            (opt if "optional = true" in line or "optional=true" in line else req).add(m.group(1))
    return req, opt


def tree(*args: str) -> str:
    out = subprocess.run(
        [CARGO, "tree", "-e", "normal", "--no-default-features", "-p", FACADE, *args],
        cwd=str(REPO), capture_output=True, text=True,
    )
    if out.returncode != 0:
        sys.exit(f"cargo tree failed: {out.stderr.strip()[:400]}")
    return out.stdout


def closure() -> list[str]:
    found = {m for m in NAME.findall(tree())}
    return sorted(found - {FACADE})


def parents(crate: str) -> list[str]:
    lines = tree("-i", crate, "--depth", "1").splitlines()
    # Line 0 is the crate itself; the rest are its direct dependents.
    out = []
    for line in lines[1:]:
        m = NAME.search(line)
        if m and m.group(1) != crate:
            out.append(m.group(1))
    return sorted(set(out))


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    crates = closure()
    rows = []
    for c in crates:
        ps = parents(c)
        non_hub = [p for p in ps if p not in HUBS and p != FACADE]
        if FACADE in ps:
            kind = "FACADE-DIRECT"
        elif ps and not non_hub:
            kind = "HUB-ONLY"
        else:
            kind = "SHARED"
        rows.append({"crate": c, "kind": kind, "parents": ps, "non_hub_parents": non_hub})

    if args.json:
        print(json.dumps(rows, indent=2))
        return 0

    # ⛔ ANTI-VACUITY FLOOR, BEFORE ANY VERDICT. An empty closure, or one where
    # every crate reports zero parents, means the tree command changed shape and
    # every classification below is noise rather than a clean result.
    if not rows:
        sys.exit("no crates in the featureless closure — the measurement did not run")
    parentless = [r["crate"] for r in rows if not r["parents"]]
    if len(parentless) > 1:
        sys.exit(f"{len(parentless)} crates report NO parent, which cannot be true "
                 f"inside a closure: {parentless[:5]} — the parse is wrong")

    # ⭐⭐ THE HEADLINE, AND IT IS NOT THE COUNT. A capability the facade
    # ADVERTISES as optional, which a consumer selecting nothing links anyway,
    # is not optional — it is a promise the manifest does not keep. The row
    # already found this shape ONCE and fixed it: "`ambition_menu` <=
    # `ambition_platformer2d_runtime`, which installed `MapStatePlugin`
    # unconditionally — an optional capability with one unconditional installer
    # is not optional". This asks how many others there are.
    required, optional = facade_declared()
    present = {r["crate"] for r in rows}
    false_optional = sorted(optional & present)
    if not optional:
        sys.exit("the facade declares NO optional deps — the manifest parse is wrong, "
                 "and every verdict below would be vacuously clean")

    by_kind: dict[str, list[str]] = {}
    for r in rows:
        by_kind.setdefault(r["kind"], []).append(r["crate"])

    print(f"featureless closure of `{FACADE}`: {len(rows)} workspace crates\n")
    for kind in ("FACADE-DIRECT", "HUB-ONLY", "SHARED"):
        names = by_kind.get(kind, [])
        print(f"{kind}: {len(names)}")
        for n in names:
            row = next(r for r in rows if r["crate"] == n)
            extra = ""
            if kind == "SHARED":
                extra = "  <- parents: " + ", ".join(row["parents"])
            print(f"    {n}{extra}")
        print()
    print(f"FALSE OPTIONAL: {len(false_optional)} of the facade's "
          f"{len(optional)} advertised-optional capabilities are linked anyway "
          f"by a consumer that selected NONE of them")
    for n in false_optional:
        row = next(r for r in rows if r["crate"] == n)
        # ⛔⛔ EVERY PARENT, NOT THE INTERESTING ONES. This line printed only the
        # NON-HUB parents and truncated at four, and I then read it as the parent
        # set and called `cutscene <- boss_encounter` a single-edge fix. It has
        # three parents; two are hubs the filter hid, and one of those owns the
        # cutscene PLAYER. A summary that drops rows is not a summary a decision
        # can be made on, and the decision I made on it was wrong.
        via = ", ".join(row["parents"])
        print(f"    {n}  ({len(row['parents'])} edges) <- {via}")
    print()
    print("HUB-ONLY is the set a consumer selecting NO capability cannot decline "
          "except by an ownership change; it is a subject list, not a carve list.")
    print("FALSE OPTIONAL is the honest-capabilities defect: each is a manifest "
          "promise a consumer can check and find broken.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
