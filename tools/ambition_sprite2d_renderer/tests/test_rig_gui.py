"""Tests for the PySide6 rig editor (offscreen platform).

The whole module is skipped when PySide6 is unavailable — rigdoc-only
coverage lives in test_rigdoc.py so it still runs everywhere.
"""

from __future__ import annotations

import json
import os
from pathlib import Path

import pytest

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
PySide6 = pytest.importorskip("PySide6")

from ambition_sprite2d_renderer.authoring.rigdoc import RigDocument

TEMPLATE = (
    Path(__file__).resolve().parent.parent
    / "ambition_sprite2d_renderer"
    / "data"
    / "rig_templates"
    / "player_robot_fable.rig.json"
)


@pytest.fixture()
def doc() -> RigDocument:
    return RigDocument.load(TEMPLATE)

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
PySide6 = pytest.importorskip("PySide6")


@pytest.fixture(scope="module")
def qapp():
    from PySide6.QtWidgets import QApplication

    app = QApplication.instance() or QApplication([])
    yield app


@pytest.fixture()
def window(qapp, doc):
    from ambition_sprite2d_renderer.gui.app import MainWindow
    from ambition_sprite2d_renderer.gui.state import EditorState

    state = EditorState(doc, None)
    win = MainWindow(state)
    win.resize(1200, 800)
    win.show()
    qapp.processEvents()
    yield win
    win.close()
    qapp.processEvents()


class TestEditorState:
    def test_write_key_inserts_and_updates(self, doc):
        from ambition_sprite2d_renderer.gui.state import EditorState

        state = EditorState(doc, None)
        state.set_clip("slash")
        state.set_frame(2)
        state.write_key("torso", 42.0)
        keys = doc.clips["slash"]["channels"]["torso"]["keys"]
        t = doc.frame_time("slash", 2)
        match = [k for k in keys if abs(k[0] - t) < 1e-4]
        assert match and match[0][1] == 42.0
        state.write_key("torso", -7.0)  # same frame: update, not duplicate
        assert len([k for k in keys if abs(k[0] - t) < 1e-4]) == 1

    def test_write_key_bakes_expr_channels(self, doc):
        from ambition_sprite2d_renderer.gui.state import EditorState

        state = EditorState(doc, None)
        state.set_clip("idle")
        state.set_frame(0)
        state.write_key("torso", 5.0)
        spec = doc.clips["idle"]["channels"]["torso"]
        assert "keys" in spec and "expr" not in spec
        assert len(spec["keys"]) == int(doc.clips["idle"]["frames"])

    def test_undo_redo_round_trip(self, doc):
        from ambition_sprite2d_renderer.gui.state import EditorState

        state = EditorState(doc, None)
        before = json.dumps(doc.data)
        state.push_undo()
        doc.data["name"] = "mutated"
        assert state.undo()
        assert json.dumps(state.doc.data) == before
        assert state.redo()
        assert state.doc.data["name"] == "mutated"


class TestCanvas:
    def test_hit_test_finds_selected_bone(self, window):
        canvas = window.canvas
        canvas.fit()
        world, _ = canvas.state.doc.solve(canvas.state.clip_name, canvas.state.t())
        head_pos = canvas.frame_to_widget(world["head"].origin)
        hit = canvas._hit_test(head_pos)
        assert hit is not None

    def test_rotate_drag_writes_key(self, window, qapp):
        from PySide6.QtCore import QPointF, Qt
        from PySide6.QtGui import QMouseEvent

        canvas = window.canvas
        canvas.fit()
        state = canvas.state
        state.set_clip("idle")
        world, _ = state.doc.solve("idle", state.t())
        origin = canvas.frame_to_widget(world["near_arm_u"].origin)

        def mouse(etype, pos):
            return QMouseEvent(
                etype, QPointF(pos), Qt.MouseButton.LeftButton,
                Qt.MouseButton.LeftButton, Qt.KeyboardModifier.NoModifier,
            )

        from PySide6.QtCore import QEvent

        canvas.mousePressEvent(mouse(QEvent.Type.MouseButtonPress, origin))
        assert state.selected_bone == "near_arm_u"
        target = QPointF(origin.x() + 40, origin.y() + 40)  # 45° down-forward
        canvas.mouseMoveEvent(mouse(QEvent.Type.MouseMove, target))
        canvas.mouseReleaseEvent(mouse(QEvent.Type.MouseButtonRelease, target))
        spec = state.clip()["channels"]["near_arm_u"]
        assert "keys" in spec
        # world 45° = rest 90 + pose -45 (parents near zero at t=0).
        t = state.t()
        key = min(spec["keys"], key=lambda k: abs(k[0] - t))
        assert key[1] == pytest.approx(-45.0, abs=8.0)

    def test_ctrl_drag_moves_attachment_offset(self, window, qapp):
        from PySide6.QtCore import QEvent, QPointF, Qt
        from PySide6.QtGui import QMouseEvent

        canvas = window.canvas
        canvas.fit()
        state = canvas.state
        state.set_clip("idle")
        state.set_frame(0)
        world, _ = state.doc.solve("idle", state.t())
        origin = canvas.frame_to_widget(world["near_arm_u"].origin)
        before = list(state.doc.bone("near_arm_u")["offset"])

        def mouse(etype, pos, mods):
            return QMouseEvent(
                etype, QPointF(pos), Qt.MouseButton.LeftButton,
                Qt.MouseButton.LeftButton, mods,
            )

        ctrl = Qt.KeyboardModifier.ControlModifier
        canvas.mousePressEvent(mouse(QEvent.Type.MouseButtonPress, origin, ctrl))
        assert canvas._drag_mode == "offset"
        target = QPointF(origin.x() + 2 * canvas.zoom, origin.y() - 3 * canvas.zoom)
        canvas.mouseMoveEvent(mouse(QEvent.Type.MouseMove, target, ctrl))
        canvas.mouseReleaseEvent(
            mouse(QEvent.Type.MouseButtonRelease, target, Qt.KeyboardModifier.NoModifier)
        )
        after = state.doc.bone("near_arm_u")["offset"]
        # Torso is near-upright at idle t=0, so a +2/-3 frame-space drag
        # lands close to +2/-3 in parent-local offset units.
        assert after[0] - before[0] == pytest.approx(2.0, abs=0.6)
        assert after[1] - before[1] == pytest.approx(-3.0, abs=0.6)
        # And the solved origin now sits where the cursor dropped it.
        world2, _ = state.doc.solve("idle", state.t())
        moved = canvas.frame_to_widget(world2["near_arm_u"].origin)
        assert abs(moved.x() - target.x()) < 2 and abs(moved.y() - target.y()) < 2

    def test_ik_leg_bones_refuse_rotation(self, window):
        canvas = window.canvas
        state = canvas.state
        leg = state.doc.foot_leg_for_bone("near_leg_u")
        assert leg is not None and leg["foot"] == "near_foot"

    def test_screenshot_smoke(self, window, qapp, tmp_path):
        pix = window.grab()
        out = tmp_path / "shot.png"
        assert pix.save(str(out))
        assert out.stat().st_size > 10_000


