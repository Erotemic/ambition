#!/usr/bin/env python3
"""Pick the integration suite a turn actually needs, and run it.

⭐ **Jon, 2026-08-08, verbatim:**

> *"I want to bias towards running less tests to balance out the agent urge to
> run more. So yes run a much much smaller suite for docs only changes. We will
> catch regressions eventually."*

The measurement that earned the ruling: `app_it` was **94.6% of all Stop-check
time** (158.4 s of 161.1 s), `cargo check` was 0.5 s warm, and 114 checks in
11h17m came to roughly **five hours of suite** — most of it on turns that edited
nothing but planning prose.

⛔ **The "we will catch regressions eventually" clause is the cost, taken
knowingly.** A smaller gate does not make a regression less likely; it makes it
arrive later than the turn that caused it. That trade is the maintainer's and he
made it. So this script must not hedge it back — a "small" suite that quietly
stays large would be disobeying the ruling, not being careful.

## Whitelist, never blacklist

Only `docs/` and the measurements submodule are skippable. Everything else —
source, assets, `.ron` data the tests read, generated files, other submodule
pointers, `Cargo.toml`, this script — forces the full suite.

The asymmetry is deliberate and is the whole safety argument: **the failure mode
of the RULE is a slower turn, and the failure mode of the SUITE is a later
catch.** A blacklist ("skip when no `.rs` changed") inverts that — assets and
generated files are not `.rs` and do change behaviour — so it is not offered even
as an option.

⭐ **`dev/ambition_dev_measurements` is the second entry and it is NOT prose**, so
it is named for what it is: a write-only RECORD of what past runs cost. It earns
the exemption twice over.

* Nothing in the build or test graph reads it. This script runs
  `cargo test -p ambition_app --test app_it`, and no row of telemetry can change
  what that reports. ⚠ the one measurement artifact that IS an input to a check —
  `dev/compile_ratchet_baseline.json`, which the bare
  `python3 scripts/compile_ratchet.py` gate reads on every run — deliberately
  stayed OUT of the submodule and is still non-prose, so it still forces the
  full suite when it moves.
* Without the exemption the prose-only case would be dead within one suite run.
  `run_tests.py` appends a row to `run_tests_cost.jsonl` on EVERY run, which
  leaves the submodule's working tree dirty, which `git status --porcelain` in
  the PARENT reports as a modified `dev/ambition_dev_measurements`. So the next
  turn — the pure-prose one this ruling exists for — would see a non-prose path
  and run the full suite, for a file it did not touch and cannot be affected by.

## What the smoke subset is FOR

⭐ Not "the tests that could break from editing a document" — that set is empty,
which is the point of the ruling. The smoke subset exists to catch **this script
being wrong**: a path that should have forced the full suite and did not. So the
modules are chosen to fail loudly if the app stops composing at all, and they are
named with a reason each rather than picked by runtime. A subset chosen by
runtime is one nobody can defend, and it rots silently as tests are added.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]

#: The ONLY prefix that counts as prose. See the whitelist note above.
PROSE_PREFIXES = ("docs/",)

#: Prose, plus the append-only measurement submodule — everything whose change
#: cannot alter what the suite would report. ⛔ this is the whole whitelist; a
#: prefix added here is a class of change nobody gates on again.
SKIPPABLE_PREFIXES = PROSE_PREFIXES + ("dev/ambition_dev_measurements",)

#: Named smoke modules, each with the reason it is here. ⛔ do not add a module
#: because it is fast, and do not remove one because it is slow — the question is
#: only "would this fail if the app stopped composing".
SMOKE_MODULES: dict[str, str] = {
    "composes_through_the_sdk": (
        "the public composition path builds an app at all — if a non-prose "
        "change slipped through the whitelist and broke wiring, this is where "
        "it shows"
    ),
    "direct_and_shell_agree": (
        "the two ways to build the game still agree; one test, and it is the "
        "cheapest proof that neither composition drifted"
    ),
}


def changed_paths(since: str | None) -> list[str]:
    """Every path this turn touched — working tree AND commits since `since`.

    ⚠ **both halves matter.** Uncommitted work is what a Stop hook is usually
    looking at, and committed work is what a turn that already committed left
    behind. Reading only one of them was the first draft's bug: a turn that
    committed a `.rs` change and then edited a doc would have looked prose-only.
    """
    paths: set[str] = set()

    status = subprocess.run(
        ["git", "-C", str(REPO), "status", "--porcelain"],
        capture_output=True, text=True, timeout=60,
    )
    for line in status.stdout.splitlines():
        if len(line) > 3:
            entry = line[3:]
            # a rename reads "old -> new"; both sides count
            for part in entry.split(" -> "):
                paths.add(part.strip().strip('"'))

    if since:
        diff = subprocess.run(
            ["git", "-C", str(REPO), "diff", "--name-only", f"{since}..HEAD"],
            capture_output=True, text=True, timeout=60,
        )
        if diff.returncode == 0:
            paths.update(p for p in diff.stdout.splitlines() if p.strip())

    return sorted(paths)


def is_skippable_only(paths: list[str]) -> bool:
    """True when EVERY changed path is one the full suite cannot have an opinion about.

    ⛔ an empty change set is NOT skippable. "Nothing changed" and "only docs
    changed" are different states, and answering the first with a smoke suite
    would mean a turn that somehow reported no diff silently skips the gate.
    """
    if not paths:
        return False
    return all(p.startswith(SKIPPABLE_PREFIXES) for p in paths)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--since", help="git ref the turn started from (optional)")
    ap.add_argument("--explain", action="store_true",
                    help="print the decision and the paths, run nothing")
    args = ap.parse_args(argv)

    paths = changed_paths(args.since)
    skippable = is_skippable_only(paths)

    if skippable:
        filters = list(SMOKE_MODULES)
        why = (f"{len(paths)} path(s), all under "
               f"{' or '.join(SKIPPABLE_PREFIXES)}")
    else:
        filters = []
        offenders = [p for p in paths if not p.startswith(SKIPPABLE_PREFIXES)]
        why = (f"{len(offenders)} gated path(s), first: "
               f"{offenders[0] if offenders else '(none — empty diff)'}")

    print(f"gate_suite: {'SMOKE' if skippable else 'FULL'} — {why}")
    if skippable:
        for name, reason in SMOKE_MODULES.items():
            print(f"  + {name}: {reason}")

    if args.explain:
        for p in paths:
            print(f"    {p}")
        return 0

    cmd = ["cargo", "test", "-p", "ambition_app", "--test", "app_it", "--quiet"]
    if filters:
        cmd += ["--", *filters]
    print(f"  $ {' '.join(cmd)}", flush=True)
    return subprocess.run(cmd, cwd=str(REPO)).returncode


if __name__ == "__main__":
    raise SystemExit(main())
