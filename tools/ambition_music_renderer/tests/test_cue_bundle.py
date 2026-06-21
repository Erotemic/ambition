from __future__ import annotations

import json
import tempfile
from pathlib import Path

import numpy as np
import soundfile as sf

from ambition_music_renderer.cli import build_parser
from ambition_music_renderer.cue_bundle import (
    copy_manifest_referenced_files,
    make_zip,
    manifest_audio_entries,
    should_include_in_report_zip,
    summarize_mix_diagnostics,
    prepare_manifest_analysis_root,
    write_manifest_audio_level_report,
    write_spectral_fingerprint,
    write_state_mix_report,
    write_stem_export_report,
)
from ambition_music_renderer.dissonance_audit import audit_spec, write_reports as write_dissonance_reports
from ambition_music_renderer.render_group_worker import build_parser as build_worker_parser
from ambition_music_renderer.render_isolated import build_parser as build_isolated_parser


def test_backend_defaults_prefer_pretty_midi():
    assert build_isolated_parser().parse_args(["cue.music.yaml"]).backend == "pretty-midi"
    assert build_worker_parser().parse_args(
        ["cue.music.yaml", "--outdir", "out", "--group", "keys"]
    ).backend == "pretty-midi"
    assert build_parser().parse_args(["render", "lofi_study_loop"]).backend == "pretty-midi"
    assert build_parser().parse_args(["cue", "bundle", "lofi_study_loop"]).backend == "pretty-midi"
    shared_args = build_isolated_parser().parse_args([
        "cue.music.yaml",
        "--runtime-stem-gain-mode",
        "shared",
        "--runtime-stem-max-gain-db",
        "18",
    ])
    assert shared_args.runtime_stem_gain_mode == "shared"
    assert shared_args.runtime_stem_max_gain_db == 18.0


def test_bundle_parser_exposes_publish_and_zip_flags():
    args = build_parser().parse_args(
        [
            "cue",
            "bundle",
            "for_emmy_forever_ago",
            "--publish",
            "--zip",
            "--jobs",
            "2",
            "--runtime-stem-gain-mode",
            "shared",
            "--runtime-stem-max-gain-db",
            "18",
            "--zip-report",
            "--plot-format",
            "jpg",
        ]
    )
    assert args.command == "cue"
    assert args.cue_action == "bundle"
    assert args.cue == "for_emmy_forever_ago"
    assert args.publish is True
    assert args.zip_bundle is True
    assert args.jobs == 2
    assert args.runtime_stem_gain_mode == "shared"
    assert args.runtime_stem_max_gain_db == 18.0
    assert args.zip_report_bundle is True
    assert args.plot_format == "jpg"


