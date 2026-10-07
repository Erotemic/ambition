"""`apt_ensure` must reach apt-get with DEBIAN_FRONTEND=noninteractive THROUGH sudo.

⛔ `DEBIAN_FRONTEND=noninteractive sudo apt-get ...` sets the variable for `sudo`,
and sudo's default `env_reset` drops it before apt-get runs, so a debconf dialog
(or needrestart's "pending kernel upgrade" whiptail) opened in the middle of an
unattended setup. This test puts a fake `sudo` first on PATH that resets the
environment the way the real one does, and a fake `apt-get` that records what it
was given.
"""

from __future__ import annotations

import os
import stat
import subprocess
from pathlib import Path

LIB = Path(__file__).resolve().parent.parent / "lib/apt_ensure.sh"


def _script(path: Path, body: str) -> None:
    path.write_text("#!/usr/bin/env bash\n" + body)
    path.chmod(path.stat().st_mode | stat.S_IEXEC)


def test_the_frontend_reaches_apt_get_through_an_env_resetting_sudo(tmp_path):
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    seen = tmp_path / "seen.txt"
    # env_reset: the command runs with a CLEAN environment (PATH kept).
    _script(bin_dir / "sudo", 'exec env -i PATH="$PATH" "$@"\n')
    _script(
        bin_dir / "apt-get",
        f'echo "frontend=${{DEBIAN_FRONTEND:-UNSET}} needrestart=${{NEEDRESTART_SUSPEND:-UNSET}} args=$*" >> "{seen}"\n',
    )
    _script(bin_dir / "dpkg-query", "exit 1\n")  # nothing is installed
    env = dict(os.environ, PATH=f"{bin_dir}:{os.environ['PATH']}")
    # Not root, and sudo is "passwordless": the helper picks the sudo prefix.
    script = (
        f". {LIB}\n"
        "_apt_ensure_sudo_prefix() { printf sudo; }\n"
        "apt_ensure some-package\n"
    )
    result = subprocess.run(["bash", "-c", script], capture_output=True, text=True, env=env)
    assert result.returncode == 0, result.stderr
    line = seen.read_text().strip()
    assert "frontend=noninteractive" in line, line
    assert "needrestart=1" in line, line
    assert "apt-get" not in line or "install -y some-package" in line


def test_the_old_spelling_would_have_failed_this_test(tmp_path):
    """The control: with the variable written BEFORE sudo, the same fake sudo
    drops it. Without this the test above could pass for any reason."""
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    seen = tmp_path / "seen.txt"
    _script(bin_dir / "sudo", 'exec env -i PATH="$PATH" "$@"\n')
    _script(bin_dir / "apt-get", f'echo "frontend=${{DEBIAN_FRONTEND:-UNSET}}" >> "{seen}"\n')
    env = dict(os.environ, PATH=f"{bin_dir}:{os.environ['PATH']}")
    subprocess.run(
        ["bash", "-c", "DEBIAN_FRONTEND=noninteractive sudo apt-get install -y x"],
        capture_output=True, text=True, env=env, check=True,
    )
    assert seen.read_text().strip() == "frontend=UNSET"
