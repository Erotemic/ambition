"""Frame-boundary records survive the stamped game-log exporter."""

import csv
import tempfile
import unittest
from pathlib import Path

from scripts.lib.profile_frame_wait_csv import extract
from scripts.lib.profile_bundle_summary import build_summary, Bundle


class FrameWaitCsvTests(unittest.TestCase):
    def test_stamped_rows_and_missing_rows(self):
        with tempfile.TemporaryDirectory() as temp:
            source = Path(temp) / "game-stderr-stamped.txt"
            dest = Path(temp) / "frame_wait.csv"
            self.assertEqual(extract(source, dest), 0)
            source.write_text(
                "[ 12.500s] [frame-wait] t=1.001 frames=61 gaps=60 "
                "main_wall_ms=3.1 main_cpu_ms=2.5 gap_wall_ms=13.1 "
                "gap_cpu_ms=0.1 gap_p95_ms=15.0 gap_over_10ms=57 cpu_clock=thread\n"
                "[ 13.500s] [frame-wait] incomplete\n"
                "[census] phases t=1 frames=61 outside=13\n"
            )
            self.assertEqual(extract(source, dest), 1)
            with dest.open() as file:
                rows = list(csv.DictReader(file))
            self.assertEqual(len(rows), 1)
            self.assertEqual(rows[0]["capture_s"], "12.500")
            self.assertEqual(rows[0]["gap_wall_ms"], "13.1")
            self.assertEqual(rows[0]["cpu_clock"], "thread")
            self.assertEqual(rows[0]["focused_frames"], "")

    def test_focus_count_and_gap_classification(self):
        with tempfile.TemporaryDirectory() as temp:
            source = Path(temp) / "game-stderr-stamped.txt"
            dest = Path(temp) / "frame_wait.csv"
            source.write_text(
                "[ 22.000s] [frame-wait] t=10 frames=90 gaps=90 "
                "main_wall_ms=3.2 main_cpu_ms=3.1 gap_wall_ms=7.9 "
                "gap_cpu_ms=0.1 gap_p95_ms=15.5 gap_over_10ms=40 "
                "cpu_clock=thread focused_frames=40 unfocused_frames=50 "
                "unknown_focus_frames=0 focus_transitions=2 focused_gaps=38 "
                "unfocused_gaps=52 unknown_focus_gaps=0 "
                "focused_gap_ms=0.7 unfocused_gap_ms=13.2\n"
            )
            self.assertEqual(extract(source, dest), 1)
            with dest.open() as file:
                row = next(csv.DictReader(file))
            self.assertEqual(row["unfocused_frames"], "50")
            self.assertEqual(row["focus_transitions"], "2")
            self.assertEqual(row["focused_gap_ms"], "0.7")
            self.assertEqual(row["unfocused_gap_ms"], "13.2")
            summary = build_summary(Bundle(temp))
            self.assertIn("Window focus during measured gameplay", summary)
            self.assertIn("Background-window samples present", summary)

    def test_clean_focus_summary(self):
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp)
            (out / "frame_wait.csv").write_text(
                "focused_frames,unfocused_frames,unknown_focus_frames,focus_transitions\n"
                "261,0,0,0\n"
            )
            summary = build_summary(Bundle(temp))
            self.assertIn("All sampled gameplay frames were focused", summary)


if __name__ == "__main__":
    unittest.main()
