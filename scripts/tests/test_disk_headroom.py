"""The disk-headroom guard, and the one thing it must not get wrong.

⛔ **it must measure the volume CARGO writes to, not the repo's.** The first
version of this check read the repo's filesystem; on this checkout the repo sits
on a 1.8 TB disk while `.cargo/config.toml` points `target-dir` at a 387 GB one.
It reported 380 GB free while the volume that actually fills had two — green, and
answering a question nobody asked. That is pinned first, because a headroom guard
that reads the wrong device is worse than none: it is a reason not to look.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import check_disk_headroom as guard  # noqa: E402


def test_it_reads_the_target_dir_from_cargo_config():
    """The configured `target-dir` wins over the repo's own `target/`."""
    config = REPO / ".cargo" / "config.toml"
    configured = None
    for line in config.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("target-dir"):
            configured = line.partition("=")[2].strip().strip('"').strip("'")
    if not configured:
        # A checkout that does not redirect: the fallback is the repo's target.
        assert guard.target_dir() == REPO / "target"
        return
    assert guard.target_dir() == Path(configured), (
        "the guard is measuring a different filesystem than cargo writes to — "
        "the exact defect that made the first version report 380 GB free while "
        "the volume cargo fills had 2"
    )


def test_cargo_target_dir_env_overrides_the_config(monkeypatch, tmp_path):
    monkeypatch.setenv("CARGO_TARGET_DIR", str(tmp_path))
    assert guard.target_dir() == tmp_path


def test_free_space_is_a_positive_number_of_gb():
    free = guard.free_gb_on_target()
    assert free >= 0.0 and free < 1_000_000.0


def _run(*args) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(REPO / "scripts" / "check_disk_headroom.py"), *args],
        capture_output=True,
        text=True,
    )


def test_an_impossible_floor_refuses():
    """The RED direction, run for real rather than reasoned about."""
    done = _run("--min-gb", "999999")
    assert done.returncode == 1
    assert "REFUSING" in done.stderr
    assert "cargo clean" in done.stderr, (
        "the refusal must carry the remedy — this fires in the middle of "
        "someone else's work, and a bare 'not enough space' sends them looking "
        "at the wrong thing"
    )


def test_a_floor_of_zero_passes():
    done = _run("--min-gb", "0", "--quiet")
    assert done.returncode == 0, done.stderr


def test_run_tests_uses_this_guard_rather_than_its_own_copy():
    """⭐ the reason this file exists at all.

    The refusal in `run_tests.py` worked; the disk still filled a third time,
    because a bare `cargo test --workspace` never reaches it. Keeping ONE
    implementation is what makes the check available to anything that wants it.
    """
    text = (REPO / "scripts" / "run_tests.py").read_text(encoding="utf-8")
    assert "from check_disk_headroom import" in text
    assert "shutil.disk_usage" not in text, (
        "run_tests.py has grown its own copy again; two copies drift and the "
        "second one is the one nobody updates"
    )
