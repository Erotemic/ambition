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
import argparse, json, math, os, shlex, subprocess, sys
from pathlib import Path
import numpy as np
import yaml
from . import musicir_renderer as r


def in_game_preview_mixes(spec: dict, group_names: list[str]) -> dict[str, dict[str, float]]:
    """Define preview mixes that approximate runtime playback at different
    dynamic intensities.

    - `minimal`: just the cue's bridge stems (the layers the runtime keeps
      audible during low-action gameplay) at full gain. Gives a sense of
      the cue's sustained foundation.
    - `maximal`: every stem at gain 1.0, simulating the loudest moment
      where every layer is fully active.
    - `state_<name>`: one preview per state_map entry that has explicit
      `stems` weights, using the runtime weights as authored. Useful for
      A/B-ing how the cue actually sounds during specific gameplay states.

    All previews use the same per-stem post-processed audio that the runtime
    loads, so the mixes are honest about runtime balance.
    """
    out: dict[str, dict[str, float]] = {}

    bridge = (spec.get("playback", {}) or {}).get("exit_policy", {}).get("bridge_stems") or []
    bridge = [s for s in bridge if s in group_names]
    if bridge:
        out["minimal"] = {s: 1.0 for s in bridge}
    out["maximal"] = {g: 1.0 for g in group_names}

    sm = spec.get("state_map", {}) or {}
    for name, cfg in sm.items():
        if not isinstance(cfg, dict):
            continue
        stems = cfg.get("stems")
        if not isinstance(stems, dict):
            continue
        weights = {k: float(v) for k, v in stems.items() if isinstance(v, (int, float)) and float(v) > 0.0}
        if weights:
            out[f"state_{name}"] = weights

    return out


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="Render Ambition MusicIR via isolated stem workers")
    ap.add_argument("spec")
    ap.add_argument("--outdir", default="output")
    ap.add_argument("--backend", default="fallback", choices=["fallback", "auto", "fluidsynth-cli", "pretty-midi"])
    ap.add_argument(
        "--simple-mix",
        action="store_true",
        help=(
            "Only emit the mastered preview/full_soundtrack_preview.ogg. "
            "Skips per-section per-group adaptive stem OGGs, per-section "
            "full slices, and the in-game preview mixes. Cuts ~10 OGG "
            "encodes per cue down to 1; appropriate for non-adaptive "
            "single-track music (e.g. sandbox lofi cues) where the runtime "
            "loads only the master mix anyway."
        ),
    )
    ap.add_argument(
        "--jobs",
        "-j",
        type=int,
        default=max(1, (os.cpu_count() or 2) // 2),
        help=(
            "Parallel worker subprocess count for per-group synth. Default "
            "is half the CPU count (each worker is single-threaded "
            "fluidsynth + reverb DSP, so going past physical cores hurts). "
            "Pass 1 for sequential rendering."
        ),
    )
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

    # Run per-group workers in parallel up to --jobs at a time. Each
    # worker is a separate Python subprocess with its own FluidSynth
    # state, so concurrency here is safe (the original sequential loop
    # picked subprocess isolation for stability, not for serialization).
    def worker_cmd(group: str) -> list[str]:
        cmd = [
            sys.executable, "-m", "ambition_music_renderer.render_group_worker",
            str(spec_path), "--outdir", str(outdir), "--group", group, "--backend", ns.backend,
        ]
        if ns.simple_mix:
            cmd.append("--skip-section-ogg")
        return cmd

    jobs = max(1, min(ns.jobs, len(group_names)))
    if jobs == 1:
        for group in group_names:
            subprocess.run(worker_cmd(group), check=True)
    else:
        # Schedule with a sliding window: launch up to `jobs` at once,
        # await any completion, then launch the next. Polls in a small
        # sleep loop because `Popen.wait(timeout=...)` raises on timeout
        # which makes the "wait for any" idiom awkward.
        import time as _time
        pending: list[tuple[str, subprocess.Popen]] = []
        remaining = list(group_names)
        while remaining or pending:
            while remaining and len(pending) < jobs:
                grp = remaining.pop(0)
                pending.append((grp, subprocess.Popen(worker_cmd(grp))))
            done_idx = None
            while done_idx is None:
                for i, (_, proc) in enumerate(pending):
                    if proc.poll() is not None:
                        done_idx = i
                        break
                if done_idx is None:
                    _time.sleep(0.1)
            grp, proc = pending.pop(done_idx)
            if proc.returncode != 0:
                # Tear down the rest before propagating so we don't leak
                # fluidsynth subprocesses if one worker crashes.
                for _, other in pending:
                    other.terminate()
                for _, other in pending:
                    other.wait()
                raise subprocess.CalledProcessError(proc.returncode, worker_cmd(grp))

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

    # Per-section full slices. If a section defines its own `postprocess`
    # block (per-section ambience override), apply *that* chain to the
    # raw stem-sum slice instead of using the master-mixed version. This
    # lets a section sound markedly different from the rest of the cue
    # (e.g. an intimate intro while the climax sounds cathedral) without
    # remixing every stem.
    sections_in_spec = {s["id"]: s for s in spec.get("sections", [])}
    if not ns.simple_mix:
        for sec in meta:
            sec_spec = sections_in_spec.get(sec["id"], {})
            section_pp = sec_spec.get("postprocess")
            if section_pp:
                # Slice the raw stem sum (pre-master), apply the section's
                # postprocess chain to that slice.
                raw_piece = r.slice_audio(full, sr, sec["start_seconds"], sec["end_seconds"])
                section_settings = dict(master_settings)
                section_settings.update(section_pp)
                piece = r.post_process(raw_piece, sr, section_settings)
            else:
                piece = r.slice_audio(master, sr, sec["start_seconds"], sec["end_seconds"])
            path = outdir / "adaptive" / sec["id"] / f"{spec['id']}_{cue_hash}.{sec['id']}.full.ogg"
            r.write_ogg_from_audio(piece, sr, path, quality=quality, keep_wav=False)
            output_files["adaptive"].setdefault(sec["id"], {})["full"] = str(path.relative_to(outdir))

    # ---- In-game-style previews (no master chain, soft limit only) ----
    # The runtime layers stems on the fly and never runs the master postprocess.
    # These previews approximate that mixing path so it's possible to listen
    # to what each gameplay state actually sounds like in-engine. Skipped in
    # --simple-mix mode because non-adaptive single-track cues never load
    # them and the OGG encoding cost dominates render time.
    if not ns.simple_mix:
        in_game_mixes = in_game_preview_mixes(spec, group_names)

        for label, weights in in_game_mixes.items():
            mix = np.zeros((target, 2), dtype="float32")
            for group, weight in weights.items():
                if group in stem_audio and weight > 0.0:
                    mix += stem_audio[group] * float(weight)
            # Normalize each preview to a similar peak as the mastered preview
            # so listening A/B between them is about timbre and balance rather
            # than absolute level. The runtime would still play stems at their
            # native level — these previews are an authoring aid.
            mix = r.soft_limit(mix, target_peak_db=-2.5, drive=1.0, normalize=True)
            path = outdir / "preview" / f"{spec['id']}_{cue_hash}.in_game_{label}.ogg"
            r.write_ogg_from_audio(mix, sr, path, quality=quality, keep_wav=False)
            output_files["preview"][f"in_game_{label}"] = str(path.relative_to(outdir))

    manifest = r.build_manifest(spec, cue_hash, meta, group_names, output_files, sr)
    manifest["render_mode"] = "isolated_process_stem_warmmix"
    manifest_path = outdir / f"{spec['id']}_{cue_hash}.adaptive_manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf8")

    # Write a regen.sh into the output directory so the cue can be re-rendered
    # from the same inputs without remembering the CLI invocation. The script
    # activates a sibling .venv if one exists in the renderer dir, so users
    # can `bash regen.sh` from anywhere.
    renderer_dir = Path(__file__).resolve().parent.parent
    abs_spec = spec_path.resolve()
    abs_outdir = outdir.resolve()
    regen = outdir / "regen.sh"
    regen.write_text(
        "#!/usr/bin/env bash\n"
        "# Auto-generated by render_isolated.py — regenerates this cue from the\n"
        "# same spec + backend that produced the contents of this directory.\n"
        "set -euo pipefail\n"
        f"renderer_dir={shlex.quote(str(renderer_dir))}\n"
        f"spec={shlex.quote(str(abs_spec))}\n"
        f"outdir={shlex.quote(str(abs_outdir))}\n"
        f"backend={shlex.quote(ns.backend)}\n"
        'cd "$renderer_dir"\n'
        'if [ -d .venv ]; then source .venv/bin/activate; fi\n'
        'rm -rf "$outdir"\n'
        'python -m ambition_music_renderer.render_isolated "$spec" \\\n'
        '    --outdir "$outdir" \\\n'
        '    --backend "$backend"\n',
        encoding="utf8",
    )
    regen.chmod(0o755)

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
