#!/usr/bin/env python3
"""Pixel-parity harness for the sprite renderer refactor.

The refactor (see docs/planning/sprite-renderer-refactor.md) consolidates a
sprawling renderer behind one small core. To do that safely we pin the *output
pixels* of every target and re-check them after each change.

Policy (per Jon): pixel drift is NOT a hard failure — small drift is acceptable,
especially when correct behaviour starts to emerge. So `check` does not fail on
drift by default; instead it dumps before/after PNGs (and a side-by-side
comparison) into `<repo>/tmp/sprite-drift/` so the drift can be eyeballed and
either blessed or fixed. Use `--strict` to make drift exit non-zero (CI).

Usage (from tools/ambition_sprite2d_renderer/, deps: Pillow + stdlib):

    PYTHONPATH=. python3 parity_harness.py capture            # snapshot current pixels
    PYTHONPATH=. python3 parity_harness.py check              # compare; dump drift to tmp/
    PYTHONPATH=. python3 parity_harness.py check --targets alice,boss
    PYTHONPATH=. python3 parity_harness.py check --strict     # non-zero exit on drift

The baseline lives in `.parity-baseline/` (gitignored). Capture it BEFORE
starting a behaviour-preserving change, then `check` as you refactor.
"""
from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import shutil
import sys
import tempfile
import time
import traceback
from pathlib import Path
from typing import Dict, List, Optional, Tuple

PKG_DIR = Path(__file__).resolve().parent
REPO_ROOT = PKG_DIR.parents[1]
BASELINE_DIR = PKG_DIR / ".parity-baseline"
DRIFT_DIR = REPO_ROOT / "tmp" / "sprite-drift"


def _discover() -> Dict[str, object]:
    from ambition_sprite2d_renderer.registry import discover_all_targets

    return discover_all_targets().targets


# Output kinds we pin: PNG pixels AND the RON/YAML manifests (so changes to
# measured metadata or the emitter are caught, not just pixel changes).
OUTPUT_GLOBS = ("*.png", "*.ron", "*.yaml")


def render_target_files(target) -> Dict[str, bytes]:
    """Render one target's sheet(s) to a temp dir; return {relpath: bytes}.

    Keyed by path relative to the render dir so multi-file targets (bosses)
    compare file-by-file. Raises on render failure (caller records it).
    """
    out = Path(tempfile.mkdtemp(prefix="parity_"))
    try:
        target.render_sheet(out)
        result: Dict[str, bytes] = {}
        for pat in OUTPUT_GLOBS:
            for f in sorted(out.rglob(pat)):
                result[str(f.relative_to(out))] = f.read_bytes()
        return result
    finally:
        shutil.rmtree(out, ignore_errors=True)


def _select(targets: Dict[str, object], spec: Optional[str]) -> List[str]:
    if not spec:
        return sorted(targets)
    names = [s.strip() for s in spec.split(",") if s.strip()]
    missing = [n for n in names if n not in targets]
    if missing:
        print(f"!! unknown targets: {', '.join(missing)}", file=sys.stderr)
    return [n for n in names if n in targets]


def cmd_capture(args) -> int:
    targets = _discover()
    names = _select(targets, args.targets)
    if BASELINE_DIR.exists() and not args.targets:
        shutil.rmtree(BASELINE_DIR)
    BASELINE_DIR.mkdir(parents=True, exist_ok=True)
    ok = err = 0
    t0 = time.time()
    for i, name in enumerate(names, 1):
        try:
            files = render_target_files(targets[name])
        except Exception:
            err += 1
            print(f"[{i}/{len(names)}] ERROR {name}")
            (BASELINE_DIR / name).mkdir(parents=True, exist_ok=True)
            (BASELINE_DIR / name / "render_error.txt").write_text(traceback.format_exc())
            continue
        tdir = BASELINE_DIR / name
        tdir.mkdir(parents=True, exist_ok=True)
        hashes = {}
        for rel, data in files.items():
            dest = tdir / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(data)
            hashes[rel] = hashlib.sha256(data).hexdigest()
        (tdir / "hashes.json").write_text(json.dumps(hashes, indent=2, sort_keys=True))
        ok += 1
        if i % 20 == 0 or i == len(names):
            print(f"[{i}/{len(names)}] captured… ({time.time()-t0:.0f}s)")
    print(f"\ncaptured {ok} targets, {err} render errors → {BASELINE_DIR}")
    return 0


