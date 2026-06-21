"""LDtk project load/write helpers."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def load_project(path: Path) -> dict[str, Any]:
    """Load an LDtk JSON project as a mutable dictionary."""
    return json.loads(path.read_text())


def write_project(path: Path, project: dict[str, Any]) -> None:
    """Write an LDtk project using the editor-friendly Ambition formatting.

    The formatter also refreshes derived editor fields when the full validator is
    available.  Feature commands should prefer this over direct ``json.dump`` so
    no command has to decide when to run the editor normalizer itself.
    """
    try:
        from ambition_ldtk_tools.editor_format import dump_editor_style
        from ambition_ldtk_tools.validate import normalize_project_for_editor

        normalize_project_for_editor(project)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(dump_editor_style(project))
    except Exception:  # pragma: no cover - keeps tiny test/tool installs usable
        write_project_json(path, project)


def write_project_json(path: Path, project: dict[str, Any]) -> None:
    """Write a project with plain JSON formatting.

    This is mainly a fallback / test helper.  Tool commands should use
    :func:`write_project` unless they explicitly want raw JSON.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(project, indent=2, sort_keys=False) + "\n")
