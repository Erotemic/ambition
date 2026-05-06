#!/usr/bin/env python3
"""Internal isolated worker for one MusicIR stem.

This keeps long production renders robust by resetting Python/SciPy/FFmpeg state
between stems. It is intentionally a small command invoked by render_isolated.py.
"""
from __future__ import annotations
import argparse, json, math, os, sys, tempfile
from pathlib import Path
import numpy as np
import yaml
from . import musicir_renderer as r


def main(argv=None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("spec")
    ap.add_argument("--outdir", required=True)
    ap.add_argument("--group", required=True)
    ap.add_argument("--backend", default="fast")
    ap.add_argument("--simple-post", action="store_true", help="Use cheap gain/limit only for this stem")
    ns = ap.parse_args(argv)
    spec_path = Path(ns.spec)
    spec = yaml.safe_load(spec_path.read_text())
    render_cfg = spec.get("render", {})
    sr = int(render_cfg.get("sample_rate", 48000))
    bpm = float(spec.get("tempo", {}).get("bpm", spec.get("bpm", 120)))
    soundfont = r.choose_soundfont(render_cfg.get("soundfont"))
    cue_hash = r.spec_hash(spec_path, soundfont, ns.backend)
    quality = float(render_cfg.get("ogg_quality", 5.0))
    pm, groups, meta = r.build_score(spec)
    total = meta[-1]["end_seconds"]
    target = int(math.ceil(total * sr))
    outdir = Path(ns.outdir)
    with tempfile.TemporaryDirectory() as td:
        raw = r.render_group_audio(pm, groups, ns.group, ns.backend, soundfont, sr, Path(td), total, bpm)
        raw = r.ensure_audio_length(raw, target)
        settings = dict(spec.get("stem_postprocess", {}) or {})
        settings.update((spec.get("group_postprocess", {}) or {}).get(ns.group, {}))
        settings.setdefault("normalize", False)
        settings.setdefault("target_peak_db", -2.5)
        if ns.simple_post:
            # Deprecated compatibility flag. The old path only honored gain_db,
            # which made YAML EQ/reverb/transient settings silently dead.
            settings.setdefault("simple_post_compat", True)
        audio = r.post_process(raw, sr, settings)
    npy = outdir / "debug_stems" / f"{spec['id']}_{cue_hash}.{ns.group}.npy"
    npy.parent.mkdir(parents=True, exist_ok=True)
    np.save(npy, audio.astype("float32"))
    files = {}
    for sec in meta:
        piece = r.slice_audio(audio, sr, sec["start_seconds"], sec["end_seconds"])
        path = outdir / "adaptive" / sec["id"] / f"{spec['id']}_{cue_hash}.{sec['id']}.{ns.group}.ogg"
        r.write_ogg_from_audio(piece, sr, path, quality=quality, keep_wav=False)
        files[sec["id"]] = str(path.relative_to(outdir))
    print(json.dumps({"group": ns.group, "npy": str(npy), "files": files, "hash": cue_hash}))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
