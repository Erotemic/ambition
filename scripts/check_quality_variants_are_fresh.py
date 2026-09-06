#!/usr/bin/env python3
"""Check that reduced-resolution sprite tiers are fresh relative to full resolution.

Low/Medium/Potato profiles load downscaled tier files, so stale tier assets can
show older character art while the full-resolution profile is current. Freshness
uses modification time with a tolerance large enough to ignore checkout-order
jitter but small enough to catch real stale generations.

Usage::

    python3 scripts/check_quality_variants_are_fresh.py
    python3 scripts/check_quality_variants_are_fresh.py --asset-root <dir>

Exit 1 names stale files and the regeneration command."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# The tier roots the runtime loads, keyed by the `sprites/`-relative source they
# are derived from. `parallax_layers_*` lives under `backgrounds/` and is checked
# through the same rule.
SPRITE_TIERS = ("sprites_0_5x", "sprites_0_25x", "sprites_potato")
PARALLAX_TIERS = (
    "parallax_layers_0_5x",
    "parallax_layers_0_25x",
    "parallax_layers_potato",
)

# Ten minutes also covers a long regen run, where a tier written early legitimately predates a
# source touched late in the same invocation.
STALE_AFTER_S = 600.0


def page_names(manifest: Path) -> set[str]:
    """The page PNGs a spritesheet manifest names.

    ⛔ **THE ORPHAN PAGE IS WHY THIS FUNCTION EXISTS**, and the check found it on
    its first real run. A downscaled sheet REPACKS: `perfect_cellular_automaton`
    is 7 pages at full resolution and 2 at half, because half-size cells fit a
    page. So `sprites_0_5x/..._spritesheet.3.png` is a leftover from an older
    build with more pages — a file the runtime never opens, because the TIER'S OWN
    manifest does not name it.

    ⭐ a tier file is only worth an opinion if something loads it. Comparing every
    file on disk instead flagged three dead PNGs as a product bug, which is the
    behaviour that gets a check muted rather than fixed.
    """
    try:
        text = manifest.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return set()
    return set(re.findall(r'[\w.\-]+\.png', text))


# The only per-tier manifest the runtime loads: `build.rs` bakes one sheet
# record per tier from it. `_actor.ron` and `_portraits.ron` also appear in a
# tier directory and are read by nothing — see `stale_pairs` for the evidence.
LOADED_TIER_MANIFEST_SUFFIX = "_spritesheet.ron"


def newest_input(source_dir: Path, manifest: Path) -> float:
    """Newest mtime among a source sheet's manifest and the pages it names."""
    stamps = [manifest.stat().st_mtime]
    for name in page_names(manifest):
        page = source_dir / name
        if page.is_file():
            stamps.append(page.stat().st_mtime)
    return max(stamps)


