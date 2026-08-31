"""The inspector must answer with the binary it tells you to build.

⛔⛔ TWO WAYS A STALE ANSWER LOOKED LIKE A CURRENT ONE, and neither is visible
from the browser. `find_renderer` took the first EXISTING candidate with
`release` ahead of `debug`, so a release binary from last week outranked a debug
binary built a minute ago — while the build hint printed beside it says to run an
ordinary debug `cargo build`. And the disk cache was accepted on frame count and
stride alone, so a render taken before an hour of engine changes was served as
this build's picture of the move. Nothing in the inspector builds anything, which
is what makes both of these long-lived rather than momentary.
"""

from __future__ import annotations

import importlib
import json
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "tools" / "ambition_moveset_inspector"))
server = importlib.import_module("ambition_moveset_inspector.server")


def _binary(path: Path, mtime: float) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("#!/bin/sh\nexit 0\n")
    path.chmod(0o755)
    import os

    os.utime(path, (mtime, mtime))
    return path


def test_a_newer_debug_binary_outranks_an_older_release_one(tmp_path):
    old = _binary(tmp_path / "release" / "moveset_render", 1_000_000)
    new = _binary(tmp_path / "debug" / "moveset_render", 2_000_000)
    assert server._newest([old, new]) == new
    # And the other order, so this is a comparison rather than a reversed
    # preference: the newest wins whichever profile it is in.
    assert server._newest([new, old]) == new
    older_debug = _binary(tmp_path / "debug" / "moveset_render", 500_000)
    assert server._newest([old, older_debug]) == old


def test_nothing_built_is_none(tmp_path):
    assert server._newest([tmp_path / "release" / "nope", tmp_path / "debug" / "nope"]) is None


def test_an_explicit_override_wins_outright(tmp_path, monkeypatch):
    chosen = _binary(tmp_path / "elsewhere" / "moveset_render", 1)
    monkeypatch.setenv("AMBITION_MOVESET_RENDER", str(chosen))
    assert server.find_renderer() == chosen
    monkeypatch.setenv("AMBITION_MOVESET_RENDER", str(tmp_path / "gone"))
    assert server.find_renderer() is None


@pytest.fixture
def sandbox(tmp_path, monkeypatch):
    """A renders directory and a target tree this test owns."""
    renders = tmp_path / "renders"
    renders.mkdir()
    monkeypatch.setattr(server, "RENDERS", renders)
    monkeypatch.setenv("CARGO_TARGET_DIR", str(tmp_path / "target"))
    monkeypatch.delenv("AMBITION_MOVESET_RENDER", raising=False)
    return tmp_path, renders


def _cache(renders: Path, *, renderer_mtime: float | None, frames: int = 24) -> Path:
    out = renders / "npc_pirate_admiral__special_up"
    out.mkdir(parents=True, exist_ok=True)
    doc = {
        "available": True,
        "frames": frames,
        "stride": 2,
        "reached_intended_move": True,
        "shots": [{"file": "frame.0000.png", "sim_tick": 31, "action_tick": 0}],
        "renderer_built": "2026-08-01 09:00",
    }
    if renderer_mtime is not None:
        doc["renderer_mtime"] = renderer_mtime
    (out / "manifest.json").write_text(json.dumps(doc))
    return out / "manifest.json"


def test_a_cache_drawn_by_the_current_binary_is_served(sandbox):
    tmp_path, renders = sandbox
    _binary(tmp_path / "target" / "debug" / "moveset_render", 1_000)
    _cache(renders, renderer_mtime=1_000)
    status, doc = server.render_animation("npc_pirate_admiral", "special_up", 24, 2)
    assert status == 200
    assert doc["frames"] == 24
    assert "stale" not in doc


def test_a_different_scenario_is_not_served_from_another_scenarios_cache(sandbox):
    """⛔⛔ THE CACHE KEY IS THE WHOLE REQUEST, AND THIS TOOL HAS BEEN BITTEN.

    Caching by character alone once served the up-B's frames for a jab. The
    scenario is the same class of mistake one layer along: a render staged from
    across the stage and one staged at 40px are different fights, and the browser
    shows the result BESIDE a recorded take. A stub renderer that writes no
    manifest makes the miss visible — a cache hit would answer 200.
    """
    tmp_path, renders = sandbox
    _binary(tmp_path / "target" / "debug" / "moveset_render", 1_000)
    _cache(renders, renderer_mtime=1_000)

    # The scenario the cache was written for is served.
    status, _ = server.render_animation("npc_pirate_admiral", "special_up", 24, 2)
    assert status == 200

    # A different spacing is a different fight, so it is a MISS.
    status, doc = server.render_animation(
        "npc_pirate_admiral", "special_up", 24, 2, None, 40.0
    )
    assert status == 503, "a spacing change must not be answered from the old cache"

    # So is a different target.
    status, _ = server.render_animation(
        "npc_pirate_admiral", "special_up", 24, 2, "projectile_polygon", None
    )
    assert status == 503, "a target change must not be answered from the old cache"


def test_a_scenario_id_that_is_not_a_plain_catalog_id_is_refused(sandbox):
    tmp_path, renders = sandbox
    _binary(tmp_path / "target" / "debug" / "moveset_render", 1_000)
    status, doc = server.render_animation(
        "npc_pirate_admiral", "special_up", 24, 2, "../../etc/passwd", None
    )
    assert status == 400 and "target" in doc["error"]


def test_a_cache_older_than_the_binary_is_not_served(sandbox):
    """⛔⛔ The regression this exists for: the picture survives the binary.

    The renderer here is a stub that writes no manifest, so a re-render FAILS —
    which is the point. A server that had served the stale cache would answer
    200 with an hour-old picture of the move; this one refuses and says why.
    """
    tmp_path, renders = sandbox
    _binary(tmp_path / "target" / "debug" / "moveset_render", 9_000)
    _cache(renders, renderer_mtime=1_000)
    status, doc = server.render_animation("npc_pirate_admiral", "special_up", 24, 2)
    assert status == 503
    assert doc["available"] is False


def test_a_cache_with_no_provenance_is_not_served_as_current(sandbox):
    """A manifest recorded before the stamp existed cannot prove its own age."""
    tmp_path, renders = sandbox
    _binary(tmp_path / "target" / "debug" / "moveset_render", 9_000)
    _cache(renders, renderer_mtime=None)
    status, _ = server.render_animation("npc_pirate_admiral", "special_up", 24, 2)
    assert status == 503


def test_with_no_renderer_the_cache_is_served_and_labelled(sandbox):
    """⭐ A cached picture beats no picture — as long as it says what it is."""
    _tmp, renders = sandbox
    _cache(renders, renderer_mtime=1_000)
    status, doc = server.render_animation("npc_pirate_admiral", "special_up", 24, 2)
    assert status == 200
    assert doc["cached_only"] is True
    assert "not built" in doc["reason"]


def test_with_no_renderer_and_no_cache_the_build_command_is_the_answer(sandbox):
    status, doc = server.render_animation("npc_pirate_admiral", "special_up", 24, 2)
    assert status == 503
    assert doc["hint"] == "cargo build -p ambition_app_tools --bin moveset_render"
    assert doc["looked_in"]
