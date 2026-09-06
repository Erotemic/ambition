#!/usr/bin/env python3
"""Validate authored LDtk entities against the runtime-owned entity contract.

The shared JSON contract is proved against the Rust converters and read here by
the authoring tools. `refused` values are errors, `open` grammars remain provider
extension points, and `silent_default` grammars are rejected during authoring so
a typo cannot be accepted as an unrelated default. Fields with no declared
grammar accept any value while still participating in presence checks."""

from __future__ import annotations

import json
import re
from functools import lru_cache
from pathlib import Path
from typing import Any, Iterable

from ambition_ldtk_tools.ldtk.issues import Issue
from ambition_ldtk_tools.ldtk.paths import default_entity_contract

# What the census in `vocabulary.py` calls blank, spelled the same way: every
# Rust reader in this repo trims before testing, and the editor leaves an empty
# string behind when a value is cleared.
_BLANK = (None, "")


class ContractUnavailable(RuntimeError):
    """The contract file is missing or malformed.

    ⚠ raised rather than swallowed. A validator that silently skips its own
    contract when the file moves is a check that cannot fail, which is worse than
    no check at all — the caller decides whether to degrade, out loud.
    """


@lru_cache(maxsize=8)
def load_contract(path: Path | None = None) -> dict[str, Any]:
    path = path or default_entity_contract()
    try:
        document = json.loads(Path(path).read_text())
    except FileNotFoundError as ex:
        raise ContractUnavailable(
            f"the LDtk authoring contract is missing at {path}. It is committed "
            f"beside the crate that proves it; a partial checkout or a moved "
            f"crate is the usual cause."
        ) from ex
    except json.JSONDecodeError as ex:
        raise ContractUnavailable(f"{path} is not valid JSON: {ex}") from ex
    if not isinstance(document.get("entities"), list):
        raise ContractUnavailable(f"{path} declares no `entities` list")
    return document


def entity_contracts(path: Path | None = None) -> dict[str, dict[str, Any]]:
    """`{identifier: contract}` for every entity the engine converters handle."""
    return {
        entity["identifier"]: entity for entity in load_contract(path)["entities"]
    }


def contract_identifiers(path: Path | None = None) -> set[str]:
    """The engine's LDtk vocabulary, from the same table the converters obey.

    ⛔ **this replaced a hand-typed `KNOWN_ENTITIES` set, which had already
    drifted.** `SurfaceRamp` — the engine's own floor-to-wall fillet — was a legal
    entity to Rust and an unknown one to this validator, so authoring one failed a
    check that was supposed to be describing the engine.
    """
    return set(entity_contracts(path))


# ---------------------------------------------------------------------------
# Reading one authored value


def _is_blank(value: Any) -> bool:
    if isinstance(value, dict):
        # An unset LDtk EntityRef is `null`; a set one is an object.
        return not value.get("entityIid")
    if value in _BLANK:
        return True
    return isinstance(value, str) and not value.strip()


def _field_value(entity: dict[str, Any], name: str) -> Any:
    for field in entity.get("fieldInstances") or []:
        if field.get("__identifier") == name:
            return field.get("__value")
    return None


def _authored(entity: dict[str, Any], name: str) -> bool:
    return not _is_blank(_field_value(entity, name))


def _as_text(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value)


def _as_number(value: Any) -> float | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        return float(value)
    try:
        return float(str(value).strip())
    except (TypeError, ValueError):
        return None


def _point_count(value: Any) -> int:
    count = 0
    for pair in _as_text(value).split(";"):
        parts = [part.strip() for part in pair.split(",")]
        if len(parts) < 2:
            continue
        try:
            float(parts[0])
            float(parts[1])
        except ValueError:
            continue
        count += 1
    return count


