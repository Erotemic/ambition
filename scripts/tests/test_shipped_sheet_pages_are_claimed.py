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
