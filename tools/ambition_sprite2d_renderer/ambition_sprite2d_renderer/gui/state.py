"""Shared editor state for the rig editor.

One ``EditorState`` instance is owned by the main window and handed to
every panel. Panels mutate ``state.doc.data`` directly, then emit the
matching signal; everything else refreshes off the signals. Undo is
snapshot-based: callers ``push_undo()`` once before a mutation (or before
a drag begins), and undo/redo swap whole-document JSON snapshots.
"""

from __future__ import annotations

import json
from typing import List, Optional

from PySide6.QtCore import QObject, Signal

from ..authoring.rigdoc import RigDocument, sample_channel_spec

MAX_UNDO = 200


class EditorState(QObject):
    docChanged = Signal()  # any document mutation -> re-render + refresh panels
    selectionChanged = Signal()  # selected bone / part changed
    timeChanged = Signal()  # clip / frame cursor moved

    def __init__(self, doc: RigDocument, path: Optional[str] = None) -> None:
        super().__init__()
        self.doc = doc
        self.path: Optional[str] = path
        self.clip_name: str = next(iter(doc.clips), "idle")
        self.frame_idx: int = 0
        self.selected_bone: Optional[str] = None
        self.selected_part: Optional[int] = None  # index into doc.parts
        self.dirty: bool = False
        self._undo: List[str] = []
        self._redo: List[str] = []

    # ---- Document lifecycle ------------------------------------------------

    def set_doc(self, doc: RigDocument, path: Optional[str]) -> None:
        self.doc = doc
        self.path = path
        self.clip_name = next(iter(doc.clips), "idle")
        self.frame_idx = 0
        self.selected_bone = None
        self.selected_part = None
        self.dirty = False
        self._undo.clear()
        self._redo.clear()
        self.docChanged.emit()
        self.timeChanged.emit()
        self.selectionChanged.emit()

    def mark_changed(self) -> None:
        self.dirty = True
        self.docChanged.emit()

    # ---- Undo ----------------------------------------------------------------

    def push_undo(self) -> None:
        self._undo.append(json.dumps(self.doc.data))
        if len(self._undo) > MAX_UNDO:
            self._undo.pop(0)
        self._redo.clear()

    def undo(self) -> bool:
        if not self._undo:
            return False
        self._redo.append(json.dumps(self.doc.data))
        self.doc.data = json.loads(self._undo.pop())
        self._after_history_swap()
        return True

    def redo(self) -> bool:
        if not self._redo:
            return False
        self._undo.append(json.dumps(self.doc.data))
        self.doc.data = json.loads(self._redo.pop())
        self._after_history_swap()
        return True

    def _after_history_swap(self) -> None:
        if self.clip_name not in self.doc.clips:
            self.clip_name = next(iter(self.doc.clips), "idle")
        if self.selected_part is not None and self.selected_part >= len(self.doc.parts):
            self.selected_part = None
        if self.selected_bone and self.doc.bone(self.selected_bone) is None:
            self.selected_bone = None
        self.dirty = True
        self.docChanged.emit()
        self.timeChanged.emit()
        self.selectionChanged.emit()

    # ---- Time cursor -----------------------------------------------------------

    def clip(self) -> dict:
        return self.doc.clips.setdefault(
            self.clip_name, {"loop": True, "frames": 8, "duration_ms": 100, "channels": {}}
        )

    def frames(self) -> int:
        return max(1, int(self.clip().get("frames", 8)))

    def t(self) -> float:
        return self.doc.frame_time(self.clip_name, self.frame_idx)

    def set_frame(self, idx: int) -> None:
        idx = max(0, min(self.frames() - 1, idx))
        if idx != self.frame_idx:
            self.frame_idx = idx
            self.timeChanged.emit()

    def set_clip(self, name: str) -> None:
        if name in self.doc.clips and name != self.clip_name:
            self.clip_name = name
            self.frame_idx = 0
            self.timeChanged.emit()

    # ---- Key authoring (canvas drags + timeline edits) ----------------------------

    def write_key(self, channel: str, value: float, ease: str = "smooth") -> None:
        """Set ``channel`` to ``value`` at the current frame time, converting
        expr/const channels to baked per-frame keys first (statusbar-level
        surprise is better than silently discarding an expression edit)."""
        clip = self.clip()
        channels = clip.setdefault("channels", {})
        spec = channels.get(channel)
        loop = bool(clip.get("loop", True))
        if spec is not None and "keys" not in spec:
            n = self.frames()
            spec = {
                "keys": [
                    [
                        round(self.doc.frame_time(self.clip_name, i), 4),
                        round(sample_channel_spec(spec, self.doc.frame_time(self.clip_name, i), loop), 3),
                        "linear",
                    ]
                    for i in range(n)
                ]
            }
            channels[channel] = spec
        elif spec is None:
            spec = {"keys": []}
            channels[channel] = spec
        keys = spec["keys"]
        t = round(self.t(), 4)
        for k in keys:
            if abs(float(k[0]) - t) < 1e-4:
                k[1] = round(float(value), 3)
                break
        else:
            keys.append([t, round(float(value), 3), ease])
            keys.sort(key=lambda k: float(k[0]))
        self.mark_changed()
