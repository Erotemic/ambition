#!/usr/bin/env python3
"""Compatibility shim for the PySide6 rig pose editor.

The primary editor moved to ``tools/rig_pose_editor_pyside.py`` when the GUI
backend switched to PySide6.  This module remains so older scripts
that invoke ``tools/rig_pose_editor_qt.py`` continue to work.
"""
from __future__ import annotations

try:  # pragma: no cover - package/module invocation path
    from tools.rig_pose_editor_pyside import *  # noqa: F401,F403
    from tools.rig_pose_editor_pyside import main
except Exception:  # pragma: no cover - direct script fallback
    import importlib.util
    import sys
    from pathlib import Path

    _path = Path(__file__).with_name("rig_pose_editor_pyside.py")
    _spec = importlib.util.spec_from_file_location("rig_pose_editor_pyside", _path)
    if _spec is None or _spec.loader is None:
        raise
    _module = importlib.util.module_from_spec(_spec)
    sys.modules[_spec.name] = _module
    _spec.loader.exec_module(_module)
    globals().update({k: v for k, v in vars(_module).items() if not k.startswith("__")})
    main = _module.main


if __name__ == "__main__":
    raise SystemExit(main())
