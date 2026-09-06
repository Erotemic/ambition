#!/usr/bin/env python3
"""Project policy checks/fixes for LDtk files.

Policy commands are meant for agents and CI: they collect the small set of
rules that prevent generated LDtk edits from drifting away from editor/runtime
conventions. The policy layer is intentionally lightweight and JSON/RON-sidecar
friendly; it does not introduce runtime gameplay semantics.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from ambition_ldtk_tools.edit.visual_manifest import collect_visual_ref_issues
from ambition_ldtk_tools.edit.entity_layer_rules import (
    DEFAULT_LDTK,
    change_layer,
    collect_rule_violations,
    parse_rule,
)
from ambition_ldtk_tools.ldtk import (
    Issue,
    entity_defs as _entity_defs,
    format_issue_lines,
    iter_entities,
    layer_defs as _layer_defs,
    load_project,
    write_project,
)
from ambition_ldtk_tools.ldtk.transaction import LdtkTransaction

DEFAULT_ENTITY_LAYER_RULES = {"CameraZone": "AmbitionCameras"}
DEFAULT_DISALLOW_ENTITIES_ON_LAYERS = {"Collision", "Water", "Climbable"}

PolicyIssue = Issue


def layer_defs(project: dict) -> dict[str, dict]:
    return {str(layer.get("identifier")): layer for layer in _layer_defs(project)}


def entity_defs(project: dict) -> dict[str, dict]:
    return {str(entity.get("identifier")): entity for entity in _entity_defs(project)}


def iter_entity_instances(project: dict):
    for loc in iter_entities(project):
        if loc.layer.get("__type") != "Entities":
            continue
        yield loc.level, loc.layer, loc.entity


def parse_rules(raw_rules: list[str], include_defaults: bool = True) -> dict[str, str]:
    rules = dict(DEFAULT_ENTITY_LAYER_RULES) if include_defaults else {}
    for raw in raw_rules:
        entity, layer = parse_rule(raw)
        rules[entity] = layer
    return rules


def collect_policy_issues(project: dict, rules: dict[str, str]) -> list[Issue]:
    issues: list[Issue] = []
    ldefs = layer_defs(project)
    edefs = entity_defs(project)
    for entity_type, layer_id in rules.items():
        if entity_type not in edefs:
            issues.append(Issue("error", "missing_entity_def", f"entity def {entity_type!r} is missing", entity=entity_type))
        if layer_id not in ldefs:
            issues.append(Issue("error", "missing_layer_def", f"layer def {layer_id!r} is missing", layer=layer_id))

    for v in collect_rule_violations(project, rules):
        issues.append(Issue(
            "error",
            "entity_wrong_layer",
            f"is on {v.layer}, expected {v.expected_layer}",
            level=v.level,
            layer=v.layer,
            entity=v.identifier,
            entity_iid=v.iid,
            fixable=True,
            fix_hint=f"move {v.identifier} to {v.expected_layer}",
            data={"expected_layer": v.expected_layer},
        ))

    for level, layer, entity in iter_entity_instances(project):
        if layer.get("__identifier") in DEFAULT_DISALLOW_ENTITIES_ON_LAYERS:
            issues.append(Issue(
                "error",
                "entity_on_non_entity_policy_layer",
                f"is on {layer.get('__identifier')}",
                level=str(level.get("identifier")),
                layer=str(layer.get("__identifier")),
                entity=str(entity.get("__identifier")),
                entity_iid=str(entity.get("iid")),
            ))

    for issue in collect_visual_ref_issues(project):
        issues.append(issue)

    # Verify LDtk editor tag restrictions agree with the default entity-layer rules
    # when tags have been applied. Missing tags are warnings, not errors, because
    # older files may rely on CI checks only.
    for entity_type, layer_id in rules.items():
        ent = edefs.get(entity_type)
        target = ldefs.get(layer_id)
        if not ent or not target:
            continue
        ent_tags = set(ent.get("tags") or [])
        required = set(target.get("requiredTags") or [])
        if ent_tags and not (ent_tags & required):
            issues.append(Issue(
                "warning",
                "ldtk_tag_rule_not_enforced",
                f"has tags {sorted(ent_tags)} but target layer {layer_id} does not require one of them",
                entity=entity_type,
                layer=layer_id,
                fixable=False,
            ))
    return issues


def fix_policy(project: dict, rules: dict[str, str]) -> int:
    moved = 0
    for entity_type, layer_id in rules.items():
        moved += len(change_layer(
            project,
            level_filter=None,
            from_layer=None,
            to_layer=layer_id,
            iid=None,
            identifier=entity_type,
            field_filters=[],
        ))
    return moved


def format_issues(issues: list[Issue]) -> str:
    return format_issue_lines(issues, title="LDtk policy issues:", empty="LDtk policy check passed.")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="Check/fix LDtk agent authoring policy.")
    ap.add_argument("action", choices=["check", "fix"])
    ap.add_argument("ldtk", type=Path, nargs="?", default=DEFAULT_LDTK)
    ap.add_argument("--rule", action="append", default=[], help="Entity=Layer rule; repeatable.")
    ap.add_argument("--no-default-rules", action="store_true")
    ap.add_argument("--format", choices=["text", "json"], default="text")
    ap.add_argument("--in-place", action="store_true")
    ap.add_argument("--output", type=Path)
    args = ap.parse_args(argv)

    rules = parse_rules(args.rule, include_defaults=not args.no_default_rules)
    if args.action == "fix":
        tx = LdtkTransaction(
            args.ldtk,
            in_place=args.in_place,
            output=args.output,
        )
        moved = fix_policy(tx.project, rules)
        if moved:
            tx.note_changed([f"policy fix moved {moved} entit(y/ies)"])
        out = tx.finish(
            noop_message="policy fix: no entities needed moving",
            write_message=f"policy fix: moved {moved} entit(y/ies); wrote {{path}}",
        )
        if out is None and not moved:
            pass
    else:
        project = load_project(args.ldtk)

    if args.action == "fix":
        project = tx.project
    issues = collect_policy_issues(project, rules)
    if args.format == "json":
        print(json.dumps([issue.as_dict() for issue in issues], indent=2, sort_keys=True))
    else:
        print(format_issues(issues), end="")
    return 1 if any(i.severity == "error" for i in issues) else 0


if __name__ == "__main__":
    raise SystemExit(main())
