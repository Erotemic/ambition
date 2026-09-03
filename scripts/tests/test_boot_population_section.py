"""The capture summary's boot-population section, and the two ways it could lie.

The hall reveal is fixed and the same host run says the remaining hitch is
STARTUP, so a walk should answer "what decoded before the first room?" without a
second command. This section does that inside the bundle summary.

⛔⛔ IT DELEGATES TO `scripts/measure_first_room_manifest.py` ON PURPOSE. Two
implementations of one question eventually print two answers — that already
happened tonight with the orphan census — and the parser being reused carries
three things a fresh copy gets wrong: ordering by FRAME rather than clock, the
`>= 1.0 MP` print threshold, and both stamped/unstamped line shapes.

⭐ ORDERING IS THE ONE THAT BIT. The census runs in `Last`, so its clock can read
AFTER a `room-loaded` that the same frame's `PreUpdate` insertion preceded.
Ordering by time put a 7.6 MP decode in the wrong bucket and turned 7 images /
23.7 MP into 6 / 16.1.
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
MODULE = REPO / "scripts/lib/profile_bundle_summary.py"


def load():
    spec = importlib.util.spec_from_file_location("pbs", MODULE)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


# The two orderings DISAGREE here on purpose: by clock the second decode looks
# like it came after the room loaded; by frame it came two frames before.
DISAGREEING = """\
[image]    1.021s f      0 3072x2468    7.6MP live=0 game://sprites/player.png demand=unknown (not through load_sheet_image)
[image]    3.924s f   1129 3072x2468    7.6MP live=0 sprites/player.png demand→insert 9ms via character-sheet
[world-event]    3.863s f   1131 room-loaded central_hub_complex
[image-census]   10.002s +130 images (+36.2MP) | total 252 images, 78.3MP, 313.1MB resident | gpu +130
"""


def test_it_orders_by_frame_so_a_late_clock_does_not_move_a_decode(capsys):
    module = load()
    text = "\n".join(module.boot_population_lines(DISAGREEING))
    assert "ordered by: frame" in text
    assert "2 image(s), 15.2 MP" in text, (
        "both decodes precede room-loaded BY FRAME; ordering by clock drops the "
        f"second and reports one. got:\n{text}"
    )


def test_the_sample_bound_travels_with_the_count():
    """⛔ `[image]` prints only decodes >= 1.0 MP. Without the denominator a
    reader takes '7 images' for the whole boot; on the real bundle it is 7 of
    252."""
    module = load()
    text = "\n".join(module.boot_population_lines(DISAGREEING))
    assert "SAMPLE, NOT POPULATION" in text
    assert "252 images" in text
    # ⚠ anchor on the COUNT line, not on the word "before" — the section title
    # is "what decoded before the first room", so a naive `index("before ")`
    # matches the heading and the assertion passes for the wrong reason. My
    # first draft did exactly that and failed against correct output.
    assert text.index("SAMPLE, NOT POPULATION") < text.index("before central_hub"), (
        "the caveat must precede the number it bounds"
    )


def test_a_log_with_no_room_loaded_emits_NO_SECTION_rather_than_a_zero():
    """⛔ Without the boundary every image is 'before the first room', which is
    true and useless. An absent section beats a confident wrong one."""
    module = load()
    no_boundary = "\n".join(
        line for line in DISAGREEING.splitlines() if "room-loaded" not in line
    )
    assert module.boot_population_lines(no_boundary) == []


def test_a_log_with_no_image_lines_emits_no_section():
    module = load()
    assert module.boot_population_lines("[world-event] 1.0s f 5 room-loaded hub") == []


def test_it_reproduces_the_host_bundle_figures_if_that_bundle_is_present():
    """⭐ POSITIVE CONTROL against the capture the campaign was measured from.
    Skips where the bundle is not checked out."""
    module = load()
    rel = (
        "dev/ambition_dev_measurements/profiles/"
        "desktop-timeline-run-20260902T215256Z/game-stderr-stamped.txt"
    )
    bundle = next(
        (c for c in (REPO / rel, REPO.parent.parent / rel) if c.is_file()), REPO / rel
    )
    if not bundle.is_file():
        pytest.skip("the 20260902T215256Z bundle is not present")
    text = "\n".join(module.boot_population_lines(bundle.read_text(errors="ignore")))
    assert "7 image(s), 23.7 MP" in text, text
    assert "character-sheet" in text and "8.9 MP" in text


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
