"""Main window for the rig editor.

Launch with::

    python -m ambition_sprite2d_renderer.gui [path/to/file.rig.json]

File → New starts from the bundled player_robot_fable template (or an
empty biped stub); Save As suggests ``targets/characters/rigged/`` so the
character auto-registers as a sheet target; Export renders the standard
spritesheet bundle + per-clip GIFs without leaving the editor.
"""

from __future__ import annotations

from pathlib import Path

from PySide6.QtCore import Qt
from PySide6.QtGui import QAction, QKeySequence
from PySide6.QtWidgets import (
    QApplication,
    QDockWidget,
    QFileDialog,
    QInputDialog,
    QMainWindow,
    QMessageBox,
    QTabWidget,
)

from ..rigdoc import RigDocument, render_gifs_for_doc, render_sheet_for_doc
from .canvas import CanvasWidget
from .panels import BonesPanel, PalettePanel, PartsPanel
from .state import EditorState
from .timeline import TimelinePanel

TEMPLATE_DIR = Path(__file__).resolve().parent.parent / "data" / "rig_templates"
RIGGED_DIR = Path(__file__).resolve().parent.parent / "targets" / "characters" / "rigged"


class MainWindow(QMainWindow):
    def __init__(self, state: EditorState) -> None:
        super().__init__()
        self.state = state
        self.canvas = CanvasWidget(state)
        self.setCentralWidget(self.canvas)
        self.canvas.statusMessage.connect(lambda m: self.statusBar().showMessage(m, 4000))

        left = QDockWidget("Bones", self)
        left.setWidget(BonesPanel(state))
        self.addDockWidget(Qt.DockWidgetArea.LeftDockWidgetArea, left)

        right = QDockWidget("Parts / Palette", self)
        tabs = QTabWidget()
        tabs.addTab(PartsPanel(state), "Parts")
        tabs.addTab(PalettePanel(state), "Palette")
        right.setWidget(tabs)
        self.addDockWidget(Qt.DockWidgetArea.RightDockWidgetArea, right)

        bottom = QDockWidget("Timeline", self)
        self.timeline = TimelinePanel(state)
        bottom.setWidget(self.timeline)
        self.addDockWidget(Qt.DockWidgetArea.BottomDockWidgetArea, bottom)

        self._build_menus()
        state.docChanged.connect(self._refresh_title)
        self._refresh_title()
        self.resize(1380, 900)

    # ---- menus ------------------------------------------------------------------

    def _build_menus(self) -> None:
        bar = self.menuBar()
        filem = bar.addMenu("&File")
        self._action(filem, "New from template…", "Ctrl+N", self.new_from_template)
        self._action(filem, "New empty", None, self.new_empty)
        self._action(filem, "Open…", "Ctrl+O", self.open_doc)
        filem.addSeparator()
        self._action(filem, "Save", "Ctrl+S", self.save)
        self._action(filem, "Save As…", "Ctrl+Shift+S", self.save_as)
        filem.addSeparator()
        self._action(filem, "Export spritesheet + GIFs…", "Ctrl+E", self.export_bundle)
        filem.addSeparator()
        self._action(filem, "Quit", "Ctrl+Q", self.close)

        editm = bar.addMenu("&Edit")
        self._action(editm, "Undo", QKeySequence.StandardKey.Undo, self._undo)
        self._action(editm, "Redo", QKeySequence.StandardKey.Redo, self._redo)
        self._action(editm, "Rename character…", None, self.rename_character)

        viewm = bar.addMenu("&View")
        bones_act = self._action(viewm, "Bone overlay", "B", self._toggle_bones, checkable=True)
        bones_act.setChecked(True)
        onion_act = self._action(viewm, "Onion skin", "O", self._toggle_onion, checkable=True)
        onion_act.setChecked(False)
        self._action(viewm, "Fit view", "F", self.canvas.fit)

    def _action(self, menu, text, shortcut, fn, checkable=False) -> QAction:
        act = QAction(text, self)
        if shortcut:
            act.setShortcut(QKeySequence(shortcut))
        act.setCheckable(checkable)
        if checkable:
            act.toggled.connect(fn)
        else:
            act.triggered.connect(fn)
        menu.addAction(act)
        return act

    # ---- file ops -----------------------------------------------------------------

    def _confirm_discard(self) -> bool:
        if not self.state.dirty:
            return True
        ret = QMessageBox.question(
            self,
            "Unsaved changes",
            "Discard unsaved changes?",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
        )
        return ret == QMessageBox.StandardButton.Yes

    def new_from_template(self) -> None:
        if not self._confirm_discard():
            return
        templates = sorted(TEMPLATE_DIR.glob("*.rig.json"))
        if not templates:
            self.new_empty()
            return
        names = [p.name for p in templates]
        name, ok = QInputDialog.getItem(self, "New character", "Template:", names, 0, False)
        if not ok:
            return
        doc = RigDocument.load(TEMPLATE_DIR / name)
        new_name, ok = QInputDialog.getText(self, "New character", "Character name:", text=doc.name)
        if ok and new_name.strip():
            doc.data["name"] = new_name.strip()
        self.state.set_doc(doc, None)
        self.canvas.fit()
        self.timeline.refresh()

    def new_empty(self) -> None:
        if not self._confirm_discard():
            return
        name, ok = QInputDialog.getText(self, "New character", "Character name:", text="new_character")
        if not ok:
            return
        self.state.set_doc(RigDocument.new_empty(name.strip() or "new_character"), None)
        self.canvas.fit()
        self.timeline.refresh()

    def open_doc(self) -> None:
        if not self._confirm_discard():
            return
        start = str(RIGGED_DIR if RIGGED_DIR.is_dir() else Path.cwd())
        path, _ = QFileDialog.getOpenFileName(self, "Open rig", start, "Rig documents (*.rig.json)")
        if not path:
            return
        try:
            doc = RigDocument.load(path)
        except Exception as ex:  # noqa: BLE001
            QMessageBox.critical(self, "Open rig", f"Failed to load:\n{ex}")
            return
        self.state.set_doc(doc, path)
        self.canvas.fit()
        self.timeline.refresh()

    def save(self) -> None:
        if not self.state.path:
            self.save_as()
            return
        self.state.doc.save(self.state.path)
        self.state.dirty = False
        self._refresh_title()
        self.statusBar().showMessage(f"Saved {self.state.path}", 4000)

    def save_as(self) -> None:
        RIGGED_DIR.mkdir(parents=True, exist_ok=True)
        suggested = str(RIGGED_DIR / f"{self.state.doc.name}.rig.json")
        path, _ = QFileDialog.getSaveFileName(self, "Save rig", suggested, "Rig documents (*.rig.json)")
        if not path:
            return
        if not path.endswith(".rig.json"):
            path += ".rig.json"
        self.state.path = path
        self.save()

    def rename_character(self) -> None:
        name, ok = QInputDialog.getText(self, "Rename character", "Name:", text=self.state.doc.name)
        name = name.strip()
        if not ok or not name:
            return
        self.state.push_undo()
        self.state.doc.data["name"] = name
        self.state.mark_changed()
        self._refresh_title()

    def export_bundle(self) -> None:
        start = self.state.path and str(Path(self.state.path).parent) or str(Path.cwd())
        out = QFileDialog.getExistingDirectory(self, "Export into directory", start)
        if not out:
            return
        app = QApplication.instance()
        app.setOverrideCursor(Qt.CursorShape.WaitCursor)
        try:
            paths = render_sheet_for_doc(self.state.doc, Path(out))
            paths += render_gifs_for_doc(self.state.doc, Path(out) / "gifs")
        except Exception as ex:  # noqa: BLE001
            app.restoreOverrideCursor()
            QMessageBox.critical(self, "Export", f"Export failed:\n{ex}")
            return
        app.restoreOverrideCursor()
        self.statusBar().showMessage(f"Exported {len(paths)} files to {out}", 8000)

    # ---- edit ops -----------------------------------------------------------------

    def _undo(self) -> None:
        if not self.state.undo():
            self.statusBar().showMessage("Nothing to undo", 2000)

    def _redo(self) -> None:
        if not self.state.redo():
            self.statusBar().showMessage("Nothing to redo", 2000)

    def _toggle_bones(self, checked: bool) -> None:
        self.canvas.show_bones = checked
        self.canvas.update()

    def _toggle_onion(self, checked: bool) -> None:
        self.canvas.onion_skin = checked
        self.canvas.update()

    def _refresh_title(self) -> None:
        star = " *" if self.state.dirty else ""
        path = self.state.path or "(unsaved)"
        self.setWindowTitle(f"{self.state.doc.name} — {path}{star} — Ambition Rig Editor")
