"""A push is refused until the checks its change requires passed on it (TEST-LANES).

Each test builds a small git repository with a workspace of its own, so the
witness does not depend on what this checkout has run.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "lib"))

REPO = Path(__file__).resolve().parents[2]

import lane_ledger  # noqa: E402
import required_checks  # noqa: E402
from required_checks import DEMO_HOST_APPS, REPO_TOOLING_JOB, CargoTest, Job, covers, judge  # noqa: E402


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@t", *args],
        cwd=repo, capture_output=True, text=True, check=True,
    ).stdout.strip()


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    """A workspace of two members, `crates/alpha` and `crates/beta`, at a base commit."""
    (tmp_path / "Cargo.toml").write_text('[workspace]\nmembers = [\n    "crates/alpha",\n    "crates/beta",\n]\n')
    for name in ("alpha", "beta"):
        (tmp_path / "crates" / name / "src").mkdir(parents=True)
        (tmp_path / "crates" / name / "src" / "lib.rs").write_text("// one\n")
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "a.md").write_text("one\n")
    git(tmp_path, "init", "-q", "-b", "main")
    git(tmp_path, "add", "-A")
    git(tmp_path, "commit", "-q", "-m", "base")
    git(tmp_path, "tag", "base")
    return tmp_path


def commit(repo: Path, path: str, text: str) -> None:
    file = repo / path
    file.parent.mkdir(parents=True, exist_ok=True)
    file.write_text(text)
    git(repo, "add", path)
    git(repo, "commit", "-q", "-m", f"edit {path}")


def ran(repo: Path, argv: list[str], ok: bool = True, job: str = "j") -> None:
    """Record a job as `run_tests.py` does, on the working tree as it is now."""
    tree = lane_ledger.tested_tree(repo)
    assert tree is not None
    lane_ledger.record(repo, tree, job, argv, ok, None)


def verdicts(repo: Path) -> dict[str, tuple[bool, str]]:
    _, found = judge(repo, "base", "HEAD")
    return {v.requirement.label(): (v.certified, v.reason) for v in found}


def test_a_change_to_a_member_requires_its_tests_and_a_pass_on_it_certifies(repo: Path) -> None:
    """⭐ THE WITNESS AND ITS CONTROL. A change to `alpha` requires `cargo test
    -p alpha` and nothing of `beta`; with no row it is refused; a pass recorded
    on the working tree before the commit certifies the commit."""
    (repo / "crates/alpha/src/lib.rs").write_text("// two\n")
    assert verdicts(repo) == {}, "an uncommitted change is not the change of HEAD"
    ran(repo, ["cargo", "test", "-p", "alpha"])
    git(repo, "commit", "-qam", "alpha")
    assert verdicts(repo) == {"cargo test -p alpha": (True, "passed on this change")}


def test_a_member_is_named_by_its_manifest_not_its_directory(repo: Path) -> None:
    """`cargo test -p` takes the manifest's package name. The receipt asked for
    `cargo test -p extension_fixture_modules` for a change under
    `fixtures/extension_fixture_modules`, whose package is
    `ambition_extension_fixture_modules`, and the run failed at cargo's
    "did not match any packages". The control: a member with no manifest name
    keeps its directory name (`alpha`)."""
    (repo / "crates/alpha/Cargo.toml").write_text('[package]\nname = "ambition_alpha"\n')
    git(repo, "add", "-A")
    git(repo, "commit", "-qm", "alpha manifest")
    git(repo, "tag", "-f", "base")
    members = required_checks.workspace_members(repo)
    assert members == {"crates/alpha": "ambition_alpha", "crates/beta": "beta"}
    commit(repo, "crates/alpha/src/lib.rs", "// two\n")
    assert verdicts(repo) == {"cargo test -p ambition_alpha": (False, "NOT RUN")}


def test_the_live_fixture_member_is_its_package() -> None:
    """The one live member whose directory and package differ."""
    members = required_checks.workspace_members(REPO)
    assert members["fixtures/extension_fixture_modules"] == "ambition_extension_fixture_modules"


def test_with_no_row_the_requirement_is_not_run(repo: Path) -> None:
    commit(repo, "crates/alpha/src/lib.rs", "// two\n")
    assert verdicts(repo) == {"cargo test -p alpha": (False, "NOT RUN")}
    assert required_checks.report(*judge(repo, "base", "HEAD")) == 1


def test_a_pass_before_a_later_edit_is_old(repo: Path) -> None:
    commit(repo, "crates/alpha/src/lib.rs", "// two\n")
    ran(repo, ["cargo", "test", "-p", "alpha"])
    commit(repo, "crates/alpha/src/lib.rs", "// three\n")
    assert verdicts(repo) == {"cargo test -p alpha": (False, "ran only on a tree before this change")}


def test_a_failure_after_a_pass_on_the_same_tree_is_a_failure(repo: Path) -> None:
    commit(repo, "crates/alpha/src/lib.rs", "// two\n")
    ran(repo, ["cargo", "test", "-p", "alpha"])
    ran(repo, ["cargo", "test", "-p", "alpha"], ok=False)
    assert verdicts(repo) == {"cargo test -p alpha": (False, "FAILED on this change")}


def test_a_doc_change_requires_the_job_whose_guards_read_docs(repo: Path) -> None:
    commit(repo, "docs/a.md", "two\n")
    assert verdicts(repo) == {f"run_tests job `{REPO_TOOLING_JOB}`": (False, "NOT RUN")}
    ran(repo, ["python", "-m", "pytest", "scripts/tests"], job=REPO_TOOLING_JOB)
    assert required_checks.report(*judge(repo, "base", "HEAD")) == 0


def test_a_doc_edited_after_a_crate_ran_does_not_void_the_crate(repo: Path) -> None:
    """A crate's tests compile no prose: the doc commit after the run asks the
    tooling job again and leaves the crate's pass standing. The control: a
    source edit after the run voids it."""
    commit(repo, "crates/alpha/src/lib.rs", "// two\n")
    ran(repo, ["cargo", "test", "-p", "alpha"])
    ran(repo, ["python", "-m", "pytest", "scripts/tests"], job=REPO_TOOLING_JOB)
    commit(repo, "docs/a.md", "two\n")
    assert verdicts(repo) == {
        "cargo test -p alpha": (True, "passed on this change"),
        f"run_tests job `{REPO_TOOLING_JOB}`": (False, "ran only on a tree before this change"),
    }
    commit(repo, "crates/alpha/src/lib.rs", "// three\n")
    assert verdicts(repo)["cargo test -p alpha"] == (False, "ran only on a tree before this change")

def test_a_script_edited_after_a_crate_ran_does_not_void_the_crate(repo: Path) -> None:
    """⭐ No cargo test reads or runs a file under `scripts/`, so a script
    commit after the run asks the tooling job again and leaves the crate's
    pass standing. The control: a source edit after the run voids it."""
    commit(repo, "crates/alpha/src/lib.rs", "// two\n")
    ran(repo, ["cargo", "test", "-p", "alpha"])
    commit(repo, "scripts/guard.py", "print('two')\n")
    assert verdicts(repo) == {
        "cargo test -p alpha": (True, "passed on this change"),
        f"run_tests job `{REPO_TOOLING_JOB}`": (False, "NOT RUN"),
    }
    commit(repo, "crates/alpha/src/lib.rs", "// three\n")
    assert verdicts(repo)["cargo test -p alpha"] == (False, "ran only on a tree before this change")


def test_a_script_edited_after_the_tooling_job_ran_voids_it(repo: Path) -> None:
    """The tooling job runs the scripts, so it still holds every script."""
    commit(repo, "scripts/guard.py", "print('one')\n")
    ran(repo, ["python", "-m", "pytest", "scripts/tests"], job=REPO_TOOLING_JOB)
    commit(repo, "scripts/guard.py", "print('two')\n")
    assert verdicts(repo) == {f"run_tests job `{REPO_TOOLING_JOB}`": (False, "ran only on a tree before this change")}


#: A Rust use of a path that reads or runs it, not a message that names it.
_READS = re.compile(r"include_str!|include_bytes!|\.join\(|Path::new|PathBuf::from|read_to_string|File::open|Command::new|concat!")


def test_no_rust_source_reads_or_runs_a_script() -> None:
    """The premise of the exemption above: Rust names `scripts/` only in
    messages. A test that reads a script or its baseline makes the crate's
    check depend on it again, and then `TOOLING_PREFIX` must not exempt it."""
    listed = subprocess.run(
        ["git", "grep", "-n", "-E", r'scripts/|"scripts"', "--", "*.rs"],
        cwd=REPO, capture_output=True, text=True,
    ).stdout.splitlines()
    readers = [line for line in listed if _READS.search(line)]
    assert listed, "the scan found no Rust mention of scripts/ at all: the search is wrong"
    assert readers == [], "Rust reads or runs a file under scripts/:\n" + "\n".join(readers)


def test_a_change_in_any_crate_requires_the_workspace_policies(repo: Path) -> None:
    """⭐ The policies read every crate (a path a policy names, a forbidden
    line), so a change to `alpha` asks for their tests as well as its own. A
    doc does not. The control: a workspace without the policy package asks for
    none (the other tests here)."""
    policy = "tests/ambition_workspace_policy"
    (repo / "Cargo.toml").write_text(
        '[workspace]\nmembers = [\n    "crates/alpha",\n    "crates/beta",\n    "%s",\n]\n' % policy
    )
    (repo / policy / "src").mkdir(parents=True)
    (repo / policy / "src" / "lib.rs").write_text("// policies\n")
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", "policies")
    git(repo, "tag", "-f", "base")
    commit(repo, "crates/alpha/src/lib.rs", "// two\n")
    assert verdicts(repo) == {
        "cargo test -p alpha": (False, "NOT RUN"),
        f"cargo test -p {required_checks.POLICY_PACKAGE}": (False, "NOT RUN"),
    }
    git(repo, "tag", "-f", "base")
    commit(repo, "docs/a.md", "two\n")
    assert f"cargo test -p {required_checks.POLICY_PACKAGE}" not in verdicts(repo)


def test_a_test_baseline_and_a_guard_baseline_are_not_prose() -> None:
    from required_checks import requirements_for

    members = {"game/ambition_app": "ambition_app"}
    assert requirements_for("game/ambition_app/tests/rollback_schema_baseline.txt", members) == {
        CargoTest("ambition_app")
    }
    assert requirements_for("scripts/baselines/rollback-schema-baseline.json", {}) == {Job(REPO_TOOLING_JOB)}
    assert requirements_for("scripts/tests/rollback_codec_shape.txt", {}) == {Job(REPO_TOOLING_JOB)}


def test_a_new_untracked_source_file_is_part_of_the_tested_tree(repo: Path) -> None:
    """A run sees a new file before it is committed; the row must too."""
    (repo / "crates/alpha/src/new.rs").write_text("// new\n")
    ran(repo, ["cargo", "test", "-p", "alpha"])
    git(repo, "add", "crates/alpha/src/new.rs")
    git(repo, "commit", "-qm", "new file")
    assert verdicts(repo) == {"cargo test -p alpha": (True, "passed on this change")}


@pytest.mark.parametrize(
    ("argv", "expected"),
    [
        (["cargo", "test", "-p", "alpha"], True),
        (["cargo", "test", "--workspace"], True),
        (["cargo", "nextest", "run", "-p", "alpha"], True),
        (["cargo", "nextest", "run", "-p", "alpha", "--run-ignored", "all"], True),
        (["cargo", "nextest", "run", "-p", "alpha", "-P", "ci"], True),
        # `run_tests.py -k` gives nextest its filter as a bare word.
        (["cargo", "nextest", "run", "-p", "alpha", "some_filter"], False),
        (["cargo", "nextest", "run", "-p", "alpha", "--run-ignored", "only"], False),
        (["cargo", "nextest", "run", "-p", "alpha", "-E", "test(x)"], False),
        (["cargo", "test", "-p", "alpha", "--", "--test-threads=4"], True),
        (["cargo", "test", "-p", "beta"], False),
        (["cargo", "test", "-p", "alpha", "--", "some_filter"], False),
        (["cargo", "test", "-p", "alpha", "--features", "x"], False),
        (["cargo", "test", "-p", "alpha", "--lib"], False),
        (["cargo", "test", "--workspace", "--exclude", "alpha"], False),
        (["cargo", "check", "-p", "alpha"], False),
    ],
)
def test_only_a_default_feature_run_of_every_target_covers_a_package(argv: list[str], expected: bool) -> None:
    assert covers({"argv": argv}, CargoTest("alpha")) is expected


def test_the_demo_host_rule_requires_the_four_demo_apps() -> None:
    for path in (
        "crates/ambition_platformer2d_rollback_ggrs/src/lib.rs",
        "crates/ambition_combat/src/rollback_registration.rs",
        "crates/ambition_platformer2d_session/src/checkpoint.rs",
        "crates/ambition_platformer2d_runtime/src/sandbox_reset.rs",
    ):
        required = required_checks.requirements_for(path, {})
        assert {CargoTest(app) for app in DEMO_HOST_APPS} <= required, path
    assert required_checks.requirements_for("crates/ambition_combat/src/moveset.rs", {}) == set()


def test_the_rules_name_what_the_live_tree_has() -> None:
    """A rule that names a package or a job that does not exist can never be
    certified, and reads as a lane nobody runs."""
    packages = set(required_checks.workspace_members(REPO).values())
    missing = [app for app in DEMO_HOST_APPS if app not in packages]
    assert not missing, f"the demo host rule names packages the workspace lacks: {missing}"
    plan = subprocess.run(
        [sys.executable, "scripts/run_tests.py", "--list"], cwd=REPO, capture_output=True, text=True
    ).stdout
    assert REPO_TOOLING_JOB in plan, "the repo tooling job is not in the default plan by that name"
    matched = [
        pattern for pattern in required_checks.DEMO_HOST_LANE_PATHS
        if not any(
            required_checks.fnmatch.fnmatch(path, pattern)
            for path in git(REPO, "ls-files", "crates", "game").splitlines()
        )
    ]
    assert not matched, f"demo host rule patterns that match no tracked file: {matched}"


def test_a_job_requirement_is_met_by_that_job_only() -> None:
    assert covers({"job": REPO_TOOLING_JOB}, Job(REPO_TOOLING_JOB))
    assert not covers({"job": "doc links (active KB)"}, Job(REPO_TOOLING_JOB))


def test_a_run_with_a_status_of_its_own_records_no_evidence(monkeypatch, tmp_path: Path) -> None:
    """Tests drive `run_tests.run` with fake jobs under real job names. Only
    the run that writes the default status may write the ledger, or a fake
    `python -c pass` certifies the repo tooling job."""
    import run_tests

    monkeypatch.setattr(run_tests, "free_gb_on_target", lambda: 500.0)
    monkeypatch.setattr(run_tests, "append_cost_ledger", lambda *a, **k: None)
    before = len(lane_ledger.rows(REPO))
    fake = run_tests.Job(REPO_TOOLING_JOB, [sys.executable, "-c", "pass"])
    assert run_tests.run([fake], False, status_json=str(tmp_path / "status.json")) == 0
    assert len(lane_ledger.rows(REPO)) == before, "a test's fake job was recorded as evidence"


def test_a_failed_job_keeps_its_evidence_in_the_lane_ledger_row(monkeypatch, repo: Path) -> None:
    """The lane ledger is the history of failures. The status file is written
    over by the next run, and the cost ledger is not written for a narrowed
    `--only-job` run, which is how a change's required checks run. So a
    failure that occurs once is counted only if its ledger row says what
    failed. Driven through `run()`, the road that writes the row."""
    import run_tests

    monkeypatch.setattr(run_tests, "REPO", repo)
    monkeypatch.delenv(run_tests.NESTED_ENV, raising=False)
    monkeypatch.setattr(run_tests, "free_gb_on_target", lambda: 500.0)
    monkeypatch.setattr(run_tests, "append_cost_ledger", lambda *a, **k: None)
    panic = "thread 'arm' panicked at tests/x.rs:1:1:"
    passing = run_tests.Job("passing job", [sys.executable, "-c", "pass"])
    failing = run_tests.Job(
        "failing job", [sys.executable, "-c", f"import sys; print({panic!r}); sys.exit(101)"]
    )
    assert run_tests.run([passing, failing], False) != 0
    rows = {row["job"]: row for row in lane_ledger.rows(repo)}
    assert panic in rows["failing job"].get("failure_evidence", []), (
        f"the failed job's ledger row does not say what failed: {rows['failing job']}"
    )
    assert "failure_evidence" not in rows["passing job"], "a passing row keeps the shape it had"


def test_a_failed_cargo_job_on_a_stale_sprite_set_says_so(monkeypatch, repo: Path, tmp_path: Path) -> None:
    """A stale sprite set fails the drawing composition's arms for the assets,
    and the row read like a code failure (2026-10-13: twice in one day). The
    verdict stays FAILED, since another failure in the job can be real; the
    row says what was stale. A passing row keeps its shape."""
    import run_tests

    monkeypatch.setattr(run_tests, "REPO", repo)
    monkeypatch.delenv(run_tests.NESTED_ENV, raising=False)
    monkeypatch.setattr(run_tests, "free_gb_on_target", lambda: 500.0)
    monkeypatch.setattr(run_tests, "append_cost_ledger", lambda *a, **k: None)
    monkeypatch.setattr(run_tests, "published_sprites_stale", lambda _repo: "the published sprites are stale (stale: x)")
    cargo = tmp_path / "bin" / "cargo"
    cargo.parent.mkdir()
    cargo.write_text("#!/bin/sh\necho \"thread 'arm' panicked at tests/x.rs:1:1:\"\nexit 101\n")
    cargo.chmod(0o755)
    failing = run_tests.Job("failing cargo job", [str(cargo), "test"])
    passing = run_tests.Job("passing job", [sys.executable, "-c", "pass"])
    assert run_tests.run([passing, failing], False) != 0
    rows = {row["job"]: row for row in lane_ledger.rows(repo)}
    assert "stale" in rows["failing cargo job"].get("environment", ""), rows["failing cargo job"]
    assert "environment" not in rows["passing job"], "a passing row keeps the shape it had"


def test_the_sprite_check_is_the_publish_scripts_own(tmp_path: Path) -> None:
    """`published_sprites_stale` asks `sprites.sh --check` and repeats its
    reason; a fresh set is `None`."""
    import run_tests

    script = tmp_path / "scripts" / "regen" / "sprites.sh"
    script.parent.mkdir(parents=True)
    script.write_text("echo 'stale: the renderer changed'\nexit 1\n")
    reason = run_tests.published_sprites_stale(tmp_path)
    assert reason is not None and "stale: the renderer changed" in reason and "republish" in reason, reason
    script.write_text("echo ok\nexit 0\n")
    assert run_tests.published_sprites_stale(tmp_path) is None


def test_an_unrelated_edit_during_the_run_does_not_void_it(repo: Path) -> None:
    """A docs file changed by the host while the job ran: the change's paths
    were the same at both ends, so the run certifies it. An edit to the
    change's own path during the run does not."""
    commit(repo, "crates/alpha/src/lib.rs", "// two\n")
    before = lane_ledger.tested_tree(repo)
    (repo / "docs/a.md").write_text("edited by the host\n")
    after = lane_ledger.tested_tree(repo)
    assert before != after
    lane_ledger.record(repo, before, "j", ["cargo", "test", "-p", "alpha"], True, None, tree_after=after)
    assert verdicts(repo) == {"cargo test -p alpha": (True, "passed on this change")}

    (repo / "crates/alpha/src/lib.rs").write_text("// moved mid-run\n")
    moved = lane_ledger.tested_tree(repo)
    lane_ledger.record(repo, after, "j", ["cargo", "test", "-p", "alpha"], True, None, tree_after=moved)
    (repo / "crates/alpha/src/lib.rs").write_text("// two\n")
    rows = lane_ledger.rows(repo)
    assert len(rows) == 2 and rows[-1]["tree_after"] == moved
    # The newest row on a current tree decides; the moved row is not current,
    # so the earlier row still certifies, and the moved one alone would not.
    lane_ledger.ledger_path(repo).write_text(lane_ledger.ledger_path(repo).read_text().splitlines()[-1] + "\n")
    assert verdicts(repo) == {"cargo test -p alpha": (False, "ran only on a tree before this change")}


