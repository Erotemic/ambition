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
import tempfile
import time
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

import numpy as np
import yaml

from . import musicir_renderer as r
from .arrangement_audit import audit_file as audit_arrangement_file
from .arrangement_audit import write_reports as write_arrangement_reports
from .dissonance_audit import audit_file as audit_dissonance_file
from .dissonance_audit import write_reports as write_dissonance_reports
from .sour_note_audit import audit_file as audit_sour_note_file
from .sour_note_audit import write_reports as write_sour_note_reports

DEFAULT_BACKEND = "pretty-midi"
BACKEND_CHOICES = ("pretty-midi", "fluidsynth-cli", "fallback", "auto")
RUNTIME_STEM_GAIN_MODES = ("native", "shared")
PLOT_FORMATS = ("jpg", "png")
REPORT_ZIP_EXCLUDED_SUFFIXES = {".ogg", ".oga", ".wav", ".flac", ".mp3", ".npy", ".mid", ".midi"}


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


def section_time_offsets(manifest: dict) -> dict[str, float]:
    """Return manifest section start times keyed by section id.

    Dynamic section-stem cues render each section's audio from local time zero.
    Reports that concatenate diagnostics over the soundtrack must reapply these
    manifest offsets; otherwise every section overlays at t=0 and the plots are
    misleading for layered encounter music.
    """
    offsets: dict[str, float] = {}
    for section in manifest.get("sections") or []:
        if not isinstance(section, dict):
            continue
        section_id = section.get("id")
        if section_id is None:
            continue
        offsets[str(section_id)] = float(section.get("start_seconds", 0.0) or 0.0)
    return offsets


def ordered_section_ids(manifest: dict) -> list[str]:
    return [
        str(section.get("id"))
        for section in manifest.get("sections") or []
        if isinstance(section, dict) and section.get("id") is not None
    ]


def adjacent_section_pairs(manifest: dict) -> list[tuple[str, str]]:
    sections = ordered_section_ids(manifest)
    return list(zip(sections, sections[1:]))


def manifest_audio_entries(manifest: dict) -> list[dict[str, str]]:
    """Return audio files explicitly referenced by an adaptive manifest.

    This intentionally ignores any extra files sitting in preview/ or adaptive/.
    Bundles and reports must be hash/manifest scoped so stale renders do not
    contaminate diagnostics.
    """
    entries: list[dict[str, str]] = []
    files = manifest.get("files") or {}
    preview = files.get("preview") or {}
    if isinstance(preview, dict):
        for name, rel in sorted(preview.items()):
            if isinstance(rel, str):
                entries.append(
                    {
                        "kind": "preview_audio",
                        "section": "*",
                        "group": name,
                        "path": rel,
                    }
                )
    adaptive = files.get("adaptive") or {}
    if isinstance(adaptive, dict):
        for section_id, section_files in sorted(adaptive.items()):
            if not isinstance(section_files, dict):
                continue
            for group, rel in sorted(section_files.items()):
                if isinstance(rel, str):
                    entries.append(
                        {
                            "kind": "adaptive_audio",
                            "section": section_id,
                            "group": group,
                            "path": rel,
                        }
                    )
    return entries


def current_scratch_stem_paths(outdir: Path, manifest: dict) -> list[Path]:
    """Return scratch stem buffers for this manifest hash only."""
    scratch_dir = outdir / "scratch_stems"
    if not scratch_dir.is_dir():
        return []
    cue_id = str(manifest.get("id", ""))
    render_hash = str(manifest.get("hash", ""))
    if cue_id and render_hash:
        return sorted(scratch_dir.glob(f"{cue_id}_{render_hash}.*.npy"))
    return sorted(scratch_dir.glob("*.npy"))


def copy_current_scratch_stems(outdir: Path, manifest: dict, dest_root: Path) -> list[str]:
    copied: list[str] = []
    for src in current_scratch_stem_paths(outdir, manifest):
        rel = Path("scratch_stems") / src.name
        dst = dest_root / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)
        copied.append(str(rel))
    return copied


def copy_manifest_referenced_files(outdir: Path, manifest: dict, bundle_dir: Path) -> list[str]:
    copied: list[str] = []
    for entry in manifest_audio_entries(manifest):
        rel = Path(entry["path"])
        src = outdir / rel
        if not src.exists():
            continue
        dst = bundle_dir / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)
        copied.append(str(rel))
    return copied


def prepare_manifest_analysis_root(outdir: Path, manifest: dict, analysis_root: Path) -> Path:
    """Create a clean manifest-scoped tree for external diagnostic scripts.

    Several legacy analysis helpers scan entire ``preview/``, ``adaptive/`` or
    ``scratch_stems/`` directories. Running them directly on a long-lived output
    directory lets stale render hashes pollute reports. This helper builds the
    small tree those tools expect, but containing only files referenced by the
    current manifest plus scratch stems matching the current render hash.
    """
    if analysis_root.exists():
        shutil.rmtree(analysis_root)
    analysis_root.mkdir(parents=True, exist_ok=True)
    copy_manifest_referenced_files(outdir, manifest, analysis_root)
    copy_current_scratch_stems(outdir, manifest, analysis_root)
    return analysis_root


def write_manifest_audio_level_report(outdir: Path, manifest: dict, reports_dir: Path) -> Path:
    """Write level stats for manifest-referenced audio only."""
    reports_dir.mkdir(parents=True, exist_ok=True)
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
    rows: list[dict[str, object]] = []
    for entry in manifest_audio_entries(manifest):
        path = outdir / entry["path"]
        stats, error = _read_audio_stats(path)
        rows.append({**entry, **(stats or {}), "error": error or ""})

    out = reports_dir / "manifest_audio_levels.tsv"
    lines = ["\t".join(columns)]
    for row in rows:
        cells: list[str] = []
        for col in columns:
            value = row.get(col, "")
            cells.append(f"{value:.3f}" if isinstance(value, float) else str(value))
        lines.append("\t".join(cells))
    out.write_text("\n".join(lines) + "\n", encoding="utf8")
    (reports_dir / "manifest_audio_levels.json").write_text(
        json.dumps({"rows": rows}, indent=2), encoding="utf8"
    )
    return out


