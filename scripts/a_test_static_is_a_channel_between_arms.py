#!/usr/bin/env python3
"""An interior-mutable `static` in test code is a channel between arms.

⛔⛤ **THIS EXISTS BECAUSE THE SAME DEFECT PRODUCED THREE MEASURED FAILURES IN ONE
DAY, AND TWO OF THEM COST SIX DAYS OF SEARCHING THE ENGINE.** `app_it` runs its
arms as THREADS OF ONE PROCESS, so a `static` with interior mutability in test
code is shared by every arm that reaches it. Measured 2026-09-16:

* `landing_repeatedly`'s cadence phase lived in a `static AtomicUsize`, two arms
  drew from it, and the arm under test never landed — its subject held one value
  and it reported *"1 distinct census at the frames the audit COMPARED"*;
* `playing`'s cadence did the same to `how_much_of_the_peer_checksum_actually_varies`,
  which reported **99** registered types written outside the rewinding schedule.
  Controlled: 15 of 15 runs pass with a per-call cadence, 8 of 10 FAIL with the
  `static` restored;
* a per-App page census read a process-global ledger keyed by a per-App asset id
  and lost 79 of 149 rows depending on who else was running.

⇒ The two hypotheses on the triage page were "a plugin leaks state" and "two Apps
interfere while simultaneous", and the answer was that **they are the same thing
when the shared state is in the MEASUREMENT.** Both pointed at the engine, because
that is where a reader hunting shared state looks. This checker points at the
harness.

⚠ **`thread_local!` IS THE REMEDY AND IS NOT REPORTED.** A per-thread cell is
per-arm under libtest, which is the property the reported form gives up. A
per-call closure (`fn cadence() -> impl FnMut() -> T`) is better still, because it
is per-USE rather than per-thread.

⚠ **AND A `static` IS NOT WRONG IN GENERAL.** A serialising lock, a cross-arm
unique sequence and a once-built immutable cache all want exactly this. Those are
rows in `ADJUDICATED`, each with the reason. What is wrong is a MEASUREMENT whose
state is shared.
"""

from __future__ import annotations

import importlib.util
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "scripts"))


def _load(name: str):
    spec = importlib.util.spec_from_file_location(name, REPO / "scripts" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


#: ⭐ REUSED: `_is_test_path` (both test-file conventions plus the `#[path]` road)
#: and `strip_test_modules` (inline `#[cfg(test)] mod` bodies, by brace balance).
#: Each of those filters was added because a fixture had crossed a corpus
#: boundary in one direction or the other.
_MUTATORS = _load("check_rollback_mutators_run_in_sim")

#: A `static` whose type can be mutated through a shared reference. ⛔ NOT
#: `thread_local!`: that is the remedy, and it is excluded by requiring `static`
#: to start the declaration rather than follow a macro's open paren.
MUTABLE_STATIC = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?static\s+([A-Z_0-9]+)\s*:\s*[^=;]*"
    r"(?:Mutex|RwLock|OnceLock|OnceCell|LazyLock|Lazy|RefCell|Cell\s*<|Atomic\w+)"
)

#: An inner `#![cfg(test)]` compiles the whole file out of a release build.
FILE_IS_TEST_ONLY = re.compile(r"^[ \t]*#!\[cfg\(test\)\]", re.M)

#: ⛔⛤ **`thread_local!` DECLARES ITS CELLS WITH THE `static` KEYWORD, so the
#: remedy reads exactly like the defect one line down.** The first version of this
#: checker reported two `thread_local!` cells as unadjudicated channels — the
#: macro opens a block and the `static` lines sit inside it. The block is removed
#: before scanning rather than matched around, because the cells are written one
#: per line and a look-behind would only see the nearest.
THREAD_LOCAL = re.compile(r"\bthread_local!\s*[{(]")


def _without_thread_locals(text: str) -> str:
    """`text` with every `thread_local! { … }` body removed, by brace balance."""
    while (match := THREAD_LOCAL.search(text)) is not None:
        opener = text[match.end() - 1]
        closer = "}" if opener == "{" else ")"
        depth, index = 1, match.end()
        while index < len(text) and depth:
            if text[index] == opener:
                depth += 1
            elif text[index] == closer:
                depth -= 1
            index += 1
        text = text[: match.start()] + text[index:]
    return text

