#!/usr/bin/env python3
"""**Render a dressed `.ldtk` the way the editor will draw it.**

⚠ **this is an instrument, and it is worth being precise about what it can
tell you.** It re-evaluates the auto-layer rules `editor_art` wrote, using the
rule fields' documented meaning, and blits the atlas through the same tile ids
and `tileRect`s the `.ldtk` now carries. So it proves the wiring: that a rule
addresses the tile someone meant, that the four quadrants of a 32px texture land
in the right cells, that an entity's rect is its own art and not its neighbour's.

It CANNOT prove LDtk agrees. The editor is the source of truth for its own file
format, and the only way to see its evaluation is to open it. A green preview
means the art and the ids are right; it does not mean the rule *fields* are
spelled the way LDtk 1.5.3 wants them.

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python3 -m ambition_ldtk_tools.edit.editor_art_preview \\
    game/ambition_demo_mary_o/assets/worlds/mary_o.ldtk out.png --level mary_o_1_1
```
"""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

from ambition_ldtk_tools.ldtk import (
    default_sprite_assets_dir,
    layer_defs,
    load_project,
    path_from_ldtk,
)

# : The editor's own backdrop, so the preview reads as the editor rather than as
# : the game (which draws its own sky).
BACKDROP = (38, 42, 56, 255)
GRID = (54, 60, 78, 255)


def _rule_matches(rule: dict[str, Any], value: int, cx: int, cy: int) -> bool:
    """Does this size-1 rule fire on a cell?

    Only the shape `editor_art` writes is understood: a single-cell pattern
    plus a modulo/offset phase. A rule of any other shape is skipped rather
    than guessed at — a preview that invents semantics is worse than one that
    admits it did not draw something.
    """
    if int(rule.get("size", 1)) != 1:
        return False
    pattern = rule.get("pattern") or []
    if len(pattern) != 1 or int(pattern[0]) != value:
        return False
    x_mod = max(1, int(rule.get("xModulo", 1)))
    y_mod = max(1, int(rule.get("yModulo", 1)))
    return (cx - int(rule.get("xOffset", 0))) % x_mod == 0 and (
        cy - int(rule.get("yOffset", 0))
    ) % y_mod == 0


def _instance_tile_rect(
    project: dict[str, Any], definition: dict[str, Any], entity: dict[str, Any]
) -> dict[str, Any] | None:
    """The tile THIS placement wears, when a field decides it.

    A field with `editorDisplayMode: "EntityTile"` overrides the def's own tile
    with the tile of the enum value the instance holds — which is how a brick
    and a `?`-block are one entity def with two looks.
    """
    for field in definition.get("fieldDefs") or []:
        if field.get("editorDisplayMode") != "EntityTile":
            continue
        enum_identifier = str(field.get("__type") or "").removeprefix("LocalEnum.")
        enum = next(
            (
                candidate
                for candidate in project.get("defs", {}).get("enums") or []
                if candidate.get("identifier") == enum_identifier
            ),
            None,
        )
        if enum is None:
            continue
        held = next(
            (
                instance.get("__value")
                for instance in entity.get("fieldInstances") or []
                if instance.get("__identifier") == field.get("identifier")
            ),
            None,
        )
        value = next(
            (v for v in enum.get("values") or [] if v.get("id") == held), None
        )
        if value and value.get("tileRect"):
            return value["tileRect"]
    return None


def _tileset_by_uid(project: dict[str, Any], uid: int | None) -> dict[str, Any] | None:
    if uid is None:
        return None
    for tileset in project.get("defs", {}).get("tilesets", []):
        if int(tileset.get("uid", -1)) == int(uid):
            return tileset
    return None


