"""Window totals from a streamed Tracy zone export."""

import csv
from pathlib import Path
import subprocess
import sys


SCRIPT = Path(__file__).resolve().parents[1] / "lib/tracy_zone_report.py"


def test_streamed_instances_keep_names_and_window_totals(tmp_path):
    instances = (
        "name,ns_since_start,exec_time_ns\n"
        "probe<Left,Right>,100000000,2000000\n"
        "probe<Left,Right>,800000000,3000000\n"
        "other,1200000000,5000000\n"
    )
    result = subprocess.run(
        [sys.executable, str(SCRIPT), str(tmp_path), "1", "--instances-stdin"],
        input=instances, text=True, capture_output=True,
    )
    assert result.returncode == 0, result.stderr
    with (tmp_path / "tracy_zone_windows.csv").open() as handle:
        rows = list(csv.DictReader(handle))
    assert len(rows) == 2
    assert rows[0]["zone"] == "probe<Left,Right>"
    assert float(rows[0]["window_start_s"]) == 0.0
    assert int(rows[0]["count"]) == 2
    assert float(rows[0]["total_ms"]) == 5.0
    assert float(rows[0]["mean_us"]) == 2500.0
    assert float(rows[0]["max_us"]) == 3000.0
    assert rows[1]["zone"] == "other"
    assert float(rows[1]["window_start_s"]) == 1.0
    assert int(rows[1]["count"]) == 1
    assert float(rows[1]["total_ms"]) == 5.0
