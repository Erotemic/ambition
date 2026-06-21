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
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from ambition_ldtk_tools.edit.entity_layer_rules import (
    DEFAULT_LDTK,
    change_layer,
    collect_rule_violations,
    parse_rule,
    write_project,
)

DEFAULT_ENTITY_LAYER_RULES = {"CameraZone": "AmbitionCameras"}
DEFAULT_DISALLOW_ENTITIES_ON_LAYERS = {"Collision", "Water", "Climbable"}


@dataclass(frozen=True)
class PolicyIssue:
    severity: str
    code: str
    level: str | None
    message: str
    fixable: bool = False


def load_project(path: Path) -> dict:
    return json.loads(path.read_text())


def layer_defs(project: dict) -> dict[str, dict]:
    return {str(layer.get("identifier")): layer for layer in project.get("defs", {}).get("layers", []) or []}


def entity_defs(project: dict) -> dict[str, dict]:
    return {str(entity.get("identifier")): entity for entity in project.get("defs", {}).get("entities", []) or []}


def iter_entity_instances(project: dict):
    for level in project.get("levels", []) or []:
        for layer in level.get("layerInstances", []) or []:
            if layer.get("__type") != "Entities":
                continue
            for entity in layer.get("entityInstances") or []:
                yield level, layer, entity


def parse_rules(raw_rules: list[str], include_defaults: bool = True) -> dict[str, str]:
    rules = dict(DEFAULT_ENTITY_LAYER_RULES) if include_defaults else {}
    for raw in raw_rules:
        entity, layer = parse_rule(raw)
        rules[entity] = layer
    return rules


def collect_policy_issues(project: dict, rules: dict[str, str]) -> list[PolicyIssue]:
    issues: list[PolicyIssue] = []
    ldefs = layer_defs(project)
    edefs = entity_defs(project)
    for entity_type, layer_id in rules.items():
        if entity_type not in edefs:
            issues.append(PolicyIssue("error", "missing_entity_def", None, f"entity def {entity_type!r} is missing"))
        if layer_id not in ldefs:
            issues.append(PolicyIssue("error", "missing_layer_def", None, f"layer def {layer_id!r} is missing"))

    for v in collect_rule_violations(project, rules):
        issues.append(PolicyIssue(
            "error",
            "entity_wrong_layer",
            v.level,
            f"{v.level}: {v.identifier} {v.iid} is on {v.layer}, expected {v.expected_layer}",
            fixable=True,
        ))

    for level, layer, entity in iter_entity_instances(project):
        if layer.get("__identifier") in DEFAULT_DISALLOW_ENTITIES_ON_LAYERS:
            issues.append(PolicyIssue(
                "error",
                "entity_on_non_entity_policy_layer",
                level.get("identifier"),
                f"{level.get('identifier')}: {entity.get('__identifier')} {entity.get('iid')} is on {layer.get('__identifier')}",
            ))

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
            issues.append(PolicyIssue(
                "warning",
                "ldtk_tag_rule_not_enforced",
                None,
                f"{entity_type} has tags {sorted(ent_tags)} but target layer {layer_id} does not require one of them",
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


def format_issues(issues: list[PolicyIssue]) -> str:
    if not issues:
        return "LDtk policy check passed.\n"
    lines = ["LDtk policy issues:"]
    for issue in issues:
        suffix = " [fixable]" if issue.fixable else ""
        lines.append(f"  {issue.severity}: {issue.code}: {issue.message}{suffix}")
    return "\n".join(lines) + "\n"


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

    project = load_project(args.ldtk)
    rules = parse_rules(args.rule, include_defaults=not args.no_default_rules)
    if args.action == "fix":
        moved = fix_policy(project, rules)
        if args.in_place:
            write_project(args.ldtk, project)
            out = args.ldtk
        elif args.output:
            write_project(args.output, project)
            out = args.output
        else:
            print("policy fix requires --in-place or --output", flush=True)
            return 64
        print(f"policy fix: moved {moved} entit(y/ies); wrote {out}")

    issues = collect_policy_issues(project, rules)
    if args.format == "json":
        print(json.dumps([issue.__dict__ for issue in issues], indent=2, sort_keys=True))
    else:
        print(format_issues(issues), end="")
    return 1 if any(i.severity == "error" for i in issues) else 0


if __name__ == "__main__":
    raise SystemExit(main())
