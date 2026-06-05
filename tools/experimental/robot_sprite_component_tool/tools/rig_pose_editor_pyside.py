#!/usr/bin/env python3
"""PySide6 rig / pose / anchor editor for the robot sprite component tool.

This is the primary GUI replacement for the earlier Tk editor.  The editing
model is intentionally split into three layers:

* component-local art metadata: crop-local pivots and named anchors
* logical part instances: front_arm, back_arm, front_leg, etc.; several
  instances can reuse the same art asset with different pivots, rotations,
  z-order, endpoint targets, and frame overrides
* rendered previews: spritesheets and animated single-action previews built
  from the current unsaved in-memory state

Run from the repository root with::

    python tools/rig_pose_editor.py examples/robot_rig_job.yaml

PySide6 is only imported for the interactive GUI.  Headless report/preview modes
continue to work without PySide6 so CI can validate the data model.
"""

from __future__ import annotations

import argparse
import math
import copy
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Mapping, Optional, Sequence, Tuple

from PIL import Image, ImageDraw

# Reuse the headless model and compositor glue from the existing editor.  Keep
# the import robust so this file works both as ``python -m tools...`` and as a
# direct script from the repo root.
try:  # pragma: no cover - the normal installed path
    from tools import rig_pose_editor as core
except Exception:  # pragma: no cover - direct script fallback
    import importlib.util

    _core_path = Path(__file__).with_name("rig_pose_editor.py")
    _spec = importlib.util.spec_from_file_location("rig_pose_editor_core", _core_path)
    if _spec is None or _spec.loader is None:
        raise
    core = importlib.util.module_from_spec(_spec)
    sys.modules[_spec.name] = core
    _spec.loader.exec_module(core)

Point = Tuple[float, float]

# Explicit editor-side rig topology.  This makes viewport selection and future
# exporter targets independent of the tree widget presentation.
RIG_TOPOLOGY = {
    "torso": {"parent": "root", "joint": "root"},
    "head": {"parent": "torso", "joint": "neck"},
    "front_arm": {"parent": "torso", "joint": "front_shoulder"},
    "back_arm": {"parent": "torso", "joint": "back_shoulder"},
    "front_hand": {"parent": "front_arm", "joint": "front_wrist"},
    "back_hand": {"parent": "back_arm", "joint": "back_wrist"},
    "front_leg": {"parent": "torso", "joint": "front_hip"},
    "back_leg": {"parent": "torso", "joint": "back_hip"},
    "front_foot": {"parent": "front_leg", "joint": "front_ankle"},
    "back_foot": {"parent": "back_leg", "joint": "back_ankle"},
}


# ---------------------------------------------------------------------------
# Headless helpers.  These deliberately do not import PySide6.


def load_paths(args: argparse.Namespace) -> core.EditorPaths:
    return core.resolve_job_paths(
        args.job.resolve(),
        args.metadata.resolve() if args.metadata else None,
        args.slices.resolve() if args.slices else None,
        args.pose_overrides.resolve() if args.pose_overrides else None,
    )


def write_anchor_report(paths: core.EditorPaths, output: Path) -> None:
    core.write_anchor_report(paths, output)


def render_preview(
    paths: core.EditorPaths,
    output: Path,
    *,
    animations: Optional[List[str]] = None,
    debug: bool = False,
    background: str = "black",
) -> None:
    meta = core.load_yaml(paths.metadata)
    pose = core.load_yaml(paths.pose_overrides)
    img, _manifest = core.build_preview(
        paths.job, meta, pose, animations=animations, debug=debug, bg=background
    )
    output.parent.mkdir(parents=True, exist_ok=True)
    img.save(output)


def pillow_to_qimage(img: Image.Image):
    """Convert a Pillow RGBA image to QImage without relying on ImageQt.

    Importing Qt is deferred until the GUI path, so this function expects
    PySide6 to be available and imported by the caller.
    """
    from PySide6 import QtGui

    img = img.convert("RGBA")
    data = img.tobytes("raw", "RGBA")
    qimg = QtGui.QImage(
        data, img.width, img.height, img.width * 4, QtGui.QImage.Format.Format_RGBA8888
    )
    # Copy to detach from the local Python bytes buffer.
    return qimg.copy()


def pillow_to_pixmap(img: Image.Image):
    from PySide6 import QtGui

    return QtGui.QPixmap.fromImage(pillow_to_qimage(img))


# ---------------------------------------------------------------------------
# Qt widgets.  These are only usable when PySide6 is installed.


try:  # pragma: no cover - availability depends on the user's workstation
    from PySide6 import QtCore as _QtCore, QtGui as _QtGui, QtWidgets as _QtWidgets  # type: ignore
except Exception:  # pragma: no cover - CI in this environment may be headless
    _QtCore = _QtGui = _QtWidgets = None  # type: ignore


class _QtUnavailable(Exception):
    pass


def _require_qt():
    global _QtCore, _QtGui, _QtWidgets
    if _QtWidgets is None:
        try:
            from PySide6 import QtCore, QtGui, QtWidgets  # type: ignore
        except Exception as ex:  # pragma: no cover - depends on environment
            raise _QtUnavailable(str(ex)) from ex
        _QtCore, _QtGui, _QtWidgets = QtCore, QtGui, QtWidgets
    return _QtCore, _QtGui, _QtWidgets


if _QtWidgets is not None:

    class PreviewGraphicsView(_QtWidgets.QGraphicsView):
        sceneClicked = _QtCore.Signal(float, float)
        sceneDragStarted = _QtCore.Signal(float, float)
        sceneDragged = _QtCore.Signal(float, float, bool)

        """Scroll/zoom-capable image view for rendered sprite previews.

        Key usability behavior:
        * mouse wheel zooms in/out around the cursor
        * panning uses the standard hand-drag mode
        * image refreshes can preserve the current zoom/pan state instead of
          constantly snapping back to "fit", which was making anchor editing
          clunky and imprecise
        """

        def __init__(self, parent=None):
            super().__init__(parent)
            self._scene = _QtWidgets.QGraphicsScene(self)
            self.setScene(self._scene)
            self._pix_item = _QtWidgets.QGraphicsPixmapItem()
            self._scene.addItem(self._pix_item)
            self._native_size = (1, 1)
            self._has_user_zoom = False
            self._direct_manipulation = False
            self._scene_dragging = False
            self.setRenderHints(
                _QtGui.QPainter.RenderHint.Antialiasing
                | _QtGui.QPainter.RenderHint.SmoothPixmapTransform
            )
            self.setDragMode(_QtWidgets.QGraphicsView.DragMode.ScrollHandDrag)
            self.setTransformationAnchor(
                _QtWidgets.QGraphicsView.ViewportAnchor.AnchorUnderMouse
            )
            self.setResizeAnchor(
                _QtWidgets.QGraphicsView.ViewportAnchor.AnchorViewCenter
            )
            self.setMouseTracking(True)

        def set_pillow_image(
            self, img: Image.Image, *, fit: bool = False, preserve_view: bool = True
        ):
            pixmap = pillow_to_pixmap(img)
            old_transform = self.transform()
            old_h = self.horizontalScrollBar().value()
            old_v = self.verticalScrollBar().value()
            self._native_size = (img.width, img.height)
            self._pix_item.setPixmap(pixmap)
            self._scene.setSceneRect(0, 0, img.width, img.height)
            if fit:
                self.resetTransform()
                self.fitInView(
                    self._scene.sceneRect(), _QtCore.Qt.AspectRatioMode.KeepAspectRatio
                )
                self._has_user_zoom = False
            elif preserve_view:
                self.setTransform(old_transform)
                self.horizontalScrollBar().setValue(old_h)
                self.verticalScrollBar().setValue(old_v)

        def zoom_by(self, factor: float) -> None:
            if factor <= 0:
                return
            self.scale(factor, factor)
            self._has_user_zoom = True

        def fit_scene(self) -> None:
            self.resetTransform()
            self.fitInView(
                self._scene.sceneRect(), _QtCore.Qt.AspectRatioMode.KeepAspectRatio
            )
            self._has_user_zoom = False

        def set_direct_manipulation(self, enabled: bool) -> None:
            self._direct_manipulation = bool(enabled)
            if enabled:
                self.setDragMode(_QtWidgets.QGraphicsView.DragMode.NoDrag)
            else:
                self.setDragMode(_QtWidgets.QGraphicsView.DragMode.ScrollHandDrag)

        def _scene_point_from_event(self, event):
            p = self.mapToScene(event.pos())
            return float(p.x()), float(p.y())

        def _scene_in_bounds(self, x: float, y: float) -> bool:
            return 0 <= x <= self._native_size[0] and 0 <= y <= self._native_size[1]

        def mousePressEvent(self, event):  # type: ignore[override]
            if event.button() == _QtCore.Qt.MouseButton.LeftButton:
                x, y = self._scene_point_from_event(event)
                if self._scene_in_bounds(x, y):
                    self.sceneClicked.emit(x, y)
                    if self._direct_manipulation:
                        self._scene_dragging = True
                        self.sceneDragStarted.emit(x, y)
                        event.accept()
                        return
            super().mousePressEvent(event)

        def mouseMoveEvent(self, event):  # type: ignore[override]
            if self._scene_dragging:
                x, y = self._scene_point_from_event(event)
                self.sceneDragged.emit(x, y, False)
                event.accept()
                return
            super().mouseMoveEvent(event)

        def mouseReleaseEvent(self, event):  # type: ignore[override]
            if (
                self._scene_dragging
                and event.button() == _QtCore.Qt.MouseButton.LeftButton
            ):
                x, y = self._scene_point_from_event(event)
                self.sceneDragged.emit(x, y, True)
                self._scene_dragging = False
                event.accept()
                return
            super().mouseReleaseEvent(event)

        def wheelEvent(self, event):  # type: ignore[override]
            delta = event.angleDelta().y()
            if delta == 0:
                super().wheelEvent(event)
                return
            self.zoom_by(1.15 if delta > 0 else (1.0 / 1.15))
            event.accept()

    class AnchorGraphicsView(PreviewGraphicsView):
        """Direct-manipulation anchor editor canvas.

        Emits image-space coordinates. Left click places the selected anchor.
        Left drag continuously updates it. Mouse wheel zooms and drag-hand pans,
        which makes precise anchor placement dramatically easier than the old
        click-only behavior.
        """

        imageClicked = _QtCore.Signal(float, float)
        imageDragged = _QtCore.Signal(float, float, bool)
        hoverMoved = _QtCore.Signal(float, float)

        def __init__(self, parent=None):
            super().__init__(parent)
            self._dragging_anchor = False

        def _scene_xy(self, event):
            p = self.mapToScene(event.pos())
            return float(p.x()), float(p.y())

        def _in_bounds(self, x: float, y: float) -> bool:
            return 0 <= x <= self._native_size[0] and 0 <= y <= self._native_size[1]

        def mousePressEvent(self, event):  # type: ignore[override]
            if event.button() == _QtCore.Qt.MouseButton.LeftButton:
                x, y = self._scene_xy(event)
                if self._in_bounds(x, y):
                    self._dragging_anchor = True
                    self.imageClicked.emit(x, y)
                    self.imageDragged.emit(x, y, False)
                    event.accept()
                    return
            super().mousePressEvent(event)

        def mouseMoveEvent(self, event):  # type: ignore[override]
            x, y = self._scene_xy(event)
            if self._in_bounds(x, y):
                self.hoverMoved.emit(x, y)
                if self._dragging_anchor:
                    self.imageDragged.emit(x, y, False)
                    event.accept()
                    return
            super().mouseMoveEvent(event)

        def mouseReleaseEvent(self, event):  # type: ignore[override]
            if (
                self._dragging_anchor
                and event.button() == _QtCore.Qt.MouseButton.LeftButton
            ):
                x, y = self._scene_xy(event)
                if self._in_bounds(x, y):
                    self.imageDragged.emit(x, y, True)
                    event.accept()
                self._dragging_anchor = False
                return
            super().mouseReleaseEvent(event)
