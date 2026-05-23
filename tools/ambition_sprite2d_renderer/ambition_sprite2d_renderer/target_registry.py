"""Auto-discovery for tack-on sprite targets.

Adding a new tack-on target is now a single-file operation: drop a
``targets/<name>.py`` module that exposes the small API below. The
registry walks ``targets/`` at import time, imports each candidate, and
collects the ones that conform. There is **no** central registration
list to edit, so independent agents can add new characters/props
without merge-conflict diffs in this file.

## Tack-on target API

A tack-on target module MUST define::

    def render(out_dir, **opts) -> Iterable[Path]: ...

and MAY define::

    SHEET_FILES: list[str]
        Files the installer copies into the sandbox sprites dir.
        Default: ``{stem}_spritesheet.{png,yaml,ron}`` (matches the
        `pirates.common.build_sheet` output convention).

    def install(render_dir, dest_root) -> Iterable[Path]: ...
        Custom installer that overrides the default copy-each-of-
        SHEET_FILES behavior. Used by targets like `mockingbird_boss`
        that ship subdirectories.

The module stem (filename without ``.py``) is the canonical target id —
the registry key, the CLI ``choices=`` value, and the default
``generated/<id>/`` subdir. Module-internal constants like
``TARGET_NAME`` exist for the target's own use (file naming, asset
ids) but are not required by the registry.

## Excluded modules

Some files under ``targets/`` are *adapter helpers* — they expose
``Generator`` classes that are imported by ``adapters.py`` and driven
by YAML configs, not by ``render()``. Those are listed explicitly in
[`ADAPTER_HELPER_STEMS`] so the discovery walker can skip them
without false-positive warnings.

Bare helper modules (e.g. shared drawing primitives used by multiple
targets) belong at the package root (``ambition_sprite2d_renderer/``),
not under ``targets/``. ``targets/`` should hold *targets* only.
"""
from __future__ import annotations

import importlib
from pathlib import Path
from typing import Dict, List, NamedTuple

# Modules under `targets/` that are imported by `adapters.py` and
# driven by YAML configs instead of a `render()` function. Keep this
# list in sync with the imports at the top of `adapters.py`.
#
# Note: `sandbag` is intentionally NOT here — it's both an adapter
# target AND a tack-on target (it exposes both `ADAPTER_ANIMATIONS`
# for the adapter system and `render()` for the tack-on system).
ADAPTER_HELPER_STEMS: frozenset[str] = frozenset({
    "alice_cryptographer",
    "bob_engineer",
    "boss_side",
    "goblin_side",
    "ninja_side",
    "robot25d",
    "robot_side",
    "toon_side",
    "trent_elder",
})


class _DiscoveredTarget(NamedTuple):
    stem: str
    module_path: str


class DiscoveryReport(NamedTuple):
    """Outcome of walking ``targets/`` once."""

    targets: Dict[str, str]
    """``{stem: dotted-module-path}`` for every conformant tack-on."""

    warnings: List[str]
    """Human-readable lines explaining why a file was skipped."""


def _targets_dir() -> Path:
    return Path(__file__).resolve().parent / "targets"


def _is_candidate(path: Path) -> bool:
    if path.suffix != ".py":
        return False
    if path.stem.startswith("_"):
        return False  # __init__, __main__, private helpers
    if path.stem in ADAPTER_HELPER_STEMS:
        return False
    return True


def discover_tackon_targets() -> DiscoveryReport:
    """Walk ``targets/`` and return every module that has a ``render()``.

    Files that don't conform are recorded in ``warnings`` so a missing
    API is visible (via ``list-targets``) rather than silent.
    """
    targets: Dict[str, str] = {}
    warnings: List[str] = []
    for path in sorted(_targets_dir().glob("*.py")):
        if not _is_candidate(path):
            continue
        stem = path.stem
        dotted = f"ambition_sprite2d_renderer.targets.{stem}"
        try:
            mod = importlib.import_module(dotted)
        except Exception as ex:
            warnings.append(f"{stem}: import failed ({type(ex).__name__}: {ex})")
            continue
        if not callable(getattr(mod, "render", None)):
            warnings.append(
                f"{stem}: no `render(out_dir, **opts) -> Iterable[Path]` "
                f"function — add one to register as a tack-on target, or "
                f"move shared helpers to the package root."
            )
            continue
        targets[stem] = dotted
    return DiscoveryReport(targets=targets, warnings=warnings)


def default_sheet_files(stem: str) -> List[str]:
    """Default install set when a target doesn't declare ``SHEET_FILES``.

    Matches the ``pirates.common.build_sheet`` output convention used
    by most tack-ons.
    """
    return [
        f"{stem}_spritesheet.png",
        f"{stem}_spritesheet.yaml",
        f"{stem}_spritesheet.ron",
    ]
