#!/usr/bin/env python3
"""Which checksummed rollback resources does installing the BACKEND demand?

`bevy_ggrs`'s `ResourceChecksumPlugin` system takes `Res<R>` — unwrapped — so a
declared checksummed resource PANICS on any frame it is absent, taking the whole
`App` down (Bevy 0.19 turns a missing system parameter into a hard failure).

`AmbitionRollbackPlugin` calls `register_engine_rollback_state`, which declares
twenty domains' state unconditionally. So every checksummed resource those
domains declare becomes a prerequisite of the BACKEND, whether or not the host
composed the domain that inserts it.

This prints that population and, for each, who inserts the resource — so a
minimum-host composition can be judged against a list rather than discovered one
panic at a time. It reports; it does not gate.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

# The two declaration forms that route through `checksum_resource`, and
# therefore through `ResourceChecksumPlugin`'s unwrapped `Res<R>`. The
# `_optional_canonical` form is deliberately EXCLUDED: it exists precisely
# because it checksums `Option<T>` and tolerates absence.
CHECKSUMMED = (
    "rollback_resource_canonical",
    "rollback_resource_clone_checksum",
    "rollback_resource_clone_checksum_with_schema_detail",
)
# Files that DEFINE the vocabulary rather than use it.
DEFINITION_SITES = ("registration.rs", "registrar.rs", "snapshot.rs")


def git_grep(pattern: str, *paths: str) -> list[tuple[str, str]]:
    out = subprocess.run(
        ["git", "grep", "-n", "-E", pattern, "--", *paths],
        capture_output=True,
        text=True,
    ).stdout
    rows = []
    for line in out.splitlines():
        path, _, rest = line.partition(":")
        rows.append((path, rest))
    return rows


def main() -> int:
    root = Path(subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True, text=True, check=True,
    ).stdout.strip())

    pattern = "|".join(f"{name}::<" for name in CHECKSUMMED)
    engine: list[tuple[str, str]] = []
    game: list[tuple[str, str]] = []
    for path, rest in git_grep(pattern, "crates", "game", "examples"):
        if any(path.endswith(site) for site in DEFINITION_SITES):
            continue
        match = re.search(r"::<([^>]+)>", rest)
        if not match:
            continue
        ty = match.group(1).strip().rsplit("::", 1)[-1]
        (game if path.startswith("game/") else engine).append((ty, path))

    print("CHECKSUMMED ROLLBACK RESOURCES — each is a `Res<R>` a missing host cannot skip\n")
    print(f"ENGINE-side ({len(engine)}) — declared through `register_engine_rollback_state`,")
    print("so installing `AmbitionRollbackPlugin` demands every one of them:\n")
    for ty, path in sorted(engine):
        inserters = [
            p for p, _ in git_grep(
                rf"(init_resource::<[A-Za-z0-9_:]*{re.escape(ty)}>|insert_resource\([A-Za-z0-9_:]*{re.escape(ty)})",
                "crates", "game",
            )
            if "test" not in p
        ]
        print(f"  {ty}")
        print(f"      declared  {path}")
        if inserters:
            for p in sorted(set(inserters)):
                print(f"      inserted  {p}")
        else:
            print("      inserted  (no production inserter found — it may be spawned by a system)")
    print(f"\nGAME-side ({len(game)}) — declared by a game's own composition, not by the backend:")
    for ty, path in sorted(game):
        print(f"  {ty:<28} {path}")
    print(
        "\n⚠ A resource whose only inserter is a plugin the host does not add is a "
        "guaranteed frame-one panic.\n"
        "⇒ See engine/capability-and-runtime-composition.md, "
        "'the rollback backend declares twenty domains' state'."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