def _normalized(text: str, rule: str | None) -> str:
    """Fold a value exactly as the converter folds it before matching.

    ⚠ **spelled as the parser spells it, not as "case insensitive".**
    `parse_path_mode` and the two camera policies do
    `trim().to_ascii_lowercase().replace('-', "_")`, while
    `PortalChannelColorSpec::from_name` only lowercases — and the difference
    decides whether `zone-bounds` is a clamp mode (it is) and whether `c-1` is a
    portal colour (it is not).
    """
    if rule == "lowercase":
        return text.strip().lower()
    if rule == "lowercase_underscore":
        return text.strip().lower().replace("-", "_")
    return text


def _grammar_rejects(field: dict[str, Any], value: Any) -> str | None:
    """Why this field's declared grammar rejects `value`, or `None` if it does not.

    ⚠ mirrors the poison the Rust prover pokes each field with, so the two agree
    about what "invalid" even means: a closed set plus regex grammars, a minimum
    point count, a positivity or non-zero requirement. A field declaring none of
    those has no grammar, and every value is legal.
    """
    values = field.get("values")
    patterns = field.get("patterns") or []
    if values or patterns:
        text = _as_text(value)
        rule = field.get("normalize")
        candidates = list(values or [])
        if _normalized(text, rule) in {_normalized(v, rule) for v in candidates}:
            return None
        # Patterns match the RAW text: a grammar that tolerates case says so in
        # the pattern (`^[cC][0-9]{1,3}$`), which keeps one spelling of the rule.
        for pattern in patterns:
            if re.search(pattern, text):
                return None
        spellings = " | ".join(candidates) if candidates else ""
        if patterns:
            grammars = " | ".join(patterns)
            spellings = f"{spellings} | {grammars}" if spellings else grammars
        return f"{text!r} is not one of: {spellings}"

    min_points = field.get("min_points")
    if min_points is not None:
        found = _point_count(value)
        if found < int(min_points):
            return (
                f"{found} parseable point(s); at least {min_points} are needed "
                f"(the format is `x,y;x,y`)"
            )
        return None

    if field.get("positive") or field.get("nonzero"):
        number = _as_number(value)
        if number is None:
            return f"{_as_text(value)!r} is not a number"
        if field.get("positive") and number <= 0.0:
            return f"{number:g} is not positive"
        if field.get("nonzero") and number == 0.0:
            return "zero describes no motion at all"
        return None

    return None


# ---------------------------------------------------------------------------
# Project shape


def _active_area(level: dict[str, Any]) -> str:
    for field in level.get("fieldInstances") or []:
        if field.get("__identifier") == "activeArea":
            value = field.get("__value")
            if value:
                return str(value)
    return str(level.get("identifier") or "")


def _entities_by_area(project: dict[str, Any]) -> dict[str, dict[str, str]]:
    """`{active_area: {entity iid: identifier}}` — the scope an EntityRef resolves in.

    ⚠ AREA-scoped on purpose. `LdtkEntityCtx::kinematic_path_ref` looks a
    reference up in exactly this scope and refuses one that points across the
    boundary, so a wider index here would call a level healthy that the converter
    rejects — which is the failure mode this whole module exists to close.
    """
    index: dict[str, dict[str, str]] = {}
    for level in project.get("levels") or []:
        area = index.setdefault(_active_area(level), {})
        for layer in level.get("layerInstances") or []:
            for instance in layer.get("entityInstances") or []:
                iid = instance.get("iid")
                if iid:
                    area[iid] = instance.get("__identifier")
    return index


# ---------------------------------------------------------------------------
# The rule


def _severity_for(field: dict[str, Any]) -> str | None:
    disposition = field.get("on_invalid")
    if disposition == "refused":
        return "error"
    if disposition == "silent_default":
        return "error"
    return None


