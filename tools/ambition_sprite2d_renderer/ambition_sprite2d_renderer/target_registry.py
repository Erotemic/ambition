"""Auto-discovery for sprite targets.

Targets live under ``targets/<category>/`` where category is one of
[`CATEGORIES`]. A target can be:

* a single ``.py`` file (e.g. ``targets/characters/ghoul_skulker.py``), or
* a package directory with an ``__init__.py`` (e.g.
  ``targets/characters/mockingbird_boss/``)

Either form is auto-registered if the module exposes ``render``; the
multi-file pattern is the right choice when a character needs its own
helper modules, part-config YAML files, or part-editor scripts.

Dropping a file or directory into the right category subdir is the
*entire* integration step — no central registration list to edit, so
independent agents can author new characters/props/tiles without
producing merge-conflict diffs in this file.

## Categories

* ``characters/`` — anything controllable by a brain (state machine,
  RL agent, player input): normal characters, bosses, tiny enemies.
* ``props/`` — items characters might hold, world objects, scene
  dressing, batched entity sprite sheets.
* ``tiles/`` — LDtk tileset atlases (map cells designed to repeat).
* ``icons/`` — UI ability/item icons.

## Tack-on target API

A target module exposes::

    def render(out_dir, **opts) -> Iterable[Path]: ...

and may optionally expose::

    SHEET_FILES: list[str]
        Files the installer copies into the sandbox sprites dir.
        Default: ``{stem}_spritesheet.{png,yaml,ron}`` (matches the
        ``tackon_sheet.build_sheet`` output convention).

    def install(render_dir, dest_root) -> Iterable[Path]: ...
        Custom installer (overrides the default copy-each-of-SHEET_FILES).
        Used by package targets that ship a subdirectory of part files.

A *single module* may register *multiple targets* by defining a
``TARGETS`` dict instead::

    TARGETS = {
        "alpha": {"render": render_alpha, "sheet_files": [...]},
        "beta": {"render": render_beta},
    }

Each entry becomes its own registry key — useful when one file
naturally yields several related sprite sheets (e.g. an entity batch).

## Adapter helpers

Files under ``targets/characters/`` listed in
[`ADAPTER_HELPER_STEMS`] are *not* tack-on targets — they expose
``Generator`` classes consumed by ``adapters.py`` and driven by
YAML configs. Discovery skips them silently.

Bare helper modules (drawing primitives shared by multiple targets)
belong at the package root, not under ``targets/``.
"""
from __future__ import annotations

import importlib
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Dict, Iterator, List, NamedTuple, Optional, Tuple

CATEGORIES: Tuple[str, ...] = ("characters", "props", "tiles", "icons")

# Modules under `targets/characters/` that are imported by
# `adapters.py` and driven by YAML configs instead of a `render()`
# function. Discovery silently skips these so they don't show up as
# warnings under `list-targets`.
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


@dataclass(frozen=True)
class TackonTarget:
    """One discovered tack-on target ready to render + install."""

    name: str
    """Registry key — the CLI ``choices=`` value and ``generated/<name>/`` subdir."""

    category: str
    """One of [`CATEGORIES`]."""

    module_path: str
    """Dotted import path of the module that defines this target."""

    render: Callable
    """The ``render(out_dir, **opts) -> Iterable[Path]`` function."""

    sheet_files: Tuple[str, ...]
    """Files the default installer copies into the sandbox sprites dir."""

    install: Optional[Callable] = None
    """Custom installer (overrides default copy-each-of-SHEET_FILES) when set."""

    render_canonical: Optional[Callable] = None
    """Optional fast canonical-only hook: ``render_canonical(out_dir, **opts) -> Path``.

    When present, ``draw-canonicals`` calls this instead of running the
    full sheet build to grab a canonical pose. Targets built on
    [`tackon_sheet.build_sheet`] should expose one (it's a 3-line
    wrapper around [`tackon_sheet.write_canonical`]). When absent, the
    canonical collector falls back to a slow ``render()`` + pluck path.
    """


class DiscoveryReport(NamedTuple):
    """Outcome of walking ``targets/`` once."""

    targets: Dict[str, TackonTarget]
    warnings: List[str]


def _targets_dir() -> Path:
    return Path(__file__).resolve().parent / "targets"


