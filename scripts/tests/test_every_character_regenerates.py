"""Check authored assets against the regeneration plans."""

from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]


def test_authored_assets_have_regeneration_paths():
    result = subprocess.run(
        [sys.executable, str(ROOT / "scripts/check_regeneration_coverage.py"), "--plans-only"],
        cwd=ROOT, capture_output=True, text=True,
    )
    assert result.returncode == 0, result.stdout + result.stderr