def summarize_mix_diagnostics(manifest: dict, reports_dir: Path) -> tuple[Path, list[str]]:
    """Write human-readable mix diagnostics from manifest renderer stats."""
    reports_dir.mkdir(parents=True, exist_ok=True)
    diagnostics = manifest.get("diagnostics") or {}
    warnings = list(diagnostics.get("warnings") or []) if isinstance(diagnostics, dict) else []
    lines: list[str] = []
    lines.append(f"cue: {manifest.get('id', 'unknown')}")
    lines.append(f"hash: {manifest.get('hash', 'unknown')}")
    lines.append(f"runtime_stem_gain_mode: {manifest.get('runtime_stem_gain_mode', 'native')}")
    if isinstance(diagnostics, dict):
        raw = diagnostics.get("raw_full") or {}
        mastered = diagnostics.get("mastered_full") or {}
        lines.append("")
        lines.append("raw all-stem reference:")
        lines.append(f"  rms_dbfs: {raw.get('rms_dbfs', 'n/a')}")
        lines.append(f"  peak_dbfs: {raw.get('peak_dbfs', 'n/a')}")
        lines.append("mastered full preview:")
        lines.append(f"  rms_dbfs: {mastered.get('rms_dbfs', 'n/a')}")
        lines.append(f"  peak_dbfs: {mastered.get('peak_dbfs', 'n/a')}")
        lines.append(f"master_rms_lift_db: {diagnostics.get('master_rms_lift_db', 'n/a')}")
        lines.append(f"runtime_gain_db: {diagnostics.get('runtime_gain_db', 'n/a')}")
        lines.append(f"runtime_gain_reason: {diagnostics.get('runtime_gain_reason', 'n/a')}")
        native = diagnostics.get("native_stems") or {}
        runtime = diagnostics.get("runtime_stems") or {}
        if isinstance(native, dict) and native:
            lines.append("")
            lines.append("native stem rms/peak:")
            for group, stats in sorted(native.items()):
                if isinstance(stats, dict):
                    lines.append(
                        f"  {group}: rms {stats.get('rms_dbfs', 'n/a')} dBFS, "
                        f"peak {stats.get('peak_dbfs', 'n/a')} dBFS"
                    )
        if isinstance(runtime, dict) and runtime and runtime != native:
            lines.append("")
            lines.append("runtime export stem rms/peak:")
            for group, stats in sorted(runtime.items()):
                if isinstance(stats, dict):
                    lines.append(
                        f"  {group}: rms {stats.get('rms_dbfs', 'n/a')} dBFS, "
                        f"peak {stats.get('peak_dbfs', 'n/a')} dBFS"
                    )
    if warnings:
        lines.append("")
        lines.append("warnings:")
        for warning in warnings:
            lines.append(f"  - {warning}")
    else:
        lines.append("")
        lines.append("warnings: none")
    out = reports_dir / "mix_diagnostics.txt"
    out.write_text("\n".join(lines) + "\n", encoding="utf8")
    (reports_dir / "mix_diagnostics.json").write_text(
        json.dumps({"diagnostics": diagnostics, "warnings": warnings}, indent=2),
        encoding="utf8",
    )
    return out, warnings


def write_state_mix_report(spec: dict, manifest: dict, reports_dir: Path) -> Path:
    """Describe how different preview states differ.

    State previews can sound nearly identical when they use the same section and
    only scale the same stems by small amounts. This report makes that explicit
    so normalized audition previews are not mistaken for distinct adaptive music.

    Dynamic cues often use ``preferred_section`` rather than ``section`` and may
    include event-style states with ``fade_in`` overlays. Keep those visible so
    the report remains useful for encounter music with intro/wave/fallback/outro
    states.
    """
    reports_dir.mkdir(parents=True, exist_ok=True)
    state_map = spec.get("state_map") or {}
    groups = sorted({inst.get("group", inst.get("name")) for inst in spec.get("instruments", [])})
    rows: list[dict[str, object]] = []
    for name, cfg in sorted(state_map.items()):
        if not isinstance(cfg, dict):
            continue
        section = cfg.get("section") or cfg.get("preferred_section") or cfg.get("outro")
        weight_source = "stems"
        stems = cfg.get("stems")
        if not isinstance(stems, dict):
            stems = cfg.get("fade_in")
            weight_source = "fade_in" if isinstance(stems, dict) else "none"
        if not isinstance(stems, dict):
            stems = {}
        vector = {g: float(stems.get(g, 0.0)) for g in groups}
        rows.append(
            {
                "state": name,
                "section": section,
                "weights": vector,
                "weight_source": weight_source,
                "active_stems": [g for g, v in vector.items() if v > 0.0],
                "weight_sum": sum(vector.values()),
                "transition": cfg.get("transition"),
                "fade_beats": cfg.get("fade_beats"),
            }
        )

    by_state = {str(row["state"]): row for row in rows}
    default = by_state.get("default")
    baseline_note = "default state"
    if default is None:
        default = next((row for row in rows if float(row.get("weight_sum", 0.0)) > 0.0), None)
        if default is not None:
            baseline_note = f"first state with explicit stem weights: {default.get('state')}"
    if default is None and rows:
        default = rows[0]
        baseline_note = f"first listed state: {default.get('state')}"

    distances: list[dict[str, object]] = []
    if default is not None:
        base = default["weights"]
        assert isinstance(base, dict)
        base_norm = math.sqrt(sum(float(v) * float(v) for v in base.values()))
        for row in rows:
            vec = row["weights"]
            assert isinstance(vec, dict)
            diff = {g: float(vec.get(g, 0.0)) - float(base.get(g, 0.0)) for g in groups}
            l2 = math.sqrt(sum(v * v for v in diff.values()))
            denom = max(base_norm, 1.0)
            distances.append(
                {
                    "state": row["state"],
                    "section": row["section"],
                    "distance_from_baseline": l2,
                    "relative_distance_from_baseline": l2 / denom,
                    # Backward-compatible keys.
                    "distance_from_default": l2,
                    "relative_distance_from_default": l2 / denom,
                    "changed_stems": {g: round(v, 4) for g, v in diff.items() if abs(v) > 1e-9},
                }
            )

    preview_stats = (((manifest.get("diagnostics") or {}).get("runtime_previews") or {}))
    payload = {
        "schema": "ambition.music_state_mix_report.v1",
        "cue": spec.get("id"),
        "states": rows,
        "baseline_state": default.get("state") if isinstance(default, dict) else None,
        "baseline_note": baseline_note,
        "distances_from_default": distances,
        "runtime_preview_stats": preview_stats,
        "note": (
            "runtime_* previews are weighted stem sums without upward audition normalization; "
            "audition_* previews are normalized for comfortable listening and may collapse loudness differences. "
            "States using fade_in are overlay events, not full replacement mixes."
        ),
    }
    json_path = reports_dir / "state_mix_report.json"
    json_path.write_text(json.dumps(payload, indent=2), encoding="utf8")

    tsv_path = reports_dir / "state_mix_report.tsv"
    columns = ["state", "section", "weight_source", "weight_sum", "distance_from_baseline", "relative_distance_from_baseline", "weights"]
    distance_by_state = {str(row["state"]): row for row in distances}
    lines = ["\t".join(columns)]
    for row in rows:
        dist = distance_by_state.get(str(row["state"]), {})
        weights = row.get("weights", {})
        weight_text = ",".join(f"{g}:{float(v):.3f}" for g, v in sorted(weights.items())) if isinstance(weights, dict) else ""
        lines.append(
            "\t".join(
                [
                    str(row.get("state", "")),
                    str(row.get("section", "")),
                    str(row.get("weight_source", "")),
                    f"{float(row.get('weight_sum', 0.0)):.3f}",
                    f"{float(dist.get('distance_from_baseline', 0.0)):.3f}",
                    f"{float(dist.get('relative_distance_from_baseline', 0.0)):.3f}",
                    weight_text,
                ]
            )
        )
    tsv_path.write_text("\n".join(lines) + "\n", encoding="utf8")

    summary = reports_dir / "state_mix_report_summary.txt"
    text: list[str] = [
        f"cue: {spec.get('id')}",
        "runtime previews are native weighted sums; audition previews are normalized.",
        f"baseline: {baseline_note}",
        "",
        "state distances from default/baseline:",
    ]
    for dist in distances:
        text.append(
            f"  {dist.get('state')}: rel {float(dist.get('relative_distance_from_baseline', 0.0)):.2f} "
            f"section {dist.get('section')} changed {dist.get('changed_stems')}"
        )
    if not by_state.get("default"):
        text.append("")
        text.append("note: no explicit default state; reports use the first state with explicit stem weights as the baseline.")
    if rows:
        no_stem_states = [str(row["state"]) for row in rows if float(row.get("weight_sum", 0.0)) <= 0.0]
        if no_stem_states:
            text.append("note: states without explicit stem weights: " + ", ".join(no_stem_states))
    if distances:
        non_base = [d for d in distances if d.get("state") != (default or {}).get("state")]
        if non_base and max(float(d.get("relative_distance_from_baseline", 0.0)) for d in non_base) < 0.35:
            text.append("")
            text.append("warning: state maps are close together; previews may sound mostly like level variants.")
    summary.write_text("\n".join(text) + "\n", encoding="utf8")
    return json_path


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

    for npy in current_scratch_stem_paths(outdir, manifest):
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

    for entry in manifest_audio_entries(manifest):
        path = outdir / entry["path"]
        stats, error = _read_audio_stats(path)
        rows.append({**entry, **(stats or {}), "error": error or ""})

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



