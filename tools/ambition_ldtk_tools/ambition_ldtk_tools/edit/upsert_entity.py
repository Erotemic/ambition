#!/usr/bin/env python3
"""Reconcile an LDtk project's entity definitions with a declared manifest.

Existing entity and field UIDs are preserved so authored instances keep valid
references. Keys outside the manifest's vocabulary contract, such as editor
placement and tile presentation metadata, are retained. New identifiers are
created using the same rules as `def register-entity`.

Removing a field or changing its type may destroy instance values. The command
refuses that operation when non-null values exist unless the exact
`ENTITY.FIELD` is authorized with `--drop-instance-values`; stale or unnecessary
authorizations are rejected. Fields with no retained values may be retired
without an override.

The manifest uses the same `{"entities": [...]}` vocabulary consumed by the
entity-definition tooling and validation."""

from __future__ import annotations

import argparse
import shutil
import sys
from dataclasses import dataclass
from pathlib import Path

from ambition_ldtk_tools.edit.defs import (  # noqa: E402
    HUMAN_TO_INTERNAL,
    build_entity_def,
    resolve_field_type,
    field_def as _new_field_def,
    load_project,
    load_spec,
    patch_validator_known_entities,
    repair_and_validate,
    write_project,
    _default_override,
)
from ambition_ldtk_tools.editor_format import editor_safe_string  # noqa: E402
from ambition_ldtk_tools.ldtk.paths import default_sandbox_ldtk  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[4]
SANDBOX_LDTK = default_sandbox_ldtk()
DEFAULT_SCHEMA = (
    REPO_ROOT / "tools" / "ambition_ldtk_tools" / "schemas" / "ldtk" / "JSON_SCHEMA.json"
)


@dataclass(frozen=True)
class ValueLoss:
    """Instance values a proposed definition change would destroy.

    `path` is the `ENTITY.FIELD` spelling `--drop-instance-values` takes, so
    the report can hand the operator the exact text that authorizes it.
    """

    path: str
    reason: str
    count: int
    values: tuple


def find_entity_def(project: dict, identifier: str) -> dict | None:
    for ent in project.get("defs", {}).get("entities", []) or []:
        if ent.get("identifier") == identifier:
            return ent
    return None


def _spec_fields(spec_entity: dict) -> list[dict]:
    fields = spec_entity.get("fields") or []
    seen = set()
    for field in fields:
        name = field.get("name")
        if not name:
            raise SystemExit(
                f"entity '{spec_entity.get('identifier')}' has a field with no 'name'"
            )
        if name in seen:
            raise SystemExit(
                f"entity '{spec_entity.get('identifier')}' declares field "
                f"'{name}' twice; one of them would be silently discarded"
            )
        seen.add(name)
        human = field.get("type")
        if human not in HUMAN_TO_INTERNAL:
            raise SystemExit(
                f"entity '{spec_entity.get('identifier')}' field '{name}' has "
                f"unsupported type {human!r}; supported: {sorted(HUMAN_TO_INTERNAL)}"
            )
    return list(fields)


def _stale_field_names(project: dict, spec_entity: dict) -> dict[str, str]:
    """Field identifiers whose `fieldInstance` records cannot survive the spec.

    Maps the identifier to the human-readable reason, which the loss report
    prints verbatim.
    """
    ent = find_entity_def(project, spec_entity["identifier"])
    if ent is None:
        return {}
    declared = {f["name"]: f for f in _spec_fields(spec_entity)}
    stale: dict[str, str] = {}
    for field_def in ent.get("fieldDefs") or []:
        name = field_def.get("identifier")
        if name not in declared:
            stale[name] = "the manifest no longer declares it"
            continue
        spec_field = declared[name]
        was = field_def.get("__type")
        now = (
            f"LocalEnum.{spec_field.get('enum')}"
            if spec_field["type"] == "Enum"
            else spec_field["type"]
        )
        if was == now:
            continue
        # The values are unchanged strings either way — what changes is that the editor offers a
        # dropdown instead of a text box. A value the enum does NOT spell is a real loss and still
        # stops the tool, naming it.
        if spec_field["type"] == "Enum" and was == "String":
            allowed = {str(value) for value in spec_field.get("values") or []}
            authored = {
                str(value)
                for value in _instance_values(project, spec_entity["identifier"], name)
            }
            missing = sorted(authored - allowed)
            if not missing:
                continue
            stale[name] = (
                f"the enum {spec_field.get('enum')} does not spell "
                + ", ".join(repr(value) for value in missing)
            )
            continue
        stale[name] = f"its type changes from {was} to {now}"
    return stale


def _instance_values(project: dict, identifier: str, field_name: str) -> list:
    """Every non-null value instances of `identifier` carry for `field_name`."""
    values = []
    for level in project.get("levels") or []:
        for layer in level.get("layerInstances") or []:
            for inst in layer.get("entityInstances") or []:
                if inst.get("__identifier") != identifier:
                    continue
                for field in inst.get("fieldInstances") or []:
                    if field.get("__identifier") != field_name:
                        continue
                    if field.get("__value") is not None:
                        values.append(field["__value"])
    return values


