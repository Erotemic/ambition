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

# # The neighbouring question, asked and answered clean

⚠ **THE SAME SHAPE EXISTS FOR MESSAGES AND IT HAS NO FINDINGS — MEASURED
2026-09-17, recorded so nobody pays for the measurement twice.** A message
written in a mispredicted future and not cleared is read again after the rewind,
which is why `clear_message_on_rollback` exists. Of the 79 message types both
written and read in production, 22 carry no `message-clear` row: twenty-one are
shell, menu or load-presentation messages that never cross the sim, and the
twenty-second is `BodyMovementOps`, which IS registered — through
`clear_instrument_message_on_rollback`, whose kind `schema_dump()` deliberately
excludes so a debugging instrument cannot move the peer schema identity. ⇒ A
guard here would have an empty population today; the census is the evidence, not
a second script.

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
from rust_source import strip_comments  # noqa: E402
from test_paths import is_test_path, strip_test_modules  # noqa: E402

BASELINE = REPO / "game/ambition_app/tests/rollback_schema_baseline.txt"

#: ⛔ THE FLOORS. Chosen an order of magnitude below the readings of 2026-09-17
#: (20 registering crates, 443 baseline rows, 114 components in the intersection
#: of which 91 are registered) because their job is to catch an instrument going
#: silent, not to pin a census. ⚠ That intersection read **95, then 104, then
#: 114** in one day as three separate blind spots were repaired — the file-tail
#: test cut, the widened test-module stripper, and `filter_sites`' own narrower
#: `git grep` grammar. A floor set from a blind reading is still a floor, which is
#: the only reason this one kept working through all three.
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
    # The read model. `ambition_sim_view`'s fact views are rebuilt from the sim
    # every tick, so a marker read only there decides what is DRAWN from a world
    # the rewind has already corrected.
    "BossRewardChest": "one site, the `ambition_sim_view/src/facts.rs` per-tick view rebuild",
    "EncounterRewardChest": "one site, the same per-tick view rebuild",
    "PortalInputWarp": "one site, `ambition_portal2d_presentation/src/visuals.rs`",
    # Two sites: the view rebuild, and `declare_ambition_dormancy`, which is
    # gated `Without<DormancyPolicy>` — so the decision this marker feeds is
    # latched in a component that IS registered (`actor.dormancy_policy`).
    "EncounterMob": "view rebuild + a setup gated on the registered `DormancyPolicy`",
    # ⭐ BOTH OF THESE BECAME VISIBLE ON 2026-09-17 when the test cut stopped
    # discarding `construction/mod.rs`'s second half, and both are the
    # `HeldByConversation` shape: the only readers are the writer's own module.
    #
    # Four line hits for `InactiveCandidate`, three of them code and one a doc
    # comment, all inside `construction/mod.rs`: `candidate_carries_identity`,
    # `outstanding_candidates` and `candidate_roots`, each an `&mut World`
    # exclusive step, and `retire_candidate`'s comment. That module is also its
    # only writer (`raise_candidate_barrier` / `publish_candidate`), and the type
    # is `pub(crate)` ON PURPOSE — its own doc says nobody outside the crate can
    # name it, so nobody outside can filter on it either.
    "InactiveCandidate": "publication barrier count; 3 code sites, all `&mut World` steps in its own module, which is its only writer",
    # And this one has no production writer AT ALL, measured: zero insert or
    # spawn sites outside tests. The scope classifier reads `Option<&_>` to
    # produce `ScopeClassification::PresentationOnly` and two queries exclude it,
    # so the opt-out vocabulary is available and currently unexercised. ⚠ A
    # rewind cannot diverge a component nothing writes; the day a production
    # spawn stamps one, this waiver has to be re-read.
    "PresentationOnly": "opt-out authority marker with ZERO production insert sites (measured 2026-09-17); 2 filter sites, both its own classifier",
    # ⭐ TWO ARRIVED 2026-09-17 WHEN `filter_sites` STOPPED PREFILTERING WITH A
    # NARROWER GRAMMAR (see that function). Both are adjudicated by measurement
    # rather than by the presentation analogy above.
    #
    # `PresentationOf(pub Entity)` cannot BE rollback state — it holds a live
    # `Entity`, which is the one thing a snapshot may never carry — and all three
    # filter sites are in crates that draw: `portal2d_presentation`'s
    # source_visibility and two in `render`'s portal_compositing. Its publisher
    # sits beside them (`portal_compositing.rs:338`) and restamps it from the sim's
    # answer to "whose body does this drawable belong to".
    "PresentationOf": "holds a live `Entity`, so it can never be rollback state; 3 filter sites, all in `ambition_render` / `ambition_portal2d_presentation`, republished by the compositing publisher beside them",
    # `BodyWalletShield` is DERIVED-BEFORE-EVERY-AUTHORITATIVE-READ, and that is
    # an ordering claim, so it is measured rather than asserted. Its one writer
    # `sync_sanic_wallet_shield` (`demo_sanic/src/lib.rs:1967`) rebuilds it from
    # the worn persona plus the ACTIVE ROOM's mode tag, registered in the
    # `PlayerInput` phase `.after(PlayerInputSet::Persona)` and explicitly
    # `.before(ambition_damage::PlayerHitResolutionSet)`. Both authoritative
    # readers are downstream of that: `apply_player_hit_events` IS
    # `PlayerHitResolutionSet` (in `PlayerSimulationSet::Outcome`, phase
    # `PlayerSimulation`), and `apply_feature_hit_events` reads it as data in the
    # `Combat` phase — and `(PlayerInput, WorldPrep, PlayerSimulation,
    # RoomTransition, Combat, ..)` is `.chain()`ed at `schedule/schedule.rs:91`.
    # ⚠ So the explicit edge covers one reader and the phase chain the other; if
    # either moves, this waiver is what has to be re-read.
    "BodyWalletShield": "derived every frame by `sync_sanic_wallet_shield`, which is registered `.before(PlayerHitResolutionSet)` in `PlayerInput` while both readers run in `PlayerSimulation` and `Combat` (phases `.chain()`ed); 1 filter site, its own rebuild loop",
}

