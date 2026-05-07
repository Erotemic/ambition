"""Render targets for ambition_sprite2d_renderer.

Each target module exposes:
    - TARGET_NAME: the canonical target id used on the CLI
    - SHEET_FILES: tuple of expected output filenames (PNG, manifest, ...)
    - render(out_dir, **opts) -> list[Path]: produce files in out_dir

The registry maps the public target id to its module path so that
``list_target_names()`` works without importing heavy graphics deps.
Modules are only imported when a target is actually rendered/installed.
"""
from __future__ import annotations

from importlib import import_module

# target_id -> dotted module path
_TARGETS: dict[str, str] = {
    "sandbag": "ambition_sprite2d_renderer.targets.sandbag",
}


def get_target(name: str):
    try:
        mod_path = _TARGETS[name]
    except KeyError as ex:
        raise KeyError(f"unknown target: {name}") from ex
    return import_module(mod_path)


def list_target_names() -> list[str]:
    return list(_TARGETS.keys())
