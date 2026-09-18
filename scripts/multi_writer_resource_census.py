#!/usr/bin/env python3
"""Which `Resource`s are written (`ResMut<T>`) from more than one FILE?

⛔⛔ THIS IS A SHORTLIST, NOT A FINDING LIST, and the docstring says so because the
count is the part that misleads. A fact written from N places is NOT an authority
violation by count. Two measured cases from 2026-09-06, same grep, opposite
verdicts:

  * `CutRopeBossArenaState` — TWO systems retracted it on the SAME message. A
    poison proved it: deleting either retractor alone left an end-to-end test
    GREEN, deleting both turned it RED. ⇒ Two copies did not merely risk drifting
    apart; each hid the other's absence, so NEITHER could be tested. Real defect.
  * `ActiveRoomTransitionLoad::asset_readiness_complete` — SIX `= true` sites
    across two crates and FOUR distinct meanings (no contributor, host cannot
    answer, assets failed, genuinely ready). CORRECT. `commit.rs` never reads it:
    the commit is gated by `phase`, every failure path sets `phase = Failed`, and
    the outcome lives in that single-authority sibling.

⭐ SO THE DISCRIMINATOR IS ON THE READER'S SIDE: ask what READS the fact, and
whether an ambiguity in it can reach a DECISION. The fighter lane's writer-side
version of the same rule: two branches of ONE function is one authority with two
exits; two SYSTEMS on one trigger is two authorities.

⇒ AND THE CONFIRMING EXPERIMENT IS A POISON, not a reading: delete one writer and
run the test that should care. If it stays green, either nothing tests the fact or
another writer is covering — and only the second is a finding. Confirm the edit
landed (`grep -c`) before believing either.

Usage:  python3 scripts/multi_writer_resource_census.py [PATH ...]
"""

from __future__ import annotations

import argparse
import collections
import pathlib
import re
import subprocess
import sys

#: `ResMut<T>`, collapsed on the type's last path segment by the caller.
#:
#: ⛔⛤ **THE `,?` IS A TRAILING COMMA AND IT HID TEN WRITE SITES.** A long
#: qualified path wraps, and the wrapped form carries a trailing comma:
#:
#:     ResMut<
#:         ambition_platformer2d_actor_monolith::session::lifecycle_commit::PendingLifecycleCommit,
#:     >,
#:
#: `\s*>` cannot cross that comma, so `room_transition/commit.rs` — the system
#: that SPENDS the lifecycle slot — was not a writer of it as far as this census
#: was concerned.
#:
#: ⛔⛔ **AND THE OPTIONAL LIFETIME IS THE BIGGEST OF THE THREE: `ResMut<'w, T>`,
#: WHICH IS HOW EVERY `SystemParam` BUNDLE SPELLS IT.** A bundle is exactly the
#: shape a resource takes when several systems share one accessor — the
#: `ActingParticipant` that answers *"which controller wants to interact"*, the
#: `DialogueDispatch` that opens a conversation — so the census was blind to the
#: most deliberate form of shared access in the tree. MEASURED 2026-09-17: 80
#: sites over 13 files and 69 types, taking the shortlist from 102 to **121**,
#: with 19 types arriving that had never been on it and 31 gaining writers,
#: including `SlotInteractionState` 4 -> 6 and `ActiveConversation` 2 -> 5. ⇒ Two
#: verdicts banked earlier the same day had to be re-derived against the complete
#: population; see their entries in the adjudication guard. MEASURED 2026-09-17: 10 sites across 10 files,
#: `PendingLifecycleCommit` alone going from 3 writer files to 6, plus
#: `FeatureEcsWorldOverlay` 8->9, `DeveloperRuntimeState` 5->6,
#: `RoomTransitionCooldown` 2->3, and `MovingPlatformSet` arriving on the
#: shortlist having never been on it. ⚠ The sites are exactly the ones a
#: formatter chose to wrap, so the
#: blind spot correlated with PATH LENGTH — which correlates with crossing a
#: crate boundary, which is where a second authority is most likely to be.
RESMUT = re.compile(
    r"ResMut<\s*(?:'[a-z_][a-z0-9_]*\s*,\s*)?"
    r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z0-9_]+)*)\s*,?\s*>"
)