#: ⚠ REAL AND OWED, not waived. A name here is a finding with somewhere to go.
ACKNOWLEDGED = {
    # ⭐ THREE LEFT THIS DICT BY BEING FIXED, 2026-09-17, schema v197 -> v198.
    # `ReleaseOnDeath`, `RecharacterizeBody` and `EncounterScript` were filed
    # here as Q142's open question and turned out not to be a question: each is
    # mutated or consumed by a system in the REWINDING schedule, so each is now
    # registered and falls out of this list by satisfying the check rather than
    # by being excused. The remaining entry is the one none of those arguments
    # reached.
    "PostBossNpc": (
        "Q142 — one site is the per-tick view rebuild and the other is "
        "`AttemptResidue` in `world/rooms/reconstitution.rs`, the set an "
        "admitted replay retires. Presence decides whether the celebrant a "
        "defeated boss left behind is swept — a question about what a LOAD "
        "does, not what a tick does, so it wants a behavioural arm rather than "
        "a registration by analogy with its three former neighbours"
    ),
}

_DERIVE_COMPONENT = re.compile(
    r"#\[derive\(([^)]*)\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum)\s+([A-Z]\w*)"
)
#: ⛔⛤ **THE PATH PREFIX IS NOT OPTIONAL TO MATCH, AND LEAVING IT OUT COST THE
#: FIRST VERSION ITS BEST POISON.** `Dormant` — the very component the doctrine
#: paragraph above is written about — is filtered three times in
#: `features/ecs/actors/update.rs` and every site spells it
#: `Without<crate::features::ecs::dormancy::Dormant>`. Deleting its baseline row
#: left the check GREEN, because a bare-name regex saw none of the three. The
#: last `::` segment is the type.
#:
#: ⚠ `Has<T>` is in, and it is a presence read like the other two: it reads the
#: marker into a `bool` the system then branches on.
_FILTER = re.compile(r"\b(?:With(?:out)?|Has)<\s*(?:[A-Za-z_][\w]*::)*([A-Z]\w*)\s*>")


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

    ⛔ Test regions are cut from the TEXT as well as by filename: a test-only
    component declared inside a production file is not the subject, and
    `tests.rs`-skipping alone does not see it.

    ⛔⛤ **AND THE CUT USED TO BE `text.find("#[cfg(test)]")` — THE FILE TAIL —
    WHICH IN THIS TREE IS USUALLY PRODUCTION CODE.** A module declares its tests
    near the TOP, so every `#[derive(Component)]` below that line was invisible.
    MEASURED 2026-09-17, on the same tree at one commit: **288 component
    definitions become 305, and the presence-filtered intersection 95 becomes
    104** (and 114 once `filter_sites` stopped prefiltering with a narrower
    grammar of its own — see that function).** Two of the nine that appeared were neither registered nor waived —
    `InactiveCandidate` and `PresentationOnly`, both in
    `shared_tangle/src/construction/mod.rs`, which declares `#[cfg(test)] mod
    tests;` and then defines most of A10's vocabulary underneath it. ⇒ The
    guard was blind to the whole construction crate's second half, and its `OK`
    said nothing about it.
    """
    defs: dict[str, str] = {}
    for crate in crates:
        src = REPO / "crates" / crate / "src"
        for path in sorted(src.rglob("*.rs")):
            rel = path.relative_to(REPO).as_posix()
            text = path.read_text(errors="replace")
            if is_test_path(path, text):
                continue
            text = strip_test_modules(text)
            for match in _DERIVE_COMPONENT.finditer(text):
                if "Component" in match.group(1):
                    defs.setdefault(match.group(2), rel)
    return defs


def filter_sites() -> dict[str, list[str]]:
    """`{Component: [file, ...]}` for every presence filter in production code.

    ⛔⛔ **THIS USED TO PREFILTER WITH `git grep` AND THE PREFILTER WAS A SECOND,
    NARROWER GRAMMAR.** It searched
    `(With(out)?|Has)<[A-Za-z_:]*[A-Z][A-Za-z0-9_]*>`, whose path class has NO
    DIGITS — so every filter written through a qualified path containing
    `platformer2d` or `portal2d` was thrown away before `_FILTER` ever saw it, and
    `Has<ambition_platformer2d::characters::actor::BodyWalletShield>` is exactly
    that shape. It also read one LINE at a time, so a filter a formatter wrapped
    was invisible, and it never stripped test regions, so an inline fixture's
    filter counted as production. Found by review 2026-09-17.

    ⇒ There is now ONE grammar (`_FILTER`) applied to the whole stripped source of
    each production file — the same treatment [`component_definitions`] gets, for
    the same reason. MEASURED: the intersection moves 104 -> 116, thirteen real
    subjects arrive, and `HitboxLifetime` LEAVES because its only apparent
    production use is inside an inline test module.

    ⚠ Comments are stripped first. A paragraph saying *"this used to be
    `Without<Foo>`"* is not a filter, and this repository has already paid for a
    census that read prose — see `lib/rust_source.py`.
    """
    sites: dict[str, list[str]] = {}
    for root in ("crates", "game"):
        for path in sorted((REPO / root).rglob("*.rs")):
            rel = path.relative_to(REPO)
            if any(part == "target" for part in rel.parts):
                continue
            text = path.read_text(errors="replace")
            if is_test_path(rel, text):
                continue
            text = strip_test_modules(strip_comments(text))
            for name in _FILTER.findall(text):
                sites.setdefault(name, []).append(rel.as_posix())
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
