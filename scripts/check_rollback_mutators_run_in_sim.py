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

## ⛔ three ways the cheap version of this check lies (S35, all paid for)

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

## What it deliberately is not

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

# Schedules that do NOT rewind. `sim_schedule()` resolves to `Update` for a
# fixed-tick host and to the GGRS schedule for a rollback one — going through it
# is exactly what this check is asking for, and a literal is what it flags.
NON_REWINDING = ("Update", "PostUpdate", "PreUpdate", "FixedUpdate")
# ⚠ `Startup` is deliberately ABSENT. It runs once, before any rewind exists, so
# writing rollback state there is INITIALISATION — it is what the first snapshot
# is taken of. Including it reported `setup_simulation_system` seeding
# `MovingPlatformSet`, which is the correct way to do that and not a defect.

_CANONICAL = re.compile(r"rollback_(?:component|resource)_canonical::<([^>]+)>")
_PUB_FN = re.compile(r"\bfn\s+([a-z_][a-z_0-9]*)\s*\(")
_CFG_TEST = re.compile(r"#\[cfg\(test\)\]\s*mod\s+[A-Za-z_][A-Za-z_0-9]*\s*\{")

# ── Waivers ──
#
# name → why mutating rollback state outside the rewinding schedule is correct
# here. An entry is a claim that a value's drift across a rewind does not matter,
# which is a strong claim and should read like one — so both entries below cite
# the code that makes them true rather than asserting it.
WAIVERS: dict[str, str] = {
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
    return "tests" in path.parts or path.name in {"tests.rs", "test.rs"}


def _production_sources(repo: Path):
    for root in SOURCE_ROOTS:
        if not (repo / root).is_dir():
            continue
        for src in sorted((repo / root).rglob("*.rs")):
            if _is_test_path(src):
                continue
            text = strip_test_modules(strip_comments(src.read_text(errors="replace")))
            yield src, text


def rollback_types(repo: Path = REPO) -> set[str]:
    """Every type registered for canonical rollback, by its bare name."""
    registry = repo / ROLLBACK_REGISTRY.relative_to(REPO)
    if not registry.is_file():
        return set()
    return {
        m.group(1).strip().split("::")[-1]
        for m in _CANONICAL.finditer(registry.read_text(errors="replace"))
    }


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


def mutating_systems(repo: Path = REPO) -> dict[str, list[str]]:
    """system name → the rollback types its signature takes mutably."""
    types = rollback_types(repo)
    found: dict[str, list[str]] = {}
    for _src, text in _production_sources(repo):
        for match in _PUB_FN.finditer(text):
            params = _params(text, match.end())
            hits = sorted(
                t
                for t in types
                if re.search(rf"(?:&mut\s+|ResMut\s*<\s*)[\w:]*\b{t}\b", params)
            )
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
        print(f"{len(rollback_types())} rollback-registered types")
        print(f"{len(mutators)} systems take one mutably\n")

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
