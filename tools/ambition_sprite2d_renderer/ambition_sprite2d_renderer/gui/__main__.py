"""Entry point: ``python -m ambition_sprite2d_renderer.gui [file.rig.json]``."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Ambition rig editor (PySide6)")
    parser.add_argument("file", nargs="?", help="rig document (*.rig.json) to open")
    args = parser.parse_args(argv)

    from PySide6.QtWidgets import QApplication

    from ..authoring.rigdoc import RigDocument
    from .app import TEMPLATE_DIR, MainWindow
    from .state import EditorState

    app = QApplication(sys.argv[:1])
    app.setApplicationName("Ambition Rig Editor")

    if args.file:
        doc = RigDocument.load(args.file)
        state = EditorState(doc, str(Path(args.file).resolve()))
    else:
        template = TEMPLATE_DIR / "player_robot_fable.rig.json"
        doc = RigDocument.load(template) if template.exists() else RigDocument.new_empty()
        state = EditorState(doc, None)

    win = MainWindow(state)
    win.show()
    return app.exec()


if __name__ == "__main__":
    raise SystemExit(main())