def render_preview(
    ldtk: Path,
    out: Path,
    *,
    level: str | None = None,
    sprites_dir: Path | None = None,
    scale: int = 1,
) -> Path:
    from PIL import Image, ImageDraw

    project = load_project(ldtk)
    levels = project.get("levels", [])
    if level is not None:
        levels = [lvl for lvl in levels if lvl.get("identifier") == level]
        if not levels:
            raise SystemExit(f"level {level!r} not found in {ldtk.name}")

    images: dict[int, Any] = {}

    def tileset_image(tileset: dict[str, Any]) -> Any:
        uid = int(tileset["uid"])
        if uid not in images:
            path = path_from_ldtk(ldtk, str(tileset.get("relPath") or ""))
            if not path.is_file() and sprites_dir is not None:
                path = sprites_dir / Path(str(tileset.get("relPath"))).name
            if not path.is_file():
                raise SystemExit(f"tileset {tileset['identifier']}: no PNG at {path}")
            images[uid] = Image.open(path).convert("RGBA")
        return images[uid]

    layer_by_uid = {int(layer["uid"]): layer for layer in layer_defs(project)}
    entity_by_id = {
        str(ent["identifier"]): ent for ent in project.get("defs", {}).get("entities", [])
    }

    rendered: list[Any] = []
    for lvl in levels:
        width, height = int(lvl["pxWid"]), int(lvl["pxHei"])
        canvas = Image.new("RGBA", (width, height), BACKDROP)
        draw = ImageDraw.Draw(canvas)
        for x in range(0, width, 16):
            draw.line((x, 0, x, height), fill=GRID)
        for y in range(0, height, 16):
            draw.line((0, y, width, y), fill=GRID)

        # Bottom-up, exactly like LDtk: the layer list is top-first.
        for instance in reversed(lvl.get("layerInstances", [])):
            definition = layer_by_uid.get(int(instance.get("layerDefUid", -1)))
            if definition is None:
                continue
            if definition.get("type") == "AutoLayer":
                # An AutoLayer owns no cells: it answers another layer's
                # IntGrid, so the values come from the SOURCE instance.
                source_uid = definition.get("autoSourceLayerDefUid")
                source = next(
                    (
                        inst
                        for inst in lvl.get("layerInstances", [])
                        if inst.get("layerDefUid") == source_uid
                    ),
                    None,
                )
                tileset = _tileset_by_uid(project, definition.get("tilesetDefUid"))
                if source is None or tileset is None:
                    continue
                sheet = tileset_image(tileset)
                grid = int(tileset.get("tileGridSize") or 16)
                cols = max(1, int(tileset.get("__cWid") or 1))
                cell = int(definition.get("gridSize") or 16)
                c_wid = int(source.get("__cWid") or 0)
                rules = [
                    rule
                    for group in definition.get("autoRuleGroups") or []
                    if group.get("active", True)
                    for rule in group.get("rules") or []
                    if rule.get("active", True)
                ]
                for index, value in enumerate(source.get("intGridCsv") or []):
                    if not value or not c_wid:
                        continue
                    cx, cy = index % c_wid, index // c_wid
                    for rule in rules:
                        if not _rule_matches(rule, int(value), cx, cy):
                            continue
                        rects = rule.get("tileRectsIds") or []
                        if not rects or not rects[0]:
                            break
                        tile_id = int(rects[0][0])
                        sx, sy = (tile_id % cols) * grid, (tile_id // cols) * grid
                        tile = sheet.crop((sx, sy, sx + grid, sy + grid))
                        canvas.alpha_composite(tile, (cx * cell, cy * cell))
                        if rule.get("breakOnMatch", True):
                            break
            if definition.get("type") == "IntGrid":
                # `displayOpacity` is the editor's own slider, so the preview reads the same
                # number the editor will.
                cell = int(instance.get("__gridSize") or definition.get("gridSize") or 16)
                c_wid = int(instance.get("__cWid") or 0)
                # the INACTIVE alpha, because that is the state you look at a
                # level in — some other layer is current while you place things
                # and read the shape. Selecting the collision layer in the
                # editor takes it to `displayOpacity` and the art disappears
                # behind it, which is what painting wants and what a still
                # picture cannot show two of.
                opacity = float(
                    definition.get("displayOpacity", 1.0) or 0.0
                ) * float(definition.get("inactiveOpacity", 1.0) or 0.0)
                colours = {
                    int(value["value"]): str(value.get("color") or "#FFFFFF")
                    for value in definition.get("intGridValues") or []
                }
                if opacity > 0 and c_wid:
                    wash = Image.new("RGBA", (cell, cell), (0, 0, 0, 0))
                    for index, value in enumerate(instance.get("intGridCsv") or []):
                        colour = colours.get(int(value or 0))
                        if not colour:
                            continue
                        rgb = tuple(int(colour.lstrip("#")[i : i + 2], 16) for i in (0, 2, 4))
                        wash.paste(rgb + (int(255 * opacity),), (0, 0, cell, cell))
                        canvas.alpha_composite(
                            wash, ((index % c_wid) * cell, (index // c_wid) * cell)
                        )
            for entity in instance.get("entityInstances", []):
                definition = entity_by_id.get(str(entity.get("__identifier")))
                if definition is None:
                    continue
                px = entity.get("px") or [0, 0]
                w, h = int(entity.get("width", 16)), int(entity.get("height", 16))
                rect = _instance_tile_rect(project, definition, entity) or definition.get(
                    "tileRect"
                )
                if not rect:
                    colour = str(definition.get("color") or "#FFFFFF").lstrip("#")
                    rgb = tuple(int(colour[i : i + 2], 16) for i in (0, 2, 4))
                    draw.rectangle(
                        (px[0], px[1], px[0] + w, px[1] + h), outline=rgb + (255,), width=1
                    )
                    continue
                tileset = _tileset_by_uid(project, rect.get("tilesetUid"))
                if tileset is None:
                    continue
                sheet = tileset_image(tileset)
                art = sheet.crop(
                    (
                        int(rect["x"]),
                        int(rect["y"]),
                        int(rect["x"]) + int(rect["w"]),
                        int(rect["y"]) + int(rect["h"]),
                    )
                )
                if definition.get("tileRenderMode") == "FitInside":
                    ratio = min(w / art.width, h / art.height)
                    size = (max(1, round(art.width * ratio)), max(1, round(art.height * ratio)))
                    art = art.resize(size, Image.NEAREST)
                    offset = (px[0] + (w - size[0]) // 2, px[1] + (h - size[1]) // 2)
                else:
                    art = art.resize((max(1, w), max(1, h)), Image.NEAREST)
                    offset = (px[0], px[1])
                canvas.alpha_composite(art, offset)
        rendered.append(canvas)

    if not rendered:
        raise SystemExit(f"{ldtk.name}: nothing to preview")
    width = max(image.width for image in rendered)
    height = sum(image.height for image in rendered) + 8 * (len(rendered) - 1)
    sheet = Image.new("RGBA", (width, height), (20, 22, 30, 255))
    y = 0
    for image in rendered:
        sheet.alpha_composite(image, (0, y))
        y += image.height + 8
    if scale != 1:
        sheet = sheet.resize((sheet.width * scale, sheet.height * scale), Image.NEAREST)
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="ambition_ldtk_tools.edit.editor_art_preview")
    parser.add_argument("ldtk", type=Path)
    parser.add_argument("out", type=Path)
    parser.add_argument("--level", default=None)
    parser.add_argument("--scale", type=int, default=1)
    args = parser.parse_args(argv)
    path = render_preview(
        args.ldtk,
        args.out,
        level=args.level,
        sprites_dir=default_sprite_assets_dir(args.ldtk),
        scale=args.scale,
    )
    print(f"wrote {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
