"""Shared mapping from validation messages to first-class Issue codes.

This keeps the long-standing validate implementation stable while new rule
modules migrate toward emitting structured issues directly.
"""

from __future__ import annotations

import re
from collections.abc import Iterable

from ambition_ldtk_tools.ldtk.issues import Issue

_PATTERNS: list[tuple[str, re.Pattern[str]]] = [
    ("validate.json.parse", re.compile(r"failed to parse JSON")),
    ("validate.schema", re.compile(r"schema|JSON_SCHEMA|jsonVersion", re.I)),
    ("validate.project.identity", re.compile(r"root iid|project has no levels|worldLayout", re.I)),
    ("validate.editor.shape", re.compile(r"first-class LDtk|field definition|realEditorValues|editor", re.I)),
    ("validate.definitions", re.compile(r"defs\.|definition|definitions|known entity", re.I)),
    ("validate.level.geometry", re.compile(r"level .*dimensions|worldX|worldY|grid|edge|walk off", re.I)),
    ("validate.loading_zone", re.compile(r"LoadingZone|target_room|target_zone|Door|EdgeExit", re.I)),
    ("validate.spawn", re.compile(r"PlayerStart|spawn|EnemySpawn|NpcSpawn|PickupSpawn", re.I)),
    ("validate.overlap", re.compile(r"overlap|intersect", re.I)),
    ("validate.authoring_hygiene", re.compile(r"intro|hygiene|sprite render bleeds|mid-air", re.I)),
]


def code_for_message(message: str, *, default: str) -> str:
    for code, pattern in _PATTERNS:
        if pattern.search(message):
            return code
    return default


def issues_from_messages(errors: Iterable[str], warnings: Iterable[str]) -> list[Issue]:
    issues: list[Issue] = []
    for warning in warnings:
        text = str(warning)
        issues.append(
            Issue(
                severity="warning",
                code=code_for_message(text, default="validate.warning"),
                message=text,
            )
        )
    for error in errors:
        text = str(error)
        issues.append(
            Issue(
                severity="error",
                code=code_for_message(text, default="validate.error"),
                message=text,
            )
        )
    return issues