#: `world.resource_mut::<T>()` / `get_resource_mut::<T>()` — THE OTHER WAY BEVY
#: HANDS OUT A MUTABLE RESOURCE, and for a year this census knew only one.
#:
#: ⛔⛔ **MEASURED 2026-09-17: 83 MULTI-WRITER TYPES BECOME 102 WHEN THIS SHAPE
#: COUNTS.** Nineteen types were not on the shortlist at all
#: (`LocalSessionOwnership`, `ShellRouteCatalog`, `ConstructionSchemaCatalog`, …)
#: and twenty-one gained writers. This is not a softer signal than `ResMut<T>`:
#: it is the same `&mut T`, taken from an exclusive-world system instead of a
#: parameter, and there is no reading of it as a helper call the way a `&mut T`
#: PARAMETER can be.
#:
#: ⭐ **AND THE MISSING FILES ARE NOT A RANDOM SAMPLE**, which is why this was
#: worth more than the count suggests. An exclusive-world system is what a
#: COMMIT EXECUTOR is, so the road this shape hides is the destructive one:
#: `rollback_ggrs/lifecycle_commit.rs` clears `PendingLifecycleCommit` and
#: `RoomTransitionLoadState` and touches `LoadCoordinator`;
#: `session/reset/mod.rs` reaches `AmbitionGameSave`, `AuthoredOccurrences`,
#: `QuestRegistry`, `GameplayBanner` and `RoomTransitionCooldown`. The census was
#: blind to the systems that spend the state it was auditing.
#:
#: ⚠ Deliberately NOT included: `insert_resource`, `init_resource` and
#: `get_resource_or_insert_with`. Those INSTALL a value rather than mutate a live
#: one, which is a different question (who owns construction) with a different
#: right answer, and folding it in here would put every plugin's `build` on the
#: shortlist.
WORLD_RESOURCE_MUT = re.compile(
    r"(?:get_)?resource_mut::<\s*([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z0-9_]+)*)\s*,?\s*>"
)

# ⛔⛤ **BOTH TEST PREDICATES ARE IMPORTED, NOT RESPELLED, AND THIS CENSUS BRIEFLY
# HAD ITS OWN COPY OF EACH — 2026-09-17.** `scripts/lib/test_paths.py` exists
# because there were FIVE spellings of *"is this Rust file test-only?"* giving
# five different answers, and it owns the inline-module half too (which MOVED
# there from `check_rollback_mutators_run_in_sim.py` once this census and
# `architecture_census.py` turned out to hold copies). Mine were a sixth and a
# second:
# the file rule missed `test.rs`, `test_support.rs` and the four files whose
# first attribute is an inner `#![cfg(test)]`, which no name rule can see.
#
# ⭐ MEASURED BEFORE COLLAPSING, all four combinations of the two rules against
# this corpus: **333 `ResMut<T>` types and 85 multi-writer, identically.** So the
# de-duplication changes nothing here and the keepers are strictly better
# informed — which is the only kind of collapse worth doing without re-poisoning
# every consumer.
#
# ⚠ WHAT THE SHARED STRIPPER LEAVES: it removes inline `#[cfg(test)] mod X { }`
# blocks only, so a `#[cfg(test)] fn helper(mut r: ResMut<T>)` sitting in a
# production file still counts as a writer. There are none today (the measurement
# above would differ), and widening that function reaches every one of its
# consumers — its own docstring says not to do that without reading each one's
# counts.
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent / "lib"))
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from rust_source import strip_comments  # noqa: E402
from test_paths import is_test_path, strip_test_modules  # noqa: E402
DEFAULT_PATHS = ("crates", "game")


def rust_files(paths: tuple[str, ...]) -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", *paths], capture_output=True, text=True, check=True
    ).stdout.split()
    return [f for f in out if f.endswith(".rs")]