def test_a_moved_submodule_pointer_is_tested_at_its_checkout(repo: Path, tmp_path_factory) -> None:
    """Measured 2026-10-09: a change that moved `dev/ambition_dev_measurements`
    was refused after a passing run, because the tested tree took the pointer
    from the index (the old commit) while the run used the new checkout."""
    upstream = tmp_path_factory.mktemp("upstream")
    git(upstream, "init", "-q", "-b", "main")
    (upstream / "row.txt").write_text("one\n")
    git(upstream, "add", "-A")
    git(upstream, "commit", "-q", "-m", "one")
    git(repo, "-c", "protocol.file.allow=always", "submodule", "add", "-q", str(upstream), "sub")
    git(repo, "commit", "-q", "-m", "add sub")
    git(repo, "tag", "-f", "base")

    (repo / "sub" / "row.txt").write_text("two\n")
    git(repo / "sub", "commit", "-qam", "two")
    (repo / "crates/alpha/src/lib.rs").write_text("// two\n")
    ran(repo, ["cargo", "test", "-p", "alpha"])
    git(repo, "add", "sub", "crates/alpha/src/lib.rs")
    git(repo, "commit", "-q", "-m", "alpha and the pointer")

    assert verdicts(repo) == {"cargo test -p alpha": (True, "passed on this change")}


