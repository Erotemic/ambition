"""Tests for rig documents (rigdoc): channels, rendering, IK, export,
and auto-registration of targets/characters/rigged/ documents."""

from __future__ import annotations

from pathlib import Path

import pytest

from ambition_sprite2d_renderer.authoring.rigdoc import (
    RigDocument,
    parse_color,
    render_sheet_for_doc,
    sample_channel_spec,
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
