#!/usr/bin/env python3
"""Install the rendered ``lofi_study_loop`` cue OGG into the sandbox crate.

Companion to ``install_first_goblin_tune_v2_assets.py``: that one handles
the adaptive multi-stem goblin encounter cue; this one handles the simple
single-track lofi loop that replaces the procedural ``render_lofi_theme``
synth path on ``original_lofi_loop``.

Reads the renderer's preview directory (default
``tools/audio/music_renderer/output/lofi_study_loop/preview``), picks the
newest ``lofi_study_loop_<hash>.full_soundtrack_preview.ogg``, and copies
it to a stable hash-free name at
``crates/ambition_sandbox/assets/audio/music/generated/lofi_study_loop/full.ogg``.

The Rust loader (``MusicTrackSpec.asset_path`` in
``crates/ambition_sandbox/assets/ambition/sandbox.ron``) targets that
stable filename, so re-rendering the cue does not require Rust changes.

Usage:

    cd tools/audio/music_renderer
    source .venv/bin/activate
    python -m ambition_music_renderer.render_isolated \\
        examples/lofi_study_loop.music.yaml \\
        --outdir output/lofi_study_loop \\
        --backend pretty-midi
    cd ../../../
    python tools/audio/install_lofi_study_loop_asset.py
"""
from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path

CUE_ID = "lofi_study_loop"


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def find_full_mix(preview_dir: Path) -> Path:
    candidates = sorted(
        preview_dir.glob(f"{CUE_ID}_*.full_soundtrack_preview.ogg"),
        key=lambda p: p.stat().st_mtime,
        reverse=True,
    )
    if not candidates:
        raise SystemExit(
            f"no full_soundtrack_preview.ogg matching '{CUE_ID}_*' in {preview_dir}.\n"
            f"hint: run the renderer first --\n"
            f"  cd tools/audio/music_renderer && source .venv/bin/activate &&\n"
            f"  python -m ambition_music_renderer.render_isolated "
            f"examples/{CUE_ID}.music.yaml --outdir output/{CUE_ID} --backend pretty-midi"
        )
    return candidates[0]


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--src",
        type=Path,
        default=repo_root()
        / "tools"
        / "audio"
        / "music_renderer"
        / "output"
        / CUE_ID
        / "preview",
        help="renderer preview output directory",
    )
    parser.add_argument(
        "--dest",
        type=Path,
        default=repo_root()
        / "crates"
        / "ambition_sandbox"
        / "assets"
        / "audio"
        / "music"
        / "generated"
        / CUE_ID,
        help="install destination",
    )
    args = parser.parse_args(argv)

    src_file = find_full_mix(args.src)
    args.dest.mkdir(parents=True, exist_ok=True)
    dst_file = args.dest / "full.ogg"
    shutil.copy2(src_file, dst_file)
    rel = dst_file.relative_to(repo_root())
    src_rel = src_file.relative_to(repo_root())
    print(f"installed: {src_rel} -> {rel}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
