#!/usr/bin/env python3
"""Identify the live Smash interval for automatically filtered native perf reports.

The performance pass has no Tracy client. The driver logs LIVE, early-results,
and normal completion markers; logs are stamped relative to game launch. perf
starts immediately before game launch, so the percentage filter is approximate
by the perf startup offset (normally small). Both original and filtered native
reports remain in the bundle for review.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

STAMP = re.compile(r"^\[\s*(\d+(?:\.\d+)?)s\]\s+(.*)$")


def match_window(log: str) -> dict:
    live = None
    ended = None
    done = None
    last = 0.0
    for line in log.splitlines():
        found = STAMP.match(line)
        if not found:
            continue
        seconds = float(found.group(1))
        message = found.group(2)
        last = max(last, seconds)
        if "[smash-profile] live after" in message and live is None:
            live = seconds
        if "[smash-profile] WARNING: the match ended" in message and ended is None:
            ended = seconds
        if "[smash-profile] done seats_at_end=" in message:
            done = seconds
    if live is None or last <= 0:
        return {
            "status": "missing-live-marker",
            "note": "No usable live match marker; only whole-run native reports available.",
        }
    stop = ended if ended is not None else done
    if stop is None or stop <= live or last <= live:
        return {
            "status": "missing-match-end-marker",
            "live_start_s": live,
            "note": "No reliable endpoint; only whole-run native reports available.",
        }
    start_pct = max(0.0, min(99.0, live / last * 100))
    end_pct = min(100.0, stop / last * 100)
    if end_pct - start_pct < 0.5:
        return {"status": "interval-too-short", "live_start_s": live,
                "match_end_s": stop, "log_duration_s": last}
    return {
        "status": "ok",
        "live_start_s": live,
        "match_end_s": stop,
        "log_duration_s": last,
        "active_seconds": round(stop - live, 3),
        "match_ended_early": ended is not None,
        "end_marker": "early-results" if ended is not None else "normal-completion",
        "perf_time_percent": f"{start_pct:.3f}%-{end_pct:.3f}%",
        "note": "Approximate perf time percentages from game-log-relative timestamps; "
                "perf startup and teardown can shift the interval slightly.",
    }


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: profile_smash_match_window.py <bundle-dir>", file=sys.stderr)
        return 2
    directory = Path(sys.argv[1])
    log_path = directory / "game-stderr-stamped.txt"
    result = match_window(log_path.read_text(errors="replace") if log_path.exists() else "")
    (directory / "match-window.json").write_text(json.dumps(result, indent=2) + "\n")
    print(f"[smash-match-window] {result['status']}: {result.get('perf_time_percent', '<none>')}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
