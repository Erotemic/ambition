#!/usr/bin/env python3
"""Does anything mutate ROLLBACK state from a schedule that never rewinds?

A component registered for rollback is restored on every rewind. A system that
mutates it must therefore run inside the schedule GGRS resimulates — reached via
`app.sim_schedule()` — and not in a literal `Update`.

⛔ **under a fixed-tick host the two are the same schedule**, so the mistake
costs nothing and shows nothing. Under GGRS they are different: the value
rewinds, the mutation does not replay with it, and the meter drifts a little
further from the peer's every rewind. Nothing crashes and no test fails.

This found nothing new on the day it was written, which is the point — it was
written *after* `regen_player_mana` turned up by hand (S34). That system mutates
`BodyMana`, registered as `body.mana`, from the app's HUD chain in `Update`, at
render rate. Its own doc even stated the rule it was breaking: *"a sim mutator
never lives in presentation"*. The code had moved out of the render module; the
`add_systems` call had not.

# # three ways the cheap version of this check lies (S35, all paid for)

The first draft returned 87 rows and essentially all were artifacts:

1. **a system named `update`.** Searching for its name near an `add_systems(
   Update, …)` also matches the `app.update();` in nearly every test helper —
   about 80 of those 87 rows. Reading each call by PAREN BALANCE removes it by
   construction: `app.update()` is not inside the parentheses.
2. **a fixed-size text window.** Reading N characters after `add_systems(` runs
   past the end of that call into the next one, attributing systems to schedules
   they were never registered in.
3. **`#[cfg(test)]` modules inside PRODUCTION files.** Skipping `tests.rs` and
   `tests/` is not enough — `vortex.rs` and `mary_o/powerups.rs` both register
   rollback mutators into `Update` inside an inline `fn test_app()`. That is
   correct, and it looks exactly like the defect.

⚠ and the parsing rules of the sibling guard apply unchanged: comments are not
registrations, and `run_if`/`after`/`before` name systems somebody ELSE
registered. Both are imported rather than reimplemented.

# # What it deliberately is not

⚠ it reads the CANONICAL rollback registrations as the source of truth for what
is snapshot state. A type rolled back through some other path is invisible here.

⚠ and it cannot see a system registered through a helper, a macro, or a variable
holding a schedule label other than the `sim_schedule()` idiom. The shape it
catches is a literal `Update`/`PostUpdate`/`PreUpdate`/`FixedUpdate`, which is
the shape the one real instance had.

Usage:
    python3 scripts/check_rollback_mutators_run_in_sim.py
    python3 scripts/check_rollback_mutators_run_in_sim.py --list
"""

from __future__ import annotations

import argparse
import functools
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "scripts"))

from check_engine_systems_are_engine_installed import (  # noqa: E402
    add_systems_bodies,
    strip_comments,
    strip_run_conditions,
)

SOURCE_ROOTS = ["crates", "game"]
ROLLBACK_REGISTRY = REPO / "crates/ambition_platformer2d_runtime/src/rollback/mod.rs"

# Schedules that do NOT rewind.
NON_REWINDING = ("Update", "PostUpdate", "PreUpdate", "FixedUpdate")
# `Startup` is deliberately ABSENT.

