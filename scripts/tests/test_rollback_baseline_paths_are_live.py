"""Every crate named in the rollback schema baseline still exists.

## The miss this closes

`ambition_geometry` was carved out of `ambition_platformer2d_core` on
2026-08-01, and `CenteredAabb` moved with it. That is a WIRE-FORMAT change,
because the fingerprint hashes `std::any::type_name`, which carries the crate
path — same type, same projection, same bytes, different fingerprint, and two
peers that agree about everything refuse to talk.

**It sat in main for four commits.** `cargo check --all-targets` COMPILES tests
without running them, so a green check plus targeted per-crate tests missed it
entirely; the only thing that could see it was one integration test in the app
crate, behind a full build. It was found by going looking for something else.

## Why a static check can catch it at all

The baseline is a text file of `type_name` strings, so each line names the crate
a type lives in — and where a type is DEFINED is greppable. Comparing the two
takes under a second: no build, no app, fast loop.

⛔ **the first version of this guard checked only that the named crate still
EXISTS, and its probe did not fire.** `ambition_platformer2d_core` was never
deleted; `CenteredAabb` moved out of it into another live crate. A guard written
from the memory of a bug rather than from the bug is how you get a check that is
green through its own motivating case — the fifth time that happened on
2026-08-01, and the fifth time the probe was the only thing that noticed.

⚠ **this is a strictly weaker check than the baseline test, on purpose.** It
cannot see a projection change, a new registration, a removed one, or a type
that moved between MODULES of the same crate. It catches one class — the class
that a crate reorganisation produces, which is the campaign the engine is in the
middle of. Do not read a green here as "the schema is unchanged"; the authority
for that is `rollback_schema_baseline.rs`.

⚠ if S30 is decided in favour of hashing below the crate, this guard becomes
pointless and should be DELETED rather than left passing — at that point a crate
path is not a wire-format fact and there is nothing here to protect.
"""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
BASELINE = REPO / "game/ambition_app/tests/rollback_schema_baseline.txt"

# `<name>\t<kind>\t<crate::module::Type>\t<detail>`
TYPE_COLUMN = 2


def _workspace_crates() -> set[str]:
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return {pkg["name"] for pkg in json.loads(out)["packages"]}


def _baseline_entries() -> list[tuple[int, str, str]]:
    """`(line number, crate, type name)` for every `ambition_*` path recorded."""
    out: list[tuple[int, str, str]] = []
    for number, line in enumerate(BASELINE.read_text(encoding="utf-8").splitlines(), 1):
        parts = line.split("\t")
        if len(parts) <= TYPE_COLUMN:
            continue
        # Generic parameters carry their own paths; the head is what owns the type.
        head = re.split(r"[<(]", parts[TYPE_COLUMN].strip())[0]
        segments = head.split("::")
        if len(segments) < 2 or not segments[0].startswith("ambition"):
            continue
        out.append((number, segments[0], segments[-1]))
    return out


def _defining_crates() -> dict[str, set[str]]:
    """Type name → the workspace crates that DEFINE it.

    A grep, deliberately: the question is "where does this name come from", and
    a re-export cannot answer it — `pub use` is where a crate is willing to SHOW
    a type, not where it lives. Types defined in more than one crate are dropped
    rather than guessed at.
    """
    listing = subprocess.run(
        ["git", "grep", "-hnE", r"pub (struct|enum|type) [A-Z][A-Za-z0-9_]*", "--", "crates/", "game/"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=False,
    ).stdout
    paths = subprocess.run(
        ["git", "grep", "-lE", r"pub (struct|enum|type) [A-Z][A-Za-z0-9_]*", "--", "crates/", "game/"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=False,
    ).stdout.split()
    defined: dict[str, set[str]] = {}
    for rel in paths:
        crate_dir = Path(rel).parts[1] if len(Path(rel).parts) > 1 else None
        if crate_dir is None:
            continue
        try:
            source = (REPO / rel).read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        for match in re.finditer(r"pub (?:struct|enum|type) ([A-Z][A-Za-z0-9_]*)", source):
            defined.setdefault(match.group(1), set()).add(crate_dir)
    del listing
    return defined


def test_every_baseline_type_still_lives_in_the_crate_it_names():
    assert BASELINE.is_file(), (
        f"{BASELINE.relative_to(REPO)} is gone, so this guard is watching "
        "nothing. It exists because a crate move silently invalidates that "
        "file; if the baseline moved, point this at it."
    )
    entries = _baseline_entries()
    assert entries, (
        "no `ambition_*` type paths parsed out of the baseline — the file's "
        "column layout changed and this guard is now green about nothing."
    )

    live = _workspace_crates()
    defined = _defining_crates()
    moved: list[str] = []
    for number, crate, type_name in entries:
        homes = defined.get(type_name, set())
        # Undecidable: a name defined nowhere (a foreign type, a macro-generated
        # one) or in several crates. Silence beats a guess.
        if len(homes) != 1:
            continue
        home = next(iter(homes))
        if home not in live or home == crate:
            continue
        moved.append(f"line {number}: `{type_name}` says {crate}, lives in {home}")

    assert not moved, (
        "the rollback schema baseline names the wrong crate for a type, so its "
        "fingerprint no longer matches the live registry:\n  "
        + "\n  ".join(moved)
        + "\n\nA type moved between crates. That IS a wire-format change while "
        "the fingerprint hashes `type_name` (queue S30) — same bytes, different "
        "fingerprint, two peers that cannot agree. Re-run\n  cargo test -p "
        "ambition_app --features rl_sim --test app_it -- rollback_schema_baseline"
        "\nand rewrite the baseline from the live dump, saying why in the commit."
    )
