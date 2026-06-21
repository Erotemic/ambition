"""Layer-definition and layer-instance helpers."""

from __future__ import annotations

from typing import Any

from .ids import alloc_uid
from .query import find_layer_def, find_layer_instance


def _is_entities_layer_def(layer_def: dict[str, Any]) -> bool:
    return layer_def.get("__type") == "Entities" or layer_def.get("type") == "Entities"


def ensure_entities_layer_def(
    project: dict[str, Any],
    identifier: str,
    clone_from: str | None = None,
) -> dict[str, Any]:
    """Return an Entities layer definition, cloning a sibling if absent."""
    existing = find_layer_def(project, identifier)
    if existing is not None:
        if not _is_entities_layer_def(existing):
            raise SystemExit(
                f"layer {identifier!r} exists but is not an Entities layer; refusing to use it as an entity destination"
            )
        return existing

    template = find_layer_def(project, clone_from or "Ambition")
    if template is None:
        for layer_def in project.get("defs", {}).get("layers", []) or []:
            if _is_entities_layer_def(layer_def):
                template = layer_def
                break
    if template is None:
        raise SystemExit("project has no Entities layer definition to clone")

    new_def = dict(template)
    new_def["identifier"] = identifier
    new_def["uid"] = alloc_uid(project)
    new_def.setdefault("requiredTags", [])
    new_def.setdefault("excludedTags", [])
    project.setdefault("defs", {}).setdefault("layers", []).append(new_def)
    return new_def


def ensure_entities_layer_instance(
    project: dict[str, Any],
    level: dict[str, Any],
    identifier: str,
    *,
    dest_def: dict[str, Any],
    clone_from: str | None = None,
    insert_before_source: bool = True,
) -> dict[str, Any]:
    """Return an Entities layer instance on ``level``, cloning a sibling if absent."""
    existing = find_layer_instance(level, identifier)
    if existing is not None:
        return existing

    template = find_layer_instance(level, clone_from or "Ambition")
    if template is None:
        for layer in level.get("layerInstances", []) or []:
            if layer.get("__type") == "Entities" or "entityInstances" in layer:
                template = layer
                break
    if template is None:
        raise SystemExit(f"level {level.get('identifier')!r} has no Entities layer instance to clone")

    new_layer = dict(template)
    new_layer["__identifier"] = identifier
    new_layer["layerDefUid"] = dest_def.get("uid")
    new_layer["iid"] = f"{identifier}-{alloc_uid(project)}"
    new_layer["entityInstances"] = []
    layers = level.setdefault("layerInstances", [])
    try:
        idx = layers.index(template)
        layers.insert(idx if insert_before_source else idx + 1, new_layer)
    except ValueError:
        layers.append(new_layer)
    return new_layer
