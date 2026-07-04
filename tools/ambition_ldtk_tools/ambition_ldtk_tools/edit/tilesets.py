#!/usr/bin/env python3
"""Register a tileset definition in an LDtk project.

Companion to `def register-entity`: that tool adds new entity types
to `defs.entities[]`; this one adds new tilesets to `defs.tilesets[]`.
A tileset is the source PNG + grid metadata that LDtk's Tiles layer
draws from. Without a registered tileset, an LDtk file's Tiles layer
has nothing to render even if the image exists on disk.

See ADR 0015 (LDtk tileset rendering) for the larger plan this
tool unblocks. The intended call sequence is:

1. `tileset add <ldtk> <png> <grid-size>` — register the tileset def.
2. Author Tiles layer instances in the LDtk editor (or via a
   future `tileset add-layer` subcommand) referencing the new uid.
3. Author tile content (hand-painted or via auto-tile rules tied to
   the Collision IntGrid).

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python -m ambition_ldtk_tools tileset add \\
    crates/ambition_content/assets/worlds/intro.ldtk \\
    crates/ambition_gameplay_core/assets/sprites/intro_lab_tileset.png \\
    16 \\
    --identifier intro_lab \\
    --in-place
```

The tool:

1. Computes `pxWid`/`pxHei` from the PNG header (stdlib `struct` —
   no Pillow dependency) and the grid-based width/height from the
   image-size + `tileGridSize`.
2. Verifies the PNG is a multiple of `tileGridSize` on both axes —
   otherwise emits a warning (LDtk silently truncates the last
   row/column of partial tiles, which is rarely what authors want).
3. Resolves `relPath` as the path from the .ldtk file's directory to
   the PNG so LDtk's editor finds it on reload.
4. Allocates a fresh `uid` and appends the new tileset to
   `defs.tilesets[]` with all required schema fields populated.
5. Runs the standard `repair --in-place` + `validate --require-schema`
   post-pass (`--no-repair` skips, same as the other edit tools).

It refuses to overwrite an existing identifier — pass a different
`--identifier` or delete the old def by hand first.
"""

from __future__ import annotations

import argparse
import json
import shutil
import struct
import subprocess
import sys
from pathlib import Path

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/tilesets.py -> repo root
REPO_ROOT = Path(__file__).resolve().parents[4]


def png_dimensions(path: Path) -> tuple[int, int]:
    """Read width + height from a PNG IHDR chunk without Pillow.

    PNG files always start with an 8-byte signature, then a 13-byte
    IHDR chunk whose first 8 payload bytes are big-endian uint32
    width + height. This is a stable layout — the PNG format hasn't
    changed since 1996.
    """
    with path.open("rb") as fh:
        signature = fh.read(8)
        if signature != b"\x89PNG\r\n\x1a\n":
            raise SystemExit(f"{path} is not a valid PNG file")
        # 4 bytes chunk length + 4 bytes chunk type ("IHDR")
        fh.read(8)
        header = fh.read(8)
        width, height = struct.unpack(">II", header)
        return int(width), int(height)


def resolve_rel_path(ldtk_path: Path, png_path: Path) -> str:
    """Relative path from the LDtk file's directory to the PNG.

    LDtk persists `relPath` as a forward-slash-separated string;
    matches the editor's normalization.
    """
    ldtk_dir = ldtk_path.resolve().parent
    png_abs = png_path.resolve()
    rel = Path(__import__("os").path.relpath(png_abs, ldtk_dir))
    return str(rel).replace("\\", "/")


def alloc_uid(project: dict) -> int:
    next_uid = int(project.get("nextUid", 1))
    project["nextUid"] = next_uid + 1
    return next_uid