def write_spectral_fingerprint(
    outdir: Path,
    manifest: dict,
    reports_dir: Path,
    *,
    bucket_seconds: float = 1.0,
    max_events_per_band: int = 24,
) -> Path:
    """Write compact, LLM-friendly spectral summaries from scratch stems.

    The PNG/JPEG spectrograms are useful for human/vision review, but a chat
    agent can reason much more reliably from small JSON/TSV summaries. This
    report mirrors the broad bands used by ``spectral_localize.py`` and records
    per-band group fractions plus the strongest dominant time buckets.
    """
    reports_dir.mkdir(parents=True, exist_ok=True)
    sample_rate = int(manifest.get("sample_rate", 48000))
    bands = [
        ("low", 0.0, 300.0),
        ("mid", 300.0, 1000.0),
        ("high", 1000.0, 3000.0),
        ("vhigh", 3000.0, 6000.0),
        ("air", 6000.0, 12000.0),
    ]
    paths = current_scratch_stem_paths(outdir, manifest)
    groups: list[str] = []
    audios: dict[str, np.ndarray] = {}
    max_frames = 0
    for path in paths:
        group = path.stem.split(".")[-1]
        try:
            arr = np.load(path).astype("float32", copy=False)
            arr = r._coerce_stereo(arr)
        except Exception:
            continue
        groups.append(group)
        audios[group] = arr.mean(axis=1).astype("float32", copy=False)
        max_frames = max(max_frames, len(audios[group]))

    frames_per_bucket = max(1, int(round(bucket_seconds * sample_rate)))
    bucket_count = max(1, int(math.ceil(max_frames / frames_per_bucket))) if max_frames else 0
    energy = {
        group: {band[0]: [0.0 for _ in range(bucket_count)] for band in bands}
        for group in groups
    }
    for group, mono in audios.items():
        for idx in range(bucket_count):
            start = idx * frames_per_bucket
            stop = min(len(mono), start + frames_per_bucket)
            chunk = mono[start:stop]
            if len(chunk) < 16:
                continue
            window = np.hanning(len(chunk)).astype("float32")
            spectrum = np.fft.rfft(chunk * window)
            freqs = np.fft.rfftfreq(len(chunk), d=1.0 / sample_rate)
            power = np.square(np.abs(spectrum)).astype("float64")
            for name, lo, hi in bands:
                mask = (freqs >= lo) & (freqs < hi)
                energy[group][name][idx] = float(power[mask].sum()) if np.any(mask) else 0.0

    mean_fractions: dict[str, dict[str, float]] = {}
    for name, _lo, _hi in bands:
        totals = {group: float(np.sum(energy[group][name])) for group in groups}
        denom = sum(totals.values())
        mean_fractions[name] = {
            group: (totals[group] / denom if denom > 0.0 else 0.0)
            for group in groups
        }

    dominant_events: dict[str, list[dict[str, object]]] = {}
    for name, _lo, _hi in bands:
        events: list[dict[str, object]] = []
        for idx in range(bucket_count):
            bucket_values = {group: energy[group][name][idx] for group in groups}
            total = sum(bucket_values.values())
            if total <= 0.0:
                continue
            top_group, top_energy = max(bucket_values.items(), key=lambda item: item[1])
            share = top_energy / total
            events.append(
                {
                    "time_start_s": round(idx * bucket_seconds, 3),
                    "time_end_s": round(min((idx + 1) * bucket_seconds, manifest_duration(manifest)), 3),
                    "group": top_group,
                    "share": share,
                    "band_energy": top_energy,
                }
            )
        events.sort(key=lambda row: (float(row["share"]), float(row["band_energy"])), reverse=True)
        dominant_events[name] = events[:max_events_per_band]

    payload = {
        "schema": "ambition.music_spectral_fingerprint.v1",
        "cue": manifest.get("id"),
        "hash": manifest.get("hash"),
        "sample_rate": sample_rate,
        "bucket_seconds": bucket_seconds,
        "groups": groups,
        "bands": [
            {"name": name, "low_hz": lo, "high_hz": hi} for name, lo, hi in bands
        ],
        "mean_band_fraction_by_group": mean_fractions,
        "dominant_events": dominant_events,
    }
    json_path = reports_dir / "spectral_fingerprint.json"
    json_path.write_text(json.dumps(payload, indent=2), encoding="utf8")

    tsv = reports_dir / "spectral_fingerprint.tsv"
    lines = ["band\tgroup\tmean_fraction"]
    for band_name, fractions in mean_fractions.items():
        for group, fraction in sorted(fractions.items(), key=lambda item: item[1], reverse=True):
            lines.append(f"{band_name}\t{group}\t{fraction:.6f}")
    tsv.write_text("\n".join(lines) + "\n", encoding="utf8")

    summary = reports_dir / "spectral_fingerprint_summary.txt"
    text_lines: list[str] = [
        f"cue: {manifest.get('id')}",
        f"hash: {manifest.get('hash')}",
        f"bucket_seconds: {bucket_seconds}",
        "",
        "mean band fraction by group:",
    ]
    for band_name, fractions in mean_fractions.items():
        ordered = sorted(fractions.items(), key=lambda item: item[1], reverse=True)
        pieces = [f"{group} {fraction * 100:.1f}%" for group, fraction in ordered]
        text_lines.append(f"  {band_name}: " + ", ".join(pieces))
    text_lines.append("")
    text_lines.append("top dominant events:")
    for band_name, events in dominant_events.items():
        text_lines.append(f"  {band_name}:")
        for event in events[:8]:
            text_lines.append(
                f"    {event['time_start_s']:>6.2f}-{event['time_end_s']:>6.2f}s "
                f"{event['group']} {float(event['share']) * 100:.1f}%"
            )
    summary.write_text("\n".join(text_lines) + "\n", encoding="utf8")
    return json_path


