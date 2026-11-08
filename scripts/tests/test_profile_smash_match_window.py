"""Live match samples exclude launch and results-screen time."""
from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/lib/profile_smash_match_window.py"
spec = importlib.util.spec_from_file_location("profile_smash_match_window", SCRIPT)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class SmashWindowTests(unittest.TestCase):
    def test_early_match_end_excludes_results(self):
        log = "\n".join([
            "[    0.010s] [smash-profile] windowed: fighters=4",
            "[    6.012s] [smash-profile] live after 34 frames; fighters=4",
            "[   84.049s] [smash-profile] WARNING: the match ended — results",
            "[   96.016s] [smash-profile] done seats_at_end=0",
        ])
        result = module.match_window(log)
        self.assertEqual(result["status"], "ok")
        self.assertTrue(result["match_ended_early"])
        self.assertAlmostEqual(result["active_seconds"], 78.037)
        self.assertAlmostEqual(result["match_end_s"], 84.049)
        self.assertEqual(result["perf_time_percent"], "6.261%-87.536%")

    def test_normal_completion_is_the_end_marker(self):
        log = "\n".join([
            "[ 2.000s] [smash-profile] live after 40 frames; fighters=4",
            "[32.000s] [smash-profile] done seats_at_end=4",
        ])
        result = module.match_window(log)
        self.assertEqual(result["status"], "ok")
        self.assertFalse(result["match_ended_early"])
        self.assertEqual(result["perf_time_percent"], "6.250%-100.000%")

    def test_missing_live_marker_is_not_a_valid_filter(self):
        result = module.match_window("[10.000s] [smash-profile] done seats_at_end=0")
        self.assertEqual(result["status"], "missing-live-marker")
        self.assertNotIn("perf_time_percent", result)

    def test_cli_writes_machine_readable_window(self):
        with tempfile.TemporaryDirectory() as folder:
            directory = Path(folder)
            (directory / "game-stderr-stamped.txt").write_text(
                "[1.000s] [smash-profile] live after 40 frames; fighters=4\n"
                "[4.000s] [smash-profile] done seats_at_end=4\n"
            )
            result = subprocess.run([sys.executable, str(SCRIPT), str(directory)],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            data = json.loads((directory / "match-window.json").read_text())
            self.assertEqual(data["perf_time_percent"], "25.000%-100.000%")


if __name__ == "__main__":
    unittest.main()
