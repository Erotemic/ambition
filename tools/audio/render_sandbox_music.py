#!/usr/bin/env python3
"""One-shot render + install for sandbox single-track music cues.

The Bevy runtime expects rendered OGGs at stable paths under
``crates/ambition_sandbox/assets/audio/music/generated/<cue>/full.ogg``
(declared in ``MusicTrackSpec.asset_path`` in
``crates/ambition_sandbox/assets/ambition/sandbox.ron``). Rendering
produces hashed filenames, so we publish a stable name pointing at
the latest hash.

This script combines the two steps so the whole "make music ready"
workflow is one command:

    python tools/audio/render_sandbox_music.py             # all three cues
    python tools/audio/render_sandbox_music.py --cue lofi_study_loop  # one
    python tools/audio/render_sandbox_music.py --skip-render  # just publish

Steps for each requested cue:

1. Render the YAML via ``ambition_music_renderer.render_isolated`` into
   ``tools/audio/music_renderer/output/<cue>/`` -- skipped if the YAML
   is unchanged since the last render and a preview already exists.
2. Copy the newest ``<cue>_<hash>.full_soundtrack_preview.ogg`` from
   the renderer output to
   ``crates/ambition_sandbox/assets/audio/music/generated/<cue>/full.ogg``.

The cargo build (host or VM) reads the asset directly. The cargo
target dir does not matter for asset resolution -- Bevy resolves
``assets/`` relative to ``CARGO_MANIFEST_DIR``, baked into the binary
at compile time.

The renderer requires FluidSynth + a SoundFont (set up via
``tools/audio/music_renderer/setup.sh``).
"""
from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

CUES = ("lofi_study_loop", "long_lofi_drift", "pulse_drift_voyage")


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def renderer_dir() -> Path:
    return repo_root() / "tools" / "audio" / "music_renderer"


def renderer_python() -> Path:
    """Path to the renderer's venv python. Set up via setup.sh."""
    venv_python = renderer_dir() / ".venv" / "bin" / "python"
    if venv_python.exists():
        return venv_python
    # Fallback to system python if no venv -- the user will hit a
    # missing-deps error and can react.
    return Path(sys.executable)


def yaml_path(cue: str) -> Path:
    return renderer_dir() / "examples" / f"{cue}.music.yaml"


def render_outdir(cue: str) -> Path:
    return renderer_dir() / "output" / cue


def find_full_mix(preview_dir: Path, cue: str) -> Path | None:
    candidates = sorted(
        preview_dir.glob(f"{cue}_*.full_soundtrack_preview.ogg"),
        key=lambda p: p.stat().st_mtime,
        reverse=True,
    )
    return candidates[0] if candidates else None


def needs_render(cue: str) -> bool:
    """True if the YAML has changed since the last preview was written,
    or no preview exists yet."""
    preview_dir = render_outdir(cue) / "preview"
    latest = find_full_mix(preview_dir, cue)
    if latest is None:
        return True
    return yaml_path(cue).stat().st_mtime > latest.stat().st_mtime


def render_cue(cue: str, backend: str) -> bool:
    yaml = yaml_path(cue)
    if not yaml.exists():
        print(f"skip {cue}: missing YAML at {yaml}", file=sys.stderr)
        return False
    outdir = render_outdir(cue)
    cmd = [
        str(renderer_python()),
        "-m",
        "ambition_music_renderer.render_isolated",
        str(yaml),
        "--outdir",
        str(outdir),
        "--backend",
        backend,
    ]
    print(f"render {cue}: {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=renderer_dir())
    return result.returncode == 0


def publish_cue(cue: str, dest_root: Path) -> bool:
    """Copy the newest rendered preview into the asset tree at the
    stable filename Bevy expects."""
    preview_dir = render_outdir(cue) / "preview"
    src = find_full_mix(preview_dir, cue)
    if src is None:
        print(
            f"skip publish {cue}: no full_soundtrack_preview.ogg in {preview_dir}",
            file=sys.stderr,
        )
        return False
    dest_dir = dest_root / cue
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest = dest_dir / "full.ogg"
    shutil.copy2(src, dest)
    src_rel = src.relative_to(repo_root())
    dest_rel = dest.relative_to(repo_root())
    print(f"publish {cue}: {src_rel} -> {dest_rel}")
    return True


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--cue",
        action="append",
        choices=CUES,
        help="restrict to the named cue(s); repeat to select multiple. "
        "default processes all three",
    )
    parser.add_argument(
        "--backend",
        default="pretty-midi",
        help="renderer backend (pretty-midi / fluidsynth-cli / fallback / auto)",
    )
    parser.add_argument(
        "--skip-render",
        action="store_true",
        help="skip the render step; just publish the most-recent preview",
    )
    parser.add_argument(
        "--force-render",
        action="store_true",
        help="re-render even if the YAML mtime is older than the preview",
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
        help="install destination root (default: bevy asset tree)",
    )
    args = parser.parse_args(argv)

    cues = tuple(args.cue) if args.cue else CUES
    failed = []
    for cue in cues:
        if not args.skip_render:
            if args.force_render or needs_render(cue):
                if not render_cue(cue, args.backend):
                    failed.append(f"render {cue}")
                    continue
            else:
                print(f"skip render {cue}: YAML unchanged since last render")
        if not publish_cue(cue, args.dest_root):
            failed.append(f"publish {cue}")
    if failed:
        print(f"FAILED: {', '.join(failed)}", file=sys.stderr)
        return 1
    print(f"OK: {len(cues)} cue(s) ready")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