def stale_pairs(source_dir: Path, tier_dir: Path) -> list[tuple[Path, float]]:
    """Every published unit in `tier_dir` that is meaningfully behind its source.

    Two kinds of unit, because the generator publishes two kinds:

    * a sheet — `X.ron` plus the pages it names. The unit is compared as a
      whole against the source `X.ron` and ITS pages, which is the same rule the
      generator's own freshness check uses. Comparing page-to-page cannot work:
      the tier repacks and the page counts differ (see `page_names`).
    * a loose PNG — no manifest, so it is its own unit and compares directly.

    ⚠ iterates the TIER, not the source. A source with no tier file at all is not
    this check's business — the runtime falls back to full resolution, which is
    correct art at the wrong cost, and the generator's coverage decides which
    sheets are worth downscaling. What this check owns is the narrower and much
    worse case: a tier file that EXISTS, IS LOADED, and is not the art it claims
    to be.

    ⛔⛔ "EXISTS, IS THEREFORE LOADED" WAS THIS FUNCTION'S PREMISE AND IT IS
    FALSE — corrected 2026-09-04, and the cost of the error was ten OTHER
    guards. Only `*_spritesheet.ron` is baked per tier. The two other `.ron`
    families that appear in a tier directory are installed and read by nothing:

      * `*_actor.ron` — `ArtifactClass::ActorSidecar` in
        `asset_publish/classify.rs`, in its own words *"Transitional
        actor-contract sidecar. Installed today, NOT YET CONSUMED by the
        sandbox."*
      * `*_portraits.ron` — `ambition_sprite_sheet/build.rs` collects portrait
        manifests from `assets/sprites` and never from a reduced tier dir. That
        is not an inference: `test_build_rs_still_bakes_portrait_manifests_from_
        full_resolution_only` exists to pin it, and the repo already carries a
        `reduced_tier_portraits` bucket of 487 files unreachable for exactly
        this reason.

    ⇒ Flagging them claimed the game *"is drawing OLD art at Low/Medium/Potato"*
    about files it cannot load, and prescribed `quality_variants.sh` — which
    does not produce them, and reports *"already current"* even under `--force`.
    **A failure whose prescribed fix cannot clear it, gating every guard behind
    the same canonical-assets marker: ten asset ratchets that had never been
    evaluated by any lane stayed skipped on four false positives.**

    ⭐ THE POPULATION SAYS IT TOO. Measured 2026-09-04 across the three tiers:
    206 `_spritesheet.ron` in each, plus 47 `_actor.ron` and 9 `_portraits.ron`
    in the two older ones and NONE of either in `sprites_potato`. A family the
    generator still produced would not be absent from the tier it rebuilt last.
    """
    if not tier_dir.is_dir():
        return []
    stale: list[tuple[Path, float]] = []
    claimed: set[Path] = set()
    for manifest in sorted(tier_dir.rglob("*.ron")):
        if not manifest.name.endswith(LOADED_TIER_MANIFEST_SUFFIX):
            # Installed, unread, and not the generator's to refresh. See above.
            continue
        source = source_dir / manifest.relative_to(tier_dir)
        claimed.add(manifest)
        for name in page_names(manifest):
            claimed.add(manifest.parent / name)
        if not source.is_file():
            continue
        published = min(
            [manifest.stat().st_mtime]
            + [
                (manifest.parent / name).stat().st_mtime
                for name in page_names(manifest)
                if (manifest.parent / name).is_file()
            ]
        )
        behind = newest_input(source_dir, source) - published
        if behind > STALE_AFTER_S:
            stale.append((manifest, behind))
    for published in sorted(tier_dir.rglob("*.png")):
        # Pages belong to the sheet unit above; only the loose images are their
        # own unit. (`.yaml` sidecars are debug output nothing loads.)
        if published in claimed or not published.is_file():
            continue
        # and a PNG whose SHEET is published here but which that sheet does not
        # name is an ORPHAN page, not a stale one — the tier repacked into fewer
        # pages and left the old file behind. Nothing loads it. Saying so would be
        # a false alarm about dead bytes; the generator's prune phase owns them.
        stem = re.sub(r"\.\d+$", "", published.name[: -len(".png")])
        if (published.parent / f"{stem}.ron").is_file():
            continue
        source = source_dir / published.relative_to(tier_dir)
        if not source.is_file():
            continue
        behind = source.stat().st_mtime - published.stat().st_mtime
        if behind > STALE_AFTER_S:
            stale.append((published, behind))
    return stale