def test_a_change_to_the_ldtk_tool_requires_its_tests() -> None:
    from required_checks import LDTK_TOOLS_JOB, requirements_for

    assert requirements_for("tools/ambition_ldtk_tools/ambition_ldtk_tools/ldtk/paths.py", {}) == {
        Job(LDTK_TOOLS_JOB)
    }
    assert requirements_for("tools/ambition_ldtk_tools/README.md", {}) == {Job(REPO_TOOLING_JOB)}


def test_a_change_to_the_sprite_tree_requires_its_embedded_road() -> None:
    from required_checks import SPRITE_TREE_EMBEDDED_JOB, requirements_for

    members = {"crates/ambition_sprite_sheet": "ambition_sprite_sheet"}
    assert Job(SPRITE_TREE_EMBEDDED_JOB) in requirements_for("crates/ambition_sprite_sheet/build.rs", members)
    # Control: another crate's change does not require it.
    assert Job(SPRITE_TREE_EMBEDDED_JOB) not in requirements_for("crates/other/src/lib.rs", {"crates/other": "other"})


def test_each_job_a_rule_names_is_a_job_of_the_runner() -> None:
    """A rule that names a job the runner does not plan can never be met."""
    import run_tests

    from required_checks import LDTK_TOOLS_JOB, SPRITE_TREE_EMBEDDED_JOB

    from required_checks import DETACHED_TOOL_JOBS, remedy

    planned = {job.name for job in run_tests.build_jobs([], heavy=True, libtest_args=[], everything=True)}
    detached = {job.name for job in run_tests.build_detached_tool_jobs()}
    assert SPRITE_TREE_EMBEDDED_JOB == run_tests.SPRITE_TREE_EMBEDDED_JOB
    from required_checks import UNION_FEATURE_JOB

    for name in (REPO_TOOLING_JOB, LDTK_TOOLS_JOB, SPRITE_TREE_EMBEDDED_JOB, UNION_FEATURE_JOB):
        assert name in (detached if name in DETACHED_TOOL_JOBS else planned), name
    # Planned only in the exhaustive mode, so its command must ask for it.
    default_plan = {job.name for job in run_tests.build_jobs([], heavy=False, libtest_args=[])}
    assert UNION_FEATURE_JOB not in default_plan
    assert "--run-everything-you-probably-dont-need-this --only-job" in remedy([Job(UNION_FEATURE_JOB)])
    assert "--tool-tests --only-job 'ldtk authoring tool tests'" in remedy([Job(LDTK_TOOLS_JOB)])


