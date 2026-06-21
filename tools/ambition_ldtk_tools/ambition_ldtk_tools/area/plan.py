"""Patch-plan vocabulary for area-authoring compiler work.

Area authoring is being migrated from direct LDtk mutation to a compile/apply
pipeline.  The plan remains deliberately lightweight so existing room builders
can migrate one operation at a time without depending on a heavy object model.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Callable, Protocol


class AreaPatchOp(Protocol):
    """Protocol for an area-authoring patch operation."""

    label: str

    def apply(self, project: dict[str, Any]) -> list[str]:
        """Mutate ``project`` and return human-readable summary lines."""


@dataclass(frozen=True)
class CallableAreaPatchOp:
    """Small adapter for existing imperative area-authoring helpers."""

    label: str
    callback: Callable[[dict[str, Any]], list[str]]

    def apply(self, project: dict[str, Any]) -> list[str]:
        return list(self.callback(project))


@dataclass
class AreaPatchPlan:
    """Intermediate plan produced before mutating an LDtk project."""

    area_id: str | None = None
    level_identifier: str | None = None
    ops: list[AreaPatchOp] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    preview_lines: list[str] = field(default_factory=list)

    def add_note(self, text: str) -> None:
        self.notes.append(text)

    def add_op(self, op: AreaPatchOp) -> None:
        self.ops.append(op)

    def apply(self, project: dict[str, Any]) -> list[str]:
        summaries: list[str] = []
        for op in self.ops:
            summaries.extend(op.apply(project))
        return summaries
