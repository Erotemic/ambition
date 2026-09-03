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


def _without_comments(source: str) -> str:
    """Blank out `//`, `///` and `//!` lines and trailing `//` comments.

    ⛔ **a checker that greps source reads the prose ABOUT the code as if it were
    the code.** `ambition_platformer2d/src/app.rs` carries
    `/// Not `insert_resource(CharacterCatalog)`: that would be a second
    authority` — a comment stating a resource is NOT inserted, which this script
    would otherwise count as evidence that it IS.

    ⚠ measured before changing anything: stripping comments moves ZERO writers
    and one optional-read (`CharacterCatalog`), so no finding changes today. It
    is closed because the mechanism is real and the failure direction is silent —
    a comment inventing a WRITER hides a capability that does not ship, which is
    the exact defect this script exists to catch.

    ⚠ this repository's Rust policy runner has stripped comments since it was
    written (`rules/source_reference.rs`: *"Comment lines and trailing `//`
    comments are always stripped first, so prose..."*). The Python guards did not
    inherit that, and one of them read its own documentation as evidence before
    this was noticed.
    """
    out = []
    for line in source.splitlines():
        stripped = line.lstrip()
        out.append("" if stripped.startswith("//") else line.split("//", 1)[0])
    return "\n".join(out)


def _is_test(path: Path) -> bool:
    return "tests" in path.parts or path.name in {"tests.rs", "test_support.rs"}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

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
