"""A quality tier's sheet variant must actually hold fewer pixels.

`sprites_0_5x` and `sprites_0_25x` exist so a room or a setting can ask for a
cheaper character and get one. Nothing checked that the variant IS cheaper. A
variant published without being downscaled costs the FULL page at decode,
upload and residency while the tier system believes it saved 4x or 16x — and
nothing looks wrong on screen, because the art is correct, merely larger than
asked for. That is why it survived: the failure is invisible in the only place
anyone looks.

⭐ TWO INSTRUMENTS, TWO DIFFERENT FAILURES, AND NEITHER SEES THE OTHER'S.
`check_quality_variants_are_fresh.py` asks whether a tier file is OLDER than the
art it derives from — stale art at the right size. This one asks whether it is
the same SIZE as the full sheet — current art at the wrong size. A megapixel
census cannot tell those apart, and running only one leaves half the tree
unchecked. Run both.

⛔⛔ THIS IS THE OPPOSITE OF A QUALITY REGRESSION. Jon's standing rule is that
nothing may draw FEWER pixels than the setting asks for. These sheets draw MORE.
Correcting them removes no pixels from any tier that requested them.

⛔⛔ THESE USED TO BE RUN BY HAND OR NOT AT ALL, and the sentence that said so
outlived the fix. `AMBITION_ASSETS_ARE_CANONICAL` was the ONLY road in and
nothing in the repository set it — checked 2026-09-02 — so no lane had ever
evaluated the assertions below while two planning paragraphs called them
ratchets.

⇒ THE GATE ANSWERS ITSELF NOW. `canonical_assets.assets_are_canonical(repo)`
decides, and `why_not(repo)` supplies the skip reason: real files, no symlinks
borrowed from another checkout, and tier variants that are FRESH rather than
stale build output. The environment variable survives as a manual override for a
box you believe and the detector does not — it is no longer the road.
⚠ The first run of the ten was 2026-09-03, and it found something on the first
try, which is what the detector bought.

The tests that watch unconditionally are the fixture ones at the bottom of this
file; they are what keeps this from being a check that cannot fail on a box
whose assets are borrowed.

⭐ A RATCHET, NOT A WALL. Four sheets are already in this state and fixing them
means regenerating committed art, which is not this test's call. The four are
named with the date they were measured; the test fails on a FIFTH. Cousin of
the compile-cost and doc-link ratchets.

⛔ THE KNOWN LIST IS EMPTY, AND THAT IS THE FINDING. It held four names measured
2026-09-02 on one machine (`actor` 9.03 MP, `author` 8.36, `medic` 7.54,
`officer` 8.64 — each identical at Full, 0_5x and 0_25x, 67.2 MP where the tier
promises ~10.5). A fresh-clone generation elsewhere produced all four correctly
scaled. The variant script RESIZES for every module-kind target and cannot
produce an unscaled tier; the unscaled copies came from the renderer's own
`publish` CLI aimed at a tier dest, whose module `render` does `del opts`. So an
unscaled variant is a per-tier-CLI trace, never an expected state — see the
reconciliation in `docs/planning/engine/asset-preparation-and-residency.md`.
"""

from __future__ import annotations

import importlib.util
import os
import subprocess
from pathlib import Path

import sys
import pathlib
import pytest

# ⛔ THE GATE WAS AN ENV VARIABLE NOTHING SET, so these ratchets had never
# been evaluated by any lane while planning called them ratchets. It now
# DETECTS the canonical box -- a checkout whose sprite tree holds real
# files generated them; one holding symlinks is borrowing another
# checkout's. The variable remains as an override.
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / "lib"))
from canonical_assets import assets_are_canonical, why_not  # noqa: E402

_REPO = pathlib.Path(__file__).resolve().parents[2]
ASSETS_ARE_CANONICAL = assets_are_canonical(_REPO)
NOT_CANONICAL_REASON = why_not(_REPO)

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/measure_tier_variant_scaling.py"

# ⛔⛔ EMPTIED 2026-09-02, WITH THE REASON, AFTER TWO BOXES DISAGREED.
#
# This list held `actor`, `author`, `medic`, `officer`. A fresh-clone generation
# on another machine produced CORRECTLY SCALED variants for all four, and the
# reconciliation says why: `generate_visual_quality_variants.py` sends every
# `module`-kind target to `build_sheet_variant`, which RESIZES the full sheet.
# Those four are module-kind, so through that script they scale.
#
# The unscaled copies came from a different road — the renderer's own `publish`
# CLI aimed at a tier `--dest-root`, which calls the target module's `render`,
# whose body is `del opts` and throws the `quality_scale` away.
#
# ⇒ So an unscaled variant is NOT the expected state anywhere; it is the trace
# of a per-tier CLI render. A box carrying one should fail this test and fix it
# by re-running the variant script. Encoding those four as "known" would have
# taught every future reader that the tree is meant to look like this.
#
# Do NOT add a name to make a red test green — that is what this list exists to
# prevent, and the reason it is empty rather than deleted.
KNOWN_UNSCALED: set[str] = set()


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


@pytest.mark.skipif(not ASSETS_ARE_CANONICAL, reason=NOT_CANONICAL_REASON)
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


@pytest.mark.skipif(not ASSETS_ARE_CANONICAL, reason=NOT_CANONICAL_REASON)
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


@pytest.mark.skipif(not ASSETS_ARE_CANONICAL, reason=NOT_CANONICAL_REASON)
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
# ⛔ EMPTIED 2026-09-02 FOR THE SAME REASON AS `KNOWN_UNSCALED` ABOVE.
# `performer`'s full sheet was rendered 2026-08-29, a week after this box last
# ran the variant script — so its variants are absent here for the same reason
# 82 other tier files are STALE here (`check_quality_variants_are_fresh.py`
# reports both, and exits 1 on this checkout). That is one machine's
# un-regenerated tree, not a property of the sheet, and encoding it as "known"
# would teach a future reader that `performer` is expected to ship without tiers.
KNOWN_MISSING_VARIANTS: set[str] = set()


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


@pytest.mark.skipif(not ASSETS_ARE_CANONICAL, reason=NOT_CANONICAL_REASON)
def test_no_new_sheet_ships_without_a_reduced_variant():
    found = missing_variants()
    new = set(found) - KNOWN_MISSING_VARIANTS
    assert not new, (
        "a sheet publishes no variant at a reduced tier, so a room asking for "
        f"that tier gets the full-resolution page: {sorted(new)}. Generate the "
        "variants (`scripts/regen/sprites.sh --target <name>`), or record why "
        "the sheet cannot have them."
    )


@pytest.mark.skipif(not ASSETS_ARE_CANONICAL, reason=NOT_CANONICAL_REASON)
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
