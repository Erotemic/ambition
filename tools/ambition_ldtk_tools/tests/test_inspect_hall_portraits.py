from __future__ import annotations

from pathlib import Path
import struct

from ambition_ldtk_tools.inspect_hall_portraits import classify, portrait_paths


def _entry(path="sprites/alice_spritesheet.png"):
    return {"spritesheet": path, "portrait": None, "hall_dialogue_id": "hall"}


def _write_manifest(path: Path) -> None:
    path.write_text(
        '''(
            target: "alice",
            image: "alice_portraits.png",
            frame_width: 256,
            frame_height: 320,
            default_clip: "default",
            clips: {
                "default": (
                    duration_ms: 0,
                    looping: false,
                    frames: [(x: 0, y: 0, w: 256, h: 320)],
                ),
            },
        )''',
        encoding="utf8",
    )


def _write_png_header(path: Path, width: int = 256, height: int = 320) -> None:
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + struct.pack(">I", 13)
        + b"IHDR"
        + struct.pack(">II", width, height)
    )


def test_portrait_paths_derive_from_nested_gameplay_sheet(tmp_path):
    image, manifest = portrait_paths(
        _entry("sprites/gnu_ton_boss/giant_gnu_spritesheet.png"), tmp_path
    )
    assert image == tmp_path / "gnu_ton_boss" / "giant_gnu_portraits.png"
    assert manifest == tmp_path / "gnu_ton_boss" / "giant_gnu_portraits.ron"


def test_classify_accepts_default_portrait_product(tmp_path):
    _write_png_header(tmp_path / "alice_portraits.png")
    _write_manifest(tmp_path / "alice_portraits.ron")
    assert classify(_entry(), tmp_path) == ("ok", "")


def test_classify_reports_missing_product(tmp_path):
    status, detail = classify(_entry(), tmp_path)
    assert status == "no_png"
    assert "alice_portraits.png" in detail


def test_classify_rejects_non_png_product(tmp_path):
    (tmp_path / "alice_portraits.png").write_bytes(b"not a png")
    _write_manifest(tmp_path / "alice_portraits.ron")
    status, detail = classify(_entry(), tmp_path)
    assert status == "bad_png"
    assert "not a PNG" in detail


def test_classify_rejects_frame_outside_png(tmp_path):
    _write_png_header(tmp_path / "alice_portraits.png", width=64, height=64)
    _write_manifest(tmp_path / "alice_portraits.ron")
    status, detail = classify(_entry(), tmp_path)
    assert status == "bad_manifest"
    assert "exceeds" in detail
