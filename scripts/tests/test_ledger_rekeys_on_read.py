"""A stored comparability key goes stale the moment the field set grows.

⛔⛔ **AND THE SPLIT IS INVISIBLE, BECAUSE BOTH GROUPS PRINT THE SAME LABEL.**
The key is a hash over whatever `COMPARABILITY_FIELDS` held on the day a row was
ingested. Add a field, and every row written before it hashes differently from
every row written after — while `comparable_label`, which is built from the
same fields, renders identically for both.

Measured 2026-09-01: two `hall_of_characters` captures whose `comparable_fields`
were byte-identical sat in two groups under one identical heading, because
`workload.brain_profile` was added between them. The series that was supposed to
show a 93% improvement showed two groups of one row each, and no comparison.

The query tool re-keys every row on read, so the field set is whatever the
CURRENT code says, uniformly.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts" / "lib"))


def record(record_id: str, mean: float, stored_key: str) -> dict:
    return {
        "record_id": record_id,
        "measured_at": f"2026-09-01T0{record_id[-1]}:00:00Z",
        "commit": "abc123",
        "scenario": {"id": "hall", "version": 1, "headless": True},
        "build": {"cargo_profile": "profiling", "features": []},
        "gpu": {"rendering": "headless"},
        "host": {"machine_id": "m1"},
        "frame_ms": {"mean": mean, "p99": mean * 2},
        "scene": {"bodies": 130.0},
        "sim_phases_ms": {"Decide": mean / 10.0},
        # The defect: identical facts, different keys frozen at ingest.
        "comparable_key": stored_key,
        "comparable_fields": {},
        "comparable_label": "stale",
    }


def ledger_with(rows: list[dict]) -> Path:
    handle = tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False)
    for row in rows:
        handle.write(json.dumps(row) + "\n")
    handle.close()
    return Path(handle.name)


def run(*args: str) -> str:
    proc = subprocess.run(
        [sys.executable, str(REPO / "scripts/perf_history.py"), *args],
        capture_output=True,
        text=True,
    )
    return proc.stdout + proc.stderr


def test_rows_with_identical_facts_group_together_despite_stale_keys():
    path = ledger_with(
        [
            record("run-1", 1.94, stored_key="OLD_FIELD_SET"),
            record("run-2", 1.56, stored_key="NEW_FIELD_SET"),
        ]
    )
    out = run("--ledger", str(path), "phase", "Decide")
    assert "run-1" in out and "run-2" in out, out
    assert "moved" in out, (
        "two rows with identical comparability facts must land in ONE group and "
        f"be compared; the stored keys disagreed and nothing compared them:\n{out}"
    )
    assert out.count("──") == 1, f"expected exactly one group, got:\n{out}"


def test_genuinely_different_experiments_still_refuse_to_group():
    """⛔ PREMISE GUARD. Re-keying must not collapse real differences — that
    would turn the refusal this ledger exists for into a silent average."""
    a = record("run-1", 1.94, stored_key="k")
    b = record("run-2", 1.56, stored_key="k")
    b["gpu"] = {"rendering": "hardware"}
    out = run("--ledger", str(ledger_with([a, b])), "phase", "Decide")
    assert out.count("──") == 2, (
        f"a headless row and a hardware row are different experiments:\n{out}"
    )