def build_tileset_def(
    project: dict,
    identifier: str,
    rel_path: str,
    px_wid: int,
    px_hei: int,
    grid_size: int,
    padding: int = 0,
    spacing: int = 0,
) -> dict:
    """Synthesize one `defs.tilesets[]` entry with every field the
    LDtk JSON schema requires. Mirrors `def register-entity`'s shape
    so the validator + editor round-trip stays clean.
    """
    c_wid = px_wid // grid_size
    c_hei = px_hei // grid_size
    return {
        "__cHei": c_hei,
        "__cWid": c_wid,
        "cachedPixelData": None,
        "customData": [],
        "embedAtlas": None,
        "enumTags": [],
        "identifier": identifier,
        "padding": padding,
        "pxHei": px_hei,
        "pxWid": px_wid,
        "relPath": rel_path,
        "savedSelections": [],
        "spacing": spacing,
        "tags": [],
        "tagsSourceEnumUid": None,
        "tileGridSize": grid_size,
        "uid": alloc_uid(project),
    }


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    # The first positional must be the action verb so the
    # tileset subcommand's argparse can dispatch.
    parser.add_argument(
        "action",
        choices=["add"],
        help="Subcommand action (currently only `add`).",
    )
    parser.add_argument(
        "ldtk",
        type=Path,
        help="Target .ldtk file to modify.",
    )
    parser.add_argument(
        "png",
        type=Path,
        help="Source PNG for the tileset.",
    )
    parser.add_argument(
        "grid_size",
        type=int,
        help="Tile grid size in pixels (e.g. 16).",
    )
    parser.add_argument(
        "--identifier",
        type=str,
        default=None,
        help=(
            "Identifier for the tileset (used by Tiles-layer authoring "
            "to reference it). Defaults to the PNG stem with a "
            "trailing `_tileset` stripped."
        ),
    )
    parser.add_argument(
        "--padding",
        type=int,
        default=0,
        help="Padding (px) from image borders. Default 0.",
    )
    parser.add_argument(
        "--spacing",
        type=int,
        default=0,
        help="Spacing (px) between tiles. Default 0.",
    )
    parser.add_argument(
        "--in-place",
        action="store_true",
        help="Write back to the input .ldtk path.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="Output path (alternative to --in-place).",
    )
    parser.add_argument(
        "--backup",
        action="store_true",
        help="When using --in-place, copy the original to <ldtk>.bak first.",
    )
    parser.add_argument(
        "--no-repair",
        action="store_true",
        help="Skip the repair + validate post-pass.",
    )
    parser.add_argument(
        "--schema",
        type=Path,
        default=REPO_ROOT
        / "tools"
        / "ambition_ldtk_tools"
        / "schemas"
        / "ldtk"
        / "JSON_SCHEMA.json",
    )
    args = parser.parse_args(argv)

    if args.action != "add":
        return _fail(f"unknown tileset action '{args.action}'")
    if not args.in_place and args.output is None:
        return _fail("choose --in-place or --output <path>")
    if args.grid_size <= 0:
        return _fail("--grid-size must be a positive integer")
    if not args.ldtk.exists():
        return _fail(f"ldtk file not found: {args.ldtk}")
    if not args.png.exists():
        return _fail(f"png file not found: {args.png}")

    # Compute identifier from PNG stem if not given. Strip a trailing
    # `_tileset` so `intro_lab_tileset.png` -> `intro_lab` rather than
    # `intro_lab_tileset` (the registry-key naming convention).
    identifier = args.identifier
    if identifier is None:
        stem = args.png.stem
        identifier = stem[: -len("_tileset")] if stem.endswith("_tileset") else stem

    px_wid, px_hei = png_dimensions(args.png)
    if px_wid % args.grid_size != 0 or px_hei % args.grid_size != 0:
        print(
            f"warning: PNG {args.png.name} ({px_wid}x{px_hei}) is not a "
            f"multiple of grid_size {args.grid_size}. LDtk truncates "
            f"partial tiles at the right/bottom edge.",
            file=sys.stderr,
        )

    project = json.loads(args.ldtk.read_text())
    defs = project.setdefault("defs", {})
    tilesets = defs.setdefault("tilesets", [])
    if any(t.get("identifier") == identifier for t in tilesets):
        return _fail(
            f"tileset identifier '{identifier}' already exists in {args.ldtk}; "
            f"pass --identifier to disambiguate or delete the old def first."
        )

    rel_path = resolve_rel_path(args.ldtk, args.png)
    tileset_def = build_tileset_def(
        project=project,
        identifier=identifier,
        rel_path=rel_path,
        px_wid=px_wid,
        px_hei=px_hei,
        grid_size=args.grid_size,
        padding=args.padding,
        spacing=args.spacing,
    )
    tilesets.append(tileset_def)
    print(
        f"added tileset def: {identifier} (uid={tileset_def['uid']}, "
        f"{px_wid}x{px_hei}, grid={args.grid_size}, "
        f"relPath={rel_path})"
    )

    target = args.output or args.ldtk
    if args.in_place and args.backup:
        backup = args.ldtk.with_suffix(args.ldtk.suffix + ".bak")
        shutil.copy2(args.ldtk, backup)
        print(f"wrote backup: {backup}")

    # Use the editor-style dumper so the resulting file passes
    # roundtrip without churn — same path the other edit tools use.
    from ambition_ldtk_tools.editor_format import dump_editor_style

    target.write_text(dump_editor_style(project))
    print(f"wrote {target}")

    if args.no_repair:
        return 0

    cmd = [
        sys.executable,
        "-m",
        "ambition_ldtk_tools.repair",
        str(target),
        "--in-place",
    ]
    print("$ " + " ".join(cmd))
    rc = subprocess.run(cmd).returncode
    if rc != 0:
        return rc
    cmd = [sys.executable, "-m", "ambition_ldtk_tools.validate", str(target)]
    if args.schema and args.schema.exists():
        cmd.extend(["--schema", str(args.schema), "--require-schema"])
    print("$ " + " ".join(cmd))
    return subprocess.run(cmd).returncode


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
