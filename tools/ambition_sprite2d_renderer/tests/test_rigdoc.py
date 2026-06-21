"""Tests for rig documents (rigdoc): channels, rendering, IK, export,
and auto-registration of targets/characters/rigged/ documents."""

from __future__ import annotations

from pathlib import Path

import pytest

from ambition_sprite2d_renderer.authoring.rigdoc import (
    RigDocument,
    parse_color,
    part_visible,
    render_sheet_for_doc,
    sample_channel_spec,
    visible_parts,
)

TEMPLATE = (
    Path(__file__).resolve().parent.parent
    / "ambition_sprite2d_renderer"
    / "data"
    / "rig_templates"
    / "player_robot_fable.rig.json"
)


@pytest.fixture()
def doc() -> RigDocument:
    return RigDocument.load(TEMPLATE)


NOETHER = (
    Path(__file__).resolve().parent.parent
    / "ambition_sprite2d_renderer"
    / "targets"
    / "characters"
    / "rigged"
    / "noether.rig.json"
)


class TestFeatureToggles:
    """Optional-part customization: a part tagged with a `feature` only
    renders when the doc's `features` map allows it (default on)."""

    def test_part_visible_defaults_and_toggles(self):
        plain = {"name": "torso"}
        pin = {"name": "pin", "feature": "hairpin"}
        assert part_visible(plain, {}) is True              # untagged: always on
        assert part_visible(pin, {}) is True                # unlisted feature: on
        assert part_visible(pin, {"hairpin": True}) is True
        assert part_visible(pin, {"hairpin": False}) is False

    def test_visible_parts_drops_disabled_and_keeps_z_order(self):
        parts = [
            {"name": "b", "z": 2},
            {"name": "pin", "z": 5, "feature": "hairpin"},
            {"name": "a", "z": 1},
        ]
        out = [p["name"] for p in visible_parts(parts, {"hairpin": False})]
        assert out == ["a", "b"]  # pin dropped; remaining sorted by z

    def test_sprite_tuning_flows_to_ron(self, tmp_path):
        """A rig's sprite_tuning is emitted to the RON's tuning field so the
        runtime uses it for in-game size instead of the DEFAULT fallback."""
        doc = RigDocument.new_empty("tuned")
        doc.data["sprite_tuning"] = {"collision_scale": 2.0, "frame_sample_inset": 1}
        render_sheet_for_doc(doc, tmp_path)
        ron = (tmp_path / "tuned_spritesheet.ron").read_text()
        assert "tuning: Some((" in ron
        assert "collision_scale: 2.0" in ron

    def test_render_scale_increases_resolution_not_in_game_size(self, tmp_path):
        """render_scale multiplies the texture's pixels (crisper, no upscaling)
        while the aspect ratio — all the in-game size derives from — is held."""
        doc = RigDocument.new_empty("res")

        import yaml as _yaml

        def frame_dims(scale):
            doc.frame["render_scale"] = scale
            sub = tmp_path / f"s{scale}"
            sub.mkdir()
            render_sheet_for_doc(doc, sub)
            data = _yaml.safe_load((sub / "res_spritesheet.yaml").read_text())
            return data["frame_width"], data["frame_height"]

        w1, h1 = frame_dims(1)
        w2, h2 = frame_dims(2)
        assert w2 > w1 and h2 > h1, "2x render_scale yields more native pixels"
        # Aspect (what in-game size uses) is preserved within rounding.
        assert abs((w2 / h2) - (w1 / h1)) < 0.06

    def test_noether_is_tall_and_hi_res(self):
        """Emmy ships taller (collision_scale > the 1.5 fallback) and at 2x
        render resolution so she isn't pixelated in game."""
        emmy = RigDocument.load(NOETHER)
        assert emmy.sprite_tuning.get("collision_scale", 1.5) > 1.5
        assert emmy.frame.get("render_scale", 1) >= 2

    def test_noether_hairpin_is_present_and_rigid(self):
        """Emmy's hairpin reads as a hairpin (on) and is RIGID: bound to the
        `head` bone, never the bobbing `antenna` channel that made it wave."""
        emmy = RigDocument.load(NOETHER)
        assert emmy.features.get("hairpin") is True
        pin_parts = [p for p in emmy.parts if p["name"].startswith("pin_")]
        assert pin_parts, "hairpin parts should exist"
        for p in pin_parts:
            assert p["bone"] == "head", f"{p['name']} must be rigid (head-bound)"
            assert p.get("feature") == "hairpin"
        visible = {p["name"] for p in visible_parts(emmy.parts, emmy.features)}
        assert {"pin_shaft", "pin_bead"} <= visible
        # Toggling the feature off still removes it (customization seam intact).
        assert not ({"pin_shaft", "pin_bead"} & {
            p["name"] for p in visible_parts(emmy.parts, {"hairpin": False})
        })