@pytest.mark.parametrize(
    ("argv", "name", "expected"),
    [
        (["cargo", "test", "-p", "ambition_app"], "rollback_", True),
        (["cargo", "test", "--workspace"], "rollback_", True),
        (["cargo", "test", "-p", "ambition_app", "--", "rollback_"], "rollback_", True),
        # A shorter filter runs more: every test that holds `rollback_` holds it.
        (["cargo", "test", "-p", "ambition_app", "--", "declared"], "declared_art_resolves", True),
        (["cargo", "test", "-p", "ambition_app", "--", "rollback_contact"], "rollback_", False),
        (["cargo", "test", "-p", "ambition_app", "--", "other", "rollback_"], "rollback_", True),
        (["cargo", "test", "-p", "ambition_app", "--", "other"], "rollback_", False),
        (["cargo", "test", "-p", "ambition_app", "--", "rollback_", "--exact"], "rollback_", False),
        (["cargo", "test", "-p", "ambition_app", "--", "rollback_", "--skip", "x"], "rollback_", False),
        (["cargo", "test", "-p", "ambition_app", "--lib", "--", "rollback_"], "rollback_", False),
        (["cargo", "test", "-p", "other", "--", "rollback_"], "rollback_", False),
        (["cargo", "nextest", "run", "-p", "ambition_app", "rollback_"], "rollback_", True),
        (["cargo", "nextest", "run", "-p", "ambition_app", "rollback_contact"], "rollback_", False),
        (["cargo", "nextest", "run", "-p", "ambition_app", "--run-ignored", "only"], "rollback_", False),
    ],
)
def test_a_named_check_is_covered_by_a_run_that_ran_every_test_with_that_name(
    argv: list[str], name: str, expected: bool
) -> None:
    from required_checks import CargoTestNamed

    assert covers({"argv": argv}, CargoTestNamed("ambition_app", name)) is expected