def production_files(paths: tuple[str, ...] = DEFAULT_PATHS) -> list[str]:
    """Every tracked Rust file that is not test-only, by the shared predicate.

    ⛔⛤ **THE CENSUS COUNTED WHOLE TEST FILES AS WRITERS UNTIL 2026-09-17**, and
    its docstring said it read NON-TEST code the whole time: a `mod tests` in its
    own `tests.rs` and an integration test under `tests/` carry no
    `#[cfg(test)]` line for the stripper to cut at — the parent module carries
    it. MEASURED: 1,866 tracked files, **1,294 production**, and eight types left
    the shortlist (`Captured`, `CapturedHits`, `DeathsSeen`, `FixedStepsTaken`,
    `PortalWorldFrame`, `RoomResetsSeen`, `SaveRestored`, `SeatMenuFrames`), each
    multi-writer only because a fixture wrote it.
    """
    return [f for f in rust_files(paths) if not is_test_path(pathlib.Path(f))]


#: `SessionWorldMut<T>` — the mutable accessor for a SESSION WORLD COMPONENT,
#: which is state A10 deliberately moved OUT of the resource space.
#:
#: ⛔⛤ **SO THE WRITER-SIDE INSTRUMENT OF THE DUPLICATE-AUTHORITY CAMPAIGN WAS
#: LOSING COVERAGE EXACTLY WHERE THE ARCHITECTURE WAS MOVING.** None of the eight
#: types reached this way carries `#[derive(Resource)]` — measured 2026-09-17 —
#: so [`writers`] correctly excludes them and the census reported nothing about
#: them at all. Four have more than one production writer, and one has EIGHT:
#:
#:     EncounterMusicRequest   8 files
#:     RoomSet                 2    session/reset/mod.rs + app/dev_runtime.rs
#:     RoomGeometry            2    the same pair
#:     LdtkRuntimeIndex        2    ldtk asset.rs + app/dev_runtime.rs
#:
#: ⚠ They are a DIFFERENT POPULATION and are reported separately, not folded in:
#: a session world component's lifetime is the session's, so "two writers" is a
#: question about one session's state rather than about the App's, and the
#: adjudication guard ratchets it with its own baseline and its own floor.
#:
#: ⭐ The spelling came from `check_rollback_mutators_run_in_sim.py`, whose
#: docstring already recorded BOTH of the things this census had to learn the hard
#: way — that `SessionWorldMut<T>` is a mutable param (*"a guard keyed on how a
#: write is SPELLED goes blind when a refactor respells it"*) and that the
#: optional lifetime is not cosmetic. A lesson written down in one guard while its
#: neighbour repeats the defect is the shape this repository keeps paying for.
SESSION_WORLD_MUT = re.compile(
    r"SessionWorldMut\s*<\s*(?:'[a-z_][a-z0-9_]*\s*,\s*)?"
    r"((?:[A-Za-z_][A-Za-z0-9_]*::)*[A-Z][A-Za-z0-9_]*)\s*,?\s*>"
)


def session_world_writers(files: list[str]) -> dict[str, set[str]]:
    """`{short type name: {file, ...}}` for SESSION WORLD components.

    Same test and comment stripping as [`writers`], different question: these are
    components on the session root, not resources, so they never appear in that
    function's population. See [`SESSION_WORLD_MUT`] for why that mattered.
    """
    found: dict[str, set[str]] = collections.defaultdict(set)
    for f in files:
        src = pathlib.Path(f).read_text(encoding="utf-8", errors="replace")
        src = strip_test_modules(strip_comments(src))
        for m in SESSION_WORLD_MUT.finditer(src):
            found[m.group(1).split("::")[-1]].add(f)
    return found