else:

    class PreviewGraphicsView:  # pragma: no cover - only instantiated when Qt exists
        def __init__(self, *args, **kwargs):
            raise _QtUnavailable("PySide6 is required for PreviewGraphicsView")

    class AnchorGraphicsView(PreviewGraphicsView):  # pragma: no cover
        pass


class RigPoseEditorQt:
    """Main PySide6 editor window.

    The public surface is intentionally similar to the Tk version, but this
    editor uses native Qt splitters, views, and lists so the layout is usable on
    wide sprite sheets.
    """

    def __init__(
        self, paths: core.EditorPaths, *, zoom: int = 6, background: str = "checker"
    ):
        QtCore, QtGui, QtWidgets = _require_qt()
        self.QtCore = QtCore
        self.QtGui = QtGui
        self.QtWidgets = QtWidgets

        self.paths = paths
        self.zoom = max(1, int(zoom))
        self.background = background
        self.rig = core.import_rig_module()
        self.job = self.rig.RigJob.load(paths.job)
        self.job.metadata = paths.metadata
        self.job.slices = paths.slices
        self.job.pose_overrides = paths.pose_overrides
        self.metadata: Dict[str, Any] = core.load_yaml(paths.metadata)
        self.pose_model = core.PoseModel(core.load_yaml(paths.pose_overrides))
        self.sprites: Dict[str, Any] = self.metadata.setdefault("sprites", {})
        self.sprite_names = sorted(self.sprites)
        self.selected_sprite = self.sprite_names[0] if self.sprite_names else ""
        self.selected_anchor = "pivot"
        self.selected_instance: Optional[Dict[str, Any]] = None
        self.current_manifest: Dict[str, Any] = {}
        self.dirty_meta = False
        self.dirty_pose = False
        self._refreshing = False
        self._preview_timer = QtCore.QTimer()
        self._preview_timer.setSingleShot(True)
        self._preview_timer.timeout.connect(self._timer_refresh_preview)
        self._play_timer = QtCore.QTimer()
        self._play_timer.timeout.connect(self.advance_animation)
        self._play_index = 0
        self._updating_timeline = False
        self._updating_z_list = False
        self._fast_drag_renderer = None
        self._fast_drag_meta_path = None
        self._anim_view_dirty = False

        self.window = QtWidgets.QMainWindow()
        self.window.setWindowTitle("Robot Rig Pose Editor - PySide6")
        self.window.resize(2300, 1300)
        self.window.setMinimumSize(1400, 850)
        self.status = self.window.statusBar()
        self._build_ui()
        self._connect_signals()
        self.populate_sprite_list()
        self.populate_animation_tree()
        if self.selected_sprite:
            self.select_sprite(self.selected_sprite)
        self.refresh_preview(force=True)

    # UI construction --------------------------------------------------
    def _build_ui(self) -> None:
        QtCore, QtGui, QtWidgets = self.QtCore, self.QtGui, self.QtWidgets
        central = QtWidgets.QWidget()
        self.window.setCentralWidget(central)
        layout = QtWidgets.QVBoxLayout(central)
        layout.setContentsMargins(6, 6, 6, 6)
        layout.setSpacing(6)

        topbar = QtWidgets.QHBoxLayout()
        self.save_button = QtWidgets.QPushButton("Save metadata + pose overrides")
        self.refresh_button = QtWidgets.QPushButton("Render now")
        self.play_button = QtWidgets.QPushButton("Play")
        self.preview_relevant = QtWidgets.QCheckBox("relevant only")
        self.preview_relevant.setChecked(True)
        self.preview_debug = QtWidgets.QCheckBox("debug colors")
        self.preview_fit = QtWidgets.QCheckBox("fit")
        self.preview_fit.setChecked(True)
        self.live_sheet_preview = QtWidgets.QCheckBox("live sheet")
        self.live_sheet_preview.setChecked(True)
        self.live_sheet_preview.setToolTip(
            "When off, edits update only the action preview until Render now is pressed."
        )
        self.show_bones = QtWidgets.QCheckBox("bones")
        self.show_bones.setChecked(True)
        self.fast_drag_preview = QtWidgets.QCheckBox("fast drag")
        self.fast_drag_preview.setChecked(True)
        self.fast_drag_preview.setToolTip(
            "During drag, skip full-sheet/tree refresh and update only the action-preview canvas."
        )
        self.preview_bg = QtWidgets.QComboBox()
        self.preview_bg.addItems(["black", "checker", "white"])
        self.preview_bg.setCurrentText("black")
        topbar.addWidget(self.save_button)
        topbar.addWidget(self.refresh_button)
        # Playback controls live above the animated preview so the timeline and
        # play/pause button stay together.
        topbar.addSpacing(20)
        topbar.addWidget(self.preview_relevant)
        topbar.addWidget(self.preview_debug)
        topbar.addWidget(self.preview_fit)
        topbar.addWidget(self.live_sheet_preview)
        topbar.addWidget(self.show_bones)
        topbar.addWidget(self.fast_drag_preview)
        topbar.addWidget(QtWidgets.QLabel("background"))
        topbar.addWidget(self.preview_bg)
        topbar.addStretch(1)
        layout.addLayout(topbar)

        # Three resizable columns.  Qt splitters give the resize behavior that
        # was awkward in Tk.
        self.right_panel = self._make_preview_panel()
        layout.addWidget(self.right_panel, 1)

        # Professional workflow layout: make the major tool panes dockable so
        # users can hide, float, tab, or rearrange them like Godot/Qt Creator.
        self.left_panel = self._make_animation_panel()
        self.center_panel = self._make_component_panel()
        self._dock_widgets = []
        self.animation_dock = self._add_dock(
            "Animation / Rig",
            self.left_panel,
            QtCore.Qt.DockWidgetArea.LeftDockWidgetArea,
        )
        self.component_dock = self._add_dock(
            "Components / Anchors",
            self.center_panel,
            QtCore.Qt.DockWidgetArea.RightDockWidgetArea,
        )
        self.window.tabifyDockWidget(self.animation_dock, self.component_dock)
        self.animation_dock.raise_()
        view_menu = self.window.menuBar().addMenu("View")
        for dock in self._dock_widgets:
            view_menu.addAction(dock.toggleViewAction())

    def _add_dock(self, title: str, widget, area):
        dock = self.QtWidgets.QDockWidget(title, self.window)
        dock.setObjectName(title.replace(" ", "_").replace("/", "_"))
        dock.setWidget(widget)
        dock.setAllowedAreas(
            self.QtCore.Qt.DockWidgetArea.LeftDockWidgetArea
            | self.QtCore.Qt.DockWidgetArea.RightDockWidgetArea
            | self.QtCore.Qt.DockWidgetArea.TopDockWidgetArea
            | self.QtCore.Qt.DockWidgetArea.BottomDockWidgetArea
        )
        dock.setFeatures(
            self.QtWidgets.QDockWidget.DockWidgetFeature.DockWidgetClosable
            | self.QtWidgets.QDockWidget.DockWidgetFeature.DockWidgetMovable
            | self.QtWidgets.QDockWidget.DockWidgetFeature.DockWidgetFloatable
        )
        self.window.addDockWidget(area, dock)
        self._dock_widgets.append(dock)
        return dock

    def _make_animation_panel(self):
        QtCore, QtGui, QtWidgets = self.QtCore, self.QtGui, self.QtWidgets
        panel = QtWidgets.QWidget()
        layout = QtWidgets.QVBoxLayout(panel)
        layout.setContentsMargins(0, 0, 0, 0)
        splitter = QtWidgets.QSplitter(QtCore.Qt.Orientation.Vertical)
        splitter.setChildrenCollapsible(False)
        layout.addWidget(splitter, 1)

        tree_box = QtWidgets.QWidget()
        tree_layout = QtWidgets.QVBoxLayout(tree_box)
        tree_layout.setContentsMargins(0, 0, 0, 0)
        tree_layout.addWidget(QtWidgets.QLabel("Animation / frame / logical part tree"))
        self.tree = QtWidgets.QTreeWidget()
        self.tree.setHeaderLabels(["Instance", "Art", "Constraint"])
        self.tree.setColumnWidth(0, 190)
        self.tree.setColumnWidth(1, 160)
        tree_layout.addWidget(self.tree, 1)
        splitter.addWidget(tree_box)

        form = QtWidgets.QGroupBox("Selected instance / frame")
        grid = QtWidgets.QGridLayout(form)
        self.pose_anim = QtWidgets.QLineEdit()
        self.pose_frame = QtWidgets.QSpinBox()
        self.pose_frame.setRange(0, 999)
        self.pose_role = QtWidgets.QLineEdit()
        self.pose_art = QtWidgets.QComboBox()
        self.pose_art.setEditable(True)
        self.pose_art.addItems(self.sprite_names)
        self.pose_angle = QtWidgets.QDoubleSpinBox()
        self.pose_angle.setRange(-360, 360)
        self.pose_angle.setDecimals(2)
        self.pose_scale = QtWidgets.QDoubleSpinBox()
        self.pose_scale.setRange(0.05, 3.0)
        self.pose_scale.setDecimals(3)
        self.pose_scale.setSingleStep(0.025)
        self.pose_scale.setToolTip(
            "Baseline part scale multiplier. Arms/legs may still endpoint-solve to a slightly different effective scale so their wrist/ground joint lands on target."
        )
        self.pose_dx = QtWidgets.QDoubleSpinBox()
        self.pose_dx.setRange(-300, 300)
        self.pose_dx.setDecimals(2)
        self.pose_dy = QtWidgets.QDoubleSpinBox()
        self.pose_dy.setRange(-300, 300)
        self.pose_dy.setDecimals(2)
        grid.addWidget(QtWidgets.QLabel("anim"), 0, 0)
        grid.addWidget(self.pose_anim, 0, 1)
        grid.addWidget(QtWidgets.QLabel("frame"), 0, 2)
        grid.addWidget(self.pose_frame, 0, 3)
        grid.addWidget(QtWidgets.QLabel("role"), 1, 0)
        grid.addWidget(self.pose_role, 1, 1)
        grid.addWidget(QtWidgets.QLabel("art"), 1, 2)
        grid.addWidget(self.pose_art, 1, 3)
        grid.addWidget(QtWidgets.QLabel("angle"), 2, 0)
        grid.addWidget(self.pose_angle, 2, 1)
        grid.addWidget(QtWidgets.QLabel("scale"), 2, 2)
        grid.addWidget(self.pose_scale, 2, 3)
        grid.addWidget(QtWidgets.QLabel("delta x"), 3, 0)
        grid.addWidget(self.pose_dx, 3, 1)
        grid.addWidget(QtWidgets.QLabel("delta y"), 3, 2)
        grid.addWidget(self.pose_dy, 3, 3)
        self.apply_pose_button = QtWidgets.QPushButton("Apply pose edit")
        self.navigate_button = QtWidgets.QPushButton("Navigate joint -> connected part")
        self.clear_frame_button = QtWidgets.QPushButton("Clear this frame keyframe")
        self.key_status = QtWidgets.QLabel("keyframe: inherited")
        grid.addWidget(self.apply_pose_button, 3, 2, 1, 2)
        grid.addWidget(self.navigate_button, 4, 0, 1, 4)
        grid.addWidget(self.clear_frame_button, 5, 0, 1, 2)
        grid.addWidget(self.key_status, 5, 2, 1, 2)
        splitter.addWidget(form)

        constraints = QtWidgets.QGroupBox("Rig joint constraints for current frame")
        clayout = QtWidgets.QVBoxLayout(constraints)
        note = QtWidgets.QLabel(
            "Standard rig policy: edit stable component-local sockets and animate bones/endpoints; "
            "parent/child joint deltas are advanced corrective offsets only."
        )
        note.setWordWrap(True)
        clayout.addWidget(note)
        self.connection_table = QtWidgets.QTableWidget(0, 6)
        self.connection_table.setHorizontalHeaderLabels(
            ["role", "joint", "child anchor", "parent socket", "snap px", "visible"]
        )
        self.connection_table.setSelectionBehavior(
            QtWidgets.QAbstractItemView.SelectionBehavior.SelectRows
        )
        self.connection_table.setEditTriggers(
            QtWidgets.QAbstractItemView.EditTrigger.NoEditTriggers
        )
        self.connection_table.verticalHeader().setVisible(False)
        self.connection_table.horizontalHeader().setStretchLastSection(True)
        clayout.addWidget(self.connection_table, 1)
        cbuttons = QtWidgets.QHBoxLayout()
        self.select_child_part_button = QtWidgets.QPushButton("Select child part")
        self.select_parent_part_button = QtWidgets.QPushButton("Select parent part")
        self.select_child_anchor_button = QtWidgets.QPushButton("Child anchor")
        self.select_parent_anchor_button = QtWidgets.QPushButton("Parent socket")
        cbuttons.addWidget(self.select_child_part_button)
        cbuttons.addWidget(self.select_parent_part_button)
        cbuttons.addWidget(self.select_child_anchor_button)
        cbuttons.addWidget(self.select_parent_anchor_button)
        clayout.addLayout(cbuttons)
        splitter.addWidget(constraints)

        timing = QtWidgets.QGroupBox("Animation length / timing")
        tgrid = QtWidgets.QGridLayout(timing)
        self.anim_frame_count = QtWidgets.QSpinBox()
        self.anim_frame_count.setRange(1, 128)
        self.anim_duration_ms = QtWidgets.QSpinBox()
        self.anim_duration_ms.setRange(10, 1000)
        self.add_frame_button = QtWidgets.QPushButton("Add frame")
        self.remove_frame_button = QtWidgets.QPushButton("Remove last")
        tgrid.addWidget(QtWidgets.QLabel("frames"), 0, 0)
        tgrid.addWidget(self.anim_frame_count, 0, 1)
        tgrid.addWidget(QtWidgets.QLabel("ms/frame"), 0, 2)
        tgrid.addWidget(self.anim_duration_ms, 0, 3)
        tgrid.addWidget(self.add_frame_button, 1, 0, 1, 2)
        tgrid.addWidget(self.remove_frame_button, 1, 2, 1, 2)
        splitter.addWidget(timing)

        zbox = QtWidgets.QGroupBox("Frame z-order / visibility (bottom -> top)")
        zlayout = QtWidgets.QHBoxLayout(zbox)
        self.z_list = QtWidgets.QListWidget()
        zlayout.addWidget(self.z_list, 1)
        zbuttons = QtWidgets.QVBoxLayout()
        self.z_up = QtWidgets.QPushButton("Up")
        self.z_down = QtWidgets.QPushButton("Down")
        self.z_reset = QtWidgets.QPushButton("Reset order")
        self.z_show_all = QtWidgets.QPushButton("Show all")
        self.z_only_selected = QtWidgets.QPushButton("Only selected")
        zbuttons.addWidget(self.z_up)
        zbuttons.addWidget(self.z_down)
        zbuttons.addWidget(self.z_reset)
        zbuttons.addWidget(self.z_show_all)
        zbuttons.addWidget(self.z_only_selected)
        zbuttons.addStretch(1)
        zlayout.addLayout(zbuttons)
        splitter.addWidget(zbox)
        splitter.setSizes([520, 270, 240, 140, 260])
        return panel

    def _make_component_panel(self):
        _QtCore, _QtGui, QtWidgets = self.QtCore, self.QtGui, self.QtWidgets
        panel = QtWidgets.QWidget()
        layout = QtWidgets.QVBoxLayout(panel)
        layout.setContentsMargins(0, 0, 0, 0)
        splitter = QtWidgets.QSplitter(self.QtCore.Qt.Orientation.Vertical)
        splitter.setChildrenCollapsible(False)
        layout.addWidget(splitter, 1)

        sprite_box = QtWidgets.QWidget()
        sprite_layout = QtWidgets.QVBoxLayout(sprite_box)
        sprite_layout.setContentsMargins(0, 0, 0, 0)
        filt = QtWidgets.QHBoxLayout()
        filt.addWidget(QtWidgets.QLabel("Sprite filter"))
        self.sprite_filter = QtWidgets.QLineEdit()
        filt.addWidget(self.sprite_filter, 1)
        sprite_layout.addLayout(filt)
        self.sprite_list = QtWidgets.QListWidget()
        sprite_layout.addWidget(self.sprite_list, 1)
        splitter.addWidget(sprite_box)

        anchor_box = QtWidgets.QGroupBox("Anchor editor")
        alayout = QtWidgets.QVBoxLayout(anchor_box)
        self.anchor_list = QtWidgets.QListWidget()
        alayout.addWidget(self.anchor_list, 1)
        xy = QtWidgets.QGridLayout()
        self.anchor_x = QtWidgets.QDoubleSpinBox()
        self.anchor_x.setRange(-1000, 1000)
        self.anchor_x.setDecimals(2)
        self.anchor_x.setSingleStep(0.25)
        self.anchor_y = QtWidgets.QDoubleSpinBox()
        self.anchor_y.setRange(-1000, 1000)
        self.anchor_y.setDecimals(2)
        self.anchor_y.setSingleStep(0.25)
        self.snap_anchor_pixels = QtWidgets.QCheckBox("snap click/drag to pixel")
        self.snap_anchor_pixels.setChecked(True)
        self.apply_anchor = QtWidgets.QPushButton("Apply anchor")
        self.pivot_follow = QtWidgets.QPushButton("Pivot follows selected")
        self.anchor_zoom_fit = QtWidgets.QPushButton("Fit")
        self.anchor_zoom_in = QtWidgets.QPushButton("+")
        self.anchor_zoom_out = QtWidgets.QPushButton("-")
        xy.addWidget(QtWidgets.QLabel("x"), 0, 0)
        xy.addWidget(self.anchor_x, 0, 1)
        xy.addWidget(QtWidgets.QLabel("y"), 0, 2)
        xy.addWidget(self.anchor_y, 0, 3)
        xy.addWidget(self.snap_anchor_pixels, 1, 0, 1, 4)
        xy.addWidget(self.apply_anchor, 2, 0)
        xy.addWidget(self.pivot_follow, 2, 1, 1, 3)
        xy.addWidget(self.anchor_zoom_out, 3, 0)
        xy.addWidget(self.anchor_zoom_fit, 3, 1, 1, 2)
        xy.addWidget(self.anchor_zoom_in, 3, 3)
        alayout.addLayout(xy)
        hint = QtWidgets.QLabel(
            "Direct manipulation: mouse wheel = zoom, hand-drag = pan, left click/drag = move selected anchor."
        )
        hint.setWordWrap(True)
        alayout.addWidget(hint)
        splitter.addWidget(anchor_box)

        self.component_view = AnchorGraphicsView()
        splitter.addWidget(self.component_view)
        splitter.setSizes([240, 280, 760])
        return panel

    def _make_preview_panel(self):
        _QtCore, _QtGui, QtWidgets = self.QtCore, self.QtGui, self.QtWidgets
        panel = QtWidgets.QWidget()
        layout = QtWidgets.QVBoxLayout(panel)
        layout.setContentsMargins(0, 0, 0, 0)
        splitter = QtWidgets.QSplitter(self.QtCore.Qt.Orientation.Vertical)
        splitter.setChildrenCollapsible(False)
        layout.addWidget(splitter, 1)

        preview_box = QtWidgets.QWidget()
        preview_layout = QtWidgets.QVBoxLayout(preview_box)
        preview_layout.setContentsMargins(0, 0, 0, 0)
        preview_layout.addWidget(
            QtWidgets.QLabel("Live spritesheet preview (current unsaved state)")
        )
        self.preview_view = PreviewGraphicsView()
        preview_layout.addWidget(self.preview_view, 1)
        splitter.addWidget(preview_box)

        anim_box = QtWidgets.QWidget()
        anim_layout = QtWidgets.QVBoxLayout(anim_box)
        anim_layout.setContentsMargins(0, 0, 0, 0)
        anim_layout.addWidget(
            QtWidgets.QLabel(
                "Animated action preview — drag parts to move endpoints/offsets; Shift-drag rotates"
            )
        )
        timeline = QtWidgets.QHBoxLayout()
        self.prev_frame_button = QtWidgets.QPushButton("<")
        self.next_frame_button = QtWidgets.QPushButton(">")
        self.frame_slider = QtWidgets.QSlider(self.QtCore.Qt.Orientation.Horizontal)
        self.frame_slider.setRange(0, 0)
        self.frame_label = QtWidgets.QLabel("frame 0 / 0")
        timeline.addWidget(self.prev_frame_button)
        timeline.addWidget(self.play_button)
        timeline.addWidget(self.next_frame_button)
        timeline.addWidget(self.frame_slider, 1)
        timeline.addWidget(self.frame_label)
        anim_layout.addLayout(timeline)
        self.anim_view = PreviewGraphicsView()
        self.anim_view.set_direct_manipulation(True)
        anim_layout.addWidget(self.anim_view, 1)
        splitter.addWidget(anim_box)
        splitter.setSizes([820, 360])
        return panel

    def _connect_signals(self) -> None:
        QtCore = self.QtCore
        self.save_button.clicked.connect(self.save)
        self.refresh_button.clicked.connect(lambda: self.refresh_preview(force=True))
        self.play_button.clicked.connect(self.toggle_play)
        self.preview_relevant.stateChanged.connect(
            lambda _=None: self.refresh_preview()
        )
        self.preview_debug.stateChanged.connect(lambda _=None: self.refresh_preview())
        self.preview_fit.stateChanged.connect(
            lambda _=None: self.refresh_preview(force=True)
        )
        self.live_sheet_preview.stateChanged.connect(
            lambda _=None: self.refresh_preview(force=True)
        )
        self.show_bones.stateChanged.connect(
            lambda _=None: self.render_animation_frame()
        )
        self.preview_bg.currentTextChanged.connect(
            lambda _=None: self.refresh_preview()
        )
        self.preview_view.sceneClicked.connect(self.on_spritesheet_preview_click)
        self.anim_view.sceneClicked.connect(self.on_action_preview_click)
        self.anim_view.sceneDragStarted.connect(self.on_action_preview_drag_start)
        self.anim_view.sceneDragged.connect(self.on_action_preview_drag_move)
        self.tree.itemSelectionChanged.connect(self.on_tree_select)
        self.sprite_filter.textChanged.connect(
            lambda _=None: self.populate_sprite_list()
        )
        self.sprite_list.currentTextChanged.connect(self.on_sprite_select)
        self.anchor_list.currentTextChanged.connect(self.on_anchor_select)
        self.component_view.imageClicked.connect(self.on_component_click)
        self.component_view.imageDragged.connect(self.on_component_drag)
        self.component_view.hoverMoved.connect(self.on_component_hover)
        self.apply_anchor.clicked.connect(self.apply_anchor_xy)
        self.pivot_follow.clicked.connect(self.pivot_follows_selected)
        self.anchor_zoom_in.clicked.connect(lambda: self.component_view.zoom_by(1.15))
        self.anchor_zoom_out.clicked.connect(
            lambda: self.component_view.zoom_by(1.0 / 1.15)
        )
        self.anchor_zoom_fit.clicked.connect(lambda: self.component_view.fit_scene())
        self.apply_pose_button.clicked.connect(
            lambda _=None: self.apply_pose_edit(refresh=True, changed_fields=None)
        )
        self.navigate_button.clicked.connect(self.navigate_connected_part)
        self.clear_frame_button.clicked.connect(self.clear_current_frame_override)
        self.pose_frame.valueChanged.connect(
            lambda _=None: self.on_pose_frame_changed()
        )
        self.prev_frame_button.clicked.connect(lambda: self.step_frame(-1))
        self.next_frame_button.clicked.connect(lambda: self.step_frame(1))
        self.frame_slider.valueChanged.connect(self.on_timeline_frame_changed)
        self.connection_table.itemSelectionChanged.connect(self.on_connection_select)
        self.connection_table.itemDoubleClicked.connect(
            lambda _=None: self.select_connection_child_part()
        )
        self.select_child_part_button.clicked.connect(self.select_connection_child_part)
        self.select_parent_part_button.clicked.connect(
            self.select_connection_parent_part
        )
        self.select_child_anchor_button.clicked.connect(
            self.select_connection_child_anchor
        )
        self.select_parent_anchor_button.clicked.connect(
            self.select_connection_parent_anchor
        )
        self.pose_art.currentTextChanged.connect(
            lambda _=None: self.defer_pose_edit({"sprite"})
        )
        self.pose_angle.valueChanged.connect(
            lambda _=None: self.defer_pose_edit({"angle"})
        )
        self.pose_scale.valueChanged.connect(
            lambda _=None: self.defer_pose_edit({"scale"})
        )
        self.pose_dx.valueChanged.connect(
            lambda _=None: self.defer_pose_edit({"delta"})
        )
        self.pose_dy.valueChanged.connect(
            lambda _=None: self.defer_pose_edit({"delta"})
        )
        self.anim_frame_count.valueChanged.connect(
            lambda _=None: self.apply_anim_settings()
        )
        self.anim_duration_ms.valueChanged.connect(
            lambda _=None: self.apply_anim_settings()
        )
        self.add_frame_button.clicked.connect(self.add_frame)
        self.remove_frame_button.clicked.connect(self.remove_frame)
        self.z_up.clicked.connect(lambda: self.move_z(-1))
        self.z_down.clicked.connect(lambda: self.move_z(1))
        self.z_reset.clicked.connect(self.reset_z_order)
        self.z_show_all.clicked.connect(self.show_all_parts)
        self.z_only_selected.clicked.connect(self.show_only_selected_part)
        self.z_list.itemChanged.connect(self.on_z_item_changed)
        self.window.shortcut_save = self.QtGui.QShortcut(
            self.QtGui.QKeySequence("Ctrl+S"), self.window
        )
        self.window.shortcut_save.activated.connect(self.save)
        self.window.shortcut_refresh = self.QtGui.QShortcut(
            self.QtGui.QKeySequence("Ctrl+R"), self.window
        )
        self.window.shortcut_refresh.activated.connect(
            lambda: self.refresh_preview(force=True)
        )

    # Data / selection --------------------------------------------------
    def populate_sprite_list(self) -> None:
        filt = (
            self.sprite_filter.text().strip().lower()
            if hasattr(self, "sprite_filter")
            else ""
        )
        current = self.selected_sprite
        self.sprite_list.blockSignals(True)
        self.sprite_list.clear()
        for name in self.sprite_names:
            if not filt or filt in name.lower():
                self.sprite_list.addItem(name)
        matches = self.sprite_list.findItems(
            current, self.QtCore.Qt.MatchFlag.MatchExactly
        )
        if matches:
            self.sprite_list.setCurrentItem(matches[0])
        self.sprite_list.blockSignals(False)

    def populate_animation_tree(self) -> None:
        self.tree.blockSignals(True)
        self.tree.clear()
        try:
            img, manifest = core.build_preview(
                self.paths.job,
                self.metadata,
                self.pose_model.clean_for_save(),
                animations=list(self.job.animations),
                debug=False,
                bg="black",
            )
            self.current_manifest = manifest
        except Exception as ex:
            self.status.showMessage(f"Tree render failed: {ex}")
            self.tree.blockSignals(False)
            return
        for anim, adata in (self.current_manifest.get("animations") or {}).items():
            anim_item = self.QtWidgets.QTreeWidgetItem(
                [anim, f"{adata.get('duration_ms')}ms"]
            )
            anim_item.setData(
                0,
                self.QtCore.Qt.ItemDataRole.UserRole,
                {"kind": "anim", "animation": anim},
            )
            self.tree.addTopLevelItem(anim_item)
            for frame in adata.get("frames", []):
                idx = int(frame.get("index", 0))
                frame_item = self.QtWidgets.QTreeWidgetItem([f"frame {idx}", ""])
                frame_item.setData(
                    0,
                    self.QtCore.Qt.ItemDataRole.UserRole,
                    {"kind": "frame", "animation": anim, "frame": idx},
                )
                anim_item.addChild(frame_item)
                for comp in (frame.get("pose") or {}).get("components", []):
                    role = str(comp.get("role"))
                    conn = comp.get("connects_to") or {}
                    child_anchor = comp.get("anchor") or ""
                    parent_role = conn.get("role") or ""
                    parent_anchor = conn.get("anchor") or ""
                    err = comp.get("snap_error_px")
                    if conn:
                        constraint = f"{child_anchor} -> {parent_role}.{parent_anchor}; err={err}"
                    else:
                        constraint = ""
                    part_item = self.QtWidgets.QTreeWidgetItem(
                        [role, str(comp.get("sprite", "")), constraint]
                    )
                    part_item.setData(
                        0,
                        self.QtCore.Qt.ItemDataRole.UserRole,
                        {"kind": "part", "animation": anim, "frame": idx, "role": role},
                    )
                    frame_item.addChild(part_item)
            if (
                anim == self.pose_anim.text()
                if hasattr(self, "pose_anim")
                else anim == "run"
            ):
                anim_item.setExpanded(True)
        self.tree.blockSignals(False)

    def on_tree_select(self) -> None:
        items = self.tree.selectedItems()
        if not items:
            return
        data = items[0].data(0, self.QtCore.Qt.ItemDataRole.UserRole) or {}
        kind = data.get("kind")
        if kind == "anim":
            self.pose_anim.setText(data["animation"])
            self.update_anim_fields()
            self.refresh_preview()
        elif kind == "frame":
            self.pose_anim.setText(data["animation"])
            self.pose_frame.setValue(int(data["frame"]))
            self.update_anim_fields()
            self.refresh_preview()
        elif kind == "part":
            anim, idx, role = data["animation"], int(data["frame"]), data["role"]
            comp = self.find_component(anim, idx, role)
            if not comp:
                return
            self.selected_instance = {
                "animation": anim,
                "frame_index": idx,
                "role": role,
                "sprite": comp.get("sprite"),
            }
            self.pose_anim.setText(anim)
            self.pose_frame.setValue(idx)
            self.pose_role.setText(role)
            self.pose_art.setCurrentText(str(comp.get("sprite", "")))
            self.load_pose_fields_from_model(role, anim, idx, comp)
            base = str(comp.get("sprite", "")).split("@")[0]
            if base in self.sprites:
                self.select_sprite(base)
            self.populate_z_order()
            self.update_connection_table()
            self.refresh_preview(force=True)

    def find_component(
        self, anim: str, idx: int, role: str
    ) -> Optional[Dict[str, Any]]:
        adata = (self.current_manifest.get("animations") or {}).get(anim) or {}
        for frame in adata.get("frames", []):
            if int(frame.get("index", -1)) == int(idx):
                for comp in (frame.get("pose") or {}).get("components", []):
                    if comp.get("role") == role:
                        return comp
        return None

    def on_sprite_select(self, text: str) -> None:
        if text:
            self.select_sprite(text)
            self.refresh_preview()

    def select_sprite(self, name: str) -> None:
        if name not in self.sprites:
            return
        self.selected_sprite = name
        self.populate_anchor_list()
        matches = self.sprite_list.findItems(
            name, self.QtCore.Qt.MatchFlag.MatchExactly
        )
        if matches:
            self.sprite_list.blockSignals(True)
            self.sprite_list.setCurrentItem(matches[0])
            self.sprite_list.blockSignals(False)
        self.draw_component()

    # Anchor editing ----------------------------------------------------
    def anchor_names(self) -> List[str]:
        s = self.sprites.get(self.selected_sprite, {})
        return ["pivot"] + sorted((s.get("anchors") or {}).keys())

    def populate_anchor_list(self) -> None:
        self.anchor_list.blockSignals(True)
        self.anchor_list.clear()
        for name in self.anchor_names():
            self.anchor_list.addItem(name)
        if self.selected_anchor not in self.anchor_names():
            self.selected_anchor = "pivot"
        matches = self.anchor_list.findItems(
            self.selected_anchor, self.QtCore.Qt.MatchFlag.MatchExactly
        )
        if matches:
            self.anchor_list.setCurrentItem(matches[0])
        self.anchor_list.blockSignals(False)
        self.update_anchor_fields()

    def on_anchor_select(self, name: str) -> None:
        if name:
            self.selected_anchor = name
            self.update_anchor_fields()
            self.draw_component()

    def get_anchor_point(self, name: str) -> Point:
        s = self.sprites.get(self.selected_sprite, {})
        if name == "pivot":
            pa = s.get("pivot_anchor")
            if pa and pa in (s.get("anchors") or {}):
                return core.point(s["anchors"][pa])
            return core.point(s.get("pivot"), (0, 0))
        return core.point((s.get("anchors") or {}).get(name), (0, 0))

    def set_anchor_point(self, name: str, pt: Point) -> None:
        s = self.sprites[self.selected_sprite]
        if name == "pivot":
            s.pop("pivot_anchor", None)
            s["pivot"] = core.point_list(pt)
        else:
            s.setdefault("anchors", {})[name] = core.point_list(pt)
            if s.get("pivot_anchor") == name:
                s["pivot"] = core.point_list(pt)
        self.dirty_meta = True

    def update_anchor_fields(self) -> None:
        pt = self.get_anchor_point(self.selected_anchor)
        self.anchor_x.blockSignals(True)
        self.anchor_y.blockSignals(True)
        self.anchor_x.setValue(pt[0])
        self.anchor_y.setValue(pt[1])
        self.anchor_x.blockSignals(False)
        self.anchor_y.blockSignals(False)

    def _normalize_anchor_point(self, x: float, y: float) -> Point:
        if (
            getattr(self, "snap_anchor_pixels", None) is not None
            and self.snap_anchor_pixels.isChecked()
        ):
            x, y = round(x), round(y)
        return (float(x), float(y))

    def _apply_anchor_point(
        self,
        pt: Point,
        *,
        refresh_tree: bool = False,
        refresh_preview: bool = False,
        final: bool = False,
    ) -> None:
        self.set_anchor_point(self.selected_anchor, pt)
        self.update_anchor_fields()
        self.draw_component()
        if refresh_tree:
            self.populate_animation_tree()
        if refresh_preview:
            self.refresh_preview(force=final)

    def draw_component(self) -> None:
        path = self.paths.slices / f"{self.selected_sprite}.png"
        if not path.exists():
            return
        img = Image.open(path).convert("RGBA")
        base = core.composite_bg(img, self.background)
        # Draw in native image coordinates; the QGraphicsView handles zooming.
        d = ImageDraw.Draw(base)
        for name in self.anchor_names():
            x, y = self.get_anchor_point(name)
            col = core.color_for_name(name)
            selected = name == self.selected_anchor
            r = 7 if selected else 4
            line = 14 if selected else 9
            d.line((x - line, y, x + line, y), fill=(*col, 255), width=2)
            d.line((x, y - line, x, y + line), fill=(*col, 255), width=2)
            d.ellipse(
                (x - r, y - r, x + r, y + r), outline=(255, 255, 255, 255), width=2
            )
            label = f" {name}"
            d.text((x + 8, y - 10), label, fill=(255, 255, 255, 255))
        fit = not getattr(self.component_view, "_has_user_zoom", False)
        self.component_view.set_pillow_image(base, fit=fit, preserve_view=not fit)

    def on_component_click(self, x: float, y: float) -> None:
        pt = self._normalize_anchor_point(x, y)
        self._apply_anchor_point(pt)
        self.status.showMessage(
            f"Placed {self.selected_sprite}.{self.selected_anchor} at ({pt[0]:.2f}, {pt[1]:.2f})"
        )

    def on_component_drag(self, x: float, y: float, final: bool) -> None:
        pt = self._normalize_anchor_point(x, y)
        self._apply_anchor_point(
            pt, refresh_tree=final, refresh_preview=final, final=final
        )
        suffix = " committed" if final else ""
        self.status.showMessage(
            f"Dragging {self.selected_sprite}.{self.selected_anchor} -> ({pt[0]:.2f}, {pt[1]:.2f}){suffix}"
        )

    def on_component_hover(self, x: float, y: float) -> None:
        self.status.showMessage(
            f"Anchor canvas {self.selected_sprite}: ({x:.2f}, {y:.2f})"
        )

    def nudge_anchor(self, dx: float, dy: float) -> None:
        x, y = self.get_anchor_point(self.selected_anchor)
        pt = self._normalize_anchor_point(x + float(dx), y + float(dy))
        self._apply_anchor_point(
            pt, refresh_tree=True, refresh_preview=True, final=True
        )
        self.status.showMessage(
            f"Nudged {self.selected_sprite}.{self.selected_anchor} -> ({pt[0]:.2f}, {pt[1]:.2f})"
        )

    def apply_anchor_xy(self) -> None:
        pt = self._normalize_anchor_point(
            float(self.anchor_x.value()), float(self.anchor_y.value())
        )
        self._apply_anchor_point(
            pt, refresh_tree=True, refresh_preview=True, final=True
        )

    def pivot_follows_selected(self) -> None:
        if self.selected_anchor == "pivot":
            return
        keep_anchor = self.selected_anchor
        s = self.sprites[self.selected_sprite]
        s["pivot_anchor"] = keep_anchor
        s["pivot"] = core.point_list(self.get_anchor_point(keep_anchor))
        self.dirty_meta = True
        self.selected_anchor = keep_anchor
        self.populate_anchor_list()
        self.selected_anchor = keep_anchor
        matches = self.anchor_list.findItems(
            keep_anchor, self.QtCore.Qt.MatchFlag.MatchExactly
        )
        if matches:
            self.anchor_list.blockSignals(True)
            self.anchor_list.setCurrentItem(matches[0])
            self.anchor_list.blockSignals(False)
        self.update_anchor_fields()
        self.draw_component()
        self.refresh_preview()
        self.status.showMessage(
            f"Pivot now follows {self.selected_sprite}.{keep_anchor}; anchor selection preserved"
        )

    # Pose editing ------------------------------------------------------
    def current_frame_override(self) -> Dict[str, Any]:
        return self.pose_model.frame(
            self.pose_anim.text(), int(self.pose_frame.value())
        )

    def raw_frame_overrides(self, anim: str) -> Dict[str, Any]:
        adata = self.pose_model.anim(anim)
        frames = adata.get("frame_overrides")
        if frames is None and isinstance(adata.get("frames"), dict):
            frames = adata.get("frames")
        if frames is None:
            frames = {}
            adata["frame_overrides"] = frames
        return frames

    def has_current_frame_keyframe(self) -> bool:
        frames = self.raw_frame_overrides(self.pose_anim.text())
        idx = str(int(self.pose_frame.value()))
        return idx in frames and bool(frames.get(idx))

    def current_effective_override(self) -> Dict[str, Any]:
        return self.rig.interpolated_frame_overrides(
            self.pose_model.clean_for_save(),
            self.pose_anim.text(),
            int(self.pose_frame.value()),
        )

    def scale_field_for_role(self, role: str) -> Optional[str]:
        return getattr(core, "ROLE_TO_SCALE_FIELD", {}).get(role)

    def scale_value_for_role(
        self, role: str, comp: Mapping[str, Any], fr: Mapping[str, Any]
    ) -> float:
        field = self.scale_field_for_role(role)
        if field and field in fr:
            return float(fr[field])
        # Do not reverse-engineer editable scale from the rendered manifest.
        # Arm/leg components may be endpoint-solved, so comp["scale"] can be an
        # effective constraint result.  Showing that as an editable value caused
        # the GUI to write solved scales back into unrelated frame edits.
        defaults = self.pose_model.clean_for_save().get("defaults") or {}
        if field and field in defaults:
            return float(defaults[field])
        anim_defaults = (
            (self.pose_model.clean_for_save().get("animations") or {}).get(
                self.pose_anim.text()
            )
            or {}
        ).get("defaults") or {}
        if field and field in anim_defaults:
            return float(anim_defaults[field])
        return 1.0

    def update_key_status(self) -> None:
        if self.has_current_frame_keyframe():
            self.key_status.setText("keyframe: explicit")
        else:
            eff = self.current_effective_override()
            self.key_status.setText(
                "keyframe: inherited/interpolated" if eff else "keyframe: procedural"
            )

    def clear_current_frame_override(self) -> None:
        frames = self.raw_frame_overrides(self.pose_anim.text())
        idx = str(int(self.pose_frame.value()))
        if idx in frames:
            frames.pop(idx, None)
            self.dirty_pose = True
        self.update_key_status()
        self.populate_z_order()
        self.populate_animation_tree()
        self.reload_pose_controls_for_current_frame()
        self.refresh_preview(force=True)

    def load_pose_fields_from_model(
        self, role: str, anim: str, idx: int, comp: Mapping[str, Any]
    ) -> None:
        fr = self.rig.interpolated_frame_overrides(
            self.pose_model.clean_for_save(), anim, idx
        )
        angle_field = core.ROLE_TO_ANGLE_FIELD.get(role)
        sprite_field = core.ROLE_TO_SPRITE_FIELD.get(role)
        delta_field = core.ROLE_TO_DELTA_FIELD.get(role)
        self.pose_art.blockSignals(True)
        self.pose_angle.blockSignals(True)
        self.pose_scale.blockSignals(True)
        self.pose_dx.blockSignals(True)
        self.pose_dy.blockSignals(True)
        if sprite_field:
            self.pose_art.setCurrentText(
                str(fr.get(sprite_field, comp.get("sprite", "")))
            )
        if angle_field:
            self.pose_angle.setValue(float(fr.get(angle_field, comp.get("angle", 0.0))))
        scale_field = self.scale_field_for_role(role)
        if scale_field:
            self.pose_scale.setValue(self.scale_value_for_role(role, comp, fr))
            self.pose_scale.setEnabled(True)
        else:
            self.pose_scale.setValue(1.0)
            self.pose_scale.setEnabled(False)
        if delta_field:
            default_delta = fr.get(delta_field)
            if default_delta is None:
                endpoint = comp.get("endpoint")
                target = comp.get("target")
                if endpoint and target:
                    default_delta = [
                        float(endpoint[0]) - float(target[0]),
                        float(endpoint[1]) - float(target[1]),
                    ]
            dx, dy = core.point(default_delta, (0.0, 0.0))
            self.pose_dx.setValue(dx)
            self.pose_dy.setValue(dy)
        self.pose_art.blockSignals(False)
        self.pose_angle.blockSignals(False)
        self.pose_scale.blockSignals(False)
        self.pose_dx.blockSignals(False)
        self.pose_dy.blockSignals(False)
        self._loaded_pose_fields = {
            "sprite": self.pose_art.currentText(),
            "angle": float(self.pose_angle.value()),
            "scale": float(self.pose_scale.value()),
            "delta": [float(self.pose_dx.value()), float(self.pose_dy.value())],
        }
        self.update_anim_fields()
        self.update_key_status()

    def reload_pose_controls_for_current_frame(self) -> None:
        if self.selected_instance is None:
            self.update_key_status()
            return
        role = self.pose_role.text() or str(self.selected_instance.get("role", ""))
        anim = self.pose_anim.text()
        idx = int(self.pose_frame.value())
        comp = self.find_component(anim, idx, role) or self.selected_instance
        self.load_pose_fields_from_model(role, anim, idx, comp)

    def defer_pose_edit(self, changed_fields: Optional[set[str]] = None) -> None:
        if self.selected_instance is None or self._refreshing:
            return
        self._preview_timer.start(80)
        self.apply_pose_edit(refresh=False, changed_fields=changed_fields or set())

    def apply_pose_edit(
        self, *, refresh: bool = True, changed_fields: Optional[set[str]] = None
    ) -> None:
        if self.selected_instance is None:
            return
        role = self.pose_role.text()
        fr = self.current_frame_override()
        sprite_field = core.ROLE_TO_SPRITE_FIELD.get(role)
        angle_field = core.ROLE_TO_ANGLE_FIELD.get(role)
        delta_field = core.ROLE_TO_DELTA_FIELD.get(role)
        scale_field = self.scale_field_for_role(role)
        write_all = changed_fields is None
        changed_fields = changed_fields or set()
        if (
            sprite_field
            and self.pose_art.currentText()
            and (write_all or "sprite" in changed_fields)
        ):
            fr[sprite_field] = self.pose_art.currentText()
        if angle_field and (write_all or "angle" in changed_fields):
            fr[angle_field] = float(self.pose_angle.value())
        if delta_field and (write_all or "delta" in changed_fields):
            fr[delta_field] = [float(self.pose_dx.value()), float(self.pose_dy.value())]
        if scale_field and (write_all or "scale" in changed_fields):
            # Scale edits are exact-frame overrides.  Broad scale calibration
            # belongs in top-level or animation defaults in the YAML.
            fr[scale_field] = float(self.pose_scale.value())
        self.dirty_pose = True
        self.update_key_status()
        self.render_animation_frame()
        if refresh:
            self.populate_animation_tree()
            self.refresh_preview()

    def update_anim_fields(self) -> None:
        info = self.rig.animation_info(
            self.pose_anim.text(), self.pose_model.clean_for_save()
        )
        self.anim_frame_count.blockSignals(True)
        self.anim_duration_ms.blockSignals(True)
        self.anim_frame_count.setValue(int(info["frames"]))
        self.anim_duration_ms.setValue(int(info["duration_ms"]))
        self.anim_frame_count.blockSignals(False)
        self.anim_duration_ms.blockSignals(False)
        self.sync_timeline()
        self.populate_z_order()
        self.update_connection_table()

    def sync_timeline(self) -> None:
        if not hasattr(self, "frame_slider"):
            return
        info = self.rig.animation_info(
            self.pose_anim.text(), self.pose_model.clean_for_save()
        )
        max_idx = max(0, int(info["frames"]) - 1)
        idx = max(0, min(int(self.pose_frame.value()), max_idx))
        self._updating_timeline = True
        try:
            self.frame_slider.setRange(0, max_idx)
            self.frame_slider.setValue(idx)
            if self.pose_frame.value() != idx:
                self.pose_frame.setValue(idx)
            self.frame_label.setText(f"frame {idx + 1} / {max_idx + 1}")
        finally:
            self._updating_timeline = False
        self._play_index = idx

    def set_current_frame(self, idx: int, *, refresh_sheet: bool = True) -> None:
        info = self.rig.animation_info(
            self.pose_anim.text(), self.pose_model.clean_for_save()
        )
        max_idx = max(0, int(info["frames"]) - 1)
        idx = max(0, min(int(idx), max_idx))
        self._updating_timeline = True
        try:
            self.pose_frame.setValue(idx)
            self.frame_slider.setValue(idx)
            self.frame_label.setText(f"frame {idx + 1} / {max_idx + 1}")
        finally:
            self._updating_timeline = False
        self._play_index = idx
        if self.selected_instance is not None:
            self.selected_instance["animation"] = self.pose_anim.text()
            self.selected_instance["frame_index"] = idx
        self.reload_pose_controls_for_current_frame()
        self.populate_z_order()
        self.update_connection_table()
        self.render_animation_frame()
        if refresh_sheet:
            self.refresh_preview()

    def on_pose_frame_changed(self) -> None:
        if self._updating_timeline or self._refreshing:
            return
        self.set_current_frame(int(self.pose_frame.value()), refresh_sheet=True)

    def on_timeline_frame_changed(self, idx: int) -> None:
        if self._updating_timeline:
            return
        self.set_current_frame(int(idx), refresh_sheet=True)

    def step_frame(self, delta: int) -> None:
        self.set_current_frame(
            int(self.pose_frame.value()) + int(delta), refresh_sheet=True
        )

    def apply_anim_settings(self) -> None:
        if self._refreshing:
            return
        adata = self.pose_model.anim(self.pose_anim.text())
        adata["frames"] = int(self.anim_frame_count.value())
        adata["duration_ms"] = int(self.anim_duration_ms.value())
        self.dirty_pose = True
        self.sync_timeline()
        self.populate_animation_tree()
        self.refresh_preview()

    def add_frame(self) -> None:
        self.anim_frame_count.setValue(self.anim_frame_count.value() + 1)
        self.apply_anim_settings()

    def remove_frame(self) -> None:
        self.anim_frame_count.setValue(max(1, self.anim_frame_count.value() - 1))
        self.apply_anim_settings()

    def populate_z_order(self) -> None:
        self._updating_z_list = True
        self.z_list.blockSignals(True)
        self.z_list.clear()
        effective = self.current_effective_override()
        order = (
            effective.get("z_order")
            or self.current_frame_override().get("z_order")
            or list(core.DEFAULT_Z_ORDER)
        )
        hidden = {str(v) for v in (effective.get("hidden_parts") or [])}
        visible_parts = effective.get("visible_parts")
        visible = None if visible_parts is None else {str(v) for v in visible_parts}
        for role in order:
            item = self.QtWidgets.QListWidgetItem(str(role))
            item.setFlags(item.flags() | self.QtCore.Qt.ItemFlag.ItemIsUserCheckable)
            checked = str(role) not in hidden and (
                visible is None or str(role) in visible
            )
            item.setCheckState(
                self.QtCore.Qt.CheckState.Checked
                if checked
                else self.QtCore.Qt.CheckState.Unchecked
            )
            self.z_list.addItem(item)
        self.z_list.blockSignals(False)
        self._updating_z_list = False
        self.update_key_status()

    def z_roles(self) -> List[str]:
        return [self.z_list.item(i).text() for i in range(self.z_list.count())]

    def hidden_roles_from_z_list(self) -> List[str]:
        hidden: List[str] = []
        for i in range(self.z_list.count()):
            item = self.z_list.item(i)
            if item.checkState() != self.QtCore.Qt.CheckState.Checked:
                hidden.append(item.text())
        return hidden

    def apply_z_state_from_widget(self) -> None:
        fr = self.current_frame_override()
        fr["z_order"] = self.z_roles()
        hidden = self.hidden_roles_from_z_list()
        fr.pop("visible_parts", None)
        if hidden:
            fr["hidden_parts"] = hidden
        else:
            fr.pop("hidden_parts", None)
        self.dirty_pose = True
        self.update_key_status()
        self.refresh_preview()

    def on_z_item_changed(self, item) -> None:
        if self._updating_z_list or self._refreshing:
            return
        self.apply_z_state_from_widget()

    def move_z(self, delta: int) -> None:
        row = self.z_list.currentRow()
        if row < 0:
            return
        new = max(0, min(self.z_list.count() - 1, row + delta))
        if new == row:
            return
        self._updating_z_list = True
        try:
            item = self.z_list.takeItem(row)
            self.z_list.insertItem(new, item)
            self.z_list.setCurrentRow(new)
        finally:
            self._updating_z_list = False
        self.apply_z_state_from_widget()

    def reset_z_order(self) -> None:
        fr = self.current_frame_override()
        fr.pop("z_order", None)
        self.dirty_pose = True
        self.populate_z_order()
        self.refresh_preview()

    def show_all_parts(self) -> None:
        fr = self.current_frame_override()
        fr.pop("hidden_parts", None)
        fr.pop("visible_parts", None)
        self.dirty_pose = True
        self.populate_z_order()
        self.refresh_preview()

    def show_only_selected_part(self) -> None:
        role = self.pose_role.text() or (self.selected_instance or {}).get("role")
        if not role:
            self.status.showMessage("Select a logical part before using Only selected")
            return
        roles = self.z_roles() or list(core.DEFAULT_Z_ORDER)
        self.current_frame_override()["hidden_parts"] = [r for r in roles if r != role]
        self.current_frame_override().pop("visible_parts", None)
        self.dirty_pose = True
        self.populate_z_order()
        self.refresh_preview()

    def find_frame_components(
        self, anim: str, idx: int
    ) -> Optional[List[Dict[str, Any]]]:
        adata = (self.current_manifest.get("animations") or {}).get(anim) or {}
        for frame in adata.get("frames", []):
            if int(frame.get("index", -1)) == int(idx):
                return (frame.get("pose") or {}).get("components", [])
        return None

    def navigate_connected_part(self) -> None:
        anim = self.pose_anim.text()
        idx = int(self.pose_frame.value())
        anchor = self.selected_anchor
        for comp in self.find_frame_components(anim, idx) or []:
            conn = comp.get("connects_to") or {}
            if (
                conn.get("sprite") == self.selected_sprite
                and conn.get("anchor") == anchor
            ):
                role = str(comp.get("role") or "")
                if role and self.select_rig_role(
                    role, anim=anim, idx=idx, comp=comp, sync_tree=True, refresh=True
                ):
                    self.status.showMessage(
                        f"Navigated {self.selected_sprite}.{anchor} -> {role} ({comp.get('sprite')})"
                    )
                    return
        self.status.showMessage(
            f"No component in {anim}[{idx}] is connected to {self.selected_sprite}.{anchor}"
        )

    # Connection / constraint browser -------------------------------------
    def current_connection_components(self) -> List[Dict[str, Any]]:
        return list(
            self.find_frame_components(
                self.pose_anim.text(), int(self.pose_frame.value())
            )
            or []
        )

    def selected_connection_component(self) -> Optional[Dict[str, Any]]:
        if not hasattr(self, "connection_table"):
            return None
        row = self.connection_table.currentRow()
        if row < 0:
            return None
        item = self.connection_table.item(row, 0)
        if item is None:
            return None
        role = item.data(self.QtCore.Qt.ItemDataRole.UserRole) or item.text()
        return self.find_component(
            self.pose_anim.text(), int(self.pose_frame.value()), str(role)
        )

    def update_connection_table(self) -> None:
        if not hasattr(self, "connection_table"):
            return
        comps = self.current_connection_components()
        self.connection_table.blockSignals(True)
        self.connection_table.setRowCount(0)
        selected_role = (self.selected_instance or {}).get("role")
        for comp in comps:
            conn = comp.get("connects_to") or {}
            if not conn:
                continue
            row = self.connection_table.rowCount()
            self.connection_table.insertRow(row)
            role = str(comp.get("role", ""))
            child_anchor = str(comp.get("anchor") or "")
            child_sprite = str(comp.get("sprite") or "")
            parent_role = str(conn.get("role") or "")
            parent_sprite = str(conn.get("sprite") or "")
            parent_anchor = str(conn.get("anchor") or "")
            snap = comp.get("snap_error_px")
            values = [
                role,
                str(comp.get("joint") or ""),
                f"{child_sprite}.{child_anchor}",
                f"{parent_role}.{parent_sprite + '.' if parent_sprite else ''}{parent_anchor}",
                "" if snap is None else f"{float(snap):.3f}",
                "yes" if comp.get("visible", True) else "no",
            ]
            for col, value in enumerate(values):
                cell = self.QtWidgets.QTableWidgetItem(value)
                cell.setData(self.QtCore.Qt.ItemDataRole.UserRole, role)
                if col == 4 and snap is not None and float(snap) > 0.5:
                    cell.setToolTip(
                        "Non-zero snap error means the child anchor did not land on the parent socket."
                    )
                self.connection_table.setItem(row, col, cell)
            if role == selected_role:
                self.connection_table.selectRow(row)
        self.connection_table.resizeColumnsToContents()
        self.connection_table.blockSignals(False)

    def on_connection_select(self) -> None:
        comp = self.selected_connection_component()
        if not comp:
            return
        conn = comp.get("connects_to") or {}
        msg = (
            f"{comp.get('role')}: {comp.get('sprite')}.{comp.get('anchor')} "
            f"snaps to {conn.get('role')}.{conn.get('sprite') or ''}.{conn.get('anchor')} "
            f"err={comp.get('snap_error_px')} px"
        )
        self.status.showMessage(msg)

    def _iter_tree_items(self):
        """Yield all tree items without relying on text matching.

        The rendered role name is display text, but the editor contract should be
        based on stable item data.  Direct-manipulation hit tests operate on the
        renderer manifest, so the tree may be filtered, stale, or temporarily
        not expanded; selection must therefore not depend on text search alone.
        """
        stack = [
            self.tree.topLevelItem(i) for i in range(self.tree.topLevelItemCount())
        ]
        while stack:
            item = stack.pop(0)
            yield item
            for j in range(item.childCount()):
                stack.append(item.child(j))

    def _select_tree_part(self, role: str, *, emit: bool = True) -> bool:
        anim = self.pose_anim.text()
        idx = int(self.pose_frame.value())
        for item in self._iter_tree_items():
            data = item.data(0, self.QtCore.Qt.ItemDataRole.UserRole) or {}
            try:
                frame_ok = int(data.get("frame", -9999)) == idx
            except Exception:
                frame_ok = False
            if (
                data.get("kind") == "part"
                and data.get("animation") == anim
                and frame_ok
                and data.get("role") == role
            ):
                self.tree.blockSignals(not emit)
                try:
                    self.tree.setCurrentItem(item)
                    parent = item.parent()
                    while parent is not None:
                        parent.setExpanded(True)
                        parent = parent.parent()
                finally:
                    self.tree.blockSignals(False)
                return True
        return False

    def _component_for_role(
        self,
        anim: str,
        idx: int,
        role: str,
        fallback: Optional[Mapping[str, Any]] = None,
    ) -> Dict[str, Any]:
        comp = self.find_component(anim, idx, role)
        if comp:
            return dict(comp)
        # The action preview is the authoritative thing the user clicked.  Use
        # its current frame manifest even when the full spritesheet/tree manifest
        # is stale or filtered.
        cur = self._component_by_role_from_animation_manifest(role)
        if cur:
            return dict(cur)
        if fallback:
            return dict(fallback)
        return {"role": role, "sprite": ""}

    def select_rig_role(
        self,
        role: str,
        *,
        anim: Optional[str] = None,
        idx: Optional[int] = None,
        comp: Optional[Mapping[str, Any]] = None,
        sync_tree: bool = True,
        refresh: bool = True,
    ) -> bool:
        """Select a logical rig role as the editing target.

        This is the central selection path for the professional/editor-style
        viewport.  It intentionally does not require a matching QTreeWidgetItem;
        the tree is just one view of the rig, while the canonical target is the
        logical role in the current animation frame.
        """
        role = str(role or "")
        if not role:
            return False
        anim = (
            anim
            or self.pose_anim.text()
            or (self.job.animations[0] if self.job.animations else "run")
        )
        if idx is None:
            idx = int(getattr(self, "_play_index", int(self.pose_frame.value())))
        idx = int(idx)

        # Keep timeline widgets in sync without triggering a recursive selection
        # or full preview rebuild before the selected role exists.
        self.pose_anim.blockSignals(True)
        self.pose_frame.blockSignals(True)
        try:
            self.pose_anim.setText(anim)
            self.pose_frame.setValue(idx)
        finally:
            self.pose_frame.blockSignals(False)
            self.pose_anim.blockSignals(False)
        self._play_index = idx

        comp_dict = self._component_for_role(anim, idx, role, comp)
        self.selected_instance = {
            "animation": anim,
            "frame_index": idx,
            "role": role,
            "sprite": comp_dict.get("sprite"),
        }
        self.pose_role.blockSignals(True)
        self.pose_art.blockSignals(True)
        try:
            self.pose_role.setText(role)
            if comp_dict.get("sprite"):
                self.pose_art.setCurrentText(str(comp_dict.get("sprite")))
        finally:
            self.pose_art.blockSignals(False)
            self.pose_role.blockSignals(False)
        self.load_pose_fields_from_model(role, anim, idx, comp_dict)
        base = str(comp_dict.get("sprite", "")).split("@")[0]
        if base in self.sprites:
            self.select_sprite(base)
        self.populate_z_order()
        self.update_connection_table()
        tree_ok = self._select_tree_part(role, emit=False) if sync_tree else False
        if refresh:
            self.render_animation_frame()
        if not tree_ok and sync_tree:
            self.status.showMessage(
                f"Selected {role} from viewport; tree has no current-frame item, so using rig-node selection"
            )
        return True

    def select_connection_child_part(self) -> None:
        comp = self.selected_connection_component()
        if not comp:
            return
        role = str(comp.get("role"))
        if not self.select_rig_role(role, comp=comp, sync_tree=True, refresh=True):
            self.status.showMessage(f"Could not select child part {role}")

    def select_connection_parent_part(self) -> None:
        comp = self.selected_connection_component()
        if not comp:
            return
        conn = comp.get("connects_to") or {}
        role = str(conn.get("role") or "")
        if (
            role
            and role != "root"
            and self.select_rig_role(
                role,
                comp=self.find_component(
                    self.pose_anim.text(), int(self.pose_frame.value()), role
                ),
                sync_tree=True,
                refresh=True,
            )
        ):
            return
        self.status.showMessage(
            f"Parent {role or '<none>'} is not a selectable rendered part in this frame"
        )

    def select_connection_child_anchor(self) -> None:
        comp = self.selected_connection_component()
        if not comp:
            return
        sprite = str(comp.get("sprite") or "").split("@")[0]
        anchor = str(comp.get("anchor") or "pivot")
        if sprite in self.sprites:
            self.select_sprite(sprite)
            if anchor in self.anchor_names():
                self.selected_anchor = anchor
                self.populate_anchor_list()
                self.draw_component()
            self.status.showMessage(f"Selected child anchor {sprite}.{anchor}")

    def select_connection_parent_anchor(self) -> None:
        comp = self.selected_connection_component()
        if not comp:
            return
        conn = comp.get("connects_to") or {}
        sprite = str(conn.get("sprite") or "").split("@")[0]
        anchor = str(conn.get("anchor") or "pivot")
        if sprite in self.sprites:
            self.select_sprite(sprite)
            if anchor in self.anchor_names():
                self.selected_anchor = anchor
                self.populate_anchor_list()
                self.draw_component()
            self.status.showMessage(f"Selected parent socket {sprite}.{anchor}")
        else:
            self.status.showMessage(
                f"Parent socket {conn.get('role')}.{anchor} has no editable art sprite"
            )

    def select_frame_tree_item(self, anim: str, idx: int) -> bool:
        for i in range(self.tree.topLevelItemCount()):
            anim_item = self.tree.topLevelItem(i)
            data = anim_item.data(0, self.QtCore.Qt.ItemDataRole.UserRole) or {}
            if data.get("kind") != "anim" or data.get("animation") != anim:
                continue
            anim_item.setExpanded(True)
            for j in range(anim_item.childCount()):
                frame_item = anim_item.child(j)
                fdata = frame_item.data(0, self.QtCore.Qt.ItemDataRole.UserRole) or {}
                if fdata.get("kind") == "frame" and int(fdata.get("frame", -1)) == int(
                    idx
                ):
                    self.tree.setCurrentItem(frame_item)
                    return True
        return False

    def on_spritesheet_preview_click(self, x: float, y: float) -> None:
        for anim, adata in (self.current_manifest.get("animations") or {}).items():
            for frame in adata.get("frames", []):
                fx, fy = float(frame.get("x", 0)), float(frame.get("y", 0))
                fw, fh = float(frame.get("w", 0)), float(frame.get("h", 0))
                if fx <= x < fx + fw and fy <= y < fy + fh:
                    idx = int(frame.get("index", 0))
                    self.pose_anim.setText(anim)
                    self.set_current_frame(idx, refresh_sheet=False)
                    self.select_frame_tree_item(anim, idx)
                    self.status.showMessage(
                        f"Selected {anim} frame {idx} from spritesheet preview"
                    )
                    return
        self.status.showMessage(
            f"No rendered frame at spritesheet click ({x:.1f}, {y:.1f})"
        )

    def hit_test_action_part(self, x: float, y: float) -> Optional[Dict[str, Any]]:
        manifest = getattr(self, "current_animation_manifest", {}) or {}
        comps = list(((manifest.get("pose") or {}).get("components") or []))
        comps.sort(key=lambda c: int(c.get("z_index", 0)), reverse=True)
        for comp in comps:
            if not comp.get("visible", True):
                continue
            bounds = comp.get("bounds")
            if not bounds or len(bounds) != 4:
                continue
            x1, y1, x2, y2 = [float(v) for v in bounds]
            if x1 <= x <= x2 and y1 <= y <= y2:
                return comp
        return None

    def on_action_preview_drag_start(self, x: float, y: float) -> None:
        comp = self.hit_test_action_part(x, y)
        self._direct_drag = None
        if not comp:
            return
        role = str(comp.get("role") or "")
        if not role or not self.select_rig_role(
            role,
            anim=self.pose_anim.text(),
            idx=int(getattr(self, "_play_index", self.pose_frame.value())),
            comp=comp,
            sync_tree=True,
            refresh=False,
        ):
            return
        # For constrained limb bones, use the distal joint as the manipulator
        # target.  The proximal joint remains snapped to its parent socket, so
        # drag behaves like a simple one-bone IK handle instead of raw x/y art
        # translation.
        handle = (
            comp.get("endpoint_anchor_world")
            or comp.get("endpoint")
            or comp.get("target")
            or comp.get("parent_target")
        )
        center = comp.get("parent_target") or comp.get("target") or handle
        if not center:
            b = comp.get("bounds") or [x, y, x, y]
            center = [
                (float(b[0]) + float(b[2])) / 2.0,
                (float(b[1]) + float(b[3])) / 2.0,
            ]
        if not handle:
            handle = center
        if self.fast_drag_preview.isChecked():
            self._begin_fast_drag_renderer()
        self._direct_drag = {
            "role": role,
            "start": (float(x), float(y)),
            "center": (float(center[0]), float(center[1])),
            "start_handle": (float(handle[0]), float(handle[1])),
            "parent_target": tuple(map(float, (comp.get("parent_target") or center))),
            "start_dx": float(self.pose_dx.value()),
            "start_dy": float(self.pose_dy.value()),
            "start_angle": float(self.pose_angle.value()),
            "start_mouse_angle": math.degrees(
                math.atan2(float(y) - float(center[1]), float(x) - float(center[0]))
            ),
        }
        self.status.showMessage(
            f"Grabbed {role}; drag uses bone constraints, Shift-drag rotates"
        )

    def on_action_preview_drag_move(self, x: float, y: float, final: bool) -> None:
        drag = getattr(self, "_direct_drag", None)
        if not drag:
            return
        role = drag["role"]
        mods = self.QtWidgets.QApplication.keyboardModifiers()
        shift = bool(mods & self.QtCore.Qt.KeyboardModifier.ShiftModifier)
        if shift:
            cx, cy = drag["center"]
            cur = math.degrees(math.atan2(float(y) - cy, float(x) - cx))
            delta = cur - float(drag["start_mouse_angle"])
            while delta > 180:
                delta -= 360
            while delta < -180:
                delta += 360
            self._pose_spin_set_angle_silent(float(drag["start_angle"]) + delta)
            self.apply_pose_edit(refresh=False, changed_fields={"angle"})
        else:
            mdx = float(x) - float(drag["start"][0])
            mdy = float(y) - float(drag["start"][1])
            field = core.ROLE_TO_DELTA_FIELD.get(role)
            if not field:
                self.status.showMessage(
                    f"{role} has no editable drag offset yet; Shift-drag can still rotate it"
                )
                return
            if role in {"front_arm", "back_arm", "front_leg", "back_leg"}:
                hx, hy = drag["start_handle"]
                px, py = drag["parent_target"]
                desired = (hx + mdx, hy + mdy)
                local = self._world_to_torso_local_delta(
                    (desired[0] - px, desired[1] - py)
                )
                self._pose_spin_set_delta_silent(local[0], local[1])
            else:
                self._pose_spin_set_delta_silent(
                    float(drag["start_dx"]) + mdx, float(drag["start_dy"]) + mdy
                )
            self.apply_pose_edit(refresh=False, changed_fields={"delta"})
        # Drag path: update only the action preview + bone overlay.  Full
        # spritesheet, tree, and connection table refresh happens on release.
        self.render_animation_frame()
        if final:
            self._direct_drag = None
            self._end_fast_drag_renderer()
            self.populate_animation_tree()
            if self.live_sheet_preview.isChecked():
                self.refresh_preview(force=True)
            else:
                self.render_animation_frame()

    def on_action_preview_click(self, x: float, y: float) -> None:
        comp = self.hit_test_action_part(x, y)
        if comp:
            role = str(comp.get("role"))
            self.select_rig_role(
                role,
                anim=self.pose_anim.text(),
                idx=int(getattr(self, "_play_index", self.pose_frame.value())),
                comp=comp,
                sync_tree=True,
                refresh=True,
            )
            self.status.showMessage(f"Selected {role} from action preview")
            return
        self.status.showMessage(f"No part at action preview click ({x:.1f}, {y:.1f})")

    # Preview -----------------------------------------------------------
    def preview_animations(self) -> List[str]:
        if self.preview_relevant.isChecked() and self.selected_sprite:
            return core.relevant_animations(
                self.paths.job,
                self.metadata,
                self.pose_model.clean_for_save(),
                self.selected_sprite,
            )
        return list(self.job.animations)

    def _timer_refresh_preview(self) -> None:
        if self.live_sheet_preview.isChecked():
            self.refresh_preview(force=True)
        else:
            self.render_animation_frame()
            self.status.showMessage(
                "Live full-sheet rendering is paused; action preview updated"
            )

    def refresh_preview(self, force: bool = False) -> None:
        if not force:
            # Keep drag/input responsive: the timer coalesces field edits and,
            # when live-sheet is disabled, only the lightweight action preview is
            # refreshed.
            self._preview_timer.start(80)
            return
        self._preview_timer.stop()
        self._refreshing = True
        try:
            if self.live_sheet_preview.isChecked() or force:
                highlight = (
                    dict(self.selected_instance) if self.selected_instance else None
                )
                img, manifest = core.build_preview(
                    self.paths.job,
                    self.metadata,
                    self.pose_model.clean_for_save(),
                    animations=self.preview_animations(),
                    debug=self.preview_debug.isChecked(),
                    highlight=highlight,
                    bg=self.preview_bg.currentText(),
                )
                self.current_manifest = manifest
                self.preview_view.set_pillow_image(
                    img, fit=self.preview_fit.isChecked()
                )
                self.update_connection_table()
                message = f"Preview {img.width}x{img.height}"
            else:
                message = "Action preview only; full sheet is paused"
            self.render_animation_frame()
            self.status.showMessage(
                f"{message}; metadata={'dirty' if self.dirty_meta else 'clean'}, pose={'dirty' if self.dirty_pose else 'clean'}"
            )
        except Exception as ex:
            self.status.showMessage(f"Preview failed: {ex}")
        finally:
            self._refreshing = False

    def _begin_fast_drag_renderer(self) -> None:
        self._end_fast_drag_renderer()
        meta_path = core.write_temp_yaml(self.metadata)
        job = self.rig.RigJob.load(self.paths.job)
        job.metadata = meta_path
        job.pose_overrides = self.paths.pose_overrides
        atlas = self.rig.ComponentAtlas(job.metadata, job.slices)
        asm = self.rig.RobotAssembler(
            atlas, job.render, pose_overrides=self.pose_model.clean_for_save()
        )
        self._fast_drag_meta_path = meta_path
        self._fast_drag_renderer = (job, asm)

    def _end_fast_drag_renderer(self) -> None:
        p = getattr(self, "_fast_drag_meta_path", None)
        if p is not None:
            try:
                p.unlink(missing_ok=True)
            except Exception:
                pass
        self._fast_drag_meta_path = None
        self._fast_drag_renderer = None

    def _draw_bone_overlay(
        self, img: Image.Image, frame_manifest: Mapping[str, Any]
    ) -> Image.Image:
        if not getattr(self, "show_bones", None) or not self.show_bones.isChecked():
            return img
        out = img.copy()
        draw = ImageDraw.Draw(out, "RGBA")
        comps = (frame_manifest.get("pose") or {}).get("components") or []
        selected = self.pose_role.text() if hasattr(self, "pose_role") else ""
        for comp in comps:
            if not comp.get("visible", True):
                continue
            role = str(comp.get("role") or "")
            p0 = (
                comp.get("parent_target")
                or comp.get("target")
                or comp.get("child_anchor_world")
            )
            p1 = (
                comp.get("endpoint_anchor_world")
                or comp.get("endpoint")
                or comp.get("child_anchor_world")
                or comp.get("target")
            )
            if not p0 or not p1:
                continue
            x0, y0 = float(p0[0]), float(p0[1])
            x1, y1 = float(p1[0]), float(p1[1])
            strong = role == selected
            color = (255, 245, 80, 245) if strong else (0, 235, 255, 185)
            width = 4 if strong else 2
            draw.line((x0, y0, x1, y1), fill=color, width=width)
            r0 = 5 if strong else 3
            r1 = 7 if strong else 4
            draw.ellipse(
                (x0 - r0, y0 - r0, x0 + r0, y0 + r0),
                fill=(255, 80, 80, 235),
                outline=(255, 255, 255, 220),
                width=1,
            )
            draw.ellipse(
                (x1 - r1, y1 - r1, x1 + r1, y1 + r1),
                fill=(80, 255, 120, 235),
                outline=(255, 255, 255, 220),
                width=1,
            )
        return out

    def _pose_spin_set_delta_silent(self, dx: float, dy: float) -> None:
        self.pose_dx.blockSignals(True)
        self.pose_dy.blockSignals(True)
        self.pose_dx.setValue(float(dx))
        self.pose_dy.setValue(float(dy))
        self.pose_dx.blockSignals(False)
        self.pose_dy.blockSignals(False)

    def _pose_spin_set_angle_silent(self, angle: float) -> None:
        self.pose_angle.blockSignals(True)
        self.pose_angle.setValue(float(angle))
        self.pose_angle.blockSignals(False)

    def _component_by_role_from_animation_manifest(
        self, role: str
    ) -> Optional[Dict[str, Any]]:
        comps = (
            (getattr(self, "current_animation_manifest", {}) or {}).get("pose") or {}
        ).get("components") or []
        for comp in comps:
            if comp.get("role") == role:
                return comp
        return None

    def _world_to_torso_local_delta(self, vec: Point) -> Point:
        torso = self._component_by_role_from_animation_manifest("torso") or {}
        torso_angle = float(torso.get("angle", 0.0))
        return self.rig.rotate_vec(float(vec[0]), float(vec[1]), -torso_angle)

    def render_animation_frame(self) -> None:
        anim = self.pose_anim.text() or (
            self.job.animations[0] if self.job.animations else "run"
        )
        info = self.rig.animation_info(anim, self.pose_model.clean_for_save())
        idx = self._play_index % max(1, int(info["frames"]))
        try:
            if self._fast_drag_renderer is not None:
                _job, asm = self._fast_drag_renderer
                asm.pose_overrides = self.pose_model.clean_for_save()
                img, frame_manifest = asm.render_frame(
                    anim, idx, debug_parts=self.preview_debug.isChecked()
                )
                self.current_animation_manifest = frame_manifest
            else:
                meta_path = core.write_temp_yaml(self.metadata)
                pose_path = core.write_temp_yaml(self.pose_model.clean_for_save())
                try:
                    job = self.rig.RigJob.load(self.paths.job)
                    job.metadata = meta_path
                    job.pose_overrides = pose_path
                    atlas = self.rig.ComponentAtlas(job.metadata, job.slices)
                    asm = self.rig.RobotAssembler(
                        atlas,
                        job.render,
                        pose_overrides=self.rig.load_pose_overrides(job.pose_overrides),
                    )
                    img, frame_manifest = asm.render_frame(
                        anim, idx, debug_parts=self.preview_debug.isChecked()
                    )
                    self.current_animation_manifest = frame_manifest
                finally:
                    meta_path.unlink(missing_ok=True)
                    pose_path.unlink(missing_ok=True)
            show = core.composite_bg(img, self.preview_bg.currentText())
            show = self._draw_bone_overlay(show, self.current_animation_manifest)
            fit = not getattr(self.anim_view, "_has_user_zoom", False)
            self.anim_view.set_pillow_image(show, fit=fit, preserve_view=not fit)
        except Exception as ex:
            canvas = Image.new("RGBA", (640, 240), (0, 0, 0, 255))
            d = ImageDraw.Draw(canvas)
            d.text(
                (10, 10), f"animated preview failed: {ex}", fill=(255, 255, 255, 255)
            )
            self.anim_view.set_pillow_image(canvas, fit=True)

    def toggle_play(self) -> None:
        if self._play_timer.isActive():
            self._play_timer.stop()
            self.play_button.setText("Play")
            return
        info = self.rig.animation_info(
            self.pose_anim.text(), self.pose_model.clean_for_save()
        )
        self._play_timer.start(int(info["duration_ms"]))
        self.play_button.setText("Pause")

    def advance_animation(self) -> None:
        info = self.rig.animation_info(
            self.pose_anim.text(), self.pose_model.clean_for_save()
        )
        frames = max(1, int(info["frames"]))
        self.set_current_frame((self._play_index + 1) % frames, refresh_sheet=False)

    # Save --------------------------------------------------------------
    def save(self) -> None:
        if self.dirty_meta:
            core.backup(self.paths.metadata)
            core.save_yaml(self.paths.metadata, self.metadata)
            self.dirty_meta = False
        if self.dirty_pose:
            core.backup(self.paths.pose_overrides)
            core.save_yaml(self.paths.pose_overrides, self.pose_model.clean_for_save())
            self.dirty_pose = False
        self.status.showMessage(
            f"Saved {self.paths.metadata.name} and {self.paths.pose_overrides.name}"
        )


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        description="PySide6 editor for robot sprite anchors, logical part instances, poses, z-order, and live previews."
    )
    p.add_argument(
        "job",
        type=Path,
        nargs="?",
        default=Path("examples/robot_rig_job.yaml"),
        help="Rig job YAML",
    )
    p.add_argument("--metadata", type=Path, default=None)
    p.add_argument("--slices", type=Path, default=None)
    p.add_argument("--pose-overrides", type=Path, default=None)
    p.add_argument("--zoom", type=int, default=6)
    p.add_argument(
        "--background", choices=["checker", "black", "white"], default="checker"
    )
    p.add_argument(
        "--render-preview",
        type=Path,
        default=None,
        help="Headless render of current preview and exit",
    )
    p.add_argument(
        "--anchor-report",
        type=Path,
        default=None,
        help="Headless JSON report of rendered part instances and exit",
    )
    p.add_argument(
        "--animations",
        nargs="*",
        default=None,
        help="Animations to render in headless preview mode",
    )
    p.add_argument(
        "--debug",
        action="store_true",
        help="Use solid-color debug render in headless preview mode",
    )
    return p


def main(argv: Optional[Sequence[str]] = None) -> int:
    args = build_parser().parse_args(argv)
    paths = load_paths(args)
    if args.anchor_report:
        write_anchor_report(paths, args.anchor_report.resolve())
        print(f"Wrote {args.anchor_report}")
        return 0
    if args.render_preview:
        render_preview(
            paths,
            args.render_preview.resolve(),
            animations=args.animations,
            debug=args.debug,
            background="black",
        )
        print(f"Wrote {args.render_preview}")
        return 0

    try:
        _QtCore, _QtGui, QtWidgets = _require_qt()
    except _QtUnavailable as ex:  # pragma: no cover - environment dependent
        print("ERROR: PySide6 is required for the interactive editor.", file=sys.stderr)
        print("Install it with: pip install PySide6", file=sys.stderr)
        print(f"Import error: {ex}", file=sys.stderr)
        return 2

    app = QtWidgets.QApplication(sys.argv[:1])
    editor = RigPoseEditorQt(paths, zoom=args.zoom, background=args.background)
    editor.window.show()
    return int(app.exec())


if __name__ == "__main__":
    raise SystemExit(main())
