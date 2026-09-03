"""A shipped sprite PNG must be named by something.

`package_asset_guard.py` records "every regular file" from the asset roots, so
a page left under `assets/sprites*/` ships whether or not any road reaches it.
It costs package size and nothing else — no decode, no residency — which is why
every runtime measurement calls the tree healthy.

Measured 2026-09-02 by `scripts/measure_orphan_shipped_pages.py`: **44 stranded
pages, 92.0 MB**, from four sheets whose manifests are single-page (`image:
"x.png"`) while `x_spritesheet.1.png` … `.5.png` sit beside them — pages from a
time the sheet was multi-page. A sheet's pages resolve ONLY through its
manifest, so these are unreachable by construction.

⭐ A RATCHET, NOT A WALL. The four are named with the date they were measured;
the test fails on a FIFTH. Whether the existing four are stale outputs or a
live generator defect is what a clean regen on another machine decides — this
test deletes nothing and asks for nothing to be deleted.

⛔⛔ AND IT ASSERTS ONE MACHINE'S GENERATED TREE. `assets/sprites*/` is
gitignored, and a worktree SYMLINKS the main checkout's copies, so a second
worktree agreeing is not a second source. The asset-reading tests are opt-in;
the SCRIPT's logic is pinned unconditionally on fixtures at the bottom, or this
file would be a check that cannot fail.
"""

from __future__ import annotations

import importlib.util
import os
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/measure_orphan_shipped_pages.py"

# ⛔ KNOWN, MEASURED 2026-09-02, AND NOT A LICENCE. Each names a single-page
# manifest with numbered siblings left beside it. Remove a name when its tree is
# regenerated clean; do NOT add one to make a red test green.
KNOWN_STRANDED_SHEETS = {
    "carl_stargan",  # 4 pages, 2.9 MB
    "pointed_polygon",  # 20 pages, 43.8 MB
    "projectile_polygon",  # 8 pages, 9.5 MB
    "pugnacious_polygon",  # 12 pages, 35.8 MB
}

CANONICAL = pytest.mark.skipif(
    not os.environ.get("AMBITION_ASSETS_ARE_CANONICAL"),
    reason=(
        "⛔ THE SPRITE TREE IS GITIGNORED AND MACHINE-LOCAL. Every PNG under "
        "assets/sprites*/ is generated, and a worktree SYMLINKS the main "
        "checkout's copies. On a box that regenerates cleanly the KNOWN_ list "
        "reads as stale and this fails for the wrong reason. Opt in with "
        "AMBITION_ASSETS_ARE_CANONICAL=1 where the assets are known-good; the "
        "script's own behaviour is pinned unconditionally below."
    ),
)


# ⛔ KNOWN, MEASURED 2026-09-02. Four sheet renders with no `.ron` at any of the
# four tiers (16 files, 13.6 MB) — `_full`, `_body` and `_hands` layer outputs
# left beside the manifested `gnu_ton_boss` / `giant_gnu` sheets the boss really
# uses. ⚠ The BOSS IS FINE: `gnu_ton_boss_spritesheet.ron` and
# `giant_gnu_spritesheet.ron` both exist and are claimed. These are extra
# renders, not missing art.
KNOWN_UNMANIFESTED_SHEETS = {
    "giant_gnu_body",
    "gnu_ton_boss_body",
    "gnu_ton_boss_full",
    "gnu_ton_boss_hands",
}