def test_a_content_change_requires_the_content_arms_and_a_registration_the_rollback_arms() -> None:
    from required_checks import APP_PACKAGE, CargoTestNamed, remedy, requirements_for

    content = requirements_for("game/ambition_content/assets/worlds/intro.ldtk", {})
    assert content == {
        CargoTestNamed(APP_PACKAGE, "declared_art_resolves"),
        CargoTestNamed(APP_PACKAGE, "registered_character_art"),
    }
    assert requirements_for("game/ambition_map_assets", {}) == content
    registration = requirements_for("crates/ambition_combat/src/rollback_registration.rs", {})
    assert {CargoTestNamed(APP_PACKAGE, "rollback_"), Job(REPO_TOOLING_JOB)} <= registration
    assert "./run_tests.sh -p ambition_app -k rollback_ --only-job '(default features)'" in remedy(
        [CargoTestNamed(APP_PACKAGE, "rollback_")]
    )


def test_a_registration_requires_the_union_feature_job() -> None:
    """A registration can add a row that only a feature's composition
    registers: the `portal` composition's presence probes were red in the
    union job alone (2026-10-12)."""
    from required_checks import UNION_FEATURE_JOB, requirements_for

    members = {"crates/ambition_portal2d": "ambition_portal2d"}
    assert Job(UNION_FEATURE_JOB) in requirements_for("crates/ambition_portal2d/src/rollback_registration.rs", members)
    # Control: the crate's other files do not.
    assert Job(UNION_FEATURE_JOB) not in requirements_for("crates/ambition_portal2d/src/extension.rs", members)


