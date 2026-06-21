"""Side panels for the rig editor: bone tree + properties, parts list +
per-kind property forms, palette editor.

Every panel guards programmatic refreshes with ``self._refreshing`` so
widget signals fired during a rebuild don't write back into the document.
"""

from __future__ import annotations

from typing import Optional

from PySide6.QtCore import Qt
from PySide6.QtGui import QColor
from PySide6.QtWidgets import (
    QColorDialog,
    QComboBox,
    QDoubleSpinBox,
    QFormLayout,
    QGroupBox,
    QHBoxLayout,
    QInputDialog,
    QLabel,
    QLineEdit,
    QListWidget,
    QListWidgetItem,
    QMessageBox,
    QPushButton,
    QTableWidget,
    QTableWidgetItem,
    QTreeWidget,
    QTreeWidgetItem,
    QVBoxLayout,
    QWidget,
)

from ..authoring.rigdoc import PART_KINDS, parse_color
from .state import EditorState


def _dspin(lo=-512.0, hi=512.0, step=0.5, decimals=2) -> QDoubleSpinBox:
    s = QDoubleSpinBox()
    s.setRange(lo, hi)
    s.setSingleStep(step)
    s.setDecimals(decimals)
    s.setKeyboardTracking(False)  # emit valueChanged on commit, not per keystroke
    return s


