#!/usr/bin/env python3
"""The renderer source files each sprite target reads, for its cache key.

`scripts/regen/sprites.sh` keys each sheet on the shared renderer core plus
the files of that target. The family helpers under `targets/` (`_*.py`) were
part of the shared core, so an edit to one family's helper (the crew's
`_person_rig.py`) made every sheet stale, and a publish after it rendered the
whole roster again: about one hour (measured 2026-10-13). Two kinds of file
that a target reads were in no key at all: the modules of a helper package
that loads them by name (`_toon_presets/`), and a shared module whose name has
no underscore (`targets/super_mary_o_common.py`).

So a target's files are the closure of its static imports inside `targets/`.
A module that imports by name (`importlib.import_module`, `__import__`,
`pkgutil.iter_modules`) reads modules that no import statement names, so it
brings every module of its own package. The core (`core/`, `authoring/`,
`registry/`, `cli/`, the package top level and each `__init__.py`) stays in
the shared key.

A target's root is what the registry says it is made of: the module that
defines it (all of it, when that module is a package), or the config file of
a config-authored target. One module can define several targets (the crew),
so a file name is not a target name.

The core runs for every target, so the files under `targets/` that the core
imports are in the shared key too (`core` mode); the toon presets are there.

Usage, from the renderer directory with the renderer's interpreter:

    sprite_target_closure.py core RENDERER_DIR
        prints each file under `targets/` that the core reaches.
    sprite_target_closure.py targets RENDERER_DIR [NAME ...]
        prints `NAME<TAB>PATH` for each file of each target (every registered
        target when no NAME is given).

PATH is relative to RENDERER_DIR. A file that does not parse is an error
(exit 2): a target that cannot be read cannot be drawn. A renderer that does
not import is exit 3, as in `sprite_install_names.py`.
"""

from __future__ import annotations

import ast
import functools
import sys
from pathlib import Path

PACKAGE = "ambition_sprite2d_renderer"

#: A module that names one of these reads modules no import statement names.
BY_NAME = ("import_module(", "__import__(", "iter_modules(")


def _module_name(renderer: Path, file: Path) -> str:
    parts = list(file.relative_to(renderer).with_suffix("").parts)
    if parts[-1] == "__init__":
        parts.pop()
    return ".".join(parts)


def _module_file(renderer: Path, dotted: str) -> Path | None:
    base = renderer.joinpath(*dotted.split("."))
    if base.with_suffix(".py").is_file():
        return base.with_suffix(".py")
    if (base / "__init__.py").is_file():
        return base / "__init__.py"
    return None


@functools.cache
def _imports(renderer: Path, file: Path) -> tuple[Path, ...]:
    """The files `file` imports, resolved against the renderer tree. Cached:
    every target of a family reads that family's helpers."""
    tree = ast.parse(file.read_text(), filename=str(file))
    package = _module_name(renderer, file)
    if file.name != "__init__.py":
        package = package.rpartition(".")[0]
    found: list[Path] = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                if (hit := _module_file(renderer, alias.name)) is not None:
                    found.append(hit)
        elif isinstance(node, ast.ImportFrom):
            base = package
            for _ in range(max(node.level - 1, 0)):
                base = base.rpartition(".")[0]
            if node.level == 0:
                base = node.module or ""
            elif node.module:
                base = f"{base}.{node.module}"
            if (hit := _module_file(renderer, base)) is not None:
                found.append(hit)
            for alias in node.names:
                if (hit := _module_file(renderer, f"{base}.{alias.name}")) is not None:
                    found.append(hit)
    if any(marker in file.read_text() for marker in BY_NAME):
        found.extend(sorted(file.parent.glob("*.py")))
    return tuple(found)


def core_files(renderer: Path) -> list[Path]:
    """The renderer core: the package top level and `core/`, `authoring/`,
    `registry/` and `cli/`, as `compute_core_shared` hashes them."""
    package = renderer / PACKAGE
    files = sorted(package.glob("*.py"))
    for part in ("core", "authoring", "registry", "cli"):
        files.extend(sorted((package / part).rglob("*.py")))
    return files


def target_roots(renderer: Path, registry: dict) -> dict[str, list[Path]]:
    """Each registered target and the files it is made of, by the registry:
    its module (each file of it, for a package) or its config file."""
    roots: dict[str, list[Path]] = {}
    for name, target in registry.items():
        module = getattr(target, "module_path", None)
        config = getattr(target, "_config_path", None)
        if module:
            file = _module_file(renderer, module)
            if file is None:
                continue
            roots[name] = sorted(file.parent.rglob("*.py")) if file.name == "__init__.py" else [file]
        elif config:
            roots[name] = [Path(config).resolve()]
    return roots


def closure(renderer: Path, roots: list[Path]) -> list[Path]:
    """`roots` and every file under `targets/` they import, transitively. A
    root that is not Python (a config file) is part of the key and is not
    read for imports.

    The core is not followed: each core file is a root of the `core` mode
    closure, so a helper that a target reaches through the core is in the
    shared key already."""
    targets = renderer / PACKAGE / "targets"
    seen: set[Path] = set()
    todo = list(roots)
    while todo:
        file = todo.pop()
        if file in seen:
            continue
        seen.add(file)
        if file.suffix != ".py":
            continue
        for hit in _imports(renderer, file):
            if targets in hit.parents and hit not in seen:
                todo.append(hit)
    return sorted(file for file in seen if targets in file.parents or file in roots)


def _registry() -> dict | None:
    try:
        from ambition_sprite2d_renderer.cli.commands import discover_all_targets
    except Exception:
        return None
    result = discover_all_targets()
    if isinstance(result, dict):
        return result
    return next((element for element in result if isinstance(element, dict)), None)


def main(argv: list[str]) -> int:
    if len(argv) < 2 or argv[0] not in ("core", "targets"):
        print(__doc__, file=sys.stderr)
        return 2
    mode, renderer = argv[0], Path(argv[1]).resolve()
    try:
        if mode == "core":
            for file in closure(renderer, core_files(renderer)):
                if renderer / PACKAGE / "targets" in file.parents:
                    print(file.relative_to(renderer))
            return 0
        if str(renderer) not in sys.path:
            sys.path.insert(0, str(renderer))
        registry = _registry()
        if registry is None:
            print("sprite_target_closure: the sprite renderer is not importable here", file=sys.stderr)
            return 3
        roots = target_roots(renderer, registry)
        for name in argv[2:] or sorted(roots):
            for file in closure(renderer, roots.get(name, [])):
                print(f"{name}\t{file.relative_to(renderer)}")
    except SyntaxError as error:
        print(f"sprite_target_closure: {error.filename}: {error.msg} (line {error.lineno})", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
