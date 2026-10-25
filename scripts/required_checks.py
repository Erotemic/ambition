#!/usr/bin/env python3
"""Which checks a change requires, and whether this tree has passed them.

    python3 scripts/required_checks.py                 # HEAD against origin/main
    python3 scripts/required_checks.py --base <rev> --rev <rev>
    python3 scripts/required_checks.py --run           # run what is not certified

The change is what `--rev` adds since it left `--base` (`git diff base...rev`).
Each rule below names paths and the checks a change to them requires. A check is
CERTIFIED when `run_tests.py` recorded it as passed (`target/lane_ledger.jsonl`)
on a tree that agrees with `--rev` on every path of the change. A doc (`.md`)
counts only for the repo tooling job, which reads docs: a doc edited after a
crate's run does not void that run.

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
import shlex
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts" / "lib"))

import lane_ledger  # noqa: E402
from cargo_package import package_name  # noqa: E402

#: The run_tests job that runs `scripts/tests` (the repo tooling lane).
REPO_TOOLING_JOB = "repo tooling (scripts/tests; repo-coupled)"
#: The authoring tool's own tests. A change to the tool reached the worlds
#: through a sprite regen, and no rule named this job.
LDTK_TOOLS_DIR = "tools/ambition_ldtk_tools"
LDTK_TOOLS_JOB = "ldtk authoring tool tests"
#: Jobs that only the detached tool lane plans (`run_tests.py --tool-tests`).
DETACHED_TOOL_JOBS = {LDTK_TOOLS_JOB}

#: The sprite tree (`ambition_sprite_sheet`): a development build reads it at
#: run time, so its own tests at default features never compile the embedded
#: tables a player build reads. A change to the crate requires the job that
#: embeds them (`run_tests.SPRITE_TREE_EMBEDDED_JOB`).
SPRITE_SHEET_DIR = "crates/ambition_sprite_sheet"
SPRITE_TREE_EMBEDDED_JOB = "ambition_sprite_sheet [static_content] — the embedded sprite tree"

#: The shipped game's package; `app_it` is one of its test targets.
APP_PACKAGE = "ambition_app"

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
    "crates/ambition_platformer2d_session/src/checkpoint*",
    "crates/ambition_platformer2d_session/src/death*",
    "crates/ambition_platformer2d_session/src/reset/*",
    "*/checkpoint_horizon.rs",
    "*/sandbox_reset.rs",
    "*/death_rules.rs",
    "*/death_traits.rs",
)

#: Authored content the shipped game loads: its worlds, catalogs and
#: characters (the matrix row "authored CONTENT"), and the map assets.
CONTENT_PATHS = (
    "game/ambition_content/assets/*",
    "game/ambition_map_assets",
)
#: The arms that row names for a content change.
CONTENT_ARMS = ("declared_art_resolves", "registered_character_art")

#: The matrix row "a rollback registration, a schedule pin, a message
#: channel": the `rollback_` arms and the repo tooling job. A channel is not
#: a path, so only the first two are held.
ROLLBACK_REGISTRATION_PATHS = (
    "*/rollback_registration.rs",
    "*/sim_phase_pins.rs",
)

#: The one job that runs the tests behind an opt-in feature: the whole
#: workspace with every headless-safe feature (`run_tests.py` plans it only
#: with `--run-everything-you-probably-dont-need-this`). Measured 2026-10-12
#: from an empty target: 3283 s, of which the tests are 1764 s (`app_it` 1248
#: s). Too long for every Rust change, so two rules require it, from where its
#: reds came on 2026-10-11 and 2026-10-12:
#: - a rollback registration or a schedule pin (three reds were rollback arms:
#:   an exit-oracle channel count, a schema parity under one feature, and a
#:   portal probe that only the `portal` composition registers);
#: - a file that holds a test behind an opt-in feature (`match_shots.rs` did not
#:   compile, and the portal exit-copy test ran on a copy);
#: - a change to the feature wiring of the root or a member manifest
#:   (`feature_wiring_changed`): the boundary table of
#:   `docs/concepts/testing-and-validation.md` gives "a change to features" to
#:   the exhaustive run.
#: A change in one crate can redden the gated test of a crate that depends on
#: it. 31 crates hide tests behind features (2026-10-12), so nearly each
#: engine crate has such a dependent, and this job for each of those changes
#: is the release boundary's cost at each push. That case is not required: the
#: receipt names it as NOT RUN, not required at this boundary
#: (`deferred_union_job`), so it is not silent.
UNION_FEATURE_JOB = "workspace [every headless-safe feature] — one graph, every gated test"
#: Jobs that `run_tests.py` plans only in its exhaustive mode.
EXHAUSTIVE_JOBS = {UNION_FEATURE_JOB}

#: A demo host app's tests behind its own opt-in features (`run_tests.py`
#: plans the job when `-p` names the app). A change to the demo's crates or
#: to its map assets requires it: on 2026-10-12 the Mary-O and Sanic HUD
#: declarations reddened their apps' `visible` pins, and a 1-2 map edit
#: reddened a `visible` test of its blocks, and no default lane saw either.
OWN_FEATURES_SUFFIX = " — its tests behind its own features"


def own_features_job(app: str) -> "Job":
    return Job(app + OWN_FEATURES_SUFFIX)


def demo_app_of(path: str) -> str | None:
    """The demo host app whose drawn tests a change to `path` can redden."""
    for app in DEMO_HOST_APPS:
        demo = app.removesuffix("_app")
        if path.startswith((f"game/{demo}/", f"game/{demo}_", f"game/ambition_map_assets/{demo}/")):
            return app
    return None

#: Procedural modules (fast-iteration I4). The module crate has no Bevy, and
#: its own tests check little more than its wire. What a module DOES is
#: measured where it is composed: the parity and reference harnesses of
#: `ambition_content` compare a module with its native reference, and the
#: `app_it` arms below run it in the game. Measured 2026-10-10: a port
#: version bump moved three boss modules (fsm, trex, mockingbird), and the
#: change was certified on the module crate's 18 tests alone.
MODULE_CRATE_DIR = "game/ambition_content_modules"
MODULE_HARNESS_PACKAGE = "ambition_content"
#: Each `app_it` arm that names the module crate
#: (`test_every_app_arm_that_names_the_modules_is_held` holds this), and the
#: arms that run module code through content and do not name the crate:
#: `fsm_fight` (the fsm module) and `gnu_ton_fight` (GNU-ton's choreography).
#: ⚠ An arm that runs a module and does not name the crate is held only when
#: it is in this list.
MODULE_ARMS = (
    "a_boss_special_runs_on_the_extension_host",
    "a_loaded_module_keeps_session_state",
    "fsm_fight",
    "gnu_ton_fight",
    "hunny_fight",
    "inugami_fight",
    "mockingbird_fight",
    "the_behemoth_shoots_from_its_eyes",
    "trex_fight",
    "wolfenstein_fight",
)

#: The workspace policies (`tests/ambition_workspace_policy`) read sources
#: across these trees and the root manifest: a file a policy names, a line a
#: policy forbids, a module's size. A change in any crate can redden them, so
#: each requires their tests. Measured 2026-10-10: the policies named
#: `crates/*` and `game/*` paths and `Cargo.toml` only, and three policies
#: named a file a crate change deleted, red on main for a day.
POLICY_PACKAGE = "ambition_workspace_policy"
POLICY_READS = ("crates/", "game/")

#: Prose. No crate compiles it, so a crate's check is not asked to agree with
#: it; the repo tooling job's guards read it (citations, anchors, `AGENTS.md`),
#: so a change to it requires that job. A picture or a `.txt` is not prose: an
#: art guard reads a sheet, and a test reads its `.txt` baseline.
PROSE_SUFFIX = ".md"

#: Tooling. No Rust source reads or runs a file under `scripts/` (it names one
#: only in a message; `test_no_rust_source_reads_or_runs_a_script` holds this),
#: so a crate's cargo test is not asked to agree with it. A job can run a
#: script, so a job still is.
TOOLING_PREFIX = "scripts/"


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


@dataclass(frozen=True)
class CargoTestNamed:
    """The tests of `package` whose names contain `name`: what
    `cargo test -p <package> -- <name>` runs."""

    package: str
    name: str

    def label(self) -> str:
        return f"cargo test -p {self.package} -- {self.name}"


Requirement = CargoTest | CargoTestNamed | Job


def workspace_members(repo: Path) -> dict[str, str]:
    """`{member directory: package name}`. The name is the manifest's
    `[package] name` (`cargo_package.package_name`), as `cargo test -p` reads it:
    `fixtures/extension_fixture_modules` is `ambition_extension_fixture_modules`,
    and its directory name asked cargo for a package that does not exist."""
    import re

    text = (repo / "Cargo.toml").read_text()
    body = re.search(r"members\s*=\s*\[(.*?)\]", text, re.S)
    members = {}
    for line in (body.group(1) if body else "").splitlines():
        line = line.split("#", 1)[0].strip().strip(",").strip().strip('"')
        if line:
            members[line] = package_name(repo / line)
    return members


def holds_gated_test(path: str, members: dict[str, str], repo: Path) -> bool:
    """Whether `path` holds a test that a default-feature run of its crate does
    not run: the test is behind an opt-in feature, in the file or through a
    gated `mod` declaration (`feature_gated_tests.py` decides both)."""
    import feature_gated_tests as census

    file = repo / path
    if not path.endswith(".rs") or not file.is_file():
        return False
    crate = next((repo / d for d in members if path.startswith(d + "/")), None)
    if crate is None:
        return False
    on_by_default = census.default_features(crate)
    tests, gated, _ = census.scan_file(file, on_by_default)
    return gated > 0 or (tests > 0 and file in census.gated_subtrees(crate, on_by_default))


#: The tables of a manifest that name dependencies.
DEPENDENCY_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")


def dependency_tables(manifest: dict) -> list[tuple[str, dict]]:
    """Each dependency table of a parsed manifest, by its dotted name: the
    plain tables, each `[target.<cfg>.*]` table and `[workspace.dependencies]`."""
    tables = [(table, manifest.get(table, {})) for table in DEPENDENCY_TABLES]
    for cfg, target in manifest.get("target", {}).items():
        tables += [(f"target.{cfg}.{table}", target.get(table, {})) for table in DEPENDENCY_TABLES]
    tables.append(("workspace.dependencies", manifest.get("workspace", {}).get("dependencies", {})))
    return tables


def feature_wiring(text: str) -> dict:
    """What in a manifest decides which features a build turns on: the
    `[features]` table, and for each dependency its `features`, `optional` and
    `default-features`. A version, a path or a dependency with none of these
    is not wiring."""
    manifest = tomllib.loads(text) if text else {}
    wiring: dict = {"[features]": manifest.get("features", {})}
    for table, deps in dependency_tables(manifest):
        for name, spec in deps.items():
            if not isinstance(spec, dict):
                continue
            wired = (
                sorted(spec.get("features", [])),
                spec.get("optional", False),
                spec.get("default-features", spec.get("default_features", True)),
            )
            if wired != ([], False, True):
                wiring[f"{table}.{name}"] = wired
    return wiring


def show(repo: Path, rev: str, path: str) -> str:
    """The text of `path` at `rev`, or "" where it is not there."""
    result = subprocess.run(["git", "show", f"{rev}:{path}"], cwd=repo, capture_output=True, text=True)
    return result.stdout if result.returncode == 0 else ""


def feature_wiring_changed(repo: Path, base: str, rev: str, path: str, members: dict[str, str]) -> bool:
    """Whether the change to `path` changes the feature wiring of the root or a
    member manifest. The change is `git diff base...rev`, so it is compared
    from the merge base. A manifest that does not parse counts as changed."""
    if not (path == "Cargo.toml" or path.removesuffix("/Cargo.toml") in members):
        return False
    fork = subprocess.run(
        ["git", "merge-base", base, rev], cwd=repo, capture_output=True, text=True, check=True
    ).stdout.strip()
    try:
        return feature_wiring(show(repo, fork, path)) != feature_wiring(show(repo, rev, path))
    except tomllib.TOMLDecodeError:
        return True


def workspace_dependents(repo: Path, members: dict[str, str]) -> dict[str, set[str]]:
    """`{package: each member that depends on it, directly or through other
    members}`. A dev-dependency counts: its change can redden the tests of
    the member that names it."""
    names = set(members.values())
    users: dict[str, set[str]] = {name: set() for name in names}
    for directory, package in members.items():
        try:
            manifest = tomllib.loads((repo / directory / "Cargo.toml").read_text())
        except (OSError, tomllib.TOMLDecodeError):
            continue
        for _, deps in dependency_tables(manifest):
            for key, spec in deps.items():
                dep = spec.get("package", key) if isinstance(spec, dict) else key
                if dep in names and dep != package:
                    users[dep].add(package)
    closure: dict[str, set[str]] = {}
    for name in names:
        seen: set[str] = set()
        stack = list(users[name])
        while stack:
            user = stack.pop()
            if user not in seen:
                seen.add(user)
                stack.extend(users[user])
        closure[name] = seen
    return closure


def deferred_union_job(paths: list[str], members: dict[str, str], repo: Path) -> tuple[list[str], list[str]]:
    """The case of the union job that no rule requires: a change in a member
    whose dependents hide tests behind features. Returns the paths of that
    change and the dependents (the members changed included) that hide a
    test, sorted. Both are empty when the change has no such dependent."""
    import feature_gated_tests as census

    touched: dict[str, list[str]] = {}
    for path in paths:
        if path.endswith(PROSE_SUFFIX):
            continue
        for directory, package in members.items():
            if path.startswith(directory + "/"):
                touched.setdefault(package, []).append(path)
    if not touched:
        return [], []
    dependents = workspace_dependents(repo, members)
    reached = set(touched)
    for package in touched:
        reached |= dependents.get(package, set())
    directory_of = {package: directory for directory, package in members.items()}
    hiding = sorted(p for p in reached if census.scan_crate(repo / directory_of[p])[1] > 0)
    if not hiding:
        return [], []
    return sorted(path for found in touched.values() for path in found), hiding


def requirements_for(path: str, members: dict[str, str], repo: Path = REPO) -> set[Requirement]:
    """The checks a change to `path` requires."""
    if path.endswith(PROSE_SUFFIX):
        return {Job(REPO_TOOLING_JOB)}
    required: set[Requirement] = set()
    # A member's own files: its own tests (the matrix row "Rust inside ONE crate").
    for directory, package in members.items():
        if path.startswith(directory + "/"):
            required.add(CargoTest(package))
    if any(fnmatch.fnmatch(path, pattern) for pattern in DEMO_HOST_LANE_PATHS):
        required.update(CargoTest(app) for app in DEMO_HOST_APPS)
    if any(fnmatch.fnmatch(path, pattern) for pattern in CONTENT_PATHS):
        required.update(CargoTestNamed(APP_PACKAGE, arm) for arm in CONTENT_ARMS)
    if path.startswith(MODULE_CRATE_DIR + "/"):
        required.add(CargoTest(MODULE_HARNESS_PACKAGE))
        required.update(CargoTestNamed(APP_PACKAGE, arm) for arm in MODULE_ARMS)
    if any(fnmatch.fnmatch(path, pattern) for pattern in ROLLBACK_REGISTRATION_PATHS):
        required.update({CargoTestNamed(APP_PACKAGE, "rollback_"), Job(REPO_TOOLING_JOB), Job(UNION_FEATURE_JOB)})
    if holds_gated_test(path, members, repo):
        required.add(Job(UNION_FEATURE_JOB))
    if (app := demo_app_of(path)) is not None:
        required.add(own_features_job(app))
    # The workspace policies read every crate, when the workspace has them.
    if (path.startswith(POLICY_READS) or path == "Cargo.toml") and POLICY_PACKAGE in members.values():
        required.add(CargoTest(POLICY_PACKAGE))
    # A guard's script and the baselines it reads (`scripts/baselines`,
    # `scripts/tests/*.txt`).
    if path.startswith("scripts/"):
        required.add(Job(REPO_TOOLING_JOB))
    if path.startswith(SPRITE_SHEET_DIR + "/"):
        required.add(Job(SPRITE_TREE_EMBEDDED_JOB))
    if path.startswith(LDTK_TOOLS_DIR + "/") and path.endswith(".py"):
        required.add(Job(LDTK_TOOLS_JOB))
    return required


#: `cargo test` flags that select fewer targets than all of them, or other features.
NARROWING_FLAGS = {
    "--lib", "--bin", "--bins", "--test", "--tests", "--example", "--examples",
    "--bench", "--benches", "--doc", "--features", "-F", "--all-features",
    "--no-default-features", "--exclude",
}


#: nextest flags that take a value: the word after one is not a filter.
NEXTEST_VALUE_FLAGS = {
    "-p", "--package", "--exclude", "--features", "-F", "--run-ignored", "-E",
    "--filterset", "--filter-expr", "-j", "--jobs", "--test-threads", "--profile",
    "-P", "--partition", "--retries", "--target", "--manifest-path", "--color",
    "--message-format", "--bin", "--test", "--example", "--bench",
}


def nextest_filters(after: list[str]) -> list[str]:
    """The bare-word name filters of a `cargo nextest run`."""
    filters, taken = [], False
    for arg in after:
        if taken:
            taken = False
        elif arg in NEXTEST_VALUE_FLAGS:
            taken = True
        elif not arg.startswith("-"):
            filters.append(arg)
    return filters


def nextest_narrows(after: list[str]) -> bool:
    """Whether a `cargo nextest run` selects fewer tests than its packages have.

    `run_tests.py` gives nextest a name filter as a bare word, with no `--`
    before it (`cargo_test`), so the libtest rule below cannot see it.
    """
    taken = False
    for index, arg in enumerate(after):
        if taken:
            taken = False
            continue
        flag = arg.split("=", 1)[0]
        value = arg.split("=", 1)[1] if "=" in arg else (after[index + 1] if index + 1 < len(after) else "")
        if flag in ("-E", "--filterset", "--filter-expr", "--partition"):
            return True
        if flag == "--run-ignored" and value == "only":
            return True
        if arg in NEXTEST_VALUE_FLAGS:
            taken = True
        elif not arg.startswith("-"):
            return True
    return False


def covers(row: dict, requirement: Requirement) -> bool:
    """Whether a ledger row is the check `requirement` names."""
    if isinstance(requirement, Job):
        return row.get("job") == requirement.name
    argv = row.get("argv") or []
    # nextest runs no doctests; `run_tests.py` plans them as a job beside it,
    # and this rule does not ask for that job.
    if argv[1:3] == ["nextest", "run"]:
        after = argv[3:]
        if isinstance(requirement, CargoTest) and nextest_narrows(after):
            return False
    elif argv[1:2] == ["test"]:
        after = argv[2:]
    else:
        return False
    if "--" in after:
        cargo_args, libtest = after[: after.index("--")], after[after.index("--") + 1:]
    else:
        cargo_args, libtest = after, []
    filters = [arg for arg in libtest if not arg.startswith("-")]
    if isinstance(requirement, CargoTestNamed):
        if argv[1:3] == ["nextest", "run"]:
            # The same filter as a bare word; a filter expression is not read.
            if any(arg in ("-E", "--filterset", "--filter-expr", "--partition") for arg in after):
                return False
            if any(a == "--run-ignored" and b == "only" for a, b in zip(after, after[1:])):
                return False
            filters = nextest_filters(after)
        # libtest runs a test when ANY filter is in its name, so a filter
        # that is part of `name` runs every test whose name holds `name`.
        if filters and not any(f in requirement.name for f in filters):
            return False
        if {"--exact", "--skip", "--ignored"} & set(libtest):
            return False
    # A positional libtest argument is a name filter: some of the tests ran.
    elif filters or "--ignored" in libtest:
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
    #: False for a check that this boundary does not require (Q59: NOT RUN,
    #: not required at this boundary). It counts against the change only
    #: when it failed on it.
    required: bool = True

    def blocks(self) -> bool:
        """Whether this verdict leaves the change not certified."""
        return not self.certified and (self.required or self.reason == "FAILED on this change")


def judge(repo: Path, base: str, rev: str) -> tuple[list[str], list[Verdict]]:
    paths = changed_paths(repo, base, rev)
    members = workspace_members(repo)
    triggers: dict[Requirement, list[str]] = {}
    for path in paths:
        for requirement in requirements_for(path, members, repo):
            triggers.setdefault(requirement, []).append(path)
        if feature_wiring_changed(repo, base, rev, path, members):
            triggers.setdefault(Job(UNION_FEATURE_JOB), []).append(path)
    deferred: dict[Requirement, tuple[list[str], str]] = {}
    if Job(UNION_FEATURE_JOB) not in triggers:
        by, hiding = deferred_union_job(paths, members, repo)
        if hiding:
            shown = ", ".join(hiding[:3]) + (f" and {len(hiding) - 3} more" if len(hiding) > 3 else "")
            deferred[Job(UNION_FEATURE_JOB)] = (
                by,
                f"not required at this boundary (a release or a change to features is); "
                f"{len(hiding)} crate(s) the change reaches hide tests behind features: {shown}",
            )
    ledger = lane_ledger.rows(repo)
    verdicts = []
    for requirement, by in sorted(triggers.items(), key=lambda item: item[0].label()):
        verdicts.append(verdict_for(repo, rev, paths, ledger, requirement, by))
    for requirement, (by, why) in deferred.items():
        found = verdict_for(repo, rev, paths, ledger, requirement, by)
        if found.reason == "NOT RUN" or found.reason.startswith("ran only on a tree before"):
            found.reason = f"NOT RUN, {why}"
        found.required = False
        verdicts.append(found)
    return paths, verdicts


def verdict_for(
    repo: Path, rev: str, paths: list[str], ledger: list[dict], requirement: Requirement, by: list[str]
) -> Verdict:
    """The verdict on one check, from the ledger rows that ran it."""
    relevant = [row for row in ledger if covers(row, requirement)]
    # The paths the run must have held as `rev` does: every path of the
    # change, but prose only for the job that reads prose, and scripts
    # not for a cargo test. A doc or a script edited while a crate's tests
    # ran did not change what they tested.
    held = paths if requirement == Job(REPO_TOOLING_JOB) else [p for p in paths if not p.endswith(PROSE_SUFFIX)]
    if isinstance(requirement, (CargoTest, CargoTestNamed)):
        held = [p for p in held if not p.startswith(TOOLING_PREFIX)]
    # Both ends of the run: a path of the change that moved while the job
    # ran was tested in neither state.
    current = [
        row for row in relevant
        if not held
        or all(agrees(repo, row.get(end) or row["tree"], rev, held) for end in ("tree", "tree_after"))
    ]
    # The newest row on a current tree is the verdict: a later failure
    # after an earlier pass on the same content is a failure.
    if current:
        newest = max(current, key=lambda row: row.get("finished", 0))
        if newest.get("ok") and not newest.get("unrunnable"):
            return Verdict(requirement, by, True, "passed on this change")
        if newest.get("unrunnable"):
            return Verdict(requirement, by, False, f"NOT RUN: {newest['unrunnable']}")
        return Verdict(requirement, by, False, "FAILED on this change")
    if relevant:
        return Verdict(requirement, by, False, "ran only on a tree before this change")
    return Verdict(requirement, by, False, "NOT RUN")


def remedy_commands(missing: list[Requirement]) -> list[list[str]]:
    """The `run_tests.sh` commands that run each of `missing`. Each records its
    result in the lane ledger, which is what a verdict reads."""
    packages = [r.package for r in missing if isinstance(r, CargoTest)]
    named = [r for r in missing if isinstance(r, CargoTestNamed)]
    jobs = [r.name for r in missing if isinstance(r, Job)]
    commands = []
    if packages:
        flags = [flag for package in packages for flag in ("-p", package)]
        commands.append(["./run_tests.sh", *flags, "--only-job", "(default features)"])
    # `-k` takes one filter, so each named check is a run of its own.
    for requirement in named:
        commands.append(
            ["./run_tests.sh", "-p", requirement.package, "-k", requirement.name, "--only-job", "(default features)"]
        )
    for job in jobs:
        lane = ["--tool-tests"] if job in DETACHED_TOOL_JOBS else []
        if job in EXHAUSTIVE_JOBS:
            lane = ["--run-everything-you-probably-dont-need-this"]
        if job.endswith(OWN_FEATURES_SUFFIX):
            lane = ["-p", job.removesuffix(OWN_FEATURES_SUFFIX)]
        commands.append(["./run_tests.sh", *lane, "--only-job", job])
    return commands


def remedy(missing: list[Requirement]) -> str:
    return "\n".join(f"    {shlex.join(command)}" for command in remedy_commands(missing))


def certify(repo: Path, base: str, rev: str, run=subprocess.run) -> tuple[list[str], list[Verdict]]:
    """Run the commands of each required check that is not certified, then
    judge again: the change selects the checks, and the ledger the runs write
    is the verdict. A command that fails does not stop the others; its
    failure is in the ledger."""
    paths, verdicts = judge(repo, base, rev)
    # Only what this boundary requires: a deferred check is the release's.
    missing = [verdict.requirement for verdict in verdicts if verdict.required and not verdict.certified]
    for command in remedy_commands(missing):
        print(f"required_checks: running {shlex.join(command)}", flush=True)
        run(command, cwd=repo)
    if missing:
        paths, verdicts = judge(repo, base, rev)
    return paths, verdicts


def report(paths: list[str], verdicts: list[Verdict]) -> int:
    required = sum(v.required for v in verdicts)
    deferred = len(verdicts) - required
    later = f", {deferred} not required at this boundary" if deferred else ""
    print(f"required_checks: {len(paths)} changed path(s), {required} required check(s){later}")
    for verdict in verdicts:
        mark = "ok  " if verdict.certified else "MISS" if verdict.blocks() else "defer"
        shown = ", ".join(verdict.triggered_by[:3])
        more = f" and {len(verdict.triggered_by) - 3} more" if len(verdict.triggered_by) > 3 else ""
        print(f"  {mark} {verdict.requirement.label()}  ({verdict.reason}; for {shown}{more})")
    missing = [v.requirement for v in verdicts if v.blocks()]
    if missing:
        print("\nNOT CERTIFIED: the change requires checks this tree has not passed. Run:")
        print(remedy(missing))
        return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--base", default="origin/main")
    parser.add_argument("--rev", default="HEAD")
    parser.add_argument(
        "--run",
        action="store_true",
        help="run the checks that are not certified, then report again",
    )
    args = parser.parse_args()
    try:
        if args.run:
            paths, verdicts = certify(REPO, args.base, args.rev)
        else:
            paths, verdicts = judge(REPO, args.base, args.rev)
        return report(paths, verdicts)
    except subprocess.CalledProcessError as error:
        print(f"required_checks: git refused: {error.stderr.decode() if isinstance(error.stderr, bytes) else error.stderr}")
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