class BonesPanel(QWidget):
    def __init__(self, state: EditorState, parent=None) -> None:
        super().__init__(parent)
        self.state = state
        self._refreshing = False
        layout = QVBoxLayout(self)
        self.tree = QTreeWidget()
        self.tree.setHeaderLabels(["bone"])
        self.tree.itemSelectionChanged.connect(self._on_tree_select)
        layout.addWidget(self.tree)

        btns = QHBoxLayout()
        add = QPushButton("Add child")
        add.clicked.connect(self._add_bone)
        rm = QPushButton("Delete")
        rm.clicked.connect(self._delete_bone)
        btns.addWidget(add)
        btns.addWidget(rm)
        layout.addLayout(btns)

        form_box = QGroupBox("Bone properties")
        form = QFormLayout(form_box)
        self.parent_label = QLabel("-")
        self.off_x = _dspin()
        self.off_y = _dspin()
        self.length = _dspin(0.0, 512.0)
        self.rest = _dspin(-360.0, 360.0, 1.0, 1)
        for w, key in (
            (self.off_x, ("offset", 0)),
            (self.off_y, ("offset", 1)),
            (self.length, ("length", None)),
            (self.rest, ("rest_angle", None)),
        ):
            w.valueChanged.connect(lambda v, key=key: self._apply_field(key, v))
        form.addRow("parent", self.parent_label)
        form.addRow("offset x", self.off_x)
        form.addRow("offset y", self.off_y)
        form.addRow("length", self.length)
        form.addRow("rest angle", self.rest)
        layout.addWidget(form_box)

        state.docChanged.connect(self.refresh)
        state.selectionChanged.connect(self.refresh_selection)
        self.refresh()

    # ---- refresh -------------------------------------------------------------

    def refresh(self) -> None:
        self._refreshing = True
        try:
            self.tree.clear()
            items = {}
            for b in self.state.doc.bones:
                item = QTreeWidgetItem([b["name"]])
                items[b["name"]] = item
                parent = b.get("parent")
                if parent and parent in items:
                    items[parent].addChild(item)
                else:
                    self.tree.addTopLevelItem(item)
            self.tree.expandAll()
            self._select_in_tree(self.state.selected_bone)
            self._refresh_form()
        finally:
            self._refreshing = False

    def refresh_selection(self) -> None:
        if self._refreshing:
            return
        self._refreshing = True
        try:
            self._select_in_tree(self.state.selected_bone)
            self._refresh_form()
        finally:
            self._refreshing = False

    def _select_in_tree(self, name: Optional[str]) -> None:
        self.tree.clearSelection()
        if not name:
            return
        for item in self.tree.findItems(name, Qt.MatchFlag.MatchExactly | Qt.MatchFlag.MatchRecursive):
            item.setSelected(True)
            self.tree.scrollToItem(item)
            break

    def _refresh_form(self) -> None:
        b = self.state.doc.bone(self.state.selected_bone) if self.state.selected_bone else None
        enabled = b is not None
        for w in (self.off_x, self.off_y, self.length, self.rest):
            w.setEnabled(enabled)
        if not b:
            self.parent_label.setText("-")
            return
        self.parent_label.setText(str(b.get("parent") or "(root)"))
        off = b.get("offset", [0.0, 0.0])
        self.off_x.setValue(float(off[0]))
        self.off_y.setValue(float(off[1]))
        self.length.setValue(float(b.get("length", 0.0)))
        self.rest.setValue(float(b.get("rest_angle", 0.0)))

    # ---- edits -------------------------------------------------------------

    def _on_tree_select(self) -> None:
        if self._refreshing:
            return
        sel = self.tree.selectedItems()
        name = sel[0].text(0) if sel else None
        if name != self.state.selected_bone:
            self.state.selected_bone = name
            self.state.selectionChanged.emit()

    def _apply_field(self, key, value) -> None:
        if self._refreshing:
            return
        b = self.state.doc.bone(self.state.selected_bone) if self.state.selected_bone else None
        if not b:
            return
        self.state.push_undo()
        field, idx = key
        if idx is None:
            b[field] = float(value)
        else:
            off = list(b.get(field, [0.0, 0.0]))
            off[idx] = float(value)
            b[field] = off
        self.state.mark_changed()

    def _add_bone(self) -> None:
        parent = self.state.selected_bone
        name, ok = QInputDialog.getText(self, "Add bone", "Bone name:")
        name = name.strip()
        if not ok or not name:
            return
        if self.state.doc.bone(name) is not None:
            QMessageBox.warning(self, "Add bone", f"Bone {name!r} already exists.")
            return
        self.state.push_undo()
        self.state.doc.bones.append(
            {"name": name, "parent": parent, "offset": [0.0, -8.0], "length": 8.0, "rest_angle": 0.0}
        )
        self.state.selected_bone = name
        self.state.mark_changed()
        self.state.selectionChanged.emit()

    def _delete_bone(self) -> None:
        name = self.state.selected_bone
        if not name:
            return
        doc = self.state.doc
        children = [b["name"] for b in doc.bones if b.get("parent") == name]
        parts = [p.get("name", "?") for p in doc.parts if p.get("bone") == name]
        ik = name in doc.ik_bone_names()
        if children or parts or ik:
            QMessageBox.warning(
                self,
                "Delete bone",
                f"Cannot delete {name!r}: referenced by "
                f"children={children} parts={parts} ik={ik}.",
            )
            return
        self.state.push_undo()
        doc.data["bones"] = [b for b in doc.bones if b["name"] != name]
        # Drop pose channels for the dead bone from every clip.
        for clip in doc.clips.values():
            clip.get("channels", {}).pop(name, None)
        self.state.selected_bone = None
        self.state.mark_changed()
        self.state.selectionChanged.emit()


