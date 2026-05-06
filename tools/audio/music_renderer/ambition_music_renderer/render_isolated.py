#!/usr/bin/env python3
"""Render MusicIR using isolated stem worker processes.

This is the production-oriented entry point for long adaptive cues. It writes:
- adaptive/<section>/<section>.<stem>.ogg
- adaptive/<section>/<section>.full.ogg
- preview/<cue>.full_soundtrack_preview.ogg
- <cue>.adaptive_manifest.json
"""
from __future__ import annotations
import argparse, json, math, os, subprocess, sys
from pathlib import Path
import numpy as np
import yaml
from . import musicir_renderer as r


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="Render Ambition MusicIR via isolated stem workers")
    ap.add_argument("spec")
    ap.add_argument("--outdir", default="output")
    ap.add_argument("--backend", default="fast", choices=["fast", "auto", "fluidsynth-cli", "pretty-midi"])
    ap.add_argument("--simple-groups", default="", help="Comma-separated groups to render with cheap gain/limit only")
    ns = ap.parse_args(argv)
    spec_path = Path(ns.spec)
    spec = yaml.safe_load(spec_path.read_text())
    render_cfg = spec.get("render", {})
    sr = int(render_cfg.get("sample_rate", 48000))
    soundfont = r.choose_soundfont(render_cfg.get("soundfont"))
    cue_hash = r.spec_hash(spec_path, soundfont, ns.backend)
    quality = float(render_cfg.get("ogg_quality", 5.0))
    pm, groups, meta = r.build_score(spec)
    total = meta[-1]["end_seconds"]
    target = int(math.ceil(total * sr))
    group_names = sorted(set(groups.values()))
    outdir = Path(ns.outdir)
    outdir.mkdir(parents=True, exist_ok=True)
    simple_groups = {g.strip() for g in ns.simple_groups.split(",") if g.strip()}
    for group in group_names:
        cmd = [sys.executable, "-m", "ambition_music_renderer.render_group_worker", str(spec_path), "--outdir", str(outdir), "--group", group, "--backend", ns.backend]
        if group in simple_groups:
            cmd.append("--simple-post")
        subprocess.run(cmd, check=True)
    output_files = {"preview": {}, "adaptive": {}}
    full = np.zeros((target, 2), dtype="float32")
    for group in group_names:
        npy = outdir / "debug_stems" / f"{spec['id']}_{cue_hash}.{group}.npy"
        full += r.ensure_audio_length(np.load(npy), target)
        for sec in meta:
            path = outdir / "adaptive" / sec["id"] / f"{spec['id']}_{cue_hash}.{sec['id']}.{group}.ogg"
            output_files["adaptive"].setdefault(sec["id"], {})[group] = str(path.relative_to(outdir))
    master = r.soft_limit(full, float(spec.get("postprocess", {}).get("target_peak_db", -1.2)), drive=1.02, normalize=True)
    preview = outdir / "preview" / f"{spec['id']}_{cue_hash}.full_soundtrack_preview.ogg"
    r.write_ogg_from_audio(master, sr, preview, quality=quality, keep_wav=False)
    output_files["preview"]["full_soundtrack"] = str(preview.relative_to(outdir))
    for sec in meta:
        piece = r.slice_audio(master, sr, sec["start_seconds"], sec["end_seconds"])
        path = outdir / "adaptive" / sec["id"] / f"{spec['id']}_{cue_hash}.{sec['id']}.full.ogg"
        r.write_ogg_from_audio(piece, sr, path, quality=quality, keep_wav=False)
        output_files["adaptive"].setdefault(sec["id"], {})["full"] = str(path.relative_to(outdir))
    manifest = r.build_manifest(spec, cue_hash, meta, group_names, output_files, sr)
    manifest["render_mode"] = "isolated_process_stem_warmmix"
    manifest_path = outdir / f"{spec['id']}_{cue_hash}.adaptive_manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf8")
    for npy in (outdir / "debug_stems").glob("*.npy"):
        npy.unlink()
    try:
        (outdir / "debug_stems").rmdir()
    except OSError:
        pass
    print(json.dumps({"manifest": str(manifest_path), "preview": str(preview), "hash": cue_hash}, indent=2))
    return 0

if __name__ == "__main__":
    code = main()
    sys.stdout.flush(); sys.stderr.flush(); os._exit(code)
