#!/usr/bin/env python3
"""Rollback rows that are SNAPSHOTTED but contribute nothing to the session checksum.

⛔⛔ **THESE REWIND CORRECTLY AND ARE INVISIBLE TO PEER COMPARISON.**
`rollback_component_clone_probed` installs `rollback_component_with_clone` — the
value is saved and restored — plus a `ChecksumProbe`, which is a desync
LOCALIZATION aid a test runs around a restore. It contributes nothing to the
frame checksum two peers compare. The schema records that in the row's own
detail: *"value-probed for localization, not in the session checksum"*.

⭐ RAISED BY A GUARD THAT FOUND NOTHING. The canonical-finiteness observer
(`game/ambition_app/tests/canonical_state_is_finite.rs`) is exhaustive BY
CONSTRUCTION over the canonical-checksum half, because passing through
`canonical_f32_bits` is what makes a value canonical. It is exhaustively BLIND to
these rows, which never call `encode` at all. A green run there is not "the
canonical state is finite", and this script is how you size the gap.

⚠ **THIS CLASSIFIES, IT DOES NOT RULE.** Promoting a row to canonical changes
what two peers agree about (netcode.md, N3). The float count and the
self-description below are measured; anything about WHEN a divergence would be
read is a reader question this script does not answer.

    python3 scripts/measure_unchecksummed_rollback_rows.py
"""

from __future__ import annotations

import pathlib
import re
import subprocess

REPO = pathlib.Path(__file__).resolve().parents[1]
BASELINE = REPO / "game/ambition_app/tests/rollback_schema_baseline.txt"
DETAIL = "not in the session checksum"
# `Vec2`/`Vec3`/`Quat`/`Rect` are floats wearing a name; `Timer`/`Duration` hold one.
FLOAT_BEARING = ("f32", "f64", "Vec2", "Vec3", "Quat", "Rect", "Timer", "Duration")


def rows() -> list[tuple[str, str]]:
    out = []
    for line in BASELINE.read_text(encoding="utf-8").splitlines():
        if "\t" not in line:
            continue
        parts = line.split("\t")
        if len(parts) >= 4 and parts[1] == "component-clone" and DETAIL in parts[3]:
            out.append((parts[0], parts[2]))
    return out


_DEFINITION_INDEX: dict[str, list[str]] | None = None


def _definition_index() -> dict[str, list[str]]:
    """`type name -> ["path:line", ..]`, built with ONE `git grep`.

    ⚠ NOT AN OPTIMISATION FOR ITS OWN SAKE. This ran one `git grep` per type
    across 59 types, twice over (definition and readers), and the arm that walks
    the population took 20 s of a lane other people wait on. The lane's cost is
    shared; a census that is correct and slow gets run less often, which is its
    own way of not being run.
    """
    global _DEFINITION_INDEX
    if _DEFINITION_INDEX is None:
        index: dict[str, list[str]] = {}
        out = subprocess.run(
            ["git", "grep", "-n", "-E", r"(struct|enum) [A-Z]\w*", "--", "crates/", "game/"],
            capture_output=True,
            text=True,
            cwd=REPO,
        ).stdout.splitlines()
        pattern = re.compile(r"\b(?:struct|enum)\s+([A-Z]\w*)")
        for row in out:
            head, _, body = row.partition(":")
            line, _, rest = body.partition(":")
            match = pattern.search(rest)
            if match:
                index.setdefault(match.group(1), []).append(f"{head}:{line}")
        _DEFINITION_INDEX = index
    return _DEFINITION_INDEX


def definition(ty: str) -> tuple[str, str, str]:
    """`(kind, float-bearing field types, the type's own first doc line)`."""
    found = _definition_index().get(ty, [])
    # ⚠ A TEST FIXTURE MAY DECLARE A TYPE OF THE SAME NAME.
    found = [f for f in found if "/tests" not in f and "tests.rs" not in f]
    if not found:
        return ("?", "?", "")
    path, lineno = found[0].split(":")[0], int(found[0].split(":")[1])
    src = (REPO / path).read_text(encoding="utf-8", errors="replace").split("\n")
    body, depth, started = [], 0, False
    for line in src[lineno - 1 : lineno + 60]:
        body.append(line)
        depth += line.count("{") + line.count("(") - line.count("}") - line.count(")")
        if "{" in line or "(" in line:
            started = True
        if started and depth <= 0:
            break
    text = "\n".join(body)
    kind = "enum" if re.search(rf"enum {ty}\b", text) else "struct"
    floats = sorted({f for f in FLOAT_BEARING if re.search(rf"\b{f}\b", text)})
    doc = ""
    for line in reversed(src[max(0, lineno - 14) : lineno - 1]):
        stripped = line.strip()
        if stripped.startswith("///"):
            content = stripped[3:].strip()
            if content:
                doc = content
        elif stripped.startswith("#[") or stripped == "":
            continue
        else:
            break
    return (kind, ",".join(floats) or "-", doc)


