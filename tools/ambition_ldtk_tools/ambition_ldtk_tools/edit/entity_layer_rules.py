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
import sys
from dataclasses import dataclass
from pathlib import Path

from ambition_ldtk_tools.ldtk import (
    ApplyEntityLayerTagRule,
    EntityLocation,
    LdtkTransaction,
    MoveEntitiesToLayer,
    default_sandbox_ldtk,
    ensure_entities_layer_def,
    ensure_entities_layer_instance,
    entity_field_value,
    iter_entities,
    load_project,
    write_project,
)

DEFAULT_LDTK = default_sandbox_ldtk()

DEFAULT_RULES = {"CameraZone": "AmbitionCameras"}


@dataclass(frozen=True)
class RuleViolation:
    level: str
    layer: str
    identifier: str
    iid: str
    expected_layer: str



def run_repair(path: Path) -> int:
    # Compatibility shim: write_project already canonicalizes editor metadata.
    # Do not shell out to repair here because repair also validates LoadingZone
    # targets, and sandbox worlds may intentionally link to other LDtk files.
    print(f"note: wrote canonical editor-style JSON; skipped full repair validation for {path}")
    return 0


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
    """Move selected entities through the shared LDtk patch op.

    Kept as a public helper for existing tests and callers; new commands should
    build `MoveEntitiesToLayer` directly or use `LdtkTransaction.apply`.
    """
    return MoveEntitiesToLayer(
        to_layer=to_layer,
        level_filter=level_filter,
        from_layer=from_layer,
        iid=iid,
        identifier=identifier,
        field_filters=field_filters,
    ).apply(project).messages


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


def apply_editor_layer_rule(
    project: dict,
    *,
    entity_type: str,
    to_layer: str,
    from_layer: str | None,
    tag: str,
) -> list[str]:
    """Apply editor placement metadata through the shared patch op."""
    return ApplyEntityLayerTagRule(
        entity_type=entity_type,
        to_layer=to_layer,
        from_layer=from_layer,
        tag=tag,
    ).apply(project).messages


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


def _make_transaction(args) -> LdtkTransaction:
    return LdtkTransaction(
        args.ldtk,
        dry_run=args.dry_run,
        in_place=args.in_place,
        output=args.output,
        backup=args.backup,
    )


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
    ap.add_argument("--no-repair", action="store_true", help="compatibility flag; writes already skip full repair validation")
    args = ap.parse_args(argv)
    if args.dry_run and (args.in_place or args.output):
        ap.error("--dry-run cannot be combined with --in-place/--output")
    if not args.dry_run and not args.in_place and args.output is None:
        ap.error("choose --dry-run, --in-place, or --output <path>")

    tx = _make_transaction(args)
    filters = [parse_field_filter(raw) for raw in args.field]
    result = tx.apply(
        MoveEntitiesToLayer(
            to_layer=args.to_layer,
            level_filter=args.level,
            from_layer=args.from_layer,
            iid=args.iid,
            identifier=args.identifier,
            field_filters=filters,
        )
    )
    if not result.messages:
        print("change-layer: no matching entities")
    else:
        action = "would move" if args.dry_run else "moved"
        print(f"change-layer: {action} {len(result.messages)} entit(y/ies)")
        for line in result.messages:
            print(f"  {line}")
    tx.finish(noop_message="change-layer: left file unchanged")
    return 0


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
    ap.add_argument("--no-repair", action="store_true", help="compatibility flag; writes already skip full repair validation")
    args = ap.parse_args(argv)
    if args.dry_run and (args.in_place or args.output):
        ap.error("--dry-run cannot be combined with --in-place/--output")
    if not args.dry_run and not args.in_place and args.output is None:
        ap.error("choose --dry-run, --in-place, or --output <path>")
    tx = _make_transaction(args)
    result = tx.apply(
        ApplyEntityLayerTagRule(
            entity_type=args.entity_type,
            to_layer=args.to_layer,
            from_layer=args.from_layer,
            tag=args.tag or args.entity_type,
        )
    )
    if result.messages:
        print("apply-entity-rules: " + ("would change:" if args.dry_run else "changed:"))
        for line in result.messages:
            print(f"  {line}")
    else:
        print("apply-entity-rules: no changes needed")
    tx.finish()
    return 0


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