def test_a_file_with_a_test_behind_an_opt_in_feature_requires_the_union_feature_job(tmp_path: Path) -> None:
    from required_checks import UNION_FEATURE_JOB, requirements_for

    crate = tmp_path / "crates" / "alpha"
    (crate / "src").mkdir(parents=True)
    (crate / "tests").mkdir()
    (crate / "Cargo.toml").write_text('[package]\nname = "alpha"\n[features]\ndefault = ["on"]\non = []\nextra = []\n')
    (crate / "tests" / "gated.rs").write_text('#![cfg(feature = "extra")]\n#[test]\nfn t() {}\n')
    (crate / "tests" / "default_on.rs").write_text('#![cfg(feature = "on")]\n#[test]\nfn t() {}\n')
    (crate / "src" / "lib.rs").write_text('#[cfg(feature = "extra")]\nmod hidden;\n#[test]\nfn bare() {}\n')
    (crate / "src" / "hidden.rs").write_text('#[test]\nfn t() {}\n')
    members = {"crates/alpha": "alpha"}

    def union(path: str) -> bool:
        return Job(UNION_FEATURE_JOB) in requirements_for(path, members, tmp_path)

    assert union("crates/alpha/tests/gated.rs")
    assert union("crates/alpha/src/hidden.rs"), "reached only through a gated `mod` declaration"
    # Controls: a gate on a default feature hides nothing, a bare test is run
    # by the crate's own lane, and a path that is gone holds nothing.
    assert not union("crates/alpha/tests/default_on.rs")
    assert not union("crates/alpha/src/lib.rs")
    assert not union("crates/alpha/tests/deleted.rs")


def test_a_demo_change_requires_its_apps_tests_behind_its_own_features() -> None:
    from required_checks import own_features_job, remedy, requirements_for

    mary_o = own_features_job("ambition_demo_mary_o_app")
    # The two roads of 2026-10-12: the demo's HUD declaration, and its map.
    assert mary_o in requirements_for("game/ambition_demo_mary_o/src/provider.rs", {})
    assert mary_o in requirements_for("game/ambition_map_assets/ambition_demo_mary_o/worlds/mary_o.ldtk", {})
    assert mary_o in requirements_for("game/ambition_demo_mary_o_modules/src/squash.rs", {})
    assert own_features_job("ambition_demo_sanic_app") in requirements_for(
        "game/ambition_demo_sanic_app/tests/ov1_draws_the_world.rs", {}
    )
    # Controls: content that no demo app draws, and a demo with no host app.
    for path in ("game/ambition_content/src/lib.rs", "game/ambition_demo_pocket/src/lib.rs", "game/ambition_map_assets/ambition_content/x.ldtk"):
        assert not any(r.name.endswith(" — its tests behind its own features") for r in requirements_for(path, {}) if isinstance(r, Job)), path
    assert (
        "./run_tests.sh -p ambition_demo_mary_o_app --only-job "
        "'ambition_demo_mary_o_app — its tests behind its own features'"
    ) in remedy([mary_o])


def test_each_demo_apps_own_feature_job_is_planned_for_its_package() -> None:
    """The remedy runs `-p <app> --only-job <name>`: that plan must hold the job."""
    import run_tests

    from required_checks import DEMO_HOST_APPS, own_features_job

    assert run_tests.DEMO_HOST_APPS == DEMO_HOST_APPS
    for app in DEMO_HOST_APPS:
        planned = {job.name for job in run_tests.build_jobs([app], heavy=False, libtest_args=[])}
        assert own_features_job(app).name in planned, app