#: Test-side statics that are deliberate, by `(repo-relative path, NAME)`.
#: ⛔ A ROW HERE IS A DECISION THAT CROSS-ARM SHARING IS THE POINT, not a waiver
#: that the sharing is harmless. Read the uses before adding one.
ADJUDICATED: dict[tuple[str, str], str] = {
    (
        "crates/ambition_persistence/src/settings/persistence/tests.rs",
        "TEST_DIR_LOCK",
    ): "a serialising lock: cross-arm exclusion is the whole job",
    (
        "game/ambition_app/tests/unified_melee.rs",
        "UNIFIED_MELEE_TEST_LOCK",
    ): "a serialising lock: cross-arm exclusion is the whole job",
    (
        "game/ambition_app/tests/gravity_symmetry_room.rs",
        "FAILURE_DUMP_SEQ",
    ): "cross-arm UNIQUENESS is the job — it names failure-dump files, and two "
    "arms must not pick the same one",
    (
        "crates/ambition_platformer2d_actor_monolith/src/construction/tests.rs",
        "CAST",
    ): "a `OnceLock` built by `get_or_init` from a deterministic builder, read "
    "only afterwards: every arm sees the same value, and none can change it",
    (
        "crates/ambition_platformer2d_actor_monolith/src/world/rooms/stage.rs",
        "CAST",
    ): "the same once-built immutable fixture cast as the row above",
    (
        "crates/ambition_platformer2d_actor_monolith/src/construction/tests.rs",
        "MECHANICS",
    ): "the SessionMechanics wrapper around that same cast, once-built by "
    "`get_or_init` and read only afterwards: `GenerationMechanics` borrows it "
    "for `'static`, so a per-arm local could not outlive the plan it is handed "
    "to, and no arm holds a handle that could change it",
    (
        "crates/ambition_platformer2d_actor_monolith/src/construction/tests.rs",
        "CANDIDATE_INSERTIONS",
    ): "a single-arm recorder, reset at entry to each phase. ⚠ LATENT: a second "
    "arm touching it inherits the defect this checker exists for",
    (
        "crates/ambition_platformer2d_shared_tangle/src/construction/tests.rs",
        "USE_B",
    ): "a single-arm switch, stored before each phase. ⚠ LATENT, as above",
    (
        "crates/ambition_platformer2d_shared_tangle/src/construction/tests.rs",
        "DISPATCHES",
    ): "a single-arm counter, zeroed before each phase. ⚠ LATENT, as above",
    (
        "crates/ambition_platformer2d_shared_tangle/src/construction/tests.rs",
        "ROOTS",
    ): "a single-arm recorder declared inside its own fn, cleared at entry. "
    "⚠ LATENT, as above",
    (
        "crates/ambition_platformer2d_shared_tangle/src/construction/tests.rs",
        "SEEN",
    ): "a single-arm hook log declared inside its own fn, cleared at entry. "
    "⚠ LATENT, as above",
}


def _tracked_rust() -> list[str]:
    return subprocess.run(
        ["git", "-C", str(REPO), "ls-files", "*.rs"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()


def _cfg_test_bodies(text: str) -> str:
    """The inline `#[cfg(test)] mod … { … }` bodies — the inverse of stripping them."""
    bodies, src = [], text
    while (match := _MUTATORS._CFG_TEST.search(src)) is not None:
        depth, index = 1, match.end()
        while index < len(src) and depth:
            if src[index] == "{":
                depth += 1
            elif src[index] == "}":
                depth -= 1
            index += 1
        bodies.append(src[match.start():index])
        src = src[: match.start()] + src[index:]
    return "\n".join(bodies)


def statics_in(scope: str) -> list[tuple[str, str]]:
    """`(NAME, declaration)` for each interior-mutable static in one span of text.

    ⚠ **THE WHOLE SCAN, so a unit arm and the repository arm exercise the same
    path.** The first version applied `_without_thread_locals` at the call site
    instead, and poisoning that call left the `thread_local!` arm green while only
    the repository ratchet noticed — a unit test of a helper is not a test of the
    wiring that uses it.
    """
    found: list[tuple[str, str]] = []
    for line in _without_thread_locals(scope).split("\n"):
        match = MUTABLE_STATIC.match(line)
        if match:
            found.append((match.group(1), line.strip()))
    return found


def test_side_statics() -> list[tuple[str, str, str]]:
    """`(path, NAME, declaration)` for every interior-mutable static in test code."""
    found: list[tuple[str, str, str]] = []
    for rel in _tracked_rust():
        if "/target/" in rel or rel.startswith(".worktrees"):
            continue
        path = REPO / rel
        try:
            text = path.read_text(errors="replace")
        except OSError:
            continue
        if _MUTATORS._is_test_path(path) or FILE_IS_TEST_ONLY.search(text):
            scope = text
        else:
            scope = _cfg_test_bodies(text)
            if not scope:
                continue
        for name, line in statics_in(scope):
            found.append((rel, name, line))
    return found


def main() -> int:
    statics = test_side_statics()
    unadjudicated = [row for row in statics if (row[0], row[1]) not in ADJUDICATED]
    print(
        f"{len(statics)} interior-mutable static(s) in test code, "
        f"{len(ADJUDICATED)} adjudicated"
    )
    if not statics:
        # ⛔ EVERY FILTER HERE REMOVES FILES. A scan that stopped matching prints
        # the same clean verdict as a tree with none.
        print("⛔ NO test-side static found at all — the corpus or the pattern is wrong.")
        return 1
    stale = sorted(set(ADJUDICATED) - {(row[0], row[1]) for row in statics})
    for path, name in stale:
        print(f"✔ GONE, remove from ADJUDICATED: {path}::{name}")
    if not unadjudicated and not stale:
        print("ok: every test-side static is a decision somebody wrote down.")
        return 0
    for rel, name, line in unadjudicated:
        print(f"\n⛔ {rel}::{name}\n   {line}")
    if unadjudicated:
        print(
            "\n⇒ `app_it` RUNS ITS ARMS AS THREADS OF ONE PROCESS, so this is shared\n"
            "  by every arm that reaches it. If it holds state a MEASUREMENT reads,\n"
            "  make it per-use (`fn f() -> impl FnMut() -> T`) or per-thread\n"
            "  (`thread_local!`). If cross-arm sharing is the point, add it to\n"
            "  ADJUDICATED with the reason."
        )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
