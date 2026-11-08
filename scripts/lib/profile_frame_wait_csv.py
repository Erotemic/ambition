#!/usr/bin/env python3
"""Export opt-in windowed Smash frame-wait measurements from the game log."""

from __future__ import annotations

import csv
import re
import sys
from pathlib import Path

STAMP = re.compile(r"^\[\s*([0-9.]+)s\]\s*(.*)$")
FIELD = re.compile(r"([a-z_0-9]+)=([^\s]+)")
COLUMNS = (
    "capture_s", "t", "frames", "gaps", "main_wall_ms", "main_cpu_ms",
    "gap_wall_ms", "gap_cpu_ms", "gap_p95_ms", "gap_over_10ms", "cpu_clock",
)


def extract(source: Path, dest: Path) -> int:
    if not source.is_file():
        return 0
    rows = []
    for raw in source.read_text(errors="replace").splitlines():
        match = STAMP.match(raw)
        capture_s, line = (match.group(1), match.group(2)) if match else ("", raw)
        if not line.startswith("[frame-wait] "):
            continue
        fields = dict(FIELD.findall(line[len("[frame-wait] "):]))
        if not all(key in fields for key in COLUMNS if key != "capture_s"):
            continue
        rows.append({"capture_s": capture_s, **fields})
    if not rows:
        return 0
    with dest.open("w", newline="") as file:
        writer = csv.DictWriter(file, fieldnames=COLUMNS)
        writer.writeheader()
        writer.writerows(rows)
    return len(rows)


def main() -> None:
    out_dir = Path(sys.argv[1])
    total = extract(out_dir / "game-stderr-stamped.txt", out_dir / "frame_wait.csv")
    if total:
        print(f"frame_wait.csv: {total} one-second windows")


if __name__ == "__main__":
    main()
