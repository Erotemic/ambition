from pathlib import Path
import importlib.util
import sys

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]


def _load_pyside_tool():
    path = ROOT / "tools" / "rig_pose_editor_pyside.py"
    spec = importlib.util.spec_from_file_location("rig_pose_editor_pyside", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def test_pyside_editor_headless_preview_does_not_require_pyside(tmp_path):
    tool = _load_pyside_tool()
    out = tmp_path / "pyside_preview.png"
    code = tool.main(
        [
            str(ROOT / "examples" / "robot_rig_job.yaml"),
            "--render-preview",
            str(out),
            "--animations",
            "run",
        ]
    )
    assert code == 0
    assert out.exists()
    img = Image.open(out).convert("RGBA")
    assert img.getbbox() is not None


def test_pyside_editor_headless_anchor_report(tmp_path):
    tool = _load_pyside_tool()
    out = tmp_path / "pyside_instances.json"
    code = tool.main(
        [
            str(ROOT / "examples" / "robot_rig_job.yaml"),
            "--anchor-report",
            str(out),
        ]
    )
    assert code == 0
    assert out.exists()
    text = out.read_text()
    assert '"role": "front_arm"' in text or '"role": "back_arm"' in text


def test_pyside_editor_source_mentions_pyside6_and_splitters():
    text = (ROOT / "tools" / "rig_pose_editor_pyside.py").read_text()
    assert "PySide6" in text
    assert "QSplitter" in text
    assert "Navigate joint -> connected part" in text
    assert "Rig joint constraints for current frame" in text
    assert "snap_error_px" in text
    assert "PyQt6" not in text


def test_qt_compat_shim_points_to_pyside():
    text = (ROOT / "tools" / "rig_pose_editor_qt.py").read_text()
    assert "rig_pose_editor_pyside" in text
    assert "PySide6" in text
