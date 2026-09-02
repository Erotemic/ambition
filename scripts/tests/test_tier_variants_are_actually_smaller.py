"""A quality tier's sheet variant must actually hold fewer pixels.

`sprites_0_5x` and `sprites_0_25x` exist so a room or a setting can ask for a
cheaper character and get one. Nothing checked that the variant IS cheaper. A
variant published without being downscaled costs the FULL page at decode,
upload and residency while the tier system believes it saved 4x or 16x — and
nothing looks wrong on screen, because the art is correct, merely larger than
asked for. That is why it survived: the failure is invisible in the only place
anyone looks.

⛔⛔ THIS IS THE OPPOSITE OF A QUALITY REGRESSION. Jon's standing rule is that
nothing may draw FEWER pixels than the setting asks for. These sheets draw MORE.
Correcting them removes no pixels from any tier that requested them.

⭐ A RATCHET, NOT A WALL. Four sheets are already in this state and fixing them
means regenerating committed art, which is not this test's call. The four are
named with the date they were measured; the test fails on a FIFTH. Cousin of
the compile-cost and doc-link ratchets.

Measured 2026-09-02 by `scripts/measure_tier_variant_scaling.py`:
`actor` 9.03 MP, `author` 8.36, `medic` 7.54, `officer` 8.64 — each identical at
Full, 0_5x and 0_25x. A room asking for those tiers decodes 67.2 MP where the
tier promises ~10.5.
"""

from __future__ import annotations

import importlib.util
import os
import subprocess
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/measure_tier_variant_scaling.py"

# ⛔ KNOWN, MEASURED 2026-09-02, AND NOT A LICENCE. Each of these publishes a
# 0_5x and a 0_25x variant identical in total page megapixels to its Full sheet.
# Remove a name when its variants are regenerated; do NOT add one to make a red
# test green — that is the whole thing this list exists to prevent.
KNOWN_UNSCALED = {
    "actor_spritesheet.ron",
    "author_spritesheet.ron",
    "medic_spritesheet.ron",
    "officer_spritesheet.ron",
}


def load():
    spec = importlib.util.spec_from_file_location("tier_scaling", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def offenders() -> dict[str, dict[str, float]]:
    module = load()
    if not (module.ASSETS / "sprites").is_dir():
        pytest.skip("the actor-monolith sprite tree is absent")
    found: dict[str, dict[str, float]] = {}
    for rel, per_tier in module.collect().items():
        full = per_tier.get("sprites")
        if not full:
            continue
        failed = {
            tier: per_tier[tier][0]
            for tier, _ in module.TIERS[1:3]
            if tier in per_tier and per_tier[tier][0] >= full[0] * 0.95
        }
        if failed:
            found[rel.name] = failed
    return found


@pytest.mark.skipif(
    not os.environ.get("AMBITION_ASSETS_ARE_CANONICAL"),
    reason=(
        "⛔ THE SPRITE TREE IS GITIGNORED AND MACHINE-LOCAL. Every PNG under "
        "assets/sprites* is generated, and a worktree SYMLINKS the main "
        "checkout's copies (mirror_assets_for_worktree.py), so this asserts "
        "one machine's generated state. On a box that generates them correctly "
        "the KNOWN_ lists below read as stale and this fails for the wrong "
        "reason. Opt in with AMBITION_ASSETS_ARE_CANONICAL=1 where the assets "
        "are known-good; the SCRIPT's own behaviour is pinned unconditionally "
        "by the fixture tests at the bottom."
    ),
)
def test_no_new_sheet_ships_a_tier_variant_that_is_not_smaller():
    found = offenders()
    new = set(found) - KNOWN_UNSCALED
    assert not new, (
        "a sheet's reduced-tier variant holds as many pixels as its Full sheet, "
        "so a room asking for that tier pays full residency and nothing looks "
        f"wrong on screen: {sorted(new)}. Regenerate the variant, or — if the "
        "sheet genuinely cannot downscale — add it to KNOWN_UNSCALED with the "
        "reason, never to quiet the test."
    )


@pytest.mark.skipif(
    not os.environ.get("AMBITION_ASSETS_ARE_CANONICAL"),
    reason="asserts machine-local generated assets; see the note above",
)
def test_the_known_list_does_not_rot():
    """⛔ A RATCHET THAT ONLY EVER GROWS IS NOT A RATCHET. If a name here has
    been fixed, the list must lose it — otherwise the guard silently permits a
    regression on a sheet somebody already repaired.
    """
    found = offenders()
    fixed = KNOWN_UNSCALED - set(found)
    assert not fixed, (
        f"{sorted(fixed)} now scale correctly — remove them from KNOWN_UNSCALED "
        "so a future regression on them is caught"
    )


@pytest.mark.skipif(
    not os.environ.get("AMBITION_ASSETS_ARE_CANONICAL"),
    reason="asserts machine-local generated assets; see the note above",
)
def test_the_measurement_can_see_a_sheet_that_does_scale():
    """⭐ POSITIVE CONTROL. Both tests above are absence assertions; if `collect`
    returned nothing they would pass forever. This pins that the tree really
    does contain correctly-scaled variants and that they are measured.
    """
    module = load()
    if not (module.ASSETS / "sprites").is_dir():
        pytest.skip("the actor-monolith sprite tree is absent")
    rows = module.collect()
    assert len(rows) > 100, f"premise: many sheets are measured, saw {len(rows)}"

    scaled = [
        rel.name
        for rel, per in rows.items()
        if per.get("sprites")
        and per.get("sprites_0_25x")
        and per["sprites_0_25x"][0] < per["sprites"][0] * 0.5
    ]
    assert len(scaled) > 50, (
        "the tree must contain many correctly-downscaled 0_25x variants; if it "
        f"does not, the measurement is broken rather than the art. saw {len(scaled)}"
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))


