#!/usr/bin/env python3
"""Shipped sprite PNGs that nothing names.

`package_asset_guard.py` records "every regular file" from the asset roots, so
a PNG left in `assets/sprites*/` ships whether or not anything can load it.
Nothing decodes it — no manifest names it — so the cost is package size only,
not decode or residency. That is exactly why it is invisible: every runtime
measurement says the tree is fine.

TWO BUCKETS, AND THEY DO NOT DESERVE THE SAME CONFIDENCE
--------------------------------------------------------

⭐ STRANDED PAGES (high confidence). `<base>_spritesheet.<n>.png` sitting beside
a `<base>_spritesheet.ron` that does not name it. A sheet's pages are resolved
ONLY through its manifest — an `images:` list, or a single `image:` — so a
numbered page the manifest omits is unreachable by construction. There is no
other road to it.

⚠ ALL FOUR SHEETS THAT HAVE THESE TODAY ARE SINGLE-PAGE MANIFESTS WITH NO
`images:` LIST AT ALL. I first described this bucket as "a list that shrank",
which is not what the tree shows: the manifest names one image and the numbered
siblings are left from a time the sheet was multi-page. The reachability
conclusion is stronger that way, not weaker — there is no list to consult.

⭐ SHEETS WITH NO MANIFEST (high confidence). `<base>_spritesheet.png` with no
`<base>_spritesheet.ron` beside it. `ambition_sprite_sheet/build.rs` bakes the
spec index by scanning these four tier dirs for `*_spritesheet.ron`, and every
loader goes through a spec (`try_load_spec_for_target(target)?`), so a sheet
with no manifest has no spec and therefore no road. ⚠ A build made while the
`.ron` still existed would carry a stale embedded spec; this says what a fresh
build can reach.

⚠ UNMENTIONED FILES (upper bound only). Every other PNG under `sprites*/` whose
filename appears in no baked manifest and in no committed `.rs`/`.ron`/`.ldtk`/
`.toml`/`.json`/`.py`. A path assembled at runtime — `format!("sprites/{name}.png")`
— is named nowhere and would land here while being perfectly live. This bucket
is a research prompt, NOT a delete list.

⛔⛔ EVERYTHING HERE IS GITIGNORED, GENERATED, PER-MACHINE. `assets/sprites*/` is
generated output, and a worktree SYMLINKS the main checkout's copies
(`mirror_assets_for_worktree.py`) — so a second worktree agreeing is not a
second source, and `Path.resolve()` on any of these escapes the worktree
entirely. Whether these are stale outputs or a live defect in the generator is
what a clean regen on another machine decides. This script deletes nothing.

Usage:
    scripts/measure_orphan_shipped_pages.py
    scripts/measure_orphan_shipped_pages.py --json
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
ASSETS = REPO / "crates/ambition_platformer2d_actor_monolith/assets"
TIER_DIRS = ["sprites", "sprites_0_5x", "sprites_0_25x", "sprites_potato"]

IMAGES_LIST_RE = re.compile(r"images:\s*\[([^\]]*)\]")
IMAGE_RE = re.compile(r'(?<!s)image:\s*"([^"]+)"')
QUOTED_RE = re.compile(r'"([^"]+)"')
# `<base>_spritesheet.<n>.png` — the numbered sibling shape.
NUMBERED_PAGE_RE = re.compile(r"^(?P<base>.+_spritesheet)\.(?P<index>\d+)\.png$")


def key(path: Path) -> str:
    """⛔ NOT `resolve()`. A worktree's assets are symlinks into the main
    checkout, so resolving turns every path into main's and silently compares
    another tree. Normalising is enough: manifests name siblings, not links."""
    return os.path.normpath(str(path))


def manifest_pages(manifest: Path) -> list[str]:
    text = manifest.read_text(errors="ignore")
    listed = IMAGES_LIST_RE.search(text)
    names = QUOTED_RE.findall(listed.group(1)) if listed else []
    return names + IMAGE_RE.findall(text)


def scan(assets: Path, tiers: list[str]) -> tuple[list[Path], set[str]]:
    """Every PNG under the tier dirs, and the set of paths a manifest claims."""
    claimed: set[str] = set()
    pngs: list[Path] = []
    for tier in tiers:
        tier_dir = assets / tier
        if not tier_dir.is_dir():
            continue
        for manifest in tier_dir.rglob("*.ron"):
            for name in manifest_pages(manifest):
                claimed.add(key(manifest.parent / name))
        pngs.extend(sorted(tier_dir.rglob("*.png")))
    return pngs, claimed


def stranded_pages(pngs: list[Path], claimed: set[str]) -> list[Path]:
    """Numbered siblings of a manifest that exists and does not list them.

    The manifest must EXIST: a numbered page whose `.ron` was removed entirely
    is a different story (the whole sheet went away) and belongs in the weaker
    bucket, where a reader will not be tempted to treat it as proven.
    """
    out = []
    for png in pngs:
        if key(png) in claimed:
            continue
        match = NUMBERED_PAGE_RE.match(png.name)
        if match and (png.parent / f"{match.group('base')}.ron").exists():
            out.append(png)
    return out


def sheets_without_manifest(pngs: list[Path], claimed: set[str]) -> list[Path]:
    """`x_spritesheet.png` with no `x_spritesheet.ron` beside it.

    Distinct from a stranded page: there is no manifest at all, so the whole
    sheet is unreachable rather than one of its pages. `build.rs` bakes the
    spec index from the `.ron` files on disk, and every loader needs a spec.
    """
    out = []
    for png in pngs:
        if key(png) in claimed or not png.name.endswith("_spritesheet.png"):
            continue
        if not (png.parent / f"{png.name[:-4]}.ron").exists():
            out.append(png)
    return out


def unmentioned(pngs: list[Path], claimed: set[str], skip: set[str]) -> list[Path]:
    """Unclaimed PNGs whose filename appears in no committed source file.

    `git grep` over TRACKED files only: the tier dirs are gitignored, so a hit
    is a real declaration rather than one generated file naming another.
    """
    candidates = [p for p in pngs if key(p) not in claimed and key(p) not in skip]
    if not candidates:
        return []
    names = sorted({p.name for p in candidates})
    handle = tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False)
    handle.write("\n".join(names))
    handle.close()
    try:
        found = subprocess.run(
            ["git", "grep", "-hoF", "-f", handle.name, "--",
             "*.rs", "*.ron", "*.toml", "*.ldtk", "*.json", "*.py"],
            capture_output=True, text=True, cwd=REPO,
        ).stdout.split()
    finally:
        os.unlink(handle.name)
    declared = set(found)
    return [p for p in candidates if p.name not in declared]


def census(assets: Path = ASSETS, tiers: list[str] | None = None) -> dict:
    pngs, claimed = scan(assets, tiers or TIER_DIRS)
    stranded = stranded_pages(pngs, claimed)
    unmanifested = sheets_without_manifest(pngs, claimed)
    seen = {key(p) for p in stranded} | {key(p) for p in unmanifested}
    rest = unmentioned(pngs, claimed, seen)
    def rows(paths):
        return [
            {"path": str(p.relative_to(assets)), "bytes": p.stat().st_size}
            for p in sorted(paths, key=lambda q: -q.stat().st_size)
        ]
    return {
        "total_pngs": len(pngs),
        "claimed": len([p for p in pngs if key(p) in claimed]),
        "stranded_pages": rows(stranded),
        "sheets_without_manifest": rows(unmanifested),
        "unmentioned": rows(rest),
    }


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args(argv)

    if not (ASSETS / "sprites").is_dir():
        print(
            "NO `assets/sprites/` IN THIS CHECKOUT. The sprite tree is gitignored\n"
            "generated output; run the regen before reading this as 'no orphans'.\n"
            "⛔ Absent is not zero."
        )
        return 2

    out = census()
    if args.json:
        print(json.dumps(out, indent=2))
        return 0

    def show(title, rows, note):
        total = sum(r["bytes"] for r in rows)
        print(f"\n=== {title}: {len(rows)} file(s), {total / 1e6:.1f} MB ===")
        print(note)
        for row in rows[:10]:
            print(f"   {row['bytes'] / 1e6:7.2f} MB  {row['path']}")
        if len(rows) > 10:
            print(f"   ... and {len(rows) - 10} more")

    print(f"{out['total_pngs']} PNG(s) under {'/'.join(TIER_DIRS)}; "
          f"{out['claimed']} claimed by a baked manifest.")
    show(
        "STRANDED PAGES", out["stranded_pages"],
        "⭐ A numbered sibling its own manifest does not name. A sheet's pages\n"
        "   resolve only through its manifest, so there is no other road to\n"
        "   these — unreachable by construction.",
    )
    show(
        "SHEETS WITH NO MANIFEST", out["sheets_without_manifest"],
        "⭐ No `<base>_spritesheet.ron` beside it. build.rs bakes the spec index\n"
        "   from the .ron files on disk and every loader needs a spec, so a fresh\n"
        "   build has no road to these at all.",
    )
    show(
        "UNMENTIONED", out["unmentioned"],
        "⚠ UPPER BOUND. Named in no manifest and in no committed source, but a\n"
        "   runtime-assembled path is also named nowhere. A research prompt, not\n"
        "   a delete list.",
    )
    print(
        "\n⛔ These are gitignored generated files, and this is ONE machine's\n"
        "   tree — a worktree symlinks the main checkout's copies. Whether they\n"
        "   are stale outputs or a live generator defect is what a clean regen\n"
        "   elsewhere decides. Nothing here is deleted."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
