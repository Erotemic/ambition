#!/usr/bin/env python3
"""
PySide6 nested scene-graph editor for the mockingbird boss YAML.

Dependencies:
    python -m pip install pillow pyyaml pyside6

Controls:
- Tree selects a group or shape.
- Press Enter Group to edit the selected group level, similar to double-clicking
  into a grouped object in Google Slides.
- While inside a group, canvas picking prefers visible nodes under the cursor;
  clicking a visible node outside the current group selects that node too.
  Hidden / focused-out nodes remain unpickable.
- z_order is a direct integer spinbox.
- Transform edits update the selected group or shape.
- Primitive YAML edits update selected shape.
- Duplicate copies an entire group subtree and offsets the copy for easy drag.
- Mouse wheel zooms. Ctrl+wheel rotates selected node.
- Double-click empty background selects root and returns to root level.
- Escape moves one level up to the parent group.
- Click selects nodes; toggle Transform Node to move/rotate/scale them on the canvas.
- In Transform Node mode: drag node body to move, corner handles scale, round handle rotates.
"""

from __future__ import annotations

import argparse
import copy
import math
from pathlib import Path

from PIL.ImageQt import ImageQt

try:
    import yaml
except Exception as ex:
    raise SystemExit("Missing dependency: python -m pip install pyyaml") from ex

try:
    from PySide6 import QtCore, QtGui, QtWidgets
except Exception as ex:
    raise SystemExit("Missing dependency: python -m pip install pyside6") from ex

from . import sprite_generator as gen

DATA_DIR = Path(__file__).resolve().parent
DEFAULT_SCENE = DATA_DIR / "mockingbird_boss_scene.yaml"


def iter_nodes(node, path=()):
    yield path, node
    for i, child in enumerate(node.get("children", [])):
        yield from iter_nodes(child, path + (i,))


def get_node(root, path):
    node = root
    for idx in path:
        node = node.setdefault("children", [])[idx]
    return node


def get_parent(root, path):
    if not path:
        return None, None
    return get_node(root, path[:-1]), path[-1]


def subtree_copy(root, path):
    return copy.deepcopy(get_node(root, path))


def prune_to_focus(node, focus_path):
    """Return a copy of the tree that keeps only the focused subtree and its ancestors."""
    node = copy.deepcopy(node)
    if not focus_path:
        return node
    head = focus_path[0]
    kept = []
    for i, child in enumerate(node.get("children", [])):
        if i == head:
            kept.append(prune_to_focus(child, focus_path[1:]))
    node["children"] = kept
    return node


def is_descendant_path(path, ancestor):
    return tuple(path[: len(ancestor)]) == tuple(ancestor)


def default_transform():
    return {"x": 0, "y": 0, "rotation": 0, "scale_x": 1, "scale_y": 1}


def slug(text):
    out = "".join(ch.lower() if ch.isalnum() else "_" for ch in text.strip())
    while "__" in out:
        out = out.replace("__", "_")
    return out.strip("_") or "node"


def collect_ids(root):
    return {node.get("id") for _, node in iter_nodes(root) if node.get("id")}


def unique_id(base, used):
    base = slug(base)
    if base not in used:
        used.add(base)
        return base
    i = 2
    while f"{base}_{i}" in used:
        i += 1
    out = f"{base}_{i}"
    used.add(out)
    return out


def rename_subtree(node, used, suffix="copy"):
    old = node.get("id", "node")
    node["id"] = unique_id(f"{old}_{suffix}", used)
    label = node.get("label") or old
    if not label.lower().endswith(" copy"):
        node["label"] = f"{label} copy"
    for child in node.get("children", []):
        rename_subtree(child, used, suffix=suffix)


def offset_node(node, dx=26, dy=18):
    t = node.setdefault("transform", default_transform())
    t["x"] = float(t.get("x", 0)) + dx
    t["y"] = float(t.get("y", 0)) + dy


def new_group(label="New Group"):
    ident = slug(label)
    return {
        "id": ident,
        "kind": "group",
        "label": label,
        "visible": True,
        "locked": False,
        "z_order": 0,
        "transform": default_transform(),
        "children": [],
    }


def new_shape(label="New Shape"):
    ident = slug(label)
    return {
        "id": ident,
        "kind": "shape",
        "label": label,
        "visible": True,
        "locked": False,
        "z_order": 0,
        "transform": default_transform(),
        "primitive": {
            "type": "rect",
            "box": [-10, -10, 10, 10],
            "fill": "steel_plate",
            "outline": None,
            "width": 1,
        },
    }


