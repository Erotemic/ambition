"""Structured room-level diagnostics for LDtk inspection tools."""

from __future__ import annotations

from collections import Counter
from typing import Any, Mapping

from ambition_ldtk_tools.ldtk.issues import Issue


def room_issues(
    level: Mapping[str, Any],
    intgrid: Mapping[str, Any],
    entities: list[Mapping[str, Any]],
) -> list[Issue]:
    """Return static room review notes as shared Issue objects."""
    issues: list[Issue] = []
    level_id = str(level.get("identifier") or "<unnamed>")
    width = int(level.get("pxWid") or 0)
    height = int(level.get("pxHei") or 0)

    starts = [e for e in entities if e.get("identifier") == "PlayerStart"]
    if not starts:
        issues.append(
            Issue(
                severity="warning",
                code="room.player_start.missing",
                message="no PlayerStart entity",
                level=level_id,
            )
        )
    if len(starts) > 1:
        issues.append(
            Issue(
                severity="warning",
                code="room.player_start.multiple",
                message=f"multiple PlayerStart entities ({len(starts)})",
                level=level_id,
                data={"count": len(starts)},
            )
        )

    for e in entities:
        ident = str(e.get("identifier") or "<entity>")
        layer = str(e.get("layer") or "<layer>")
        entity_iid = e.get("iid")
        x, y = e.get("px") or [0, 0]
        w, h = e.get("size") or [0, 0]
        if x < 0 or y < 0 or x + w > width or y + h > height:
            issues.append(
                Issue(
                    severity="warning",
                    code="room.entity.out_of_bounds",
                    message=f"{ident} at {e.get('px')} size {e.get('size')} extends outside room",
                    level=level_id,
                    layer=layer,
                    entity=ident,
                    entity_iid=str(entity_iid) if entity_iid else None,
                    data={"px": list(e.get("px") or []), "size": list(e.get("size") or [])},
                )
            )
        if ident == "CameraZone" and layer != "AmbitionCameras":
            issues.append(
                Issue(
                    severity="warning",
                    code="room.camera_zone.wrong_layer",
                    message=f"CameraZone {entity_iid} is on {layer}, expected AmbitionCameras",
                    level=level_id,
                    layer=layer,
                    entity=ident,
                    entity_iid=str(entity_iid) if entity_iid else None,
                    fixable=True,
                    fix_hint="run `policy fix` or `entity change-layer --to-layer AmbitionCameras`",
                )
            )

    gravity_dirs: Counter[str] = Counter()
    for e in entities:
        if e.get("identifier") == "GravityZone":
            fields = e.get("fields") or {}
            if isinstance(fields, Mapping):
                gravity_dirs[str(fields.get("dir"))] += 1
    if len(gravity_dirs) > 1:
        dirs = ", ".join(f"{k}:{v}" for k, v in sorted(gravity_dirs.items()))
        issues.append(
            Issue(
                severity="info",
                code="room.gravity.multiple_dirs",
                message=f"multiple gravity directions present ({dirs}); inspect frame seams",
                level=level_id,
                data={"dirs": dict(gravity_dirs)},
            )
        )

    collision = intgrid.get("Collision")
    has_collision_values = False
    if isinstance(collision, Mapping):
        has_collision_values = bool(collision.get("values"))
    if not has_collision_values:
        issues.append(
            Issue(
                severity="warning",
                code="room.collision.empty",
                message="Collision IntGrid has no non-empty cells",
                level=level_id,
                layer="Collision",
            )
        )
    return issues
