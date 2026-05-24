"""Smoke tests for `inspect_hall_sprites` — the diagnostic that
walks the embedded Hall of Characters and reports per-pedestal
render-readiness.

Pin two invariants:

  - The current Hall classifies its pedestals into one of four
    statuses (`ok` / `no_manifest` / `no_png` / `no_idle` /
    `no_catalog`); the tool returns 0 even when stragglers exist.
  - The Hall has ≥80 NpcSpawns that classify `ok` — i.e. most of
    the catalog renders. If a future commit drops below this floor
    something visible is breaking.
"""
from __future__ import annotations

import io
import sys
from contextlib import redirect_stdout
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.inspect_hall_sprites import main as inspect_main  # noqa: E402


def test_inspect_hall_sprites_reports_high_coverage():
    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = inspect_main([])
    output = buf.getvalue()
    assert rc == 0, f"inspect_hall_sprites exited non-zero; output:\n{output}"
    # Header line names the count of pedestals.
    assert "NpcSpawn pedestals" in output
    # Summary section names the counts. Pin that ≥80 entries classify
    # as `ok`. (Today the number is 96; floor at 80 leaves slack for
    # legitimate temporary regressions like a one-off broken
    # publisher without failing CI.)
    summary = output.split("# summary:")[-1]
    assert "ok:" in summary, summary
    # Extract the integer after `ok:`.
    for line in summary.splitlines():
        stripped = line.strip()
        if stripped.startswith("ok:"):
            count = int(stripped.removeprefix("ok:").strip())
            assert count >= 80, f"Hall ok-count dropped to {count} — sprite chain regression?"
            break
    else:  # pragma: no cover - sanity for unexpected output shape
        raise AssertionError(f"no `ok:` line in summary:\n{summary}")


def test_inspect_hall_sprites_only_issues_filters_out_ok_rows():
    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = inspect_main(["--only-issues"])
    output = buf.getvalue()
    assert rc == 0
    # `--only-issues` mode: no `[ok]` rows should appear in the body,
    # only `[no_*]` rows and the summary.
    body_lines = [
        line for line in output.splitlines()
        if line.strip().startswith("[")
    ]
    for line in body_lines:
        assert not line.lstrip().startswith("[ok"), \
            f"--only-issues leaked an ok row: {line!r}"
