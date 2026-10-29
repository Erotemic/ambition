#!/usr/bin/env python3
"""Check accepted quality conversions with the publication owner's decision."""
from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools/ambition_sprite2d_renderer'))
from ambition_sprite2d_renderer.products import Digests, conversion_status

SPRITE_TIERS = ('sprites_0_5x', 'sprites_0_25x', 'sprites_potato')
PARALLAX_TIERS = ('parallax_layers_0_5x', 'parallax_layers_0_25x', 'parallax_layers_potato')


def page_names(manifest: Path) -> set[str]:
    from ambition_sprite2d_renderer.quality import page_filenames_safe
    return set(page_filenames_safe(manifest))


def main() -> int:
    if not os.environ.get("AMBITION_QUALITY_CHECK_OWNER"):
        selected = subprocess.run(["bash", "-c", 'source "$1/scripts/lib/tool_python.sh"; ambition_select_tool_python "$2" AMBITION_SPRITE_PYTHON',
            "_", str(ROOT), str(ROOT / "tools/ambition_sprite2d_renderer")], capture_output=True, text=True)
        python = selected.stdout.strip()
        if not python or selected.returncode:
            print("The renderer interpreter is unavailable: " + selected.stderr, file=sys.stderr)
            return 2
        return subprocess.call([python, __file__, *sys.argv[1:]], env=dict(os.environ, AMBITION_QUALITY_CHECK_OWNER="1"))
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--library', type=Path, help='selected content library manifest')
    parser.add_argument('--asset-root', type=Path, default=None, help='override background conversion root')
    parser.add_argument('--require-evidence', action='store_true', help='fail if a conversion has no accepted receipt')
    args = parser.parse_args()
    from ambition_sprite2d_renderer.content_library import runtime_root, quality_root, read
    if args.library:
        os.environ["AMBITION_CONTENT_LIBRARY"] = str((ROOT / args.library).resolve())
    override = args.asset_root is not None
    args.asset_root = args.asset_root or runtime_root()
    hashes, failures, unknown, checked = Digests(), [], [], 0
    producers = read().get("producers", {})
    if "sprites" in producers:
        from ambition_sprite2d_renderer.publication_store import Store
        from ambition_sprite2d_renderer.publication import store_root
        store = Store(store_root())
        from ambition_sprite2d_renderer.publication import source_files, toolchain, recipe_state
        sources, chain = source_files(), toolchain()
        selected = store.selected()
        if not selected["generation"]:
            failures.append("No accepted art publication. Run scripts/regen/sprites.sh.")
        for unit, key in selected["bundles"].items():
            manifest = store.bundle(key)
            checked += 1
            failures += [f"{unit}: {defect}" for defect in store.integrity(key)]
            if manifest["recipe"] is None:
                unknown.append(unit)
            else:
                state, reasons = recipe_state(manifest["recipe"], sources, chain, hashes)
                if state != "current":
                    failures.append(f"{unit}: {state}: {reasons}")
                if manifest["recipe"].get("quality") is False:
                    failures.append(f"{unit}: quality representations were not generated")
            if unit.startswith("target:"):
                supported = (manifest["recipe"] or {}).get("tiers", ["full", "half", "quarter", "potato"])
                for tier in SPRITE_TIERS if len(supported) > 1 else ():
                    if not any(path.startswith(tier + "/") for path in manifest["files"]):
                        failures.append(f"{unit}: missing {tier}")
    for source, tier_names in (((args.asset_root / 'backgrounds/parallax_layers', PARALLAX_TIERS),) if 'backgrounds' in producers else ()):
        manifests = list(source.rglob('*_spritesheet.ron'))
        metadata = manifests + list(source.rglob('*_parts.ron')) + list(source.rglob('*_portraits.ron'))
        claimed = {path.parent / name for path in metadata for name in page_names(path)}
        loose = [path for path in source.rglob('*.png') if path not in claimed and '_spritesheet' not in path.name and not path.name.endswith(('_portraits.png','_parts.png','_canonical.png','_canonical_transparent.png','_preview_labeled.png','_debug.png'))]
        for tier_name in tier_names:
            suffix = tier_name.removeprefix("parallax_layers_")
            tier = source.parent / tier_name if override else quality_root(suffix) / "backgrounds" / tier_name
            if not tier.is_dir():
                failures.append(f'Missing tier root: {tier}')
                continue
            for path in manifests + loose:
                published = tier / path.relative_to(source)
                checked += 1
                if not published.is_file():
                    failures.append(f'Missing: {published}')
                    continue
                status = conversion_status(published.parent, published.stem, hashes)
                if status['state'] == 'unknown':
                    unknown.append(str(published))
                elif status['state'] != 'current':
                    failures.append(f"{status['state']}: {published}: {status['reasons']}")
    print(f'{checked} quality products checked; {len(failures)} defects; {len(unknown)} without complete conversion evidence.')
    for row in failures[:30]:
        print(row, file=sys.stderr)
    if failures or (args.require_evidence and unknown) or (not checked and ("sprites" in producers or "backgrounds" in producers)):
        print('Publish affected products with ./scripts/regen/quality_variants.sh --target NAME', file=sys.stderr)
        return 1
    if unknown:
        print('Source freshness is unknown for legacy conversions. Publish them to record evidence.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