class SceneCanvas(QtWidgets.QWidget):
    sceneChanged = QtCore.Signal()
    nodeSelected = QtCore.Signal(tuple)
    editLevelChanged = QtCore.Signal(tuple)
    backgroundDoubleClicked = QtCore.Signal()

    def __init__(self, scene_path: Path):
        super().__init__()
        self.scene_path = Path(scene_path)
        self.scene = gen.Scene.load(self.scene_path)
        self.selected_path = ()
        self.edit_path = ()
        self.focus_path = None
        self.zoom = 1.0
        work = tuple(
            self.scene.render_cfg.get(
                "work_size", self.scene.meta.get("work_size", [900, 640])
            )
        )
        self.work_size = work
        self.view_center = QtCore.QPointF(work[0] / 2, work[1] / 2)
        self.dragging = False
        self.transform_mode = False
        self.drag_mode = None
        self.active_handle = None
        self.last_pos = None
        self.rotate_center_widget = None
        self.rotate_last_angle = None
        self.scale_anchor = None
        self._pixmap = None
        self._bounds_by_id = {}
        self._path_by_id = {}
        self.setMinimumSize(740, 620)
        self.setFocusPolicy(QtCore.Qt.StrongFocus)
        self.refresh()

    def selected_node(self):
        return get_node(self.scene.root, self.selected_path)

    def edit_node(self):
        return get_node(self.scene.root, self.edit_path)

    def selected_transform(self):
        return self.selected_node().setdefault("transform", default_transform())

    def immediate_child_paths(self):
        group = self.edit_node()
        return [self.edit_path + (i,) for i, _ in enumerate(group.get("children", []))]

    def preview_scene(self):
        if not self.focus_path:
            return self.scene
        data = copy.deepcopy(self.scene.data)
        data["root"] = prune_to_focus(data["root"], tuple(self.focus_path))
        return gen.Scene(data)

    def visible_bg_qcolor(self):
        r, g, b, a = self.scene.background_rgba()
        return QtGui.QColor(r, g, b, a)

    def refresh(self):
        self.work_size = tuple(
            self.scene.render_cfg.get(
                "work_size", self.scene.meta.get("work_size", [900, 640])
            )
        )
        origin = tuple(
            self.scene.meta.get(
                "origin", [self.work_size[0] / 2, self.work_size[1] / 2]
            )
        )
        scene_for_preview = self.preview_scene()
        renderer = gen.Renderer(
            scene_for_preview, frame_size=self.work_size, aa_scale=2, origin=origin
        )
        img = renderer.render("hover", 1, 6, debug=False)
        self._bounds_by_id = renderer.bounds_by_id.copy()
        self._path_by_id = {
            node.get("id"): path for path, node in iter_nodes(self.scene.root)
        }
        self._pixmap = QtGui.QPixmap.fromImage(ImageQt(img.convert("RGBA")))
        self.update()

    def base_view_rect(self):
        margin = 16
        avail = QtCore.QRectF(
            margin, margin, self.width() - 2 * margin, self.height() - 2 * margin
        )
        aspect = self.work_size[0] / self.work_size[1]
        if avail.width() / avail.height() > aspect:
            base_h = avail.height()
            base_w = base_h * aspect
        else:
            base_w = avail.width()
            base_h = base_w / aspect
        return QtCore.QRectF(
            avail.center().x() - base_w / 2,
            avail.center().y() - base_h / 2,
            base_w,
            base_h,
        )

    def compute_view_rect(self):
        base = self.base_view_rect()
        scale = self.zoom * base.width() / self.work_size[0]
        left = base.center().x() - self.view_center.x() * scale
        top = base.center().y() - self.view_center.y() * scale
        return QtCore.QRectF(
            left, top, self.work_size[0] * scale, self.work_size[1] * scale
        )

    def image_to_widget(self, p):
        base = self.base_view_rect()
        scale = self.zoom * base.width() / self.work_size[0]
        return QtCore.QPointF(
            base.center().x() + (p[0] - self.view_center.x()) * scale,
            base.center().y() + (p[1] - self.view_center.y()) * scale,
        )

    def widget_to_image(self, p):
        base = self.base_view_rect()
        scale = self.zoom * base.width() / self.work_size[0]
        return (
            self.view_center.x() + (p.x() - base.center().x()) / scale,
            self.view_center.y() + (p.y() - base.center().y()) / scale,
        )

    def image_delta(self, delta):
        base = self.base_view_rect()
        scale = self.zoom * base.width() / self.work_size[0]
        return delta.x() / scale, delta.y() / scale

    def box_to_rect(self, box):
        p1 = self.image_to_widget((box[0], box[1]))
        p2 = self.image_to_widget((box[2], box[3]))
        return QtCore.QRectF(p1, p2).normalized()

    def paintEvent(self, event):
        painter = QtGui.QPainter(self)
        painter.fillRect(self.rect(), self.visible_bg_qcolor())
        rect = self.compute_view_rect()
        if self._pixmap:
            painter.drawPixmap(
                rect,
                self._pixmap,
                QtCore.QRectF(0, 0, self.work_size[0], self.work_size[1]),
            )

        selected_id = self.selected_node().get("id")
        edit_id = self.edit_node().get("id")
        child_paths = set(self.immediate_child_paths())

        for path in child_paths:
            node = get_node(self.scene.root, path)
            box = self._bounds_by_id.get(node.get("id"))
            if not box:
                continue
            br = self.box_to_rect(box)
            painter.setPen(QtGui.QPen(QtGui.QColor(40, 110, 190, 95), 1))
            painter.drawRect(br)

        edit_box = self._bounds_by_id.get(edit_id)
        if edit_box and self.edit_path:
            pen = QtGui.QPen(QtGui.QColor(70, 70, 70, 130), 1)
            pen.setStyle(QtCore.Qt.DashLine)
            painter.setPen(pen)
            painter.drawRect(self.box_to_rect(edit_box))

        selected_box = self._bounds_by_id.get(selected_id)
        if selected_box:
            painter.setPen(QtGui.QPen(QtGui.QColor(255, 40, 120), 2))
            br = self.box_to_rect(selected_box)
            painter.drawRect(br)
            painter.drawText(br.topLeft() + QtCore.QPointF(2, -4), selected_id)

        painter.setPen(QtGui.QColor(20, 20, 20))
        if self.transform_mode:
            box = self.selected_box()
            if box:
                x1, y1, x2, y2 = box
                ptop = self.image_to_widget(((x1 + x2) / 2, y1))
                painter.setBrush(QtGui.QBrush(QtGui.QColor(255, 255, 255, 230)))
                for hname, rect in self.handle_boxes().items():
                    if hname == "rotate":
                        painter.setPen(QtGui.QPen(QtGui.QColor(40, 120, 255), 2))
                        painter.drawLine(ptop, rect.center())
                        painter.drawEllipse(rect)
                    else:
                        painter.setPen(QtGui.QPen(QtGui.QColor(255, 120, 40), 2))
                        painter.drawRect(rect)

        crumb = " / ".join(
            get_node(self.scene.root, self.edit_path[:i]).get("label", "")
            for i in range(1, len(self.edit_path) + 1)
        )
        focus_txt = (
            "off"
            if not self.focus_path
            else get_node(self.scene.root, self.focus_path).get(
                "label", get_node(self.scene.root, self.focus_path).get("id", "")
            )
        )
        painter.drawText(
            20,
            self.height() - 18,
            f"edit level: {crumb or 'Root'}   focus: {focus_txt}   zoom {self.zoom * 100:.0f}%",
        )

    def pick_path(self, ix, iy):
        """Pick the topmost visible node under the cursor.

        Bounds come from the rendered preview, so hidden/focused-out nodes are
        not pickable. The raw hit may be a deep child; callers should pass it
        through target_path_for_hit() to respect the current edit level.
        """
        hits = []
        for node_id, box in self._bounds_by_id.items():
            path = self._path_by_id.get(node_id)
            if path is None:
                continue
            x1, y1, x2, y2 = box
            if x1 <= ix <= x2 and y1 <= iy <= y2:
                node = get_node(self.scene.root, path)
                hits.append(
                    (len(path), int(node.get("z_order", 0)), str(node_id), path)
                )
        if hits:
            return sorted(hits)[-1][3]
        return None

    def target_path_for_hit(self, hit_path):
        """Coerce a visible hit to the node at the current edit level.

        Example: at root level, clicking an individual rib selects the ribs
        group, not the rib primitive. If editing inside the ribs group, clicking
        the same rib selects that rib. If a visible object outside the current
        edit group is clicked, select its representative node at the current
        edit depth so the user can jump to it.
        """
        if hit_path is None:
            return None
        hit_path = tuple(hit_path)
        edit_path = tuple(self.edit_path)

        if edit_path and hit_path[: len(edit_path)] == edit_path:
            if len(hit_path) > len(edit_path):
                return edit_path + (hit_path[len(edit_path)],)
            return edit_path

        depth = max(1, len(edit_path) + 1)
        return hit_path[: min(depth, len(hit_path))]

    def selected_box(self):
        try:
            node_id = self.selected_node().get("id")
        except Exception:
            return None
        return self._bounds_by_id.get(node_id)

    def handle_boxes(self):
        box = self.selected_box()
        if not box:
            return {}
        x1, y1, x2, y2 = box
        pts = {
            "scale_nw": (x1, y1),
            "scale_ne": (x2, y1),
            "scale_sw": (x1, y2),
            "scale_se": (x2, y2),
            "rotate": ((x1 + x2) / 2, y1 - 30),
        }
        out = {}
        for name, pt in pts.items():
            wp = self.image_to_widget(pt)
            r = 7 if name != "rotate" else 9
            out[name] = QtCore.QRectF(wp.x() - r, wp.y() - r, 2 * r, 2 * r)
        return out

    def hit_handle(self, pos):
        if not self.transform_mode:
            return None
        for name, rect in self.handle_boxes().items():
            if rect.contains(pos):
                return name
        return None

    def mousePressEvent(self, event):
        self.setFocus()
        self.last_pos = event.position()

        if event.button() == QtCore.Qt.MiddleButton or (
            event.modifiers() & QtCore.Qt.AltModifier
            and event.button() == QtCore.Qt.LeftButton
        ):
            self.dragging = True
            self.drag_mode = "pan"
            return

        handle = self.hit_handle(event.position())
        if handle:
            self.dragging = True
            self.active_handle = handle
            if handle == "rotate":
                self.drag_mode = "rotate"
                box = self.selected_box()
                if box:
                    cx = (box[0] + box[2]) / 2
                    cy = (box[1] + box[3]) / 2
                    self.rotate_center_widget = self.image_to_widget((cx, cy))
                    v = event.position() - self.rotate_center_widget
                    self.rotate_last_angle = math.degrees(math.atan2(v.y(), v.x()))
            else:
                self.drag_mode = "scale"
                self.scale_anchor = handle
            return

        if self.transform_mode:
            # Transform mode locks selection. Any left drag on the canvas
            # translates the currently selected node by default.
            if event.button() == QtCore.Qt.LeftButton:
                self.dragging = True
                self.drag_mode = "move"
            return

        ix, iy = self.widget_to_image(event.position())
        raw = self.pick_path(ix, iy)
        target = self.target_path_for_hit(raw)
        if target is not None:
            self.selected_path = target
            self.nodeSelected.emit(target)
            return

        self.dragging = False
        self.drag_mode = None

    def mouseMoveEvent(self, event):
        if not self.dragging or self.last_pos is None:
            return

        delta = event.position() - self.last_pos
        if self.drag_mode == "pan":
            dx, dy = self.image_delta(delta)
            self.view_center -= QtCore.QPointF(dx, dy)
            self.update()
            self.last_pos = event.position()
            return

        if not self.transform_mode:
            self.last_pos = event.position()
            return

        dx, dy = self.image_delta(delta)
        t = self.selected_transform()

        if self.drag_mode == "move":
            t["x"] = float(t.get("x", 0)) + dx
            t["y"] = float(t.get("y", 0)) + dy
            self.sceneChanged.emit()
            self.refresh()
        elif self.drag_mode == "rotate":
            if self.rotate_center_widget is not None:
                v = event.position() - self.rotate_center_widget
                ang = math.degrees(math.atan2(v.y(), v.x()))
                if self.rotate_last_angle is not None:
                    t["rotation"] = float(t.get("rotation", 0)) + (
                        ang - self.rotate_last_angle
                    )
                self.rotate_last_angle = ang
            else:
                t["rotation"] = float(t.get("rotation", 0)) + delta.x() * 0.35
            self.sceneChanged.emit()
            self.refresh()
        elif self.drag_mode == "scale":
            sx = float(t.get("scale_x", 1))
            sy = float(t.get("scale_y", 1))
            sign_x = -1 if self.scale_anchor in {"scale_nw", "scale_sw"} else 1
            sign_y = -1 if self.scale_anchor in {"scale_nw", "scale_ne"} else 1
            nsx = max(-10.0, min(10.0, sx + sign_x * dx / 100.0))
            nsy = max(-10.0, min(10.0, sy + sign_y * dy / 100.0))
            if abs(nsx) < 0.05:
                nsx = 0.05 if nsx >= 0 else -0.05
            if abs(nsy) < 0.05:
                nsy = 0.05 if nsy >= 0 else -0.05
            t["scale_x"] = nsx
            t["scale_y"] = nsy
            self.sceneChanged.emit()
            self.refresh()

        self.last_pos = event.position()

    def mouseDoubleClickEvent(self, event):
        ix, iy = self.widget_to_image(event.position())
        picked = self.pick_path(ix, iy)
        if picked is None:
            self.backgroundDoubleClicked.emit()
            return
        target = self.target_path_for_hit(picked)
        if target is not None:
            self.selected_path = target
            self.nodeSelected.emit(target)
        node = self.selected_node()
        if node.get("kind") == "group":
            self.edit_path = self.selected_path
            self.editLevelChanged.emit(self.edit_path)
            self.update()

    def mouseReleaseEvent(self, event):
        self.dragging = False
        self.drag_mode = None
        self.active_handle = None
        self.last_pos = None
        self.rotate_center_widget = None
        self.rotate_last_angle = None
        self.scale_anchor = None

    def wheelEvent(self, event):
        if (
            event.modifiers() & QtCore.Qt.ControlModifier
            and self.selected_path
            and self.transform_mode
        ):
            t = self.selected_transform()
            t["rotation"] = (
                float(t.get("rotation", 0)) + event.angleDelta().y() / 120.0 * 2.0
            )
            self.sceneChanged.emit()
            self.refresh()
            return
        cursor = event.position()
        before = self.widget_to_image(cursor)
        steps = event.angleDelta().y() / 120.0
        self.zoom = max(0.1, min(24.0, self.zoom * (1.15**steps)))
        base = self.base_view_rect()
        scale = self.zoom * base.width() / self.work_size[0]
        self.view_center = QtCore.QPointF(
            before[0] - (cursor.x() - base.center().x()) / scale,
            before[1] - (cursor.y() - base.center().y()) / scale,
        )
        self.update()