def _rms_envelope(audio: np.ndarray, sample_rate: int, bucket_seconds: float) -> list[dict[str, float]]:
    """Return a short-time RMS envelope for plotting and report tables."""
    mono = audio.mean(axis=1).astype("float32", copy=False) if audio.ndim == 2 else audio.astype("float32", copy=False)
    hop = max(1, int(round(sample_rate * bucket_seconds)))
    rows: list[dict[str, float]] = []
    for start in range(0, len(mono), hop):
        stop = min(len(mono), start + hop)
        chunk = mono[start:stop]
        rms = float(np.sqrt(np.mean(np.square(chunk), dtype=np.float64))) if chunk.size else 0.0
        peak = float(np.max(np.abs(chunk))) if chunk.size else 0.0
        rows.append({
            "time_start_s": float(start / sample_rate),
            "time_end_s": float(stop / sample_rate),
            "rms_dbfs": _db(rms),
            "peak_dbfs": _db(peak),
            "rms_linear": rms,
            "peak_linear": peak,
        })
    return rows


def write_stem_amplitude_report(
    outdir: Path,
    spec: dict,
    manifest: dict,
    reports_dir: Path,
    plots_dir: Path | None = None,
    *,
    bucket_seconds: float = 0.5,
    plot_format: str = "jpg",
    jpeg_quality: int = 84,
) -> Path:
    """Write section-aware stem-level amplitude reports and plots.

    Adaptive section-stem cues contain the same group name in multiple section
    directories. Older reports keyed rows only by group, which meant later
    sections overwrote earlier sections and all envelopes were plotted from
    local t=0. This version keeps section/group rows distinct and adds absolute
    soundtrack time from the adaptive manifest.
    """
    reports_dir.mkdir(parents=True, exist_ok=True)
    if plots_dir is not None:
        plots_dir.mkdir(parents=True, exist_ok=True)
    state_map = spec.get("state_map") or {}
    state_weights: dict[str, dict[str, float]] = {}
    if isinstance(state_map, dict):
        for state, cfg in state_map.items():
            if not isinstance(cfg, dict):
                continue
            stems = cfg.get("stems")
            if not isinstance(stems, dict):
                stems = cfg.get("fade_in")
            if isinstance(stems, dict):
                state_weights[str(state)] = {str(k): float(v) for k, v in stems.items()}

    default_weights = state_weights.get("default")
    default_is_raw_reference = default_weights is None
    offsets = section_time_offsets(manifest)

    rows_by_section_group: dict[tuple[str, str], dict[str, object]] = {}
    envelope_rows: list[dict[str, object]] = []
    sample_rate = int(manifest.get("sample_rate", 48000))
    for entry in manifest_audio_entries(manifest):
        if entry.get("kind") != "adaptive_audio":
            continue
        section = str(entry.get("section", ""))
        group = str(entry.get("group", ""))
        if not group or group == "full":
            continue
        path = outdir / str(entry["path"])
        try:
            import soundfile as sf
            audio, sr = sf.read(path, always_2d=True, dtype="float32")
            sample_rate = int(sr)
        except Exception as ex:  # noqa: BLE001
            rows_by_section_group[(section, group)] = {
                "group": group,
                "section": section,
                "section_start_s": offsets.get(section, 0.0),
                "path": entry.get("path"),
                "error": f"{type(ex).__name__}: {ex}",
            }
            continue
        stats = _audio_stats(audio.astype("float32", copy=False), sample_rate)
        default_weight = 1.0 if default_weights is None else float(default_weights.get(group, 0.0))
        row: dict[str, object] = {
            "group": group,
            "section": section,
            "section_start_s": offsets.get(section, 0.0),
            "path": entry.get("path"),
            "state_default_weight": default_weight,
            "state_default_is_raw_reference": default_is_raw_reference,
            "rms_dbfs": stats["rms_dbfs"],
            "peak_dbfs": stats["peak_dbfs"],
            "duration_s": stats["duration_s"],
            "weighted_default_rms_dbfs": stats["rms_dbfs"] + _db(default_weight) if default_weight > 0 else -120.0,
            "weighted_default_peak_dbfs": stats["peak_dbfs"] + _db(default_weight) if default_weight > 0 else -120.0,
            "error": "",
        }
        for state, weights in sorted(state_weights.items()):
            weight = float(weights.get(group, 0.0))
            row[f"state_{state}_weight"] = weight
            row[f"state_{state}_rms_dbfs"] = stats["rms_dbfs"] + _db(weight) if weight > 0 else -120.0
        rows_by_section_group[(section, group)] = row
        section_offset = float(offsets.get(section, 0.0))
        for env in _rms_envelope(audio, sample_rate, bucket_seconds):
            default_linear = float(env["rms_linear"] * default_weight)
            env_row: dict[str, object] = {
                "group": group,
                "section": section,
                "section_start_s": section_offset,
                "time_start_s": env["time_start_s"],
                "time_end_s": env["time_end_s"],
                "time_start_s_absolute": section_offset + float(env["time_start_s"]),
                "time_end_s_absolute": section_offset + float(env["time_end_s"]),
                "rms_dbfs": env["rms_dbfs"],
                "peak_dbfs": env["peak_dbfs"],
                "rms_linear": env["rms_linear"],
                "peak_linear": env["peak_linear"],
                "state_default_rms_linear": default_linear,
                "state_default_rms_dbfs": _db(default_linear),
            }
            envelope_rows.append(env_row)

    ordered_groups = sorted(
        rows_by_section_group.values(),
        key=lambda row: (
            str(row.get("section", "")),
            -float(row.get("weighted_default_rms_dbfs", -120.0)),
            str(row.get("group", "")),
        ),
    )
    payload = {
        "schema": "ambition.music_stem_amplitude.v1",
        "cue": manifest.get("id"),
        "hash": manifest.get("hash"),
        "bucket_seconds": bucket_seconds,
        # Backward-compatible key name; rows are now section/group rows.
        "groups": ordered_groups,
        "section_group_rows": ordered_groups,
        "envelope_rows": envelope_rows,
        "state_weights": state_weights,
        "default_is_raw_reference": default_is_raw_reference,
        "note": (
            "Rows are section/group scoped. When no explicit default state exists, "
            "weighted_default_* is a raw unweighted reference for plots, not a runtime default."
        ),
    }
    json_path = reports_dir / "stem_amplitude.json"
    json_path.write_text(json.dumps(payload, indent=2), encoding="utf8")

    columns = [
        "section",
        "group",
        "section_start_s",
        "state_default_weight",
        "rms_dbfs",
        "weighted_default_rms_dbfs",
        "peak_dbfs",
        "weighted_default_peak_dbfs",
        "duration_s",
        "path",
        "error",
    ]
    tsv_path = reports_dir / "stem_amplitude.tsv"
    lines = ["\t".join(columns)]
    for row in ordered_groups:
        lines.append("\t".join(f"{row.get(c, ''):.3f}" if isinstance(row.get(c, ""), float) else str(row.get(c, "")) for c in columns))
    tsv_path.write_text("\n".join(lines) + "\n", encoding="utf8")

    env_columns = [
        "section",
        "group",
        "time_start_s",
        "time_end_s",
        "time_start_s_absolute",
        "time_end_s_absolute",
        "rms_dbfs",
        "peak_dbfs",
        "state_default_rms_dbfs",
    ]
    env_tsv = reports_dir / "stem_amplitude_envelope.tsv"
    env_lines = ["\t".join(env_columns)]
    for row in envelope_rows:
        env_lines.append("\t".join(f"{row.get(c, ''):.3f}" if isinstance(row.get(c, ""), float) else str(row.get(c, "")) for c in env_columns))
    env_tsv.write_text("\n".join(env_lines) + "\n", encoding="utf8")

    summary = reports_dir / "stem_amplitude_summary.txt"
    text_lines = [
        f"cue: {manifest.get('id')}",
        f"hash: {manifest.get('hash')}",
        f"bucket_seconds: {bucket_seconds}",
        "section/group scoped: true",
    ]
    if default_is_raw_reference:
        text_lines.append("note: no explicit default state; weighted_default values use raw stem weight 1.0.")
    text_lines.extend(["", "section stem levels:"])
    by_section: dict[str, list[dict[str, object]]] = {}
    for row in ordered_groups:
        by_section.setdefault(str(row.get("section", "")), []).append(row)
    for section in ordered_section_ids(manifest) or sorted(by_section):
        rows = sorted(
            by_section.get(section, []),
            key=lambda row: float(row.get("weighted_default_rms_dbfs", -120.0)),
            reverse=True,
        )
        if not rows:
            continue
        text_lines.append(f"  {section}:")
        top = float(rows[0].get("weighted_default_rms_dbfs", -120.0))
        for row in rows:
            if row.get("error"):
                text_lines.append(f"    {row.get('group')}: ERROR {row.get('error')}")
            else:
                rel = float(row.get("weighted_default_rms_dbfs", -120.0)) - top
                text_lines.append(
                    f"    {row.get('group')}: raw {float(row.get('rms_dbfs', -120.0)):.1f} dBFS, "
                    f"weighted {float(row.get('weighted_default_rms_dbfs', -120.0)):.1f} dBFS, "
                    f"rel {rel:+.1f} dB, weight {float(row.get('state_default_weight', 0.0)):.2f}"
                )
    summary.write_text("\n".join(text_lines) + "\n", encoding="utf8")

    if plots_dir is not None and ordered_groups:
        try:
            import matplotlib.pyplot as plt
            suffix = "jpg" if plot_format in {"jpg", "jpeg"} else "png"
            save_kwargs: dict[str, object] = {"dpi": 130, "bbox_inches": "tight"}
            if suffix == "jpg":
                save_kwargs["format"] = "jpeg"
                save_kwargs["pil_kwargs"] = {"quality": int(jpeg_quality), "optimize": True}
            labels = [f"{row['section']}/{row['group']}" for row in ordered_groups if not row.get("error")]
            values = [float(row.get("weighted_default_rms_dbfs", -120.0)) for row in ordered_groups if not row.get("error")]
            if labels:
                fig, ax = plt.subplots(figsize=(9, max(3.5, 0.28 * len(labels) + 1.5)))
                positions = np.arange(len(labels))
                ax.barh(positions, values)
                ax.set_yticks(positions, labels=labels, fontsize=7)
                ax.invert_yaxis()
                ax.set_xlabel("weighted RMS (dBFS)")
                ax.set_title("Section/stem amplitude balance")
                ax.grid(True, axis="x", alpha=0.3)
                fig.savefig(plots_dir / f"stem_amplitude_balance.{suffix}", **save_kwargs)
                plt.close(fig)
            by_group: dict[str, list[dict[str, object]]] = {}
            for row in envelope_rows:
                by_group.setdefault(str(row["group"]), []).append(row)
            if by_group:
                fig, ax = plt.subplots(figsize=(12, 4.8))
                for group in sorted(by_group):
                    rows = sorted(by_group[group], key=lambda r0: float(r0["time_start_s_absolute"]))
                    xs = [(float(r0["time_start_s_absolute"]) + float(r0["time_end_s_absolute"])) * 0.5 for r0 in rows]
                    ys = [float(r0.get("state_default_rms_dbfs", -120.0)) for r0 in rows]
                    ax.plot(xs, ys, label=group)
                for section, start in sorted(offsets.items(), key=lambda item: item[1]):
                    if start > 0.0:
                        ax.axvline(start, alpha=0.18, linewidth=0.8)
                ax.set_xlabel("absolute soundtrack time (s)")
                ax.set_ylabel("weighted RMS (dBFS)")
                ax.set_title("Stem amplitude over absolute section time")
                ax.grid(True, alpha=0.3)
                ax.legend(loc="best", fontsize=8)
                fig.savefig(plots_dir / f"stem_amplitude_timeline.{suffix}", **save_kwargs)
                plt.close(fig)

                bucket_centers = sorted({
                    round((float(r0["time_start_s_absolute"]) + float(r0["time_end_s_absolute"])) * 0.5, 6)
                    for r0 in envelope_rows
                })
                index = {x: i for i, x in enumerate(bucket_centers)}
                stack_values = []
                stack_labels = []
                for group in sorted(by_group):
                    vals = [0.0 for _ in bucket_centers]
                    for r0 in by_group[group]:
                        x = round((float(r0["time_start_s_absolute"]) + float(r0["time_end_s_absolute"])) * 0.5, 6)
                        vals[index[x]] = float(r0.get("state_default_rms_linear", 0.0))
                    stack_values.append(vals)
                    stack_labels.append(group)
                if bucket_centers and stack_values:
                    fig, ax = plt.subplots(figsize=(12, 4.8))
                    ax.stackplot(bucket_centers, stack_values, labels=stack_labels)
                    for section, start in sorted(offsets.items(), key=lambda item: item[1]):
                        if start > 0.0:
                            ax.axvline(start, alpha=0.18, linewidth=0.8)
                    ax.set_xlabel("absolute soundtrack time (s)")
                    ax.set_ylabel("weighted RMS magnitude")
                    ax.set_title("Section-aware stem amplitude stack")
                    ax.legend(loc="best", fontsize=8)
                    fig.savefig(plots_dir / f"stem_amplitude_stack.{suffix}", **save_kwargs)
                    plt.close(fig)
        except Exception as ex:  # noqa: BLE001
            (plots_dir / "stem_amplitude_plots_skipped.txt").write_text(f"stem amplitude plot generation skipped: {type(ex).__name__}: {ex}\n", encoding="utf8")
    return json_path