# ⭐ RESOURCE CLONE REGISTRATIONS ARE IN THE POPULATION SINCE 2026-09-16, and the
# COMPONENT ones deliberately are not. They are snapshotted and restored on every
# rewind exactly like the canonical ones, so an outside mutation drifts
# identically — the distinction was never about rollback semantics, only about
# which spelling the first version of this file happened to grep for.
#
# ⛔ THE COMPONENT HALF STAYS OUT, AND NOT FOR LACK OF NERVE. MEASURED: widening
# to ALL registrations surfaces 65 unwaived offenders, and most are `Transform`
# writes from camera, sprite and inspection systems — PRESENTATION reading a
# component that happens to be rollback-registered. That needs a way to tell a
# simulation write from a presentation write, which does not exist yet.
# Resources carry no `Transform`, so this half costs 9 findings instead of 65
# and is separable. See `ROLLBACK-MUTATOR-POPULATION` in `docs/planning/queue.md`.
#
# ⚠ SO `RoomSet` AND `LdtkRuntimeIndex` ARE STILL INVISIBLE: both are
# `rollback_component_clone_checksum`. That is why `handle_ldtk_hot_reload`'s
# waiver reads stale and why widening the RESOURCE half does not cure it.
_ROLLBACK_REGISTRATION = re.compile(
    r"rollback_(?:component|resource)_canonical::<([^>]+)>"
    r"|rollback_resource_clone(?:_checksum)?::<([^>]+)>"
)
_PUB_FN = re.compile(r"\bfn\s+([a-z_][a-z_0-9]*)\s*\(")
_CFG_TEST = re.compile(r"#\[cfg\(test\)\]\s*mod\s+[A-Za-z_][A-Za-z_0-9]*\s*\{")
# ⛔⛤ `SessionWorldMut<T>` IS A MUTABLE PARAM AND WAS INVISIBLE UNTIL
# 2026-09-15. A10.3d rewrote `handle_ldtk_hot_reload`'s `ResMut<RoomSet>` as
# `SessionWorldMut<RoomSet>`, and this scanner stopped seeing the system at all —
# its waiver went "stale" while the mutation was untouched. Measured when the
# hole was found: SIX distinct types are reached through this param and ALL SIX
# are rollback-registered. A guard keyed on how a write is SPELLED goes blind
# when a refactor respells it, and the direction is the dangerous one — it
# reports no offenders.
# ⚠ THE OPTIONAL LIFETIME IS NOT COSMETIC. A system signature elides it
# (`ResMut<T>`), but a `#[derive(SystemParam)]` FIELD cannot (`ResMut<'w, T>`),
# so the lifetime-free form matched every function and no struct — and the
# struct bodies are where 13 rollback-registered types were hiding.
_MUTABLE_PARAM_TYPE = re.compile(
    r"(?:&mut\s+|ResMut\s*<\s*|SessionWorldMut\s*<\s*)"
    r"(?:'[a-z_][A-Za-z_0-9]*\s*,\s*)?"
    r"(?:[A-Za-z_][A-Za-z_0-9]*::)*([A-Z][A-Za-z_0-9]*)\b"
)

#: A `#[derive(SystemParam)]` bundle: the derive, then the struct it decorates.
_SYSTEM_PARAM_STRUCT = re.compile(
    r"#\[derive\([^)]*\bSystemParam\b[^)]*\)\]"
    r"(?:\s*#\[[^\]]*\])*"
    r"\s*(?:pub(?:\([^)]*\))?\s+)?struct\s+([A-Za-z_][A-Za-z_0-9]*)"
)

