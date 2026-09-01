"""No spritesheet page may quietly grow past what a GPU will upload.

⛔⛔ **NOTHING IN THIS REPOSITORY CHECKS SPRITESHEET PAGE SIZE.** The only
texture-dimension clamp anywhere is `settings/video/quality.rs`, and it applies
to BACKGROUNDS. Sprite pages are produced by the packer with a `page_size`
policy, and when that policy is loose the result is not a warning — it is a page
that a device refuses to upload, at the moment a player selects that character.

Measured 2026-09-01, published sprite variants:

    variant          files   worst side   >2048  >4096  >8192
    sprites            554        16236      64     23      7
    sprites_0_5x       542         8118      24      7      0
    sprites_0_25x      540         4059      13      0      0
    sprites_potato     536         1015       0      0      0
    sprite_packs       456         2048       0      0      0

`sprite_packs` is the ULTRAPACK path and caps at its tier `page_size`. The
`sprites*` variants come from the older publication path, which does not.

⚠ THE LIMITS THAT MATTER ARE NOT ONE NUMBER:

    wgpu downlevel_webgl2        2048    101 published files exceed it
    wgpu Limits::default()       8192      7 files exceed it
    typical desktop Vulkan      16384    the worst page (16236) fits, barely

⭐ THIS IS A RATCHET, NOT A TARGET. It pins today's numbers so the problem cannot
grow while nobody is looking. Lowering them is the improvement; a failure here
means a sheet got BIGGER, which is the direction that ships a broken character.
"""

from __future__ import annotations

import struct
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

# variant -> (worst side, count over 2048, over 4096, over 8192)
RATCHET = {
    "sprites": (16236, 64, 23, 7),
    "sprites_0_5x": (8118, 24, 7, 0),
    "sprites_0_25x": (4059, 13, 0, 0),
    "sprites_potato": (1015, 0, 0, 0),
    "sprite_packs": (2048, 0, 0, 0),
}


def png_dimensions(path: Path) -> tuple[int, int] | None:
    """Width and height from the IHDR, without decoding the image.

    Reading 24 bytes rather than pulling in an image library keeps this test
    runnable in an environment with no Pillow, which is where it has to run.
    """
    try:
        head = path.open("rb").read(24)
    except OSError:
        return None
    if len(head) < 24 or head[:8] != b"\x89PNG\r\n\x1a\n":
        return None
    return struct.unpack(">II", head[16:24])


def survey() -> dict[str, tuple[int, int, int, int]]:
    found: dict[str, list[int]] = {}
    seen: set[Path] = set()
    # ⚠ RECURSIVE, and the variant is the segment after `assets/`, not the parent
    # directory. `sprite_packs` nests one deeper (`sprite_packs/full/*.png`), so
    # reading `path.parent.name` silently dropped the ONE variant that proves
    # the packer's cap works.
    for pattern in ("crates/*/assets/**/*.png", "game/*/assets/**/*.png"):
        for path in REPO.glob(pattern):
            # ⚠ The web build's asset tree is SYMLINKS into these same files.
            # Following both would double every count and make the ratchet a
            # measure of how many trees link the file rather than how big it is.
            real = path.resolve()
            if real in seen:
                continue
            seen.add(real)
            parts = path.relative_to(REPO).parts
            if "assets" not in parts:
                continue
            variant = parts[parts.index("assets") + 1]
            if variant not in RATCHET:
                continue
            size = png_dimensions(path)
            if size is None:
                continue
            found.setdefault(variant, []).append(max(size))
    return {
        variant: (
            max(sides),
            sum(1 for s in sides if s > 2048),
            sum(1 for s in sides if s > 4096),
            sum(1 for s in sides if s > 8192),
        )
        for variant, sides in found.items()
    }


def test_no_sprite_variant_grew_past_its_ratchet():
    measured = survey()
    assert measured, (
        "no published sprite pages were found at all; this test would pass "
        "vacuously and guard nothing"
    )
    for variant, expected in RATCHET.items():
        assert variant in measured, f"{variant} disappeared; update the ratchet deliberately"
        worst, over2k, over4k, over8k = measured[variant]
        e_worst, e_2k, e_4k, e_8k = expected
        assert worst <= e_worst, (
            f"{variant}: worst page side grew {e_worst} -> {worst}. A bigger page is "
            f"the direction that ships a character no device will draw."
        )
        assert over8k <= e_8k, (
            f"{variant}: pages over 8192px grew {e_8k} -> {over8k}. That is wgpu's "
            f"DEFAULT limit, not an exotic one."
        )
        assert over4k <= e_4k, f"{variant}: pages over 4096px grew {e_4k} -> {over4k}"
        assert over2k <= e_2k, (
            f"{variant}: pages over 2048px grew {e_2k} -> {over2k}. 2048 is the "
            f"downlevel WebGL2 limit."
        )


def test_the_ultrapack_path_still_caps_its_pages():
    """Premise guard: the ratchet above must not be the only thing holding.

    `sprite_packs` comes from the packer's tier `page_size` and is capped by
    construction. If it ever exceeds 2048 the cap has stopped working, and the
    ratchet numbers for the other variants are measuring a pipeline that changed
    underneath them.
    """
    measured = survey()
    assert "sprite_packs" in measured, "the ULTRAPACK output is missing entirely"
    worst = measured["sprite_packs"][0]
    assert worst <= 2048, (
        f"ULTRAPACK page is {worst}px; its tier page_size is meant to cap this at 2048"
    )
