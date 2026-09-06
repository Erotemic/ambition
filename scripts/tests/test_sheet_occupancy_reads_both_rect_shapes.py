"""The occupancy census must read BOTH rect shapes, or it invents waste.

`measure_sheet_occupancy.py` asks what fraction of each spritesheet page the
runtime ever samples. Its first version answered **5% occupancy, 447.9 MP of
waste** across the tree. The real answer is **90% and 66.6 MP**.

⛔⛔ THE CAUSE WAS A REGEX THAT MATCHED ONE OF TWO FORMATS. 384 baked manifests
carry plain grid rects:

    (x: 112, y: 0, w: 144, h: 144)

and 174 carry packed ones, from a trimmed multi-page atlas:

    (x: 3519, y: 2457, w: 271, h: 405, page: 4, off: (121, 69))

A pattern anchored on `h: NNN)` matches the first and NOTHING in the second, so
the most tightly packed sheets in the tree reported **0% occupancy** — and 0%
reads as "this page is entirely waste", which is a finding, not an obvious
parser failure. It was caught because zero is not a plausible measurement, and
it would have been published otherwise.

⛔ A SECOND, EARLIER VERSION counted `body_metrics`' `body_pixel_bbox`,
`hurtbox` and `hitbox` rects — identical in shape, dozens per sheet, in image
space — as sampled area, which inflates occupancy toward 100% and argues there
is no waste at all. The two mistakes fail in OPPOSITE directions, which is why
neither a low nor a high number is self-evidently right.

These tests pin the parse against fixtures of both shapes, and against the
manifests' own declared `frame_count`.
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
SCRIPT = REPO / "scripts/measure_sheet_occupancy.py"


def load():
    spec = importlib.util.spec_from_file_location("occupancy", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


PLAIN = """[
(
    target: "plain",
    image: "plain_spritesheet.png",
    body_metrics: Some((body_pixel_bbox: Some((x: 31, y: 11, w: 86, h: 120)),
        animations: {"idle": (hurtbox: Some((bbox: Some((x: 34, y: 16, w: 110, h: 119)))))})),
    rows: [
    (
        animation: "idle",
        frame_count: 2,
        rects: [
            (x: 0, y: 0, w: 10, h: 10),
            (x: 10, y: 0, w: 10, h: 10),
        ],
    ),
    ],
),
]"""

PACKED = """[
(
    target: "packed",
    image: "packed_spritesheet.png",
    images: ["packed_spritesheet.png", "packed_spritesheet.1.png"],
    rows: [
    (
        animation: "idle",
        frame_count: 3,
        rects: [
            (x: 0, y: 0, w: 10, h: 10, page: 0, off: (1, 2)),
            (x: 10, y: 0, w: 10, h: 10, page: 0, off: (3, 4)),
            (x: 0, y: 0, w: 20, h: 5, page: 1, off: (0, 0)),
        ],
    ),
    ],
),
]"""


def test_a_plain_grid_manifest_parses_its_frames():
    rects = load().frame_rects(PLAIN)
    assert len(rects) == 2, f"both grid frames, got {rects}"
    assert all(r[4] == 0 for r in rects), "a rect with no `page:` belongs to page 0"


def test_a_packed_manifest_parses_its_frames_and_their_pages():
    rects = load().frame_rects(PACKED)
    assert len(rects) == 3, (
        "a packed rect carries `page:` and `off:` after `h:`; a pattern that "
        "requires `h: NNN)` reads NONE of them and reports the page as empty"
    )
    assert sorted(r[4] for r in rects) == [0, 0, 1], "each rect keeps its page index"


def test_body_metrics_boxes_are_not_counted_as_sampled_area():
    """⛔ The opposite failure: hurtboxes are the same shape and are not frames."""
    rects = load().frame_rects(PLAIN)
    assert (31, 11, 86, 120, 0) not in rects, "body_pixel_bbox is not a frame rect"
    assert (34, 16, 110, 119, 0) not in rects, "a hurtbox is not a frame rect"


def test_pages_are_resolved_from_the_images_list():
    module = load()
    assert module.page_images(PACKED) == [
        "packed_spritesheet.png",
        "packed_spritesheet.1.png",
    ]
    assert module.page_images(PLAIN) == ["plain_spritesheet.png"], (
        "a plain manifest has only `image:`, and `images:` must not be invented"
    )


def test_covered_area_is_a_union_not_a_sum():
    union_area = load().union_area
    assert union_area([(0, 0, 10, 10), (10, 0, 10, 10)]) == (200, False)
    assert union_area([(0, 0, 10, 10), (0, 0, 10, 10)]) == (100, False), (
        "two rows naming the SAME frame must not be counted twice — that is how "
        "a sheet reports over 100% occupancy and reads as broken art"
    )
    assert union_area([(0, 0, 10, 10), (5, 0, 10, 10)]) == (150, True), (
        "and a genuine overlap takes the exact path, reporting that it did"
    )
    assert union_area([]) == (0, False)


def test_the_real_tree_still_parses_every_single_page_sheet():
    """⭐ THE PREMISE, AGAINST THE ACTUAL MANIFESTS — the check that would have
    caught the shipped bug. Every single-page sheet's parsed rects must equal
    the `frame_count` its own rows declare. A format change that this script
    cannot read shows up here as a mismatch instead of as a waste number.
    """
    module = load()
    tier_dir = module.SPRITE_DIRS[0][1]
    if not tier_dir.is_dir():
        pytest.skip(f"{tier_dir} is absent")
    rows = [r for r in module.sheets_in(tier_dir) if "skipped" not in r]
    assert len(rows) > 100, f"premise: the Full tier publishes many pages, saw {len(rows)}"

    single = [r for r in rows if "#" not in r["sheet"]]
    mismatched = [
        (r["sheet"], r["frames"], r["declared_frames"])
        for r in single
        if r["frames"] != r["declared_frames"]
    ]
    assert not mismatched, (
        "parsed rects disagree with declared frame_count — the census cannot be "
        f"quoted until this is understood: {mismatched[:5]}"
    )
    assert not [r for r in rows if r["occupancy"] > 1.0], "no page may exceed 100%"
    # ⛔ AND NOT ZERO EITHER. A page at 0% is what the broken parser produced;
    # it is not a state any published sheet is in.
    assert not [r for r in rows if r["occupancy"] == 0.0], (
        "a published page sampling NOTHING is a parse failure, not a finding"
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