# ── Waivers ──
#
# name → why mutating rollback state outside the rewinding schedule is correct
# here. An entry is a claim that a value's drift across a rewind does not matter,
# which is a strong claim and should read like one — so both entries below cite
# the code that makes them true rather than asserting it.
WAIVERS: dict[str, str] = {
    # ── added 2026-09-02 with the widening, triaged by ambition-df ───────────
    # These six appeared the moment `rollback_types` stopped reading one file.
    # Each is waived with the reason its drift across a rewind does not matter,
    # in this table's existing idiom: state what was CHECKED, not what the name
    # suggests.
    "ask_for_the_split_observer_view": (
        "⛔ a CAPTURE BINARY, never a rollback host. "
        "`game/ambition_demo_twintrack_app/src/bin/capture_twintrack.rs` builds "
        "its own app to record a comparison; it starts no GGRS session, so "
        "`TwinTrackExperiment` has no timeline to be inconsistent with."
    ),
    "materialize_projectiles_for_this_tick": (
        "⛔ the FIGHTER HARNESS builds its OWN app and cannot be composed into a "
        "rollback host. Checked rather than inferred: `fighter_harness.rs` does "
        "`App::new()` + `MinimalPlugins` and steps it with `self.app.update()`; "
        "the file contains no GGRS/rollback reference of any kind; and "
        "`FighterHarness` appears in exactly ONE file in the workspace — its own "
        "— so no composition can hand it a session. `ProjectileSeqCounter` "
        "therefore never rewinds under it. One reason for all three harness "
        "systems."
    ),
    "spawn_projectiles_from_brain_actions": (
        "⛔ fighter harness — same composition as "
        "`materialize_projectiles_for_this_tick`: it steps the sim from `Update` "
        "with no rollback host, so `BodyKinematics`/`BodyMelee` do not rewind."
    ),
    "tick_body_cooldowns": (
        "⛔ fighter harness — same composition again. Checked at the registration, "
        "not inferred from the file name: `fighter_harness.rs` adds all three of "
        "these to `Update` in the app it builds itself, and that app installs no "
        "GGRS host, so `BodyMelee` is never restored and there is no history for "
        "the cooldown tick to be inconsistent with."
    ),
    "restore_inventory_from_save": (
        "⚠ WAIVED FOR THE ACTIVATION CASE ONLY, AND THE OTHER CASE IS OPEN. "
        "It writes `BodyWallet` from `Update` while applying a save. At session "
        "activation no sim tick has advanced, so there is no history to diverge "
        "from. ⛔ BUT THE CODE EXPLICITLY SUPPORTS A MID-SESSION LOAD — "
        "`durable_horizon.rs` says 'THE `Update` ADOPTER STAYS. A file can also "
        "arrive after activation (a mid-session load), and adoption is "
        "idempotent' — so the pre-first-tick condition is NOT provable from the "
        "code and is not asserted here. Idempotence makes the write consistent "
        "with itself, which is not the same as consistent with a peer that never "
        "applied it. Queue row owes the mid-session question."
    ),
    "handle_ldtk_hot_reload": (
        "⛔ a hot reload DISCARDS the rollback timeline rather than continuing "
        "it. `restart_local_ggrs_after_hot_reload` runs in the same module and "
        "calls `stop_session` then `start_sync_test_session`, so there is no "
        "history for the mutation to be inconsistent with. Checked at that "
        "function, not inferred from the word 'reload'."
    ),
    "refresh_world_time": (
        "⛔ NOT installed by any composition. `ambition_time::TimePlugin` — the "
        "only thing that registers this into `Update` — is added exactly once in "
        "the whole workspace, inside that crate's own `#[test] fn`. Production "
        "registers `refresh_world_time` through `player_schedule` into the sim "
        "schedule, which is correct. Verified by two routes: no `add_plugins` "
        "hit outside the test, and every other `TimePlugin` mention resolves to "
        "`bevy::time::TimePlugin`.\n"
        "⚠ so the plugin is a TRAP rather than a defect: correct today because "
        "nobody uses it, wrong the moment somebody does. `ambition_time` is a "
        "content-free crate with no access to `sim_schedule()`, so it cannot fix "
        "this itself — which is the argument for deleting the plugin rather than "
        "keeping a registration nothing exercises."
    ),
}


def strip_test_modules(source: str) -> str:
    """Remove inline `#[cfg(test)] mod … { … }` blocks by brace balance.

    ⛔ trap 3 above. These modules legitimately register rollback mutators into
    `Update` — a test app has no GGRS schedule to reach — and they sit inside
    production files, so path-based test filtering never sees them.
    """
    while (match := _CFG_TEST.search(source)) is not None:
        depth = 1
        index = match.end()
        while index < len(source) and depth:
            if source[index] == "{":
                depth += 1
            elif source[index] == "}":
                depth -= 1
            index += 1
        source = source[: match.start()] + source[index:]
    return source


