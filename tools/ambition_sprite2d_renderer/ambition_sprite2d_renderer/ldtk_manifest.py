"""Emit an LDtk-consumable visual manifest from generated sprite sheets.

The LDtk editor only shows whatever tilesets are registered in a `.ldtk`
project. Today the sandbox/intro/cut-the-rope worlds register a single
procedural-gizmo atlas (`editor_icons.png`), so every entity shows a crude
colored shape instead of its real rendered sprite.

This module is the *producer* half of the bridge: it scans the published
sprite sheets and emits the small, stable manifest shape that
`ambition_ldtk_tools.edit.visual_manifest` already consumes
(`apply-manifest`). That consumer turns each `tilesets[]` entry into a real
LDtk tileset def (allocating uids, reading PNG dimensions, recording
`relPath`) and points entity defs at a `tileRect`.

Manifest shape (matches `visual_manifest.normalize_manifest`):

    {
      "tilesets": [
        {"identifier": "sprite_player_robot",
         "path": "crates/.../sprites/player_robot_spritesheet.png",
         "tile_width": 96, "tile_height": 112, "tags": ["sprite"]}
      ],
      "entity_icons": {
        "PlayerStart": {"tileset": "sprite_player_robot", "tile": [0, 0, 96, 112]}
      }
    }

A sheet's frame size (= the LDtk tile size) is read from the sidecar the
renderer already writes next to each PNG — either `<stem>_spritesheet.yaml`
(`frame_width`/`frame_height`, simple character sheets) or
`<stem>_spritesheet_manifest.json` (`frame_size`, boss sheets). Sheets with
no readable sidecar are skipped (we can't tell LDtk how to slice them).

Paths are emitted relative to the repo root so the committed manifest is
portable; run `apply-manifest` with the repo root as CWD so they resolve.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

# Curated entity-def -> representative sprite mapping. Conservative on
# purpose: only entities with a genuine 1:1 sprite belong here. The generic
# spawners (EnemySpawn / NpcSpawn / BossSpawn) are 1:many — a single
# representative would mislead, so per-instance editor visuals (an enum/Tile
# field that carries each placed character's frame) are the right answer and
# are deliberately left for a follow-up. APPEND clearly-1:1 entries here.
DEFAULT_ENTITY_SPRITE_MAP: dict[str, str] = {
    # The runtime player file root is `player_robot` (see
    # crates/ambition_gameplay_core/src/character_sprites/attack_hitbox.rs).
    # ONLY entities with a clear, correct, identity-specific sprite belong
    # here. The generic spawners (NpcSpawn/EnemySpawn/BossSpawn) are 1:many and
    # a single representative misleads ("which boss is this?"), so they get NO
    # sprite — they fall back to a plain colored region box. Per-instance
    # canvas art needs an LDtk-schema change (enum field carrying each
    # character's tileRect); see main-machine-review.md §2.
    "PlayerStart": "player_robot",
}

_AUX_PNG_SUFFIXES = ("_canonical", "_preview", "_debug")


def _frame_size_from_sidecar(stem: str, directory: Path) -> tuple[int, int] | None:
    """Read a sheet's (frame_width, frame_height) from its sidecar.

    Tries the boss `_spritesheet_manifest.json` (`frame_size`) first, then the
    character `_spritesheet.yaml` (`frame_width`/`frame_height`). Returns None
    when neither sidecar exists or carries a usable size.
    """
    manifest_json = directory / f"{stem}_spritesheet_manifest.json"
    if manifest_json.is_file():
        try:
            data = json.loads(manifest_json.read_text())
            fs = data.get("frame_size")
            if isinstance(fs, (list, tuple)) and len(fs) == 2:
                w, h = int(fs[0]), int(fs[1])
                if w > 0 and h > 0:
                    return w, h
        except (json.JSONDecodeError, ValueError, TypeError):
            pass

    yaml_path = directory / f"{stem}_spritesheet.yaml"
    if yaml_path.is_file():
        size = _frame_size_from_yaml(yaml_path)
        if size is not None:
            return size
    return None


def _frame_size_from_yaml(path: Path) -> tuple[int, int] | None:
    try:
        import yaml  # type: ignore
    except Exception:  # pragma: no cover - environment-dependent
        return _frame_size_from_yaml_scan(path)
    try:
        data = yaml.safe_load(path.read_text())
    except Exception:
        return None
    if not isinstance(data, dict):
        return None
    w = data.get("frame_width")
    h = data.get("frame_height")
    try:
        if w is not None and h is not None and int(w) > 0 and int(h) > 0:
            return int(w), int(h)
    except (ValueError, TypeError):
        return None
    return None


def _frame_size_from_yaml_scan(path: Path) -> tuple[int, int] | None:
    """PyYAML-free fallback: scan top-level `frame_width:`/`frame_height:`."""
    w = h = None
    for line in path.read_text().splitlines():
        if line.startswith("frame_width:"):
            try:
                w = int(line.split(":", 1)[1].strip())
            except ValueError:
                pass
        elif line.startswith("frame_height:"):
            try:
                h = int(line.split(":", 1)[1].strip())
            except ValueError:
                pass
    if w and h and w > 0 and h > 0:
        return w, h
    return None


def discover_sheets(sprites_dir: Path) -> dict[str, dict[str, Any]]:
    """Map each renderable sheet's target stem -> {path, frame_size}.

    Globs every `*_spritesheet.png` (flat sheets + boss subdirs), derives the
    target stem, and keeps only stems with a readable frame-size sidecar. That
    sidecar check naturally filters layer variants (`*_full_spritesheet.png`,
    `*_body_spritesheet.png`) and aux renders, which have no sidecar of their
    own.
    """
    sheets: dict[str, dict[str, Any]] = {}
    for png in sorted(sprites_dir.rglob("*_spritesheet.png")):
        stem = png.name[: -len("_spritesheet.png")]
        if any(stem.endswith(suffix) for suffix in _AUX_PNG_SUFFIXES):
            continue
        size = _frame_size_from_sidecar(stem, png.parent)
        if size is None:
            continue
        # First writer wins; rglob is sorted so this is deterministic.
        sheets.setdefault(stem, {"path": png, "frame_size": size})
    return sheets


def build_manifest(
    sprites_dir: Path,
    *,
    repo_root: Path,
    entity_map: dict[str, str] | None = None,
    all_sheets: bool = True,
) -> dict[str, Any]:
    """Build the LDtk visual manifest.

    `all_sheets=True` registers every discovered sheet as a tileset (the
    full, editor-browsable set). `all_sheets=False` registers only the sheets
    referenced by `entity_map`, keeping a `.ldtk` apply diff minimal.
    """
    entity_map = DEFAULT_ENTITY_SPRITE_MAP if entity_map is None else entity_map
    sheets = discover_sheets(sprites_dir)

    wanted: set[str]
    if all_sheets:
        wanted = set(sheets)
    else:
        wanted = {stem for stem in entity_map.values() if stem in sheets}

    tilesets: list[dict[str, Any]] = []
    for stem in sorted(wanted):
        info = sheets[stem]
        w, h = info["frame_size"]
        rel = _rel_to_repo(info["path"], repo_root)
        tilesets.append(
            {
                "identifier": f"sprite_{stem}",
                "path": rel,
                "tile_width": w,
                "tile_height": h,
                "tags": ["sprite"],
            }
        )

    entity_icons: dict[str, Any] = {}
    for entity_id, stem in sorted(entity_map.items()):
        if stem not in sheets:
            continue
        w, h = sheets[stem]["frame_size"]
        entity_icons[entity_id] = {
            "tileset": f"sprite_{stem}",
            # Frame 0 (top-left cell) as a stable representative icon.
            "tile": [0, 0, w, h],
        }

    return {"tilesets": tilesets, "entity_icons": entity_icons}


def _rel_to_repo(path: Path, repo_root: Path) -> str:
    try:
        return path.resolve().relative_to(repo_root.resolve()).as_posix()
    except ValueError:
        # Outside the repo (unusual) — fall back to an absolute path.
        return str(path.resolve())


def write_manifest(manifest: dict[str, Any], out_path: Path) -> Path:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    return out_path
