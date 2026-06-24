#!/usr/bin/env python3
"""Canonical scratch layout for the PCA work, so progress is trackable.

    agent-scratch/
      inputs/                     stable reference material (don't churn)
        reference_full.png        original 1448 concept sheet
        segment_raw.png           Jon's manual segmentation (transparent bg)
        segment_clean.png         cleaned segmentation
        refs/<pose>.png           10 per-pose clean crops == the EVAL TARGETS
        palette.json              shared k-means palette
      versions/<NN_name>/         one folder per tactic, SAME structure:
        cand/<pose>.png             the candidate render, per pose (eval frame)
        eval/metrics.json           standard metrics
        eval/montage.png            ref | candidate | diff, all poses (the diff!)
        eval/<pose>.png             ref | candidate | diff, per pose
        notes.md                    what this tactic is
      diagnostics/                ad-hoc one-off analyses
      LATEST_*.png                current representative(s)

Every tactic drops per-pose candidate PNGs into its ``cand/`` (aligned to the
ref crops) and runs ``pca_eval`` -> identical diagnostics for all versions.
"""
from __future__ import annotations

from pathlib import Path

SCRATCH = Path(__file__).resolve().parents[3] / "agent-scratch"
INPUTS = SCRATCH / "inputs"
REFS = INPUTS / "refs"
PALETTE_JSON = INPUTS / "palette.json"
VERSIONS = SCRATCH / "versions"
DIAGNOSTICS = SCRATCH / "diagnostics"

POSES = ["top_front", "top_side", "top_back", "pose_idle", "pose_walk_1",
         "pose_walk_2", "pose_attack", "pose_jump", "pose_air", "pose_land"]


def version_dir(name: str) -> Path:
    d = VERSIONS / name
    (d / "cand").mkdir(parents=True, exist_ok=True)
    (d / "eval").mkdir(parents=True, exist_ok=True)
    return d