def plan_losses(project: dict, entities: list[dict]) -> list[ValueLoss]:
    """What applying `entities` to `project` would destroy, and nothing else.

    Creating an identifier loses nothing, and retiring a field no instance ever
    set loses nothing either — the gate is on data actually at risk, not on how
    alarming the diff looks.
    """
    losses: list[ValueLoss] = []
    for spec_entity in entities:
        identifier = spec_entity["identifier"]
        for name, reason in _stale_field_names(project, spec_entity).items():
            values = _instance_values(project, identifier, name)
            if not values:
                continue
            distinct = sorted({repr(v) for v in values})
            losses.append(
                ValueLoss(
                    path=f"{identifier}.{name}",
                    reason=reason,
                    count=len(values),
                    values=tuple(distinct),
                )
            )
    return losses


def format_losses(losses: list[ValueLoss]) -> str:
    lines = [
        "this would destroy authored instance values:",
        "",
    ]
    for loss in losses:
        lines.append(f"  {loss.path} — {loss.reason}")
        lines.append(
            f"    {loss.count} instance(s) carry a value; "
            f"distinct: {', '.join(loss.values)}"
        )
    lines.append("")
    lines.append("If losing those is the intent, name each one:")
    lines.append(
        "  " + " ".join(f"--drop-instance-values {loss.path}" for loss in losses)
    )
    return "\n".join(lines)


def _retype_field_instances(
    project: dict, identifier: str, field_name: str, human: str
) -> int:
    """Point every instance record of one field at the def's new type."""
    retyped = 0
    for level in project.get("levels") or []:
        for layer in level.get("layerInstances") or []:
            for inst in layer.get("entityInstances") or []:
                if inst.get("__identifier") != identifier:
                    continue
                for field in inst.get("fieldInstances") or []:
                    if field.get("__identifier") != field_name:
                        continue
                    if field.get("__type") != human:
                        field["__type"] = human
                        retyped += 1
    return retyped


def _forget_field_instances(
    project: dict, identifier: str, field_names: set[str]
) -> int:
    if not field_names:
        return 0
    forgotten = 0
    for level in project.get("levels") or []:
        for layer in level.get("layerInstances") or []:
            for inst in layer.get("entityInstances") or []:
                if inst.get("__identifier") != identifier:
                    continue
                kept = [
                    f
                    for f in inst.get("fieldInstances") or []
                    if f.get("__identifier") not in field_names
                ]
                forgotten += len(inst.get("fieldInstances") or []) - len(kept)
                inst["fieldInstances"] = kept
    return forgotten


# The manifest keys that map onto an entity definition, and the definition key
# each one writes. A key the manifest omits is left as the author had it — see
# the module docstring on why silence is not "reset to the default".
PRESENTATION_KEYS = (
    ("color", "color"),
    ("width", "width"),
    ("height", "height"),
    ("docs", "doc"),
)


def _as_stored(value):
    """The value the `.ldtk` will actually hold once written.

    Comparing the manifest's raw text against the project would report a
    difference forever for any multi-line doc string, because the editor's
    serializer collapses newlines on the way out. Reconcile against what the
    file can store, not against what the manifest wished it could.
    """
    return editor_safe_string(value) if isinstance(value, str) else value


