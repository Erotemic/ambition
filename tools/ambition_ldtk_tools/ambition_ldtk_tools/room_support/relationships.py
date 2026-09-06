#!/usr/bin/env python3
"""**What authored relationships does this world actually have?**

An agent asked to change a room can already ask what is IN it (``room
describe``) and what it looks like (``room render``). It could not ask what the
entities in it POINT AT — and pointing is where content breaks silently, because
a reference that resolves to nothing degrades to "no behaviour" rather than to an
error anybody sees.

⛔ **this module reports authored FIELDS. It does not decide whether a string
reference resolves**, and that restraint is the whole design. Resolution is owned
by the engine — ``kinematic_path_aliases``/``matches_id`` for kinematic paths,
``AuthoredPlatformMotion::classify`` for platform motion, the room binding sweep
and the game-side content validators for the rest. A Python re-derivation of any
of those would be a second authority for exactly the rules that have already
drifted three times, so this tool points at the authority instead of impersonating
it.

⭐ **the asymmetry in the output is the point.** A NATIVE relationship — an LDtk
``EntityRef`` field — is discoverable from the project schema alone: the tool
finds it without being told it exists, names its target entity, and can say the
target is missing without knowing anything about what the relationship MEANS. A
STRING-CONVENTION relationship cannot be discovered at all; it is a plain string
field, indistinguishable from a label, so this tool can only list the ones it has
been told about by hand. That hand-kept list below is a migration ledger, and a
relationship that moves to ``EntityRef`` leaves it. When it is empty, this half of
the tool deletes itself.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Iterator

# ---------------------------------------------------------------------------
# The hand-kept half: string conventions that LDtk cannot describe.
#
# Each row is (entity identifier, field, what it points at, who owns resolution).
# a row here is a relationship that a tool cannot see without being told. That
# is the cost being paid, and the reason L2 wants these migrated to `EntityRef`.
# ---------------------------------------------------------------------------
STRING_CONVENTIONS: tuple[tuple[str, str, str, str], ...] = (
    ("MovingPlatform", "path_id", "KinematicPath", "AuthoredPlatformMotion::classify + KinematicPathSpec::matches_id"),
    ("NpcSpawn", "path_id", "KinematicPath", "KinematicPathSpec::matches_id (via InteractionKindSpec::Npc)"),
    ("DamageVolume", "path_id", "KinematicPath", "KinematicPathSpec::matches_id (via HazardVolumeSpec)"),
    ("LoadingZone", "target_room", "a level identifier", "validate_ldtk_room_links"),
    ("LoadingZone", "target_zone", "a LoadingZone id in the target level", "validate_ldtk_room_links"),
    ("Portal", "link", "the other Portal sharing the link", "portal pairing at conversion"),
    ("MaryOPipe", "link", "the other MaryOPipe half sharing the link", "Mary-O pipe pairing at load"),
    ("Switch", "target_encounter", "an EncounterTrigger id", "encounter wiring at conversion"),
)

# A reference hidden INSIDE another field's value, which is the least visible
# shape of all: nothing about the field's name or type says it is a reference.
#
# Each row is (entity identifier, field, prefix, what it points at, who owns
# resolution) — the SAME five facts a string-convention row carries, in the same
# order, because they are the same kind of row.
#
# The whole value of this half of the tool is naming the right authority to go read, so a report
# that names the wrong one is worse than no report. A row that carries its own owner cannot inherit
# somebody else's by being added.
PREFIX_CONVENTIONS: tuple[tuple[str, str, str, str, str], ...] = (
    (
        "BossSpawn",
        "brain",
        "PhaseScript:",
        "a boss phase script id",
        "parse_boss_brain -> BossBrain::PhaseScript (boss phase scripts)",
    ),
)


def _iter_entities(project: dict) -> Iterator[tuple[dict, dict]]:
    for level in project.get("levels") or []:
        for layer in level.get("layerInstances") or []:
            if layer.get("__type") != "Entities":
                continue
            for entity in layer.get("entityInstances") or []:
                yield level, entity


def _field_map(entity: dict) -> dict:
    return {f["__identifier"]: f.get("__value") for f in entity.get("fieldInstances") or []}


def _entity_ref_fields(project: dict) -> dict[str, list[str]]:
    """Every ``EntityRef`` field, discovered from the schema alone.

    This is the native half: no list, no convention, no engine knowledge. If a
    game adds an `EntityRef` field tomorrow, it shows up here with no edit to
    this file — which is exactly what a string field can never do.
    """
    found: dict[str, list[str]] = {}
    for ed in (project.get("defs") or {}).get("entities") or []:
        refs = [
            fd["identifier"]
            for fd in ed.get("fieldDefs") or []
            if fd.get("__type") == "EntityRef"
        ]
        if refs:
            found[ed["identifier"]] = refs
    return found


def relationship_report(project: dict, level_id: str | None = None) -> dict:
    """The authored relationship graph, as data."""
    ref_fields = _entity_ref_fields(project)
    by_iid = {e["iid"]: (lv["identifier"], e["__identifier"]) for lv, e in _iter_entities(project)}

    native: list[dict] = []
    conventional: list[dict] = []
    declared_never_authored: list[dict] = []

    seen_string_fields: set[tuple[str, str]] = set()

    for level, entity in _iter_entities(project):
        if level_id is not None and level["identifier"] != level_id:
            continue
        ident = entity["__identifier"]
        fields = _field_map(entity)

        # --- native: EntityRef, discovered from the schema ------------------
        for field in ref_fields.get(ident, ()):
            value = fields.get(field)
            if not value:
                continue
            target_iid = value.get("entityIid") if isinstance(value, dict) else value
            target = by_iid.get(target_iid)
            native.append(
                {
                    "level": level["identifier"],
                    "source": entity["iid"],
                    "source_kind": ident,
                    "field": field,
                    "target_iid": target_iid,
                    "target_kind": target[1] if target else None,
                    "target_level": target[0] if target else None,
                    # the ONE verdict this tool is entitled to: LDtk's own
                    # referential integrity. It is the file's pointer, not an
                    # engine rule, so checking it re-derives nothing.
                    "broken": target is None,
                }
            )

        # --- conventional: a plain string this tool had to be told about ----
        for conv_kind, field, points_at, authority in STRING_CONVENTIONS:
            if conv_kind != ident:
                continue
            seen_string_fields.add((ident, field))
            value = fields.get(field)
            if not isinstance(value, str) or not value.strip():
                continue
            conventional.append(
                {
                    "level": level["identifier"],
                    "source": entity["iid"],
                    "source_kind": ident,
                    "field": field,
                    "spelling": value,
                    "points_at": points_at,
                    "resolution_owned_by": authority,
                    "shape": "string field",
                }
            )

        for conv_kind, field, prefix, points_at, authority in PREFIX_CONVENTIONS:
            if conv_kind != ident:
                continue
            seen_string_fields.add((ident, field))
            value = fields.get(field)
            if not isinstance(value, str) or not value.startswith(prefix):
                continue
            conventional.append(
                {
                    "level": level["identifier"],
                    "source": entity["iid"],
                    "source_kind": ident,
                    "field": field,
                    "spelling": value[len(prefix):],
                    "points_at": points_at,
                    # read off the ROW, never spelled here. One resolver name
                    # at this site is one resolver name for every prefix row,
                    # which is how a phase-script reference came to be reported
                    # as owned by the kinematic-path resolver.
                    "resolution_owned_by": authority,
                    # the least visible shape: the field is called `brain`
                    # and its TYPE is String. Nothing but this table knows a
                    # reference is in there.
                    "shape": f"reference hidden inside `{field}` behind `{prefix}`",
                }
            )

    # --- what is declared and never used -----------------------------------
    # a field every world declares and no world authors is a relationship the
    # docs describe and the content does not have. Measuring it is how you find
    # out that the "first proof" relationship has no instances to prove on.
    for kind, field in sorted(seen_string_fields):
        authored = any(
            row["source_kind"] == kind and row["field"] == field for row in conventional
        )
        if not authored:
            declared_never_authored.append({"entity": kind, "field": field})

    return {
        "native_entity_ref_fields": ref_fields,
        "native": native,
        "conventional": conventional,
        "declared_never_authored": declared_never_authored,
    }


def _tally(rows: list[dict]) -> list[tuple[str, int]]:
    counts: dict[str, int] = {}
    for row in rows:
        key = f"{row['source_kind']}.{row['field']}"
        counts[key] = counts.get(key, 0) + 1
    return sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))


def format_report_text(report: dict, *, world: str, detail: bool = True) -> str:
    out: list[str] = []
    out.append(f"# Authored relationships — {world}\n")

    native = report["native"]
    conv = report["conventional"]
    broken = [row for row in native if row["broken"]]

    out.append("\n## Summary\n\n")
    out.append(f"  native (EntityRef):  {len(native):4d}  ({len(broken)} broken)\n")
    out.append(f"  string convention:   {len(conv):4d}\n")
    for key, count in _tally(native):
        out.append(f"    native   {key:<34s} {count:4d}\n")
    for key, count in _tally(conv):
        out.append(f"    string   {key:<34s} {count:4d}\n")

    # a dangling native ref is the one verdict this tool owns, so it is never
    # hidden behind --detail: it names the offending entity every time.
    if broken:
        out.append("\n  BROKEN native references:\n")
        for row in broken:
            out.append(
                f"    {row['level']}: {row['source_kind']} `{row['source']}`"
                f" --{row['field']}--> missing entity `{row['target_iid']}`\n"
            )

    if not detail:
        out.append("\n(pass --detail, or --level <id>, for every row)\n")
        dead = report["declared_never_authored"]
        out.append(f"\n## Reference conventions with no instances — {len(dead)}\n\n")
        for row in dead:
            out.append(f"  {row['entity']}.{row['field']}\n")
        return "".join(out)

    out.append(f"\n## Native (LDtk EntityRef) — {len(native)} authored\n")
    out.append(
        "\nDiscovered from the project schema. This tool needed no list to find\n"
        "them, and can name a dangling target without knowing what the link means.\n\n"
    )
    if not native:
        out.append("(none)\n")
    for row in native:
        mark = "BROKEN" if row["broken"] else "ok"
        target = row["target_kind"] or "<missing>"
        out.append(
            f"  [{mark}] {row['level']}: {row['source_kind']} `{row['source']}`"
            f" --{row['field']}--> {target} `{row['target_iid']}`\n"
        )

    out.append(f"\n## String conventions — {len(conv)} authored\n")
    out.append(
        "\n⚠ these are plain strings. A tool can only list the ones it was TOLD\n"
        "about, and this one cannot say whether they resolve — the named authority\n"
        "owns that, and re-deriving it here is how three copies drifted before.\n\n"
    )
    if not conv:
        out.append("(none)\n")
    for row in conv:
        out.append(
            f"  {row['level']}: {row['source_kind']} `{row['source']}`"
            f" --{row['field']}--> {row['points_at']} `{row['spelling']}`\n"
            f"      shape: {row['shape']}\n"
            f"      resolution owned by: {row['resolution_owned_by']}\n"
        )

    dead = report["declared_never_authored"]
    out.append(f"\n## Reference conventions with no instances — {len(dead)}\n")
    out.append(
        "\nThe entity is placed here and this field carries no reference on any of\n"
        "them. Either the relationship is authored some OTHER way — which is worth\n"
        "knowing, because two spellings for one relationship is how they drift —\n"
        "or it is vocabulary with no content behind it.\n\n"
    )
    if not dead:
        out.append("(none)\n")
    for row in dead:
        out.append(f"  {row['entity']}.{row['field']}\n")

    return "".join(out)


def file_link(path: Path) -> str:
    """A clickable ``file://`` URI, matching ``scripts/git_debloat.py``."""
    import urllib.parse

    return "file://" + urllib.parse.quote(str(Path(path).expanduser().resolve()))


def write_report(report: dict, out: Path, *, world: str) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    if out.suffix == ".json":
        out.write_text(json.dumps(report, indent=2, sort_keys=True))
    else:
        out.write_text(format_report_text(report, world=world, detail=True))
