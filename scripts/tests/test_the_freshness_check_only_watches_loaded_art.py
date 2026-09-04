"""The quality-tier freshness check must watch what the runtime LOADS.

⛔⛔ IT ONCE WATCHED EVERYTHING IN THE TIER, AND THE COST WAS TEN OTHER GUARDS.
`stale_pairs` iterated every `*.ron` under a tier directory on the premise that
"a tier file that EXISTS is therefore loaded". Two families in those directories
are installed and read by nothing:

  * `*_actor.ron` — `ArtifactClass::ActorSidecar`, in its own words
    *"Transitional actor-contract sidecar. Installed today, NOT YET CONSUMED by
    the sandbox."*
  * `*_portraits.ron` — `ambition_sprite_sheet/build.rs` collects portrait
    manifests from `assets/sprites` and never from a reduced tier dir, which
    `test_build_rs_still_bakes_portrait_manifests_from_full_resolution_only`
    exists to pin.

⇒ Four such files being older than their source made the check fail with
*"the game is drawing OLD art at Low/Medium/Potato"* — about files the game
cannot load — and prescribed `quality_variants.sh`, which does not produce them
and reports *"already current"* even under `--force`. **A failure whose
prescribed fix cannot clear it.** Because `assets_are_canonical()` consults this
check, every guard behind that marker skipped: ten asset ratchets that had never
been evaluated by any lane stayed unevaluated on four false positives.

⭐ THE POPULATION AGREED. Measured 2026-09-04 across the three tiers: 206
`_spritesheet.ron` in each, plus 47 `_actor.ron` and 9 `_portraits.ron` in the
two older ones and NONE of either in `sprites_potato`. A family the generator
still produced would not be missing from the tier it rebuilt last.

⚠ THIS TEST PINS BOTH DIRECTIONS, because narrowing a check is exactly the edit
that can quietly remove its teeth: a loaded sheet gone stale must still be
caught, and an unloaded sidecar gone stale must not be reported.
"""

from __future__ import annotations

import importlib.util
import os
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent


def _checker():
    path = REPO / "scripts/check_quality_variants_are_fresh.py"
    spec = importlib.util.spec_from_file_location("quality_freshness", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules["quality_freshness"] = module
    spec.loader.exec_module(module)
    return module


def _publish(directory: Path, stem: str, *, mtime: float) -> None:
    """A sheet unit: the manifest and the one page it names."""
    directory.mkdir(parents=True, exist_ok=True)
    page = f"{stem}.png"
    (directory / f"{stem}.ron").write_text(f'(pages: ["{page}"])', encoding="utf-8")
    (directory / page).write_bytes(b"\x89PNG")
    for name in (f"{stem}.ron", page):
        os.utime(directory / name, (mtime, mtime))


def test_a_stale_sheet_the_runtime_loads_is_still_reported(tmp_path) -> None:
    module = _checker()
    source, tier = tmp_path / "sprites", tmp_path / "sprites_0_5x"
    _publish(source, "officer_spritesheet", mtime=2_000_000)
    _publish(tier, "officer_spritesheet", mtime=1_000_000)

    stale = module.stale_pairs(source, tier)
    assert [path.name for path, _ in stale] == ["officer_spritesheet.ron"], (
        "a tier sheet older than its source is the defect this check exists for; "
        f"narrowing it must not remove that. Got {stale}"
    )


def test_a_stale_sidecar_the_runtime_never_loads_is_not_reported(tmp_path) -> None:
    module = _checker()
    source, tier = tmp_path / "sprites", tmp_path / "sprites_0_5x"
    # The loaded family is current, so anything reported comes from the others.
    _publish(source, "officer_spritesheet", mtime=1_000_000)
    _publish(tier, "officer_spritesheet", mtime=1_000_000)
    for stem in ("officer_actor", "officer_portraits"):
        _publish(source, stem, mtime=2_000_000)
        _publish(tier, stem, mtime=1_000_000)

    assert module.stale_pairs(source, tier) == [], (
        "`_actor.ron` is an ActorSidecar the sandbox does not consume and "
        "`_portraits.ron` is baked from full resolution only, so neither being "
        "behind means the game draws old art — and reporting them fails a check "
        "whose prescribed fix cannot clear it"
    )
