"""A reduced-tier portrait must be bigger than the box it is drawn into.

The generator emits `*_portraits.png` at four tiers; only full resolution is
bakeable. Before deciding whether to stop generating them or start baking them,
the question is whether a cheaper portrait is USEFUL — and the answer turns on a
fact about the UI, not about the art.

⭐ PORTRAIT DRAW SIZE IS CHOSEN BY VIEWPORT, NEVER BY QUALITY TIER.
`DialogLayoutProfile::for_viewport` picks 56x62, 82x94 or 104x120 from the
window size; no quality setting is consulted. So no quality tier can select a
portrait resolution, and a tier whose frame is under the drawn box is not a
cheaper portrait but a blurrier one.

⛔ THE FAILURE MODE THIS GUARDS: a parser that finds NO draw sizes reports
infinite headroom — "every tier is big enough" — which is the most reassuring
possible wrong answer, and would be produced by a stale regex rather than by
the tree.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/measure_portrait_tier_headroom.py"


def load():
    spec = importlib.util.spec_from_file_location("portrait_headroom", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_the_draw_sizes_still_parse_from_the_layout():
    """⭐ POSITIVE CONTROL against committed Rust. Runs on every machine — the
    dialog layout is source, not generated art."""
    module = load()
    if not module.DIALOG.exists():
        pytest.skip("the dialog presentation module is absent")
    boxes = module.draw_sizes()
    assert len(boxes) >= 3, (
        f"expected the three DialogLayoutProfile portrait boxes, parsed {boxes}. "
        "A stale pattern reports infinite headroom, which reads as 'every tier "
        "is big enough'."
    )
    assert (104.0, 120.0) in boxes, (
        "the desktop profile's 104x120 box is the largest and sets the bar"
    )


def test_the_script_refuses_when_no_draw_size_parses(tmp_path, monkeypatch, capsys):
    """⛔ THE POISON: an empty set of boxes must refuse, not report headroom."""
    module = load()
    empty = tmp_path / "dialog.rs"
    empty.write_text("fn for_viewport() {}")
    monkeypatch.setattr(module, "DIALOG", empty)
    code = module.main([])
    out = capsys.readouterr().out
    assert code == 2
    assert "NO PORTRAIT DRAW SIZES PARSED" in out
    assert "most reassuring possible wrong answer" in out


def test_a_frame_under_the_smallest_box_upscales_at_every_scale():
    """The verdict that decides the question: `sprites_potato` is 16x20 for a
    portrait drawn no smaller than 56x62."""
    module = load()
    if not module.DIALOG.exists():
        pytest.skip("the dialog presentation module is absent")
    boxes = module.draw_sizes()
    smallest = min(boxes, key=lambda b: b[0] * b[1])
    assert 16 < smallest[0] and 20 < smallest[1], (
        "premise: the potato frame (16x20 for alice) is under even the smallest "
        f"drawn box {smallest}, so it can only ever be upscaled"
    )


def test_frames_scale_with_the_png_and_absence_is_reported(monkeypatch):
    """A reduced tier has no manifest — that is the finding — so its frame is
    derived from the PNG ratio. A tier with no PNG must be reported absent, not
    silently given the full-resolution frame."""
    module = load()
    if not (module.ASSETS / "sprites").is_dir():
        pytest.skip("the sprite tree is gitignored generated output; absent here")
    frames = module.portrait_frames("alice_portraits")
    if not frames:
        pytest.skip("alice_portraits is not in this tree")
    assert "sprites" in frames
    full = frames["sprites"]
    for tier, frame in frames.items():
        if tier == "sprites":
            continue
        assert frame[0] < full[0] and frame[1] < full[1], (
            f"{tier} claims a frame of {frame} against a full frame of {full}; a "
            "reduced tier that is not reduced would make this whole question moot"
        )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
