"""**WHICH TREE THE GOAL GUARD JUDGES**, pinned against a real worktree.

The resolver in `scripts/goal_guard_hook.sh` sits between two failures that
have both actually happened, and a fix for either one alone re-opens the other:

- taking `$CLAUDE_PROJECT_DIR` unconditionally made a worktree session judge the
  MAIN checkout — *"main had 132 dirty files and 2 compile errors while this
  branch was clean at 445/0"*;
- taking `$PWD` unconditionally made one `cd` into a nested repository resolve a
  root with no `.goal/active.json`, so the guard took its "not armed" path and
  SILENTLY RELEASED a 72-hour run (2026-08-05).

⚠ **the fixtures are real `git worktree` and real nested repositories**, because
the discriminator IS a git fact (the common git dir) and a mocked one would
agree with whatever the script asked it.
"""

import subprocess
from pathlib import Path

HOOK = Path(__file__).resolve().parents[1] / "goal_guard_hook.sh"


def _git(cwd, *args):
    subprocess.run(
        ["git", *args],
        cwd=cwd,
        check=True,
        capture_output=True,
        env={"HOME": str(cwd), "PATH": "/usr/bin:/bin", "GIT_CONFIG_NOSYSTEM": "1"},
    )


def _repo(root: Path) -> Path:
    """A repository carrying a guard that prints where it was run from."""
    root.mkdir(parents=True, exist_ok=True)
    scripts = root / "scripts"
    scripts.mkdir(exist_ok=True)
    (scripts / "goal_guard.py").write_text(
        "import sys, pathlib\nprint(pathlib.Path(__file__).resolve().parents[1])\n"
    )
    _git(root, "init", "-q", "-b", "main")
    _git(root, "config", "user.email", "t@t")
    _git(root, "config", "user.name", "t")
    _git(root, "add", "-A")
    _git(root, "commit", "-qm", "guard")
    return root


def _run(cwd: Path, declared: Path) -> str:
    done = subprocess.run(
        ["bash", str(HOOK)],
        cwd=cwd,
        capture_output=True,
        text=True,
        env={
            "PATH": "/usr/bin:/bin",
            "HOME": str(cwd),
            "CLAUDE_PROJECT_DIR": str(declared),
            "GIT_CONFIG_NOSYSTEM": "1",
        },
    )
    return done.stdout.strip()


def test_a_worktree_session_judges_its_own_tree(tmp_path):
    """⛔ the failure this replaces: the lane was clean and the verdict was main's."""
    main = _repo(tmp_path / "main")
    lane = tmp_path / "lane"
    _git(main, "worktree", "add", "-q", "-b", "lane", str(lane))

    # The shape that breaks it: the session STARTED in main, so it still declares
    # main, and only the working directory moved.
    assert _run(lane, main) == str(lane)


def test_a_stray_cd_into_a_nested_repository_keeps_the_declared_tree(tmp_path):
    """⛔ the 2026-08-05 failure: a nested repo silently released a 72-hour run.

    ⚠ the nested repository carries its OWN guard, which is what makes it a
    convincing wrong answer — the walk finds one and it is not this project's.
    """
    main = _repo(tmp_path / "main")
    nested = _repo(main / "vendor" / "other")

    assert _run(nested, main) == str(main)


def test_the_ordinary_session_is_unchanged(tmp_path):
    """The case that is neither: cwd and declaration are the same tree."""
    main = _repo(tmp_path / "main")
    assert _run(main, main) == str(main)
    assert _run(main / "scripts", main) == str(main)


def test_a_missing_guard_blocks_rather_than_passing_silently(tmp_path):
    """⛔ a guard that cannot be found must never read as 'nothing to check'.

    That is the whole failure mode this tooling exists to prevent, and a hook
    that exits 0 on a missing script is indistinguishable from a green run.
    """
    bare = tmp_path / "bare"
    bare.mkdir()
    out = _run(bare, bare)
    assert '"decision":"block"' in out, out
