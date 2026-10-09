#!/usr/bin/env python3
"""Which checks a change requires, and whether this tree has passed them.

    python3 scripts/required_checks.py                 # HEAD against origin/main
    python3 scripts/required_checks.py --base <rev> --rev <rev>

The change is what `--rev` adds since it left `--base` (`git diff base...rev`).
Each rule below names paths and the checks a change to them requires. A check is
CERTIFIED when `run_tests.py` recorded it as passed (`target/lane_ledger.jsonl`)
on a tree that agrees with `--rev` on every path of the change.

It reports. A push does not wait for it (Q166, ruled 2026-10-09).

Exit codes: 0 every required check is certified; 1 a required check is not
certified (each one is named, with the command that runs it); 2 the question
could not be asked (no git, an unknown revision).

What a certificate does NOT say:
- that a peer's change merged after the run was tested with yours. Only the
  paths of THIS change must agree with the tested tree;
- anything about a check no rule names. The rules are the prose rules of
  `docs/recipes/cheapest-sufficient-check.md` and
  `docs/recipes/running-the-heavy-app-it-lane.md` that a path can express.
  "An instrument that a demo test reads" is not a path, and no rule holds it.
"""

from __future__ import annotations

import argparse
import fnmatch
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts" / "lib"))

import lane_ledger  # noqa: E402

#: The run_tests job that runs `scripts/tests` (the repo tooling lane).
REPO_TOOLING_JOB = "repo tooling (scripts/tests; repo-coupled)"
#: The authoring tool's own tests. A change to the tool reached the worlds
#: through a sprite regen, and no rule named this job.
LDTK_TOOLS_DIR = "tools/ambition_ldtk_tools"
LDTK_TOOLS_JOB = "ldtk authoring tool tests"
#: Jobs that only the detached tool lane plans (`run_tests.py --tool-tests`).
DETACHED_TOOL_JOBS = {LDTK_TOOLS_JOB}

#: The demo host apps. `app_it` and the pytest lane run none of their tests.
DEMO_HOST_APPS = (
    "ambition_demo_mary_o_app",
    "ambition_demo_sanic_app",
    "ambition_demo_smash_app",
    "ambition_demo_twintrack_app",
)

#: The paths of the demo host lane rule (running-the-heavy-app-it-lane.md,
#: "When this lane is required"): session death, a checkpoint restore, room
#: replay, a sandbox reset, the rollback host and a rollback registration.
DEMO_HOST_LANE_PATHS = (
    "crates/ambition_platformer2d_rollback_ggrs/*",
    "crates/ambition_platformer2d_runtime/src/rollback/*",
    "crates/ambition_platformer2d/src/rollback.rs",
    "*/rollback_registration.rs",
    "*/session/checkpoint*",
    "*/session/death*",
    "*/session/reset/*",
    "*/checkpoint_horizon.rs",
    "*/sandbox_reset.rs",
    "*/death_rules.rs",
    "*/death_traits.rs",
)

#: A path whose change requires no check: prose and pictures. A test that
#: reads one of these is named by a rule of its own.
NO_CHECK_SUFFIXES = (".md", ".png", ".svg", ".txt")


@dataclass(frozen=True)
class CargoTest:
    """`cargo test` over `package` with its default features and every target."""

    package: str

    def label(self) -> str:
        return f"cargo test -p {self.package}"


@dataclass(frozen=True)
class Job:
    """One `run_tests.py` job, by its exact name."""

    name: str

    def label(self) -> str:
        return f"run_tests job `{self.name}`"


Requirement = CargoTest | Job


def workspace_members(repo: Path) -> dict[str, str]:
    """`{member directory: package name}`. The runner names a package by its
    directory (`run_tests.py` `-p crate.name`), so this does too."""
    import re

    text = (repo / "Cargo.toml").read_text()
    body = re.search(r"members\s*=\s*\[(.*?)\]", text, re.S)
    members = {}
    for line in (body.group(1) if body else "").splitlines():
        line = line.split("#", 1)[0].strip().strip(",").strip().strip('"')
        if line:
            members[line] = Path(line).name
    return members


def requirements_for(path: str, members: dict[str, str]) -> set[Requirement]:
    """The checks a change to `path` requires."""
    if path.endswith(NO_CHECK_SUFFIXES):
        return set()
    required: set[Requirement] = set()
    # A member's own files: its own tests (the matrix row "Rust inside ONE crate").
    for directory, package in members.items():
        if path.startswith(directory + "/"):
            required.add(CargoTest(package))
    if any(fnmatch.fnmatch(path, pattern) for pattern in DEMO_HOST_LANE_PATHS):
        required.update(CargoTest(app) for app in DEMO_HOST_APPS)
    if path.startswith("scripts/") and path.endswith(".py"):
        required.add(Job(REPO_TOOLING_JOB))
    if path.startswith(LDTK_TOOLS_DIR + "/") and path.endswith(".py"):
        required.add(Job(LDTK_TOOLS_JOB))
    return required


#: `cargo test` flags that select fewer targets than all of them, or other features.
NARROWING_FLAGS = {
    "--lib", "--bin", "--bins", "--test", "--tests", "--example", "--examples",
    "--bench", "--benches", "--doc", "--features", "-F", "--all-features",
    "--no-default-features", "--exclude",
}


