"""The music pruner deletes from gitignored directories, so a wrong rule is not recoverable.

These tests make a small renderer tree and read the plan. They do not delete.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import prune_generated_music as pruner  # noqa: E402

DAY = 86400.0
NOW = 1_000 * DAY


def render(root: Path, cue: str, version: str, *, age_days: float, latest: bool = False) -> Path:
    run = root / "generated" / cue / ".versioned" / version
    (run / "scratch_stems").mkdir(parents=True)
    (run / "scratch_stems" / "bass.npy").write_bytes(b"stem")
    (run / "preview").mkdir()
    (run / "preview" / "full.ogg").write_bytes(b"ogg")
    stamp = NOW - age_days * DAY
    os.utime(run, (stamp, stamp))
    if latest:
        (root / "generated" / cue / "latest").symlink_to(f".versioned/{version}")
    return run


def bundle(root: Path, name: str, *, age_days: float) -> Path:
    path = root / "agent" / name
    path.mkdir(parents=True)
    stamp = NOW - age_days * DAY
    os.utime(path, (stamp, stamp))
    return path


def score(root: Path, cue: str, suffix: str = ".music.yaml") -> None:
    path = root / "scores" / "active" / f"{cue}{suffix}"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("id: x\n")


def planned(root: Path, **options) -> set[tuple[str, str]]:
    options.setdefault("days", 2)
    return {
        (kind, path.relative_to(root).as_posix())
        for kind, path in pruner.plan(root, now=NOW, **options)
    }


def test_a_render_that_latest_does_not_point_at_goes_when_it_is_old(tmp_path):
    score(tmp_path, "song")
    render(tmp_path, "song", "aaaaaaaa", age_days=30)
    render(tmp_path, "song", "bbbbbbbb", age_days=1)
    render(tmp_path, "song", "cccccccc", age_days=30, latest=True)
    assert planned(tmp_path) == {("superseded", "generated/song/.versioned/aaaaaaaa")}
    assert planned(tmp_path, days=0) == {
        ("superseded", "generated/song/.versioned/aaaaaaaa"),
        ("superseded", "generated/song/.versioned/bbbbbbbb"),
    }


def test_a_cue_with_no_latest_and_a_render_in_progress_are_not_touched(tmp_path):
    score(tmp_path, "cut_short")
    render(tmp_path, "cut_short", "aaaaaaaa", age_days=30)
    score(tmp_path, "busy")
    render(tmp_path, "busy", "aaaaaaaa", age_days=30, latest=True)
    render(tmp_path, "busy", "bbbbbbbb", age_days=30)
    (tmp_path / "generated" / "busy" / "building").symlink_to(".versioned/bbbbbbbb")
    assert planned(tmp_path, days=0) == set()


def test_a_bundle_goes_but_not_the_only_copy_of_a_current_render(tmp_path):
    score(tmp_path, "song")
    render(tmp_path, "song", "aaaaaaaa", age_days=30, latest=True)
    bundle(tmp_path, "song_aaaaaaaa_bundle", age_days=30)
    bundle(tmp_path, "song_bbbbbbbb_bundle", age_days=30)
    bundle(tmp_path, "song_cccccccc_bundle", age_days=1)
    bundle(tmp_path, "hand_made_bank", age_days=30)
    score(tmp_path, "gone")
    (tmp_path / "generated" / "gone").mkdir()
    (tmp_path / "generated" / "gone" / "latest").symlink_to(".versioned/dddddddd")
    bundle(tmp_path, "gone_dddddddd_bundle", age_days=30)
    assert planned(tmp_path) == {
        ("bundle", "agent/song_aaaaaaaa_bundle"),
        ("bundle", "agent/song_bbbbbbbb_bundle"),
    }


def test_a_cue_with_no_score_loses_its_stems_and_keeps_its_audio(tmp_path):
    score(tmp_path, "kept", suffix=".yaml")
    render(tmp_path, "kept", "aaaaaaaa", age_days=30, latest=True)
    render(tmp_path, "lost_score", "aaaaaaaa", age_days=30, latest=True)
    assert planned(tmp_path) == {("stems", "generated/lost_score/.versioned/aaaaaaaa/scratch_stems")}
    assert planned(tmp_path, stems=True) == {
        ("stems", "generated/kept/.versioned/aaaaaaaa/scratch_stems"),
        ("stems", "generated/lost_score/.versioned/aaaaaaaa/scratch_stems"),
    }
    assert planned(tmp_path, orphans=True) == {("orphan", "generated/lost_score")}


def test_the_cue_filter_limits_renders_and_bundles(tmp_path):
    for cue in ("song", "other"):
        score(tmp_path, cue)
        render(tmp_path, cue, "aaaaaaaa", age_days=30)
        render(tmp_path, cue, "bbbbbbbb", age_days=30, latest=True)
        bundle(tmp_path, f"{cue}_aaaaaaaa_bundle.zip", age_days=30)
    assert planned(tmp_path, cues=["so*"]) == {
        ("superseded", "generated/song/.versioned/aaaaaaaa"),
        ("bundle", "agent/song_aaaaaaaa_bundle.zip"),
    }
