#!/usr/bin/env python3
"""A capability whose only installer is behind a DEV feature does not ship.


`LocalSeatTopology` is the frozen seat map every couch-multiplayer consumer
agrees about. It was CAPTURED in exactly one non-test place — a function inside
`game/ambition_app/src/dev/rollback_observatory.rs`, whose module is
`#[cfg(feature = "dev_tools")]`. Every other use was `Option<Res<...>>`, a read
that returns early when the resource is absent.

So in any build a player runs, and on desktop until somebody pressed F9:
`reconcile_roster_with_frozen_topology` returned on its first line every frame,
and `assign_local_seat_devices` always took the live-discovery branch its own
docs describe as wrong. **The fix had landed; the mechanism that makes it apply
had not shipped.**

⛔ this is invisible to both the compiler and the test suite. It compiles — the
reader is `Option`. It tests green — every test constructs the resource by hand.
The only detector is a person playing a shipped build and noticing an absence.

# # What it checks

A resource type that is READ as `Option<Res<T>>` / `Option<ResMut<T>>` from
code that ships, and whose only WRITERS (`init_resource::<T>`,
`insert_resource(T { .. })`) sit in modules gated by a non-shipping feature.

⚠ deliberately narrow. `DEV_ONLY_FEATURES` holds the one feature that has
actually caused this, not every feature a shipping persona omits. A wider net
here would fire on platform features (`android_platform` installs things a
desktop build genuinely should not have), and a guard that cries wolf gets
waived — see `feedback_ask_the_tool_dont_model_it`.

⚠ `Option<Res<T>>` is also the correct shape for a genuinely optional
capability. That is why the check is "no shipping writer AT ALL" rather than
"has a dev writer": a capability with both is fine, and one with neither is not
this script's business.
"""

from __future__ import annotations

import argparse
import functools
import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))
from cargo_bin import cargo_binary  # noqa: E402
from rust_source import strip_comments as _without_comments  # noqa: E402,F401

REPO = Path(__file__).resolve().parents[1]

# Features that no shipping persona turns on. `desktop_dev` pulls `dev_tools`,
# and `desktop_dev` is a DEVELOPER persona; `visible` / `android` / `web` do not.
DEV_ONLY_FEATURES = {"dev_tools"}

OPTIONAL_READ = re.compile(
    r"Option\s*<\s*(?:[\w:]+::)?Res(?:Mut)?\s*<\s*(?:[\w:]+::)?(?P<ty>[A-Z]\w+)\s*>"
)
INIT_RESOURCE = re.compile(r"init_resource::<\s*(?:[\w:]+::)?(?P<ty>[A-Z]\w+)\s*>")
INSERT_RESOURCE = re.compile(
    r"insert_resource\s*(?:::<\s*(?:[\w:]+::)?(?P<turbo>[A-Z]\w+)\s*>)?"
    r"\s*\(\s*(?:(?:[\w:]+::)?(?P<ty>[A-Z]\w+)\s*[({]|(?P<local>[a-z_]\w*)\s*\))"
)
# `let mut topology = ambition_input::LocalSeatTopology::default();`
#
# the writer that MOTIVATED this script does not name its type at the call: it builds a local,
# mutates it, and passes the binding.
LOCAL_BINDING = re.compile(
    r"let\s+(?:mut\s+)?(?P<name>[a-z_]\w*)\s*(?::[^=;]+)?=\s*"
    r"(?:[\w:]+::)?(?P<ty>[A-Z]\w+)\s*(?:::\s*(?:default|new)\s*\(|\{)"
)
# `#[cfg(feature = "x")]` directly above a `mod y;`
GATED_MOD = re.compile(
    r'#\[cfg\(\s*feature\s*=\s*"(?P<feature>[^"]+)"\s*\)\]\s*'
    r'(?:pub(?:\([^)]*\))?\s+)?mod\s+(?P<module>\w+)\s*;'
)
PLAIN_MOD = re.compile(r"(?:pub(?:\([^)]*\))?\s+)?mod\s+(?P<module>\w+)\s*;")


