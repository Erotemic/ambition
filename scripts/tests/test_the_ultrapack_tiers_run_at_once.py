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


def _run(
    tmp_path: Path, jobs: int, failing: str = "", inputs: str = "inputs-a"
) -> tuple[str, list[tuple[str, float, float]]]:
    trace = tmp_path / f"trace_{jobs}"
    trace.unlink(missing_ok=True)
    pack_root = tmp_path / "packs"
    script = f"""
set -euo pipefail
trace={trace}
run_renderer_python() {{
    local label="$1"
    local started; started="$(date +%s.%N)"
    local arg threads=""
    for arg in "$@"; do [ "${{previous-}}" != --jobs ] || threads="$arg"; previous="$arg"; done
    echo "$label $threads" >> "$trace.jobs"
    sleep 0.4
    echo "$label $started $(date +%s.%N)" >> "$trace"
    echo "packed $label"
    [ "$label" != "ultrapack-{failing}" ] || return 1
    local tier="${{label#ultrapack-}}"
    mkdir -p "$pack_root/$tier" && echo "{{}}" > "$pack_root/$tier/ultrapack.json"
}}
ultrapack_tiers=("full 1.0 1 2048" "half 0.5 1 1024" "quarter 0.25 1 512" "potato 0.0625 8 256")
ultrapack_plan=()
pack_root={pack_root}
ultrapack_key_dir={tmp_path / "keys"}
mkdir -p "$ultrapack_key_dir"
ultrapack_inputs={inputs}
pack_debug_root=/nowhere
sprites_dir=/nowhere
AMBITION_SPRITE_JOBS={jobs}
{_tier_loop()}
echo after
"""
    out = subprocess.run(["bash", "-c", script], capture_output=True, text=True, check=True).stdout
    spans = []
    for line in (trace.read_text().splitlines() if trace.exists() else []):
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


def test_a_tier_whose_inputs_did_not_change_is_not_packed_again(tmp_path: Path) -> None:
    """The key: a second run over the same inputs packs nothing and prints
    each tier as current; a changed input packs every tier again."""
    _, spans = _run(tmp_path, jobs=4)
    assert len(spans) == 4, spans
    out, spans = _run(tmp_path, jobs=4)
    assert spans == [], spans
    assert out.count(": current (no pooled sheet") == 4, out
    assert out.rstrip().endswith("after"), out
    _, spans = _run(tmp_path, jobs=4, inputs="inputs-b")
    assert len(spans) == 4, spans


def test_a_failed_tier_keeps_no_key_and_packs_again(tmp_path: Path) -> None:
    _run(tmp_path, jobs=4)
    _, spans = _run(tmp_path, jobs=4, failing="half", inputs="inputs-b")
    assert len(spans) == 4, spans
    assert not (tmp_path / "keys" / "half.key").exists()
    # Control: the tiers that ended well are current on the next run; the
    # failed one packs again.
    _, spans = _run(tmp_path, jobs=4, inputs="inputs-b")
    assert [label for label, _, _ in spans] == ["ultrapack-half"], spans


def _threads(tmp_path: Path, jobs: int) -> dict[str, int]:
    # A run of its own: the keys of an earlier run make every tier current.
    root = tmp_path / f"cap_{jobs}"
    root.mkdir()
    _run(root, jobs=jobs)
    lines = (root / f"trace_{jobs}.jobs").read_text().split("\n")
    return {label.removeprefix("ultrapack-"): int(n) for label, n in (line.split() for line in lines if line)}


def test_the_threads_of_the_tiers_are_within_the_job_cap(tmp_path: Path) -> None:
    """Each tier reads and encodes on threads (`--jobs`). At once, the four
    tiers' threads are at most the cap, and the two slowest get the most."""
    assert _threads(tmp_path, 6) == {"full": 2, "half": 2, "quarter": 1, "potato": 1}
    # Control: under a cap of 2, two tiers run at once, on one thread each.
    assert _threads(tmp_path, 2) == {"full": 1, "half": 1, "quarter": 1, "potato": 1}