def entity_contract_issues(
    project: dict[str, Any],
    *,
    contract_path: Path | None = None,
) -> list[Issue]:
    """Every way this project's placements disagree with the runtime's contract."""

    contracts = entity_contracts(contract_path)
    ref_index = _entities_by_area(project)

    issues: list[Issue] = []
    for level in project.get("levels") or []:
        level_id = level.get("identifier")
        area = _active_area(level)
        for layer in level.get("layerInstances") or []:
            layer_id = layer.get("__identifier")
            for instance in layer.get("entityInstances") or []:
                identifier = instance.get("__identifier")
                entity = contracts.get(identifier)
                if not entity:
                    # A game-registered entity (`MaryOBlock`) or an unknown one.
                    # Unknown identifiers are the existing vocabulary check's job.
                    continue
                issues.extend(
                    _instance_issues(
                        entity, instance, level_id, layer_id, area, ref_index
                    )
                )
    return issues


def _issue(
    *,
    severity: str,
    code: str,
    message: str,
    entity: dict[str, Any],
    instance: dict[str, Any],
    field: dict[str, Any],
    level: str | None,
    layer: str | None,
    hint: str | None = None,
) -> Issue:
    note = field.get("note")
    fix = hint or ""
    if note:
        fix = f"{fix} — {note}" if fix else note
    return Issue(
        severity=severity,
        code=code,
        message=message,
        level=level,
        layer=layer,
        entity=entity["identifier"],
        entity_iid=instance.get("iid"),
        field=field["name"],
        fix_hint=fix or None,
    )


def _instance_issues(
    entity: dict[str, Any],
    instance: dict[str, Any],
    level_id: str | None,
    layer_id: str | None,
    area: str,
    ref_index: dict[str, dict[str, str]],
) -> Iterable[Issue]:
    issues: list[Issue] = []
    identifier = entity["identifier"]

    for field in entity.get("fields") or []:
        name = field["name"]
        value = _field_value(instance, name)
        blank = _is_blank(value)
        presence = field.get("presence", "optional")

        # --- absence
        if blank:
            if presence == "required":
                issues.append(
                    _issue(
                        severity="error",
                        code="contract.required_field_missing",
                        message=(
                            f"{identifier} authors no `{name}`, and the LDtk "
                            f"converter REFUSES the room without it — this level "
                            f"would fail to load"
                        ),
                        entity=entity,
                        instance=instance,
                        field=field,
                        level=level_id,
                        layer=layer_id,
                        hint=f"set `{name}` on this placement",
                    )
                )
            elif presence == "recommended":
                issues.append(
                    _issue(
                        severity="warning",
                        code="contract.recommended_field_missing",
                        message=(
                            f"{identifier} authors no `{name}`. The converter "
                            f"tolerates that, but the result is not what the "
                            f"placement means"
                        ),
                        entity=entity,
                        instance=instance,
                        field=field,
                        level=level_id,
                        layer=layer_id,
                    )
                )
            condition = field.get("requires_value_of")
            if condition and _as_text(
                _field_value(instance, condition["field"])
            ) == condition["equals"]:
                issues.append(
                    _issue(
                        severity="error",
                        code="contract.conditional_field_missing",
                        message=(
                            f"{identifier} sets `{condition['field']}` to "
                            f"{condition['equals']!r}, which the converter refuses "
                            f"without `{name}`"
                        ),
                        entity=entity,
                        instance=instance,
                        field=field,
                        level=level_id,
                        layer=layer_id,
                        hint=f"set `{name}`, or pick another {condition['field']}",
                    )
                )
            continue

        # --- an authored value

        # a conditional field's GRAMMAR is conditional too. Nine
        # `Breakable*` placements in sandbox.ldtk carry `respawn_seconds: 0`
        # beside `respawn: OnRoomReload`, and `parse_breakable_respawn` never
        # looks at the number unless `respawn` is exactly `AfterSeconds`. Checking
        # it anyway called nine healthy platforms broken on the first run —
        # measured, not reasoned, and the reason this branch exists.
        condition = field.get("requires_value_of")
        if condition and _as_text(
            _field_value(instance, condition["field"])
        ) != condition["equals"]:
            continue

        for companion in field.get("requires_fields") or []:
            if not _authored(instance, companion):
                issues.append(
                    _issue(
                        severity="error",
                        code="contract.companion_field_missing",
                        message=(
                            f"{identifier} authors `{name}` without `{companion}`, "
                            f"which the converter refuses"
                        ),
                        entity=entity,
                        instance=instance,
                        field=field,
                        level=level_id,
                        layer=layer_id,
                        hint=f"author `{companion}` too, or clear `{name}`",
                    )
                )

        for other in field.get("conflicts_with") or []:
            if _authored(instance, other):
                issues.append(
                    _issue(
                        severity="error",
                        code="contract.conflicting_fields",
                        message=(
                            f"{identifier} authors both `{name}` and `{other}`, "
                            f"which are two answers to one question — the converter "
                            f"refuses rather than picking one"
                        ),
                        entity=entity,
                        instance=instance,
                        field=field,
                        level=level_id,
                        layer=layer_id,
                        hint=f"keep one of `{name}` / `{other}` and clear the other",
                    )
                )

        text = _as_text(value) if not isinstance(value, dict) else ""
        for pattern in field.get("refused_patterns") or []:
            if re.search(pattern, text):
                issues.append(
                    _issue(
                        severity="error",
                        code="contract.retired_spelling",
                        message=(
                            f"{identifier}.{name} is {text!r}, a retired spelling "
                            f"the converter refuses out loud"
                        ),
                        entity=entity,
                        instance=instance,
                        field=field,
                        level=level_id,
                        layer=layer_id,
                    )
                )
                break

        target = field.get("entity_ref_target")
        if target:
            issues.extend(
                _entity_ref_issues(
                    entity, instance, field, value, area, ref_index, level_id, layer_id
                )
            )
            continue

        problem = _grammar_rejects(field, value)
        if problem is None:
            continue
        severity = _severity_for(field)
        if severity is None:
            # `open`: the fallthrough is the extension point, and refusing here
            # would break the thing it exists for.
            continue
        if field.get("on_invalid") == "silent_default":
            substitute = field.get("default") or "a fixed default"
            issues.append(
                _issue(
                    severity=severity,
                    code="contract.value_silently_defaulted",
                    message=(
                        f"{identifier}.{name}: {problem}. The converter does not "
                        f"refuse this — it silently becomes {substitute}, so the "
                        f"mistake is invisible in play"
                    ),
                    entity=entity,
                    instance=instance,
                    field=field,
                    level=level_id,
                    layer=layer_id,
                )
            )
        else:
            issues.append(
                _issue(
                    severity=severity,
                    code="contract.value_refused",
                    message=f"{identifier}.{name}: {problem}. The converter refuses it",
                    entity=entity,
                    instance=instance,
                    field=field,
                    level=level_id,
                    layer=layer_id,
                )
            )
    return issues