def load():
    spec = importlib.util.spec_from_file_location("orphan_pages", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    # ⛔ REGISTER BEFORE EXEC — a loader that swallows an import-time error into
    # a skip hides most of a file, which has already happened once here.
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def stranded_sheets() -> dict[str, int]:
    import re

    module = load()
    if not (module.ASSETS / "sprites").is_dir():
        pytest.skip("the actor-monolith sprite tree is absent")
    counts: dict[str, int] = {}
    for row in module.census()["stranded_pages"]:
        name = row["path"].split("/")[-1]
        base = re.match(r"(.+)_spritesheet\.\d+\.png", name)
        if base:
            counts[base.group(1)] = counts.get(base.group(1), 0) + 1
    return counts


@CANONICAL
def test_no_new_sheet_strands_pages():
    found = stranded_sheets()
    new = set(found) - KNOWN_STRANDED_SHEETS
    assert not new, (
        "a sheet ships numbered pages its own manifest does not name, so they "
        f"go into the package and nothing can ever load them: {sorted(new)}. "
        "Regenerate the sheet, or record why the pages must stay."
    )


@CANONICAL
def test_the_known_list_does_not_rot():
    """⛔ A RATCHET THAT ONLY EVER GROWS IS NOT A RATCHET."""
    found = stranded_sheets()
    fixed = KNOWN_STRANDED_SHEETS - set(found)
    assert not fixed, (
        f"{sorted(fixed)} no longer strand pages — remove them from "
        "KNOWN_STRANDED_SHEETS so a regression on them is caught"
    )


@CANONICAL
def test_the_census_can_see_claimed_pages():
    """⭐ POSITIVE CONTROL. Both tests above are absence assertions; if `scan`
    returned nothing they would pass forever."""
    module = load()
    out = module.census()
    assert out["total_pngs"] > 1000, f"premise: the tree is large, saw {out}"
    assert out["claimed"] > 500, (
        "most pages must be claimed by a manifest; if almost none are, the "
        "parser is broken rather than the tree"
    )


def unmanifested_sheets() -> set[str]:
    module = load()
    if not (module.ASSETS / "sprites").is_dir():
        pytest.skip("the actor-monolith sprite tree is absent")
    return {
        row["path"].split("/")[-1][: -len("_spritesheet.png")]
        for row in module.census()["sheets_without_manifest"]
    }


@CANONICAL
def test_no_new_sheet_ships_without_a_manifest():
    new = unmanifested_sheets() - KNOWN_UNMANIFESTED_SHEETS
    assert not new, (
        "a `<base>_spritesheet.png` ships with no `<base>_spritesheet.ron` "
        "beside it, so build.rs bakes no spec for it and no loader can reach "
        f"it: {sorted(new)}. Generate the manifest, or record why the render "
        "is kept unmanifested."
    )


@CANONICAL
def test_the_unmanifested_list_does_not_rot():
    fixed = KNOWN_UNMANIFESTED_SHEETS - unmanifested_sheets()
    assert not fixed, (
        f"{sorted(fixed)} now ship a manifest — remove them from "
        "KNOWN_UNMANIFESTED_SHEETS so a regression is caught"
    )


def test_build_rs_still_bakes_portrait_manifests_from_full_resolution_only():
    """⛔⛔ THE FACT THE PORTRAIT BUCKET RESTS ON, PINNED AT ITS SOURCE.

    `reduced_tier_portraits` calls 487 files unreachable purely because
    `bake_portrait_manifests` collects from `assets/sprites` and never from the
    reduced tier dirs. If that changes, the bucket silently becomes a list of
    perfectly live files — and nothing else in this suite would notice, because
    the assets themselves would not move.

    This reads committed Rust, so it runs on every machine.
    """
    build_rs = REPO / "crates/ambition_sprite_sheet/build.rs"
    if not build_rs.exists():
        pytest.skip("ambition_sprite_sheet/build.rs is absent")
    text = build_rs.read_text()
    start = text.index("fn bake_portrait_manifests")
    body = text[start : text.index("\nfn ", start + 1)]
    assert len(body) > 200, "premise: a real function body was read, not an empty slice"
    assert 'let sprites_dir = assets_dir.join("sprites");' in body, (
        "the portrait baker no longer roots at the full-resolution sprites dir; "
        "re-check `reduced_tier_portraits`, which calls reduced-tier portraits "
        "unreachable ONLY because this scan never sees them"
    )
    for tier in ["sprites_0_5x", "sprites_0_25x", "sprites_potato"]:
        assert tier not in body, (
            f"the portrait baker now scans {tier}, so reduced-tier portraits "
            "are reachable and the census bucket is wrong"
        )


# ── The SCRIPT's behaviour, on fixtures, unconditionally ──────────────────


def _png(path: Path, size: int = 32) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + b"x" * size)
    return path