def apply_upsert(project: dict, entities: list[dict]) -> list[str]:
    """Reconcile `project`'s entity defs with `entities`; return what changed.

    The caller is responsible for having resolved `plan_losses` first — this
    applies the spec unconditionally, because a function that both decides
    policy and executes it cannot be probed against a policy it rejects.
    """
    changes: list[str] = []
    created: list[str] = []
    for spec_entity in entities:
        identifier = spec_entity["identifier"]
        fields = _spec_fields(spec_entity)
        ent = find_entity_def(project, identifier)
        if ent is None:
            ent = build_entity_def(spec_entity, project)
            project["defs"].setdefault("entities", []).append(ent)
            created.append(identifier)
            changes.append(f"{identifier}: created entity def (uid={ent['uid']})")
            continue

        stale = set(_stale_field_names(project, spec_entity))

        for spec_key, def_key in PRESENTATION_KEYS:
            if spec_key not in spec_entity:
                continue
            value = _as_stored(spec_entity[spec_key])
            if def_key in {"width", "height"}:
                value = int(value)
            if ent.get(def_key) != value:
                changes.append(f"{identifier}.{def_key}: {ent.get(def_key)!r} -> {value!r}")
                ent[def_key] = value

        existing = {f.get("identifier"): f for f in ent.get("fieldDefs") or []}
        ordered = []
        for field in fields:
            name, human, default = field["name"], field["type"], field.get("default")
            old = existing.pop(name, None)
            if old is None:
                fresh = _new_field_def(name, human, default, project, field)
                ordered.append(fresh)
                changes.append(
                    f"{identifier}.{name}: added {human} field (uid={fresh['uid']})"
                )
                continue
            # The uid is never touched: it is what every fieldInstance in every
            # level points at.
            human, internal = resolve_field_type(project, field)
            if old.get("__type") != human or old.get("type") != internal:
                changes.append(
                    f"{identifier}.{name}: retyped {old.get('__type')} -> {human}"
                )
                old["__type"] = human
                old["type"] = internal
                # every INSTANCE carries the type too, and LDtk reads it. A
                # def that says `LocalEnum.X` over instances still saying
                # `String` is the mismatch that makes the editor drop values.
                retyped = _retype_field_instances(project, identifier, name, human)
                if retyped:
                    changes.append(
                        f"{identifier}.{name}: retyped {retyped} field instance(s)"
                    )
            if "docs" in field and old.get("doc") != _as_stored(field["docs"]):
                old["doc"] = _as_stored(field["docs"])
                changes.append(f"{identifier}.{name}: doc updated")
            wanted_default = _default_override(human, _as_stored(default))
            if old.get("defaultOverride") != wanted_default:
                changes.append(
                    f"{identifier}.{name}: default "
                    f"{old.get('defaultOverride')!r} -> {wanted_default!r}"
                )
                old["defaultOverride"] = wanted_default
            ordered.append(old)

        for name, retired in existing.items():
            changes.append(
                f"{identifier}.{name}: retired field def (uid={retired.get('uid')})"
            )
        ent["fieldDefs"] = ordered

        forgotten = _forget_field_instances(project, identifier, stale)
        if forgotten:
            changes.append(
                f"{identifier}: forgot {forgotten} field instance record(s) for "
                f"{', '.join(sorted(stale))}"
            )

    if created:
        changes.append(f"created: {', '.join(created)}")
    return changes


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("spec", type=Path, help="YAML or JSON spec with `entities` list")
    parser.add_argument("--ldtk", type=Path, default=SANDBOX_LDTK)
    parser.add_argument("--in-place", action="store_true", help="write to --ldtk")
    parser.add_argument("--output", type=Path, default=None)
    parser.add_argument("--backup", action="store_true")
    parser.add_argument(
        "--game-owned",
        action="store_true",
        help=(
            "this entity belongs to a GAME, not the engine: register it in the "
            "project only, and leave the engine's vocabulary lists alone"
        ),
    )
    parser.add_argument(
        "--drop-instance-values",
        action="append",
        default=[],
        metavar="ENTITY.FIELD",
        help=(
            "Authorize destroying the values instances carry for this field. "
            "Repeat per field. Refused when nothing at that path would be lost, "
            "so an opt-in cannot outlive the migration it was written for."
        ),
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="report the changes and any refusals; write nothing",
    )
    parser.add_argument(
        "--no-repair", action="store_true", help="skip repair + validate post-pass"
    )
    parser.add_argument(
        "--no-source-patch",
        action="store_true",
        help="skip patching validator + runtime source files for CREATED entities",
    )
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    args = parser.parse_args(argv)

    if not args.dry_run and not args.in_place and args.output is None:
        parser.error("choose --in-place, --output <path>, or --dry-run")
    if not args.ldtk.exists():
        return _fail(f"ldtk file not found: {args.ldtk}")

    spec = load_spec(args.spec)
    if not isinstance(spec, dict) or "entities" not in spec:
        return _fail("spec must be a mapping with an `entities` list")
    entities = spec["entities"]

    project = load_project(args.ldtk)
    losses = plan_losses(project, entities)
    authorized = set(args.drop_instance_values)
    at_risk = {loss.path for loss in losses}

    stale_optin = sorted(authorized - at_risk)
    if stale_optin:
        return _fail(
            "--drop-instance-values names "
            + ", ".join(stale_optin)
            + ", where no instance value would be lost. An opt-in that no longer "
            "describes the change is how a one-time migration turns into a "
            "standing licence to delete; drop the flag."
        )

    unauthorized = [loss for loss in losses if loss.path not in authorized]
    if unauthorized:
        return _fail(format_losses(unauthorized))

    known_before = {e["identifier"] for e in project["defs"].get("entities") or []}
    changes = apply_upsert(project, entities)
    if not changes:
        print("entity definitions already match the spec — nothing to do")
        if args.dry_run:
            return 0
    for change in changes:
        print(f"  - {change}")

    if args.dry_run:
        print("(--dry-run: nothing written)")
        return 0

    created = [
        e["identifier"] for e in entities if e["identifier"] not in known_before
    ]

    target = args.output or args.ldtk
    if args.in_place and args.backup:
        backup = args.ldtk.with_suffix(args.ldtk.suffix + ".bak")
        shutil.copy2(args.ldtk, backup)
        print(f"wrote backup: {backup}")
    write_project(target, project)
    print(f"wrote {target}")

    # Only a CREATED identifier can owe the engine's vocabulary lists anything;
    # an upsert of an existing noun has nothing to add to them. The `--game-owned`
    # reasoning is `register-entity`'s and unchanged: a game's noun rides
    # `install_ldtk_entity_converters`, never the engine's lists.
    if created and not args.no_source_patch and not args.game_owned:
        patch_validator_known_entities(created)

    if args.no_repair:
        return 0
    return repair_and_validate(target, args.schema)


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