class PartsPanel(QWidget):
    def __init__(self, state: EditorState, parent=None) -> None:
        super().__init__(parent)
        self.state = state
        self._refreshing = False
        layout = QVBoxLayout(self)

        layout.addWidget(QLabel("Parts (front of character at top)"))
        self.listw = QListWidget()
        self.listw.currentRowChanged.connect(self._on_select)
        layout.addWidget(self.listw)

        zrow = QHBoxLayout()
        raise_btn = QPushButton("▲ Raise")
        raise_btn.setToolTip("Draw later (move toward the front / top of the list)")
        raise_btn.clicked.connect(lambda: self._bump_z(+1))
        lower_btn = QPushButton("▼ Lower")
        lower_btn.setToolTip("Draw earlier (move toward the back / bottom of the list)")
        lower_btn.clicked.connect(lambda: self._bump_z(-1))
        zrow.addWidget(raise_btn)
        zrow.addWidget(lower_btn)
        layout.addLayout(zrow)

        btns = QHBoxLayout()
        for label, fn in (
            ("Add", self._add_part),
            ("Dup", self._dup_part),
            ("Del", self._del_part),
            ("$VISUAL", self._edit_in_visual),
        ):
            b = QPushButton(label)
            b.clicked.connect(fn)
            btns.addWidget(b)
        layout.addLayout(btns)

        form_box = QGroupBox("Part properties")
        form = QFormLayout(form_box)
        self.name_edit = QLineEdit()
        self.name_edit.editingFinished.connect(lambda: self._apply("name", self.name_edit.text()))
        self.bone_combo = QComboBox()
        self.bone_combo.currentTextChanged.connect(lambda v: self._apply("bone", v))
        self.z_spin = _dspin(-1000, 1000, 1.0, 1)
        self.z_spin.valueChanged.connect(lambda v: self._apply("z", float(v)))
        self.kind_label = QLabel("-")
        self.fill_edit = QLineEdit()
        self.fill_edit.editingFinished.connect(lambda: self._apply("fill", self.fill_edit.text() or None))
        self.fill_btn = QPushButton("pick")
        self.fill_btn.clicked.connect(lambda: self._pick_color(self.fill_edit, "fill"))
        self.outline_edit = QLineEdit()
        self.outline_edit.editingFinished.connect(
            lambda: self._apply("outline", self.outline_edit.text() or None)
        )
        self.outline_btn = QPushButton("pick")
        self.outline_btn.clicked.connect(lambda: self._pick_color(self.outline_edit, "outline"))
        self.outline_w = _dspin(0.0, 8.0, 0.05)
        self.outline_w.valueChanged.connect(lambda v: self._apply("outline_w", float(v)))
        self.radius_spin = _dspin(0.0, 128.0, 0.1)
        self.radius_spin.valueChanged.connect(lambda v: self._apply("radius", float(v)))
        self.opacity_edit = QLineEdit()
        self.opacity_edit.setPlaceholderText("(always visible)")
        self.opacity_edit.editingFinished.connect(
            lambda: self._apply("opacity_channel", self.opacity_edit.text() or None)
        )
        form.addRow("name", self.name_edit)
        form.addRow("bone", self.bone_combo)
        form.addRow("z", self.z_spin)
        form.addRow("kind", self.kind_label)
        fill_row = QHBoxLayout()
        fill_row.addWidget(self.fill_edit)
        fill_row.addWidget(self.fill_btn)
        form.addRow("fill", fill_row)
        outline_row = QHBoxLayout()
        outline_row.addWidget(self.outline_edit)
        outline_row.addWidget(self.outline_btn)
        form.addRow("outline", outline_row)
        form.addRow("outline w", self.outline_w)
        form.addRow("radius", self.radius_spin)
        form.addRow("opacity ch", self.opacity_edit)
        layout.addWidget(form_box)

        self.points_box = QGroupBox("Geometry")
        pb = QVBoxLayout(self.points_box)
        self.points_table = QTableWidget(0, 2)
        self.points_table.setHorizontalHeaderLabels(["x", "y"])
        self.points_table.cellChanged.connect(self._on_point_edit)
        pb.addWidget(self.points_table)
        prow = QHBoxLayout()
        addp = QPushButton("+ point")
        addp.clicked.connect(self._add_point)
        delp = QPushButton("- point")
        delp.clicked.connect(self._del_point)
        prow.addWidget(addp)
        prow.addWidget(delp)
        pb.addLayout(prow)
        layout.addWidget(self.points_box)

        state.docChanged.connect(self.refresh)
        self.refresh()

    # ---- helpers ---------------------------------------------------------------

    def _part(self) -> Optional[dict]:
        i = self.state.selected_part
        parts = self.state.doc.parts
        if i is None or not (0 <= i < len(parts)):
            return None
        return parts[i]

    def refresh(self) -> None:
        self._refreshing = True
        try:
            doc = self.state.doc
            self.listw.clear()
            # Display front-to-back (highest z first) so list order matches
            # what's drawn on top. Each row stores its doc.parts index in
            # UserRole so display order and storage order stay decoupled.
            order = sorted(
                range(len(doc.parts)),
                key=lambda i: float(doc.parts[i].get("z", 0)),
                reverse=True,
            )
            for di in order:
                p = doc.parts[di]
                item = QListWidgetItem(
                    f"z{p.get('z', 0):>5}  {p.get('name', '?')} ({p.get('kind', '?')} @ {p.get('bone', '?')})"
                )
                item.setData(Qt.ItemDataRole.UserRole, di)
                self.listw.addItem(item)
            if self.state.selected_part is not None:
                for r in range(self.listw.count()):
                    if self.listw.item(r).data(Qt.ItemDataRole.UserRole) == self.state.selected_part:
                        self.listw.setCurrentRow(r)
                        break
            self.bone_combo.clear()
            self.bone_combo.addItems([b["name"] for b in doc.bones])
            self._refresh_form()
        finally:
            self._refreshing = False

    def _refresh_form(self) -> None:
        p = self._part()
        enabled = p is not None
        for w in (
            self.name_edit, self.bone_combo, self.z_spin, self.fill_edit, self.fill_btn,
            self.outline_edit, self.outline_btn, self.outline_w, self.radius_spin,
            self.opacity_edit, self.points_table,
        ):
            w.setEnabled(enabled)
        if not p:
            self.kind_label.setText("-")
            self.points_table.setRowCount(0)
            return
        self.name_edit.setText(str(p.get("name", "")))
        self.bone_combo.setCurrentText(str(p.get("bone", "")))
        self.z_spin.setValue(float(p.get("z", 0)))
        self.kind_label.setText(str(p.get("kind", "?")))
        self.fill_edit.setText(str(p.get("fill") or ""))
        self.outline_edit.setText(str(p.get("outline") or ""))
        self.outline_w.setValue(float(p.get("outline_w", 0.0)))
        self.radius_spin.setValue(float(p.get("radius", 0.0)))
        self.opacity_edit.setText(str(p.get("opacity_channel") or ""))
        # Geometry table: polygon points, capsule endpoints, or circle center.
        kind = p.get("kind")
        rows = []
        if kind == "polygon":
            rows = [tuple(q) for q in p.get("points", [])]
        elif kind == "capsule":
            rows = [tuple(p.get("a", (0, 0)))]
            if p.get("b") is not None:
                rows.append(tuple(p["b"]))
        elif kind == "circle":
            rows = [tuple(p.get("center", (0, 0)))]
        self.points_table.setRowCount(len(rows))
        for r, (x, y) in enumerate(rows):
            self.points_table.setItem(r, 0, QTableWidgetItem(f"{float(x):.2f}"))
            self.points_table.setItem(r, 1, QTableWidgetItem(f"{float(y):.2f}"))

    # ---- edits -----------------------------------------------------------------

    def _on_select(self, row: int) -> None:
        if self._refreshing:
            return
        item = self.listw.item(row) if row >= 0 else None
        self.state.selected_part = item.data(Qt.ItemDataRole.UserRole) if item else None
        self._refreshing = True
        try:
            self._refresh_form()
        finally:
            self._refreshing = False

    def _bump_z(self, direction: int) -> None:
        """Swap the selected part's z with its neighbor in draw order.
        ``+1`` raises it toward the front, ``-1`` lowers it toward the back."""
        i = self.state.selected_part
        parts = self.state.doc.parts
        if i is None or not (0 <= i < len(parts)):
            return
        # Ascending z = back-to-front; the neighbor in front is at pos+1.
        order = sorted(range(len(parts)), key=lambda j: float(parts[j].get("z", 0)))
        pos = order.index(i)
        target = pos + direction
        if not (0 <= target < len(order)):
            return  # already at the front/back
        other = order[target]
        za, zb = float(parts[i].get("z", 0)), float(parts[other].get("z", 0))
        if za == zb:
            # Equal z: a plain swap wouldn't change anything; nudge instead.
            zb = za + direction
        self.state.push_undo()
        parts[i]["z"], parts[other]["z"] = zb, za
        self.state.mark_changed()

    def _apply(self, field: str, value) -> None:
        if self._refreshing:
            return
        p = self._part()
        if p is None:
            return
        self.state.push_undo()
        if value is None:
            p.pop(field, None)
        else:
            p[field] = value
        self.state.mark_changed()

    def _pick_color(self, edit: QLineEdit, field: str) -> None:
        p = self._part()
        if p is None:
            return
        current = parse_color(p.get(field), self.state.doc.palette) or (255, 255, 255, 255)
        qc = QColorDialog.getColor(
            QColor(*current), self, f"Pick {field}",
            QColorDialog.ColorDialogOption.ShowAlphaChannel,
        )
        if not qc.isValid():
            return
        hexv = f"#{qc.red():02X}{qc.green():02X}{qc.blue():02X}"
        if qc.alpha() != 255:
            hexv += f"{qc.alpha():02X}"
        edit.setText(hexv)
        self._apply(field, hexv)

    def _on_point_edit(self, row: int, col: int) -> None:
        if self._refreshing:
            return
        p = self._part()
        if p is None:
            return
        item = self.points_table.item(row, col)
        try:
            v = float(item.text())
        except (TypeError, ValueError):
            return
        self.state.push_undo()
        kind = p.get("kind")
        if kind == "polygon":
            pts = [list(q) for q in p.get("points", [])]
            if row < len(pts):
                pts[row][col] = v
                p["points"] = pts
        elif kind == "capsule":
            key = "a" if row == 0 else "b"
            cur = list(p.get(key) or (0.0, 0.0))
            cur[col] = v
            p[key] = cur
        elif kind == "circle":
            cur = list(p.get("center", (0.0, 0.0)))
            cur[col] = v
            p["center"] = cur
        self.state.mark_changed()

    def _add_point(self) -> None:
        p = self._part()
        if p is None or p.get("kind") != "polygon":
            return
        self.state.push_undo()
        pts = [list(q) for q in p.get("points", [])]
        pts.append(list(pts[-1]) if pts else [0.0, 0.0])
        p["points"] = pts
        self.state.mark_changed()

    def _del_point(self) -> None:
        p = self._part()
        if p is None or p.get("kind") != "polygon":
            return
        row = self.points_table.currentRow()
        pts = [list(q) for q in p.get("points", [])]
        if 0 <= row < len(pts) and len(pts) > 3:
            self.state.push_undo()
            pts.pop(row)
            p["points"] = pts
            self.state.mark_changed()

    def _add_part(self) -> None:
        kind, ok = QInputDialog.getItem(self, "Add part", "Kind:", list(PART_KINDS), 0, False)
        if not ok:
            return
        bone = self.state.selected_bone or (
            self.state.doc.bones[0]["name"] if self.state.doc.bones else None
        )
        if bone is None:
            return
        self.state.push_undo()
        zmax = max((float(p.get("z", 0)) for p in self.state.doc.parts), default=0.0)
        part = {"name": f"new_{kind}", "bone": bone, "z": zmax + 1, "kind": kind,
                "fill": "shell", "outline": "outline", "outline_w": 1.0}
        if kind == "polygon":
            part.update({"points": [[-5, -5], [5, -5], [5, 5], [-5, 5]], "radius": 2.0})
        elif kind == "capsule":
            part.update({"a": [0.0, 0.0], "b": None, "radius": 2.5})
        else:
            part.update({"center": [0.0, 0.0], "radius": 3.0})
        self.state.doc.parts.append(part)
        self.state.selected_part = len(self.state.doc.parts) - 1
        self.state.mark_changed()

    def _dup_part(self) -> None:
        p = self._part()
        if p is None:
            return
        import copy

        self.state.push_undo()
        q = copy.deepcopy(p)
        q["name"] = f"{q.get('name', 'part')}_copy"
        q["z"] = float(q.get("z", 0)) + 1
        self.state.doc.parts.append(q)
        self.state.selected_part = len(self.state.doc.parts) - 1
        self.state.mark_changed()

    def _del_part(self) -> None:
        i = self.state.selected_part
        if i is None or not (0 <= i < len(self.state.doc.parts)):
            return
        self.state.push_undo()
        self.state.doc.parts.pop(i)
        self.state.selected_part = None
        self.state.mark_changed()

    def _edit_in_visual(self) -> None:
        """Open the selected part's JSON in $VISUAL and reload on exit."""
        import json

        from .external import edit_text_in_visual, visual_command

        p = self._part()
        if p is None:
            return
        if visual_command() is None:
            QMessageBox.warning(self, "$VISUAL", "Set $VISUAL (or $EDITOR) to use this.")
            return
        edited = edit_text_in_visual(json.dumps(p, indent=1))
        if edited is None:
            return
        try:
            new_part = json.loads(edited)
            if not isinstance(new_part, dict):
                raise ValueError("part must be a JSON object")
        except Exception as ex:  # noqa: BLE001
            QMessageBox.critical(self, "$VISUAL", f"Edited JSON is invalid:\n{ex}")
            return
        self.state.push_undo()
        self.state.doc.parts[self.state.selected_part] = new_part
        self.state.mark_changed()


