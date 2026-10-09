"""A push is refused until the checks its change requires passed on it (TEST-LANES).

Each test builds a small git repository with a workspace of its own, so the
witness does not depend on what this checkout has run.
"""

from __future__ import annotations

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


def test_a_doc_change_requires_nothing(repo: Path) -> None:
    commit(repo, "docs/a.md", "two\n")
    assert verdicts(repo) == {}
    assert required_checks.report(*judge(repo, "base", "HEAD")) == 0


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
        "crates/ambition_platformer2d_actor_monolith/src/session/checkpoint.rs",
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


def test_the_hook_block_is_installed_once_and_keeps_other_text() -> None:
    from install_pre_push_hook import BLOCK, with_block

    other = "#!/bin/sh\necho other hook\n"
    once = with_block(other)
    assert once.startswith(other) and BLOCK.rstrip("\n") in once
    assert with_block(once) == once, "a second install must not add a second block"
    assert with_block("").startswith("#!/bin/sh\n")


def test_the_hook_checks_a_push_to_main_from_the_remote_tip(repo: Path) -> None:
    """git hands the hook `<local ref> <local sha> <remote ref> <remote sha>`;
    the base is the remote tip, and a deletion or another branch is not judged."""
    base = git(repo, "rev-parse", "base")
    commit(repo, "crates/alpha/src/lib.rs", "// two\n")
    head = git(repo, "rev-parse", "HEAD")
    zero = "0" * 40
    lines = [
        f"refs/heads/x {head} refs/heads/main {base}",
        f"refs/heads/x {head} refs/heads/feature {base}",
        f"(delete) {zero} refs/heads/main {base}",
    ]
    assert required_checks.pre_push_base(repo, lines) == [(base, head)]


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
