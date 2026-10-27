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
    from generate_visual_quality_variants import page_filenames_safe
    return set(page_filenames_safe(manifest))


def stale_pairs(source_dir: Path, tier_dir: Path) -> list[tuple[Path, str]]:
    """Check runtime sheet members. Optional sidecars cannot make a sheet stale."""
    hashes = Digests()
    result = []
    for manifest in sorted(tier_dir.rglob('*_spritesheet.ron')) if tier_dir.is_dir() else []:
        status = conversion_status(manifest.parent, manifest.stem, hashes)
        if status['state'] in ('stale', 'missing', 'partial'):
            result.append((manifest, status['state']))
    return result


def absent_variants(source_dir: Path, tier_dir: Path) -> list[Path]:
    return [path for path in sorted(source_dir.rglob('*_spritesheet.ron'))
            if not (tier_dir / path.relative_to(source_dir)).is_file()]


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
    parser.add_argument('--asset-root', type=Path, default=ROOT / 'crates/ambition_platformer2d_actor_monolith/assets')
    parser.add_argument('--require-evidence', action='store_true', help='fail if a conversion has no accepted receipt')
    args = parser.parse_args()
    hashes, failures, unknown, checked = Digests(), [], [], 0
    for source, tier_names in ((args.asset_root / 'sprites', SPRITE_TIERS), (args.asset_root / 'backgrounds/parallax_layers', PARALLAX_TIERS)):
        manifests = list(source.rglob('*_spritesheet.ron'))
        metadata = manifests + list(source.rglob('*_parts.ron')) + list(source.rglob('*_portraits.ron'))
        claimed = {path.parent / name for path in metadata for name in page_names(path)}
        loose = [path for path in source.rglob('*.png') if path not in claimed and '_spritesheet' not in path.name and not path.name.endswith(('_portraits.png','_parts.png','_canonical.png','_canonical_transparent.png','_preview_labeled.png','_debug.png'))]
        for tier_name in tier_names:
            tier = source.parent / tier_name
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
    if failures or (args.require_evidence and unknown) or not checked:
        print('Publish affected products with ./scripts/regen/quality_variants.sh --target NAME', file=sys.stderr)
        return 1
    if unknown:
        print('Source freshness is unknown for legacy conversions. Publish them to record evidence.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
