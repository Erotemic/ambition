"""The output digests that key a published sheet's freshness.

`sheet_output_digests` in `scripts/regen/sprites.sh` gives `<sha256> <path>`
for each declared file on disk, sorted, and fails when none is on disk. The
per-sheet cache stores these lines and compares them on the next publish, so
the format is a contract with every stored entry: a change to it makes every
sheet stale (a republish of about one hour). The test runs the script's own
function and compares it with digests made here.
"""

from __future__ import annotations

import hashlib
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
REGEN = REPO / "scripts/regen/sprites.sh"


def _function() -> str:
    text = REGEN.read_text()
    start = text.index("sheet_output_digests() {")
    end = text.index("\n}\n", start) + 3
    return text[start:end]


def _digests(sprites: Path, records: str) -> subprocess.CompletedProcess[str]:
    script = f"""
set -euo pipefail
sprites_dir={sprites}
{_function()}
sheet_output_digests "$1"
"""
    return subprocess.run(
        ["bash", "-c", script, "digests", records],
        capture_output=True,
        text=True,
        env={"PATH": "/usr/bin:/bin", "LC_ALL": "C"},
    )


def test_each_declared_file_on_disk_is_one_sorted_digest_line(tmp_path: Path) -> None:
    sprites = tmp_path / "sprites"
    (sprites / "gnu_ton_boss").mkdir(parents=True)
    files = {
        "alice_spritesheet.png": b"alice",
        "alice_actor.ron": b"(actor)",
        "gnu_ton_boss/gnu_ton_apple.png": b"apple",
    }
    for rel, data in files.items():
        (sprites / rel).write_bytes(data)
    # A declared file that is not on disk is left out, as is an empty line.
    records = "\n".join([*files, "alice_portrait.png", ""]) + "\n"

    done = _digests(sprites, records)
    assert done.returncode == 0, done.stderr
    expected = sorted(f"{hashlib.sha256(data).hexdigest()} {rel}" for rel, data in files.items())
    assert done.stdout.splitlines() == expected


def test_a_target_with_no_declared_file_on_disk_has_no_digests(tmp_path: Path) -> None:
    sprites = tmp_path / "sprites"
    sprites.mkdir()
    assert _digests(sprites, "missing.png\n").returncode == 1
    assert _digests(sprites, "").returncode == 1