def write_spectrograms(
    outdir: Path,
    manifest: dict,
    plots_dir: Path,
    *,
    limit: int = 16,
    plot_format: str = "jpg",
    jpeg_quality: int = 84,
) -> list[Path]:
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

    def save_audio_plot(audio: np.ndarray, title: str, dest: Path) -> None:
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
        save_kwargs = {"dpi": 120}
        if dest.suffix.lower() in {".jpg", ".jpeg"}:
            save_kwargs["format"] = "jpeg"
            save_kwargs["pil_kwargs"] = {"quality": int(jpeg_quality), "optimize": True}
        plt.savefig(dest, **save_kwargs)
        plt.close()

    candidates: list[tuple[str, Path, str]] = []
    for npy in current_scratch_stem_paths(outdir, manifest):
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
            suffix = "jpg" if plot_format in {"jpg", "jpeg"} else "png"
            dest = plots_dir / f"{label}.spectrogram.{suffix}"
            save_audio_plot(audio, label, dest)
            if dest.exists():
                written.append(dest)
        except Exception as ex:  # noqa: BLE001
            (plots_dir / f"{label}.spectrogram.error.txt").write_text(
                f"failed to render {path}: {type(ex).__name__}: {ex}\n",
                encoding="utf8",
            )
    return written


def run_transition_audits(
    analysis_root: Path,
    manifest: dict,
    reports_dir: Path,
    tools_dir: Path,
    *,
    max_pairs: int = 8,
    crossfade_seconds: float = 0.65,
    crossfade_shape: str = "equal_power",
) -> list[CommandResult]:
    """Run audio seam diagnostics for adjacent adaptive sections.

    The generated report zip omits WAV previews, but keeping transition metrics,
    envelopes, and spectrogram PNGs in the bundle makes dynamic encounter cues
    auditable without opening the game.
    """
    results: list[CommandResult] = []
    pairs = adjacent_section_pairs(manifest)[:max_pairs]
    if not pairs:
        return results
    audit_script = (tools_dir / "transition_audit.py").resolve()
    if not audit_script.exists():
        return results
    audit_root = reports_dir / "transition_audit"
    audit_root.mkdir(parents=True, exist_ok=True)
    for first, second in pairs:
        outdir = audit_root / f"{first}_to_{second}"
        cmd = [
            sys.executable,
            str(audit_script),
            str(analysis_root),
            "--sections",
            first,
            second,
            "--crossfade",
            str(crossfade_seconds),
            "--crossfade-shape",
            crossfade_shape,
            "--outdir",
            str(outdir),
        ]
        safe_name = f"transition_audit_{first}_to_{second}".replace("/", "_")
        results.append(run_logged(safe_name, cmd, reports_dir, cwd=tools_dir))
    return results


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