def _is_test_path(path: Path) -> bool:
    """Is this file test-only, so its `Update` registrations are fixtures?

    ⚠ **`*_tests.rs` is the repo's other test-file convention and this used to
    miss all 51 of them.** A file named `foo_tests.rs` is always declared as
    `#[cfg(test)] mod foo_tests;` (sometimes through a `#[path]` attribute, which
    is why a naive parent grep does not see it), so it cannot carry a production
    registration — the same reason `strip_test_modules` drops inline
    `#[cfg(test)] mod` blocks out of production files. The first such file to
    register a rollback mutator into `Update` was flagged as a real breach.
    """
    return (
        "tests" in path.parts
        or path.name in {"tests.rs", "test.rs"}
        or path.name.endswith("_tests.rs")
    )


#: A whole file compiled out of a release build. ⛔ THE NAME CONVENTIONS ABOVE
#: ARE A PROXY AND THIS IS THE FACT. `features/ecs/fighter_harness.rs` is a test
#: harness living beside the code it exercises, declared `mod fighter_harness;`
#: with no `#[cfg(test)]` on the declaration and `#![cfg(test)]` INSIDE the file
#: — so every name rule above passes it and its `Update` registrations entered
#: the production corpus as if they shipped.
#:
#: ⚠ MEASURED 2026-09-16: 4 files carry this attribute and ALL FOUR were missed,
#: so the proxy had no overlap with the fact at all. Found by chasing
#: `materialize_projectiles_for_this_tick`, which read as a serious breach —
#: projectile spawning is simulation, not presentation — and was a fixture.
_FILE_IS_TEST_ONLY = re.compile(r"^[ \t]*#!\[cfg\(test\)\]", re.M)


@functools.cache
def _production_sources(repo: Path = REPO) -> tuple[tuple[Path, str], ...]:
    """Read and normalize each production Rust source once per repository.

    `collect()` needs the same source corpus twice: once to discover which
    systems mutate rollback state and once to find where those systems are
    scheduled. Re-reading and re-stripping every Rust file for the second pass
    adds no information inside one checker invocation.
    """
    found: list[tuple[Path, str]] = []
    for root in SOURCE_ROOTS:
        if not (repo / root).is_dir():
            continue
        for src in sorted((repo / root).rglob("*.rs")):
            if _is_test_path(src):
                continue
            text = strip_test_modules(strip_comments(src.read_text(errors="replace")))
            # An inner `#![cfg(test)]` compiles the WHOLE file out, so nothing in
            # it can be a production registration whatever the file is called.
            if _FILE_IS_TEST_ONLY.search(text):
                continue
            found.append((src, text))
    return tuple(found)


@functools.cache
def rollback_types(repo: Path = REPO) -> set[str]:
    """Every type registered for canonical rollback, by its bare name.

    ⛔⛔ THIS READ ONE FILE UNTIL 2026-09-02, AND THE GUARD WAS GREEN OVER 1% OF
    ITS OWN STATED POPULATION. It scanned only
    `crates/ambition_platformer2d_runtime/src/rollback/mod.rs`, which holds a
    single `rollback_component_canonical::<…>` — `MovingPlatformSet`. The
    workspace holds **87 such registrations across 10 files, naming 113 types**
    (`ambition_characters`, `ambition_combat`, `platformer2d_core`,
    `actor_monolith`, `shared_tangle`, `projectiles`, …). So "4 systems mutate
    rollback state, none registered into a non-rewinding schedule" read as a
    clean bill of health for rollback and certified one type.

    ⭐ Registration is DISTRIBUTED — each crate owns its own
    `rollback_registration.rs` — so any single-file registry is a snapshot of
    one crate, not the source of truth this docstring always claimed to read.
    Scanning the same production sources the mutator scan uses keeps the two
    halves over one population by construction.
    """
    return {
        (m.group(1) or m.group(2)).strip().split("::")[-1]
        for _path, text in _production_sources(repo)
        for m in _ROLLBACK_REGISTRATION.finditer(text)
    }