class TestChannelSpecs:
    def test_const_expr_keys(self):
        assert sample_channel_spec({"const": 3.5}, 0.7, True) == 3.5
        assert sample_channel_spec({"expr": "2*t"}, 0.25, False) == pytest.approx(0.5)
        spec = {"keys": [[0.0, 0.0, "linear"], [1.0, 10.0, "linear"]]}
        assert sample_channel_spec(spec, 0.5, False) == pytest.approx(5.0)

    def test_expr_rejects_builtins(self):
        with pytest.raises(Exception):
            sample_channel_spec({"expr": "__import__('os')"}, 0.0, True)

    def test_parse_color(self):
        pal = {"shell": "#FDFDFB"}
        assert parse_color("shell", pal) == (253, 253, 251, 255)
        assert parse_color("#FF000080", pal) == (255, 0, 0, 128)
        assert parse_color("#00FF00", pal, opacity=0.5) == (0, 255, 0, 127)
        assert parse_color(None, pal) is None


class TestTemplateDocument:
    def test_loads_and_lists_rows(self, doc):
        assert doc.name == "player_robot_fable_rig"
        assert [r[0] for r in doc.rows()] == ["idle", "walk", "slash"]

    def test_render_frames_all_clips(self, doc):
        for anim, frames, _ in doc.rows():
            img = doc.render_frame(anim, 0, frames)
            assert img.size == (128, 128)
            assert img.getchannel("A").getbbox() is not None

    def test_ik_feet_stay_on_ground_in_walk(self, doc):
        gy = doc.frame["ground_y"]
        ankle_h = doc.frame["ankle_h"]
        for side, stance in (("near_foot", (0.1, 0.35)), ("far_foot", (0.6, 0.85))):
            for t in (stance[0], (stance[0] + stance[1]) / 2, stance[1]):
                world, _ = doc.solve("walk", t)
                ankle = world[side].origin
                assert ankle[1] == pytest.approx(gy - ankle_h, abs=0.05), (side, t)

    def test_blade_hidden_outside_slash(self, doc):
        # opacity_channel parts default to invisible when their channel is
        # absent: idle must not paint the blade.
        _, params_idle = doc.solve("idle", 0.25)
        assert "slash_vis" not in params_idle
        _, params_slash = doc.solve("slash", 0.45)
        assert params_slash["slash_vis"] > 0.5

    def test_sheet_export_bundle(self, doc, tmp_path):
        paths = render_sheet_for_doc(doc, tmp_path)
        names = {p.name for p in paths}
        assert f"{doc.name}_spritesheet.png" in names
        assert f"{doc.name}_spritesheet.ron" in names
        ron = (tmp_path / f"{doc.name}_spritesheet.ron").read_text()
        assert 'animation: "idle"' in ron

    def test_save_load_round_trip(self, doc, tmp_path):
        out = tmp_path / "x.rig.json"
        doc.save(out)
        again = RigDocument.load(out)
        assert again.data == doc.data


class TestRiggedRegistration:
    def test_rigged_module_imports(self):
        from ambition_sprite2d_renderer.targets.characters import rigged

        assert isinstance(rigged.TARGETS, dict)

    def test_doc_in_rigged_dir_registers(self, tmp_path, monkeypatch):
        from ambition_sprite2d_renderer.targets.characters import rigged

        doc = RigDocument.load(TEMPLATE)
        doc.data["name"] = "test_rigged_bot"
        doc.save(tmp_path / "test_rigged_bot.rig.json")
        monkeypatch.setattr(rigged, "RIGGED_DIR", tmp_path)
        targets = rigged._discover()
        assert "test_rigged_bot" in targets
        assert callable(targets["test_rigged_bot"]["render"])
