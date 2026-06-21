"""Shared issue model for LDtk diagnostics.

Validation, policy, visual-reference, and room-inspection commands should emit
this small stable shape instead of inventing command-specific issue records.
The model is intentionally dict-friendly so CLI JSON output and agent tooling can
consume it without importing Python classes.
"""

from __future__ import annotations

from dataclasses import dataclass, field as dc_field
from typing import Any, Mapping


@dataclass(frozen=True)
class Issue:
    """Structured diagnostic emitted by LDtk tools.

    Fields are deliberately broad rather than deeply typed. The LDtk file stays
    as normal dictionaries, and commands can incrementally fill in richer
    location fields without changing the public JSON schema.
    """

    severity: str
    code: str
    message: str
    level: str | None = None
    layer: str | None = None
    entity: str | None = None
    entity_iid: str | None = None
    field: str | None = None
    fix_hint: str | None = None
    fixable: bool = False
    data: Mapping[str, Any] = dc_field(default_factory=dict)

    @property
    def location(self) -> str | None:
        parts: list[str] = []
        if self.level:
            parts.append(self.level)
        if self.layer:
            parts.append(self.layer)
        if self.entity:
            entity = self.entity
            if self.entity_iid:
                entity = f"{entity} {self.entity_iid}"
            parts.append(entity)
        elif self.entity_iid:
            parts.append(self.entity_iid)
        if self.field:
            parts.append(f"field:{self.field}")
        return " / ".join(parts) if parts else None

    def as_dict(self, *, include_none: bool = False) -> dict[str, Any]:
        data: dict[str, Any] = {
            "severity": self.severity,
            "code": self.code,
            "message": self.message,
            "level": self.level,
            "layer": self.layer,
            "entity": self.entity,
            "entity_iid": self.entity_iid,
            "field": self.field,
            "fix_hint": self.fix_hint,
            "fixable": self.fixable,
            "data": dict(self.data),
        }
        if not include_none:
            data = {k: v for k, v in data.items() if v is not None and v != {} and v is not False}
        return data


def has_errors(issues: list[Issue]) -> bool:
    return any(issue.severity == "error" for issue in issues)


def format_issue_lines(issues: list[Issue], *, title: str, empty: str) -> str:
    if not issues:
        return empty.rstrip("\n") + "\n"
    lines = [title]
    for issue in issues:
        loc = issue.location
        where = f"{loc}: " if loc else ""
        suffix = ""
        if issue.fixable:
            suffix += " [fixable]"
        if issue.fix_hint:
            suffix += f" fix: {issue.fix_hint}"
        lines.append(f"  {issue.severity}: {issue.code}: {where}{issue.message}{suffix}")
    return "\n".join(lines) + "\n"
