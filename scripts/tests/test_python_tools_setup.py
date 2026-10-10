"""Setup and `--verify` install and check each tool where its regen script looks.

`scripts/lib/tool_python.sh` resolves a tool's interpreter as: the tool's own
`AMBITION_<TOOL>_PYTHON`, then `AMBITION_PYTHON`, then a venv.
`test_tool_python_resolution.py` proves that order for the resolver alone.
These tests run `scripts/setup/python_tools.sh` itself.

⛔⛔ BEFORE 2026-09-30, SETUP AND `--verify` DID NOT READ THE TOOL VARIABLE. With
`AMBITION_PYTHON` = 3.13 and `AMBITION_SFX_PYTHON` = 3.12 (Jon's desk: one global
3.13 environment, and the SFX renderer needs Python < 3.13), setup tried the SFX
renderer in 3.13. The install failed, setup said "set AMBITION_SFX_PYTHON"
(already set) and returned success. `scripts/regen/sfx.sh` then chose 3.12, where
nothing had been installed.

The fixture puts a fake `uv` first on PATH. It records each install and fails
an install of the SFX renderer into the "3.13" interpreter, as the real one
does. The two interpreters are shell scripts that answer `-c "import ..."`.
"""

from __future__ import annotations

import os
import re
import subprocess
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SETUP = REPO / "scripts/setup/python_tools.sh"

FAKE_UV = """#!/bin/sh
if [ "$1" = "--version" ]; then echo "uv 0.0.0 (fake)"; exit 0; fi
python=""
prev=""
for arg in "$@"; do
    if [ "$prev" = "--python" ]; then python="$arg"; fi
    prev="$arg"
done
printf '%s|%s|%s\\n' "$(basename "$PWD")" "$python" "$*" >> "$UV_LOG"
case "$python:$(basename "$PWD")" in
    "$PY313:ambition_sfx_renderer") echo "requires-python <3.13" >&2; exit 1 ;;
esac
exit 0
"""

# The 3.13 interpreter cannot import the SFX renderer; the 3.12 one imports all.
FAKE_PY313 = """#!/bin/sh
case "$*" in *"import ambition_sfx_renderer"*) exit 1 ;; esac
exit 0
"""
FAKE_PY312 = "#!/bin/sh\nexit 0\n"


@pytest.fixture
def desk(tmp_path):
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    for name, text in (("uv", FAKE_UV), ("py313", FAKE_PY313), ("py312", FAKE_PY312)):
        exe = bin_dir / name
        exe.write_text(text)
        exe.chmod(0o755)
    log = tmp_path / "uv.log"
    log.write_text("")
    env = {
        k: v
        for k, v in os.environ.items()
        if not (k == "PYTHON" or re.fullmatch(r"AMBITION_\w*PYTHON", k))
    }
    env.update(
        PATH=f"{bin_dir}{os.pathsep}{os.environ['PATH']}",
        UV_LOG=str(log),
        UV_EXCLUDE_NEWER="2026-01-01",
        AMBITION_TOOL_VENVS=str(tmp_path / "store"),
        PY313=str(bin_dir / "py313"),
        PY312=str(bin_dir / "py312"),
    )

    def run(*args: str, **extra: str):
        result = subprocess.run(
            ["bash", str(SETUP), *args],
            capture_output=True, text=True, cwd=REPO, env={**env, **extra},
        )
        installs = [line.split("|", 2) for line in log.read_text().splitlines()]
        return result, installs

    return run, env["PY313"], env["PY312"]


def sfx_installs(installs, python: str) -> int:
    return sum(
        1 for cwd, py, args in installs
        if cwd == "ambition_sfx_renderer" and py == python and "-e" in args.split()
    )


def test_setup_installs_a_tool_into_its_own_interpreter(desk):
    """⭐ THE MOTIVATING DESK. The SFX renderer goes into 3.12, and only there."""
    run, py313, py312 = desk
    result, installs = run(AMBITION_PYTHON=py313, AMBITION_SFX_PYTHON=py312)
    assert result.returncode == 0, result.stderr
    assert sfx_installs(installs, py312) == 1, installs
    assert sfx_installs(installs, py313) == 0, "setup ignored AMBITION_SFX_PYTHON"
    assert "not installed into AMBITION_PYTHON" not in result.stderr


def test_a_failed_install_into_a_named_interpreter_stops_setup(desk):
    """⛔ The user named the interpreter. "Set AMBITION_SFX_PYTHON" is not advice
    that can help, so setup must fail rather than warn and return success."""
    run, py313, py312 = desk
    result, _ = run(AMBITION_PYTHON=py312, AMBITION_SFX_PYTHON=py313)
    assert result.returncode != 0
    assert "ambition_sfx_renderer" in result.stderr


def test_a_tool_the_global_interpreter_cannot_hold_is_named_with_its_variable(desk):
    """With no tool variable, the old warning stays, and names the right one."""
    run, py313, _py312 = desk
    result, _ = run(AMBITION_PYTHON=py313)
    assert result.returncode == 0, result.stderr
    assert "AMBITION_SFX_PYTHON" in result.stderr
    assert "not installed into AMBITION_PYTHON" in result.stderr


def test_verify_checks_the_interpreter_the_regen_script_will_use(desk):
    run, py313, py312 = desk
    ok, _ = run("--verify", AMBITION_PYTHON=py313, AMBITION_SFX_PYTHON=py312)
    assert ok.returncode == 0, ok.stderr
    bad, _ = run("--verify", AMBITION_PYTHON=py313)
    assert bad.returncode != 0
    assert "ambition_sfx_renderer" in bad.stderr


def _joined_shell_lines(path: Path):
    return path.read_text().replace("\\\n", " ").splitlines()


def test_the_setup_table_names_the_variables_the_regen_scripts_read():
    """The table in `tool_projects` and the names the callers give the resolver
    must be one set, or setup installs where nothing looks."""
    table = {}
    text = SETUP.read_text()
    body = text.split("tool_projects() {", 1)[1].split("<<'EOF'\n", 1)[1].split("\nEOF", 1)[0]
    for row in body.strip().splitlines():
        project, _module, variable, _editable = row.split()
        table[variable] = project
    assert len(table) == 5, table

    called = set()
    shell_files = [*REPO.glob("*.sh"), *(REPO / "scripts").rglob("*.sh")]
    for path in shell_files:
        for line in _joined_shell_lines(path):
            match = re.search(r'ambition_select_tool_python\s+\S+\s+(AMBITION_\w+_PYTHON)', line)
            if match:
                called.add(match.group(1))
    assert called, "no resolver call names a tool variable; the scan is broken"
    assert called <= set(table), f"read by a caller, absent from setup: {called - set(table)}"
    assert set(table) <= called, f"in setup, read by no caller: {set(table) - called}"