def test_a_single_page_manifest_strands_its_numbered_siblings(tmp_path):
    """The real shape: `image: "x.png"` with `x.1.png` beside it. All four
    sheets in the tree today are this, not a shrunken `images:` list."""
    module = load()
    tier = tmp_path / "sprites"
    _png(tier / "hero_spritesheet.png")
    _png(tier / "hero_spritesheet.1.png")
    _png(tier / "hero_spritesheet.2.png")
    (tier / "hero_spritesheet.ron").write_text('(image: "hero_spritesheet.png")')

    pngs, claimed = module.scan(tmp_path, ["sprites"])
    assert len(pngs) == 3
    stranded = module.stranded_pages(pngs, claimed)
    assert sorted(p.name for p in stranded) == [
        "hero_spritesheet.1.png",
        "hero_spritesheet.2.png",
    ]


def test_a_listed_page_is_not_stranded(tmp_path):
    module = load()
    tier = tmp_path / "sprites"
    _png(tier / "hero_spritesheet.png")
    _png(tier / "hero_spritesheet.1.png")
    (tier / "hero_spritesheet.ron").write_text(
        '(images: ["hero_spritesheet.png", "hero_spritesheet.1.png"])'
    )
    pngs, claimed = module.scan(tmp_path, ["sprites"])
    assert module.stranded_pages(pngs, claimed) == []


def test_a_numbered_page_with_no_manifest_is_not_called_stranded(tmp_path):
    """⛔ CONFIDENCE DISCIPLINE. If the `.ron` is gone the whole sheet went
    away, which is a different story — it belongs in the weaker bucket where a
    reader will not treat it as proven."""
    module = load()
    tier = tmp_path / "sprites"
    _png(tier / "ghost_spritesheet.1.png")
    pngs, claimed = module.scan(tmp_path, ["sprites"])
    assert module.stranded_pages(pngs, claimed) == []


def test_a_sheet_with_no_ron_is_reported_and_one_with_a_ron_is_not(tmp_path):
    """⭐ The distinction the bucket rests on. `build.rs` bakes the spec index
    from `*_spritesheet.ron` on disk, so the manifest's presence is the whole
    question — not whether the PNG looks like a sheet."""
    module = load()
    tier = tmp_path / "sprites"
    _png(tier / "orphan_spritesheet.png")
    _png(tier / "kept_spritesheet.png")
    (tier / "kept_spritesheet.ron").write_text('(image: "kept_spritesheet.png")')

    pngs, claimed = module.scan(tmp_path, ["sprites"])
    found = module.sheets_without_manifest(pngs, claimed)
    assert [p.name for p in found] == ["orphan_spritesheet.png"]


def test_a_manifest_that_names_another_page_still_counts_as_a_manifest(tmp_path):
    """⛔⛔ THE DISCRIMINATING CASE, AND THE OTHER FIXTURES DO NOT PROVIDE IT.

    In every natural fixture "unclaimed" and "has no .ron" coincide, so a
    `sheets_without_manifest` that ignored the `.ron` entirely still passed
    them — I poisoned it exactly that way and got six green. This is the only
    shape that separates the two conditions: a sheet PNG its own manifest does
    NOT name, so it is unclaimed, while a manifest for it plainly exists.

    It must NOT be reported here: `build.rs` bakes a spec from that `.ron`, so
    the "no spec exists" claim this bucket rests on is false for it. It belongs
    in the weaker bucket instead.
    """
    module = load()
    tier = tmp_path / "sprites"
    _png(tier / "lonely_spritesheet.png")
    _png(tier / "lonely_spritesheet.0.png")
    (tier / "lonely_spritesheet.ron").write_text('(image: "lonely_spritesheet.0.png")')

    pngs, claimed = module.scan(tmp_path, ["sprites"])
    assert module.key(tier / "lonely_spritesheet.png") not in claimed, (
        "premise: the manifest does not name this page, so it is unclaimed"
    )
    assert module.sheets_without_manifest(pngs, claimed) == [], (
        "a manifest exists for this sheet, so the no-spec claim does not apply"
    )


