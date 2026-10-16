"""The iteration recorder times an edit only to the acknowledgment of the
generation that edit made (fast-iteration I0). A host that loads nothing, or
answers with an old generation, gets the run refused, not a faster row."""

from __future__ import annotations

import json
import sys
import textwrap
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import iteration_record  # noqa: E402

# A host that watches one file and names each generation by the file's bytes,
# as `content::live_reload` names it by the pack's fingerprint. `mode` is the
# defect under test.
FAKE_HOST = textwrap.dedent(
    """
    import hashlib, os, sys, time
    path, mode = sys.argv[1], sys.argv[2]
    def generation(data):
        return hashlib.sha256(data).hexdigest()[:16]
    def ack(event, gen, extra=""):
        print(f"[live-reload] event={event} pack=probe generation={gen}{extra}", flush=True)
    running = generation(open(path, "rb").read())
    seen = os.stat(path).st_mtime
    ack("watching", running)
    while True:
        time.sleep(0.02)
        now = os.stat(path).st_mtime
        if now == seen:
            continue
        seen = now
        candidate = generation(open(path, "rb").read())
        if mode == "silent":
            continue
        if mode == "refuse" and candidate != running:
            ack("refused", running, " prepare_ms=1.0")
            continue
        if mode == "stale" and candidate != running:
            ack("staged", running, " prepare_ms=1.0")
            ack("published", running)
            continue
        if candidate == running and mode != "noise" or mode in ("stale", "refuse"):
            continue
        if mode == "noise":
            candidate = generation(os.urandom(8))
        ack("staged", candidate, " prepare_ms=1.5")
        ack("published", candidate)
        running = candidate
    """
)


def record(tmp_path: Path, mode: str, samples: int = 2) -> tuple[int, Path]:
    host = tmp_path / "host.py"
    host.write_text(FAKE_HOST)
    data = tmp_path / "rules.ron"
    data.write_text("(speed: 10)\n")
    out = tmp_path / "out"
    code = iteration_record.main([
        "--host", sys.executable, "--host-arg", str(host), "--host-arg", str(data), "--host-arg", mode,
        "--pack", "probe", "--file", str(data), "--old", "10", "--new", "11",
        "--samples", str(samples), "--timeout", "3", "--start-timeout", "10", "--quiet", "0.5",
        "--out", str(out),
    ])
    assert data.read_text() == "(speed: 10)\n", "the recorder must put the file back"
    return code, out


def test_a_host_that_plays_each_edit_gives_a_row_for_each_edit_and_revert(tmp_path: Path) -> None:
    code, out = record(tmp_path, "good")
    assert code == 0
    rows = [json.loads(line) for line in (out / "records.jsonl").read_text().splitlines()]
    assert [r["sample_kind"] for r in rows] == ["edit", "revert", "edit", "revert"]
    edit, revert = rows[0], rows[1]
    assert edit["admitted_generation_digest"] != revert["admitted_generation_digest"]
    assert edit["changed_input_digest"] == iteration_record.digest(b"(speed: 11)\n")
    assert edit["prepare_seconds"] == pytest.approx(0.0015)
    assert edit["activate_seconds"] > 0
    assert edit["first_observed_result_seconds"] is None, "unmeasured is null, not zero"


@pytest.mark.parametrize(
    ("mode", "why"),
    [
        ("silent", "did not load the edit"),
        ("stale", "the running generation"),
        ("refuse", "refused the edit"),
        ("noise", "a save with no change"),
    ],
)
def test_a_host_that_does_not_play_the_edit_gets_the_run_refused(
    tmp_path: Path, capsys: pytest.CaptureFixture[str], mode: str, why: str
) -> None:
    code, out = record(tmp_path, mode, samples=1)
    assert code == 1
    assert why in capsys.readouterr().err
    assert not (out / "records.jsonl").exists(), "a refused run writes no rows"