def should_include_in_report_zip(path: Path) -> bool:
    """Return True for compact, LLM-friendly bundle artifacts.

    Report zips are meant for chat/agent upload: keep source YAML, manifests,
    text/JSON/TSV diagnostics, rerun scripts, and spectrogram images, but omit
    heavyweight binary audio and raw NumPy/MIDI intermediates. The full bundle
    directory on disk remains complete either way.
    """
    return path.suffix.lower() not in REPORT_ZIP_EXCLUDED_SUFFIXES


def make_zip(src_dir: Path, zip_path: Path, *, report_only: bool = False) -> Path:
    zip_path.parent.mkdir(parents=True, exist_ok=True)
    if zip_path.exists():
        zip_path.unlink()
    with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for path in sorted(src_dir.rglob("*")):
            if path == zip_path or path.is_dir():
                continue
            if report_only and not should_include_in_report_zip(path):
                continue
            zf.write(path, path.relative_to(src_dir.parent))
    return zip_path


def file_uri(path: Path) -> str:
    return path.resolve().as_uri()


def terminal_link(path: Path, label: str | None = None) -> str:
    """Return an OSC-8 terminal hyperlink with a plain absolute-path label.

    Terminals that do not support OSC-8 still show a ctrl-clickable absolute
    path. This keeps command output ergonomic without requiring a rich console
    dependency.
    """
    path = path.resolve()
    shown = label or str(path)
    return f"\033]8;;{path.as_uri()}\033\\{shown}\033]8;;\033\\"


def progress_line(message: str, *, stream=None) -> None:
    """Emit a visible progress update for long bundle workflows."""
    if stream is None:
        stream = sys.stderr
    print(f"[music bundle] {message}", file=stream, flush=True)


