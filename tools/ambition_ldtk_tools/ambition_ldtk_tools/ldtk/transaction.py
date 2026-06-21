"""Shared transaction/writeback helper for LDtk mutating commands."""

from __future__ import annotations

import shutil
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from ambition_ldtk_tools.ldtk.io import load_project, write_project
from ambition_ldtk_tools.ldtk.patch import PatchOp, PatchResult


@dataclass
class LdtkTransaction:
    """Load, mutate, and optionally write an LDtk project.

    Commands should use this instead of hand-rolling dry-run/no-op/writeback
    logic.  The class deliberately avoids running full project validation;
    validation remains an explicit command so unrelated cross-file links do not
    break a local edit.
    """

    source: Path
    dry_run: bool = False
    in_place: bool = False
    output: Path | None = None
    backup: bool = False
    project: dict[str, Any] = field(init=False)
    changed: bool = field(default=False, init=False)
    messages: list[str] = field(default_factory=list, init=False)

    def __post_init__(self) -> None:
        self.source = Path(self.source)
        self.project = load_project(self.source)

    @property
    def target(self) -> Path | None:
        return self.source if self.in_place else self.output

    def apply(self, op: PatchOp) -> PatchResult:
        result = op.apply(self.project)
        self.changed = self.changed or result.changed
        self.messages.extend(result.messages)
        return result

    def note_changed(self, messages: list[str] | None = None) -> None:
        self.changed = True
        if messages:
            self.messages.extend(messages)

    def require_write_target(self) -> None:
        if self.dry_run:
            return
        if self.target is None:
            raise SystemExit("choose --dry-run, --in-place, or --output <path>")

    def write_if_changed(self) -> Path | None:
        self.require_write_target()
        if self.dry_run or not self.changed:
            return None
        target = self.target
        if target is None:  # pragma: no cover - guarded by require_write_target
            return None
        if self.backup and self.in_place:
            backup_path = self.source.with_suffix(self.source.suffix + ".bak")
            shutil.copy2(self.source, backup_path)
            print(f"backup written: {backup_path}")
        write_project(target, self.project)
        return target

    def finish(self, *, noop_message: str | None = None, write_message: str | None = None) -> Path | None:
        target = self.write_if_changed()
        if target is None:
            if noop_message and not self.changed:
                print(noop_message)
            return None
        if write_message:
            print(write_message.format(path=target))
        return target
