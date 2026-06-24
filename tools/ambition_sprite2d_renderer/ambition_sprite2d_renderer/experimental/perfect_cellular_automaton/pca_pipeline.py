#!/usr/bin/env python3
"""End-to-end PCA polygon-fit pipeline (reproducible from committed inputs).

    v14 polygon JSON  --(palette fix + motif layer)-->  optimize per pose
                      --(stamp back/side spots)-->  final sheet + IoU report

Everything is derived from the committed
``perfect_cellular_automaton_pose_polygons_v14.json`` plus the reference image;
all outputs go to an out-dir (default: the repo's gitignored agent-scratch).

    python -m ...experimental.perfect_cellular_automaton.pca_pipeline \
        --out-dir <dir>            # full run (~20 min: 10 poses)
        --poses pose_jump          # just one pose
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import pca_fit as F
import pca_finalize  # noqa: F401  (imported for side-effect parity / reuse)
import pca_detect_spots as D
from PIL import Image

HERE = Path(__file__).resolve().parent
DEFAULT_DATA = HERE / "perfect_cellular_automaton_pose_polygons_v14.json"
DEFAULT_REF = Path("/home/joncrall/code/ambition/assets/concept_art/"
                   "prefect-cellular-automaton-reference-image.png")


def add_spots(geoms, ref, specs):
    for name in ("top_back", "top_side"):
        for r in D.detect(ref, specs[name]["roi"])[:6]:
            gx0, gy0, gx1, gy1 = r["global_box"]
            import numpy as np
            pts = np.asarray([[gx0, gy0], [gx1, gy0], [gx1, gy1], [gx0, gy1]], np.float32)
            geoms[name].polys.append(F.Poly("dark_green", False, pts, locked=True))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", type=Path, default=DEFAULT_DATA)
    ap.add_argument("--ref", type=Path, default=DEFAULT_REF)
    ap.add_argument("--out-dir", type=Path, required=True)
    ap.add_argument("--poses", nargs="*", default=None)
    ap.add_argument("--seed", type=int, default=0)
    args = ap.parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)

    fit_json = args.out_dir / "pca_v15_fit.json"
    fit_png = args.out_dir / "pca_v15_fit.png"
    final_json = args.out_dir / "pca_v15_final.json"
    final_png = args.out_dir / "pca_v15_final.png"

    # 1. optimize (palette fix + motif applied inside pca_fit.run)
    F.run(args.data, args.ref, fit_json, fit_png, poses=args.poses, seed=args.seed)

    # 2. stamp static detail (back/side carapace spots) as a locked layer
    data = json.loads(fit_json.read_text())
    geoms = F.load_geom_v15(data)
    ref = Image.open(args.ref).convert("RGB")
    specs = json.loads((HERE / "pca_roi_specs_v14.json").read_text())["rois"]
    add_spots(geoms, ref, specs)
    F.save_geom_v15(geoms, data["palette"], data.get("meta", {}), final_json)
    F.render_sheet(geoms, data["palette"]).save(final_png)
    print(f"wrote {final_json}\nwrote {final_png}")


if __name__ == "__main__":
    main()
