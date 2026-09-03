"""The first-room census must refuse an empty log rather than report zero.

`measure_first_room_manifest.py` answers "what decodes at boot that no
first-room cover waited for?" from a run's own stderr. Two of its failure modes
would be published as findings if it did not refuse them:

  * a log with no `[image]` lines — a run without `AMBITION_PROFILE_CENSUS=1`
    reads as "nothing decoded at boot", which is a spectacular false all-clear;
  * a log with no `room-loaded` — every image then counts as "before the first
    room", which is true and useless.

⛔ AND `coverable` IS AN UPPER BOUND. It means the image arrived on a road the
manifest vocabulary speaks, not that a manifest would resolve it. An image with
NO demand stamp cannot be classified either way and goes in its own bucket —
absence of a stamp is absence of evidence, not evidence of uncoverable.
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
SCRIPT = REPO / "scripts/measure_first_room_manifest.py"


def load():
    spec = importlib.util.spec_from_file_location("first_room", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


# ⛔⛔ THESE ARE THE REAL LINE SHAPES, taken from a captured boot
# (`headless-room-frame-20260902T135533Z`) rather than from reading the emitter.
# My first fixture was written from the source and got two things wrong: it
# required the `f{frame}` column, which older captures do not have, and it took
# the road as the first token after the path when the real phrase is
# `demand→insert 232ms via fx-sheet`. The parser matched NOTHING on 8 real
# lines. Both shapes are pinned here so a fixture built from source cannot
# quietly replace one built from output.
BOOT = """\
[image]    0.500s 512x512    0.3MP live=1 sprites/hud/icon.png demand=unknown (not through load_sheet_image)
[image]    0.900s 2048x2048    4.2MP live=0 sprites/alice_spritesheet.png demand→insert 232ms via character-sheet
[    0.3s] [image]    1.100s f     20 4096x2048    8.4MP live=0 sprites/sky_parallax.png first demanded via parallax
[image]    1.300s 1024x1024    1.0MP live=1 <runtime-generated> — allocated during gameplay. No asset path, so this is generated (an atlas or a render target), not content that could have been demanded earlier.
[    0.5s] [first-room-art] room 'intro_lab' ready after 42 updates (7 of them waiting only on GPU uploads): 9 assets, 3 characters
[world-event]    1.500s f    745 room-loaded intro_lab
[image]    2.000s 2048x2048    4.2MP live=1 sprites/bob_spritesheet.png demand→insert 12ms via character-sheet
"""


def test_it_reads_the_three_line_shapes():
    parsed = load().parse(BOOT.splitlines())
    assert len(parsed["images"]) == 5, "every [image] line, before and after"
    assert parsed["room_loaded"][2] == "intro_lab"
    assert parsed["first_room_art"]["assets"] == 9
    assert parsed["first_room_art"]["characters"] == 3


def test_an_unstamped_line_still_parses_with_no_road():
    """⛔ `<runtime-generated>` has no demand token at all. Dropping the line
    would understate the boot; guessing a road would invent one."""
    images = load().parse(BOOT.splitlines())["images"]
    generated = [i for i in images if i["path"] == "<runtime-generated>"]
    assert len(generated) == 1
    assert generated[0]["road"] == "", "no stamp means no road, not a default"
    unknown = [i for i in images if i["road"] == "unknown"]
    assert unknown and "not through load_sheet_image" in unknown[0]["raw_demand"]


def test_both_line_shapes_parse_with_and_without_the_frame_column():
    """⛔ A captured boot may predate the `f{frame}` column, and profile_desktop
    stamps a `[   N.NNNs]` prefix that a raw `2>` capture lacks. The fixture
    mixes all four combinations on purpose: a parser that handles only the
    current emitter reads nothing from the measurements already on disk."""
    module = load()
    images = module.parse(BOOT.splitlines())["images"]
    assert len(images) == 5
    assert any(i["frame"] is None for i in images), "an unframed line still parses"
    assert any(i["frame"] == 20 for i in images), "and a framed one keeps its frame"


def test_it_refuses_a_log_with_no_image_lines(capsys):
    module = load()
    code = module.report({"images": [], "room_loaded": None, "first_room_art": None})
    out = capsys.readouterr().out
    assert code == 2, "a run without the census must not exit 0"
    assert "NO `[image]` LINES" in out
    assert "Absent is not zero" in out, (
        "the message must say why silence is not an all-clear — a reader who "
        "sees '0 MP decoded at boot' will believe it"
    )


def test_it_refuses_a_log_with_no_room_loaded(capsys):
    module = load()
    parsed = module.parse(
        [line for line in BOOT.splitlines() if "room-loaded" not in line]
    )
    code = module.report(parsed)
    out = capsys.readouterr().out
    assert code == 2, "without the boundary there is nothing to measure against"
    assert "NO `room-loaded`" in out


def test_coverable_counts_only_content_roads_and_never_the_unstamped(capsys):
    module = load()
    module.report(module.parse(BOOT.splitlines()))
    out = capsys.readouterr().out
    # parallax 8.4 + character-sheet 4.2, and NOT the 1.0 generated or 0.3 unknown
    assert "2 image(s) / 12.6 MP" in out, out
    assert "2 image(s) / 1.3 MP carry NO demand stamp" in out, out
    assert "UPPER BOUND" in out, (
        "the bound must travel with the number, or it gets quoted as a plan"
    )


def test_the_after_bucket_is_not_counted_as_boot_work(capsys):
    """The image at 2.000s is AFTER room-loaded at 1.500s. Counting it would
    inflate 'what the cover missed' with work the cover was never for."""
    module = load()
    module.report(module.parse(BOOT.splitlines()))
    out = capsys.readouterr().out
    assert "decoded BEFORE room-loaded: 4 images, 13.9 MP" in out, out
    assert "decoded after:              1 images, 4.2 MP" in out, out


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))


# ── The denominator, found only by running it on a real bundle ────────────

HOST_CENSUS = (
    "[   11.243s] [image-census]   10.002s +130 images (+36.2MP) | total 252 images, "
    "78.3MP, 313.1MB resident | gpu +130 (+36.2MP) insert→gpu p50 4ms max 9ms\n"
)


def test_the_report_says_it_is_a_sample_not_a_population():
    """⛔⛔ `[image]` PRINTS ONLY DECODES >= 1.0 MP (NOTABLE_MEGAPIXELS). On the
    real host bundle that is ELEVEN printed lines against TWO HUNDRED AND
    FIFTY-TWO images actually decoded. "6 images before room-loaded" reads as
    the whole boot and is 4% of it by count.

    Found by running the script on a real capture and reconciling its 11 lines
    against df's brief, which said 98 — not by reading the emitter, which
    prints the threshold check several screens from the format string.
    """
    module = load()
    out = module.parse((BOOT + HOST_CENSUS).splitlines())
    assert out["census_total"] == (252, 78.3)


def test_the_bound_is_printed_before_the_counts(capsys):
    module = load()
    module.report(module.parse((BOOT + HOST_CENSUS).splitlines()))
    out = capsys.readouterr().out
    assert "SAMPLE, NOT POPULATION" in out
    assert out.index("SAMPLE, NOT POPULATION") < out.index("decoded BEFORE"), (
        "the caveat must precede the numbers it bounds — a reader who meets "
        "the count first has already formed the belief"
    )
    assert "252 images / 78.3 MP" in out


def test_a_log_with_no_census_line_says_the_counts_are_a_floor(capsys):
    """Absent is not zero: with no `[image-census]` there is no denominator,
    and the report must say the counts are a floor rather than imply totals."""
    module = load()
    module.report(module.parse(BOOT.splitlines()))
    out = capsys.readouterr().out
    assert "no `[image-census]` line" in out and "a floor" in out