def test_stem_export_report_compares_scratch_adaptive_and_preview_audio():
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        sr = 48_000
        t = np.arange(sr // 10, dtype="float32") / sr
        tone = 0.1 * np.sin(2 * np.pi * 440.0 * t)
        stereo = np.stack([tone, tone], axis=1).astype("float32")

        scratch = root / "scratch_stems"
        scratch.mkdir()
        np.save(scratch / "testcue_deadbeef.keys.npy", stereo)

        adaptive = root / "adaptive" / "loop"
        adaptive.mkdir(parents=True)
        sf.write(adaptive / "testcue_deadbeef.loop.keys.wav", stereo, sr)
        sf.write(adaptive / "testcue_deadbeef.loop.full.wav", stereo, sr)

        preview = root / "preview"
        preview.mkdir()
        sf.write(preview / "testcue_deadbeef.full_soundtrack_preview.wav", stereo, sr)

        manifest = {
            "id": "testcue",
            "sample_rate": sr,
            "files": {
                "adaptive": {
                    "loop": {
                        "keys": "adaptive/loop/testcue_deadbeef.loop.keys.wav",
                        "full": "adaptive/loop/testcue_deadbeef.loop.full.wav",
                    }
                },
                "preview": {
                    "full_soundtrack": "preview/testcue_deadbeef.full_soundtrack_preview.wav"
                },
            },
        }

        report_path = write_stem_export_report(root, manifest, root / "reports")
        text = report_path.read_text()
        assert "scratch_npy" in text
        assert "adaptive_audio" in text
        assert "preview_audio" in text
        assert "keys" in text
        data = json.loads((root / "reports" / "stem_export_report.json").read_text())
        assert data["cue_id"] == "testcue"
        assert len(data["rows"]) == 4


def test_make_zip_contains_bundle_files():
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        bundle = root / "mycue_hash_bundle"
        (bundle / "reports").mkdir(parents=True)
        (bundle / "reports" / "report.txt").write_text("ok", encoding="utf8")
        zip_path = make_zip(bundle, root / "mycue_hash_bundle.zip")
        assert zip_path.exists()
        import zipfile

        with zipfile.ZipFile(zip_path) as zf:
            names = set(zf.namelist())
        assert "mycue_hash_bundle/reports/report.txt" in names


def test_report_zip_excludes_large_binary_artifacts():
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        bundle = root / "mycue_hash_bundle"
        (bundle / "reports").mkdir(parents=True)
        (bundle / "adaptive" / "loop").mkdir(parents=True)
        (bundle / "plots").mkdir(parents=True)
        (bundle / "reports" / "report.txt").write_text("ok", encoding="utf8")
        (bundle / "source.music.yaml").write_text("id: mycue", encoding="utf8")
        (bundle / "plots" / "stem.spectrogram.jpg").write_bytes(b"jpeg")
        (bundle / "adaptive" / "loop" / "mycue.loop.full.ogg").write_bytes(b"ogg")
        (bundle / "scratch_stems").mkdir()
        (bundle / "scratch_stems" / "mycue.keys.npy").write_bytes(b"npy")

        assert should_include_in_report_zip(bundle / "reports" / "report.txt")
        assert should_include_in_report_zip(bundle / "plots" / "stem.spectrogram.jpg")
        assert not should_include_in_report_zip(bundle / "adaptive" / "loop" / "mycue.loop.full.ogg")
        assert not should_include_in_report_zip(bundle / "scratch_stems" / "mycue.keys.npy")

        zip_path = make_zip(bundle, root / "mycue_hash_bundle_report.zip", report_only=True)
        import zipfile

        with zipfile.ZipFile(zip_path) as zf:
            names = set(zf.namelist())
        assert "mycue_hash_bundle/reports/report.txt" in names
        assert "mycue_hash_bundle/plots/stem.spectrogram.jpg" in names
        assert "mycue_hash_bundle/adaptive/loop/mycue.loop.full.ogg" not in names
        assert "mycue_hash_bundle/scratch_stems/mycue.keys.npy" not in names


def test_manifest_audio_entries_and_bundle_copy_are_manifest_scoped():
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        current = root / "preview" / "cue_hash.full_soundtrack_preview.ogg"
        stale = root / "preview" / "cue_old.full_soundtrack_preview.ogg"
        adaptive = root / "adaptive" / "loop" / "cue_hash.loop.full.ogg"
        current.parent.mkdir(parents=True)
        adaptive.parent.mkdir(parents=True)
        current.write_bytes(b"current")
        stale.write_bytes(b"stale")
        adaptive.write_bytes(b"adaptive")
        manifest = {
            "files": {
                "preview": {"full_soundtrack": "preview/cue_hash.full_soundtrack_preview.ogg"},
                "adaptive": {"loop": {"full": "adaptive/loop/cue_hash.loop.full.ogg"}},
            }
        }
        entries = manifest_audio_entries(manifest)
        assert {e["path"] for e in entries} == {
            "preview/cue_hash.full_soundtrack_preview.ogg",
            "adaptive/loop/cue_hash.loop.full.ogg",
        }
        bundle = root / "bundle"
        copied = copy_manifest_referenced_files(root, manifest, bundle)
        assert sorted(copied) == [
            "adaptive/loop/cue_hash.loop.full.ogg",
            "preview/cue_hash.full_soundtrack_preview.ogg",
        ]
        assert (bundle / "preview" / current.name).exists()
        assert not (bundle / "preview" / stale.name).exists()


def test_manifest_audio_level_report_ignores_stale_audio():
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        sr = 48_000
        t = np.arange(sr // 20, dtype="float32") / sr
        tone = 0.05 * np.sin(2 * np.pi * 220.0 * t)
        stereo = np.stack([tone, tone], axis=1).astype("float32")
        preview = root / "preview"
        preview.mkdir()
        sf.write(preview / "cue_hash.full_soundtrack_preview.wav", stereo, sr)
        sf.write(preview / "cue_old.full_soundtrack_preview.wav", stereo, sr)
        manifest = {
            "files": {
                "preview": {"full_soundtrack": "preview/cue_hash.full_soundtrack_preview.wav"},
                "adaptive": {},
            }
        }
        report = write_manifest_audio_level_report(root, manifest, root / "reports")
        text = report.read_text()
        assert "cue_hash.full_soundtrack_preview.wav" in text
        assert "cue_old.full_soundtrack_preview.wav" not in text


def test_mix_diagnostics_surfaces_renderer_warnings():
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        manifest = {
            "id": "cue",
            "hash": "abc123",
            "runtime_stem_gain_mode": "native",
            "diagnostics": {
                "raw_full": {"rms_dbfs": -75.0, "peak_dbfs": -55.0},
                "mastered_full": {"rms_dbfs": -24.0, "peak_dbfs": -8.0},
                "master_rms_lift_db": 51.0,
                "runtime_gain_db": 0.0,
                "runtime_gain_reason": "native",
                "native_stems": {"keys": {"rms_dbfs": -75.0, "peak_dbfs": -55.0}},
                "runtime_stems": {"keys": {"rms_dbfs": -75.0, "peak_dbfs": -55.0}},
                "warnings": ["native runtime stems are very quiet"],
            },
        }
        report, warnings = summarize_mix_diagnostics(manifest, root / "reports")
        text = report.read_text()
        assert "master_rms_lift_db" in text
        assert "native runtime stems are very quiet" in text
        assert warnings == ["native runtime stems are very quiet"]



def test_analysis_root_copies_only_current_hash_scratch_stems():
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        sr = 48_000
        audio = np.zeros((128, 2), dtype="float32")
        scratch = root / "scratch_stems"
        scratch.mkdir()
        np.save(scratch / "cue_current.keys.npy", audio)
        np.save(scratch / "cue_old.keys.npy", audio)
        preview = root / "preview"
        preview.mkdir()
        sf.write(preview / "cue_current.full_soundtrack_preview.wav", audio, sr)
        sf.write(preview / "cue_old.full_soundtrack_preview.wav", audio, sr)
        manifest = {
            "id": "cue",
            "hash": "current",
            "files": {
                "preview": {"full_soundtrack": "preview/cue_current.full_soundtrack_preview.wav"},
                "adaptive": {},
            },
        }
        analysis = prepare_manifest_analysis_root(root, manifest, root / "analysis")
        assert (analysis / "scratch_stems" / "cue_current.keys.npy").exists()
        assert not (analysis / "scratch_stems" / "cue_old.keys.npy").exists()
        assert (analysis / "preview" / "cue_current.full_soundtrack_preview.wav").exists()
        assert not (analysis / "preview" / "cue_old.full_soundtrack_preview.wav").exists()



def test_spectral_fingerprint_is_llm_friendly_json_and_tsv():
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        sr = 48_000
        duration = 0.25
        t = np.arange(int(sr * duration), dtype="float32") / sr
        low = 0.1 * np.sin(2 * np.pi * 120.0 * t)
        high = 0.1 * np.sin(2 * np.pi * 4200.0 * t)
        scratch = root / "scratch_stems"
        scratch.mkdir()
        np.save(scratch / "cue_hash.low_keys.npy", np.stack([low, low], axis=1).astype("float32"))
        np.save(scratch / "cue_hash.pluck.npy", np.stack([high, high], axis=1).astype("float32"))
        manifest = {
            "id": "cue",
            "hash": "hash",
            "sample_rate": sr,
            "sections": [{"end_seconds": duration}],
        }
        report = write_spectral_fingerprint(root, manifest, root / "reports", bucket_seconds=0.25)
        payload = json.loads(report.read_text())
        assert payload["schema"] == "ambition.music_spectral_fingerprint.v1"
        assert payload["mean_band_fraction_by_group"]["low"]["low_keys"] > 0.9
        assert payload["mean_band_fraction_by_group"]["vhigh"]["pluck"] > 0.9
        assert (root / "reports" / "spectral_fingerprint.tsv").exists()
        assert (root / "reports" / "spectral_fingerprint_summary.txt").exists()


def test_state_mix_report_flags_similar_states():
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        spec = {
            "id": "cue",
            "instruments": [
                {"name": "piano", "group": "keys"},
                {"name": "bass", "group": "bass"},
            ],
            "state_map": {
                "default": {"section": "loop", "stems": {"keys": 0.8, "bass": 0.6}},
                "quiet": {"section": "loop", "stems": {"keys": 0.7, "bass": 0.5}},
            },
        }
        manifest = {"diagnostics": {"runtime_previews": {}}}
        report = write_state_mix_report(spec, manifest, root / "reports")
        payload = json.loads(report.read_text())
        assert payload["schema"] == "ambition.music_state_mix_report.v1"
        text = (root / "reports" / "state_mix_report_summary.txt").read_text()
        assert "state distances from default" in text
        assert "warning: state maps are close together" in text


def test_dissonance_audit_identifies_close_layer_clash():
    spec = {
        "schema": "ambition.musicir.v1",
        "id": "clash_test",
        "tempo": {"bpm": 120},
        "meter": {"beats_per_bar": 4, "beat_unit": 4},
        "instruments": [
            {"name": "a", "group": "keys", "program": "acoustic_grand_piano"},
            {"name": "b", "group": "lead", "program": "acoustic_grand_piano"},
        ],
        "layer_templates": {
            "a_note": {
                "kind": "motif",
                "instrument": "a",
                "motif": "a_motif",
                "root": "C4",
                "starts": [[0, 0.0]],
                "repeats": 1,
                "velocity": 90,
            },
            "b_note": {
                "kind": "motif",
                "instrument": "b",
                "motif": "b_motif",
                "root": "C#4",
                "starts": [[0, 0.0]],
                "repeats": 1,
                "velocity": 90,
            },
        },
        "motifs": [
            {"id": "a_motif", "root": "C4", "intervals": [0], "rhythm": [1.0], "velocities": [1.0]},
            {"id": "b_motif", "root": "C#4", "intervals": [0], "rhythm": [1.0], "velocities": [1.0]},
        ],
        "sections": [
            {"id": "loop", "bars": 1, "harmony": ["C"], "layers": ["a_note", "b_note"]}
        ],
    }
    payload = audit_spec(spec)
    assert payload["hotspots"]
    top = payload["hotspots"][0]
    assert top["worst_pairs"][0]["interval_class"] == 1
    assert top["worst_pairs"][0]["layers"] == ["a_note", "b_note"]
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        paths = write_dissonance_reports(payload, root / "reports", plots_dir=root / "plots", plot_format="jpg")
        assert Path(paths["summary"]).exists()
        assert Path(paths["markdown"]).exists()
        assert "minor second" in Path(paths["summary"]).read_text()
        assert "Top Hotspots" in Path(paths["markdown"]).read_text()
        if "timeline_plot" in paths:
            assert Path(paths["timeline_plot"]).exists()
        if "layer_pair_plot" in paths:
            assert Path(paths["layer_pair_plot"]).exists()