def test_a_claimed_sheet_is_never_called_unmanifested(tmp_path):
    """⛔ A manifest may name a page that is not its own basename. Claimed wins
    over the filename heuristic, or a legitimately-shared page reads as orphaned."""
    module = load()
    tier = tmp_path / "sprites"
    _png(tier / "shared_spritesheet.png")
    (tier / "other_spritesheet.ron").write_text('(image: "shared_spritesheet.png")')
    pngs, claimed = module.scan(tmp_path, ["sprites"])
    assert module.sheets_without_manifest(pngs, claimed) == []


def test_reduced_tier_portraits_are_counted_even_when_a_ron_claims_them(tmp_path):
    """⛔ CLAIMEDNESS IS THE WRONG QUESTION FOR PORTRAITS, and this is the case
    that proves it: a reduced tier that DOES carry a `_portraits.ron` still has
    no baked manifest, because the baker never scans that directory. Filtering
    this bucket by claimedness — or by a sibling `.ron` — understates it by 34
    files."""
    module = load()
    _png(tmp_path / "sprites" / "alice_portraits.png")
    _png(tmp_path / "sprites_0_5x" / "alice_portraits.png")
    (tmp_path / "sprites_0_5x" / "alice_portraits.ron").write_text(
        '(image: "alice_portraits.png")'
    )
    pngs, claimed = module.scan(tmp_path, ["sprites", "sprites_0_5x"])
    assert module.key(tmp_path / "sprites_0_5x" / "alice_portraits.png") in claimed, (
        "premise: a .ron here does mark the PNG claimed, which is why a bucket "
        "that honours claimedness would silently drop it"
    )
    found = module.reduced_tier_portraits(pngs, tmp_path)
    assert [str(p.relative_to(tmp_path)) for p in found] == [
        "sprites_0_5x/alice_portraits.png"
    ], "the full-resolution portrait is reachable; the reduced one is not"


def test_the_age_signal_separates_stale_from_still_produced(tmp_path):
    """⭐ THE DISTINCTION THE BUCKETS COULD NOT MAKE ON THEIR OWN. "Left by an
    earlier render" and "the pipeline makes this every time" look identical in a
    file listing, and they have different fixes: a clean regen elsewhere removes
    the first and REPRODUCES the second. Measured 2026-09-02, every stranded page
    predates its manifest (44/44) while the unmanifested sheets and reduced-tier
    portraits are mostly same-run — so yardrat's regen settles one bucket and
    cannot settle the other two.
    """
    module = load()
    old_file = tmp_path / "old.png"
    new_file = tmp_path / "new.png"
    reference = tmp_path / "ref.ron"
    old_file.write_bytes(b"x")
    reference.write_bytes(b"x")
    new_file.write_bytes(b"x")
    import os

    os.utime(old_file, (0, reference.stat().st_mtime - 86400))
    os.utime(new_file, (0, reference.stat().st_mtime + 86400))

    signal = module.age_signal([old_file, new_file], lambda _p: reference)
    assert signal["comparable"] == 2
    assert signal["older_than_reference"] == 1
    assert signal["same_run_or_newer"] == 1


def test_the_age_signal_reports_zero_comparable_rather_than_a_verdict(tmp_path):
    """⛔ A missing reference must not read as 'not stale'. With nothing to
    compare against, the honest output is `comparable: 0` and no counts — a
    bucket silently scored 0-older/0-newer would print STILL PRODUCED."""
    module = load()
    orphan = tmp_path / "a.png"
    orphan.write_bytes(b"x")
    assert module.age_signal([orphan], lambda _p: None) == {"comparable": 0}
    assert module.age_signal([orphan], lambda _p: tmp_path / "gone.ron") == {
        "comparable": 0
    }


def test_key_does_not_resolve_through_a_symlink(tmp_path):
    """⛔⛔ THE WORKTREE HAZARD. `Path.resolve()` on a mirrored asset returns
    the MAIN checkout's path, so a census that resolves compares another tree
    without saying so. This pins that identity stays inside the given tree."""
    module = load()
    shared = tmp_path / "main.png"
    shared.write_bytes(b"x")
    link = tmp_path / "worktree.png"
    link.symlink_to(shared)
    assert module.key(link) != module.key(shared)
    assert link.resolve() == shared.resolve(), (
        "premise: resolve() really does collapse the two, which is why key() "
        "must not use it"
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
