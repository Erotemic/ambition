#!/usr/bin/env python3
"""Check a publish batch through the renderer's accepted generation receipts."""
from __future__ import annotations
import argparse
import os
import sys
from pathlib import Path
from typing import Dict, List, Optional


def records_of(tsv: Optional[Path]) -> Dict[str, List[str]]:
    out: Dict[str, List[str]] = {}
    if tsv is None:
        return out
    for line in tsv.read_text().splitlines():
        target, _, rel = line.partition("\t")
        if target:
            out.setdefault(target, []).append(rel)
    return out


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--renderer-dir", type=Path, default=Path.cwd())
    parser.add_argument("--cache-dir", type=Path, required=True)
    parser.add_argument("--sprites-dir", type=Path, required=True)
    parser.add_argument("--installs", type=Path, default=None)
    parser.add_argument("--portraits", type=Path, default=None)
    parser.add_argument("--portraits-failed", action="store_true", help="the portrait records could not be resolved")
    parser.add_argument("--force", action="store_true")
    parser.add_argument("targets", nargs="*")
    args = parser.parse_args(argv)

    installs = records_of(args.installs)
    portraits = records_of(args.portraits)
    sys.path.insert(0, str(args.renderer_dir))
    from ambition_sprite2d_renderer.products import Digests, evaluate, runtime_required
    from ambition_sprite2d_renderer.provenance import read_record
    record_root = Path(os.environ.get("AMBITION_ART_PROVENANCE_DIR", Path(os.environ.get("AMBITION_ART_PROVENANCE_ROOT", args.sprites_dir.parent.parent / "asset_provenance")) / "sprites"))
    digests = Digests(args.cache_dir / "art_inputs.json", deep=True)
    for target in args.targets:
        plan = {"root": "sprites", "required": sorted({p for p in installs.get(target, []) + portraits.get(target, []) if runtime_required(p)}),
                "tiers": [], "repair": f"./scripts/regen/sprites.sh --target {target}"}
        status = evaluate(plan, read_record(record_root / f"{target}.json"), args.sprites_dir, args.repo_root, digests, tiers=False)
        fresh = not args.force and not args.portraits_failed and status["state"] == "current"
        print(f"{'fresh' if fresh else 'stale'}\t{target}")
    digests.save()
    return 0


if __name__ == "__main__":
    sys.exit(main())
