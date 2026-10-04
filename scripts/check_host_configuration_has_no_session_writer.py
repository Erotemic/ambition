#!/usr/bin/env python3
r"""A host-configuration resource has no writer in production code.

Some rollback-registered resources are not session state. They are the
configuration of the host or of the world: a composer installs a value when it
builds the App, and each session reads it. `FriendlyFire` is the baseline rule
that a declared combat ruleset plays over (`project_combat_rules` reads it as
`baseline_ff`). `RegimePolicy` is the regime of the process, and it grants or
denies developer clock requests. `FactionRelations` is the table of which
faction is a foe of which; no production road changes it from the default.

⛔ SUCH A RESOURCE IS NOT IN THE SESSION RESET, AND THAT IS SAFE ONLY WHILE
NOTHING WRITES IT. `SessionScopedResources` (`session/teardown.rs`) sets its
members to the default at each session activation. A reset there would replace
a configured value: `resolved_combat_tuning` sets `FriendlyFire` on a composed
host before the first update and asserts that the fold reads it. So these stay
out of the reset, and they cross the session edge by construction.

MEASURED 2026-10-04 on the shell host (rollback): a value written by hand
before one session was replaced was still there at every tick of the next
session, and `RegimePolicy` is in the peer checksum. While the value is a
constant of the host, that is correct. The first system that writes one of
them inside a session makes it session state that no reset clears, and a
census at tick 0 does not see it, because both hosts hold the constant until
then. This guard is the structural half: it fails when such a writer arrives.

What it checks, over the production Rust files of `crates/` and `game/`
(comments and `#[cfg(test)]` items removed, test files left out):

1. No file takes `&mut` to the resource: `ResMut<T>`, `resource_mut::<T>()`,
   `get_resource_mut::<T>()`, `get_resource_or_init::<T>()`,
   `get_resource_or_insert_with::<T>(..)`, `resource_scope::<T, _>(..)`.
2. Each file that INSTALLS it (`init_resource::<T>()`, `insert_resource(T ..)`)
   is named in `CONFIGURATION`. Those are the composer-time sites. A new one
   is a new composer, and it is named here with its reason.
3. Each named install site still installs it, so a row cannot stay after its
   subject left.
4. The type is declared in the corpus, so a rename cannot make the guard pass
   over nothing.

⚠ THE STATED RESIDUALS. `insert_resource(value)` with a variable is not seen:
the type is not in the call. A `&mut T` parameter is not counted (see
`multi_writer_resource_census.writers` for why). A rollback restore and a
snapshot decode (`SnapshotState for RegimePolicy`) put back a value that was
there; they are not writers of a new one.

When this fails on a writer: if the value must change inside a session, the
resource is session state. Add it to `SessionScopedResources`, give its
configured value another home, and remove its row here.
"""

from __future__ import annotations

import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent / "lib"))
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import multi_writer_resource_census as census  # noqa: E402
from rust_source import strip_comments  # noqa: E402
from test_paths import strip_test_modules  # noqa: E402

#: `{type: (why it is configuration, the files that install it)}`.
CONFIGURATION: dict[str, tuple[str, tuple[str, ...]]] = {
    "FactionRelations": (
        "the table of which faction selects which as a foe; each perception "
        "and target-selection road reads it, and its only value in production "
        "is the default (measured 2026-10-04: equal on each tick of a session "
        "that followed another one and of a fresh host)",
        (
            # `init_targeting_resources`: the combat domain's default.
            "crates/ambition_combat/src/targeting.rs",
        ),
    ),
    "FriendlyFire": (
        "the baseline friendly-fire rule of the world; `project_combat_rules` "
        "folds a declared ruleset over it, and `resolved_combat_tuning` sets it "
        "on a composed host as the world's authored rule",
        (
            # `init_targeting_resources`: the combat domain's default.
            "crates/ambition_combat/src/targeting.rs",
        ),
    ),
    "RegimePolicy": (
        "the regime of the process (`Solo`, `RLDeterministic`, `Cinematic`); "
        "`apply_clock_scale_requests` grants or denies a requester by it",
        (
            # `SimCoreResourcesPlugin`: the default regime, `Solo`.
            "crates/ambition_platformer2d_runtime/src/sim_core_resources.rs",
        ),
    ),
}

#: A corpus smaller than this is a broken `git ls-files`, not a clean tree.
MIN_FILES = 500

_PATH = r"(?:[A-Za-z_][A-Za-z0-9_]*::)*"


def _mutable_reach(ty: str) -> re.Pattern[str]:
    """The `&mut` shapes `census.writers` does not read."""
    return re.compile(
        r"\b(?:get_resource_or_init|get_resource_or_insert_with|resource_scope)"
        rf"::<\s*{_PATH}{ty}\b"
    )


def _install(ty: str) -> re.Pattern[str]:
    return re.compile(
        rf"\binit_resource::<\s*{_PATH}{ty}\s*>"
        rf"|\binsert_resource\(\s*{_PATH}{ty}\b"
    )


def _declared(ty: str) -> re.Pattern[str]:
    return re.compile(rf"\bpub struct {ty}\b")


def findings(files: list[str]) -> list[str]:
    """Each problem as one line. Empty when the tree holds the rule."""
    out: list[str] = []
    writers = census.writers(files)
    sources = {
        f: strip_test_modules(
            strip_comments(pathlib.Path(f).read_text(encoding="utf-8", errors="replace"))
        )
        for f in files
    }
    for ty, (why, allowed) in sorted(CONFIGURATION.items()):
        if not any(_declared(ty).search(src) for src in sources.values()):
            out.append(
                f"`{ty}` is declared in no production file. It was renamed or "
                "deleted; update or remove its row."
            )
            continue
        reach = _mutable_reach(ty)
        writing = set(writers.get(ty, ())) | {
            f for f, src in sources.items() if reach.search(src)
        }
        for f in sorted(writing):
            out.append(
                f"`{ty}` is host configuration ({why}), and {f} takes `&mut` to "
                "it. It is not in the session reset, so a value written inside "
                "a session stays for the next one."
            )
        install = _install(ty)
        installing = {f for f, src in sources.items() if install.search(src)}
        for f in sorted(installing - set(allowed)):
            out.append(
                f"{f} installs `{ty}` and is not a named composer-time site. "
                "Name it in CONFIGURATION with its reason, or install nothing."
            )
        for f in sorted(set(allowed) - installing):
            out.append(
                f"{f} is named as an install site of `{ty}` and installs it no "
                "more. Remove the row."
            )
    return out


def main() -> int:
    files = census.production_files()
    if len(files) < MIN_FILES:
        print(
            f"⛔ only {len(files)} production Rust file(s) found (expected "
            f"{MIN_FILES}+). The corpus is broken, not clean."
        )
        return 1
    problems = findings(files)
    if problems:
        print("⛔ a host-configuration resource has a writer or an unnamed install:\n")
        for problem in problems:
            print(f"  {problem}")
        print("\nSee the module docstring for what to do with a writer.")
        return 1
    print(
        f"OK: {len(CONFIGURATION)} host-configuration resource(s) "
        f"({', '.join(sorted(CONFIGURATION))}) have no production writer over "
        f"{len(files)} files; "
        f"{sum(len(sites) for _, sites in CONFIGURATION.values())} named install site(s)."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
