"""A cue the renderer rendered as a seamless loop is packed with the LOOPING flag.

The runtime reads `ambition_sfx_bank::flag::LOOPING` to know a clip may repeat;
the only evidence the packer has is the `loop` block in the render manifest.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_sfx_pack"))

import pack  # noqa: E402


def rendered(root: Path, cue: str, loop) -> None:
    out = root / cue
    out.mkdir(parents=True)
    (out / f"{cue}.ogg").write_bytes(b"OggS fixture")
    (out / f"{cue}.render.json").write_text(json.dumps({
        "id": cue, "duration_seconds": 1.0, "peak_db": -10.0, "rms_db": -20.0, "channels": 2,
        "sample_rate": 48000, "resolved_format_policy": "auto:ogg",
        "outputs": {"ogg": str(out / f"{cue}.ogg")}, "loop": loop,
    }))


def test_a_loop_is_flagged_looping_and_a_one_shot_is_not(tmp_path):
    rendered(tmp_path, "fixture.hum", {"preroll_ms": 500.0, "crossfade_ms": 500.0, "postroll_ms": 100.0,
                                       "normalize_peak_db": -18.0})
    rendered(tmp_path, "fixture.open", None)
    flags = {entry.id: entry.flags for entry in pack.collect_entries(tmp_path)}
    assert flags["fixture.hum"] == pack.FLAG_LOOPING | pack.FLAG_STREAMABLE_HINT
    assert flags["fixture.open"] == pack.FLAG_STREAMABLE_HINT
