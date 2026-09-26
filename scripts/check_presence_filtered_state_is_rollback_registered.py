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
decision phase for one advance"*, and `SensesUndecided`'s said (until AP57 derived
a body's senses and deleted the marker) *"'Re-derived next
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
    "PlayerTrail": "the avatar's visual trail",
    "LightEmitter2d": "a relativity2d render signal",
    # ⭐⛤ THE `game/*` ARRIVALS, 2026-09-18. These became visible when
    # `registering_crates` stopped assuming every registering crate lives under
    # `crates/` — `game/ambition_content` registers rollback state and its whole
    # component population had been outside this guard. Five of the six that
    # appeared are presentation and are waived HERE, each with the schedule its
    # filter site runs in, because "it is under `presentation/`" is a category
    # argument and this table does not take those.
    "AmbitionDialogPortraitImage": (
        "one site, `advance_ambition_dialog_portrait`, registered in `Update` in "
        "`AmbitionDialogUiPlugin`; it filters a `Query<&mut ImageNode>` — the "
        "portrait's texture, not a simulated fact"
    ),
    "PuppySlugDeepDreamOverlay": (
        "one site, `sync_puppy_slug_deep_dream_overlays`, registered in `Update` "
        "in `ActorOverlaySet`; the marker says a shader overlay child exists"
    ),
    "PuppySlugDeepDreamSource": (
        "two sites, `attach_…` and `cleanup_puppy_slug_deep_dream_overlays`, both "
        "`Update` in `ActorOverlaySet`; the `Without<…>` is the attach pass's own "
        "idempotence, as its doc comment states"
    ),
    "VanityCardStage": (
        "one site, `fit_card_to_display`, registered in `Update` under "
        "`run_if(resource_exists::<ActiveShellSequence>)`; it filters a "
        "`Query<&mut UiTransform>` inside a shell sequence, which is not the sim"
    ),
    "VanityCardViewport": (
        "one site, the same `fit_card_to_display`, filtering `&ComputedNode` — "
        "bevy_ui's computed layout, recomputed every frame from scratch"
    ),
    # World build. These mark entities the LDtk loader owns; the sim reads the
    # collision world it builds from them, not the markers.
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
    # ⛔⛤ THE SIXTH `game/*` ARRIVAL, AND THE ONLY ONE THAT IS NOT PRESENTATION.
    "SmirkingBehemothVictoryNpc": (
        "Q142 — its one filter site is `spawn_cut_rope_victory_npc`'s `existing: "
        "Query<&FeatureId, With<SmirkingBehemothVictoryNpc>>`, a SPAWN-ONCE guard, "
        "and that system is registered in the REWINDING schedule "
        "(`app.add_systems(sim, .. .in_set(ContentEncounterVictorySet))` in "
        "`bosses/mod.rs:379`). So presence decides whether a re-simulated victory "
        "frame spawns a second celebrant — exactly the shape Q142 is about, on a "
        "component the question could not name because this guard could not see "
        "`game/ambition_content` at all until 2026-09-18. ⚠ NOT registered by "
        "analogy: Q142's own text records that the first `ReleaseOnDeath` arm "
        "asserted THIS NPC's presence and its poison PASSED, because a visible "
        "consequence that is not itself rollback state survives the rewind "
        "whatever the registration does. It wants the per-pass census shape that "
        "closed the other three, not a fourth row added because its neighbours "
        "moved"
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
    """Crates that declare at least one rollback row of any kind.

    ⛔⛤ **REPOSITORY-RELATIVE ROOTS (`crates/foo`, `game/foo`) SINCE 2026-09-18,
    AND THE OLD BARE-NAME FORM WAS A SILENT POPULATION HOLE.** The grep already
    searched both trees; the comprehension then kept only lines starting with
    `crates/` and reduced each to its second path segment, and
    [`component_definitions`] rebuilt it as `crates/<name>/src`. So a crate under
    `game/` could register rollback state all day and none of its components
    entered the population — while [`filter_sites`] scanned `game/` the whole
    time. The guard's stated subject is *"a component defined in a crate that
    registers rollback state"*; what it measured was that sentence with
    `crates/*` silently appended.
    ⚠ MEASURED, so this is not hypothetical: `game/ambition_content` registers
    `EchoFanState` and the rest of `bosses/specials/rollback.rs`, plus
    `PortalHostScanned` through `portal/plugin.rs`. Its components were outside
    the guard's reach entirely.
    ⛔ THE FLOORS DID NOT PROTECT AGAINST THIS and could not have. They catch a
    join that returns almost NOTHING; a stable omitted category leaves the
    remaining population comfortably above every floor, which is exactly what it
    did."""
    hits = _git_grep(r"registrar\.(rollback|clear|require)", "crates", "game")
    return sorted({"/".join(line.split(":", 1)[0].split("/")[:2]) for line in hits})


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
        # `crate` is repository-relative (`crates/foo`, `game/foo`) — see
        # [`registering_crates`] for what assuming `crates/` used to cost.
        src = REPO / crate / "src"
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


#: `Q142` and friends: the row an acknowledged subject is owed to.
_OWED_TO = re.compile(r"\b(Q\d+)\b")
#: Where those rows live.
OPEN_ROWS = Path("docs/planning/awaiting-maintainer-decision.md")


def owed_rows() -> dict[str, str]:
    """`subject -> the question id its reading says it is owed to`."""
    found: dict[str, str] = {}
    for name, reading in ACKNOWLEDGED.items():
        match = _OWED_TO.search(reading)
        if match:
            found[name] = match.group(1)
    return found


def subjects_missing_from_their_row(repo: Path = REPO) -> list[tuple[str, str]]:
    """Acknowledged subjects whose own question does not name them.

    ⛔⛤ **A HEADING IS A SECOND OWNER OF THE BODY'S FACT, AND NOTHING CHECKED
    IT.** `Q142` read *"the question is down to `PostBossNpc`"* in its heading
    for a day after this guard started reporting TWO subjects: widening the
    component population to `game/` added `SmirkingBehemothVictoryNpc`, the body
    of the row recorded it in four places, and the heading — the part a reader
    sees first and the part every index quotes — still said one. ⇒ This asks the
    cheapest version of the question: does the row an acknowledgement CITES
    actually name the subject anywhere? It cannot check a count, and it does not
    try; it catches the case where a subject is owed to a row that has never
    heard of it.
    """
    text = (repo / OPEN_ROWS).read_text(errors="replace")
    sections: dict[str, str] = {}
    parts = re.split(r"^## (Q\d+)", text, flags=re.M)
    for i in range(1, len(parts) - 1, 2):
        sections[parts[i]] = parts[i + 1]
    missing = []
    for subject, row in sorted(owed_rows().items()):
        body = sections.get(row)
        # ⛔ WORD-BOUNDED, because the first version used `in` and its own poison
        # PASSED: renaming the subject to `SmirkingBehemothVictoryNpcXX` on the
        # page left the old name as a SUBSTRING of the new one, so the check
        # reported the row still named a component that no longer existed.
        if body is None or not re.search(rf"\b{re.escape(subject)}\b", body):
            missing.append((subject, row))
    return missing


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

    orphaned = subjects_missing_from_their_row()
    if orphaned:
        print(
            "an acknowledged subject is owed to a row that does not name it:\n\n  "
            + "\n  ".join(f"{subject}  → {row}" for subject, row in orphaned)
            + f"\n\nThe reading here says {OPEN_ROWS} owes this subject an answer, and "
            "that row has never heard of it — so the question a maintainer reads is "
            "narrower than the one this guard is holding open. Add the subject to the "
            "row (its heading too, if the heading counts them), or point the "
            "acknowledgement at the row that really owns it.",
            file=sys.stderr,
        )
        return 1

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
