"""Query helpers over raw LDtk dictionaries."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Iterable, Iterator


@dataclass(frozen=True)
class EntityLocation:
    level: dict[str, Any]
    layer: dict[str, Any]
    entity: dict[str, Any]


def tileset_defs(project: dict[str, Any]) -> list[dict[str, Any]]:
    return project.setdefault("defs", {}).setdefault("tilesets", [])


def entity_defs(project: dict[str, Any]) -> list[dict[str, Any]]:
    return project.setdefault("defs", {}).setdefault("entities", [])


def layer_defs(project: dict[str, Any]) -> list[dict[str, Any]]:
    return project.setdefault("defs", {}).setdefault("layers", [])


def find_tileset(project: dict[str, Any], identifier: str) -> dict[str, Any] | None:
    for ts in tileset_defs(project):
        if ts.get("identifier") == identifier:
            return ts
    return None


def find_entity_def(project: dict[str, Any], identifier: str) -> dict[str, Any] | None:
    for entity_def in entity_defs(project):
        if entity_def.get("identifier") == identifier:
            return entity_def
    return None


def find_layer_def(project: dict[str, Any], identifier: str) -> dict[str, Any] | None:
    for layer_def in layer_defs(project):
        if layer_def.get("identifier") == identifier:
            return layer_def
    return None


def find_level(project: dict[str, Any], identifier: str) -> dict[str, Any] | None:
    for level in project.get("levels", []) or []:
        if level.get("identifier") == identifier:
            return level
    return None


def find_layer_instance(level: dict[str, Any], identifier: str) -> dict[str, Any] | None:
    for layer in level.get("layerInstances", []) or []:
        if layer.get("__identifier") == identifier:
            return layer
    return None


def iter_entities(
    project: dict[str, Any],
    *,
    level_filter: str | None = None,
    layer_filter: str | None = None,
) -> Iterator[EntityLocation]:
    for level in project.get("levels", []) or []:
        if level_filter and level.get("identifier") != level_filter:
            continue
        for layer in level.get("layerInstances", []) or []:
            if layer_filter and layer.get("__identifier") != layer_filter:
                continue
            for entity in layer.get("entityInstances", []) or []:
                yield EntityLocation(level=level, layer=layer, entity=entity)
