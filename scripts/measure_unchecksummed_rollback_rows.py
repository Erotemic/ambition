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


def definition(ty: str) -> tuple[str, str, str]:
    """`(kind, float-bearing field types, the type's own first doc line)`."""
    found = subprocess.run(
        ["git", "grep", "-n", "-E", rf"(struct|enum) {ty}\b", "--", "crates/", "game/"],
        capture_output=True,
        text=True,
        cwd=REPO,
    ).stdout.splitlines()
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
