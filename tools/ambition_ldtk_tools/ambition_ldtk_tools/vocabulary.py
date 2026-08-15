#!/usr/bin/env python3
"""**What can I place here, and what may I say about it?**

The two questions an author asks first, and the two the toolbox could not
answer. `entity query` lists what a level ALREADY holds; `room describe`
summarises one room; `def register-entity` / `def upsert-entity` WRITE
definitions. Nothing read the vocabulary back out, so discovering that
`MaryOBlock.kind` is a closed enum, that `PickupSpawn.kind` is a
colon-separated string with a shape, or that `EnemySpawn.character_id` even
EXISTS meant opening the `.ldtk` and reading `defs.entities` by hand.

⛔ **AND A TYPED FIELD DEF IS NOT THE ANSWER EITHER, WHICH IS THE POINT OF THE
CENSUS.** Most authored vocabulary in this project is `String`: `PickupSpawn`
`kind`, `Prop.kind`, `EnemySpawn` `character_id` / `respawn`,
`MaryOBlock.contents`, `KinematicPath.mode`, `LoadingZone.activation`,
`BreakablePlatform.trigger`. Their legal values live in a Rust parser, and
re-declaring that list here is exactly how three copies of a convention drift
apart. So this does not declare anything: it COUNTS what the world already
authors. `respawn` reads as an opaque string until you are told that 24 of 24
enemies in this file say `OnRoomReenter`, at which point the answer is obvious
and it came from the content rather than from a second authority.

⭐ **`check` is that census turned into a diagnostic, and it found a real
defect the first time it ran.** `mary_o_1_3` was authored on 2026-08-15 through
`area create` + `repair` + `validate` — all three green — with six `EnemySpawn`
entities carrying no `character_id`. The converter REFUSES that
(`convert_enemy_spawn`: *"a placement states which creature it places"*), so
the room would have panicked the game on load, and the whole Python authoring
loop had no way to say so. `check` says it without knowing anything about Rust:
every other `EnemySpawn` in the file authors the field, and these do not.

    vocabulary list  --ldtk W [--identifier ID]   what may I place, and what may I say
    vocabulary check --ldtk W [--level L]         who is the odd one out

⚠ **`check` is a CONSISTENCY signal, not a specification.** A field that only
some placements want (`EnemySpawn.mounted_on`, `MovingPlatform.loop_dy`) is
never flagged, because the rule only fires at 100%. The first placement of a
genuinely optional-but-always-used-so-far field will be a false positive; that
is the honest cost of deriving the rule from content instead of restating it.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any

from ambition_ldtk_tools.ldtk import (
    Issue,
    entity_defs,
    format_issue_lines,
    has_errors,
    load_project,
)
from ambition_ldtk_tools.ldtk.paths import default_sandbox_ldtk

# A field whose value is one of these is "not authored". `None` is an absent or
# nulled field; the empty string is what the editor leaves behind when a value
# is cleared, and every Rust reader in this repo trims before testing.
_BLANK = (None, "")


def _is_blank(value: Any) -> bool:
    if value in _BLANK:
        return True
    return isinstance(value, str) and not value.strip()


def _enum_values(project: dict[str, Any], field_def: dict[str, Any]) -> list[str] | None:
    """The declared values of an `Enum` / `LocalEnum.X` field, if it is one."""
    type_name = str(field_def.get("__type") or "")
    if "Enum" not in type_name:
        return None
    enum_id = type_name.split(".")[-1]
    for enum_def in project.get("defs", {}).get("enums", []) or []:
        if enum_def.get("identifier") == enum_id:
            return [value.get("id") for value in enum_def.get("values", [])]
    for enum_def in project.get("defs", {}).get("externalEnums", []) or []:
        if enum_def.get("identifier") == enum_id:
            return [value.get("id") for value in enum_def.get("values", [])]
    return None


def _default_of(field_def: dict[str, Any]) -> Any:
    override = field_def.get("defaultOverride")
    if isinstance(override, dict):
        params = override.get("params") or []
        return params[0] if params else None
    return None


def _placeable_layers(project: dict[str, Any], entity_def: dict[str, Any]) -> list[str]:
    """Which Entities layers the EDITOR will offer this def on.

    LDtk filters by tags: a layer with `requiredTags` only accepts defs carrying
    all of them, and `excludedTags` hides defs carrying any. Answering this from
    the tag tables rather than from where instances happen to live is the
    difference between "where may I put one" and "where did somebody put one".
    """
    tags = set(entity_def.get("tags") or [])
    out = []
    for layer in project.get("defs", {}).get("layers", []) or []:
        if (layer.get("__type") or layer.get("type")) != "Entities":
            continue
        required = set(layer.get("requiredTags") or [])
        excluded = set(layer.get("excludedTags") or [])
        if required and not required <= tags:
            continue
        if excluded & tags:
            continue
        out.append(layer.get("identifier"))
    return out


def _sidecar_docs(ldtk: Path) -> dict[str, str]:
    """Per-world entity documentation from `<world>.entities.json`.

    That manifest is what `def upsert-entity` reconciles definitions FROM, so
    its `docs` are the authored prose for the game's own nouns. LDtk itself has
    no place to keep them — an entity def carries `doc`, but nothing writes it —
    which is why the manifest exists and why this reads it rather than the def.
    """
    manifest = ldtk.parent / f"{ldtk.stem}.entities.json"
    if not manifest.is_file():
        return {}
    try:
        data = json.loads(manifest.read_text())
    except (OSError, json.JSONDecodeError):
        return {}
    return {
        entry.get("identifier"): entry.get("docs", "")
        for entry in data.get("entities", [])
        if entry.get("identifier")
    }


def _census(project: dict[str, Any]) -> dict[str, dict[str, Any]]:
    """Per-identifier: how many instances exist, and what each field says.

    Returns `{identifier: {"count": n, "fields": {name: Counter}, "authored":
    {name: n}, "levels": {name: {level: n}}}}`. Everything the report and the
    check need comes out of this one pass.
    """
    out: dict[str, dict[str, Any]] = {}
    for level in project.get("levels", []) or []:
        level_id = level.get("identifier")
        for layer in level.get("layerInstances", []) or []:
            for inst in layer.get("entityInstances", []) or []:
                ident = inst.get("__identifier")
                row = out.setdefault(
                    ident,
                    {"count": 0, "fields": {}, "authored": {}, "blank_by_level": {}},
                )
                row["count"] += 1
                for field in inst.get("fieldInstances", []) or []:
                    name = field.get("__identifier")
                    value = field.get("__value")
                    if _is_blank(value):
                        row["blank_by_level"].setdefault(name, {})
                        row["blank_by_level"][name][level_id] = (
                            row["blank_by_level"][name].get(level_id, 0) + 1
                        )
                        continue
                    row["authored"][name] = row["authored"].get(name, 0) + 1
                    row["fields"].setdefault(name, Counter())[_key(value)] += 1
    return out


def _key(value: Any) -> str:
    if isinstance(value, (dict, list)):
        return json.dumps(value, sort_keys=True)
    return str(value)


def _observed_line(counter: Counter, limit: int = 8) -> str:
    parts = [f"{value} ×{n}" for value, n in counter.most_common(limit)]
    if len(counter) > limit:
        parts.append(f"… +{len(counter) - limit} more")
    return ", ".join(parts)


# ---------------------------------------------------------------------------
# `vocabulary list`


def build_listing(project: dict[str, Any], ldtk: Path, identifier: str | None) -> list[dict]:
    docs = _sidecar_docs(ldtk)
    census = _census(project)
    rows = []
    for entity_def in entity_defs(project):
        ident = entity_def.get("identifier")
        if identifier and ident != identifier:
            continue
        seen = census.get(ident, {"count": 0, "fields": {}, "authored": {}})
        fields = []
        for field_def in entity_def.get("fieldDefs", []) or []:
            name = field_def.get("identifier")
            counter = seen["fields"].get(name, Counter())
            fields.append(
                {
                    "name": name,
                    "type": field_def.get("__type"),
                    "default": _default_of(field_def),
                    "enum": _enum_values(project, field_def),
                    "authored_by": seen["authored"].get(name, 0),
                    "observed": dict(counter),
                }
            )
        rows.append(
            {
                "identifier": ident,
                "size": [entity_def.get("width"), entity_def.get("height")],
                "color": entity_def.get("color"),
                "tags": entity_def.get("tags") or [],
                "layers": _placeable_layers(project, entity_def),
                "instances": seen["count"],
                "docs": docs.get(ident) or entity_def.get("doc") or "",
                "fields": fields,
            }
        )
    return rows


def format_listing(rows: list[dict], *, verbose_docs: bool) -> str:
    lines: list[str] = []
    for row in rows:
        w, h = row["size"]
        lines.append(
            f"{row['identifier']}  {w}x{h}  {row['color']}  "
            f"placeable on: {', '.join(row['layers']) or '(no Entities layer accepts it)'}  "
            f"[{row['instances']} placed]"
        )
        if row["docs"]:
            doc = row["docs"] if verbose_docs else row["docs"].split("\n\n")[0]
            for para in doc.strip().split("\n"):
                lines.append(f"    | {para}")
        for field in row["fields"]:
            head = f"    {field['name']:<16} {field['type']}"
            if field["default"] not in (None, ""):
                head += f"  default={field['default']!r}"
            lines.append(head)
            if field["enum"]:
                lines.append(f"        one of: {' | '.join(field['enum'])}")
            if field["observed"]:
                lines.append(
                    f"        authored by {field['authored_by']}/{row['instances']}: "
                    f"{_observed_line(Counter(field['observed']))}"
                )
            elif row["instances"]:
                lines.append(f"        authored by 0/{row['instances']} placements")
        lines.append("")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# `vocabulary check`


def check_issues(project: dict[str, Any], level_filter: str | None) -> list[Issue]:
    """Two rules, both derived from the file's own content.

    1. `vocabulary.field_omitted_here` — every OTHER placement of this entity
       type authors the field and this one does not. That is what a missing
       required field looks like from outside Rust, and it is the shape that
       took `mary_o_1_3` past three green validators.
    2. `vocabulary.value_outside_enum` — an authored value the declared enum
       cannot spell. LDtk stores an enum field's value as a plain string, so a
       hand-written spec or an enum narrowed after the fact can leave one behind.
    """
    census = _census(project)
    issues: list[Issue] = []
    enum_by_field: dict[tuple[str, str], list[str]] = {}
    for entity_def in entity_defs(project):
        for field_def in entity_def.get("fieldDefs", []) or []:
            values = _enum_values(project, field_def)
            if values:
                enum_by_field[(entity_def["identifier"], field_def["identifier"])] = values

    for level in project.get("levels", []) or []:
        level_id = level.get("identifier")
        if level_filter and level_id != level_filter:
            continue
        for layer in level.get("layerInstances", []) or []:
            layer_id = layer.get("__identifier")
            for inst in layer.get("entityInstances", []) or []:
                ident = inst.get("__identifier")
                row = census.get(ident)
                if not row:
                    continue
                for field in inst.get("fieldInstances", []) or []:
                    name = field.get("__identifier")
                    value = field.get("__value")
                    if _is_blank(value):
                        blanks_here = sum(
                            n
                            for lvl, n in row["blank_by_level"].get(name, {}).items()
                            if lvl == level_id
                        )
                        authored = row["authored"].get(name, 0)
                        # Fires only when EVERY placement outside this level
                        # authors it: an optional field somebody happens not to
                        # use is not a defect, and saying so would drown the
                        # signal that is.
                        if authored and authored + blanks_here == row["count"]:
                            observed = Counter(row["fields"].get(name, Counter()))
                            issues.append(
                                Issue(
                                    severity="warning",
                                    code="vocabulary.field_omitted_here",
                                    message=(
                                        f"{ident}.{name} is blank here, and all "
                                        f"{authored} other {ident} placements in this "
                                        f"world author it"
                                    ),
                                    level=level_id,
                                    layer=layer_id,
                                    entity=ident,
                                    entity_iid=inst.get("iid"),
                                    field=name,
                                    fix_hint=(
                                        f"values already in use: {_observed_line(observed)}"
                                    ),
                                    data={"authored_elsewhere": authored},
                                )
                            )
                        continue
                    allowed = enum_by_field.get((ident, name))
                    if allowed and str(value) not in allowed:
                        issues.append(
                            Issue(
                                severity="error",
                                code="vocabulary.value_outside_enum",
                                message=(
                                    f"{ident}.{name} is {value!r}, which its enum "
                                    f"cannot spell"
                                ),
                                level=level_id,
                                layer=layer_id,
                                entity=ident,
                                entity_iid=inst.get("iid"),
                                field=name,
                                fix_hint=f"one of: {' | '.join(allowed)}",
                            )
                        )
    return issues


# ---------------------------------------------------------------------------
# CLI


def _cmd_list(args: argparse.Namespace) -> int:
    project = load_project(args.ldtk)
    rows = build_listing(project, args.ldtk, args.identifier)
    if not rows:
        known = ", ".join(d.get("identifier") for d in entity_defs(project))
        print(f"no entity def named {args.identifier!r}. This world declares: {known}")
        return 1
    if args.format == "json":
        print(json.dumps(rows, indent=2))
    else:
        print(format_listing(rows, verbose_docs=args.docs))
    return 0


def _cmd_check(args: argparse.Namespace) -> int:
    project = load_project(args.ldtk)
    issues = check_issues(project, args.level)
    if args.format == "json":
        print(json.dumps([issue.as_dict() for issue in issues], indent=2))
    else:
        scope = args.level or args.ldtk.name
        print(
            format_issue_lines(
                issues,
                title=f"Vocabulary consistency — {scope}",
                empty=(
                    f"OK: every placement in {scope} authors the fields its "
                    f"siblings do, and every enum value is spellable"
                ),
            )
        )
    return 1 if has_errors(issues) else 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=default_sandbox_ldtk(),
        help="LDtk project path (default: Ambition sandbox.ldtk)",
    )
    sub = parser.add_subparsers(dest="action", required=True)

    listing = sub.add_parser("list", help="what can be placed, and what each field says")
    listing.add_argument("--identifier", help="only this entity def")
    listing.add_argument("--format", choices=["text", "json"], default="text")
    listing.add_argument(
        "--docs",
        action="store_true",
        help="print each def's full authored documentation, not its first paragraph",
    )
    listing.set_defaults(func=_cmd_list)

    check = sub.add_parser("check", help="placements that disagree with their siblings")
    check.add_argument("--level", help="only this level (default: the whole world)")
    check.add_argument("--format", choices=["text", "json"], default="text")
    check.set_defaults(func=_cmd_check)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