def absent_variants(source_dir: Path, tier_dir: Path) -> list[Path]:
    """Source manifests with NO counterpart in `tier_dir` at all.

    ⛔⛔ **`stale_pairs` CANNOT SEE THESE.** It walks the TIER directory, so a
    manifest that was never published there is never visited — a variant that is
    ABSENT is invisible to a check whose whole question is "is this file older
    than its source". Measured 2026-09-01: `performer_spritesheet` has no Half,
    Quarter or Potato variant, and `actor`/`medic` have no Potato, and this tool
    reported 166 stale files and said nothing about any of them.

    An absent variant does not draw old art. It draws the FULL-resolution pages
    to a device that asked for less, which is the failure the tiers exist to
    prevent.

    ⚠ GAMEPLAY SHEETS ONLY, because that is the only family published at every
    tier. Measured over the same root: `_spritesheet.ron` is 206 at full and 205
    at each variant, while `_portraits.ron` is 160 against 9 and `_actor.ron` is
    192 against 47. Those two are published SELECTIVELY, so their absence is
    policy and reporting it drowns the real finding — a first cut of this check
    said 979 files were missing and every one of them was fine.
    """
    if not tier_dir.is_dir() or not source_dir.is_dir():
        return []
    return [
        manifest
        for manifest in sorted(source_dir.rglob("*_spritesheet.ron"))
        if not (tier_dir / manifest.relative_to(source_dir)).exists()
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--asset-root",
        type=Path,
        default=Path("crates/ambition_platformer2d_actor_monolith/assets"),
        help="gameplay-core asset root containing sprites/ and backgrounds/",
    )
    args = parser.parse_args()
    root: Path = args.asset_root.resolve()

    checked = 0
    stale: list[tuple[Path, float]] = []
    absent: list[tuple[str, Path]] = []
    for tier in SPRITE_TIERS:
        tier_dir = root / tier
        checked += tier_dir.is_dir()
        stale.extend(stale_pairs(root / "sprites", tier_dir))
        absent.extend((tier, m) for m in absent_variants(root / "sprites", tier_dir))
    for tier in PARALLAX_TIERS:
        tier_dir = root / "backgrounds" / tier
        checked += tier_dir.is_dir()
        stale.extend(stale_pairs(root / "backgrounds" / "parallax_layers", tier_dir))

    # zero tier roots is a FAILURE, not a pass. A check that silently succeeds
    # when pointed at the wrong directory is the shape of a guard that reports
    # green for years — and `--asset-root` makes pointing it somewhere wrong a
    # single typo away.
    if checked == 0:
        print(
            f"no quality-tier roots under {root} — this check looked at nothing.\n"
            f"  expected some of: {', '.join(SPRITE_TIERS)}",
            file=sys.stderr,
        )
        return 1

    if absent:
        print(
            f"{len(absent)} source manifest(s) have NO variant at a tier the game "
            f"ships.\nA missing variant does not draw old art — it draws the "
            f"FULL-resolution pages to a device that asked for less, which is the "
            f"failure the tiers exist to prevent.\n",
            file=sys.stderr,
        )
        for tier, manifest in absent[:20]:
            print(
                f"    no {tier:16} for  {manifest.relative_to(root)}",
                file=sys.stderr,
            )
        if len(absent) > 20:
            print(f"    ... and {len(absent) - 20} more", file=sys.stderr)
        print("", file=sys.stderr)

    if stale:
        print(
            f"{len(stale)} published quality-tier file(s) are older than the "
            f"full-resolution art they are derived from.\n"
            f"The game loads these under the Low / Medium / Potato visual "
            f"quality profiles, so it is drawing OLD art at those settings and "
            f"current art at High — which looks like the character changing when "
            f"the quality setting does.\n",
            file=sys.stderr,
        )
        for published, behind in stale[:20]:
            print(
                f"    {behind / 86400:6.1f} days behind  "
                f"{published.relative_to(root)}",
                file=sys.stderr,
            )
        if len(stale) > 20:
            print(f"    ... and {len(stale) - 20} more", file=sys.stderr)
        print(
            "\n  fix: ./scripts/regen/quality_variants.sh   (incremental; "
            "rebuilds only what is stale)",
            file=sys.stderr,
        )
        return 1

    if absent:
        print(
            "  fix: ./scripts/regen/quality_variants.sh   (publishes the missing "
            "tiers too)",
            file=sys.stderr,
        )
        return 1

    print(f"quality tiers are current with {root}/sprites")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
