"""Cue regeneration, diagnostics, and shareable debug bundles.

This module is intentionally an orchestration layer around the current renderer
rather than a replacement renderer.  Its job is to make one cue reproducible and
inspectable from a single command while the lower-level MusicIR internals are
refactored behind a stable workflow.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import shutil
import subprocess
import sys
import time
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

import numpy as np
import yaml

from . import musicir_renderer as r

DEFAULT_BACKEND = "pretty-midi"
BACKEND_CHOICES = ("pretty-midi", "fluidsynth-cli", "fallback", "auto")


@dataclass(frozen=True)
class CommandResult:
    name: str
    command: list[str]
    returncode: int
    stdout: Path
    stderr: Path


def package_dir() -> Path:
    return Path(__file__).resolve().parent.parent


def repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


def default_generated_root() -> Path:
    return package_dir() / "generated"


def default_bundle_root() -> Path:
    return package_dir() / "bundles"


def default_publish_dest_root() -> Path:
    return (
        repo_root()
        / "crates"
        / "ambition_gameplay_core"
        / "assets"
        / "audio"
        / "music"
        / "generated"
    )


def find_score(cue: str) -> Path | None:
    """Locate a MusicIR score by cue id or path.

    Kept local to avoid importing the top-level CLI from this lower-level helper.
    """
    p = Path(cue)
    if p.suffix in (".yaml", ".yml") and p.exists():
        return p.resolve()
    for sub in ("active", "examples", "archive", "experiments"):
        for suffix in (".music.yaml", ".yaml", ".yml"):
            candidate = package_dir() / "scores" / sub / f"{cue}{suffix}"
            if candidate.exists():
                return candidate.resolve()
    return None


def load_yaml(path: Path) -> dict:
    data = yaml.safe_load(path.read_text(encoding="utf8"))
    if not isinstance(data, dict):
        raise ValueError(f"expected YAML mapping in {path}")
    return data


def latest_manifest(outdir: Path, cue_id: str) -> Path | None:
    candidates = sorted(
        outdir.glob(f"{cue_id}_*.adaptive_manifest.json"),
        key=lambda p: p.stat().st_mtime,
        reverse=True,
    )
    return candidates[0] if candidates else None


def safe_rel(path: Path, root: Path | None = None) -> str:
    path = Path(path)
    if root is None:
        root = repo_root()
    try:
        return str(path.resolve().relative_to(root.resolve()))
    except Exception:
        return str(path)


def run_logged(name: str, command: list[str], reports_dir: Path, *, cwd: Path) -> CommandResult:
    reports_dir.mkdir(parents=True, exist_ok=True)
    stdout = reports_dir / f"{name}.stdout.txt"
    stderr = reports_dir / f"{name}.stderr.txt"
    with stdout.open("w", encoding="utf8") as out_f, stderr.open("w", encoding="utf8") as err_f:
        proc = subprocess.run(command, cwd=cwd, stdout=out_f, stderr=err_f)
    return CommandResult(name, command, proc.returncode, stdout, stderr)


def _db(value: float) -> float:
    value = max(float(value), 1e-12)
    return 20.0 * math.log10(value)


def _audio_stats(audio: np.ndarray, sample_rate: int) -> dict[str, float]:
    if audio.size == 0:
        return {
            "sample_rate": float(sample_rate),
            "duration_s": 0.0,
            "peak_dbfs": _db(0.0),
            "rms_dbfs": _db(0.0),
        }
    frames = audio.shape[0]
    peak = float(np.max(np.abs(audio)))
    rms = float(np.sqrt(np.mean(np.square(audio), dtype=np.float64)))
    return {
        "sample_rate": float(sample_rate),
        "duration_s": float(frames / sample_rate) if sample_rate else 0.0,
        "peak_dbfs": _db(peak),
        "rms_dbfs": _db(rms),
    }


def _read_audio_stats(path: Path) -> tuple[dict[str, float] | None, str | None]:
    try:
        import soundfile as sf

        audio, sample_rate = sf.read(path, always_2d=True, dtype="float32")
        return _audio_stats(audio.astype("float32", copy=False), int(sample_rate)), None
    except Exception as ex:  # noqa: BLE001 - report diagnostics, do not fail the bundle.
        return None, f"{type(ex).__name__}: {ex}"


def manifest_duration(manifest: dict) -> float:
    sections = manifest.get("sections") or []
    ends = [float(sec.get("end_seconds", 0.0)) for sec in sections if isinstance(sec, dict)]
    return max(ends) if ends else 0.0


def write_stem_export_report(outdir: Path, manifest: dict, reports_dir: Path) -> Path:
    """Compare retained .npy stem buffers with exported per-stem audio files.

    This is the report we wanted during the Emmy debugging session: it answers
    whether scratch stem buffers, adaptive stem OGGs, and section full mixes have
    matching durations and plausible levels.
    """
    reports_dir.mkdir(parents=True, exist_ok=True)
    sample_rate = int(manifest.get("sample_rate", 48000))
    cue_id = manifest.get("id", "unknown")
    rows: list[dict[str, object]] = []

    scratch_dir = outdir / "scratch_stems"
    for npy in sorted(scratch_dir.glob("*.npy")):
        group = npy.stem.split(".")[-1]
        try:
            arr = np.load(npy).astype("float32", copy=False)
            stats = _audio_stats(arr, sample_rate)
            error = ""
        except Exception as ex:  # noqa: BLE001
            stats = {}
            error = f"{type(ex).__name__}: {ex}"
        rows.append(
            {
                "kind": "scratch_npy",
                "section": "*",
                "group": group,
                "path": str(npy.relative_to(outdir)),
                **stats,
                "error": error,
            }
        )

    files = manifest.get("files") or {}
    adaptive = files.get("adaptive") or {}
    if isinstance(adaptive, dict):
        for section_id, section_files in sorted(adaptive.items()):
            if not isinstance(section_files, dict):
                continue
            for group, rel in sorted(section_files.items()):
                if not isinstance(rel, str):
                    continue
                path = outdir / rel
                stats, error = _read_audio_stats(path)
                rows.append(
                    {
                        "kind": "adaptive_audio",
                        "section": section_id,
                        "group": group,
                        "path": rel,
                        **(stats or {}),
                        "error": error or "",
                    }
                )

    preview = files.get("preview") or {}
    if isinstance(preview, dict):
        for name, rel in sorted(preview.items()):
            if not isinstance(rel, str):
                continue
            path = outdir / rel
            stats, error = _read_audio_stats(path)
            rows.append(
                {
                    "kind": "preview_audio",
                    "section": "*",
                    "group": name,
                    "path": rel,
                    **(stats or {}),
                    "error": error or "",
                }
            )

    columns = [
        "kind",
        "section",
        "group",
        "duration_s",
        "rms_dbfs",
        "peak_dbfs",
        "sample_rate",
        "path",
        "error",
    ]
    out = reports_dir / "stem_export_report.tsv"
    lines = ["\t".join(columns)]
    for row in rows:
        cells = []
        for col in columns:
            value = row.get(col, "")
            if isinstance(value, float):
                cells.append(f"{value:.3f}")
            else:
                cells.append(str(value))
        lines.append("\t".join(cells))
    out.write_text("\n".join(lines) + "\n", encoding="utf8")

    summary = {
        "cue_id": cue_id,
        "outdir": str(outdir),
        "rows": rows,
    }
    (reports_dir / "stem_export_report.json").write_text(
        json.dumps(summary, indent=2), encoding="utf8"
    )
    return out


def write_spectrograms(outdir: Path, manifest: dict, plots_dir: Path, *, limit: int = 16) -> list[Path]:
    """Write compact spectrogram PNGs for retained scratch stems and key previews.

    Matplotlib is intentionally optional. If it is not installed, write a clear
    note and let the rest of the bundle succeed.
    """
    plots_dir.mkdir(parents=True, exist_ok=True)
    try:
        import matplotlib.pyplot as plt
        from scipy import signal
    except Exception as ex:  # noqa: BLE001
        note = plots_dir / "spectrograms_skipped.txt"
        note.write_text(
            f"spectrogram generation skipped: {type(ex).__name__}: {ex}\n",
            encoding="utf8",
        )
        return []

    sample_rate = int(manifest.get("sample_rate", 48000))
    written: list[Path] = []

    def save_audio_png(audio: np.ndarray, title: str, dest: Path) -> None:
        mono = audio.mean(axis=1) if audio.ndim == 2 else audio.astype("float32")
        if mono.size == 0:
            return
        nperseg = min(4096, max(256, int(2 ** math.floor(math.log2(max(256, min(len(mono), 4096)))))))
        noverlap = max(0, int(nperseg * 0.75))
        freqs, times, spec = signal.spectrogram(
            mono,
            fs=sample_rate,
            nperseg=nperseg,
            noverlap=noverlap,
            scaling="spectrum",
            mode="magnitude",
        )
        spec_db = 20 * np.log10(spec + 1e-10)
        plt.figure(figsize=(14, 5))
        plt.pcolormesh(times, freqs, spec_db, shading="auto", vmin=-110, vmax=-35)
        plt.yscale("log")
        plt.ylim(80, 12000)
        plt.axhspan(3000, 6000, alpha=0.15)
        plt.axhspan(6000, 12000, alpha=0.10)
        plt.title(title)
        plt.xlabel("time (s)")
        plt.ylabel("frequency (Hz)")
        plt.colorbar(label="dB")
        plt.tight_layout()
        plt.savefig(dest, dpi=140)
        plt.close()

    candidates: list[tuple[str, Path, str]] = []
    for npy in sorted((outdir / "scratch_stems").glob("*.npy")):
        candidates.append(("npy", npy, npy.stem.split(".")[-1]))
    files = manifest.get("files") or {}
    preview = files.get("preview") or {}
    if isinstance(preview, dict):
        for name, rel in sorted(preview.items()):
            if isinstance(rel, str):
                candidates.append(("audio", outdir / rel, f"preview_{name}"))

    for kind, path, label in candidates[:limit]:
        try:
            if kind == "npy":
                audio = np.load(path).astype("float32", copy=False)
            else:
                import soundfile as sf

                audio, _sample_rate = sf.read(path, always_2d=True, dtype="float32")
            dest = plots_dir / f"{label}.spectrogram.png"
            save_audio_png(audio, label, dest)
            if dest.exists():
                written.append(dest)
        except Exception as ex:  # noqa: BLE001
            (plots_dir / f"{label}.spectrogram.error.txt").write_text(
                f"failed to render {path}: {type(ex).__name__}: {ex}\n",
                encoding="utf8",
            )
    return written


def copy_tree_if_exists(src: Path, dst: Path) -> None:
    if not src.exists():
        return
    if src.is_dir():
        if dst.exists():
            shutil.rmtree(dst)
        shutil.copytree(src, dst)
    else:
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)


def make_zip(src_dir: Path, zip_path: Path) -> Path:
    zip_path.parent.mkdir(parents=True, exist_ok=True)
    if zip_path.exists():
        zip_path.unlink()
    with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for path in sorted(src_dir.rglob("*")):
            if path == zip_path or path.is_dir():
                continue
            zf.write(path, path.relative_to(src_dir.parent))
    return zip_path


def build_rerun_script(bundle_dir: Path, cue: str, backend: str, outdir: Path, publish: bool) -> Path:
    script = bundle_dir / "rerun_bundle.sh"
    publish_flag = " --publish" if publish else ""
    script.write_text(
        "#!/usr/bin/env bash\n"
        "set -euo pipefail\n"
        "cd \"$(git rev-parse --show-toplevel)\"\n"
        "PYTHONPATH=tools/ambition_music_renderer \\\n"
        f"python -m ambition_music_renderer cue bundle {cue} \\\n"
        f"  --backend {backend} \\\n"
        f"  --outdir {outdir} \\\n"
        f"  --force{publish_flag} --zip\n",
        encoding="utf8",
    )
    script.chmod(0o755)
    return script


def create_bundle(
    cue: str,
    *,
    backend: str = DEFAULT_BACKEND,
    outdir: Path | None = None,
    bundle_root: Path | None = None,
    force: bool = False,
    publish: bool = False,
    dest_root: Path | None = None,
    zip_bundle: bool = False,
    jobs: int = 1,
    include_scratch_stems: bool = False,
    skip_render: bool = False,
    skip_spectrograms: bool = False,
) -> dict[str, object]:
    score_path = find_score(cue)
    if score_path is None:
        raise FileNotFoundError(f"cue not found: {cue}")
    spec = load_yaml(score_path)
    cue_id = str(spec.get("id", cue))
    if cue_id != Path(score_path.name).name.split(".music.yaml")[0] and score_path.name.endswith(".music.yaml"):
        # Warn in the final report without preventing compatibility renders.
        id_warning = f"score id {cue_id!r} does not match filename {score_path.name!r}"
    else:
        id_warning = ""

    if outdir is None:
        outdir = default_generated_root() / cue_id
    else:
        outdir = Path(outdir)
    if bundle_root is None:
        bundle_root = default_bundle_root()
    else:
        bundle_root = Path(bundle_root)
    if dest_root is None:
        dest_root = default_publish_dest_root()
    else:
        dest_root = Path(dest_root)

    reports_dir = outdir / "reports"
    plots_dir = outdir / "plots"
    reports_dir.mkdir(parents=True, exist_ok=True)
    commands: list[CommandResult] = []

    if not skip_render:
        render_cmd = [
            sys.executable,
            "-m",
            "ambition_music_renderer.render_isolated",
            str(score_path),
            "--outdir",
            str(outdir),
            "--backend",
            backend,
            "--keep-debug-stems",
            "--jobs",
            str(jobs),
        ]
        if force:
            render_cmd.append("--force")
        commands.append(run_logged("render_isolated", render_cmd, reports_dir, cwd=package_dir()))
        if commands[-1].returncode != 0:
            return {
                "cue": cue_id,
                "ok": False,
                "error": "render_isolated failed",
                "commands": [c.__dict__ for c in commands],
                "outdir": str(outdir),
            }

    manifest_path = latest_manifest(outdir, cue_id)
    if manifest_path is None:
        raise FileNotFoundError(f"no adaptive manifest found in {outdir} for {cue_id}")
    manifest = json.loads(manifest_path.read_text(encoding="utf8"))
    render_hash = str(manifest.get("hash", "unknown"))
    duration = manifest_duration(manifest)

    # Diagnostics. These tools are report-only; a failure should not destroy the bundle.
    tools_dir = package_dir()
    commands.append(
        run_logged(
            "audit_cue_balance",
            [sys.executable, str(tools_dir / "audit_cue_balance.py"), str(outdir)],
            reports_dir,
            cwd=tools_dir,
        )
    )
    commands.append(
        run_logged(
            "level_report_preview",
            [
                sys.executable,
                str(tools_dir / "level_report.py"),
                "--root",
                str(outdir),
                "--glob",
                "preview/*.ogg",
                "--format",
                "tsv",
                "--target-rms-db",
                "-24",
            ],
            reports_dir,
            cwd=tools_dir,
        )
    )
    if (outdir / "scratch_stems").is_dir():
        hi = f"{duration:.3f}" if duration > 0 else "-1"
        commands.append(
            run_logged(
                "spectral_compare",
                [
                    sys.executable,
                    str(tools_dir / "spectral_compare.py"),
                    str(outdir),
                    "--window",
                    "0",
                    hi,
                    "--label",
                    cue_id,
                ],
                reports_dir,
                cwd=tools_dir,
            )
        )
        commands.append(
            run_logged(
                "spectral_localize",
                [
                    sys.executable,
                    str(tools_dir / "spectral_localize.py"),
                    str(outdir),
                    "--window",
                    "0",
                    "-1",
                    "--bucket",
                    "0.25",
                ],
                reports_dir,
                cwd=tools_dir,
            )
        )
    write_stem_export_report(outdir, manifest, reports_dir)
    if not skip_spectrograms:
        write_spectrograms(outdir, manifest, plots_dir)

    published: str | None = None
    if publish:
        # Import lazily so this module can be used by tests without importing the CLI.
        from .cli import publish_cue

        ok = publish_cue(cue_id, outdir, dest_root)
        if ok:
            published = str(dest_root / cue_id / "full.ogg")
        else:
            published = "publish failed"

    bundle_name = f"{cue_id}_{render_hash}_bundle"
    bundle_dir = bundle_root / bundle_name
    if bundle_dir.exists():
        shutil.rmtree(bundle_dir)
    bundle_dir.mkdir(parents=True, exist_ok=True)

    source_dir = bundle_dir / "source"
    source_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(score_path, source_dir / score_path.name)
    (source_dir / "normalized_spec.json").write_text(json.dumps(spec, indent=2), encoding="utf8")
    copy_tree_if_exists(outdir / "preview", bundle_dir / "preview")
    copy_tree_if_exists(outdir / "adaptive", bundle_dir / "adaptive")
    copy_tree_if_exists(reports_dir, bundle_dir / "reports")
    copy_tree_if_exists(plots_dir, bundle_dir / "plots")
    shutil.copy2(manifest_path, bundle_dir / manifest_path.name)
    if include_scratch_stems:
        copy_tree_if_exists(outdir / "scratch_stems", bundle_dir / "scratch_stems")

    rerun_script = build_rerun_script(bundle_dir, cue_id, backend, outdir, publish)

    command_rows = [
        {
            "name": c.name,
            "returncode": c.returncode,
            "command": c.command,
            "stdout": str(c.stdout),
            "stderr": str(c.stderr),
        }
        for c in commands
    ]
    report = {
        "schema": "ambition.music_debug_bundle.v1",
        "cue": cue_id,
        "score": safe_rel(score_path),
        "backend": backend,
        "render_hash": render_hash,
        "outdir": str(outdir),
        "bundle_dir": str(bundle_dir),
        "manifest": str(manifest_path),
        "duration_s": duration,
        "published": published,
        "include_scratch_stems": include_scratch_stems,
        "warnings": [w for w in [id_warning] if w],
        "commands": command_rows,
        "rerun_script": str(rerun_script),
    }
    (bundle_dir / "bundle_manifest.json").write_text(json.dumps(report, indent=2), encoding="utf8")

    zip_path: Path | None = None
    if zip_bundle:
        zip_path = make_zip(bundle_dir, bundle_root / f"{bundle_name}.zip")
        report["zip"] = str(zip_path)
        (bundle_dir / "bundle_manifest.json").write_text(json.dumps(report, indent=2), encoding="utf8")

    return report


def build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("cue", help="cue id or .music.yaml path")
    ap.add_argument("--backend", default=DEFAULT_BACKEND, choices=BACKEND_CHOICES)
    ap.add_argument("--outdir", type=Path, default=None)
    ap.add_argument("--bundle-root", type=Path, default=None)
    ap.add_argument("--force", action="store_true", help="force render regeneration")
    ap.add_argument("--publish", action="store_true", help="publish full.ogg to game assets after rendering")
    ap.add_argument("--dest-root", type=Path, default=None, help="game music generated asset root")
    ap.add_argument("--zip", dest="zip_bundle", action="store_true", help="write an uploadable bundle zip")
    ap.add_argument("--jobs", "-j", type=int, default=1, help="render worker count")
    ap.add_argument(
        "--include-scratch-stems",
        action="store_true",
        help="include raw scratch_stems/*.npy in the bundle zip; useful but can be large",
    )
    ap.add_argument("--skip-render", action="store_true", help="bundle/analyze existing outdir")
    ap.add_argument("--skip-spectrograms", action="store_true", help="skip PNG spectrogram generation")
    return ap


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    report = create_bundle(
        args.cue,
        backend=args.backend,
        outdir=args.outdir,
        bundle_root=args.bundle_root,
        force=args.force,
        publish=args.publish,
        dest_root=args.dest_root,
        zip_bundle=args.zip_bundle,
        jobs=args.jobs,
        include_scratch_stems=args.include_scratch_stems,
        skip_render=args.skip_render,
        skip_spectrograms=args.skip_spectrograms,
    )
    print(json.dumps(report, indent=2, default=str))
    return 0 if report.get("ok", True) else 1


if __name__ == "__main__":
    raise SystemExit(main())
