"""Patch-plan vocabulary for future area-authoring compiler work.

This module is intentionally small. It gives future refactors a named seam for
``AreaSpec -> patch ops`` without changing the current CLI behavior yet.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass
class AreaPatchPlan:
    """Intermediate plan produced before mutating an LDtk project."""

    area_id: str | None = None
    level_identifier: str | None = None
    ops: list[Any] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)

    def add_note(self, text: str) -> None:
        self.notes.append(text)