def print_bundle_summary(report: dict[str, object], *, stream=None) -> None:
    """Print human-friendly paths in addition to the machine-readable JSON."""
    if stream is None:
        stream = sys.stderr
    keys = [
        ("render output", "outdir"),
        ("bundle dir", "bundle_dir"),
        ("manifest", "manifest"),
        ("full zip", "zip"),
        ("report zip", "zip_report"),
        ("published", "published"),
    ]
    print("\nMusic bundle outputs:", file=stream)
    for label, key in keys:
        value = report.get(key)
        if not value or value == "publish failed":
            continue
        path = Path(str(value))
        print(f"  {label:13s}: {terminal_link(path)}", file=stream)
    if report.get("warnings"):
        print("  warnings     :", file=stream)
        for warning in report.get("warnings", []):
            print(f"    - {warning}", file=stream)
    print("", file=stream)


def build_rerun_script(
    bundle_dir: Path,
    cue: str,
    backend: str,
    outdir: Path,
    publish: bool,
    runtime_stem_gain_mode: str,
    plot_format: str,
    runtime_stem_max_gain_db: float | None,
    zip_bundle: bool,
    zip_report_bundle: bool,
) -> Path:
    script = bundle_dir / "rerun_bundle.sh"
    publish_flag = " --publish" if publish else ""
    cmd = [
        "PYTHONPATH=tools/ambition_music_renderer python -m ambition_music_renderer cue bundle",
        str(cue),
        "--backend",
        str(backend),
        "--runtime-stem-gain-mode",
        str(runtime_stem_gain_mode),
    ]
    if runtime_stem_max_gain_db is not None:
        cmd.extend(["--runtime-stem-max-gain-db", str(runtime_stem_max_gain_db)])
    cmd.extend(["--plot-format", str(plot_format)])
    cmd.extend(["--outdir", str(outdir), "--force"])
    if publish:
        cmd.append("--publish")
    if zip_bundle:
        cmd.append("--zip")
    if zip_report_bundle:
        cmd.append("--zip-report")
    wrapped = " \\\n  ".join(cmd)
    body = (
        "#!/usr/bin/env bash\n"
        "set -euo pipefail\n"
        "cd \"$(git rev-parse --show-toplevel)\"\n"
        f"{wrapped}\n"
    )
    script.write_text(body, encoding="utf8")
    script.chmod(0o755)
    return script


