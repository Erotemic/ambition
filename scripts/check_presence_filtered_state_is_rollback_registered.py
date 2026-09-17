#!/usr/bin/env python3
"""Does a query FILTER on a component that no rollback registration restores?

The sibling guards ask about rows that EXIST: `check_rollback_mutators_run_in_sim`
asks who writes them from the wrong schedule, `rollback_schema_baseline` asks
whether the set changed. Neither can see a row that was never written — a
component a rewind leaves in whatever state the mispredicted future left it in,
because nothing told the snapshot about it.

⛔ **A COMPONENT WHOSE PRESENCE IS READ BY A QUERY FILTER IS AUTHORITATIVE EVEN
WHEN ITS VALUE IS DERIVED.** That sentence is the repository's own, from
`simulation-authority-and-determinism.md`, and both halves of the pair it was
written about are registered for exactly this reason: `Dormant`'s registration
says *"a rewind that dropped the marker would put a sleeping body back into the
decision phase for one advance"*, and `SensesUndecided`'s says *"'Re-derived next
tick' is not a reason to omit it: `ITEM 0` of this project's own record is a
component declared derived, dropped by a restore, and read before its writer ran
again."*

⇒ This guard is the inverse reading of the same rule, and it is a DETECTOR, not a
verdict. A hit means a presence-filtered component is absent from the snapshot
schema; whether that matters depends on whether the filtered query runs in a
rewinding schedule and on what the presence latches, which this script cannot
decide. Every exclusion below therefore states its measurement rather than its
category.

# # The population, and why it is an intersection

A component is in scope when BOTH hold:

* it is DEFINED in a crate that registers at least one rollback row (so
  presentation-only and tool crates are out by construction, not by a name list);
* its presence is read by a literal `With<X>` / `Without<X>` outside test code.

⚠ **AND THE FLOOR IS ON THE INTERSECTION, NOT ON ITS OPERANDS.** Both halves can
be large while the overlap is zero — a regex that stopped matching derives, or a
baseline whose third column moved, would print a serene `OK: 0` forever.

# # What it deliberately is not

⚠ The registered set is read from `rollback_schema_baseline.txt` rather than from
a running App. That file is held byte-identical against the LIVE registry by
`game/ambition_app/tests/rollback_schema_baseline.rs`, so the chain is sound —
but it is a chain, and a registration installed by a composition the sandbox
harness does not build is invisible to both.

⚠ It reads `#[derive(..Component..)]` above a `struct`/`enum`. A component
declared any other way — a manual `impl Component`, a macro — is not in the
population.

⚠ It cannot see a filter spelled through a type alias (`PrimaryPlayerOnly`) or
built in a generic. Those are filters on types this guard never learns about.

Usage:
    python3 scripts/check_presence_filtered_state_is_rollback_registered.py
    python3 scripts/check_presence_filtered_state_is_rollback_registered.py --list
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "scripts" / "lib"))
from test_paths import is_test_path  # noqa: E402

BASELINE = REPO / "game/ambition_app/tests/rollback_schema_baseline.txt"

#: ⛔ THE FLOORS. Chosen an order of magnitude below the readings of 2026-09-17
#: (20 registering crates, 288 component definitions in them, 443 baseline rows,
#: 293 distinct filtered names, 16 in the intersection) because their job is to
#: catch an instrument going silent, not to pin a census.
MIN_REGISTERED = 100
MIN_DEFINED = 50
MIN_FILTERED = 50
MIN_INTERSECTION = 5

#: ⛔ EXCLUDED BY NAME, EACH WITH THE MEASUREMENT THAT JUSTIFIES IT — never by
#: category, because "it looks like a camera" is what a category argument always
#: says right before it is wrong about one member. A name here is a READING: it
#: says this component's presence is not authoritative over rewinding state.
WAIVERS = {
    # Presentation. Each is filtered only by systems outside the sim schedule,
    # and a rewind that lost one would cost a frame of drawing, not a divergence.
    "MainCamera": "camera identity; 12 filter sites, all in render/camera systems",
    "FrontHudCamera": "the HUD camera layer, same population as `MainCamera`",
    "BossAnimator": "sprite-sheet animator state, rebuilt from the sim's pose view",
    "CharacterAnimator": "sprite-sheet animator state, rebuilt from the sim's pose view",
    "BossSpriteMetricsApplied": "a once-per-sheet render sync latch in `ecs/sync.rs`",
    "PlayerTrail": "the avatar's visual trail",
    "LightEmitter2d": "a relativity2d render signal",
    # World build. These mark entities the LDtk loader owns; the sim reads the
    # collision world it builds from them, not the markers.
    "LdtkSolid": "an LDtk loader marker; the sim reads the built collision world",
    "LdtkOneWayPlatform": "an LDtk loader marker; same road as `LdtkSolid`",
    "LdtkWorldRoot": "the LDtk asset root entity, not a simulated body",
    # Local lifecycle. ID-PEER's own tokens: these are deliberately per-App and
    # deliberately outside the peer surface. See `id_peer_audit.rs`.
    "SessionRoot": "a host-local publication owner; ID-PEER keeps it out of canonical state",
    # A separate demo crate's own timeline, not the platformer sim.
    "ActiveSpacetime2d": "`ambition_relativity2d`'s own world marker",
    "RelativisticClock2d": "`ambition_relativity2d`'s own clock marker",
    # Measured 2026-09-17: both filter sites are inside the component's OWN
    # writer, `project_conversation_hold`, whose authority `ActiveConversation`
    # IS registered (`resource.active_conversation`) and whose projection is
    # idempotent — a half-applied hold falls through and is repaired.
    "HeldByConversation": "projection of the registered `ActiveConversation`; both filter sites are its own writer",
}

#: ⚠ REAL AND OWED, not waived. A name here is a finding with somewhere to go.
ACKNOWLEDGED = {
    "ReleaseOnDeath": (
        "Q142 — a once-only latch in the sim schedule "
        "(`release_payloads_on_death`, `ProgressionSet::BossHazards`) whose "
        "REMOVAL is what stops a second emission, while the message it emits "
        "IS registered `message-clear`. The pair is asymmetric: the message is "
        "cleared so a resimulation can re-emit, and the latch that would let it "
        "is not restored"
    ),
    "EncounterScript": (
        "Q142 — attached by `setup_cut_rope_encounter` under a "
        "`Without<EncounterScript>` idempotence filter, and carries live beat "
        "state; unregistered, so what a rewind does to a mid-fight script is "
        "unmeasured"
    ),
}

_DERIVE_COMPONENT = re.compile(
    r"#\[derive\(([^)]*)\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum)\s+([A-Z]\w*)"
)
_FILTER = re.compile(r"\bWith(?:out)?<([A-Z]\w*)>")


def _git_grep(pattern: str, *paths: str) -> list[str]:
    out = subprocess.run(
        ["git", "grep", "-n", "-E", pattern, "--", *paths],
        cwd=REPO,
        capture_output=True,
        text=True,
    )
    return out.stdout.splitlines()


def registered_type_names() -> set[str]:
    """Short type names in the recorded schema, one per row after the header."""
    names = set()
    for line in BASELINE.read_text().splitlines()[1:]:
        cols = line.split("\t")
        if len(cols) >= 3:
            names.add(cols[2].split("<")[0])
    return names


def registering_crates() -> list[str]:
    """Crates that declare at least one rollback row of any kind."""
    hits = _git_grep(r"registrar\.(rollback|clear|require)", "crates", "game")
    return sorted(
        {
            line.split(":", 1)[0].split("/")[1]
            for line in hits
            if line.startswith("crates/")
        }
    )


def component_definitions(crates: list[str]) -> dict[str, str]:
    """`{Component: defining file}` for production code in those crates.

    ⛔ The text is cut at the first `#[cfg(test)]` rather than by filename: a
    test-only component declared inside a production file is not the subject,
    and `tests.rs`-skipping alone does not see it.
    """
    defs: dict[str, str] = {}
    for crate in crates:
        src = REPO / "crates" / crate / "src"
        for path in sorted(src.rglob("*.rs")):
            rel = path.relative_to(REPO).as_posix()
            text = path.read_text(errors="replace")
            if is_test_path(path, text):
                continue
            cut = text.find("#[cfg(test)]")
            if cut != -1:
                text = text[:cut]
            for match in _DERIVE_COMPONENT.finditer(text):
                if "Component" in match.group(1):
                    defs.setdefault(match.group(2), rel)
    return defs


def filter_sites() -> dict[str, list[str]]:
    sites: dict[str, list[str]] = {}
    for line in _git_grep(r"With(out)?<[A-Z][A-Za-z0-9_]*>", "crates", "game"):
        path, _, rest = line.partition(":")
        if is_test_path(Path(path)):
            continue
        for name in _FILTER.findall(rest):
            sites.setdefault(name, []).append(path)
    return sites


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--list", action="store_true", help="print the whole intersection, waivers included"
    )
    args = parser.parse_args()

    registered = registered_type_names()
    crates = registering_crates()
    defs = component_definitions(crates)
    sites = filter_sites()

    intersection = sorted(name for name in sites if name in defs)
    findings = [n for n in intersection if n not in registered]

    # ⛔ THE ANTI-VACUITY FLOOR, ON THE INTERSECTION AND ON EACH OPERAND. A join
    # keyed wrongly returns an empty map and every assertion below passes over
    # nothing; two large operands with no overlap print the same `OK` as a clean
    # tree.
    measured = {
        "registered rows": (len(registered), MIN_REGISTERED),
        "component definitions": (len(defs), MIN_DEFINED),
        "filtered names": (len(sites), MIN_FILTERED),
        "intersection": (len(intersection), MIN_INTERSECTION),
    }
    starved = [f"{k}: {got} < {floor}" for k, (got, floor) in measured.items() if got < floor]
    if starved:
        print(
            "this check measured almost nothing, so its verdict is about ITSELF "
            "and not about the tree:\n  " + "\n  ".join(starved),
            file=sys.stderr,
        )
        return 2

    if args.list:
        for name in intersection:
            mark = (
                "registered"
                if name in registered
                else "WAIVED"
                if name in WAIVERS
                else "OWED"
                if name in ACKNOWLEDGED
                else "FINDING"
            )
            print(f"  {mark:10s} {name:34s} {len(sites[name]):3d} site(s)  {defs[name]}")

    banked = [n for n in findings if n in ACKNOWLEDGED]
    if banked:
        print(
            f"{len(banked)} acknowledged, each owed to an open row and NOT waived:\n  "
            + "\n  ".join(f"{n}  → {ACKNOWLEDGED[n]}" for n in banked),
            file=sys.stderr,
        )

    new = [n for n in findings if n not in WAIVERS and n not in ACKNOWLEDGED]
    if new:
        print(
            "a query filters on these components and no rollback registration "
            "restores them:\n\n  "
            + "\n  ".join(f"{n}  ({len(sites[n])} site(s))  defined in {defs[n]}" for n in new)
            + "\n\nPresence is authoritative when a query FILTERS on it, however "
            "derived the value is: a rewind that leaves the marker in the state "
            "the mispredicted future left it in hands the filtered system a world "
            "its own writer never produced.\n\n"
            "Register it, or add it to WAIVERS with the MEASUREMENT that says its "
            "presence is not authoritative over rewinding state — which filter "
            "sites, and which schedule each runs in.",
            file=sys.stderr,
        )
        return 1

    print(
        f"OK: {len(intersection)} presence-filtered component(s) defined in "
        f"{len(crates)} registering crate(s); {len(intersection) - len(findings)} registered, "
        f"{len(WAIVERS)} waived, {len(banked)} owed, no NEW unregistered filter subject."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