@functools.cache
def _crate_roots() -> list[Path]:
    out = subprocess.run(
        [cargo_binary(), "metadata", "--no-deps", "--format-version", "1"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    meta = json.loads(out)
    ids = set(meta["workspace_members"])
    roots = []
    for pkg in meta["packages"]:
        if pkg["id"] not in ids:
            continue
        src = Path(pkg["manifest_path"]).parent / "src"
        for entry in ("lib.rs", "main.rs"):
            if (src / entry).is_file():
                roots.append(src / entry)
    return roots


@functools.cache
def _source(path: Path) -> str:
    """Read one Rust source file once per checker process."""
    return path.read_text(encoding="utf-8", errors="ignore")


def _resolve_mod(parent: Path, module: str) -> Path | None:
    for candidate in (
        parent.parent / f"{module}.rs",
        parent.parent / module / "mod.rs",
        parent.parent / parent.stem / f"{module}.rs",
        parent.parent / parent.stem / module / "mod.rs",
    ):
        if candidate.is_file():
            return candidate
    return None


@functools.cache
def _gates_by_file() -> dict[Path, frozenset[str]]:
    """Every source file → the features gating it, walked from each crate root.

    A file inherits its parent module's gates. `dev/mod.rs` being
    `#[cfg(feature = "dev_tools")]` is what makes everything under `dev/`
    dev-only, and that is the shape S35 had.
    """
    gates: dict[Path, frozenset[str]] = {}
    for root in _crate_roots():
        stack: list[tuple[Path, frozenset[str]]] = [(root, frozenset())]
        while stack:
            path, inherited = stack.pop()
            if path in gates and gates[path] <= inherited:
                continue
            gates[path] = inherited if path not in gates else gates[path] & inherited
            source = _source(path)
            gated = {m["module"]: m["feature"] for m in GATED_MOD.finditer(source)}
            for match in PLAIN_MOD.finditer(source):
                module = match["module"]
                child = _resolve_mod(path, module)
                if child is None:
                    continue
                extra = {gated[module]} if module in gated else set()
                stack.append((child, inherited | extra))
    return gates


# ⛔⛤ NOT A RESPELLING. `_without_comments` used to blank whole comment-only
# lines and split trailing `//` comments by hand, one of six drifted copies of
# the same rule across `scripts/`. It moved to `lib/rust_source.strip_comments`
# unchanged in effect for this file's own patterns — `Option<Res<>>` and
# `init_resource::<>` are matched by `.finditer()` over the whole blob, never by
# line shape, so the owner's narrower "keep indentation, drop only the `//...`
# suffix" behaviour finds the same writers and reads. Routed 2026-09-18: before
# and after this file's own report are byte-identical
# (`every Option-read capability has at least one shipping writer`,
# 1547 files, 192 optional-read types).


#: ⭐ MOVED to `scripts/lib/test_paths.py` 2026-09-16 and re-exported here. This
#: was one of FIVE copies that had drifted into five different answers; this one
#: was the narrowest, missing `test.rs` and `*_tests.rs`, and like all five it
#: missed a file whose inner `#![cfg(test)]` compiles it out entirely.
#: ⚠ The union sees MORE test files, so this check sees FEWER production ones —
#: the green direction. `POPULATION_FLOOR` above is what makes that reviewable.
from test_paths import is_test_path  # noqa: E402


def _is_test(path: Path) -> bool:
    return is_test_path(path)


#: What this scan must still be able to SEE. ⛔ THE FAILURE MODE OF A
#: SOURCE-READING GUARD IS A CLEAN REPORT. Every finding here needs BOTH an
#: `Option<Res<T>>` reader and a writer to be found; lose either pattern and the
#: intersection empties silently, which is indistinguishable from "every
#: capability ships".
#:
#: ⚠ THIS SCRIPT IS A KNOWN DIVERGENCE POINT. Its `_is_test` matches
#: `test_support.rs` but NOT `test.rs` or `*_tests.rs` — the narrowest of the
#: five copies of that rule in `scripts/` (see `GUARD-CORPUS` in
#: `docs/planning/queue.md`). Widening it toward the others REMOVES files from
#: `production files`, which is the green direction, so the floor exists to make
#: that change reviewable rather than invisible.
POPULATION_FLOOR = {
    "files scanned": 1450,
    "production files": 1250,
    "optional read types": 175,
    "writer types": 380,
}


def population_sizes() -> dict[str, int]:
    gates = _gates_by_file()
    production = [path for path in gates if not _is_test(path)]
    optional: set[str] = set()
    writers: set[str] = set()
    for path in production:
        source = _without_comments(_source(path))
        optional.update(match["ty"] for match in OPTIONAL_READ.finditer(source))
        writers.update(match["ty"] for match in INIT_RESOURCE.finditer(source))
    return {
        "files scanned": len(gates),
        "production files": len(production),
        "optional read types": len(optional),
        "writer types": len(writers),
    }


def population_shortfalls() -> list[str]:
    sizes = population_sizes()
    return [
        f"{label}: {sizes[label]} visible, floor is {floor}"
        for label, floor in POPULATION_FLOOR.items()
        if sizes[label] < floor
    ]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

    # ⛔ BEFORE ANY FINDING. A finding here is an INTERSECTION of two patterns,
    # so losing either one empties the report and reads as "every capability
    # ships" — the exact answer this guard exists to doubt.
    shortfalls = population_shortfalls()
    if shortfalls:
        print(
            "the scan lost reach — it can no longer see part of its own "
            "population:\n\n  " + "\n  ".join(shortfalls) + "\n\n"
            "Find the pattern or path rule that stopped matching before "
            "trusting any verdict here. If the drop is legitimate, lower "
            "POPULATION_FLOOR in the same commit that causes it.",
            file=sys.stderr,
        )
        return 1

    gates = _gates_by_file()
    optional_reads: dict[str, set[Path]] = {}
    writers: dict[str, set[Path]] = {}

    for path, features in gates.items():
        if _is_test(path):
            continue
        source = _without_comments(_source(path))
        for match in OPTIONAL_READ.finditer(source):
            optional_reads.setdefault(match["ty"], set()).add(path)
        bindings = {m["name"]: m["ty"] for m in LOCAL_BINDING.finditer(source)}
        for match in INIT_RESOURCE.finditer(source):
            writers.setdefault(match["ty"], set()).add(path)
        for match in INSERT_RESOURCE.finditer(source):
            ty = match["turbo"] or match["ty"] or bindings.get(match["local"] or "")
            if ty:
                writers.setdefault(ty, set()).add(path)

    findings: list[str] = []
    for ty, read_sites in sorted(optional_reads.items()):
        write_sites = writers.get(ty, set())
        if not write_sites:
            # Written nowhere at all: either it comes from outside the workspace
            # or it is genuinely optional. Not this script's question.
            continue
        shipping_writers = [
            path for path in write_sites if not (gates[path] & DEV_ONLY_FEATURES)
        ]
        if shipping_writers:
            continue
        dev = ", ".join(
            sorted(str(path.relative_to(REPO)) for path in write_sites)
        )
        readers = sorted(str(path.relative_to(REPO)) for path in read_sites)
        findings.append(
            f"{ty}: every writer is dev-only ({dev}); "
            f"{len(readers)} shipping reader(s) take it as Option and return "
            f"early forever, e.g. {readers[0]}"
        )


    if findings:
        print("CAPABILITIES THAT DO NOT SHIP:\n")
        for line in findings:
            print(f"  {line}")
        print(
            "\nA capability installed only behind a dev feature is absent from "
            "every build a player runs, and it is invisible to the compiler (the "
            "reader is Option) and to the suite (tests build the resource by "
            "hand). Register it where the capability's LIFETIME begins — for a "
            "session-scoped one, that is the session, not a debug overlay."
        )
        return 1

    # the counts are on the SUCCESS line, not behind `--verbose`. They were
    # behind it when this script was written earlier today, which meant its
    # ordinary output — "every Option-read capability has at least one shipping
    # writer" — was indistinguishable from a run that scanned nothing. A crate
    # root that stopped resolving, a `cargo metadata` shape change, a module walk
    # that returned early: all of those print the same clean sentence.
    if not gates:
        print(
            "scanned NO source files — the crate walk is broken, not the code. "
            "A pass here would mean nothing.",
            file=sys.stderr,
        )
        return 1
    print(
        f"every Option-read capability has at least one shipping writer "
        f"({len(gates)} files, {len(optional_reads)} optional-read types)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