def create_bundle(
    cue: str,
    *,
    backend: str = DEFAULT_BACKEND,
    runtime_stem_gain_mode: str = "native",
    outdir: Path | None = None,
    bundle_root: Path | None = None,
    force: bool = False,
    publish: bool = False,
    dest_root: Path | None = None,
    zip_bundle: bool = False,
    zip_report_bundle: bool = False,
    jobs: int = 1,
    include_scratch_stems: bool = False,
    skip_render: bool = False,
    skip_spectrograms: bool = False,
    plot_format: str = "jpg",
    jpeg_quality: int = 84,
    runtime_stem_max_gain_db: float | None = None,
) -> dict[str, object]:
    progress_line(f"locating score for {cue!r}")
    score_path = find_score(cue)
    if score_path is None:
        raise FileNotFoundError(f"cue not found: {cue}")
    spec = load_yaml(score_path)
    cue_id = str(spec.get("id", cue))
    progress_line(f"loaded {cue_id} from {terminal_link(score_path)}")
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

    progress_line(f"render output directory: {terminal_link(outdir)}")
    progress_line(f"bundle root: {terminal_link(bundle_root)}")

    reports_dir = outdir / "reports"
    plots_dir = outdir / "plots"
    # Reports and plots are derived products for the current bundle. Clear them
    # up front so stale diagnostics from older hashes cannot contaminate a new
    # upload bundle. Audio output dirs are left alone; bundle copying is
    # manifest-scoped below.
    for derived_dir in (reports_dir, plots_dir):
        if derived_dir.exists():
            shutil.rmtree(derived_dir)
    reports_dir.mkdir(parents=True, exist_ok=True)
    commands: list[CommandResult] = []

    progress_line("running arrangement preflight")
    arrangement_payload = audit_arrangement_file(score_path)
    write_arrangement_reports(arrangement_payload, reports_dir)

    if not skip_render:
        progress_line(f"rendering {cue_id} with backend={backend}, runtime_stems={runtime_stem_gain_mode}")
        render_cmd = [
            sys.executable,
            "-m",
            "ambition_music_renderer.render_isolated",
            str(score_path),
            "--outdir",
            str(outdir),
            "--backend",
            backend,
            "--runtime-stem-gain-mode",
            runtime_stem_gain_mode,
            "--keep-debug-stems",
            "--jobs",
            str(jobs),
        ]
        if runtime_stem_max_gain_db is not None:
            render_cmd.extend(["--runtime-stem-max-gain-db", str(runtime_stem_max_gain_db)])
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

    progress_line("loading adaptive manifest")
    manifest_path = latest_manifest(outdir, cue_id)
    if manifest_path is None:
        raise FileNotFoundError(f"no adaptive manifest found in {outdir} for {cue_id}")
    manifest = json.loads(manifest_path.read_text(encoding="utf8"))
    render_hash = str(manifest.get("hash", "unknown"))
    duration = manifest_duration(manifest)

    # Diagnostics. These tools are report-only; a failure should not destroy the
    # bundle. Run directory-scanning legacy helpers against a clean manifest-
    # scoped analysis root so stale hashes in the real output dir cannot leak
    # into the reports.
    tools_dir = package_dir()
    progress_line("running manifest-scoped reports and plots")
    with tempfile.TemporaryDirectory(prefix=f"{cue_id}_{render_hash}_analysis_") as td:
        analysis_root = prepare_manifest_analysis_root(outdir, manifest, Path(td))
        commands.append(
            run_logged(
                "audit_cue_balance",
                [sys.executable, str(tools_dir / "audit_cue_balance.py"), str(analysis_root)],
                reports_dir,
                cwd=tools_dir,
            )
        )
        if (analysis_root / "scratch_stems").is_dir():
            hi = f"{duration:.3f}" if duration > 0 else "-1"
            commands.append(
                run_logged(
                    "spectral_compare",
                    [
                        sys.executable,
                        str(tools_dir / "spectral_compare.py"),
                        str(analysis_root),
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
                        str(analysis_root),
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
        write_stem_export_report(analysis_root, manifest, reports_dir)
        write_manifest_audio_level_report(analysis_root, manifest, reports_dir)
        write_stem_amplitude_report(
            analysis_root,
            spec,
            manifest,
            reports_dir,
            plots_dir=plots_dir,
            plot_format=plot_format,
            jpeg_quality=jpeg_quality,
        )
        write_spectral_fingerprint(analysis_root, manifest, reports_dir)
        write_state_mix_report(spec, manifest, reports_dir)
        progress_line("running adjacent-section transition audits")
        commands.extend(run_transition_audits(analysis_root, manifest, reports_dir, tools_dir))
        # Re-run arrangement preflight after render report cleanup so it is present in the final bundle.
        arrangement_payload = audit_arrangement_file(score_path)
        write_arrangement_reports(arrangement_payload, reports_dir)
        dissonance_payload = audit_dissonance_file(score_path)
        write_dissonance_reports(
            dissonance_payload,
            reports_dir,
            plots_dir=plots_dir,
            plot_format=plot_format,
            jpeg_quality=jpeg_quality,
        )
        sour_note_payload = audit_sour_note_file(score_path)
        write_sour_note_reports(
            sour_note_payload,
            reports_dir,
            plots_dir=plots_dir,
            plot_format=plot_format,
            jpeg_quality=jpeg_quality,
        )
        mix_diag_path, mix_warnings = summarize_mix_diagnostics(manifest, reports_dir)
        dissonance_warnings = list(dissonance_payload.get("warnings") or [])
        sour_note_warnings = list(sour_note_payload.get("warnings") or [])
        if not skip_spectrograms:
            write_spectrograms(
                analysis_root,
                manifest,
                plots_dir,
                plot_format=plot_format,
                jpeg_quality=jpeg_quality,
            )

    published: str | None = None
    if publish:
        progress_line("publishing full.ogg to game assets")
        # Import lazily so this module can be used by tests without importing the CLI.
        from .cli import publish_cue

        ok = publish_cue(cue_id, outdir, dest_root)
        if ok:
            published = str(dest_root / cue_id / "full.ogg")
        else:
            published = "publish failed"

    progress_line("assembling shareable bundle directory")
    bundle_name = f"{cue_id}_{render_hash}_bundle"
    bundle_dir = bundle_root / bundle_name
    if bundle_dir.exists():
        shutil.rmtree(bundle_dir)
    bundle_dir.mkdir(parents=True, exist_ok=True)

    source_dir = bundle_dir / "source"
    source_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(score_path, source_dir / score_path.name)
    (source_dir / "normalized_spec.json").write_text(json.dumps(spec, indent=2), encoding="utf8")
    copied_audio = copy_manifest_referenced_files(outdir, manifest, bundle_dir)
    copy_tree_if_exists(reports_dir, bundle_dir / "reports")
    copy_tree_if_exists(plots_dir, bundle_dir / "plots")
    shutil.copy2(manifest_path, bundle_dir / manifest_path.name)
    if include_scratch_stems:
        copy_current_scratch_stems(outdir, manifest, bundle_dir)

    rerun_script = build_rerun_script(
        bundle_dir,
        cue_id,
        backend,
        outdir,
        publish,
        runtime_stem_gain_mode,
        plot_format,
        runtime_stem_max_gain_db,
        zip_bundle,
        zip_report_bundle,
    )

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
        "runtime_stem_gain_mode": runtime_stem_gain_mode,
        "runtime_stem_max_gain_db": runtime_stem_max_gain_db,
        "plot_format": plot_format,
        "render_hash": render_hash,
        "outdir": str(outdir),
        "bundle_dir": str(bundle_dir),
        "manifest": str(manifest_path),
        "duration_s": duration,
        "published": published,
        "include_scratch_stems": include_scratch_stems,
        "copied_audio_files": copied_audio,
        "mix_diagnostics": str(mix_diag_path),
        "warnings": [w for w in [id_warning, *mix_warnings, *dissonance_warnings, *sour_note_warnings] if w],
        "commands": command_rows,
        "rerun_script": str(rerun_script),
    }
    (bundle_dir / "bundle_manifest.json").write_text(json.dumps(report, indent=2), encoding="utf8")

    zip_path: Path | None = None
    zip_report_path: Path | None = None
    if zip_bundle:
        zip_path = make_zip(bundle_dir, bundle_root / f"{bundle_name}.zip")
        report["zip"] = str(zip_path)
    if zip_report_bundle:
        zip_report_path = make_zip(
            bundle_dir, bundle_root / f"{bundle_name}_report.zip", report_only=True
        )
        report["zip_report"] = str(zip_report_path)
    if zip_path or zip_report_path:
        (bundle_dir / "bundle_manifest.json").write_text(json.dumps(report, indent=2), encoding="utf8")

    return report


def build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("cue", help="cue id or .music.yaml path")
    ap.add_argument("--backend", default=DEFAULT_BACKEND, choices=BACKEND_CHOICES)
    ap.add_argument(
        "--runtime-stem-gain-mode",
        choices=RUNTIME_STEM_GAIN_MODES,
        default="native",
        help=(
            "runtime adaptive stem export mode: native preserves current raw levels; "
            "shared applies one shared reference gain across all stems"
        ),
    )
    ap.add_argument(
        "--runtime-stem-max-gain-db",
        type=float,
        default=None,
        help="cap shared runtime stem gain; default is renderer policy or YAML render.runtime_stems.max_gain_db",
    )
    ap.add_argument("--outdir", type=Path, default=None)
    ap.add_argument("--bundle-root", type=Path, default=None)
    ap.add_argument("--force", action="store_true", help="force render regeneration")
    ap.add_argument("--publish", action="store_true", help="publish full.ogg to game assets after rendering")
    ap.add_argument("--dest-root", type=Path, default=None, help="game music generated asset root")
    ap.add_argument("--zip", dest="zip_bundle", action="store_true", help="write a complete uploadable bundle zip including manifest-referenced audio")
    ap.add_argument("--zip-report", dest="zip_report_bundle", action="store_true", help="write a compact report zip excluding OGG/WAV/NPY/MIDI binaries")
    ap.add_argument(
        "--plot-format",
        choices=PLOT_FORMATS,
        default="jpg",
        help="spectrogram image format for bundles; jpg is much smaller and reports keep numeric values",
    )
    ap.add_argument("--jpeg-quality", type=int, default=84, help="JPEG quality for spectrogram plots")
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
        runtime_stem_gain_mode=args.runtime_stem_gain_mode,
        outdir=args.outdir,
        bundle_root=args.bundle_root,
        force=args.force,
        publish=args.publish,
        dest_root=args.dest_root,
        zip_bundle=args.zip_bundle,
        zip_report_bundle=args.zip_report_bundle,
        jobs=args.jobs,
        include_scratch_stems=args.include_scratch_stems,
        skip_render=args.skip_render,
        skip_spectrograms=args.skip_spectrograms,
        plot_format=args.plot_format,
        jpeg_quality=args.jpeg_quality,
        runtime_stem_max_gain_db=args.runtime_stem_max_gain_db,
    )
    print_bundle_summary(report)
    print(json.dumps(report, indent=2, default=str))
    return 0 if report.get("ok", True) else 1


if __name__ == "__main__":
    raise SystemExit(main())