# ---------------------------------------------------------------------------
# THE READER CHECK
#
# ⛔⛔ **THE LATENCY CLASS COMES FROM THE READERS, NOT FROM THE TYPE'S OWN DOC
# COMMENT.** Read by hand on 2026-09-10, the 14 float-free rows were picked as
# likely event-latched state on the reasoning that a row with nothing continuous
# to drift is read on an event. **Ten of them are read every tick.** The prose
# and the readers disagreed for ten of fourteen, so the prose is not evidence.
#
# ⚠ THIS IS A TRIAGE, NOT A VERDICT. A `Query<..&T..>` with no change-detection
# filter runs every tick, so its divergence propagates on the next frame; a
# `Changed<..>`/`Added<..>` filter or a point `get::<T>()` inside a branch may
# not. The second class is what a human must read. This prints the split and
# names which rows need reading rather than deciding for them.

READ_IN_QUERY = re.compile(r"Query\s*<[^;{]{0,400}?&\s*(?:mut\s+)?[\w:]*\b{t}\b", re.S)
GATED = re.compile(r"\b(?:Changed|Added|Or)\s*<")
POINT_READ = re.compile(r"\.get(?:_mut)?::<\s*[\w:]*\b{t}\b")


def reader_sites(ty: str) -> tuple[list[str], list[str]]:
    """`(per-tick query sites, sites a human must read)`."""
    per_tick, needs_reading = [], []
    found = subprocess.run(
        ["git", "grep", "-n", rf"\b{ty}\b", "--", "crates/", "game/"],
        capture_output=True,
        text=True,
        cwd=REPO,
    ).stdout.splitlines()
    for row in found:
        path = row.split(":")[0]
        # ⚠ A TEST, A REGISTRATION AND A RE-EXPORT ARE NOT READERS, and counting
        # them is how a row with no reader at all looks well used.
        if "test" in path or "rollback_registration" in path:
            continue
        body = row.split(":", 2)[-1]
        if re.search(POINT_READ.pattern.format(t=re.escape(ty)), body):
            needs_reading.append(row[:150])
            continue
        # ⛔⛔ TWO DEFECTS THE FIRST RUN OF THIS TRIAGE HAD, both found by
        # checking it against a hand reading rather than believing it:
        #   * `&'static T` -- a lifetime between the `&` and the type. Bevy
        #     `SystemParam` type aliases spell every borrow that way, so
        #     `OwnedPortalGunPair` came back with NO READER when its reader is a
        #     `Query<.. Option<&'static ..OwnedPortalGunPair>>` in a menu. A zero
        #     is a claim about the scan.
        #   * A CHECKSUM PROBE IS NOT A READER. `fn seat_credit_probe(credit:
        #     &SeatCredit)` takes `&T` and reads it, and counting it made a
        #     component that NOTHING in production consults look well used --
        #     hiding the exact case this census exists to surface.
        if "_probe(" in body or "probes.rs" in path:
            continue
        if re.search(rf"&\s*(?:'\w+\s+)?(?:mut\s+)?(?:[\w:]*::)?{re.escape(ty)}\b", body):
            window = "\n".join(
                (REPO / path).read_text(encoding="utf-8", errors="replace").split("\n")[
                    max(0, int(row.split(":")[1]) - 12) : int(row.split(":")[1]) + 4
                ]
            )
            (needs_reading if GATED.search(window) else per_tick).append(row[:150])
    return per_tick, needs_reading


def main() -> int:
    subjects = rows()
    # ⛔ ANTI-VACUITY. An empty read of the baseline prints "0 uncovered rows",
    # which is the reassuring direction.
    assert len(subjects) >= 40, (
        f"only {len(subjects)} rows carry the '{DETAIL}' detail; measured at 59 on "
        "2026-09-10. Either the registration arm was renamed or this scan lost the "
        "baseline — check before believing the number went down."
    )
    floaty = 0
    print(f"⛔ ROLLBACK ROWS OUTSIDE THE SESSION CHECKSUM: {len(subjects)}\n")
    for name, ty in subjects:
        kind, floats, doc = definition(ty)
        if floats != "-":
            floaty += 1
        print(f"   {kind:6} {floats:22} {name:44} {doc[:70]}")
    print(f"\n   {floaty} of {len(subjects)} carry a float-bearing field type.")
    print("\n⚠ Snapshotted and restored; NOT compared between peers. The probe on")
    print("  each is a localization aid, not a checksum contribution.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
