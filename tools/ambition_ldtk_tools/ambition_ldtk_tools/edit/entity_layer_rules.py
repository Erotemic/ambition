#!/usr/bin/env python3
"""Entity layer relocation and placement-rule helpers.

These commands keep large editor-only zones (CameraZone, combat volumes,
large debug overlays, etc.) out of the catch-all Ambition layer and give CI / agents
a way to catch future mistakes.

Common workflows:

  # Move all CameraZone instances currently on Ambition into AmbitionCameras.
  python -m ambition_ldtk_tools entity change-layer <ldtk> \
      --identifier CameraZone --from-layer Ambition --to-layer AmbitionCameras \
      --in-place

  # Move a single known entity instance.
  python -m ambition_ldtk_tools entity change-layer <ldtk> \
      --iid CameraZone-123 --to-layer AmbitionCameras --in-place

  # Ask LDtk itself to prevent future CameraZone placement on Ambition.
  python -m ambition_ldtk_tools layer apply-entity-rules <ldtk> \
      --type CameraZone --to-layer AmbitionCameras --from-layer Ambition \
      --tag Camera --in-place

  # CI / agent preflight: fail if CameraZone is on the wrong layer.
  python -m ambition_ldtk_tools layer check-entity-rules <ldtk> \
      --rule CameraZone=AmbitionCameras
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[4]
DEFAULT_LDTK = (
    REPO_ROOT
    / "crates"
    / "ambition_gameplay_core"
    / "assets"
    / "ambition"
    / "worlds"
    / "sandbox.ldtk"
)

DEFAULT_RULES = {"CameraZone": "AmbitionCameras"}


@dataclass(frozen=True)
class EntityLocation:
    level: dict
    layer: dict
    entity: dict


@dataclass(frozen=True)
class RuleViolation:
    level: str
    layer: str
    identifier: str
    iid: str
    expected_layer: str


def load_project(path: Path) -> dict:
    return json.loads(path.read_text())


def write_project(path: Path, project: dict) -> None:
    path.write_text(json.dumps(project, indent=2))


def run_repair(path: Path) -> int:
    cmd = [sys.executable, "-m", "ambition_ldtk_tools.repair", str(path), "--in-place"]
    print("$ " + " ".join(cmd))
    return subprocess.run(cmd).returncode


def find_layer_def(project: dict, identifier: str) -> dict | None:
    for layer_def in project.get("defs", {}).get("layers", []) or []:
        if layer_def.get("identifier") == identifier:
            return layer_def
    return None


def find_entity_def(project: dict, identifier: str) -> dict | None:
    for entity_def in project.get("defs", {}).get("entities", []) or []:
        if entity_def.get("identifier") == identifier:
            return entity_def
    return None


def alloc_uid(project: dict) -> int:
    next_uid = int(project.get("nextUid", 1))
    project["nextUid"] = next_uid + 1
    return next_uid


def ensure_entities_layer_def(project: dict, identifier: str, clone_from: str | None = None) -> dict:
    existing = find_layer_def(project, identifier)
    if existing is not None:
        if existing.get("__type") != "Entities" and existing.get("type") != "Entities":
            raise SystemExit(
                f"layer {identifier!r} exists but is not an Entities layer; "
                "refusing to use it as an entity destination"
            )
        return existing

    template = find_layer_def(project, clone_from or "Ambition")
    if template is None:
        for layer_def in project.get("defs", {}).get("layers", []) or []:
            if layer_def.get("__type") == "Entities" or layer_def.get("type") == "Entities":
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


def find_layer_instance(level: dict, identifier: str) -> dict | None:
    for layer in level.get("layerInstances", []) or []:
        if layer.get("__identifier") == identifier:
            return layer
    return None


def ensure_entities_layer_instance(
    project: dict,
    level: dict,
    identifier: str,
    *,
    dest_def: dict,
    clone_from: str | None = None,
) -> dict:
    existing = find_layer_instance(level, identifier)
    if existing is not None:
        return existing

    template = find_layer_instance(level, clone_from or "Ambition")
    if template is None:
        for layer in level.get("layerInstances", []) or []:
            if layer.get("__type") == "Entities" or layer.get("__identifier"):
                if "entityInstances" in layer:
                    template = layer
                    break
    if template is None:
        raise SystemExit(
            f"level {level.get('identifier')!r} has no Entities layer instance to clone"
        )

    new_layer = dict(template)
    new_layer["__identifier"] = identifier
    new_layer["layerDefUid"] = dest_def.get("uid")
    new_layer["iid"] = f"{identifier}-{alloc_uid(project)}"
    new_layer["entityInstances"] = []
    layers = level.setdefault("layerInstances", [])
    try:
        idx = layers.index(template)
        layers.insert(idx, new_layer)
    except ValueError:
        layers.append(new_layer)
    return new_layer


def entity_field_value(entity: dict, name: str):
    for field in entity.get("fieldInstances", []) or []:
        if field.get("__identifier") == name:
            return field.get("__value")
    return None


def parse_field_filter(raw: str) -> tuple[str, str]:
    if "=" not in raw:
        raise SystemExit(f"--field expects NAME=VALUE; got {raw!r}")
    name, _, value = raw.partition("=")
    if not name.strip():
        raise SystemExit(f"--field {raw!r} has an empty field name")
    return name.strip(), value.strip()


def parse_rule(raw: str) -> tuple[str, str]:
    if "=" not in raw:
        raise SystemExit(f"--rule expects EntityIdentifier=LayerIdentifier; got {raw!r}")
    entity, _, layer = raw.partition("=")
    entity = entity.strip()
    layer = layer.strip()
    if not entity or not layer:
        raise SystemExit(f"--rule {raw!r} must name both entity and layer")
    return entity, layer


def parse_rules(raw_rules: list[str], use_defaults: bool) -> dict[str, str]:
    rules: dict[str, str] = dict(DEFAULT_RULES) if use_defaults else {}
    for raw in raw_rules:
        entity, layer = parse_rule(raw)
        rules[entity] = layer
    return rules


def matches_filters(
    entity: dict,
    *,
    iid: str | None,
    identifier: str | None,
    field_filters: list[tuple[str, str]],
) -> bool:
    if iid is not None and entity.get("iid") != iid:
        return False
    if identifier is not None and entity.get("__identifier") != identifier:
        return False
    for name, expected in field_filters:
        value = entity_field_value(entity, name)
        if isinstance(value, str):
            if value != expected:
                return False
        elif str(value) != expected:
            return False
    return True


def iter_entities(
    project: dict,
    *,
    level_filter: str | None = None,
    layer_filter: str | None = None,
):
    for level in project.get("levels", []) or []:
        if level_filter and level.get("identifier") != level_filter:
            continue
        for layer in level.get("layerInstances", []) or []:
            if layer_filter and layer.get("__identifier") != layer_filter:
                continue
            for entity in layer.get("entityInstances", []) or []:
                yield EntityLocation(level=level, layer=layer, entity=entity)


def change_layer(
    project: dict,
    *,
    level_filter: str | None,
    from_layer: str | None,
    to_layer: str,
    iid: str | None,
    identifier: str | None,
    field_filters: list[tuple[str, str]],
) -> list[str]:
    if iid is None and identifier is None:
        raise SystemExit("select entities with --iid or --identifier")

    dest_def = ensure_entities_layer_def(project, to_layer, clone_from=from_layer)
    selected: list[EntityLocation] = []
    for loc in iter_entities(project, level_filter=level_filter, layer_filter=from_layer):
        if loc.layer.get("__identifier") == to_layer:
            continue
        if matches_filters(
            loc.entity,
            iid=iid,
            identifier=identifier,
            field_filters=field_filters,
        ):
            selected.append(loc)

    by_level_layer: dict[tuple[str, str], list[dict]] = {}
    for loc in selected:
        level_id = str(loc.level.get("identifier"))
        layer_id = str(loc.layer.get("__identifier"))
        by_level_layer.setdefault((level_id, layer_id), []).append(loc.entity)

    for (level_id, layer_id), entities in by_level_layer.items():
        level = next(level for level in project.get("levels", []) if level.get("identifier") == level_id)
        source = find_layer_instance(level, layer_id)
        dest = ensure_entities_layer_instance(
            project,
            level,
            to_layer,
            dest_def=dest_def,
            clone_from=from_layer or layer_id,
        )
        remaining = []
        move_ids = {id(entity) for entity in entities}
        for entity in source.get("entityInstances", []) or []:
            if id(entity) in move_ids:
                dest.setdefault("entityInstances", []).append(entity)
            else:
                remaining.append(entity)
        source["entityInstances"] = remaining

    return [
        f"{loc.level.get('identifier')}:{loc.layer.get('__identifier')} -> {to_layer} "
        f"{loc.entity.get('__identifier')} ({loc.entity.get('iid')})"
        for loc in selected
    ]


def collect_rule_violations(project: dict, rules: dict[str, str]) -> list[RuleViolation]:
    violations: list[RuleViolation] = []
    for loc in iter_entities(project):
        identifier = str(loc.entity.get("__identifier"))
        expected = rules.get(identifier)
        if expected is None:
            continue
        actual_layer = str(loc.layer.get("__identifier"))
        if actual_layer != expected:
            violations.append(
                RuleViolation(
                    level=str(loc.level.get("identifier")),
                    layer=actual_layer,
                    identifier=identifier,
                    iid=str(loc.entity.get("iid")),
                    expected_layer=expected,
                )
            )
    return violations


def add_unique(values: list, value) -> None:
    if value not in values:
        values.append(value)


def remove_value(values: list, value) -> None:
    while value in values:
        values.remove(value)


def apply_editor_layer_rule(
    project: dict,
    *,
    entity_type: str,
    to_layer: str,
    from_layer: str | None,
    tag: str,
) -> list[str]:
    entity_def = find_entity_def(project, entity_type)
    if entity_def is None:
        raise SystemExit(f"entity definition {entity_type!r} not found")
    dest_def = ensure_entities_layer_def(project, to_layer, clone_from=from_layer)
    source_def = find_layer_def(project, from_layer) if from_layer else None

    changes: list[str] = []
    tags = entity_def.setdefault("tags", [])
    if tag not in tags:
        tags.append(tag)
        changes.append(f"entity {entity_type}: added tag {tag!r}")

    required = dest_def.setdefault("requiredTags", [])
    if tag not in required:
        required.append(tag)
        changes.append(f"layer {to_layer}: requiredTags += {tag!r}")

    excluded = dest_def.setdefault("excludedTags", [])
    if tag in excluded:
        remove_value(excluded, tag)
        changes.append(f"layer {to_layer}: excludedTags -= {tag!r}")

    if source_def is not None:
        excluded = source_def.setdefault("excludedTags", [])
        if tag not in excluded:
            excluded.append(tag)
            changes.append(f"layer {from_layer}: excludedTags += {tag!r}")
        required = source_def.setdefault("requiredTags", [])
        if tag in required:
            remove_value(required, tag)
            changes.append(f"layer {from_layer}: requiredTags -= {tag!r}")
    return changes


def emit_rule_report(violations: list[RuleViolation], *, json_output: bool) -> None:
    if json_output:
        print(
            json.dumps(
                [
                    {
                        "level": v.level,
                        "layer": v.layer,
                        "identifier": v.identifier,
                        "iid": v.iid,
                        "expected_layer": v.expected_layer,
                    }
                    for v in violations
                ],
                indent=2,
            )
        )
        return
    if not violations:
        print("entity layer rules: ok")
        return
    print(f"entity layer rules: {len(violations)} violation(s)")
    for v in violations:
        print(
            f"  {v.level}: {v.identifier} ({v.iid}) is on {v.layer}; "
            f"expected {v.expected_layer}"
        )


def maybe_write(
    *,
    project: dict,
    source: Path,
    output: Path | None,
    in_place: bool,
    backup: bool,
    no_repair: bool,
) -> int:
    target = source if in_place else output
    if target is None:
        raise SystemExit("choose --in-place or --output <path>, or use --dry-run")
    if backup and in_place:
        backup_path = source.with_suffix(source.suffix + ".bak")
        shutil.copy2(source, backup_path)
        print(f"backup written: {backup_path}")
    write_project(target, project)
    if no_repair:
        return 0
    return run_repair(target)


def cmd_change_layer(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="Move selected entities to another Entities layer")
    ap.add_argument("ldtk", type=Path, nargs="?", default=DEFAULT_LDTK)
    ap.add_argument("--level", default=None)
    ap.add_argument("--from-layer", default=None, help="optional source layer filter")
    ap.add_argument("--to-layer", required=True)
    ap.add_argument("--iid", default=None)
    ap.add_argument("--identifier", default=None)
    ap.add_argument("--field", action="append", default=[])
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--in-place", action="store_true")
    ap.add_argument("--output", type=Path, default=None)
    ap.add_argument("--backup", action="store_true")
    ap.add_argument("--no-repair", action="store_true")
    args = ap.parse_args(argv)
    if args.dry_run and (args.in_place or args.output):
        ap.error("--dry-run cannot be combined with --in-place/--output")
    if not args.dry_run and not args.in_place and args.output is None:
        ap.error("choose --dry-run, --in-place, or --output <path>")

    project = load_project(args.ldtk)
    filters = [parse_field_filter(raw) for raw in args.field]
    moved = change_layer(
        project,
        level_filter=args.level,
        from_layer=args.from_layer,
        to_layer=args.to_layer,
        iid=args.iid,
        identifier=args.identifier,
        field_filters=filters,
    )
    if not moved:
        print("change-layer: no matching entities")
    else:
        print(f"change-layer: would move {len(moved)} entit(y/ies)" if args.dry_run else f"change-layer: moved {len(moved)} entit(y/ies)")
        for line in moved:
            print(f"  {line}")
    if args.dry_run:
        return 0
    return maybe_write(
        project=project,
        source=args.ldtk,
        output=args.output,
        in_place=args.in_place,
        backup=args.backup,
        no_repair=args.no_repair,
    )


def cmd_check(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="Check entity/layer placement policies")
    ap.add_argument("ldtk", type=Path, nargs="?", default=DEFAULT_LDTK)
    ap.add_argument("--rule", action="append", default=[])
    ap.add_argument("--no-defaults", action="store_true")
    ap.add_argument("--format", choices=["text", "json"], default="text")
    args = ap.parse_args(argv)
    project = load_project(args.ldtk)
    rules = parse_rules(args.rule, use_defaults=not args.no_defaults)
    if not rules:
        raise SystemExit("no rules configured; pass --rule Entity=Layer or omit --no-defaults")
    violations = collect_rule_violations(project, rules)
    emit_rule_report(violations, json_output=args.format == "json")
    return 1 if violations else 0


def cmd_apply_rules(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="Apply LDtk tag-based entity/layer placement rules")
    ap.add_argument("ldtk", type=Path, nargs="?", default=DEFAULT_LDTK)
    ap.add_argument("--type", dest="entity_type", required=True)
    ap.add_argument("--to-layer", required=True)
    ap.add_argument("--from-layer", default="Ambition")
    ap.add_argument("--tag", default=None, help="entity tag to enforce; default: entity type")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--in-place", action="store_true")
    ap.add_argument("--output", type=Path, default=None)
    ap.add_argument("--backup", action="store_true")
    ap.add_argument("--no-repair", action="store_true")
    args = ap.parse_args(argv)
    if args.dry_run and (args.in_place or args.output):
        ap.error("--dry-run cannot be combined with --in-place/--output")
    if not args.dry_run and not args.in_place and args.output is None:
        ap.error("choose --dry-run, --in-place, or --output <path>")
    project = load_project(args.ldtk)
    tag = args.tag or args.entity_type
    changes = apply_editor_layer_rule(
        project,
        entity_type=args.entity_type,
        to_layer=args.to_layer,
        from_layer=args.from_layer,
        tag=tag,
    )
    if changes:
        print("apply-entity-rules: " + ("would change:" if args.dry_run else "changed:"))
        for line in changes:
            print(f"  {line}")
    else:
        print("apply-entity-rules: no changes needed")
    if args.dry_run:
        return 0
    return maybe_write(
        project=project,
        source=args.ldtk,
        output=args.output,
        in_place=args.in_place,
        backup=args.backup,
        no_repair=args.no_repair,
    )


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv:
        print(__doc__)
        return 0
    action = argv.pop(0)
    if action == "change-layer":
        return cmd_change_layer(argv)
    if action == "check-entity-rules":
        return cmd_check(argv)
    if action == "apply-entity-rules":
        return cmd_apply_rules(argv)
    raise SystemExit(f"unknown entity/layer rule action: {action}")


if __name__ == "__main__":
    raise SystemExit(main())