def writers(files: list[str]) -> dict[str, set[str]]:
    """`{short type name: {file, ...}}` over the files it is GIVEN.

    A file is a writer if it takes a `ResMut<T>` parameter OR reaches the
    resource through [`WORLD_RESOURCE_MUT`] — the two ways Bevy hands out `&mut
    T` for a resource. Comments are stripped first, then `#[cfg(test)]` items.

    ⚠ Each `#[cfg(test)]` ITEM is cut by [`strip_test_modules`]. A fixture that
    builds a resource by hand is not a second authority over it, and counting
    fixtures is how a census manufactures findings nobody can act on.

    ⛔ **A WHOLE TEST FILE HAS NO SUCH MARKER**, so the caller filters those out
    with [`production_files`] — see what that cost when it did not.

    ⚠ **THE STATED RESIDUAL: A `&mut T` PARAMETER IS NOT COUNTED.** MEASURED
    2026-09-17, it would take the shortlist from 102 to 109 and add writers to 20
    more types. It is left out because unlike the two shapes above it is
    genuinely ambiguous: `fn grant(items: &mut OwnedItems, ..)` may be a SECOND
    AUTHORITY or a helper the single owner calls, and only the call sites say
    which. Folding it in would put every extracted helper on a duplicate-
    authority shortlist. ⇒ When adjudicating a type, check its `&mut T` sites by
    hand — `OwnedItems` has 5 files' worth and `AmbitionGameSave` 2.
    """
    found: dict[str, set[str]] = collections.defaultdict(set)
    for f in files:
        src = pathlib.Path(f).read_text(encoding="utf-8", errors="replace")
        src = strip_test_modules(strip_comments(src))
        for pattern in (RESMUT, WORLD_RESOURCE_MUT):
            for m in pattern.finditer(src):
                found[m.group(1).split("::")[-1]].add(f)
    return found


#: The enclosing ITEM of a write site. ⚠ NOT a parser: it takes the nearest
#: preceding `fn` or `struct` header, which is what a Rust file's layout makes
#: true for a system parameter, for a `world.resource_mut` call in a body, and
#: for a `SystemParam` bundle's field. A nested closure is still attributed to
#: the item it sits in, which is the granularity the question needs — *"how many
#: items in this file write it"* — rather than a call graph.
#:
#: ⛔⛤ **THE `struct` HALF IS NOT SYMMETRY, IT IS A CORRECTION.** With `fn` alone,
#: `NewGameResetRequested`'s write in `menu/kaleidoscope_app.rs` — a `ResMut<'w,
#: ..>` FIELD of the `SystemMenuParams` bundle — was attributed to
#: `capture_armed_rebind`, the nearest `fn` above it, which does not write it at
#: all. ⇒ A site-level instrument that names the WRONG system is worse than a
#: file-level one that names none, because a verdict quoting it reads as
#: measured. A bundle field is reported as `<param bundle: Name>`: the honest
#: answer is that the writer is *whichever systems take this bundle*, and that
#: needs the bundle's own call sites.
_FN_HEADER = re.compile(
    r"^\s*(?:pub(?:\([a-z:]+\))?\s+)?(?:async\s+)?fn\s+([a-z_][A-Za-z0-9_]*)",
    re.M,
)
_STRUCT_HEADER = re.compile(
    r"^\s*(?:pub(?:\([a-z:]+\))?\s+)?struct\s+([A-Z][A-Za-z0-9_]*)",
    re.M,
)


def write_sites(ty: str, files: set[str] | list[str]) -> dict[str, list[str]]:
    """`{file: [enclosing fn name, ...]}` for one type's write sites.

    ⛔⛤ **WHY THIS EXISTS: [`writers`] IS FILE-GRANULAR AND A VERDICT THAT SAYS
    "one owner" IS ABOUT FUNCTIONS.** Measured 2026-09-18 over the 13 types whose
    only second writer file is the session-scope reset: eleven have exactly one
    writing function in the other file, and two do NOT — `ActiveCutscene` has two
    (`drain_cutscene_triggers` and `tick_active_cutscene`) and
    `ProjectileSeqCounter` has three. ⇒ *"would be single-writer without the
    reset"* was false for two of thirteen, and the adjudication guard now checks
    the claim instead of restating it.

    The same comment and test-module strip as [`writers`], so the two cannot
    disagree about what counts as code.
    """
    sites: dict[str, list[str]] = {}
    for f in sorted(files):
        src = strip_test_modules(
            strip_comments(pathlib.Path(f).read_text(encoding="utf-8", errors="replace"))
        )
        heads = [(m.start(), m.group(1)) for m in _FN_HEADER.finditer(src)]
        heads += [
            (m.start(), f"<param bundle: {m.group(1)}>")
            for m in _STRUCT_HEADER.finditer(src)
        ]
        heads.sort()
        found: list[tuple[int, str]] = []
        # ⛔⛤ **THE PATTERNS ARE [`RESMUT`] AND [`WORLD_RESOURCE_MUT`], NOT A THIRD
        # REGEX — AND THE FIRST VERSION OF THIS FUNCTION DID WRITE A THIRD.** It
        # spelled the turbofish arm without the optional trailing comma, so
        # `world.get_resource_mut::<\n    ..::FeatureEcsWorldOverlay,\n>()` in
        # `world/gated_lock_walls.rs` was a writer to [`writers`] and invisible
        # here: the per-file census said 10 files and the per-system view showed
        # 9. ⇒ Two instruments over one population must SHARE the pattern, not
        # agree by inspection; filtering the shared matches by short name cannot
        # drift.
        for pattern in (RESMUT, WORLD_RESOURCE_MUT):
            for m in pattern.finditer(src):
                if m.group(1).split("::")[-1] != ty:
                    continue
                before = [name for start, name in heads if start < m.start()]
                found.append((m.start(), before[-1] if before else "<file scope>"))
        if found:
            sites[f] = [name for _, name in sorted(found)]
    return sites


