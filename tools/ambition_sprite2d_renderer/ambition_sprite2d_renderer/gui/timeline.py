"""Timeline panel: clip selection + transport, channel list, key editor.

Channels can be keyframes (table of t / value / ease), an expression of
``t``, or a constant — matching ``rigdoc.sample_channel_spec``.
"""

from __future__ import annotations

from typing import Optional

from PySide6.QtCore import Qt, QTimer
from PySide6.QtWidgets import (
    QCheckBox,
    QComboBox,
    QDoubleSpinBox,
    QHBoxLayout,
    QInputDialog,
    QLabel,
    QLineEdit,
    QListWidget,
    QMessageBox,
    QPushButton,
    QSlider,
    QSpinBox,
    QStackedWidget,
    QTableWidget,
    QTableWidgetItem,
    QVBoxLayout,
    QWidget,
)

from ..authoring.rigdoc import EASE_NAMES
from .state import EditorState


class FrameSlider(QSlider):
    """Horizontal slider that advances exactly one frame per wheel notch.

    Qt's default scrolls ``wheelScrollLines`` (typically 3) per tick, which
    overshoots when scrubbing a short animation frame by frame."""

    def wheelEvent(self, event) -> None:
        delta = event.angleDelta().y()
        if delta == 0:
            event.ignore()
            return
        self.setValue(self.value() + (1 if delta > 0 else -1))
        event.accept()


