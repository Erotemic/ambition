"""The shipped font FILES must carry the glyphs Ambition actually draws.

⭐ THIS IS A FILE-LEVEL CLAIM, so it is asked of the files rather than of Rust.
A font with no glyph for a character does not fail — it draws an empty box —
which is why leaving menu prose at Bevy's built-in `FiraMono-subset.ttf` drew
tofu for `·` and `—` in every menu and curly quotation marks in speech bubbles
without a single error. The engine-side seam (`UiFonts`) now names a FAMILY and
a WEIGHT; this says the family it names can actually render the text.
"""

from __future__ import annotations

import struct
from pathlib import Path

import pytest

BUNDLED = (
    Path(__file__).resolve().parents[2]
    / "crates/ambition_platformer2d_actor_monolith/assets/fonts/bundled"
)

#  Every one of these has been drawn as an empty box in this game at some point.
#
#  - `·` and `—` are the menu tofu that `MenuFont` was created for.
#  - the curly quotes are what a speech bubble wraps its own text in, and are the
#    reason `fx.rs` refuses to leave a bubble at `TextFont::default()`.
#  - `►` is the selected-row marker; `UiFonts::selected_marker` falls back to a
#    bare `>` precisely when no bundled font is loaded, so the product face has
#    to be able to draw the good one.
PRODUCT_GLYPHS = "AMBITION 0123456789 · — “quoted” ► ×"


def _tables(data: bytes) -> dict[str, tuple[int, int]]:
    _, num_tables = struct.unpack(">IH", data[0:6])
    out = {}
    for i in range(num_tables):
        rec = 12 + 16 * i
        tag = data[rec : rec + 4].decode("latin1")
        _, offset, length = struct.unpack(">III", data[rec + 4 : rec + 16])
        out[tag] = (offset, length)
    return out


def _covered_codepoints(path: Path) -> set[int]:
    """Every codepoint the file's `cmap` maps, from its Unicode subtables.

    Formats 4 and 12 are the two that matter for a modern OTF/TTF; a subtable in
    any other format is skipped rather than guessed at, which can only make this
    check STRICTER, never falsely green.
    """
    data = path.read_bytes()
    cmap_offset, _ = _tables(data)["cmap"]
    _, num_subtables = struct.unpack(">HH", data[cmap_offset : cmap_offset + 4])
    covered: set[int] = set()
    for i in range(num_subtables):
        rec = cmap_offset + 4 + 8 * i
        platform, encoding, sub_offset = struct.unpack(">HHI", data[rec : rec + 8])
        # Unicode (0, *) or Windows-Unicode (3, 1) / (3, 10).
        if not (platform == 0 or (platform == 3 and encoding in (1, 10))):
            continue
        sub = cmap_offset + sub_offset
        (fmt,) = struct.unpack(">H", data[sub : sub + 2])
        if fmt == 4:
            seg_x2 = struct.unpack(">H", data[sub + 6 : sub + 8])[0]
            segs = seg_x2 // 2
            ends = struct.unpack(f">{segs}H", data[sub + 14 : sub + 14 + seg_x2])
            starts_at = sub + 14 + seg_x2 + 2
            starts = struct.unpack(f">{segs}H", data[starts_at : starts_at + seg_x2])
            deltas_at = starts_at + seg_x2
            deltas = struct.unpack(f">{segs}h", data[deltas_at : deltas_at + seg_x2])
            ranges_at = deltas_at + seg_x2
            ranges = struct.unpack(f">{segs}H", data[ranges_at : ranges_at + seg_x2])
            for seg in range(segs):
                for cp in range(starts[seg], min(ends[seg], 0xFFFF) + 1):
                    if cp == 0xFFFF:
                        continue
                    if ranges[seg] == 0:
                        glyph = (cp + deltas[seg]) & 0xFFFF
                    else:
                        idx = ranges_at + seg * 2 + ranges[seg] + (cp - starts[seg]) * 2
                        if idx + 2 > len(data):
                            continue
                        (glyph,) = struct.unpack(">H", data[idx : idx + 2])
                        if glyph:
                            glyph = (glyph + deltas[seg]) & 0xFFFF
                    if glyph:
                        covered.add(cp)
        elif fmt == 12:
            (n_groups,) = struct.unpack(">I", data[sub + 12 : sub + 16])
            for g in range(n_groups):
                at = sub + 16 + g * 12
                start, end, start_glyph = struct.unpack(">III", data[at : at + 12])
                if start_glyph:
                    covered.update(range(start, end + 1))
    return covered


@pytest.mark.parametrize(
    "filename",
    ["InterDisplay-Regular.otf", "InterDisplay-SemiBold.otf"],
)
def test_the_product_faces_draw_every_glyph_ambition_ships(filename: str) -> None:
    """⛔ BOTH WEIGHTS, because the semibold face is what nameplates use.

    A regular face that covers `—` and a semibold one that does not is a bug that
    only appears on plates, which is exactly the kind of gap a single-face check
    would miss.
    """
    path = BUNDLED / filename
    if not path.exists():
        pytest.skip(f"{filename} is a git-ignored bundled asset and is absent here")
    covered = _covered_codepoints(path)
    missing = sorted(
        {ch for ch in PRODUCT_GLYPHS if ch != " " and ord(ch) not in covered}
    )
    assert not missing, (
        f"{filename} has no glyph for {missing} — these render as empty boxes "
        "rather than failing, which is the whole reason this check exists"
    )


def test_the_reader_actually_reads_something() -> None:
    """⛔ THE GUARD ABOVE CANNOT BE ALLOWED TO PASS BY READING NOTHING.

    A cmap parser that returned an empty set would make every glyph "missing" and
    go red — but one that returned a set containing everything, or a file that
    silently parsed to a huge range, would go green forever. Pin a character the
    face must NOT have, so the check is proven to discriminate.
    """
    path = BUNDLED / "InterDisplay-Regular.otf"
    if not path.exists():
        pytest.skip("bundled font absent")
    covered = _covered_codepoints(path)
    assert ord("A") in covered
    assert 0x1F600 not in covered, (
        "Inter Display is not an emoji font; if U+1F600 reads as covered the "
        "cmap reader is returning ranges it did not actually parse"
    )
