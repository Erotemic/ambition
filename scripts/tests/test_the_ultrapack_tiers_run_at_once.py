"""The four ultrapack tiers of a sprite publish run at once, under the job cap.

They read the same published sheets and write separate roots. One after
another they cost 362 s, 61% of a publish that rendered one target
(measured 2026-10-15); at once they cost the slowest tier. The test runs
the script's own tier loop with a renderer stub that records when each tier
starts and ends.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
REGEN = REPO / "scripts/regen/sprites.sh"


def _tier_loop() -> str:
    text = REGEN.read_text()
    start = text.index('    ultrapack_logs="$(mktemp -d)"')
    end = text.index('    rm -rf "$ultrapack_logs"\n', start) + len('    rm -rf "$ultrapack_logs"\n')
    return text[start:end]


def _run(tmp_path: Path, jobs: int, failing: str = "") -> tuple[str, list[tuple[str, float, float]]]:
    trace = tmp_path / f"trace_{jobs}"
    script = f"""
set -euo pipefail
trace={trace}
run_renderer_python() {{
    local label="$1"
    local started; started="$(date +%s.%N)"
    sleep 0.4
    echo "$label $started $(date +%s.%N)" >> "$trace"
    echo "packed $label"
    [ "$label" != "ultrapack-{failing}" ]
}}
ultrapack_tiers=("full 1.0 1 2048" "half 0.5 1 1024" "quarter 0.25 1 512" "potato 0.0625 8 256")
ultrapack_plan=()
pack_root=/nowhere
pack_debug_root=/nowhere
sprites_dir=/nowhere
AMBITION_SPRITE_JOBS={jobs}
{_tier_loop()}
echo after
"""
    out = subprocess.run(["bash", "-c", script], capture_output=True, text=True, check=True).stdout
    spans = []
    for line in trace.read_text().splitlines():
        label, started, ended = line.split()
        spans.append((label, float(started), float(ended)))
    return out, spans


def _most_at_once(spans: list[tuple[str, float, float]]) -> int:
    return max(sum(1 for _, s, e in spans if s <= at < e) for _, at, _ in spans)


def test_every_tier_runs_at_once_and_prints_in_tier_order(tmp_path: Path) -> None:
    out, spans = _run(tmp_path, jobs=4)
    assert _most_at_once(spans) == 4, spans
    printed = [line.strip() for line in out.splitlines() if line.strip().startswith("packed")]
    assert printed == [f"packed ultrapack-{t}" for t in ("full", "half", "quarter", "potato")], out
    assert out.rstrip().endswith("after"), out


def test_the_job_cap_bounds_the_tiers(tmp_path: Path) -> None:
    _, spans = _run(tmp_path, jobs=2)
    assert len(spans) == 4 and _most_at_once(spans) == 2, spans


def test_a_failed_tier_is_named_and_the_others_still_run(tmp_path: Path) -> None:
    out, spans = _run(tmp_path, jobs=4, failing="half")
    assert len(spans) == 4, spans
    assert "WARN: ultrapack tier 'half' failed (non-fatal)" in out, out
