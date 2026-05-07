#!/usr/bin/env python3
"""Install rendered single-track lofi/voyage cue OGGs into the sandbox crate.

Companion to ``install_first_goblin_tune_v2_assets.py``: that one handles
the adaptive multi-stem goblin encounter cue; this one handles the simple
single-track music cues that replace the procedural ``render_lofi_theme``
synth path on the three sandbox music tracks.

For each cue, reads the renderer's preview directory (default
``tools/audio/music_renderer/output/<cue>/preview``), picks the newest
``<cue>_<hash>.full_soundtrack_preview.ogg``, and copies it to a stable
hash-free name at
``crates/ambition_sandbox/assets/audio/music/generated/<cue>/full.ogg``.

The Rust loader (``MusicTrackSpec.asset_path`` in
``crates/ambition_sandbox/assets/ambition/sandbox.ron``) targets that
stable filename, so re-rendering the cue does not require Rust changes.

Usage:

    cd tools/audio/music_renderer && source .venv/bin/activate
    for cue in lofi_study_loop long_lofi_drift pulse_drift_voyage; do
        python -m ambition_music_renderer.render_isolated \\
            examples/$cue.music.yaml \\
            --outdir output/$cue --backend pretty-midi
    done
    cd ../../..
    python tools/audio/install_lofi_study_loop_asset.py
        # installs all three by default; use --cue to install only one

The Rust runtime falls back to the procedural synth path for any track
whose asset_path file is missing, so partial installs work too.
"""
from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path

CUES = ("lofi_study_loop", "long_lofi_drift", "pulse_drift_voyage")


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def find_full_mix(preview_dir: Path, cue: str) -> Path | None:
    candidates = sorted(
        preview_dir.glob(f"{cue}_*.full_soundtrack_preview.ogg"),
        key=lambda p: p.stat().st_mtime,
        reverse=True,
    )
    return candidates[0] if candidates else None


def install_cue(cue: str, src_root: Path, dest_root: Path) -> bool:
    preview_dir = src_root / cue / "preview"
    src_file = find_full_mix(preview_dir, cue)
    if src_file is None:
        print(
            f"skip {cue}: no full_soundtrack_preview.ogg in {preview_dir}.\n"
            f"  hint: render first --\n"
            f"    cd tools/audio/music_renderer && source .venv/bin/activate &&\n"
            f"    python -m ambition_music_renderer.render_isolated "
            f"examples/{cue}.music.yaml --outdir output/{cue} --backend pretty-midi",
            file=sys.stderr,
        )
        return False
    dest_dir = dest_root / cue
    dest_dir.mkdir(parents=True, exist_ok=True)
    dst_file = dest_dir / "full.ogg"
    shutil.copy2(src_file, dst_file)
    rel = dst_file.relative_to(repo_root())
    src_rel = src_file.relative_to(repo_root())
    print(f"installed {cue}: {src_rel} -> {rel}")
    return True


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--src-root",
        type=Path,
        default=repo_root() / "tools" / "audio" / "music_renderer" / "output",
        help="renderer output root (each cue lives in <root>/<cue>/preview)",
    )
    parser.add_argument(
        "--dest-root",
        type=Path,
        default=repo_root()
        / "crates"
        / "ambition_sandbox"
        / "assets"
        / "audio"
        / "music"
        / "generated",
        help="install destination root",
    )
    parser.add_argument(
        "--cue",
        action="append",
        choices=CUES,
        help="install only the named cue(s); repeat to select multiple. "
        "default installs all three",
    )
    args = parser.parse_args(argv)

    cues = tuple(args.cue) if args.cue else CUES
    installed = 0
    for cue in cues:
        if install_cue(cue, args.src_root, args.dest_root):
            installed += 1
    print(f"installed {installed}/{len(cues)} cue(s)")
    return 0 if installed == len(cues) else 1


if __name__ == "__main__":
    raise SystemExit(main())
