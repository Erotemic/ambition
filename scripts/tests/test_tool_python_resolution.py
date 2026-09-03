"""The tool-venv store is keyed by DIRECTORY BASENAME, and a worktree's is wrong.

`scripts/lib/tool_python.sh` resolves the interpreter every Python job in the
suite runs under. It looks up `$AMBITION_TOOL_VENVS/<basename of project dir>`.

⛔⛔ IN `.worktrees/agent-worktree1` THAT IS `agent-worktree1`, WHICH HAS NO
VENV. The resolver fell through to a bare `python3`, and `run_tests.py` then
refused the entire suite:

    run_tests: this interpreter cannot run the Python lane.
      interpreter : /usr/bin/python3
      missing     : soundfile, tree_sitter_rust
      affected    : 5 planned job(s)

⇒ The full gate could not run in ANY agent worktree — which is exactly where
the agents work, and the same shape as `check_authored_levels_survive` being
inoperative in those trees for the same reason. Found 2026-09-03.

⚠ THE FIX IS A FALLBACK, NOT A REPLACEMENT, and that is what these tests pin: a
TOOL venv is legitimately keyed by its own directory (`ambition_music_renderer`
→ that venv), so the basename lookup must still win where it resolves.
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
LIB = REPO / "scripts/lib/tool_python.sh"


def select(project_dir: Path, store: Path) -> str:
    """Run the shell resolver the way run_tests.sh does."""
    script = (
        f'source "{LIB}"; '
        f'ambition_select_tool_python "{project_dir}" "" 0'
    )
    env = {**os.environ, "AMBITION_TOOL_VENVS": str(store)}
    env.pop("PYTHON", None)
    out = subprocess.run(
        ["bash", "-c", script], capture_output=True, text=True, env=env
    )
    return out.stdout.strip()


def make_venv(store: Path, name: str) -> Path:
    exe = store / name / "bin" / "python"
    exe.parent.mkdir(parents=True, exist_ok=True)
    exe.write_text("#!/bin/sh\nexit 0\n")
    exe.chmod(0o755)
    return exe


@pytest.fixture
def tree(tmp_path):
    """A repo named `proj` with a worktree named `wt-1` — the real shape."""
    main = tmp_path / "proj"
    main.mkdir()
    git = lambda *a, **kw: subprocess.run(  # noqa: E731
        ["git", *a], cwd=kw.pop("cwd", main), capture_output=True, text=True, check=True
    )
    git("init", "-q")
    git("config", "user.email", "t@t")
    git("config", "user.name", "t")
    (main / "f").write_text("x")
    git("add", "f")
    git("commit", "-qm", "c")
    worktree = tmp_path / "wt-1"
    git("worktree", "add", "-q", str(worktree))
    return main, worktree, tmp_path / "store"


def test_a_worktree_finds_the_repositorys_venv(tree):
    """⭐ THE FIX. `wt-1` has no venv; `proj` does, and that is the one to use."""
    main, worktree, store = tree
    expected = make_venv(store, "proj")
    assert select(worktree, store) == str(expected)


def test_the_main_tree_is_unchanged(tree):
    main, _worktree, store = tree
    expected = make_venv(store, "proj")
    assert select(main, store) == str(expected)


def test_a_directorys_own_venv_still_wins(tree):
    """⛔ THE ARM THAT KEEPS TOOLS WORKING. A tool venv is keyed by the tool's
    own directory; the repo fallback must not steal it."""
    main, _worktree, store = tree
    tool_dir = main / "tools" / "renderer"
    tool_dir.mkdir(parents=True)
    make_venv(store, "proj")
    own = make_venv(store, "renderer")
    assert select(tool_dir, store) == str(own)


def test_no_venv_anywhere_still_falls_back_to_python3(tree):
    """⛔ The pre-existing behaviour for a fresh clone must survive."""
    main, worktree, store = tree
    store.mkdir(parents=True, exist_ok=True)
    assert select(worktree, store) in {"python3", "python"}


def test_this_repository_resolves_to_an_interpreter_that_can_host_the_lane():
    """The live case, asserted rather than assumed: whatever this checkout
    resolves to must actually import what the Python lane needs."""
    script = f'source "{LIB}"; ambition_select_tool_python "{REPO}" "" 0'
    env = {**os.environ}
    env.pop("PYTHON", None)
    chosen = subprocess.run(
        ["bash", "-c", script], capture_output=True, text=True, env=env
    ).stdout.strip()
    probe = subprocess.run(
        [chosen, "-c", "import soundfile, tree_sitter_rust"],
        capture_output=True, text=True,
    )
    assert probe.returncode == 0, (
        f"{chosen} cannot host the Python lane: {probe.stderr.strip()}\n"
        "run_tests.py refuses the whole suite when this happens."
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