def covers(row: dict, requirement: Requirement) -> bool:
    """Whether a ledger row is the check `requirement` names."""
    if isinstance(requirement, Job):
        return row.get("job") == requirement.name
    argv = row.get("argv") or []
    # nextest runs no doctests; `run_tests.py` plans them as a job beside it,
    # and this rule does not ask for that job.
    if argv[1:3] == ["nextest", "run"]:
        after = argv[3:]
    elif argv[1:2] == ["test"]:
        after = argv[2:]
    else:
        return False
    if "--" in after:
        cargo_args, libtest = after[: after.index("--")], after[after.index("--") + 1:]
    else:
        cargo_args, libtest = after, []
    # A positional libtest argument is a name filter: some of the tests ran.
    if any(not arg.startswith("-") for arg in libtest) or "--ignored" in libtest:
        return False
    if any(arg.split("=", 1)[0] in NARROWING_FLAGS for arg in cargo_args):
        return False
    if "--workspace" in cargo_args:
        return True
    packages = {
        cargo_args[i + 1] for i, arg in enumerate(cargo_args[:-1]) if arg in ("-p", "--package")
    }
    return requirement.package in packages


def changed_paths(repo: Path, base: str, rev: str) -> list[str]:
    out = subprocess.run(
        ["git", "diff", "--name-only", "--no-renames", f"{base}...{rev}"],
        cwd=repo, capture_output=True, text=True, check=True,
    ).stdout
    return [line for line in out.splitlines() if line]


def agrees(repo: Path, tree: str, rev: str, paths: list[str]) -> bool:
    """Whether `tree` holds every one of `paths` as `rev` does."""
    result = subprocess.run(
        ["git", "diff", "--quiet", "--no-renames", tree, rev, "--", *paths],
        cwd=repo, capture_output=True,
    )
    return result.returncode == 0


@dataclass
class Verdict:
    requirement: Requirement
    triggered_by: list[str]
    certified: bool
    #: Why not, when it is not: no row ran it, it failed, or its tree is old.
    reason: str


def judge(repo: Path, base: str, rev: str) -> tuple[list[str], list[Verdict]]:
    paths = changed_paths(repo, base, rev)
    members = workspace_members(repo)
    triggers: dict[Requirement, list[str]] = {}
    for path in paths:
        for requirement in requirements_for(path, members):
            triggers.setdefault(requirement, []).append(path)
    ledger = lane_ledger.rows(repo)
    verdicts = []
    for requirement, by in sorted(triggers.items(), key=lambda item: item[0].label()):
        relevant = [row for row in ledger if covers(row, requirement)]
        # Both ends of the run: a path of the change that moved while the job
        # ran was tested in neither state.
        current = [
            row for row in relevant
            if all(agrees(repo, row.get(end) or row["tree"], rev, paths) for end in ("tree", "tree_after"))
        ]
        # The newest row on a current tree is the verdict: a later failure
        # after an earlier pass on the same content is a failure.
        if current:
            newest = max(current, key=lambda row: row.get("finished", 0))
            if newest.get("ok") and not newest.get("unrunnable"):
                verdicts.append(Verdict(requirement, by, True, "passed on this change"))
            elif newest.get("unrunnable"):
                verdicts.append(Verdict(requirement, by, False, f"NOT RUN: {newest['unrunnable']}"))
            else:
                verdicts.append(Verdict(requirement, by, False, "FAILED on this change"))
        elif relevant:
            verdicts.append(Verdict(requirement, by, False, "ran only on a tree before this change"))
        else:
            verdicts.append(Verdict(requirement, by, False, "NOT RUN"))
    return paths, verdicts


def remedy(missing: list[Requirement]) -> str:
    packages = [r.package for r in missing if isinstance(r, CargoTest)]
    jobs = [r.name for r in missing if isinstance(r, Job)]
    lines = []
    if packages:
        flags = " ".join(f"-p {package}" for package in packages)
        lines.append(f"./run_tests.sh {flags} --only-job '(default features)'")
    for job in jobs:
        lane = "--tool-tests " if job in DETACHED_TOOL_JOBS else ""
        lines.append(f"./run_tests.sh {lane}--only-job '{job}'")
    return "\n".join(f"    {line}" for line in lines)


def report(paths: list[str], verdicts: list[Verdict]) -> int:
    print(f"required_checks: {len(paths)} changed path(s), {len(verdicts)} required check(s)")
    for verdict in verdicts:
        mark = "ok  " if verdict.certified else "MISS"
        shown = ", ".join(verdict.triggered_by[:3])
        more = f" and {len(verdict.triggered_by) - 3} more" if len(verdict.triggered_by) > 3 else ""
        print(f"  {mark} {verdict.requirement.label()}  ({verdict.reason}; for {shown}{more})")
    missing = [v.requirement for v in verdicts if not v.certified]
    if missing:
        print("\nNOT CERTIFIED: the change requires checks this tree has not passed. Run:")
        print(remedy(missing))
        return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--base", default="origin/main")
    parser.add_argument("--rev", default="HEAD")
    args = parser.parse_args()
    try:
        paths, verdicts = judge(REPO, args.base, args.rev)
        return report(paths, verdicts)
    except subprocess.CalledProcessError as error:
        print(f"required_checks: git refused: {error.stderr.decode() if isinstance(error.stderr, bytes) else error.stderr}")
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