def _entity_ref_issues(
    entity: dict[str, Any],
    instance: dict[str, Any],
    field: dict[str, Any],
    value: Any,
    area: str,
    ref_index: dict[str, dict[str, str]],
    level_id: str | None,
    layer_id: str | None,
) -> list[Issue]:
    target_kind = field["entity_ref_target"]
    target_iid = value.get("entityIid") if isinstance(value, dict) else _as_text(value)
    scope = field.get("entity_ref_scope")
    if scope == "active_area":
        found = (ref_index.get(area) or {}).get(target_iid)
        where = f"active area {area!r}"
    else:
        found = None
        for entities in ref_index.values():
            if target_iid in entities:
                found = entities[target_iid]
                break
        where = "this world"

    if found == target_kind:
        return []
    if found is None:
        detail = f"names no entity in {where}"
    else:
        detail = f"names a {found} in {where}, not a {target_kind}"
    return [
        _issue(
            severity="error",
            code="contract.entity_ref_unresolved",
            message=(
                f"{entity['identifier']}.{field['name']} {detail}. The converter "
                f"refuses a reference it cannot resolve rather than degrading to "
                f"'no motion'"
            ),
            entity=entity,
            instance=instance,
            field=field,
            level=level_id,
            layer=layer_id,
            hint=f"point it at a {target_kind} in {where}, or clear it",
        )
    ]
