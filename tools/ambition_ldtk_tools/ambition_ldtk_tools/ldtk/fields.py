"""LDtk field helpers."""

from __future__ import annotations

from typing import Any


def entity_field_value(entity: dict[str, Any], name: str) -> Any:
    for field in entity.get("fieldInstances", []) or []:
        if field.get("__identifier") == name:
            return field.get("__value")
    return None


def default_field_value(field_def: dict[str, Any]) -> Any:
    """Return a conservative default value for a field definition."""
    default = field_def.get("defaultOverride")
    if isinstance(default, dict) and default.get("params"):
        return default.get("params", [None])[0]
    if field_def.get("canBeNull"):
        return None
    typ = field_def.get("__type") or field_def.get("type")
    if typ in {"String", "F_String"}:
        return ""
    if typ in {"Int", "F_Int"}:
        return 0
    if typ in {"Float", "F_Float"}:
        return 0.0
    if typ in {"Bool", "F_Bool"}:
        return False
    return None
