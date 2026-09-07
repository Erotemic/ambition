#!/usr/bin/env python3
"""Ask which composition-level installer CALLS the test suite can actually see.

⭐ WHY THIS EXISTS. The C2 carve pattern turns N inline `add_systems` lines into
ONE `install_*(app, schedule)` call. That is the point -- one authority for an
ordering fact -- and it also concentrates the failure into a single deletable
line. Measured 2026-09-07 on the first one anybody checked: deleting
`install_save_mirror(app, sim)` from `progression_schedule.rs` leaves the ENTIRE
`app_it` suite green, 583 passed, because the two systems' own tests
(`save_sync/actor_liveness_tests.rs`) call `app.add_systems(...)` themselves. A
test that CONSTRUCTS its subject can never witness the shipped app failing to
install it.

⇒ So the question this answers is not "are these systems tested" -- they are --
but "would anything notice if the COMPOSITION stopped calling them". Those are
different questions and the crate-level suites cannot tell them apart.

⭐ FIRST FULL RUN, 2026-09-07 at `c02c3f6f7`, ~70 minutes, result in
`dev/installer_call_coverage.json`: **7 of 14 -- exactly half -- can be deleted
with `app_it` green.** The seven:

    combat_schedule.rs:472     ambition_damage::install_staged_hit_lifecycle_guard
    progression_schedule.rs:69 monolith::features::install_save_mirror
    progression_schedule.rs:98 ambition_menu::map::install_map_simulation_systems
    sim_core_resources.rs:196  monolith::time::time_control::install_sim_clock_reporting
    host/src/lib.rs:65         ambition_input::install_provider_action_road
    host/src/lib.rs:66         ambition_input::install_seat_device_tracking
    host/src/lib.rs:443        ambition_render::fx::install_fx_pipeline

⚠ ONE OF THE SEVEN IS EXPECTED AND SHOULD NOT BE COUNTED AS A HOLE:
`install_fx_pipeline` is the render FX pipeline and `app_it` is headless. Reading
"7 of 14" without that caveat overstates the finding by one. The other six are
simulation-side.

⚠ AND THE OTHER SEVEN ARE WELL WITNESSED -- 121, 57, 42, 21, 17, 11 and 1 failing
tests. The distribution is what makes this a finding rather than a claim that the
suite is weak: the same suite that catches an input carve with 121 failures
notices nothing when the save mirror leaves.

ⓘ `install_staged_hit_lifecycle_guard` was ALREADY KNOWN, and the fix stopped one
level short. `ambition_damage/src/tests.rs` carries
`the_lifecycle_guard_installer_registers_the_guard`, whose own doc says "removing
the call from `combat_schedule.rs` leaves `app_it` at 578/578" -- so the gap was
measured, and the test written for it builds its OWN `App` and calls the installer
itself. It witnesses that the installer works. Nothing witnesses that the
composition calls it.

⛔ IT EDITS THE WORKING TREE, one line at a time, and restores after each subject.
The tree is shared, so: the original bytes are written to a sidecar BEFORE the
edit, every restore is verified with `git diff --quiet`, and the run ABORTS on the
first restore that does not verify rather than continuing to poison more files.
`--restore` alone recovers a tree left dirty by a killed run.

⚠ A `compile_error` outcome is NOT a coverage result. It means the call cannot be
removed in isolation (an unused import or binding becomes an error), which is a
weaker kind of witness than a failing test and is reported separately rather than
counted as covered.

Usage::

    python3 scripts/measure_installer_call_coverage.py --list
    python3 scripts/measure_installer_call_coverage.py --run
    python3 scripts/measure_installer_call_coverage.py --restore
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
SIDECAR = REPO / "target" / "installer-coverage-sidecar.json"

# The compositions this asks about: the engine-side hosts that assemble
# capabilities. The game crate's own `install_*` helpers are a different
# question (a crate calling its own function) and are out of scope here.
COMPOSITIONS = [
    "crates/ambition_platformer2d_runtime/src",
    "crates/ambition_platformer2d_host/src",
]

CALL = re.compile(r"^\s*[A-Za-z_][A-Za-z_0-9:]*install_[a-z_0-9]+\(.*\);\s*$")

TEST_CMD = [
    "cargo", "test", "-p", "ambition_app", "--test", "app_it", "-q",
]


def subjects() -> list[tuple[str, int, str]]:
    """`(relative path, 1-based line number, the call)` for every subject."""
    out = []
    for root in COMPOSITIONS:
        for path in sorted((REPO / root).rglob("*.rs")):
            if path.name.endswith("_tests.rs") or path.name == "tests.rs":
                continue
            rel = str(path.relative_to(REPO))
            for number, line in enumerate(path.read_text().splitlines(), 1):
                if CALL.match(line):
                    out.append((rel, number, line.strip()))
    return out


def _write_line(rel: str, number: int, replacement: str) -> None:
    path = REPO / rel
    lines = path.read_text().splitlines(keepends=True)
    lines[number - 1] = replacement
    tmp = path.with_suffix(path.suffix + ".measuring")
    tmp.write_text("".join(lines))
    os.replace(tmp, path)


def _clean(rel: str) -> bool:
    """Is this path free of working-tree changes?

    ⚠ `--ignore-submodules=all`, because this repo carries submodule pointers that
    are routinely dirty and are nobody's business here. Without it the first
    version of this check called a pristine tree dirty and refused to run.
    """
    return subprocess.run(
        ["git", "diff", "--quiet", "--ignore-submodules=all", "--", rel], cwd=REPO
    ).returncode == 0


def restore() -> int:
    """Put back whatever a killed run left poisoned."""
    if not SIDECAR.exists():
        print("no sidecar; nothing to restore")
        return 0
    saved = json.loads(SIDECAR.read_text())
    rel, number, original = saved["file"], saved["line"], saved["original"]
    _write_line(rel, number, original)
    SIDECAR.unlink()
    if not _clean(rel):
        print(f"⛔ {rel} is STILL dirty after restore — inspect it by hand", file=sys.stderr)
        return 1
    print(f"restored {rel}:{number}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--list", action="store_true", help="name the subjects and stop")
    parser.add_argument("--run", action="store_true", help="poison each subject in turn")
    parser.add_argument("--restore", action="store_true", help="undo a killed run")
    parser.add_argument("--json", help="write results here")
    args = parser.parse_args()

    if args.restore:
        return restore()

    found = subjects()
    # ⛔ ANTI-VACUITY: a pattern that matches nothing reports a clean sweep.
    if len(found) < 5:
        print(
            f"⛔ only {len(found)} installer call(s) found across {COMPOSITIONS}. "
            "The pattern broke or the compositions moved; this is not a result.",
            file=sys.stderr,
        )
        return 1

    if args.list or not args.run:
        for rel, number, call in found:
            print(f"  {rel}:{number}  {call}")
        print(f"\n{len(found)} subject(s)")
        return 0

    # ⭐ ASK ABOUT WHAT THIS WILL EDIT, not about the tree. A dirty file elsewhere
    # is somebody else's work and no business of this measurement; a dirty file
    # HERE means a restore would overwrite an edit that is not ours.
    dirty = sorted({rel for rel, _, _ in found if not _clean(rel)})
    if dirty:
        print(
            "⛔ these files carry uncommitted changes and this measurement edits "
            f"them: {', '.join(dirty)}",
            file=sys.stderr,
        )
        return 1

    results = []
    for index, (rel, number, call) in enumerate(found, 1):
        original = (REPO / rel).read_text().splitlines(keepends=True)[number - 1]
        SIDECAR.parent.mkdir(parents=True, exist_ok=True)
        SIDECAR.write_text(json.dumps({"file": rel, "line": number, "original": original}))
        indent = original[: len(original) - len(original.lstrip())]
        _write_line(rel, number, f"{indent}// REMOVED BY MEASUREMENT: {call}\n")

        done = subprocess.run(TEST_CMD, cwd=REPO, capture_output=True, text=True)
        blob = done.stdout + done.stderr
        if "error[E" in blob or "error: could not compile" in blob:
            verdict = "compile_error"
        elif done.returncode == 0:
            verdict = "ALL GREEN — nothing witnessed the removal"
        else:
            failed = re.search(r"(\d+) failed", blob)
            verdict = f"caught by {failed.group(1) if failed else '?'} failing test(s)"

        _write_line(rel, number, original)
        SIDECAR.unlink(missing_ok=True)
        if not _clean(rel):
            print(f"⛔ ABORTING: {rel} did not restore cleanly", file=sys.stderr)
            return 1

        print(f"[{index}/{len(found)}] {rel}:{number}  {call}\n      -> {verdict}", flush=True)
        results.append({"file": rel, "line": number, "call": call, "verdict": verdict})

    unseen = [r for r in results if r["verdict"].startswith("ALL GREEN")]
    print(f"\n  {len(unseen)} of {len(results)} installer calls can be deleted with app_it green")
    for r in unseen:
        print(f"      {r['file']}:{r['line']}  {r['call']}")
    if args.json:
        Path(args.json).write_text(json.dumps(results, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
