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


# ---------------------------------------------------------------------------
# Native `EntityRef` relationships.
#
# the registry is here because there are TWO of them now. `mounted_on` (ADR 0020, rider →
# mount) was the first and its field-def synthesis lived in `mount_split`, the one command that
# needed it. One synthesizer, one table of what each relationship MEANS.
#
# Each entry is the field's authored documentation plus `allowedRefs`, the
# editor-side scope of what it may point at ("Any" / "OnlySame").
ENTITY_REF_FIELDS: dict[str, dict[str, str]] = {
    "mounted_on": {
        "doc": (
            "ADR 0020: the mount EnemySpawn this rider is mounted on (the "
            "mount action pre-applied). Resolved into a RidingOn/MountSlot link."
        ),
        "allowed_refs": "OnlySame",
        # a SPEC-authored mount link may cross entity types (the GNU-ton
        # scholar BossSpawn → its `giant_gnu` EnemySpawn mount), which
        # "OnlySame" would forbid. Stated as data rather than as a caller's
        # special case, because the caller is a loop over every ref field.
        "spec_allowed_refs": "Any",
    },
    "path_ref": {
        "doc": (
            "The KinematicPath this body patrols. Conversion resolves it to "
            "that path's own lookup id, so the reference cannot disagree with "
            "the target about how the path is spelled."
        ),
        # the strongest scope LDtk offers: the EDITOR itself will only offer
        # `KinematicPath` targets, so the wrong-kind mistake cannot be made by
        # hand at all. `allowedRefsEntityUid` is resolved per project below.
        "allowed_refs": "OnlySpecificEntity",
        "allowed_refs_entity": "KinematicPath",
    },
}


def ensure_entity_ref_fielddef(
    project: dict[str, Any],
    entity_identifier: str,
    field: str,
    *,
    allowed_refs: str | None = None,
) -> dict[str, Any]:
    """Ensure an entity def carries `field` as an `EntityRef`, idempotently.

    The ~30 keys below are LDtk's editor-roundtrip shape for a reference field;
    getting one wrong makes the editor refuse the file or silently drop the
    link, which is why exactly one function writes them.

    `allowed_refs` overrides the registry default — the same relationship can be
    scoped differently per referrer (a rider `EnemySpawn` points at another
    `EnemySpawn`, so `"OnlySame"`; a rider `BossSpawn` points at an `EnemySpawn`
    mount, a CROSS-type ref, so `"Any"`).
    """
    from ambition_ldtk_tools.area_authoring import allocate_iid, find_entity_def

    known = ENTITY_REF_FIELDS.get(field)
    if known is None:
        raise SystemExit(
            f"'{field}' is not a known EntityRef relationship. Add it to "
            f"`ENTITY_REF_FIELDS` with what it points at and why — a reference "
            f"field nobody documented is the string convention this replaced. "
            f"Known: {sorted(ENTITY_REF_FIELDS)}"
        )
    ent_def = find_entity_def(project, entity_identifier)
    for existing in ent_def.get("fieldDefs", []):
        if existing["identifier"] == field:
            return existing
    scope = allowed_refs or known["allowed_refs"]
    target_uid = None
    if scope == "OnlySpecificEntity":
        target = known.get("allowed_refs_entity")
        if target is None:
            raise SystemExit(
                f"'{field}' declares OnlySpecificEntity but names no "
                f"`allowed_refs_entity`; the editor would offer no targets"
            )
        target_uid = find_entity_def(project, target)["uid"]
    _, uid = allocate_iid(project, entity_identifier)  # bumps nextUid; reuse the int
    field_def = {
        "identifier": field,
        "doc": known["doc"],
        "__type": "EntityRef",
        "uid": uid,
        "type": "F_EntityRef",
        "isArray": False,
        "canBeNull": True,
        "arrayMinLength": None,
        "arrayMaxLength": None,
        "editorDisplayMode": "RefLinkBetweenCenters",
        "editorDisplayScale": 1,
        "editorDisplayPos": "Above",
        "editorLinkStyle": "CurvedArrow",
        "editorDisplayColor": None,
        "editorAlwaysShow": False,
        "editorShowInWorld": True,
        "editorCutLongValues": True,
        "editorTextSuffix": None,
        "editorTextPrefix": None,
        "useForSmartColor": False,
        "exportToToc": False,
        "searchable": False,
        "min": None,
        "max": None,
        "regex": None,
        "acceptFileTypes": None,
        "defaultOverride": None,
        "textLanguageMode": None,
        "symmetricalRef": False,
        "autoChainRef": True,
        "allowOutOfLevelRef": False,
        "allowedRefs": scope,
        "allowedRefsEntityUid": target_uid,
        "allowedRefTags": [],
        "tilesetUid": None,
    }
    ent_def.setdefault("fieldDefs", []).append(field_def)
    return field_def