#: What the scan must still be able to SEE. ⛔⛔ EVERY HOLE THIS GUARD HAS EVER
#: HAD FAILED IN THE GREEN DIRECTION — `&mut T` alone (2026-09-02, 1 type
#: visible of 113), `SessionWorldMut<T>` (2026-09-15, six types), and
#: `#[derive(SystemParam)]` (2026-09-16, THIRTEEN registered types behind one
#: identifier). Each time the report got SHORTER and CLEANER, and each time that
#: read as good news.
#:
#: ⭐ SO THE COUNT IS PART OF THE VERDICT. A fifth spelling that hides forty
#: types cannot announce itself, but it cannot avoid making these numbers FALL.
#: Raise a floor when the tree genuinely grows; a DROP is the signature of the
#: next hole and must fail loudly rather than pass quietly.
POPULATION_FLOOR = {
    "rollback types": 139,
    "system param bundles": 55,
}


def population_sizes(repo: Path = REPO) -> dict[str, int]:
    return {
        "rollback types": len(rollback_types(repo)),
        "system param bundles": len(system_param_mutables(repo)),
    }


def population_shortfalls(repo: Path = REPO) -> list[str]:
    """⚠ A guard that reads source cannot tell "clean tree" from "broken scan"
    by looking at its own output. This is the term whose correct value is known
    in advance, so the two stop looking alike."""
    sizes = population_sizes(repo)
    return [
        f"{label}: {sizes[label]} visible, floor is {floor}"
        for label, floor in POPULATION_FLOOR.items()
        if sizes[label] < floor
    ]


def _braced(text: str, open_brace: int) -> str:
    depth, index = 0, open_brace
    while index < len(text):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[open_brace : index + 1]
        index += 1
    return text[open_brace:]


@functools.cache
def system_param_mutables(repo: Path = REPO) -> dict[str, frozenset[str]]:
    """`#[derive(SystemParam)]` bundle name → every type it borrows mutably.

    ⛔⛔ THE FOURTH SPELLING OF A MUTABLE WRITE, AND THE WORST ONE. The scanner
    reads function SIGNATURES, so a system taking `resources: SessionScopedResources`
    showed ONE opaque identifier and this guard reported nothing. That single
    bundle holds 25 `ResMut` fields, **13 of them rollback-registered** —
    `MovingPlatformSet` among them, which this file's own `rollback_types`
    docstring records as the ENTIRE population the guard could see before
    2026-09-02. So the guard could be blind to a mutation of the very type it
    was built around, and read clean while doing it. MEASURED 2026-09-16; the
    workspace has 60 such bundles across 42 files, so this is a pattern rather
    than one site.

    ⇒ Same disease as the `SessionWorldMut<T>` hole above, and the fix is the
    same shape: resolve the spelling generally instead of naming the site.
    Nesting is resolved transitively, because a bundle may hold a bundle.
    """
    direct: dict[str, set[str]] = {}
    nested: dict[str, set[str]] = {}
    for _path, text in _production_sources(repo):
        for match in _SYSTEM_PARAM_STRUCT.finditer(text):
            brace = text.find("{", match.end())
            if brace < 0:
                continue
            body = _braced(text, brace)
            direct[match.group(1)] = set(_MUTABLE_PARAM_TYPE.findall(body))
            nested[match.group(1)] = set(re.findall(r"\b([A-Z][A-Za-z_0-9]*)\b", body))

    resolved: dict[str, frozenset[str]] = {}

    def resolve(name: str, seen: frozenset[str]) -> frozenset[str]:
        if name in resolved:
            return resolved[name]
        if name in seen:
            return frozenset()  # a cycle cannot add anything new
        out = set(direct.get(name, ()))
        for candidate in nested.get(name, ()):
            if candidate != name and candidate in direct:
                out |= resolve(candidate, seen | {name})
        answer = frozenset(out)
        if not (seen - {name}):
            resolved[name] = answer
        return answer

    return {name: resolve(name, frozenset()) for name in direct}


def _params(text: str, open_paren: int) -> str:
    depth = 1
    index = open_paren
    while index < len(text) and depth:
        if text[index] == "(":
            depth += 1
        elif text[index] == ")":
            depth -= 1
        index += 1
    return text[open_paren : index - 1]


