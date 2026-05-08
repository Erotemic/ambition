from pathlib import Path
import importlib.util
import sys

from PIL import Image
import yaml


def _load_anchor_editor(root: Path):
    path = root / "tools" / "anchor_editor.py"
    spec = importlib.util.spec_from_file_location("anchor_editor", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def test_anchor_editor_preview_render_headless(tmp_path):
    root = Path(__file__).resolve().parents[1]
    editor = _load_anchor_editor(root)
    meta = editor.load_yaml(root / "metadata" / "robot_components.refined.yaml")
    config = root / "examples" / "robot_rig_job.yaml"
    img = editor.render_preview_image(meta, config, max_width=320, max_height=160, bg="black")
    assert isinstance(img, Image.Image)
    assert img.width <= 320
    assert img.height <= 160
    assert img.getbbox() is not None


def test_anchor_report_includes_pivot_anchor(tmp_path):
    root = Path(__file__).resolve().parents[1]
    editor = _load_anchor_editor(root)
    meta_path = tmp_path / "meta.yaml"
    meta = editor.load_yaml(root / "metadata" / "robot_components.refined.yaml")
    meta["sprites"]["hand_fist"]["pivot_anchor"] = "wrist"
    meta["sprites"]["hand_fist"]["pivot"] = meta["sprites"]["hand_fist"]["anchors"]["wrist"]
    meta_path.write_text(yaml.safe_dump(meta, sort_keys=False), encoding="utf8")
    out = tmp_path / "report.json"
    editor.write_anchor_report(meta_path, root / "output" / "slices", out, ["hand_fist"])
    text = out.read_text()
    assert '"pivot_anchor": "wrist"' in text


def test_anchor_editor_preview_uses_unsaved_metadata_state(tmp_path):
    """Live preview must render from in-memory metadata, not only saved YAML."""
    import copy
    from PIL import ImageChops

    root = Path(__file__).resolve().parents[1]
    editor = _load_anchor_editor(root)
    meta = editor.load_yaml(root / "metadata" / "robot_components.refined.yaml")
    config = root / "examples" / "robot_rig_job.yaml"
    # Check the raw compositor output to verify the temporary metadata written
    # by render_preview_image comes from the in-memory dictionary.
    editor.preview_fit_image = lambda img, *args, **kwargs: img.convert("RGBA")
    original = editor.render_preview_image(meta, config, max_width=640, max_height=260, bg="black")
    edited = copy.deepcopy(meta)
    # Move a run-critical shoulder far enough that the rendered sheet must change.
    edited["sprites"]["torso_lean_forward"]["anchors"]["shoulder_right"] = [300, 0]
    changed = editor.render_preview_image(edited, config, max_width=640, max_height=260, bg="black")
    assert original.size == changed.size
    assert ImageChops.difference(original, changed).getbbox() is not None


def test_anchor_editor_preview_defaults_are_large():
    root = Path(__file__).resolve().parents[1]
    editor = _load_anchor_editor(root)
    cfg = editor.PreviewConfig()
    assert cfg.max_width >= 1200
    assert cfg.max_height >= 700
    assert cfg.live_unsaved is True
