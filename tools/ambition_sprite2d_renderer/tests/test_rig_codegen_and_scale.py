"""Tests for rig-doc render_scale, the doc->Python code generator, and the
$VISUAL round-trip helper. Qt-free; runs everywhere."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

from ambition_sprite2d_renderer.authoring.rigdoc import RigDocument
from ambition_sprite2d_renderer.authoring.rigdoc_codegen import doc_to_python

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


class TestRenderScale:
    def test_default_scale_is_base_size(self, doc):
        assert doc.render_frame("idle", 0, 8).size == (128, 128)

    def test_render_scale_doubles_output(self, doc):
        doc.frame["render_scale"] = 2
        img = doc.render_frame("idle", 0, 8)
        assert img.size == (256, 256)
        # The art scales with the canvas: alpha bbox should roughly double.
        bbox = img.getchannel("A").getbbox()
        base = RigDocument.load(TEMPLATE).render_frame("idle", 0, 8)
        bb0 = base.getchannel("A").getbbox()
        assert (bbox[2] - bbox[0]) == pytest.approx(2 * (bb0[2] - bb0[0]), abs=6)

    def test_explicit_scale_arg_overrides(self, doc):
        img = doc.render_at("idle", 0.0, scale=3)
        assert img.size == (384, 384)

    def test_sheet_export_uses_scaled_frames(self, doc, tmp_path):
        from ambition_sprite2d_renderer.authoring.rigdoc import render_sheet_for_doc

        doc.frame["render_scale"] = 2
        doc.data["name"] = "scale_test_bot"
        render_sheet_for_doc(doc, tmp_path)
        import yaml

        manifest = yaml.safe_load((tmp_path / "scale_test_bot_spritesheet.yaml").read_text())
        # Auto-crop shrinks below the 256 canvas, but must exceed the
        # 128-canvas upper bound to prove frames rendered at 2x.
        assert manifest["frame_height"] > 140


class TestCodegen:
    def _load_generated(self, doc, tmp_path):
        src = doc_to_python(doc)
        mod_path = tmp_path / f"{doc.name}.py"
        mod_path.write_text(src)
        spec = importlib.util.spec_from_file_location(f"gen_{doc.name}", mod_path)
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        return mod, src

    def test_generated_module_renders_all_clips(self, doc, tmp_path):
        mod, _ = self._load_generated(doc, tmp_path)
        for anim, frames, _d in mod.ROWS:
            img = mod.render_frame(anim, 0, frames)
            assert img.size == (mod.FRAME_W * mod.RENDER_SCALE, mod.FRAME_H * mod.RENDER_SCALE)
            assert img.getchannel("A").getbbox() is not None

    def test_generated_matches_rigdoc_render(self, doc, tmp_path):
        from PIL import ImageChops

        mod, _ = self._load_generated(doc, tmp_path)
        a = doc.render_frame("walk", 3, 8)
        b = mod.render_frame("walk", 3, 8)
        diff = ImageChops.difference(a.convert("RGBA"), b.convert("RGBA"))
        assert diff.getbbox() is None or max(diff.getextrema()[3]) <= 2

    def test_generated_module_has_target_hooks(self, doc, tmp_path):
        mod, src = self._load_generated(doc, tmp_path)
        assert callable(mod.render)
        assert callable(mod.render_canonical)
        assert mod.TARGET_NAME == doc.name
        # Readability spot-checks: real toolkit API calls, not opaque blobs.
        assert "sk.bone('torso', parent='pelvis'" in src.replace('"', "'")
        assert "Clip(loop=" in src
        assert "two_bone_ik" in src

    def test_generated_sheet_bundle(self, doc, tmp_path):
        mod, _ = self._load_generated(doc, tmp_path)
        paths = mod.render(tmp_path / "out")
        names = {p.name for p in paths}
        assert f"{doc.name}_spritesheet.ron" in names


class TestVisualRoundTrip:
    def test_unset_visual_returns_none(self, monkeypatch):
        from ambition_sprite2d_renderer.gui import external

        monkeypatch.delenv("VISUAL", raising=False)
        monkeypatch.delenv("EDITOR", raising=False)
        assert external.visual_command() is None
        assert external.edit_text_in_visual("{}") is None

    def test_unchanged_text_returns_none(self, monkeypatch):
        from ambition_sprite2d_renderer.gui import external

        monkeypatch.setenv("VISUAL", "true")  # /bin/true: exits 0, no edit
        assert external.edit_text_in_visual('{"a": 1}') is None

    def test_edited_text_round_trips(self, monkeypatch):
        from ambition_sprite2d_renderer.gui import external

        monkeypatch.setenv("VISUAL", "sed -i s/AAA/BBB/")
        out = external.edit_text_in_visual('{"k": "AAA"}')
        assert out == '{"k": "BBB"}'

    def test_failing_editor_returns_none(self, monkeypatch):
        from ambition_sprite2d_renderer.gui import external

        monkeypatch.setenv("VISUAL", "false")
        assert external.edit_text_in_visual("x") is None
