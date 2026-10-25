"""Where the game uses each asset: the content data that names it (with its
file, line and row) and the LDtk placements that reach it through that data.

Read from the tracked sources, not from a running game:

* any string in a game's content data (`game/*/assets/**/*.ron`) that names an
  art file, or a sheet by target (`Sheet(target: ...)`, `prop_sheets` kinds);
* the LDtk worlds: `character_id` (through the character catalogs), `Prop.kind`
  and other prop looks (through `prop_sheets`), boss brains (through
  `boss_art_keys`), and each level's `parallax_theme`;
* Rust string literals that name an art file (a heuristic: a literal is not a
  use, and a constant built at run time is invisible to it).
"""

from __future__ import annotations

import json
import re
import subprocess
from collections import defaultdict
from pathlib import Path
from typing import Any

_ROW = re.compile(r'^\s*"([^"]+)"\s*:\s*\(')
_ART = re.compile(r'"((?:[a-z0-9_\-]+/)*[A-Za-z0-9_\-.]+\.(?:png|ron))"')
_SHEET_TARGET = re.compile(r'\btarget:\s*"([a-z0-9_]+)"')


def _tracked(top: Path, *patterns: str) -> list[Path]:
    """The tracked files of the git checkout at `top` (a submodule too) that
    match `patterns` (a `*` matches across directories)."""
    done = subprocess.run(["git", "ls-files", "-z", "--", *patterns], cwd=top, capture_output=True, text=True)
    return [top / p for p in done.stdout.split("\0") if p]


class Index:
    """Resolves a name found in content to an asset id."""

    def __init__(self, assets: dict[str, Any], products: dict[str, Any]) -> None:
        self.by_file: dict[str, str] = {}
        self.sheet_by_key: dict[str, str] = {}
        for path, product in products.items():
            if product.root.of is None and product.asset:
                self.by_file.setdefault(product.rel, product.asset)
                self.by_file.setdefault(f"{product.root.name}/{product.rel}", product.asset)
                self.by_file.setdefault(Path(product.rel).name, product.asset)
        for aid in assets:
            if aid.startswith("sheet:"):
                self.sheet_by_key.setdefault(aid.split(":", 1)[1].rsplit("/", 1)[-1], aid)
        self.assets = assets

    def file(self, ref: str) -> str | None:
        ref = ref.lstrip("./")
        for candidate in (ref, ref.split("sprites/", 1)[-1], Path(ref).name):
            if candidate in self.by_file:
                return self.by_file[candidate]
        return None

    def sheet(self, key: str) -> str | None:
        return self.sheet_by_key.get(key)


def content_uses(repo: Path, index: Index) -> tuple[list[dict[str, Any]], dict[str, dict[str, str]]]:
    """Each content line that names an asset, and the tables the LDtk pass
    needs: catalog id -> asset, prop kind -> asset, boss key -> asset."""
    uses: list[dict[str, Any]] = []
    tables: dict[str, dict[str, str]] = {"character": {}, "prop": {}, "boss": {}}
    for path in _tracked(repo, "game/*/assets/*.ron"):
        rel = path.relative_to(repo).as_posix()
        name = path.name
        row = None
        try:
            lines = path.read_text(encoding="utf8").splitlines()
        except (OSError, UnicodeDecodeError):
            continue
        for number, line in enumerate(lines, 1):
            stripped = line.split("//", 1)[0]
            m = _ROW.match(stripped)
            if m:
                row = m.group(1)
            found: list[tuple[str, str]] = []
            for ref in _ART.findall(stripped):
                aid = index.file(ref)
                if aid:
                    found.append((aid, ref))
            for key in _SHEET_TARGET.findall(stripped):
                aid = index.sheet(key)
                if aid:
                    found.append((aid, f"target {key}"))
            if name == "prop_sheets.ron" and m:
                sheet = re.search(r'sheet:\s*Some\("([^"]+)"\)', stripped)
                aid = index.sheet(sheet.group(1) if sheet else m.group(1))
                if aid:
                    found.append((aid, f"prop kind {m.group(1)}"))
                    tables["prop"][m.group(1)] = aid
            for aid, ref in found:
                uses.append({"asset": aid, "path": rel, "line": number, "row": row, "ref": ref, "kind": "content"})
                if name == "character_catalog.ron" and row:
                    tables["character"].setdefault(row, aid)
                if name == "boss_art_keys.ron" and row is None:
                    key = re.match(r'\s*"([^"]+)"\s*:', stripped)
                    if key:
                        tables["boss"][key.group(1)] = aid
    return uses, tables