def _side_by_side(before: Optional[Path], after: Optional[Path], dest: Path) -> None:
    """Write a before|after(|diff) comparison image for quick eyeballing."""
    try:
        from PIL import Image, ImageChops, ImageDraw
    except Exception:
        return
    pad, label_h, bg = 8, 16, (32, 34, 40, 255)
    imgs: List[Tuple[str, Optional[Image.Image]]] = []
    b = Image.open(before).convert("RGBA") if before and before.exists() else None
    a = Image.open(after).convert("RGBA") if after and after.exists() else None
    imgs.append(("before", b))
    imgs.append(("after", a))
    if b is not None and a is not None and b.size == a.size:
        diff = ImageChops.difference(b.convert("RGB"), a.convert("RGB"))
        imgs.append(("diff", diff.convert("RGBA")))
    h = max([im.height for _, im in imgs if im is not None] + [1])
    w = sum((im.width if im is not None else 32) + pad for _, im in imgs) + pad
    canvas = Image.new("RGBA", (w, h + label_h + pad), bg)
    d = ImageDraw.Draw(canvas)
    x = pad
    for label, im in imgs:
        d.text((x, 2), label, fill=(220, 224, 235, 255))
        if im is not None:
            canvas.alpha_composite(im, (x, label_h))
            x += im.width + pad
        else:
            d.text((x, label_h), "(none)", fill=(230, 120, 120, 255))
            x += 32 + pad
    dest.parent.mkdir(parents=True, exist_ok=True)
    canvas.convert("RGBA").save(dest)


def _dump_drift(
    name: str, rel: str, before_path: Optional[Path], after_bytes: Optional[bytes]
) -> None:
    """Write before/after artifacts for one drifted file into tmp/sprite-drift.

    PNGs get a side-by-side compare image; RON/YAML manifests get a unified
    text diff — whichever is easiest to eyeball for that kind.
    """
    dest_dir = DRIFT_DIR / name
    dest_dir.mkdir(parents=True, exist_ok=True)
    ext = Path(rel).suffix
    flat = rel.replace("/", "__")
    before_dest = after_dest = None
    if before_path is not None and before_path.exists():
        before_dest = dest_dir / (flat + ".before" + ext)
        shutil.copyfile(before_path, before_dest)
    if after_bytes is not None:
        after_dest = dest_dir / (flat + ".after" + ext)
        after_dest.write_bytes(after_bytes)
    if ext == ".png":
        _side_by_side(before_dest, after_dest, dest_dir / (flat + ".compare.png"))
        return
    before_text = (
        before_path.read_text().splitlines()
        if before_path is not None and before_path.exists()
        else []
    )
    after_text = (
        after_bytes.decode("utf-8", "replace").splitlines()
        if after_bytes is not None
        else []
    )
    diff = difflib.unified_diff(
        before_text, after_text, fromfile=f"before/{rel}", tofile=f"after/{rel}", lineterm=""
    )
    (dest_dir / (flat + ".diff")).write_text("\n".join(diff))


def cmd_check(args) -> int:
    if not BASELINE_DIR.exists():
        print("no baseline — run `capture` first", file=sys.stderr)
        return 2
    targets = _discover()
    names = _select(targets, args.targets)
    if not args.targets and DRIFT_DIR.exists():
        shutil.rmtree(DRIFT_DIR)
    drifted: List[str] = []
    errored: List[str] = []
    missing_baseline: List[str] = []
    clean = 0
    for name in names:
        bdir = BASELINE_DIR / name
        if not bdir.exists() or not (bdir / "hashes.json").exists():
            missing_baseline.append(name)
            continue
        try:
            cur = render_target_files(targets[name])
        except Exception:
            errored.append(name)
            continue
        base_hashes = json.loads((bdir / "hashes.json").read_text())
        cur_hashes = {rel: hashlib.sha256(d).hexdigest() for rel, d in cur.items()}
        if cur_hashes == base_hashes:
            clean += 1
            continue
        drifted.append(name)
        rels = sorted(set(base_hashes) | set(cur_hashes))
        for rel in rels:
            if base_hashes.get(rel) == cur_hashes.get(rel):
                continue
            _dump_drift(name, rel, bdir / rel if rel in base_hashes else None,
                        cur.get(rel))

    print(f"\nparity check: {clean} clean, {len(drifted)} drifted, "
          f"{len(errored)} errors, {len(missing_baseline)} missing-baseline")
    if drifted:
        print("  drifted: " + ", ".join(drifted))
        print(f"  → before/after/compare written to {DRIFT_DIR}")
    if errored:
        print("  errors:  " + ", ".join(errored))
    if missing_baseline and args.targets:
        print("  no baseline for: " + ", ".join(missing_baseline))
    bad = (drifted or errored) if args.strict else False
    return 1 if bad else 0


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    sub = p.add_subparsers(dest="cmd", required=True)
    pc = sub.add_parser("capture", help="snapshot current pixels as the baseline")
    pc.add_argument("--targets", help="comma-separated subset (default: all)")
    pc.set_defaults(func=cmd_capture)
    pk = sub.add_parser("check", help="compare to baseline; dump drift to tmp/")
    pk.add_argument("--targets", help="comma-separated subset (default: all)")
    pk.add_argument("--strict", action="store_true", help="non-zero exit on drift/error")
    pk.set_defaults(func=cmd_check)
    args = p.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
