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
    assert "target_bindmount.sh --status" in done.stderr, (
        "the refusal must carry the remedy — this fires in the middle of "
        "someone else's work, and a bare 'not enough space' sends them looking "
        "at the wrong thing. AGENTS.md's remedy is the BIND CHECK: an enormous "
        "target/ is almost always an absent bind, and repairing it returns the "
        "space without deleting anything"
    )


def test_the_refusal_does_not_recommend_deleting_anything():
    """⛔⛔⛔ THIS TEST USED TO REQUIRE THE OPPOSITE.

    It asserted `"cargo clean" in done.stderr`, so the guard on this message was
    PINNING advice AGENTS.md forbids in the strongest terms it uses: *"NEVER
    `rm -rf` anything under a `target/`. NOT `incremental`, NOT `deps`, NOT AS A
    FAVOUR WHEN THE DISK IS FULL … the reclaim is Jon's call, on Jon's machine,
    and `cargo clean` is his to run."*

    ⚠ It was not theoretical. An agent pruning `target/debug/{deps,examples,
    incremental}` by mtime on 2026-09-03 was following exactly this chain of
    advice, with the bind mount present. A refusal message is read at the moment
    someone is under the most pressure to free space, which is why it is the
    last place a contradiction can be left standing.

    ⚠ AND IT FORBIDS RECOMMENDING, NOT MENTIONING -- the first version of this
    test failed on the message's own prohibition ("do not run `cargo clean`"),
    because a substring check cannot tell "do X" from "do not X". Naming the
    forbidden thing in order to forbid it is exactly what this message SHOULD
    do, so the rule is per line: any line naming one must also negate it.
    """
    done = _run("--min-gb", "999999")
    negations = ("do not", "don't", "never", "forbid", "jon's", "is his")
    for line in done.stderr.splitlines():
        lowered = line.lower()
        named = [f for f in ("cargo clean", "rm -rf") if f in lowered]
        if not named:
            continue
        assert any(n in lowered for n in negations), (
            f"this line RECOMMENDS {named!r}, which AGENTS.md forbids "
            f"(\"the reclaim is Jon's call ... `cargo clean` is his to run\"); "
            f"the remedy is the bind check, then reporting and stopping.\n"
            f"  line: {line.strip()}"
        )


def test_a_floor_of_zero_passes():
    done = _run("--min-gb", "0", "--quiet")
    assert done.returncode == 0, done.stderr