class MainWindow(QtWidgets.QMainWindow):
    def __init__(self, scene_path: Path):
        super().__init__()
        self.setWindowTitle("Mockingbird Scene Graph Editor")
        self.canvas = SceneCanvas(scene_path)
        self.tree = QtWidgets.QTreeWidget()
        self.tree.setHeaderLabels(["Node", "kind", "z", "👁"])
        self.tree.setSelectionMode(QtWidgets.QAbstractItemView.SingleSelection)
        self.tree.setMinimumWidth(480)
        self.tree.setMinimumHeight(420)
        self.tree.header().setStretchLastSection(False)
        self.tree.header().setSectionResizeMode(0, QtWidgets.QHeaderView.Interactive)
        self.tree.header().setSectionResizeMode(
            1, QtWidgets.QHeaderView.ResizeToContents
        )
        self.tree.header().setSectionResizeMode(
            2, QtWidgets.QHeaderView.ResizeToContents
        )
        self.tree.header().setSectionResizeMode(
            3, QtWidgets.QHeaderView.ResizeToContents
        )
        self.expand_tree_btn = QtWidgets.QPushButton("Expand Tree")
        self.collapse_tree_btn = QtWidgets.QPushButton("Collapse Tree")
        self.id_edit = QtWidgets.QLineEdit()
        self.label_edit = QtWidgets.QLineEdit()
        self.kind_label = QtWidgets.QLabel()
        self.visible_check = QtWidgets.QCheckBox("visible")
        self.z_spin = QtWidgets.QSpinBox()
        self.z_spin.setRange(-9999, 9999)
        self.x_spin = self.dspin()
        self.y_spin = self.dspin()
        self.rot_spin = self.dspin()
        self.sx_spin = self.dspin(-10, 10, 0.05)
        self.sy_spin = self.dspin(-10, 10, 0.05)
        self.primitive_yaml = QtWidgets.QPlainTextEdit()
        self.primitive_yaml.setMinimumHeight(150)
        self.edit_level_label = QtWidgets.QLabel("Root")
        self.enter_group_btn = QtWidgets.QPushButton("Enter Group")
        self.up_group_btn = QtWidgets.QPushButton("Up Level")
        self.root_group_btn = QtWidgets.QPushButton("Root Level")
        self.add_group_btn = QtWidgets.QPushButton("Add Group")
        self.add_shape_btn = QtWidgets.QPushButton("Add Shape")
        self.remove_btn = QtWidgets.QPushButton("Remove Node")
        self.dup_btn = QtWidgets.QPushButton("Duplicate Node")
        self.apply_prim_btn = QtWidgets.QPushButton("Apply Primitive YAML")
        self.transform_btn = QtWidgets.QPushButton("Transform Node")
        self.transform_btn.setCheckable(True)
        self.transform_btn.setToolTip(
            "Toggle transform workflow. While active, canvas clicks do not select other nodes; drag translates selected node by default."
        )
        self.focus_btn = QtWidgets.QPushButton("Focus Node")
        self.clear_focus_btn = QtWidgets.QPushButton("Clear Focus")
        self.focus_label = QtWidgets.QLabel("none")
        self.bg_rgba_edit = QtWidgets.QLineEdit()
        self.bg_pick_btn = QtWidgets.QPushButton("Pick…")
        self.bg_apply_btn = QtWidgets.QPushButton("Apply BG")
        self.save_btn = QtWidgets.QPushButton("Save Scene YAML")
        self.render_btn = QtWidgets.QPushButton("Render Quick Preview")
        self.status = QtWidgets.QLabel("Ready")

        side = QtWidgets.QWidget()
        side_lay = QtWidgets.QVBoxLayout(side)
        side_lay.setContentsMargins(4, 4, 4, 4)
        side_lay.setSpacing(4)

        tree_panel = QtWidgets.QWidget()
        tree_lay = QtWidgets.QVBoxLayout(tree_panel)
        tree_lay.setContentsMargins(0, 0, 0, 0)
        tree_lay.setSpacing(4)
        tree_header = QtWidgets.QHBoxLayout()
        tree_header.addWidget(QtWidgets.QLabel("Scene tree"))
        tree_header.addStretch(1)
        tree_header.addWidget(self.expand_tree_btn)
        tree_header.addWidget(self.collapse_tree_btn)
        tree_lay.addLayout(tree_header)
        tree_lay.addWidget(self.tree, 1)

        props_panel = QtWidgets.QWidget()
        lay = QtWidgets.QVBoxLayout(props_panel)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.setSpacing(4)
        lay.addWidget(QtWidgets.QLabel("Canvas edit level"))
        lay.addWidget(self.edit_level_label)
        lay.addWidget(self.transform_btn)
        lay.addWidget(QtWidgets.QLabel("Focus / isolate"))
        lay.addWidget(self.focus_label)
        row = QtWidgets.QHBoxLayout()
        row.addWidget(self.focus_btn)
        row.addWidget(self.clear_focus_btn)
        lay.addLayout(row)
        row = QtWidgets.QHBoxLayout()
        row.addWidget(self.enter_group_btn)
        row.addWidget(self.up_group_btn)
        row.addWidget(self.root_group_btn)
        lay.addLayout(row)
        row = QtWidgets.QHBoxLayout()
        row.addWidget(self.add_group_btn)
        row.addWidget(self.add_shape_btn)
        lay.addLayout(row)
        row = QtWidgets.QHBoxLayout()
        row.addWidget(self.dup_btn)
        row.addWidget(self.remove_btn)
        lay.addLayout(row)
        form = QtWidgets.QFormLayout()
        form.addRow("id", self.id_edit)
        form.addRow("label", self.label_edit)
        form.addRow("kind", self.kind_label)
        form.addRow("z_order", self.z_spin)
        form.addRow("", self.visible_check)
        form.addRow("x", self.x_spin)
        form.addRow("y", self.y_spin)
        form.addRow("rotation", self.rot_spin)
        form.addRow("scale_x", self.sx_spin)
        form.addRow("scale_y", self.sy_spin)
        bgrow = QtWidgets.QHBoxLayout()
        bgrow.addWidget(self.bg_rgba_edit, 1)
        bgrow.addWidget(self.bg_pick_btn)
        bgrow.addWidget(self.bg_apply_btn)
        bgw = QtWidgets.QWidget()
        bgw.setLayout(bgrow)
        form.addRow("background", bgw)
        fw = QtWidgets.QWidget()
        fw.setLayout(form)
        lay.addWidget(fw)
        lay.addWidget(QtWidgets.QLabel("Primitive YAML (shape nodes only)"))
        lay.addWidget(self.primitive_yaml, 2)
        lay.addWidget(self.apply_prim_btn)
        lay.addWidget(self.save_btn)
        lay.addWidget(self.render_btn)
        lay.addWidget(self.status)

        side_split = QtWidgets.QSplitter(QtCore.Qt.Vertical)
        side_split.addWidget(tree_panel)
        side_split.addWidget(props_panel)
        side_split.setStretchFactor(0, 2)
        side_split.setStretchFactor(1, 1)
        side_split.setSizes([620, 420])
        side_lay.addWidget(side_split, 1)

        side.setMinimumWidth(520)
        split = QtWidgets.QSplitter()
        split.addWidget(side)
        split.addWidget(self.canvas)
        split.setStretchFactor(0, 0)
        split.setStretchFactor(1, 1)
        split.setSizes([560, 980])
        self.setCentralWidget(split)

        self.tree.setColumnWidth(0, 280)
        self.tree.setColumnWidth(1, 70)
        self.tree.setColumnWidth(2, 48)
        self.tree.setColumnWidth(3, 38)
        self.rebuild_tree()
        self.connect_signals()
        QtGui.QShortcut(
            QtGui.QKeySequence(QtCore.Qt.Key_Escape),
            self,
            activated=self.escape_up_one_level,
        )
        self.select_path(())
        self.load_background_widgets()
        self.set_focus_label()

    def dspin(self, lo=-9999, hi=9999, step=1):
        s = QtWidgets.QDoubleSpinBox()
        s.setRange(lo, hi)
        s.setDecimals(3)
        s.setSingleStep(step)
        return s

    def connect_signals(self):
        self.tree.currentItemChanged.connect(self.on_tree_select)
        self.tree.itemChanged.connect(self.on_tree_item_changed)
        self.expand_tree_btn.clicked.connect(self.tree.expandAll)
        self.collapse_tree_btn.clicked.connect(self.collapse_tree_to_edit_level)
        self.canvas.nodeSelected.connect(self.select_path)
        self.canvas.backgroundDoubleClicked.connect(self.select_root_from_background)
        self.canvas.sceneChanged.connect(self.load_props)
        self.canvas.editLevelChanged.connect(self.on_edit_level_changed)
        for w in [self.id_edit, self.label_edit]:
            w.editingFinished.connect(self.props_changed)
        for w in [
            self.z_spin,
            self.x_spin,
            self.y_spin,
            self.rot_spin,
            self.sx_spin,
            self.sy_spin,
        ]:
            w.valueChanged.connect(self.props_changed)
        self.visible_check.toggled.connect(self.props_changed)
        self.enter_group_btn.clicked.connect(self.enter_group)
        self.transform_btn.toggled.connect(self.on_transform_toggled)
        self.focus_btn.clicked.connect(self.focus_selected)
        self.clear_focus_btn.clicked.connect(self.clear_focus)
        self.up_group_btn.clicked.connect(self.up_group)
        self.root_group_btn.clicked.connect(self.root_group)
        self.apply_prim_btn.clicked.connect(self.apply_primitive)
        self.add_group_btn.clicked.connect(lambda: self.add_node("group"))
        self.add_shape_btn.clicked.connect(lambda: self.add_node("shape"))
        self.remove_btn.clicked.connect(self.remove_node)
        self.dup_btn.clicked.connect(self.duplicate_node)
        self.save_btn.clicked.connect(self.save_scene)
        self.render_btn.clicked.connect(self.render_quick)
        self.bg_pick_btn.clicked.connect(self.pick_background)
        self.bg_apply_btn.clicked.connect(self.apply_background_text)

    def collapse_tree_to_edit_level(self):
        self.tree.collapseAll()
        # Keep the root and current edit/selection ancestors visible.
        for path in [
            (),
            tuple(self.canvas.edit_path),
            tuple(self.canvas.selected_path),
        ]:
            for depth in range(len(path) + 1):
                item = self.find_item(path[:depth])
                if item:
                    item.setExpanded(True)
        item = self.find_item(tuple(self.canvas.selected_path))
        if item:
            self.tree.scrollToItem(item, QtWidgets.QAbstractItemView.PositionAtCenter)
        self.status.setText("tree collapsed to current context")

    def load_background_widgets(self):
        bg = self.canvas.scene.background_rgba()
        self.bg_rgba_edit.blockSignals(True)
        self.bg_rgba_edit.setText(", ".join(map(str, bg)))
        self.bg_rgba_edit.blockSignals(False)

    def set_focus_label(self):
        if not self.canvas.focus_path:
            self.focus_label.setText("none")
        else:
            node = get_node(self.canvas.scene.root, self.canvas.focus_path)
            self.focus_label.setText(node.get("label", node.get("id", "")))

    def pick_background(self):
        cur = QtGui.QColor(*self.canvas.scene.background_rgba())
        col = QtWidgets.QColorDialog.getColor(cur, self, "Pick background color")
        if not col.isValid():
            return
        rgba = [col.red(), col.green(), col.blue(), col.alpha()]
        self.canvas.scene.render_cfg["background_rgba"] = rgba
        self.load_background_widgets()
        self.canvas.refresh()
        self.status.setText(f"background set to {rgba}")

    def apply_background_text(self):
        text = self.bg_rgba_edit.text().strip()
        parts = [p.strip() for p in text.replace(";", ",").split(",") if p.strip()]
        try:
            vals = [int(float(p)) for p in parts]
            if len(vals) == 3:
                vals.append(255)
            if len(vals) != 4:
                raise ValueError("expected 3 or 4 comma-separated integers")
            vals = [max(0, min(255, v)) for v in vals]
            self.canvas.scene.render_cfg["background_rgba"] = vals
            self.load_background_widgets()
            self.canvas.refresh()
            self.status.setText(f"background set to {vals}")
        except Exception as ex:
            self.status.setText(f"background parse error: {ex}")

    def on_transform_toggled(self, checked):
        self.canvas.transform_mode = bool(checked)
        self.canvas.setCursor(
            QtCore.Qt.SizeAllCursor if checked else QtCore.Qt.ArrowCursor
        )
        self.canvas.update()
        self.status.setText(
            "Transform Node mode on: drag translates selected node; handles scale/rotate; Escape exits"
            if checked
            else "Selection mode on"
        )

    def focus_selected(self):
        self.canvas.focus_path = tuple(self.canvas.selected_path)
        node = self.selected_node()
        if node.get("kind") == "group":
            self.canvas.edit_path = self.canvas.focus_path
            self.on_edit_level_changed(self.canvas.edit_path)
        self.set_focus_label()
        self.canvas.refresh()
        self.status.setText(f"focused on {node.get('id')}")

    def clear_focus(self):
        self.canvas.focus_path = None
        self.set_focus_label()
        self.canvas.refresh()
        self.status.setText("focus cleared")

    def rebuild_tree(self):
        self.tree.blockSignals(True)
        self.tree.clear()

        def add(parent_item, node, path):
            item = QtWidgets.QTreeWidgetItem(
                [
                    node.get("label", node.get("id", "")),
                    node.get("kind", ""),
                    str(node.get("z_order", 0)),
                    "",
                ]
            )
            item.setData(0, QtCore.Qt.UserRole, path)
            item.setFlags(item.flags() | QtCore.Qt.ItemIsUserCheckable)
            item.setCheckState(
                3,
                QtCore.Qt.Checked if node.get("visible", True) else QtCore.Qt.Unchecked,
            )
            if path == self.canvas.edit_path:
                item.setForeground(0, QtGui.QBrush(QtGui.QColor(25, 95, 170)))
                item.setText(0, item.text(0) + "  [editing]")
            if self.canvas.focus_path is not None and path == tuple(
                self.canvas.focus_path
            ):
                item.setForeground(0, QtGui.QBrush(QtGui.QColor(148, 86, 15)))
                item.setText(0, item.text(0) + "  [focus]")
            if parent_item is None:
                self.tree.addTopLevelItem(item)
            else:
                parent_item.addChild(item)
            children = node.get("children", [])
            for orig_idx, child_node in sorted(
                enumerate(children), key=lambda p: (p[1].get("z_order", 0), p[0])
            ):
                add(item, child_node, path + (orig_idx,))
            item.setExpanded(True)

        add(None, self.canvas.scene.root, ())
        self.tree.blockSignals(False)

    def find_item(self, path):
        def rec(item):
            if item.data(0, QtCore.Qt.UserRole) == path:
                return item
            for i in range(item.childCount()):
                found = rec(item.child(i))
                if found:
                    return found
            return None

        for i in range(self.tree.topLevelItemCount()):
            found = rec(self.tree.topLevelItem(i))
            if found:
                return found
        return None

    def set_edit_level_label(self):
        labels = [
            get_node(self.canvas.scene.root, self.canvas.edit_path[:i]).get("label", "")
            for i in range(1, len(self.canvas.edit_path) + 1)
        ]
        text = " / ".join(labels) if labels else "Root"
        self.edit_level_label.setText(text)

    def select_path(self, path):
        self.canvas.selected_path = tuple(path)
        item = self.find_item(tuple(path))
        if item:
            parent = item.parent()
            while parent:
                parent.setExpanded(True)
                parent = parent.parent()
            self.tree.blockSignals(True)
            self.tree.setCurrentItem(item)
            self.tree.scrollToItem(item, QtWidgets.QAbstractItemView.PositionAtCenter)
            self.tree.blockSignals(False)
        self.load_props()
        self.set_focus_label()
        self.canvas.update()

    def on_tree_select(self, cur, prev):
        if cur:
            self.canvas.selected_path = tuple(cur.data(0, QtCore.Qt.UserRole))
            self.load_props()
            self.canvas.update()

    def on_tree_item_changed(self, item, column):
        if column != 3:
            return
        path = item.data(0, QtCore.Qt.UserRole)
        if path is None:
            return
        node = get_node(self.canvas.scene.root, tuple(path))
        node["visible"] = item.checkState(3) == QtCore.Qt.Checked
        self.canvas.refresh()
        self.load_props()
        self.status.setText(
            f"{'shown' if node['visible'] else 'hidden'}: {node.get('id')}"
        )

    def select_root_from_background(self):
        self.canvas.selected_path = ()
        self.canvas.edit_path = ()
        self.canvas.focus_path = None
        self.rebuild_tree()
        self.set_edit_level_label()
        self.set_focus_label()
        self.select_path(())
        self.canvas.refresh()
        self.status.setText("selected root from background")

    def escape_up_one_level(self):
        """Escape exits Transform Node first, otherwise moves one level up."""
        if self.transform_btn.isChecked() or self.canvas.transform_mode:
            self.transform_btn.setChecked(False)
            self.canvas.transform_mode = False
            self.canvas.update()
            self.status.setText("Transform Node mode off")
            return
        if self.canvas.edit_path:
            parent = self.canvas.edit_path[:-1]
            self.canvas.edit_path = parent
            self.on_edit_level_changed(parent)
            return
        if self.canvas.selected_path:
            parent = self.canvas.selected_path[:-1]
            self.select_path(parent)
            self.status.setText("selected parent")
            return
        self.select_path(())
        self.status.setText("already at root")

    def selected_node(self):
        return self.canvas.selected_node()

    def on_edit_level_changed(self, path):
        self.set_edit_level_label()
        self.rebuild_tree()
        self.set_focus_label()
        self.select_path(path)
        self.status.setText(f"editing group: {self.canvas.edit_node().get('id')}")

    def enter_group(self):
        node = self.selected_node()
        if node.get("kind") != "group":
            self.status.setText("select a group before entering it")
            return
        self.canvas.edit_path = self.canvas.selected_path
        self.on_edit_level_changed(self.canvas.edit_path)

    def up_group(self):
        if not self.canvas.edit_path:
            return
        self.canvas.edit_path = self.canvas.edit_path[:-1]
        self.on_edit_level_changed(self.canvas.edit_path)

    def root_group(self):
        self.canvas.edit_path = ()
        self.on_edit_level_changed(())

    def load_props(self):
        node = self.selected_node()
        t = node.setdefault("transform", default_transform())
        widgets = [
            (self.id_edit, "setText", node.get("id", "")),
            (self.label_edit, "setText", node.get("label", "")),
        ]
        for w, meth, val in widgets:
            w.blockSignals(True)
            getattr(w, meth)(str(val))
            w.blockSignals(False)
        self.kind_label.setText(node.get("kind", ""))
        self.visible_check.blockSignals(True)
        self.visible_check.setChecked(bool(node.get("visible", True)))
        self.visible_check.blockSignals(False)
        for spin, key, default in [
            (self.z_spin, "z_order", 0),
            (self.x_spin, "x", 0),
            (self.y_spin, "y", 0),
            (self.rot_spin, "rotation", 0),
            (self.sx_spin, "scale_x", 1),
            (self.sy_spin, "scale_y", 1),
        ]:
            spin.blockSignals(True)
            spin.setValue(
                float(node.get(key, default))
                if key == "z_order"
                else float(t.get(key, default))
            )
            spin.blockSignals(False)
        self.primitive_yaml.blockSignals(True)
        if node.get("kind") == "shape":
            self.primitive_yaml.setPlainText(
                yaml.safe_dump(node.get("primitive", {}), sort_keys=False, width=100)
            )
            self.primitive_yaml.setEnabled(True)
            self.apply_prim_btn.setEnabled(True)
        else:
            self.primitive_yaml.setPlainText("")
            self.primitive_yaml.setEnabled(False)
            self.apply_prim_btn.setEnabled(False)
        self.primitive_yaml.blockSignals(False)
        self.enter_group_btn.setEnabled(node.get("kind") == "group")
        self.set_edit_level_label()
        self.load_background_widgets()
        self.set_focus_label()
        self.status.setText(f"selected: {node.get('id')}")

    def props_changed(self):
        node = self.selected_node()
        node["id"] = slug(self.id_edit.text().strip() or node.get("id", "node"))
        node["label"] = self.label_edit.text().strip() or node["id"]
        node["visible"] = self.visible_check.isChecked()
        node["z_order"] = int(self.z_spin.value())
        t = node.setdefault("transform", default_transform())
        t["x"] = self.x_spin.value()
        t["y"] = self.y_spin.value()
        t["rotation"] = self.rot_spin.value()
        t["scale_x"] = self.sx_spin.value()
        t["scale_y"] = self.sy_spin.value()
        self.rebuild_tree()
        self.select_path(self.canvas.selected_path)
        self.canvas.refresh()

    def apply_primitive(self):
        node = self.selected_node()
        if node.get("kind") != "shape":
            return
        try:
            prim = yaml.safe_load(self.primitive_yaml.toPlainText())
            if not isinstance(prim, dict) or "type" not in prim:
                raise ValueError("primitive must be a mapping with a type field")
            node["primitive"] = prim
            self.status.setText("primitive applied")
            self.canvas.refresh()
        except Exception as ex:
            self.status.setText(f"primitive YAML error: {ex}")

    def add_node(self, kind):
        selected = self.selected_node()
        if selected.get("kind") == "group":
            parent = selected
            ppath = self.canvas.selected_path
        else:
            parent, _ = get_parent(self.canvas.scene.root, self.canvas.selected_path)
            ppath = self.canvas.selected_path[:-1]
        if parent is None:
            parent = self.canvas.scene.root
            ppath = ()
        child = new_group("New Group") if kind == "group" else new_shape("New Shape")
        used = collect_ids(self.canvas.scene.root)
        child["id"] = unique_id(child["id"], used)
        child["z_order"] = len(parent.setdefault("children", []))
        parent.setdefault("children", []).append(child)
        self.rebuild_tree()
        self.select_path(ppath + (len(parent["children"]) - 1,))
        self.canvas.refresh()

    def remove_node(self):
        if not self.canvas.selected_path:
            self.status.setText("cannot remove root")
            return
        parent, idx = get_parent(self.canvas.scene.root, self.canvas.selected_path)
        parent["children"].pop(idx)
        if (
            self.canvas.edit_path[: len(self.canvas.selected_path)]
            == self.canvas.selected_path
        ):
            self.canvas.edit_path = self.canvas.selected_path[:-1]
        self.rebuild_tree()
        self.select_path(self.canvas.selected_path[:-1])
        self.canvas.refresh()

    def duplicate_node(self):
        parent, idx = get_parent(self.canvas.scene.root, self.canvas.selected_path)
        if parent is None:
            self.status.setText("cannot duplicate root")
            return
        dup = copy.deepcopy(get_node(self.canvas.scene.root, self.canvas.selected_path))
        used = collect_ids(self.canvas.scene.root)
        rename_subtree(dup, used)
        dup["z_order"] = int(dup.get("z_order", 0)) + 1
        offset_node(dup, 26, 18)
        parent["children"].insert(idx + 1, dup)
        new_path = self.canvas.selected_path[:-1] + (idx + 1,)
        self.rebuild_tree()
        self.select_path(new_path)
        self.canvas.refresh()
        self.status.setText(f"duplicated subtree: {dup.get('id')}")

    def save_scene(self):
        self.canvas.scene.save(self.canvas.scene_path)
        parts_path = self.canvas.scene_path.parent / "mockingbird_boss_parts.yaml"
        self.canvas.scene.save(parts_path)
        self.status.setText(f"saved {self.canvas.scene_path}")

    def render_quick(self):
        self.save_scene()
        outs = gen.render_outputs(self.canvas.scene_path, quick=True, force=True)
        self.status.setText(f"quick rendered {len(outs)} files")


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--scene", "--rig", default=str(DEFAULT_SCENE))
    args = parser.parse_args(argv)
    app = QtWidgets.QApplication([])
    win = MainWindow(Path(args.scene))
    win.resize(1480, 860)
    win.show()
    raise SystemExit(app.exec())


if __name__ == "__main__":
    main()
