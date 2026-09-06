"""The authored-level guard must still run when the repo IS inside `.worktrees`.

`check_authored_levels_survive.py` refuses to pass silently — it prints
"⛔ found no .ldtk worlds at all — this guard just checked nothing" and exits 1
rather than reporting a clean sweep of nothing. That refusal is why this was
findable at all.

⛔⛔ BUT IT FIRED IN EVERY AGENT WORKTREE, FOR A REASON THAT HAD NOTHING TO DO
WITH THE LEVELS. `SKIP_PARTS` contains `.worktrees` — sensible, so a scan from
the main checkout does not walk into sibling slots — and it was matched against
each file's ABSOLUTE path. An agent slot's own root is
`<repo>/.worktrees/<slot>`, so every path under it contains `.worktrees` and
every world was skipped. Measured 2026-09-02 in this slot: 12 worlds on disk,
0 kept by the absolute test, 12 by the relative one.

⇒ The guard was inoperative in exactly the trees this project's agents work in,
and its own honest refusal read as "this checkout has no levels" rather than
"this filter ate them".
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/check_authored_levels_survive.py"


def load():
    spec = importlib.util.spec_from_file_location("levels", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _world(path: Path, *levels: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"levels": [{"identifier": n} for n in levels]}))


def test_a_repo_rooted_under_dot_worktrees_still_sees_its_levels(tmp_path, monkeypatch):
    """⭐ THE REGRESSION, REPRODUCED BY PATH SHAPE ALONE."""
    module = load()
    root = tmp_path / ".worktrees" / "agent-worktree1"
    _world(root / "game" / "worlds" / "intro.ldtk", "start", "hub")
    monkeypatch.setattr(module, "REPO", root)

    found = module.worlds()
    assert found == {"intro.ldtk": {"start", "hub"}}, (
        "a slot whose own root contains `.worktrees` must still find its worlds; "
        "matching SKIP_PARTS against the absolute path skips all of them"
    )


def test_a_nested_worktrees_directory_is_still_skipped(tmp_path, monkeypatch):
    """⛔ AND THE FILTER MUST KEEP DOING ITS JOB. Scanning from a main checkout
    must not walk into sibling slots and count their copies as separate worlds —
    that is what SKIP_PARTS is for, and a fix that merely deleted it would trade
    one wrong answer for another."""
    module = load()
    root = tmp_path / "main"
    _world(root / "game" / "worlds" / "intro.ldtk", "start")
    _world(root / ".worktrees" / "slot" / "game" / "worlds" / "intro.ldtk", "start", "extra")
    _world(root / "target" / "copy" / "intro.ldtk", "junk")
    monkeypatch.setattr(module, "REPO", root)

    assert module.worlds() == {"intro.ldtk": {"start"}}, (
        "the sibling slot's copy and the build directory's copy must not merge "
        "into the authored world's level set"
    )


def test_the_guard_still_refuses_an_empty_sweep(tmp_path, monkeypatch, capsys):
    """The refusal is the property that made this findable; keep it."""
    module = load()
    monkeypatch.setattr(module, "REPO", tmp_path)
    # `main` parses sys.argv, which pytest owns — give it its own.
    monkeypatch.setattr(sys, "argv", ["check_authored_levels_survive.py"])
    assert module.main() != 0
    assert "checked nothing" in capsys.readouterr().out


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
