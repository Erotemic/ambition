"""The proposed LDtk retarget must produce a VALID world, not just a diff.

Five `.ldtk` worlds declare `sprite_player_robot_v3` at full resolution so the
LDtk editor can draw one `PlayerStart` entity preview. No level layer uses the
tileset, so the runtime decodes 7.6 MP at boot and never draws it.

⛔⛔ THE RETARGET IS NOT A relPath SWAP, which is the whole reason this file
exists. `tileRect`, `uiTileRect`, `tileGridSize`, `pxWid` and `pxHei` are in
TILESET PIXEL coordinates. Change only the path and the 256x256 crop that framed
one animation frame spans a third of a 832-pixel-wide image — the editor preview
breaks while the JSON still looks entirely plausible, and nothing in the game
would report it.

⭐ A DIFF THAT APPLIES IS NOT A DIFF THAT IS RIGHT. These tests read the
RESULT: valid JSON, the path moved, every pixel field rescaled, and the crop
preserved as a fraction of the image rather than as pixels.
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/propose_ldtk_tileset_retarget.py"


def load():
    spec = importlib.util.spec_from_file_location("ldtk_retarget", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def worlds_or_skip(module):
    if not module.MAP_ASSETS.is_dir():
        pytest.skip("game/ambition_map_assets is not checked out")
    if module.png_size(module.ASSETS / "sprites_0_25x" / module.SHEET) is None:
        pytest.skip("the sprite tiers are gitignored generated output; absent here")
    found = [
        w
        for w in sorted(module.MAP_ASSETS.rglob("*.ldtk"))
        if module.CURRENT_REL in w.read_text()
    ]
    if not found:
        pytest.skip("no world names the full-resolution sheet (retarget may have landed)")
    return found


def test_every_retargeted_world_is_still_valid_json():
    """⛔ THE FAILURE THAT WOULD BE WORST: a line-edited JSON file that no
    longer parses lands in Jon's submodule and breaks every world at once."""
    module = load()
    for world in worlds_or_skip(module):
        result = module.retarget_world(world, "sprites_0_25x")
        assert result is not None, f"{world} names the sheet but produced no change"
        json.loads("".join(result[1]))


def test_the_path_moves_and_every_pixel_field_moves_with_it():
    module = load()
    new_w, new_h = module.png_size(module.ASSETS / "sprites_0_25x" / module.SHEET)
    for world in worlds_or_skip(module):
        before = json.loads(world.read_text())
        after = json.loads("".join(module.retarget_world(world, "sprites_0_25x")[1]))
        old = [
            t
            for t in before["defs"]["tilesets"]
            if t.get("relPath") == module.CURRENT_REL
        ]
        for previous in old:
            current = next(
                t for t in after["defs"]["tilesets"] if t["uid"] == previous["uid"]
            )
            assert current["relPath"] == f"../sprites_0_25x/{module.SHEET}"
            assert (current["pxWid"], current["pxHei"]) == (new_w, new_h), (
                "the declared size must come from the REAL header of the new "
                "file — the old declaration was already stale (2484 vs 2468)"
            )
            assert current["tileGridSize"] < previous["tileGridSize"]


def test_the_crop_is_preserved_as_a_fraction_not_as_pixels():
    """⭐ THE POINT OF THE WHOLE EXERCISE. A rect kept at 256 px would frame a
    third of the new image instead of one animation frame."""
    module = load()
    for world in worlds_or_skip(module):
        before = json.loads(world.read_text())
        after = json.loads("".join(module.retarget_world(world, "sprites_0_25x")[1]))
        for previous in before["defs"]["tilesets"]:
            if previous.get("relPath") != module.CURRENT_REL:
                continue
            current = next(
                t for t in after["defs"]["tilesets"] if t["uid"] == previous["uid"]
            )
            for entity_before in before["defs"]["entities"]:
                rect_before = entity_before.get("tileRect")
                if not rect_before or rect_before.get("tilesetUid") != previous["uid"]:
                    continue
                entity_after = next(
                    e
                    for e in after["defs"]["entities"]
                    if e["identifier"] == entity_before["identifier"]
                )
                rect_after = entity_after["tileRect"]
                was = rect_before["w"] / previous["pxWid"]
                now = rect_after["w"] / current["pxWid"]
                assert abs(was - now) < 0.02, (
                    f"{world.name}: the crop was {was:.3f} of the width and is "
                    f"now {now:.3f} — the preview framing changed"
                )
                assert rect_after["w"] >= 1 and rect_after["h"] >= 1


def test_an_origin_of_zero_stays_zero():
    """⛔ MY OWN BUG, PINNED. The extent clamp that stops a rect rounding away
    to zero width was applied to x and y too, turning every `"x": 0, "y": 0`
    into `"x": 1, "y": 1` and nudging the crop of all five worlds."""
    module = load()
    assert module.rescale(0, 3072, 832, floor_at_one=False) == 0, (
        "an origin at the top-left corner must stay at the corner"
    )
    assert module.rescale(1, 3072, 832, floor_at_one=True) == 1, (
        "an extent that rounds to 0 must be clamped to 1, or the preview vanishes"
    )
    assert module.rescale(1, 3072, 832, floor_at_one=False) == 0, (
        "premise: without the clamp that same value really does round to 0, so "
        "the clamp is doing something"
    )


def test_it_refuses_to_emit_an_empty_patch(tmp_path, monkeypatch, capsys):
    """⛔ Once the retarget lands, a re-run finds nothing. Printing an empty
    diff and exiting 0 reads as 'no changes needed' — indistinguishable from
    success on a tree where the worlds moved or the submodule is absent."""
    module = load()
    monkeypatch.setattr(module, "MAP_ASSETS", tmp_path)
    (tmp_path / "empty.ldtk").write_text('{"defs": {"tilesets": []}}')
    monkeypatch.setattr(module, "ASSETS", module.ASSETS)
    code = module.main([])
    out = capsys.readouterr().out
    if code == 2 and "Absent is not zero" in out:
        pytest.skip("the sprite tiers are absent here, which this refuses first")
    assert code == 2
    assert "NO WORLD NAMES THE FULL-RESOLUTION SHEET" in out


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