class PalettePanel(QWidget):
    def __init__(self, state: EditorState, parent=None) -> None:
        super().__init__(parent)
        self.state = state
        self._refreshing = False
        layout = QVBoxLayout(self)
        self.table = QTableWidget(0, 2)
        self.table.setHorizontalHeaderLabels(["name", "color"])
        self.table.cellDoubleClicked.connect(self._pick)
        layout.addWidget(self.table)
        add = QPushButton("Add entry")
        add.clicked.connect(self._add)
        layout.addWidget(add)
        state.docChanged.connect(self.refresh)
        self.refresh()

    def refresh(self) -> None:
        self._refreshing = True
        try:
            pal = self.state.doc.palette
            self.table.setRowCount(len(pal))
            for r, (name, value) in enumerate(pal.items()):
                ni = QTableWidgetItem(name)
                ni.setFlags(ni.flags() & ~Qt.ItemFlag.ItemIsEditable)
                ci = QTableWidgetItem(value)
                rgba = parse_color(value, {})
                if rgba:
                    ci.setBackground(QColor(*rgba))
                self.table.setItem(r, 0, ni)
                self.table.setItem(r, 1, ci)
        finally:
            self._refreshing = False

    def _pick(self, row: int, col: int) -> None:
        name_item = self.table.item(row, 0)
        if name_item is None:
            return
        name = name_item.text()
        current = parse_color(self.state.doc.palette.get(name), {}) or (255, 255, 255, 255)
        qc = QColorDialog.getColor(QColor(*current), self, f"Palette: {name}")
        if not qc.isValid():
            return
        self.state.push_undo()
        self.state.doc.palette[name] = f"#{qc.red():02X}{qc.green():02X}{qc.blue():02X}"
        self.state.mark_changed()

    def _add(self) -> None:
        name, ok = QInputDialog.getText(self, "Palette entry", "Name:")
        name = name.strip()
        if not ok or not name or name in self.state.doc.palette:
            return
        self.state.push_undo()
        self.state.doc.palette[name] = "#FFFFFF"
        self.state.mark_changed()
