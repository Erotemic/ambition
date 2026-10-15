"""The variant pass reads the page names of a record without building the rest.

It asked every sheet record and part flipbook of four roots for its page
names, and built each file whole to answer: 72 s of a pass with nothing to
build (measured 2026-10-15). `RonParser.record_fields` and `top_field`
build only what was asked; each must answer as the full parse does.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))
sys.path.insert(0, str(REPO / "scripts" / "lib"))

pytest.importorskip("PIL")
pytest.importorskip("yaml")

import generate_visual_quality_variants as g  # noqa: E402

RECORDS = """// a comment with a ( bracket and a "quote
[
(
    target: "a",
    image: "a_spritesheet.png",
    fx: Some((cue_family: "a, b", residency: Core, fade: OverItsLife)),
    rows: [(animation: "idle)", rects: [(x: 1, y: 2, w: 3, h: 4)], note: "\\"]\\"")],
),
(
    target: "b",
    images: ["b_0.png", "b_1.png"],
    rows: [],
),
]
"""

FLIPBOOK = """(
    schema_version: 3,
    target: "c",
    pages: ["c_parts.png", "c_parts_1.png"],
    parts: [(name: "p0", page: 0, rect: (1, 2, 3, 4), pivot: (0.5, 0.5))],
)
"""


def _full_page_names(text: str) -> list[str]:
    root = g.RonParser(text).parse()
    records = root.items if isinstance(root, g.List_) else [root]
    return [name for record in records for name in g.page_filenames(record)]


def test_the_page_fields_of_each_record_are_the_full_parses(tmp_path: Path) -> None:
    ron = tmp_path / "x_spritesheet.ron"
    ron.write_text(RECORDS)
    assert g.page_filenames_safe(ron) == _full_page_names(RECORDS) == ["a_spritesheet.png", "b_0.png", "b_1.png"]
    built = g.RonParser(RECORDS).record_fields(frozenset(("image", "images")))
    assert [[key for key, _ in record.fields] for record in built] == [["image"], ["images"]]


def test_a_flipbooks_pages_are_read_without_its_parts() -> None:
    pages = g.RonParser(FLIPBOOK).top_field("pages")
    assert [item.value for item in pages.items] == ["c_parts.png", "c_parts_1.png"]
    # A malformed tail after `pages` is never read.
    assert g.RonParser(FLIPBOOK.replace("parts: [", "parts: [[[")).top_field("pages") is not None
    assert g.RonParser(FLIPBOOK).top_field("absent") is None
