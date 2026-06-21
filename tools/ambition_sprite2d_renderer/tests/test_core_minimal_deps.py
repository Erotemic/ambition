"""Guard: the rendering core stays renderable with only Pillow + stdlib.

The portability goal (docs/planning/sprite-renderer-refactor.md) is that a
chatbot agent with just ``pip install Pillow`` can author and render a sprite.
That only holds if ``ambition_sprite2d_renderer.core`` never grows a heavy
import. This test enforces it two ways:

  1. Static: scan every ``core/*.py`` source for a forbidden top-level import.
  2. Dynamic: import the package and exercise the seam types.

If you reach for ``yaml`` / ``rich`` / ``PySide6`` / ``numpy`` inside ``core``,
move it to an edge module instead.
"""
from __future__ import annotations

import ast
import importlib
from pathlib import Path

import pytest

FORBIDDEN = {"yaml", "rich", "PySide6", "numpy", "scipy", "cv2"}
CORE_DIR = Path(__file__).resolve().parents[1] / "ambition_sprite2d_renderer" / "core"


def _imported_top_modules(src: str) -> set[str]:
    tree = ast.parse(src)
    mods: set[str] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                mods.add(alias.name.split(".")[0])
        elif isinstance(node, ast.ImportFrom) and node.level == 0 and node.module:
            mods.add(node.module.split(".")[0])
    return mods


@pytest.mark.parametrize("src_file", sorted(CORE_DIR.glob("*.py")), ids=lambda p: p.name)
def test_core_has_no_heavy_imports(src_file: Path) -> None:
    used = _imported_top_modules(src_file.read_text())
    offenders = used & FORBIDDEN
    assert not offenders, f"{src_file.name} imports forbidden deps: {sorted(offenders)}"


def test_core_imports_and_seam_usable() -> None:
    core = importlib.import_module("ambition_sprite2d_renderer.core")
    fs = core.frameset_from_states(
        "demo", (128, 128), {"idle": lambda d, s: None, "hit": lambda d, s: None}
    )
    assert fs.animation_names == ["idle", "hit"]
    assert fs.animation("idle").frame_count == 1
    assert fs.crop == "tight"