class TimelinePanel(QWidget):
    def __init__(self, state: EditorState, parent=None) -> None:
        super().__init__(parent)
        self.state = state
        self._refreshing = False
        self.timer = QTimer(self)
        self.timer.timeout.connect(self._tick)

        root = QVBoxLayout(self)

        # ---- transport row ---------------------------------------------------
        row = QHBoxLayout()
        self.clip_combo = QComboBox()
        self.clip_combo.currentTextChanged.connect(self._on_clip_combo)
        row.addWidget(QLabel("clip"))
        row.addWidget(self.clip_combo)
        add_clip = QPushButton("+")
        add_clip.setFixedWidth(28)
        add_clip.clicked.connect(self._add_clip)
        row.addWidget(add_clip)
        del_clip = QPushButton("−")
        del_clip.setFixedWidth(28)
        del_clip.clicked.connect(self._del_clip)
        row.addWidget(del_clip)

        self.frames_spin = QSpinBox()
        self.frames_spin.setRange(1, 64)
        self.frames_spin.valueChanged.connect(lambda v: self._apply_clip_field("frames", int(v)))
        row.addWidget(QLabel("frames"))
        row.addWidget(self.frames_spin)
        self.dur_spin = QSpinBox()
        self.dur_spin.setRange(16, 1000)
        self.dur_spin.setSuffix(" ms")
        self.dur_spin.valueChanged.connect(lambda v: self._apply_clip_field("duration_ms", int(v)))
        row.addWidget(self.dur_spin)
        self.loop_check = QCheckBox("loop")
        self.loop_check.toggled.connect(lambda v: self._apply_clip_field("loop", bool(v)))
        row.addWidget(self.loop_check)

        self.play_btn = QPushButton("▶")
        self.play_btn.setCheckable(True)
        self.play_btn.setFixedWidth(36)
        self.play_btn.toggled.connect(self._on_play)
        row.addWidget(self.play_btn)
        self.frame_slider = FrameSlider(Qt.Orientation.Horizontal)
        self.frame_slider.valueChanged.connect(self._on_slider)
        row.addWidget(self.frame_slider, stretch=1)
        self.frame_label = QLabel("0/8")
        row.addWidget(self.frame_label)
        root.addLayout(row)

        # ---- channels + key editor -------------------------------------------
        body = QHBoxLayout()
        left = QVBoxLayout()
        left.addWidget(QLabel("channels"))
        self.channel_list = QListWidget()
        self.channel_list.currentTextChanged.connect(lambda _: self._refresh_editor())
        left.addWidget(self.channel_list)
        chrow = QHBoxLayout()
        addch = QPushButton("Add channel")
        addch.clicked.connect(self._add_channel)
        delch = QPushButton("Delete")
        delch.clicked.connect(self._del_channel)
        chrow.addWidget(addch)
        chrow.addWidget(delch)
        left.addLayout(chrow)
        body.addLayout(left, stretch=1)

        right = QVBoxLayout()
        type_row = QHBoxLayout()
        type_row.addWidget(QLabel("type"))
        self.type_combo = QComboBox()
        self.type_combo.addItems(["keys", "expr", "const"])
        self.type_combo.currentTextChanged.connect(self._on_type_change)
        type_row.addWidget(self.type_combo)
        type_row.addStretch(1)
        right.addLayout(type_row)

        self.editor_stack = QStackedWidget()
        # keys page
        keys_page = QWidget()
        kl = QVBoxLayout(keys_page)
        self.keys_table = QTableWidget(0, 3)
        self.keys_table.setHorizontalHeaderLabels(["t", "value", "ease"])
        self.keys_table.cellChanged.connect(self._on_key_cell)
        kl.addWidget(self.keys_table)
        krow = QHBoxLayout()
        addk = QPushButton("+ key @ frame")
        addk.clicked.connect(self._add_key_here)
        delk = QPushButton("− key")
        delk.clicked.connect(self._del_key)
        krow.addWidget(addk)
        krow.addWidget(delk)
        kl.addLayout(krow)
        self.editor_stack.addWidget(keys_page)
        # expr page
        expr_page = QWidget()
        el = QVBoxLayout(expr_page)
        self.expr_edit = QLineEdit()
        self.expr_edit.setPlaceholderText("e.g. 2.8*sin(tau*t)")
        self.expr_edit.editingFinished.connect(self._on_expr_edit)
        el.addWidget(self.expr_edit)
        el.addStretch(1)
        self.editor_stack.addWidget(expr_page)
        # const page
        const_page = QWidget()
        cl = QVBoxLayout(const_page)
        self.const_spin = QDoubleSpinBox()
        self.const_spin.setRange(-4096, 4096)
        self.const_spin.setDecimals(3)
        self.const_spin.setKeyboardTracking(False)
        self.const_spin.valueChanged.connect(self._on_const_edit)
        cl.addWidget(self.const_spin)
        cl.addStretch(1)
        self.editor_stack.addWidget(const_page)
        right.addWidget(self.editor_stack)
        body.addLayout(right, stretch=2)
        root.addLayout(body)

        state.docChanged.connect(self.refresh)
        state.timeChanged.connect(self._refresh_transport)
        self.refresh()

    # ---- helpers ----------------------------------------------------------------

    def _channel_name(self) -> Optional[str]:
        item = self.channel_list.currentItem()
        return item.text().split("  ")[0] if item else None

    def _spec(self) -> Optional[dict]:
        name = self._channel_name()
        if not name:
            return None
        return self.state.clip().get("channels", {}).get(name)

    # ---- refresh ------------------------------------------------------------------

    def refresh(self) -> None:
        self._refreshing = True
        try:
            doc = self.state.doc
            self.clip_combo.clear()
            self.clip_combo.addItems(list(doc.clips))
            self.clip_combo.setCurrentText(self.state.clip_name)
            clip = self.state.clip()
            self.frames_spin.setValue(int(clip.get("frames", 8)))
            self.dur_spin.setValue(int(clip.get("duration_ms", 100)))
            self.loop_check.setChecked(bool(clip.get("loop", True)))
            current = self._channel_name()
            self.channel_list.clear()
            for name, spec in clip.get("channels", {}).items():
                kind = "keys" if "keys" in spec else ("expr" if "expr" in spec else "const")
                self.channel_list.addItem(f"{name}  [{kind}]")
            if current:
                for i in range(self.channel_list.count()):
                    if self.channel_list.item(i).text().split("  ")[0] == current:
                        self.channel_list.setCurrentRow(i)
                        break
            self._refresh_transport()
            self._refresh_editor()
        finally:
            self._refreshing = False

    def _refresh_transport(self) -> None:
        was = self._refreshing
        self._refreshing = True
        try:
            if self.clip_combo.currentText() != self.state.clip_name:
                # Clip switched programmatically (or via undo) — resync the
                # whole panel, not just the transport row.
                self._refreshing = was
                self.refresh()
                return
            n = self.state.frames()
            self.frame_slider.setMaximum(n - 1)
            self.frame_slider.setValue(self.state.frame_idx)
            self.frame_label.setText(f"{self.state.frame_idx + 1}/{n}")
        finally:
            self._refreshing = was

    def _refresh_editor(self) -> None:
        was = self._refreshing
        self._refreshing = True
        try:
            spec = self._spec()
            if spec is None:
                self.keys_table.setRowCount(0)
                self.expr_edit.clear()
                return
            if "keys" in spec:
                self.type_combo.setCurrentText("keys")
                self.editor_stack.setCurrentIndex(0)
                keys = spec.get("keys", [])
                self.keys_table.setRowCount(len(keys))
                for r, k in enumerate(keys):
                    ease = k[2] if len(k) > 2 else "smooth"
                    self.keys_table.setItem(r, 0, QTableWidgetItem(f"{float(k[0]):.4g}"))
                    self.keys_table.setItem(r, 1, QTableWidgetItem(f"{float(k[1]):.4g}"))
                    self.keys_table.setItem(r, 2, QTableWidgetItem(str(ease)))
            elif "expr" in spec:
                self.type_combo.setCurrentText("expr")
                self.editor_stack.setCurrentIndex(1)
                self.expr_edit.setText(str(spec["expr"]))
            else:
                self.type_combo.setCurrentText("const")
                self.editor_stack.setCurrentIndex(2)
                self.const_spin.setValue(float(spec.get("const", 0.0)))
        finally:
            self._refreshing = was

    # ---- transport edits --------------------------------------------------------------

    def _on_clip_combo(self, name: str) -> None:
        if self._refreshing or not name:
            return
        self.state.set_clip(name)
        self.refresh()

    def _on_slider(self, value: int) -> None:
        if self._refreshing:
            return
        self.state.set_frame(int(value))

    def _on_play(self, playing: bool) -> None:
        self.play_btn.setText("⏸" if playing else "▶")
        if playing:
            self.timer.start(int(self.state.clip().get("duration_ms", 100)))
        else:
            self.timer.stop()

    def _tick(self) -> None:
        self.timer.setInterval(int(self.state.clip().get("duration_ms", 100)))
        self.state.set_frame((self.state.frame_idx + 1) % self.state.frames())

    def _apply_clip_field(self, field: str, value) -> None:
        if self._refreshing:
            return
        self.state.push_undo()
        self.state.clip()[field] = value
        self.state.mark_changed()

    def _add_clip(self) -> None:
        name, ok = QInputDialog.getText(self, "Add clip", "Clip name (e.g. run, jump):")
        name = name.strip()
        if not ok or not name:
            return
        if name in self.state.doc.clips:
            QMessageBox.warning(self, "Add clip", f"Clip {name!r} already exists.")
            return
        self.state.push_undo()
        self.state.doc.clips[name] = {"loop": True, "frames": 8, "duration_ms": 100, "channels": {}}
        self.state.clip_name = name
        self.state.frame_idx = 0
        self.state.mark_changed()
        self.state.timeChanged.emit()

    def _del_clip(self) -> None:
        if len(self.state.doc.clips) <= 1:
            QMessageBox.warning(self, "Delete clip", "A document needs at least one clip.")
            return
        name = self.state.clip_name
        if QMessageBox.question(self, "Delete clip", f"Delete clip {name!r}?") != QMessageBox.StandardButton.Yes:
            return
        self.state.push_undo()
        self.state.doc.clips.pop(name, None)
        self.state.clip_name = next(iter(self.state.doc.clips))
        self.state.frame_idx = 0
        self.state.mark_changed()
        self.state.timeChanged.emit()

    # ---- channel edits ------------------------------------------------------------------

    def _add_channel(self) -> None:
        doc = self.state.doc
        options = [b["name"] for b in doc.bones] + ["root_x", "root_y"]
        for leg in doc.ik_legs:
            pre = leg.get("channel_prefix", "foot")
            options += [f"{pre}_x", f"{pre}_lift", f"{pre}_pitch"]
        options += ["(custom…)"]
        existing = set(self.state.clip().get("channels", {}))
        options = [o for o in options if o not in existing]
        name, ok = QInputDialog.getItem(self, "Add channel", "Channel:", options, 0, False)
        if not ok:
            return
        if name == "(custom…)":
            name, ok = QInputDialog.getText(self, "Add channel", "Channel name:")
            name = name.strip()
            if not ok or not name:
                return
        self.state.push_undo()
        self.state.clip().setdefault("channels", {})[name] = {"keys": [[0.0, 0.0, "smooth"]]}
        self.state.mark_changed()

    def _del_channel(self) -> None:
        name = self._channel_name()
        if not name:
            return
        self.state.push_undo()
        self.state.clip().get("channels", {}).pop(name, None)
        self.state.mark_changed()

    def _on_type_change(self, kind: str) -> None:
        if self._refreshing:
            return
        name = self._channel_name()
        spec = self._spec()
        if not name or spec is None:
            return
        current = "keys" if "keys" in spec else ("expr" if "expr" in spec else "const")
        if current == kind:
            return
        self.state.push_undo()
        channels = self.state.clip().setdefault("channels", {})
        loop = bool(self.state.clip().get("loop", True))
        from ..authoring.rigdoc import sample_channel_spec

        v0 = sample_channel_spec(spec, 0.0, loop)
        if kind == "keys":
            n = self.state.frames()
            channels[name] = {
                "keys": [
                    [
                        round(self.state.doc.frame_time(self.state.clip_name, i), 4),
                        round(sample_channel_spec(spec, self.state.doc.frame_time(self.state.clip_name, i), loop), 3),
                        "linear",
                    ]
                    for i in range(n)
                ]
            }
        elif kind == "expr":
            channels[name] = {"expr": str(round(v0, 3))}
        else:
            channels[name] = {"const": round(v0, 3)}
        self.state.mark_changed()

    def _on_expr_edit(self) -> None:
        if self._refreshing:
            return
        spec = self._spec()
        if spec is None or "expr" not in spec:
            return
        text = self.expr_edit.text().strip()
        if not text or text == spec.get("expr"):
            return
        from ..authoring.rigdoc import eval_expr

        try:
            eval_expr(text, 0.0)  # validate before committing
        except Exception as ex:  # noqa: BLE001
            QMessageBox.warning(self, "Expression", f"Bad expression:\n{ex}")
            return
        self.state.push_undo()
        spec["expr"] = text
        self.state.mark_changed()

    def _on_const_edit(self, value: float) -> None:
        if self._refreshing:
            return
        spec = self._spec()
        if spec is None or "const" not in spec:
            return
        self.state.push_undo()
        spec["const"] = float(value)
        self.state.mark_changed()

    def _on_key_cell(self, row: int, col: int) -> None:
        if self._refreshing:
            return
        spec = self._spec()
        if spec is None or "keys" not in spec:
            return
        keys = spec["keys"]
        if row >= len(keys):
            return
        item = self.keys_table.item(row, col)
        if item is None:
            return
        text = item.text().strip()
        self.state.push_undo()
        k = list(keys[row]) + (["smooth"] if len(keys[row]) < 3 else [])
        if col == 2:
            if text not in EASE_NAMES:
                return
            k[2] = text
        else:
            try:
                k[col] = float(text)
            except ValueError:
                return
        keys[row] = k
        keys.sort(key=lambda kk: float(kk[0]))
        self.state.mark_changed()

    def _add_key_here(self) -> None:
        name = self._channel_name()
        if not name:
            return
        spec = self._spec()
        loop = bool(self.state.clip().get("loop", True))
        from ..authoring.rigdoc import sample_channel_spec

        value = sample_channel_spec(spec, self.state.t(), loop) if spec else 0.0
        self.state.push_undo()
        self.state.write_key(name, round(value, 3))

    def _del_key(self) -> None:
        spec = self._spec()
        row = self.keys_table.currentRow()
        if spec is None or "keys" not in spec or row < 0 or row >= len(spec["keys"]):
            return
        if len(spec["keys"]) <= 1:
            return
        self.state.push_undo()
        spec["keys"].pop(row)
        self.state.mark_changed()
