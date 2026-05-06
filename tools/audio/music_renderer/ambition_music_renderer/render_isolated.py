#!/usr/bin/env python3
"""Render MusicIR using isolated stem worker processes.

This is the production-oriented entry point for long adaptive cues. It writes:
- adaptive/<section>/<section>.<stem>.ogg
- adaptive/<section>/<section>.full.ogg
- preview/<cue>.full_soundtrack_preview.ogg     (mastered full mix)
- preview/<cue>.in_game_full_active.ogg         (all stems summed, no master)
- preview/<cue>.in_game_state_<name>.ogg        (minimal/maximal state mixes)
- <cue>.adaptive_manifest.json
"""
from __future__ import annotations
import argparse, json, math, os, subprocess, sys
from pathlib import Path
import numpy as np
import yaml
from . import musicir_renderer as r


def in_game_state_previews(spec: dict) -> dict[str, dict[str, float]]:
    """Pick a minimal/maximal pair of state_map entries (by total stem gain)
    to render as in-game-style previews. Returns a {label: {stem: gain}} dict.
    """
    sm = spec.get("state_map", {}) or {}
    states: list[tuple[str, float, dict[str, float]]] = []
    for name, cfg in sm.items():
        if not isinstance(cfg, dict):
            continue
        stems = cfg.get("stems")
        if not isinstance(stems, dict):
            continue
        total = sum(float(v) for v in stems.values() if isinstance(v, (int, float)))
        if total <= 0.0:
            continue
        states.append((name, total, {k: float(v) for k, v in stems.items()}))
    if not states:
        return {}
    states.sort(key=lambda s: s[1])
    out: dict[str, dict[str, float]] = {}
    out[f"state_{states[0][0]}_minimal"] = states[0][2]
    if len(states) > 1 and states[-1][0] != states[0][0]:
        out[f"state_{states[-1][0]}_maximal"] = states[-1][2]
    return out


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="Render Ambition MusicIR via isolated stem workers")
    ap.add_argument("spec")
    ap.add_argument("--outdir", default="output")
    ap.add_argument("--backend", default="fast", choices=["fast", "auto", "fluidsynth-cli", "pretty-midi"])
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

    for group in group_names:
        cmd = [
            sys.executable, "-m", "ambition_music_renderer.render_group_worker",
            str(spec_path), "--outdir", str(outdir), "--group", group, "--backend", ns.backend,
        ]
        subprocess.run(cmd, check=True)

    output_files: dict = {"preview": {}, "adaptive": {}}

    # Load all stems into memory once for the various preview mixes.
    stem_audio: dict[str, np.ndarray] = {}
    for group in group_names:
        npy = outdir / "debug_stems" / f"{spec['id']}_{cue_hash}.{group}.npy"
        stem_audio[group] = r.ensure_audio_length(np.load(npy), target)
        for sec in meta:
            path = outdir / "adaptive" / sec["id"] / f"{spec['id']}_{cue_hash}.{sec['id']}.{group}.ogg"
            output_files["adaptive"].setdefault(sec["id"], {})[group] = str(path.relative_to(outdir))

    # ---- Full mastered preview (matches the YAML postprocess intent) ----
    full = np.zeros((target, 2), dtype="float32")
    for arr in stem_audio.values():
        full += arr
    master_settings = dict(spec.get("postprocess", {}) or {})
    master_settings.setdefault("normalize", True)
    master_settings.setdefault("target_peak_db", -1.2)
    master = r.post_process(full, sr, master_settings)
    preview = outdir / "preview" / f"{spec['id']}_{cue_hash}.full_soundtrack_preview.ogg"
    r.write_ogg_from_audio(master, sr, preview, quality=quality, keep_wav=False)
    output_files["preview"]["full_soundtrack"] = str(preview.relative_to(outdir))

    # Per-section full slices come from the same mastered mix.
    for sec in meta:
        piece = r.slice_audio(master, sr, sec["start_seconds"], sec["end_seconds"])
        path = outdir / "adaptive" / sec["id"] / f"{spec['id']}_{cue_hash}.{sec['id']}.full.ogg"
        r.write_ogg_from_audio(piece, sr, path, quality=quality, keep_wav=False)
        output_files["adaptive"].setdefault(sec["id"], {})["full"] = str(path.relative_to(outdir))

    # ---- In-game-style previews (no master chain, soft limit only) ----
    # The runtime layers stems on the fly and never runs the master postprocess.
    # These previews approximate that mixing path so it's possible to listen
    # to what each gameplay state actually sounds like in-engine.
    in_game_mixes: dict[str, dict[str, float]] = {
        "full_active": {g: 1.0 for g in group_names},
    }
    in_game_mixes.update(in_game_state_previews(spec))

    for label, weights in in_game_mixes.items():
        mix = np.zeros((target, 2), dtype="float32")
        for group, weight in weights.items():
            if group in stem_audio and weight > 0.0:
                mix += stem_audio[group] * float(weight)
        # No master EQ/reverb/limiter chain — just a soft ceiling so summed
        # stems don't clip the OGG encoder.
        mix = r.soft_limit(mix, target_peak_db=-1.5, drive=1.0, normalize=False)
        path = outdir / "preview" / f"{spec['id']}_{cue_hash}.in_game_{label}.ogg"
        r.write_ogg_from_audio(mix, sr, path, quality=quality, keep_wav=False)
        output_files["preview"][f"in_game_{label}"] = str(path.relative_to(outdir))

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

    print(json.dumps({
        "manifest": str(manifest_path),
        "preview": str(preview),
        "in_game_previews": [v for k, v in output_files["preview"].items() if k.startswith("in_game_")],
        "hash": cue_hash,
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