def _field(entity: dict[str, Any], name: str) -> Any:
    for f in entity.get("fieldInstances", []):
        if f.get("__identifier") == name:
            return f.get("__value")
    return None


def ldtk_uses(repo: Path, index: Index, tables: dict[str, dict[str, str]]) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """Placements per asset, world and level; and the names no table resolves."""
    counts: dict[tuple[str, str, str, str], int] = defaultdict(int)
    unresolved: dict[tuple[str, str, str], int] = defaultdict(int)
    for path in _tracked(repo / "game" / "ambition_map_assets", "*.ldtk"):
        try:
            world = json.loads(path.read_text())
        except (OSError, ValueError):
            continue
        wname = path.relative_to(repo).as_posix()
        for level in world.get("levels", []):
            lname = level.get("identifier", "?")
            for f in level.get("fieldInstances", []):
                if f.get("__identifier") == "parallax_theme" and f.get("__value"):
                    theme = str(f["__value"]).lower()
                    for aid in (f"parallax:{theme}", f"skin:{theme}"):
                        if aid in index.assets:
                            counts[(aid, wname, lname, "parallax_theme")] += 1
            for layer in level.get("layerInstances") or []:
                for entity in layer.get("entityInstances", []):
                    kind = entity.get("__identifier")
                    refs: list[tuple[str, str, dict[str, str] | None]] = []
                    for fname in ("character_id",):
                        value = _field(entity, fname)
                        if value:
                            refs.append((f"{kind}.{fname}", str(value), tables["character"]))
                    for fname in ("kind", "sprite", "look", "visual"):
                        value = _field(entity, fname)
                        if isinstance(value, str) and value:
                            refs.append((f"{kind}.{fname}", value, tables["prop"]))
                    brain = _field(entity, "brain")
                    if isinstance(brain, str) and ":" in brain:
                        refs.append((f"{kind}.brain", brain.split(":", 1)[1], tables["boss"]))
                    for label, value, table in refs:
                        aid = (table or {}).get(value) or index.sheet(value) or index.sheet(value.removeprefix("npc_"))
                        if aid:
                            counts[(aid, wname, lname, label)] += 1
                        elif label.endswith(("character_id", ".brain")) or kind == "Prop":
                            unresolved[(wname, label, value)] += 1
    placements = [{"asset": a, "world": w, "level": l, "field": f, "count": n} for (a, w, l, f), n in sorted(counts.items())]
    missing = [{"world": w, "field": f, "value": v, "count": n} for (w, f, v), n in sorted(unresolved.items())]
    return placements, missing


def rust_literals(repo: Path, index: Index) -> list[dict[str, Any]]:
    """Rust string literals that name an art file, outside tests."""
    done = subprocess.run(
        ["git", "grep", "-n", "-I", "-E", r'"[a-z0-9_/\-]+\.png"', "--", "crates/*.rs", "game/*.rs", ":!*tests*", ":!*/tests/*"],
        cwd=repo, capture_output=True, text=True,
    )
    out = []
    for line in done.stdout.splitlines():
        path, number, text = line.split(":", 2)
        if text.lstrip().startswith("//"):
            continue
        for ref in re.findall(r'"([a-z0-9_/\-]+\.png)"', text):
            aid = index.file(ref)
            if aid:
                out.append({"asset": aid, "path": path, "line": int(number), "ref": ref, "kind": "rust literal"})
    return out