def _walk_category(category: str) -> Iterator[Tuple[str, str]]:
    """Yield ``(stem, dotted_module_path)`` for each candidate under
    ``targets/<category>/`` — both single-file modules and packages."""
    cat_dir = _targets_dir() / category
    if not cat_dir.is_dir():
        return
    for path in sorted(cat_dir.iterdir()):
        name = path.name
        if name.startswith("_"):
            continue  # __init__, __main__, private helpers
        if path.is_file() and path.suffix == ".py":
            stem = path.stem
            yield stem, f"ambition_sprite2d_renderer.targets.{category}.{stem}"
        elif path.is_dir() and (path / "__init__.py").exists():
            yield name, f"ambition_sprite2d_renderer.targets.{category}.{name}"


def default_sheet_files(stem: str) -> List[str]:
    """Default install set when a target doesn't declare ``SHEET_FILES``.

    Matches the ``tackon_sheet.build_sheet`` output convention used
    by most tack-ons.
    """
    return [
        f"{stem}_spritesheet.png",
        f"{stem}_spritesheet.yaml",
        f"{stem}_spritesheet.ron",
    ]


def _register_single(
    mod, stem: str, category: str, dotted: str, render: Callable,
) -> TackonTarget:
    sheet_files = tuple(getattr(mod, "SHEET_FILES", default_sheet_files(stem)))
    install = getattr(mod, "install", None)
    if not callable(install):
        install = None
    render_canonical = getattr(mod, "render_canonical", None)
    if not callable(render_canonical):
        render_canonical = None
    return TackonTarget(
        name=stem,
        category=category,
        module_path=dotted,
        render=render,
        sheet_files=sheet_files,
        install=install,
        render_canonical=render_canonical,
    )


def _register_multi(
    mod, stem: str, category: str, dotted: str, warnings: List[str],
) -> List[TackonTarget]:
    """A module exposing ``TARGETS = {name: {...}}`` registers many."""
    results: List[TackonTarget] = []
    for sub_name, spec in mod.TARGETS.items():
        render = spec.get("render")
        if not callable(render):
            warnings.append(
                f"{category}/{stem}: TARGETS[{sub_name!r}] missing `render`; skipped"
            )
            continue
        sheet_files = tuple(spec.get("sheet_files", default_sheet_files(sub_name)))
        install = spec.get("install")
        if install is not None and not callable(install):
            install = None
        render_canonical = spec.get("render_canonical")
        if render_canonical is not None and not callable(render_canonical):
            render_canonical = None
        results.append(TackonTarget(
            name=sub_name,
            category=category,
            module_path=dotted,
            render=render,
            sheet_files=sheet_files,
            install=install,
            render_canonical=render_canonical,
        ))
    return results


def discover_tackon_targets() -> DiscoveryReport:
    """Walk ``targets/<category>/`` and register every conformant module.

    Files that don't conform are recorded in ``warnings`` so a missing
    API is visible (via ``list-targets``) rather than silent.
    """
    targets: Dict[str, TackonTarget] = {}
    warnings: List[str] = []
    for category in CATEGORIES:
        for stem, dotted in _walk_category(category):
            if category == "characters" and stem in ADAPTER_HELPER_STEMS:
                continue
            try:
                mod = importlib.import_module(dotted)
            except Exception as ex:  # noqa: BLE001 - record + continue
                warnings.append(
                    f"{category}/{stem}: import failed "
                    f"({type(ex).__name__}: {ex})"
                )
                continue
            # Multi-target case: module declares its registry entries explicitly.
            multi = getattr(mod, "TARGETS", None)
            if isinstance(multi, dict):
                for tgt in _register_multi(mod, stem, category, dotted, warnings):
                    targets[tgt.name] = tgt
                continue
            # Single-target case: module-stem is the target id.
            render = getattr(mod, "render", None)
            if not callable(render):
                warnings.append(
                    f"{category}/{stem}: no `render(out_dir, **opts) -> Iterable[Path]` "
                    f"function (and no `TARGETS` dict) — add one to register as a "
                    f"tack-on target, or move shared helpers to the package root."
                )
                continue
            targets[stem] = _register_single(mod, stem, category, dotted, render)
    return DiscoveryReport(targets=targets, warnings=warnings)
