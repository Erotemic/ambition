#!/usr/bin/env python3
"""A schedule whose AUTHORIZATION is "only the commit executor runs it".

⛔⛤ **THE GUARANTEE IS MADE OUT OF *WHEN* A SYSTEM RUNS, SO IT IS ONLY AS TRUE
AS THE NUMBER OF PLACES THAT RUN IT.** `CheckpointDomainApply`'s own doc states
the contract: *"This schedule is run by the common commit executor and by
nothing else... A domain reducer that lives in it therefore cannot act on an
unadmitted request, an unprepared room, or a crossing that was cancelled before
its destructive application: there is no path from a raw `ResetToCheckpoint` to
running this."* Every word of that is a claim about CALL SITES.

⭐ **AND IT REPLACED A TOKEN, WHICH IS WHY THE REGRESSION IS SILENT.** A1c/1-2
published a one-frame `AdmittedCheckpointRestore` that every domain reducer had
to remember to read; A1c/3b deleted it precisely because *"running them from the
commit instead makes the same guarantee out of WHEN they run, which no reducer
can forget"*. A second runner does not reintroduce the token — it reintroduces
the token's PROBLEM with none of the token's visibility, because there is no
longer any value a reducer could consult to notice.

⛔⛔ **THE EXISTING ARM HOLDS THE OTHER HALF AND CANNOT SEE THIS ONE.**
`domain_restoration_is_registered_in_the_commit_schedule_and_not_in_the_simulation`
(`actor_monolith/src/session/checkpoint/tests.rs`) asserts POPULATIONS — that
`CheckpointRestore` holds exactly 2 systems and `CheckpointDomainApply` exactly
3 — so a fourth domain re-adding itself to the simulation set reddens it. It
says nothing about who RUNS the schedule, and a second `try_run_schedule` leaves
it green along with every other arm in that module. Measured 2026-09-18.

⚠ **WHY A SOURCE SCAN RATHER THAN A GRAPH QUERY.** Bevy exposes a schedule's
systems, not its callers: `run_schedule`/`try_run_schedule` is an ordinary method
call on `World`, invisible to `Schedules`. The call sites are the only evidence,
so this reads source — which is why comments and test modules are stripped by the
shared owner (`lib/rust_source`, `lib/test_paths`) rather than by a local regex.
A paragraph saying *"nothing else runs this"* is not a runner.

⚠ **TEST RUNNERS ARE EXCLUDED AND THAT IS A DELIBERATE HOLE.** A fixture driving
the schedule directly is how the reducers get arms at all. The contract is about
production roads; a test that runs it has an author who chose to.

    python3 scripts/check_commit_only_schedules_have_one_runner.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from lib.rust_source import strip_comments  # noqa: E402
from lib.test_paths import is_test_path, strip_test_modules  # noqa: E402

REPO = Path(__file__).resolve().parent.parent

#: schedule label -> (the runner this repository sanctions, why it is the only one)
#:
#: ⛔ The path is the WHOLE contract, not documentation of it. A label with a
#: second sanctioned runner does not belong here: this check cannot express
#: "two, and only these two" without becoming a list nobody re-derives.
COMMIT_ONLY: dict[str, tuple[str, str]] = {
    "CheckpointDomainApply": (
        "crates/ambition_platformer2d_actor_monolith/src/session/checkpoint.rs",
        "the common commit executor — the eager host's exclusive runner and the "
        "confirmed host's commit tail. `CheckpointRestoreInputs` is installed only "
        "for this schedule's duration and removed on every success and failure "
        "path, so even an accidental invocation is a no-op rather than a restore "
        "to an empty baseline",
    ),
}


def runner_pattern(label: str) -> re.Pattern[str]:
    """`run_schedule(L)` / `try_run_schedule(L)`, at any qualification.

    ⚠ The label may arrive qualified (`lifecycle::CheckpointDomainApply`) or
    bare, and either is the same call. Matching the bare tail with a `\\b` in
    front would also match a DIFFERENT label ending in these characters, so the
    qualification is matched explicitly instead of skipped.
    """
    return re.compile(
        r"\b(?:try_)?run_schedule\s*\(\s*(?:[A-Za-z0-9_]+\s*::\s*)*" + re.escape(label) + r"\b"
    )


def production_sources() -> list[tuple[Path, str]]:
    out: list[tuple[Path, str]] = []
    for root in ("crates", "game"):
        for path in sorted((REPO / root).rglob("*.rs")):
            rel = path.relative_to(REPO)
            if any(part == "target" for part in rel.parts):
                continue
            text = path.read_text(errors="replace")
            if is_test_path(rel, text):
                continue
            out.append((rel, strip_test_modules(strip_comments(text))))
    return out


def main() -> int:
    sources = production_sources()
    failures: list[str] = []
    for label, (sanctioned, why) in COMMIT_ONLY.items():
        pattern = runner_pattern(label)
        sites = [
            f"{rel.as_posix()}:{text[: m.start()].count(chr(10)) + 1}"
            for rel, text in sources
            for m in pattern.finditer(text)
        ]
        print(f"`{label}` — sanctioned runner: {sanctioned}")
        print(f"  {why}")
        for site in sites:
            print(f"    {site}")

        # ⛔ THE FLOOR, AND IT IS NOT DECORATION HERE. "Exactly one runner" and
        # "the pattern matches nothing" produce the same shape of silence, and
        # the second one passes every inequality below.
        if not sites:
            failures.append(
                f"`{label}`: ZERO production call sites. Either the schedule was deleted — in "
                "which case delete its entry here — or `run_schedule` is no longer how it is "
                "invoked, and this check has been passing over an empty set."
            )
            continue

        strays = [site for site in sites if not site.startswith(sanctioned + ":")]
        if strays:
            failures.append(
                f"`{label}`: {len(strays)} runner(s) outside the sanctioned executor:\n    "
                + "\n    ".join(strays)
                + f"\n  ⇒ The authorization for everything in `{label}` IS that only "
                f"{sanctioned} runs it. A second runner does not reintroduce the "
                "`AdmittedCheckpointRestore` token A1c/3b deleted — it reintroduces that "
                "token's PROBLEM with none of its visibility, because no reducer has a value "
                "left to consult. If the new site is legitimate, it is a DESIGN change: say so "
                "at the schedule's own doc first, because that doc is what every reducer in it "
                "is relying on."
            )
        elif len(sites) > 1:
            failures.append(
                f"`{label}`: {len(sites)} call sites in the sanctioned file, and this check "
                "cannot tell a second executor from a retry:\n    " + "\n    ".join(sites)
                + "\n  ⇒ Read them. If one is a retry of the other, say so at the call site; if "
                "they are two roads into the same schedule, the contract needs rewriting."
            )

    if failures:
        print("\n⛔ FAILED\n")
        for row in failures:
            print(f"  {row}\n")
        return 1
    print("\nok: every commit-only schedule has exactly one production runner")
    return 0


if __name__ == "__main__":
    sys.exit(main())
