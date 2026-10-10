"""A worker pool of the repo takes at most 6 jobs when it is not told a number.

The machine is shared: at most 6 parallel jobs, and no pool that takes every
CPU by default (Jon, 2026-10-03). Each default below is read on a machine of
32 cores (a fake `nproc` first on PATH), where a share of the cores is more
than 6. `run_tests.py` has its own witness (`test_run_tests_job_cap.py`).
"""

from __future__ import annotations

import ast
import os
import re
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CAP = 6


def _with_cores(tmp_path: Path, cores: int) -> dict[str, str]:
    shim = tmp_path / "bin"
    shim.mkdir(parents=True)
    (shim / "nproc").write_text(f"#!/bin/sh\necho {cores}\n")
    (shim / "nproc").chmod(0o755)
    env = {k: v for k, v in os.environ.items() if k != "AMBITION_SPRITE_JOBS"}
    env["PATH"] = f"{shim}:{env['PATH']}"
    return env


def _sprite_jobs(tmp_path: Path, cores: int) -> str:
    text = (REPO / "scripts/regen/sprites.sh").read_text()
    block = re.search(r'^if \[ -z "\$\{AMBITION_SPRITE_JOBS:-\}" \]; then\n.*?^fi\n', text, re.S | re.M)
    assert block, "sprites.sh no longer sets AMBITION_SPRITE_JOBS in one block: find the default again"
    return subprocess.run(
        ["bash", "-c", block.group(0) + 'echo "$AMBITION_SPRITE_JOBS"'],
        env=_with_cores(tmp_path, cores), capture_output=True, text=True, check=True,
    ).stdout.strip()


def test_the_sprite_publish_pool(tmp_path: Path) -> None:
    assert _sprite_jobs(tmp_path / "big", 32) == str(CAP)
    # The control: on 4 cores the share (half) is below the cap and stands.
    assert _sprite_jobs(tmp_path / "small", 4) == "2"


def test_the_worktree_budget_of_main(tmp_path: Path) -> None:
    env = _with_cores(tmp_path, 32)
    seen = {
        slot: subprocess.run(
            ["bash", str(REPO / "scripts/agent_worktree.sh"), "jobs", slot],
            env=env, capture_output=True, text=True, check=True,
        ).stdout.strip()
        for slot in ("main", "1")
    }
    assert seen == {"main": str(CAP), "1": str(CAP // 2)}, seen


def test_the_audio_level_pool() -> None:
    tree = ast.parse((REPO / "scripts/audio_levels.py").read_text())
    defaults = [
        kw.value
        for node in ast.walk(tree)
        if isinstance(node, ast.Call) and getattr(node.func, "attr", None) == "add_argument"
        and node.args and isinstance(node.args[0], ast.Constant) and node.args[0].value == "--jobs"
        for kw in node.keywords if kw.arg == "default"
    ]
    assert len(defaults) == 1, "audio_levels.py has no single --jobs default: find it again"
    call = defaults[0]
    assert isinstance(call, ast.Call) and getattr(call.func, "id", None) == "min", ast.unparse(call)
    bound = call.args[0]
    assert isinstance(bound, ast.Constant) and bound.value <= CAP, ast.unparse(call)