#: A `ResMut<T>` parameter's BINDING, so an access through it can be found.
#: `mut save: ResMut<AmbitionGameSave>` binds `save`; the `mut` is optional
#: because a system can take `ResMut` immutably-bound and still call `&mut self`
#: methods through it.
#: ⚠ The optional `'w,` is the same hole [`RESMUT`] carried: a `SystemParam`
#: field is `state: ResMut<'w, Foo>`, and without it every bundle's targets were
#: missing from the narrowing table even once its FILE was counted.
_BINDING = (
    r"(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:\s*ResMut<\s*(?:'[a-z_][a-z0-9_]*\s*,\s*)?"
    r"%s\s*(?:<[^<>]*>)?\s*,?\s*>"
)

#: The EXCLUSIVE-WORLD binding, which [`_BINDING`] cannot see because there is no
#: parameter to read a type off:
#:
#:     if let Some(mut pending) = world.get_resource_mut::<PendingLifecycleCommit>()
#:     let mut save = world.resource_mut::<AmbitionGameSave>();
#:
#: ⚠ Without this the narrowing table understates exactly the types
#: [`WORLD_RESOURCE_MUT`] just added — a commit executor's writes would count
#: toward the FILE total while none of its targets joined the join, so
#: `PendingLifecycleCommit` read "record, 2 of 7 files" when two of the other
#: five are the systems that spend the slot.
_WORLD_BINDING = (
    r"mut\s+([A-Za-z_][A-Za-z0-9_]*)\s*\)?\s*=\s*[A-Za-z_][A-Za-z0-9_]*\s*"
    r"\.\s*(?:get_)?resource_mut::<\s*(?:[A-Za-z_][A-Za-z0-9_]*::)*%s\s*,?\s*>"
)

#: An assignment, excluding `==` and `=>`. `+=`, `|=` and friends count.
_ASSIGN = re.compile(r"\s*(?:[+\-*/|&^%]=|=(?![=>]))")

#: The whole-resource target: `*state = Default::default()` replaces every field
#: at once, which is a different kind of authority from touching one of them.
WHOLE = "*<the resource>"