def test_the_named_rules_name_what_the_live_tree_has() -> None:
    """A pattern that matches no file, or an arm no module holds, can never
    be met or never asks."""
    from required_checks import CONTENT_ARMS, CONTENT_PATHS, ROLLBACK_REGISTRATION_PATHS

    tracked = git(REPO, "ls-files", "--", "crates", "game").splitlines()
    for pattern in (*CONTENT_PATHS, *ROLLBACK_REGISTRATION_PATHS):
        assert any(required_checks.fnmatch.fnmatch(path, pattern) for path in tracked), pattern
    modules = (REPO / "game/ambition_app/tests/app_it.rs").read_text()
    for arm in (*CONTENT_ARMS, "rollback_"):
        assert f"mod {arm}" in modules, f"no app_it module name holds `{arm}`"


def test_a_module_change_requires_the_harnesses_that_run_it() -> None:
    from required_checks import APP_PACKAGE, MODULE_ARMS, MODULE_HARNESS_PACKAGE, CargoTestNamed, requirements_for

    members = {"game/ambition_content_modules": "ambition_content_modules"}
    required = requirements_for("game/ambition_content_modules/src/fsm.rs", members)
    assert required == {
        CargoTest("ambition_content_modules"),
        CargoTest(MODULE_HARNESS_PACKAGE),
        *(CargoTestNamed(APP_PACKAGE, arm) for arm in MODULE_ARMS),
    }
    # The control: a crate the modules do not live in asks only for its own tests.
    assert requirements_for("game/ambition_content_modules_x/src/a.rs", {}) == set()


def test_every_app_arm_that_names_the_modules_is_held() -> None:
    from required_checks import MODULE_ARMS

    modules = (REPO / "game/ambition_app/tests/app_it.rs").read_text()
    for arm in MODULE_ARMS:
        assert f"mod {arm};" in modules, f"no app_it module `{arm}`"
    naming = sorted(
        path.stem for path in (REPO / "game/ambition_app/tests").glob("*.rs")
        if "ambition_content_modules" in path.read_text(errors="replace") and f"mod {path.stem};" in modules
    )
    missing = [arm for arm in naming if arm not in MODULE_ARMS]
    assert naming and not missing, f"app_it arms that run a module and are not held: {missing}"


def test_run_runs_only_the_checks_that_are_not_certified_and_judges_again(repo: Path) -> None:
    """⭐ `--run`: the change selects the checks. A change to `alpha` and `beta`
    with a pass of `alpha` only runs the command for `beta`, and the verdict
    after it is read from the ledger the run wrote. The control: with every
    check certified, nothing runs."""
    from required_checks import certify

    (repo / "crates/alpha/src/lib.rs").write_text("// two\n")
    (repo / "crates/beta/src/lib.rs").write_text("// two\n")
    ran(repo, ["cargo", "test", "-p", "alpha"])
    git(repo, "commit", "-qam", "alpha and beta")
    commands: list[list[str]] = []

    def runner(command: list[str], cwd: Path) -> None:
        commands.append(command)
        ran(repo, ["cargo", "test", "-p", "beta"])

    _, found = certify(repo, "base", "HEAD", run=runner)
    assert commands == [["./run_tests.sh", "-p", "beta", "--only-job", "(default features)"]]
    assert {v.requirement.label(): v.certified for v in found} == {
        "cargo test -p alpha": True,
        "cargo test -p beta": True,
    }

    commands.clear()
    certify(repo, "base", "HEAD", run=runner)
    assert commands == [], "control: with every check certified, nothing runs"


def rebase_at(repo: Path, files: dict[str, str]) -> None:
    """Write `files` and move the `base` tag onto a commit that holds them."""
    for path, text in files.items():
        (repo / path).parent.mkdir(parents=True, exist_ok=True)
        (repo / path).write_text(text)
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", "base manifests")
    git(repo, "tag", "-f", "base")


ALPHA_MANIFEST = '[package]\nname = "alpha"\nversion = "0.1.0"\n[dependencies]\nserde = "1"\n[features]\ndefault = []\nextra = []\n'
ROOT_MANIFEST = (
    '[workspace]\nmembers = [\n    "crates/alpha",\n    "crates/beta",\n]\n'
    '[workspace.dependencies]\nbevy = { version = "0.18", default-features = false }\n'
)


