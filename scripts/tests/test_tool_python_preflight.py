"""The tool-Python preflight names what is missing and offers the active env.

`scripts/regen/sprites.sh` used to check only `import ambition_sprite2d_renderer`.
A machine tool venv without numpy passed that, and the run died on the first
pirate as "publish roster names an unregistered target … No module named
'numpy'" — while the user's activated environment had every dependency.

These arms pin the replacement, `ambition_preflight_tool_pythons`: it checks the
DECLARED dependencies, names the interpreter it judged and why it was chosen,
and when the active venv would do, prints the command that uses it — without
switching to it.
"""

from __future__ import annotations

import os
import subprocess
import sys
import venv
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
LIB = REPO / "scripts/lib/tool_python.sh"
CHECKER = REPO / "scripts/lib/tool_requirements.py"

ACTIVE = Path(sys.prefix) / "bin" / "python"
pytestmark = pytest.mark.skipif(
    not ACTIVE.exists() or sys.prefix == sys.base_prefix,
    reason="needs to run from a venv so it can stand in as the active one",
)


def tool(tmp_path: Path, *deps: str) -> Path:
    """A tool project whose own package is `pytest` (installed in this env)."""
    project = tmp_path / "tool"
    project.mkdir()
    listed = ", ".join(f'"{d}"' for d in deps)
    (project / "pyproject.toml").write_text(
        f'[project]\nname = "pytest"\nversion = "0"\ndependencies = [{listed}]\n'
    )
    return project


@pytest.fixture(scope="module")
def bare_python(tmp_path_factory) -> Path:
    """An interpreter with nothing installed: the stale tool venv."""
    root = tmp_path_factory.mktemp("bare")
    venv.create(root, with_pip=False)
    return root / "bin" / "python"


def preflight(project: Path, chosen: Path) -> subprocess.CompletedProcess:
    script = (
        f'set -euo pipefail; source "{LIB}"; '
        f'ambition_preflight_tool_pythons "./regen.sh --force" "{project}" TOOL_PY 0'
    )
    env = {k: v for k, v in os.environ.items() if k not in ("AMBITION_PYTHON", "PYTHON", "CONDA_PREFIX")}
    env.update(TOOL_PY=str(chosen), VIRTUAL_ENV=sys.prefix)
    return subprocess.run(["bash", "-c", script], capture_output=True, text=True, env=env)


def test_the_checker_names_each_missing_dependency_and_the_tool_itself(tmp_path, bare_python):
    project = tool(tmp_path, "pluggy")
    out = subprocess.run(
        [bare_python, CHECKER, project], capture_output=True, text=True
    )
    assert out.returncode == 1
    assert out.stdout.splitlines() == [
        f"missing\t{project}\tpluggy",
        f"uninstalled\t{project}\tpytest",
    ]


def test_the_checker_is_silent_for_a_fit_interpreter(tmp_path):
    out = subprocess.run(
        [ACTIVE, CHECKER, tool(tmp_path, "pluggy")], capture_output=True, text=True
    )
    assert (out.returncode, out.stdout) == (0, "")


def test_a_fit_interpreter_passes_silently(tmp_path):
    out = preflight(tool(tmp_path, "pluggy"), ACTIVE)
    assert (out.returncode, out.stderr) == (0, "")


def test_an_unfit_interpreter_is_named_and_the_active_env_is_offered(tmp_path, bare_python):
    """⭐ THE CASE THAT CAUSED IT: chosen venv lacks a dependency, active env has all."""
    out = preflight(tool(tmp_path, "pluggy"), bare_python)
    assert out.returncode == 1
    assert f"interpreter : {bare_python}" in out.stderr
    assert "chosen by   : TOOL_PY" in out.stderr
    assert "lacks       : pluggy, the pytest package itself" in out.stderr
    # The override is set, so AMBITION_PYTHON alone would not reach the tool.
    assert f"AMBITION_PYTHON={ACTIVE} TOOL_PY={ACTIVE} ./regen.sh --force" in out.stderr
    assert f"export AMBITION_PYTHON={ACTIVE}" in out.stderr


def test_an_active_env_with_every_dependency_but_not_the_tool_gets_an_install_line(tmp_path, bare_python):
    project = tmp_path / "tool"
    project.mkdir()
    (project / "pyproject.toml").write_text(
        '[project]\nname = "not-installed-ambition-tool"\nversion = "0"\ndependencies = ["pluggy"]\n'
    )
    out = preflight(project, bare_python)
    assert out.returncode == 1
    assert "has every dependency, but not the tools themselves" in out.stderr
    assert f"-e {project}" in out.stderr
    assert f"AMBITION_PYTHON={ACTIVE} TOOL_PY={ACTIVE} ./regen.sh --force" in out.stderr


def test_no_offer_when_the_active_env_would_fail_too(tmp_path, bare_python):
    out = preflight(tool(tmp_path, "pluggy", "no-such-distribution-ambition"), bare_python)
    assert out.returncode == 1
    assert "has everything" not in out.stderr
    assert f"Your environment at {ACTIVE} would not do as it is" in out.stderr
    assert f"AMBITION_PYTHON={ACTIVE} ./scripts/setup/python_tools.sh" in out.stderr
    assert "no-such-distribution-ambition" in out.stderr
    assert "./run_developer_setup.sh" in out.stderr


def test_a_bare_interpreter_is_refused_for_the_real_renderer(bare_python):
    """The original failure, against the real pyproject: numpy must be named."""
    out = subprocess.run(
        [bare_python, CHECKER, REPO / "tools/ambition_sprite2d_renderer"],
        capture_output=True, text=True,
    )
    assert out.returncode == 1, out.stderr
    assert any(line.split("\t")[2].startswith("numpy") for line in out.stdout.splitlines())


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
