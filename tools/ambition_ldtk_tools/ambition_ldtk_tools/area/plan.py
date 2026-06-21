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


@dataclass(frozen=True)
class ReplaceExistingLevelOp:
    """Remove an existing level at a known index before appending a replacement."""

    index: int
    label: str = "replace existing level"

    def apply(self, project: dict[str, Any]) -> list[str]:
        if self.index < 0 or self.index >= len(project.get("levels", [])):
            return []
        removed = project["levels"].pop(self.index)
        return [
            f"replacing existing level '{removed['identifier']}' "
            f"({removed['pxWid']}x{removed['pxHei']} at "
            f"{removed['worldX']},{removed['worldY']})"
        ]


@dataclass(frozen=True)
class AppendGeneratedLevelOp:
    """Append a fully-built LDtk level dictionary."""

    level: dict[str, Any]
    area_id: str
    label: str = "append generated level"

    def apply(self, project: dict[str, Any]) -> list[str]:
        project.setdefault("levels", []).append(self.level)
        project.setdefault("toc", [])
        return [
            f"add level '{self.level['identifier']}' (area '{self.area_id}', "
            f"{self.level['pxWid']}x{self.level['pxHei']} at "
            f"{self.level['worldX']},{self.level['worldY']})"
        ]


@dataclass(frozen=True)
class AddReciprocalLoadingZonesOp:
    """Append reciprocal LoadingZone instances described by an area spec.

    The builder is injected to avoid a circular dependency while the legacy
    area-authoring module still owns entity-instance construction.  The op is
    still first-class: it has an explicit label, data, and summary behavior, and
    can later swap its builder for typed LDtk patch operations without changing
    the compile/apply pipeline.
    """

    connections: tuple[dict[str, Any], ...]
    source_level_id: str
    builder: Callable[[dict[str, Any], dict[str, Any], str], dict[str, Any]]
    label: str = "add reciprocal loading zones"

    def apply(self, project: dict[str, Any]) -> list[str]:
        summaries: list[str] = []
        for connection in self.connections:
            instance = self.builder(project, connection, self.source_level_id)
            fields = {
                f["__identifier"]: f.get("__value")
                for f in instance.get("fieldInstances", [])
            }
            summaries.append(
                f"reciprocal LoadingZone in '{connection['target_room']}' at "
                f"px={instance['px']} size=({instance['width']}x{instance['height']}) "
                f"→ {fields.get('target_room')}/{fields.get('target_zone')}"
            )
        return summaries


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