def shared_targets(ty: str, files: set[str] | list[str]) -> dict[str, tuple[set[str], set[str]]]:
    """`{target: (files touching it, files touching it MUTATION-SHAPED)}`.

    ⭐ **THIS IS THE NARROWING STEP, AND IT USED TO BE DONE BY HAND.** "Which
    resources have many writers" is a shortlist; "which FIELD OR METHOD do two of
    those writers both reach for" is what turns one into a question somebody can
    answer. The table it produces lived in the adjudication guard's docstring as
    hand-carried numbers until 2026-09-17.

    ⛔⛤ **TWO COUNTS, BECAUSE ONE OF THEM CANNOT BE DERIVED HONESTLY.** A call
    through a `ResMut` binding may be a read (`save.data()`) or a write
    (`save.data_mut()`), and no regex can tell `queue.record(..)` from
    `queue.len()` without the signature. So this returns BOTH: every writer file
    that TOUCHES the target, and the subset whose access is mutation-shaped —
    an assignment, or a `*_mut` name. Read the wider set as *"where to look"* and
    the narrower as *"where a write is certain"*. Neither is the type's writer
    count, which is a third number: a file can be a `ResMut<T>` writer and share
    no target with anyone.

    ⚠ Only targets reached by TWO OR MORE writer files are returned; a field one
    writer owns alone is the shape this census is looking for, not against.
    """
    binding = re.compile(_BINDING % re.escape(ty))
    world_binding = re.compile(_WORLD_BINDING % re.escape(ty))
    touched: dict[str, set[str]] = collections.defaultdict(set)
    mutated: dict[str, set[str]] = collections.defaultdict(set)
    for f in sorted(files):
        src = strip_test_modules(
            strip_comments(pathlib.Path(f).read_text(encoding="utf-8", errors="replace"))
        )
        # A short type name can be bound under its qualified path, so fall back
        # to the suffix spelling the census already collapses on.
        names = set(binding.findall(src)) | set(world_binding.findall(src))
        if not names:
            names = set(
                re.findall(
                    r"(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:\s*ResMut<[^>]*\b"
                    + re.escape(ty)
                    + r"\s*>",
                    src,
                )
            )
        for name in names:
            for m in re.finditer(r"\*" + re.escape(name) + r"\b", src):
                if _ASSIGN.match(src[m.end() :]):
                    touched[WHOLE].add(f)
                    mutated[WHOLE].add(f)
            for m in re.finditer(re.escape(name) + r"\.([A-Za-z_][A-Za-z0-9_]*)", src):
                target = m.group(1)
                touched[target].add(f)
                rest = src[m.end() :]
                if target.endswith("_mut") or _ASSIGN.match(rest):
                    mutated[target].add(f)
    return {
        target: (fs, mutated.get(target, set()))
        for target, fs in touched.items()
        if len(fs) > 1
    }


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("paths", nargs="*", default=list(DEFAULT_PATHS))
    ap.add_argument(
        "--shared-targets",
        action="store_true",
        help=(
            "for each multi-writer type, also print the fields and methods that "
            "TWO OR MORE of its writer files reach for — the narrowing step that "
            "turns the shortlist into a question somebody can answer"
        ),
    )
    args = ap.parse_args(argv)

    files = production_files(tuple(args.paths))
    if not files:
        # ⛔ An empty corpus would print "0 multi-writer types" and read as a
        # clean bill of health.
        print("no .rs files matched — the instrument found nothing to read")
        return 2

    found = writers(files)
    multi = {t: fs for t, fs in found.items() if len(fs) > 1}
    print(
        f"{len(files)} production files; {len(found)} mutably-reached resource types "
        f"(`ResMut<T>` or `resource_mut::<T>()`); "
        f"{len(multi)} written from >1 file\n"
    )
    for ty, fs in sorted(multi.items(), key=lambda kv: (-len(kv[1]), kv[0])):
        print(f"  {ty}  ({len(fs)} files)")
        for f in sorted(fs):
            print(f"      {f}")
        if not args.shared_targets:
            continue
        targets = shared_targets(ty, fs)
        if not targets:
            print("      shared targets: NONE — no field or method is reached")
            print("        for by two of these files, so there is nothing to join")
            continue
        for target, (touching, writing) in sorted(
            targets.items(), key=lambda kv: (-len(kv[1][1]), -len(kv[1][0]), kv[0])
        ):
            certain = f", {len(writing)} mutation-shaped" if writing else ", none certain"
            print(f"      -> {target}  ({len(touching)} of {len(fs)} files{certain})")
    world = {t: fs for t, fs in session_world_writers(files).items() if len(fs) > 1}
    print(
        f"\n  and {len(world)} SESSION WORLD component(s) written from >1 file "
        "(a different population — see `SESSION_WORLD_MUT`):"
    )
    for ty, fs in sorted(world.items(), key=lambda kv: (-len(kv[1]), kv[0])):
        print(f"  {ty}  ({len(fs)} files)")
        for f in sorted(fs):
            print(f"      {f}")
    print(
        "\n⇒ A SHORTLIST, NOT FINDINGS. For each: what READS this, and can an"
        "\n  ambiguity in it reach a decision? Then POISON one writer and run the"
        "\n  test that should care — a green means nothing tests it OR another"
        "\n  writer covers it, and only the second is a finding."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
