"""Per-archetype toon preset definitions.

Each ``<name>.py`` here defines one ``PRESET`` dict for one toon
archetype. Adding a new archetype is a single-file drop — create
``_toon_presets/<archetype>.py`` with ``PRESET = {...}`` and that's
it; this module walks the directory and picks it up automatically.

Modules starting with ``_`` are skipped so internal helpers can sit
alongside the archetype files without polluting the registry.
"""

from __future__ import annotations

import importlib
import pkgutil
from typing import Any, Dict


def _discover_presets() -> Dict[str, Dict[str, Any]]:
    """Walk this package's modules and collect every exported ``PRESET``."""
    presets: Dict[str, Dict[str, Any]] = {}
    for info in pkgutil.iter_modules(__path__):
        if info.name.startswith("_"):
            continue
        mod = importlib.import_module(f".{info.name}", __name__)
        preset = getattr(mod, "PRESET", None)
        if preset is None:
            continue
        presets[info.name] = preset
    return presets


PRESETS: Dict[str, Dict[str, Any]] = _discover_presets()