@functools.cache
def mutating_systems(repo: Path = REPO) -> dict[str, list[str]]:
    """system name → the rollback types its signature takes mutably.

    Extract mutable parameter type names once and intersect with the rollback
    registry. The old implementation ran one regular expression per rollback
    type for every function signature — hundreds of regex searches for each
    candidate function even though a signature names only a handful of types.
    """
    types = rollback_types(repo)
    bundles = system_param_mutables(repo)
    found: dict[str, list[str]] = {}
    for _src, text in _production_sources(repo):
        for match in _PUB_FN.finditer(text):
            params = _params(text, match.end())
            mutated = set(_MUTABLE_PARAM_TYPE.findall(params))
            # A bundle named in the signature contributes what IT borrows
            # mutably; see `system_param_mutables` for why one identifier can
            # stand for thirteen registered types.
            for identifier in re.findall(r"\b([A-Z][A-Za-z_0-9]*)\b", params):
                mutated |= bundles.get(identifier, frozenset())
            hits = sorted(types.intersection(mutated))
            if hits:
                found.setdefault(match.group(1), hits)
    return found


def collect(repo: Path = REPO) -> list[tuple[str, str, str, list[str]]]:
    """(system, file, schedule, mutated types) registered outside the rewind."""
    mutators = mutating_systems(repo)
    findings: list[tuple[str, str, str, list[str]]] = []
    seen: set[tuple[str, str]] = set()
    for src, text in _production_sources(repo):
        for body in add_systems_bodies(text):
            schedule, _, rest = body.partition(",")
            schedule = schedule.strip()
            if schedule not in NON_REWINDING:
                continue
            rest = strip_run_conditions(rest)
            for name, hits in mutators.items():
                if name in WAIVERS:
                    continue
                if re.search(rf"\b{name}\b", rest):
                    key = (name, str(src))
                    if key in seen:
                        continue
                    seen.add(key)
                    findings.append(
                        (name, str(src.relative_to(repo)), schedule, hits)
                    )
    return sorted(findings)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--list", action="store_true", help="also print what was scanned")
    args = parser.parse_args()

    mutators = mutating_systems()
    findings = collect()

    if args.list:
        for label, size in population_sizes().items():
            print(f"{size} {label} (floor {POPULATION_FLOOR[label]})")
        print(f"{len(mutators)} systems take one mutably\n")

    # ⛔ BEFORE THE FINDINGS, because "no offenders" over a collapsed population
    # is the failure this guard cannot otherwise report. A shortfall means the
    # SCAN lost reach, and every clean line below it would be a lie.
    shortfalls = population_shortfalls()
    if shortfalls:
        print(
            "the scan lost reach — it can no longer see part of its own "
            "population:\n\n  " + "\n  ".join(shortfalls) + "\n\n"
            "Every hole this guard has had reported FEWER offenders, not more, "
            "so a falling count is the symptom and an empty report is the "
            "reward. Find the write spelling that stopped matching before you "
            "trust any verdict from this file. If the drop is legitimate — "
            "registrations really were deleted — lower POPULATION_FLOOR in the "
            "same commit that deletes them.",
            file=sys.stderr,
        )
        return 1

    if findings:
        lines = [
            f"  {name}  ({', '.join(hits)})\n    registered into {schedule} at {file}"
            for name, file, schedule, hits in findings
        ]
        print(
            "rollback state is mutated from a schedule that never rewinds:\n\n"
            + "\n".join(lines)
            + "\n\nThe value is restored on every rewind and this mutation is not "
            "replayed with it, so it drifts from the peer's a little further each "
            "time — silently, and only under GGRS, because a fixed-tick host runs "
            "the same schedule either way.\n\n"
            "Register it through `app.sim_schedule()` instead. If it genuinely "
            "belongs outside the rewind, add it to WAIVERS with the reason its "
            "drift across a rewind does not matter.",
            file=sys.stderr,
        )
        return 1

    print(
        f"OK: {len(mutators)} systems mutate rollback state, none registered into "
        f"a non-rewinding schedule."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