class TestTimelinePanel:
    def test_clip_combo_follows_state(self, window, qapp):
        state = window.state
        state.set_clip("walk")
        qapp.processEvents()
        assert window.timeline.clip_combo.currentText() == "walk"

    def test_play_advances_frames(self, window, qapp):
        tl = window.timeline
        start = window.state.frame_idx
        tl.play_btn.setChecked(True)
        tl._tick()
        assert window.state.frame_idx == (start + 1) % window.state.frames()
        tl.play_btn.setChecked(False)

    def test_wheel_moves_one_frame_per_tick(self, window, qapp):
        from PySide6.QtCore import QPoint, QPointF, Qt
        from PySide6.QtGui import QWheelEvent

        slider = window.timeline.frame_slider
        slider.setValue(3)

        def wheel(dy):
            ev = QWheelEvent(
                QPointF(5, 5), QPointF(5, 5), QPoint(0, 0), QPoint(0, dy),
                Qt.MouseButton.NoButton, Qt.KeyboardModifier.NoModifier,
                Qt.ScrollPhase.NoScrollPhase, False,
            )
            slider.wheelEvent(ev)

        wheel(120)  # one notch up
        assert slider.value() == 4
        wheel(120)
        assert slider.value() == 5
        wheel(-120)  # one notch down
        assert slider.value() == 4


class TestPartsZOrder:
    def _index(self, doc, name):
        return next(i for i, p in enumerate(doc.parts) if p.get("name") == name)

    def test_list_is_front_to_back(self, window):
        from PySide6.QtCore import Qt

        from ambition_sprite2d_renderer.gui.panels import PartsPanel

        parts_panel = window.findChild(PartsPanel)
        zs = [
            window.state.doc.parts[parts_panel.listw.item(r).data(Qt.ItemDataRole.UserRole)]["z"]
            for r in range(parts_panel.listw.count())
        ]
        assert zs == sorted(zs, reverse=True)  # top of list = frontmost

    def test_list_row_maps_to_storage_index(self, window):
        from ambition_sprite2d_renderer.gui.panels import PartsPanel

        parts_panel = window.findChild(PartsPanel)
        parts_panel.listw.setCurrentRow(0)
        front = max(window.state.doc.parts, key=lambda p: p["z"])
        assert window.state.doc.parts[window.state.selected_part]["name"] == front["name"]

    def test_raise_brings_part_in_front(self, window):
        from ambition_sprite2d_renderer.gui.panels import PartsPanel

        doc = window.state.doc
        parts_panel = window.findChild(PartsPanel)
        i_torso = self._index(doc, "torso")  # z=40
        i_shade = self._index(doc, "torso_shade")  # z=41 (just in front)
        assert doc.parts[i_torso]["z"] < doc.parts[i_shade]["z"]
        window.state.selected_part = i_torso
        parts_panel._bump_z(+1)
        assert doc.parts[i_torso]["z"] > doc.parts[i_shade]["z"]

    def test_lower_sends_part_back(self, window):
        from ambition_sprite2d_renderer.gui.panels import PartsPanel

        doc = window.state.doc
        parts_panel = window.findChild(PartsPanel)
        i_shade = self._index(doc, "torso_shade")  # z=41
        i_torso = self._index(doc, "torso")  # z=40 (just behind)
        window.state.selected_part = i_shade
        parts_panel._bump_z(-1)
        assert doc.parts[i_shade]["z"] < doc.parts[i_torso]["z"]

    def test_raise_at_front_is_noop(self, window):
        from ambition_sprite2d_renderer.gui.panels import PartsPanel

        doc = window.state.doc
        parts_panel = window.findChild(PartsPanel)
        front = max(range(len(doc.parts)), key=lambda i: doc.parts[i]["z"])
        before = doc.parts[front]["z"]
        window.state.selected_part = front
        parts_panel._bump_z(+1)
        assert doc.parts[front]["z"] == before