@pytest.mark.parametrize(
    ("path", "before", "after", "wiring"),
    [
        ("crates/alpha/Cargo.toml", ALPHA_MANIFEST, ALPHA_MANIFEST.replace("extra = []", "extra = []\nmore = []"), True),
        ("crates/alpha/Cargo.toml", ALPHA_MANIFEST,
         ALPHA_MANIFEST.replace('serde = "1"', 'serde = { version = "1", features = ["derive"] }'), True),
        ("crates/alpha/Cargo.toml", ALPHA_MANIFEST,
         ALPHA_MANIFEST.replace('serde = "1"', 'serde = { version = "1", optional = true }'), True),
        ("Cargo.toml", ROOT_MANIFEST, ROOT_MANIFEST.replace("default-features = false", 'default-features = false, features = ["x"]'), True),
        # Controls: a version, a new plain dependency and package metadata are
        # not feature wiring.
        ("crates/alpha/Cargo.toml", ALPHA_MANIFEST, ALPHA_MANIFEST.replace('serde = "1"', 'serde = "1.0.200"'), False),
        ("crates/alpha/Cargo.toml", ALPHA_MANIFEST, ALPHA_MANIFEST.replace('serde = "1"', 'serde = "1"\nlog = "0.4"'), False),
        ("crates/alpha/Cargo.toml", ALPHA_MANIFEST, ALPHA_MANIFEST.replace('version = "0.1.0"', 'version = "0.2.0"'), False),
        ("Cargo.toml", ROOT_MANIFEST, ROOT_MANIFEST.replace('"0.18"', '"0.18.1"'), False),
    ],
)
def test_a_change_to_feature_wiring_requires_the_union_feature_job(
    repo: Path, path: str, before: str, after: str, wiring: bool
) -> None:
    """The boundary table gives "a change to features" to the exhaustive run:
    a feature, or a dependency's `features`, `optional` or `default-features`,
    in the root or a member manifest requires the union job."""
    from required_checks import UNION_FEATURE_JOB

    rebase_at(repo, {path: before})
    commit(repo, path, after)
    found = verdicts(repo)
    assert (Job(UNION_FEATURE_JOB).label() in found) == wiring


def test_a_manifest_outside_the_workspace_is_not_its_wiring(repo: Path) -> None:
    """An external consumer fixture builds against its own lock and features:
    the union job does not build it."""
    from required_checks import UNION_FEATURE_JOB

    rebase_at(repo, {"fixtures/outside/Cargo.toml": ALPHA_MANIFEST})
    commit(repo, "fixtures/outside/Cargo.toml", ALPHA_MANIFEST.replace("extra = []", "extra = []\nmore = []"))
    assert Job(UNION_FEATURE_JOB).label() not in verdicts(repo)


def a_dependent_that_hides_a_test(repo: Path, depends: bool) -> None:
    """`beta` hides a test behind its opt-in `extra`; it depends on `alpha` when
    `depends`. `alpha` hides none."""
    beta = '[package]\nname = "beta"\n[features]\nextra = []\n'
    if depends:
        beta += '[dependencies]\nalpha = { path = "../alpha" }\n'
    rebase_at(repo, {
        "crates/alpha/Cargo.toml": '[package]\nname = "alpha"\n',
        "crates/beta/Cargo.toml": beta,
        "crates/beta/tests/gated.rs": '#![cfg(feature = "extra")]\n#[test]\nfn t() {}\n',
    })


def test_a_change_whose_dependents_hide_tests_names_the_union_job_as_not_required(repo: Path, capsys) -> None:
    """⭐ NO CHECK IS SILENTLY SKIPPED. A change in `alpha` can redden the gated
    test of `beta`, which depends on it. This boundary does not require the
    union job for that (it is the release's), so the receipt names it as NOT
    RUN, not required at this boundary, and the change is still certified."""
    from required_checks import UNION_FEATURE_JOB, report

    a_dependent_that_hides_a_test(repo, depends=True)
    (repo / "crates/alpha/src/lib.rs").write_text("// two\n")
    ran(repo, ["cargo", "test", "-p", "alpha"])
    git(repo, "commit", "-qam", "alpha")
    paths, found = judge(repo, "base", "HEAD")
    union = next((v for v in found if v.requirement == Job(UNION_FEATURE_JOB)), None)
    assert union is not None, "the receipt does not name the union job for a change its dependents' gated tests can see"
    assert not union.required and not union.certified
    assert union.reason.startswith("NOT RUN, not required at this boundary"), union.reason
    assert "beta" in union.reason
    assert report(paths, found) == 0, "a check this boundary does not require does not refuse the change"
    assert "defer" in capsys.readouterr().out

    # A failure on this change is a failure, required or not.
    ran(repo, ["cargo", "test", "--workspace"], ok=False, job=UNION_FEATURE_JOB)
    paths, found = judge(repo, "base", "HEAD")
    union = next((v for v in found if v.requirement == Job(UNION_FEATURE_JOB)), None)
    assert union is not None, "the receipt does not name the union job for a change its dependents' gated tests can see"
    assert union.reason == "FAILED on this change" and union.blocks()
    assert report(paths, found) == 1


def test_a_change_with_no_dependent_that_hides_a_test_names_no_union_job(repo: Path) -> None:
    """Control: the same change, with `beta` not depending on `alpha`."""
    from required_checks import UNION_FEATURE_JOB

    a_dependent_that_hides_a_test(repo, depends=False)
    commit(repo, "crates/alpha/src/lib.rs", "// two\n")
    assert Job(UNION_FEATURE_JOB).label() not in verdicts(repo)


def test_run_does_not_run_a_check_this_boundary_does_not_require(repo: Path) -> None:
    from required_checks import certify

    a_dependent_that_hides_a_test(repo, depends=True)
    commit(repo, "crates/alpha/src/lib.rs", "// two\n")
    commands: list[list[str]] = []
    certify(repo, "base", "HEAD", run=lambda command, cwd: commands.append(command))
    assert commands == [["./run_tests.sh", "-p", "alpha", "--only-job", "(default features)"]]
