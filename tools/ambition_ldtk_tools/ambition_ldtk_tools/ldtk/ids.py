"""LDtk UID allocation helpers."""

from __future__ import annotations

from typing import Any


def alloc_uid(project: dict[str, Any]) -> int:
    """Allocate a UID from ``project.nextUid`` and increment the counter."""
    next_uid = int(project.get("nextUid", 1))
    project["nextUid"] = next_uid + 1
    return next_uid
