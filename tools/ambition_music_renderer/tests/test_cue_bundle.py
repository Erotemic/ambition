from __future__ import annotations

import json
import tempfile
from pathlib import Path

import numpy as np
import soundfile as sf

from ambition_music_renderer.cli import build_parser
from ambition_music_renderer.cue_bundle import make_zip, write_stem_export_report
from ambition_music_renderer.render_group_worker import build_parser as build_worker_parser
from ambition_music_renderer.render_isolated import build_parser as build_isolated_parser


def test_backend_defaults_prefer_pretty_midi():
    assert build_isolated_parser().parse_args(["cue.music.yaml"]).backend == "pretty-midi"
    assert build_worker_parser().parse_args(
        ["cue.music.yaml", "--outdir", "out", "--group", "keys"]
    ).backend == "pretty-midi"
    assert build_parser().parse_args(["render", "lofi_study_loop"]).backend == "pretty-midi"
    assert build_parser().parse_args(["cue", "bundle", "lofi_study_loop"]).backend == "pretty-midi"


def test_bundle_parser_exposes_publish_and_zip_flags():
    args = build_parser().parse_args(
        ["cue", "bundle", "for_emmy_forever_ago", "--publish", "--zip", "--jobs", "2"]
    )
    assert args.command == "cue"
    assert args.cue_action == "bundle"
    assert args.cue == "for_emmy_forever_ago"
    assert args.publish is True
    assert args.zip_bundle is True
    assert args.jobs == 2


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
