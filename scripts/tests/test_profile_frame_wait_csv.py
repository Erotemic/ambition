"""Frame-boundary records survive the stamped game-log exporter."""

import csv
import tempfile
import unittest
from pathlib import Path

from scripts.lib.profile_frame_wait_csv import extract


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


if __name__ == "__main__":
    unittest.main()
