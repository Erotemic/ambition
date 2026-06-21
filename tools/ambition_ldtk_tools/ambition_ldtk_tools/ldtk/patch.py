"""Composable LDtk patch operations.

The patch layer is intentionally small and dict-backed.  It gives mutating
commands a shared vocabulary for preview/apply behavior without forcing the
whole LDtk toolchain into a heavy object model.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from ambition_ldtk_tools.ldtk.fields import entity_field_value
from ambition_ldtk_tools.ldtk.layers import (
    ensure_entities_layer_def,
    ensure_entities_layer_instance,
)
from ambition_ldtk_tools.ldtk.query import (
    EntityLocation,
    find_entity_def,
    find_layer_def,
    find_layer_instance,
    iter_entities,
)


@dataclass
class PatchResult:
    """Result from applying one or more patch operations."""

    changed: bool = False
    messages: list[str] = field(default_factory=list)

    def extend(self, other: "PatchResult") -> None:
        self.changed = self.changed or other.changed
        self.messages.extend(other.messages)


class PatchOp:
    """Base class for LDtk patch operations."""

    def apply(self, project: dict[str, Any]) -> PatchResult:  # pragma: no cover - abstract protocol
        raise NotImplementedError


def _field_matches(entity: dict[str, Any], field_filters: list[tuple[str, str]]) -> bool:
    for name, expected in field_filters:
        value = entity_field_value(entity, name)
        if isinstance(value, str):
            if value != expected:
                return False
        elif str(value) != expected:
            return False
    return True


@dataclass(frozen=True)
class MoveEntitiesToLayer(PatchOp):
    """Move selected entity instances into a target Entities layer."""

    to_layer: str
    level_filter: str | None = None
    from_layer: str | None = None
    iid: str | None = None
    identifier: str | None = None
    field_filters: list[tuple[str, str]] = field(default_factory=list)

    def apply(self, project: dict[str, Any]) -> PatchResult:
        if self.iid is None and self.identifier is None:
            raise SystemExit("select entities with --iid or --identifier")

        selected: list[EntityLocation] = []
        for loc in iter_entities(
            project,
            level_filter=self.level_filter,
            layer_filter=self.from_layer,
        ):
            if loc.layer.get("__identifier") == self.to_layer:
                continue
            entity = loc.entity
            if self.iid is not None and entity.get("iid") != self.iid:
                continue
            if self.identifier is not None and entity.get("__identifier") != self.identifier:
                continue
            if not _field_matches(entity, self.field_filters):
                continue
            selected.append(loc)

        if not selected:
            return PatchResult(False, [])

        by_level_layer: dict[tuple[str, str], list[dict[str, Any]]] = {}
        for loc in selected:
            level_id = str(loc.level.get("identifier"))
            layer_id = str(loc.layer.get("__identifier"))
            by_level_layer.setdefault((level_id, layer_id), []).append(loc.entity)

        dest_def: dict[str, Any] | None = find_layer_def(project, self.to_layer)
        for (level_id, layer_id), entities in by_level_layer.items():
            level = next(
                level
                for level in project.get("levels", []) or []
                if level.get("identifier") == level_id
            )
            source = find_layer_instance(level, layer_id)
            if source is None:
                continue
            if dest_def is None:
                dest_def = ensure_entities_layer_def(
                    project,
                    self.to_layer,
                    clone_from=self.from_layer or layer_id,
                )
            dest = ensure_entities_layer_instance(
                project,
                level,
                self.to_layer,
                dest_def=dest_def,
                clone_from=self.from_layer or layer_id,
            )
            move_ids = {id(entity) for entity in entities}
            remaining = []
            for entity in source.get("entityInstances", []) or []:
                if id(entity) in move_ids:
                    dest.setdefault("entityInstances", []).append(entity)
                else:
                    remaining.append(entity)
            source["entityInstances"] = remaining

        messages = [
            f"{loc.level.get('identifier')}:{loc.layer.get('__identifier')} -> {self.to_layer} "
            f"{loc.entity.get('__identifier')} ({loc.entity.get('iid')})"
            for loc in selected
        ]
        return PatchResult(True, messages)


@dataclass(frozen=True)
class ApplyEntityLayerTagRule(PatchOp):
    """Apply LDtk's tag-based entity/layer placement rule metadata."""

    entity_type: str
    to_layer: str
    from_layer: str | None = None
    tag: str | None = None

    def apply(self, project: dict[str, Any]) -> PatchResult:
        tag = self.tag or self.entity_type
        entity_def = find_entity_def(project, self.entity_type)
        if entity_def is None:
            raise SystemExit(f"entity definition {self.entity_type!r} not found")
        dest_def = ensure_entities_layer_def(
            project,
            self.to_layer,
            clone_from=self.from_layer,
        )
        source_def = find_layer_def(project, self.from_layer) if self.from_layer else None

        changes: list[str] = []
        tags = entity_def.setdefault("tags", [])
        if tag not in tags:
            tags.append(tag)
            changes.append(f"entity {self.entity_type}: added tag {tag!r}")

        required = dest_def.setdefault("requiredTags", [])
        if tag not in required:
            required.append(tag)
            changes.append(f"layer {self.to_layer}: requiredTags += {tag!r}")

        excluded = dest_def.setdefault("excludedTags", [])
        while tag in excluded:
            excluded.remove(tag)
            changes.append(f"layer {self.to_layer}: excludedTags -= {tag!r}")

        if source_def is not None:
            excluded = source_def.setdefault("excludedTags", [])
            if tag not in excluded:
                excluded.append(tag)
                changes.append(f"layer {self.from_layer}: excludedTags += {tag!r}")
            required = source_def.setdefault("requiredTags", [])
            while tag in required:
                required.remove(tag)
                changes.append(f"layer {self.from_layer}: requiredTags -= {tag!r}")
        return PatchResult(bool(changes), changes)