# ⛔ THE SECOND MECHANISM. A sheet publishing NO variant at a reduced tier costs
# exactly as much as one whose variant was never downscaled, and cannot be seen
# by comparing megapixels — there is nothing to compare against. Measured
# 2026-09-02: `performer` (9.03 MP) publishes neither 0_5x nor 0_25x, and is
# byte-identical to `actor`, which publishes both and shrinks neither. The same
# artwork therefore fails to get cheaper by TWO different mechanisms.
KNOWN_MISSING_VARIANTS = {"performer_spritesheet.ron"}


def missing_variants() -> dict[str, list[str]]:
    module = load()
    if not (module.ASSETS / "sprites").is_dir():
        pytest.skip("the actor-monolith sprite tree is absent")
    rows = module.collect()
    out: dict[str, list[str]] = {}
    for rel, per_tier in rows.items():
        if "sprites" not in per_tier:
            continue
        absent = [t for t, _ in module.TIERS[1:3] if t not in per_tier]
        if absent:
            out[rel.name] = absent
    return out


@pytest.mark.skipif(
    not os.environ.get("AMBITION_ASSETS_ARE_CANONICAL"),
    reason="asserts machine-local generated assets; see the note above",
)
def test_no_new_sheet_ships_without_a_reduced_variant():
    found = missing_variants()
    new = set(found) - KNOWN_MISSING_VARIANTS
    assert not new, (
        "a sheet publishes no variant at a reduced tier, so a room asking for "
        f"that tier gets the full-resolution page: {sorted(new)}. Generate the "
        "variants (`scripts/regen/sprites.sh --target <name>`), or record why "
        "the sheet cannot have them."
    )


@pytest.mark.skipif(
    not os.environ.get("AMBITION_ASSETS_ARE_CANONICAL"),
    reason="asserts machine-local generated assets; see the note above",
)
def test_the_missing_variant_list_does_not_rot():
    found = missing_variants()
    fixed = KNOWN_MISSING_VARIANTS - set(found)
    assert not fixed, (
        f"{sorted(fixed)} now publish their reduced variants — remove them from "
        "KNOWN_MISSING_VARIANTS so a regression is caught"
    )


# ── The SCRIPT's behaviour, on fixtures, unconditionally ──────────────────
#
# ⛔⛔ EVERY TEST ABOVE IS SKIPPED BY DEFAULT, so without these this file is a
# check that cannot fail — the exact family it was written to help catch. These
# pin the measurement's logic on synthetic manifests, which are the same on
# every machine.

PLAIN_MANIFEST = '(target: "x", image: "x.png", rows: [])'
PACKED_MANIFEST = '(target: "x", image: "x.png", images: ["x.png", "x.1.png"], rows: [])'


def _png(tmp_path, name, width, height):
    """A PNG with a real IHDR and nothing else — the script reads 24 bytes."""
    import struct
    import zlib

    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    chunk = struct.pack(">I", len(ihdr)) + b"IHDR" + ihdr
    chunk += struct.pack(">I", zlib.crc32(b"IHDR" + ihdr) & 0xFFFFFFFF)
    path = tmp_path / name
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk)
    return path


def test_png_size_reads_the_header_without_decoding(tmp_path):
    module = load()
    assert module.png_size(_png(tmp_path, "a.png", 640, 480)) == (640, 480)
    bogus = tmp_path / "b.png"
    bogus.write_bytes(b"not a png at all")
    assert module.png_size(bogus) is None, "a non-PNG must not report a size"


def test_sheet_megapixels_sums_every_page(tmp_path):
    module = load()
    _png(tmp_path, "x.png", 1000, 1000)
    _png(tmp_path, "x.1.png", 500, 1000)
    plain = tmp_path / "x_spritesheet.ron"
    plain.write_text(PLAIN_MANIFEST)
    assert module.sheet_megapixels(plain) == (1.0, 1), "one page, 1 MP"

    packed = tmp_path / "y_spritesheet.ron"
    packed.write_text(PACKED_MANIFEST)
    assert module.sheet_megapixels(packed) == (1.5, 2), (
        "a packed sheet's cost is the SUM over its pages — measuring one page's "
        "dimensions called two correctly-scaled atlases offenders"
    )


def test_a_missing_page_refuses_the_sheet_rather_than_shrinking_it(tmp_path):
    """⛔ An unreadable page would make the total an UNDERCOUNT, which reads as
    'this tier is smaller' — the opposite of the truth, and silent."""
    module = load()
    _png(tmp_path, "x.png", 1000, 1000)
    packed = tmp_path / "y_spritesheet.ron"
    packed.write_text(PACKED_MANIFEST)  # names x.1.png, which does not exist
    assert module.sheet_megapixels(packed) is None
