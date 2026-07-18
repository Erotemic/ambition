#!/usr/bin/env python3
"""Asset/tileset/entity-sprite helpers for LDtk editor visuals.

The runtime can use abstract rectangles, but humans need useful pictures in the
LDtk editor. These commands find available PNGs, report what is already wired
into LDtk, and optionally point entity definitions at tiles in registered
LDtk tilesets.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from ambition_ldtk_tools.edit.entity_layer_rules import DEFAULT_LDTK
from ambition_ldtk_tools.ldtk import (
    default_sprite_assets_dir,
    find_entity_def as _find_entity_def_or_none,
    find_tileset as _find_tileset_or_none,
    load_project,
    png_dimensions,
    rel_to_ldtk,
    repo_root_from_ldtk,
    write_project,
)
from ambition_ldtk_tools.ldtk.transaction import LdtkTransaction
from ambition_ldtk_tools.edit.visual_manifest import (
    DEFAULT_ENTITY_ICON_ORDER,
    apply_manifest,
    default_icon_manifest,
    format_issues as format_manifest_issues,
    generate_editor_icons,
    load_manifest,
    preview_manifest_html,
    save_manifest,
    validate_manifest,
)



def find_tileset(project: dict, ident: str) -> dict:
    ts = _find_tileset_or_none(project, ident)
    if ts is None:
        raise SystemExit(f"tileset {ident!r} not found")
    return ts


def find_entity_def(project: dict, ident: str) -> dict:
    ent = _find_entity_def_or_none(project, ident)
    if ent is None:
        raise SystemExit(f"entity def {ident!r} not found")
    return ent


def classify_png(rel: str) -> str:
    lower = rel.lower()
    if "/backgrounds/" in lower or lower.startswith("../../backgrounds/"):
        return "background"
    if "tileset" in lower or "/tiles/" in lower or "/tilesets/" in lower:
        return "tilesheet"
    if "spritesheet" in lower or "/sprites/" in lower:
        return "spritesheet"
    if "editor" in lower and "icon" in lower:
        return "editor_icon"
    return "png"


def asset_catalog(project: dict, ldtk: Path, assets_root: Path | None = None) -> dict[str, Any]:
    repo = repo_root_from_ldtk(ldtk)
    root = assets_root or (repo / "crates" / "ambition_actors" / "assets")
    pngs = []
    if root.exists():
        for path in sorted(root.rglob("*.png")):
            dims = png_dimensions(path)
            rel = rel_to_ldtk(ldtk, path)
            pngs.append({
                "path": str(path),
                "rel_to_ldtk": rel,
                "size": list(dims) if dims else None,
                "kind": classify_png(rel),
            })
    registered_paths = {str(ts.get("relPath")) for ts in project.get("defs", {}).get("tilesets", []) or []}
    entity_sprites = []
    for ent in project.get("defs", {}).get("entities", []) or []:
        if ent.get("tileRect") or ent.get("tilesetId") is not None:
            entity_sprites.append({
                "identifier": ent.get("identifier"),
                "tilesetId": ent.get("tilesetId"),
                "tileRect": ent.get("tileRect"),
                "renderMode": ent.get("renderMode"),
            })
    by_kind: dict[str, int] = {}
    for row in pngs:
        by_kind[row["kind"]] = by_kind.get(row["kind"], 0) + 1
    return {
        "ldtk": str(ldtk),
        "assets_root": str(root),
        "tilesets": [
            {
                "identifier": ts.get("identifier"),
                "uid": ts.get("uid"),
                "relPath": ts.get("relPath"),
                "grid": ts.get("tileGridSize"),
                "size": [ts.get("pxWid"), ts.get("pxHei")],
            }
            for ts in project.get("defs", {}).get("tilesets", []) or []
        ],
        "entity_sprites": entity_sprites,
        "pngs": pngs,
        "png_kind_counts": dict(sorted(by_kind.items())),
        "unregistered_pngs": [p for p in pngs if p["rel_to_ldtk"] not in registered_paths],
    }


def format_catalog(cat: dict[str, Any]) -> str:
    lines = ["LDtk asset catalog", f"assets root: {cat['assets_root']}", ""]
    lines.append(f"Registered tilesets ({len(cat['tilesets'])}):")
    if not cat["tilesets"]:
        lines.append("  none")
    for ts in cat["tilesets"]:
        lines.append(f"  {ts['identifier']} uid={ts['uid']} grid={ts['grid']} size={ts['size']} path={ts['relPath']}")
    lines.append("")
    lines.append(f"Entity editor sprites ({len(cat['entity_sprites'])}):")
    if not cat["entity_sprites"]:
        lines.append("  none")
    for ent in cat["entity_sprites"]:
        lines.append(f"  {ent['identifier']} tileset={ent['tilesetId']} rect={ent['tileRect']}")
    lines.append("")
    if cat.get("png_kind_counts"):
        lines.append("PNG kind counts:")
        for kind, count in cat["png_kind_counts"].items():
            lines.append(f"  {kind}: {count}")
        lines.append("")
    lines.append(f"Unregistered PNGs under assets root ({len(cat['unregistered_pngs'])}):")
    for row in cat["unregistered_pngs"][:80]:
        lines.append(f"  [{row.get('kind', 'png')}] {row['rel_to_ldtk']} size={row['size']}")
    if len(cat["unregistered_pngs"]) > 80:
        lines.append(f"  ... {len(cat['unregistered_pngs']) - 80} more")
    return "\n".join(lines) + "\n"


def link_entity_tile(project: dict, entity_id: str, tileset_id: str, rect: tuple[int, int, int, int]) -> str:
    ent = find_entity_def(project, entity_id)
    ts = find_tileset(project, tileset_id)
    x, y, w, h = rect
    ent["tilesetId"] = int(ts["uid"])
    ent["renderMode"] = "Tile"
    ent["tileRenderMode"] = "Cover"
    ent["tileRect"] = {"tilesetUid": int(ts["uid"]), "x": x, "y": y, "w": w, "h": h}
    ent["uiTileRect"] = ent["tileRect"].copy()
    return f"linked entity def {entity_id} to {tileset_id} tile rect {(x, y, w, h)}"


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="LDtk asset and editor-sprite helpers.")
    ap.add_argument("action", choices=["catalog", "link-entity-tile", "generate-editor-icons", "register-entity-icons", "suggest-manifest", "apply-manifest", "validate-manifest", "preview-manifest"])
    ap.add_argument("ldtk", type=Path, nargs="?", default=DEFAULT_LDTK)
    ap.add_argument("--assets-root", type=Path)
    ap.add_argument("--format", choices=["text", "json"], default="text")
    ap.add_argument("--entity")
    ap.add_argument("--tileset")
    ap.add_argument("--tile", help="x,y,w,h tile rect in source PNG pixels")
    ap.add_argument("--in-place", action="store_true")
    ap.add_argument("--output", type=Path)
    ap.add_argument("manifest", type=Path, nargs="?")
    ap.add_argument("--out", type=Path)
    ap.add_argument("--icons", type=Path)
    ap.add_argument("--tile-size", type=int, default=32)
    ap.add_argument("--include-entity", action="append", default=[], help="Entity identifier to include in generated/suggested icon manifest; repeatable")
    args = ap.parse_args(argv)

    project = load_project(args.ldtk)
    if args.action == "catalog":
        cat = asset_catalog(project, args.ldtk, args.assets_root)
        if args.format == "json":
            print(json.dumps(cat, indent=2, sort_keys=True))
        else:
            print(format_catalog(cat), end="")
        return 0

    if args.action == "generate-editor-icons":
        target = args.icons or args.out
        if not target:
            raise SystemExit("generate-editor-icons requires --icons or --out")
        info = generate_editor_icons(target, tile_size=args.tile_size, entities=args.include_entity or None)
        print(json.dumps(info, indent=2, sort_keys=True) if args.format == "json" else f"wrote editor icons {target} size={info['size']}")
        return 0

    if args.action == "register-entity-icons":
        # One shot: (1) (re)generate the shared editor-icon atlas from the
        # canonical order, (2) wire every entity def in THIS .ldtk to its tile.
        # The atlas PNG is a regenerated asset (gitignored, like the other
        # tileset PNGs); only the .ldtk wiring is tracked.
        icon_path = args.icons or (
            default_sprite_assets_dir(args.ldtk) / "editor_icons.png"
        )
        present = [e.get("identifier") for e in (project.get("defs", {}).get("entities") or [])]
        uncovered = [e for e in present if e not in DEFAULT_ENTITY_ICON_ORDER]
        if uncovered:
            print(
                f"warning: {len(uncovered)} entity def(s) absent from the canonical "
                f"icon order (no icon assigned): {', '.join(map(str, uncovered))}\n"
                f"  -> append them to DEFAULT_ENTITY_ICON_ORDER in visual_manifest.py"
            )
        info = generate_editor_icons(icon_path, tile_size=args.tile_size, entities=DEFAULT_ENTITY_ICON_ORDER)
        manifest = default_icon_manifest(args.ldtk, icon_path, args.tile_size, DEFAULT_ENTITY_ICON_ORDER)
        tx = LdtkTransaction(args.ldtk, in_place=args.in_place, output=args.output)
        messages = apply_manifest(tx.project, args.ldtk, manifest)
        if messages:
            tx.note_changed(messages)
        out = tx.finish(
            noop_message="register-entity-icons: no LDtk changes",
            write_message="wrote {path}",
        )
        print(f"editor icons atlas: {icon_path} size={info['size']}")
        linked = sum(1 for m in messages if m.startswith("linked "))
        skipped = sum(1 for m in messages if m.startswith("skipped "))
        print(f"linked {linked} entity icon(s), {skipped} not-in-this-ldtk")
        if out is None and messages:
            raise SystemExit("register-entity-icons requires --in-place or --output")
        return 0

    if args.action == "suggest-manifest":
        icon_path = args.icons or (default_sprite_assets_dir(args.ldtk) / "editor_icons.png")
        data = default_icon_manifest(args.ldtk, icon_path, args.tile_size, args.include_entity or None)
        if args.out:
            save_manifest(args.out, data)
            print(f"wrote {args.out}")
        else:
            print(json.dumps(data, indent=2, sort_keys=True))
        return 0

    if args.action == "preview-manifest":
        if not args.manifest:
            raise SystemExit("preview-manifest requires <manifest>")
        html = preview_manifest_html(args.ldtk, load_manifest(args.manifest))
        if args.out:
            args.out.parent.mkdir(parents=True, exist_ok=True)
            args.out.write_text(html)
            print(f"wrote {args.out}")
        else:
            print(html, end="")
        return 0

    if args.action in {"apply-manifest", "validate-manifest"}:
        if not args.manifest:
            raise SystemExit(f"{args.action} requires <manifest>")
        manifest = load_manifest(args.manifest)
        if args.action == "validate-manifest":
            issues = validate_manifest(project, args.ldtk, manifest)
            if args.format == "json":
                print(json.dumps([i.as_dict() for i in issues], indent=2, sort_keys=True))
            else:
                print(format_manifest_issues(issues), end="")
            return 1 if any(i.severity == "error" for i in issues) else 0
        tx = LdtkTransaction(args.ldtk, in_place=args.in_place, output=args.output)
        messages = apply_manifest(tx.project, args.ldtk, manifest)
        if messages:
            tx.note_changed(messages)
        out = tx.finish(
            noop_message="asset apply-manifest: no LDtk visual changes",
            write_message="wrote {path}",
        )
        for msg in messages:
            print(msg)
        if out is None and messages:
            raise SystemExit("apply-manifest requires --in-place or --output")
        return 0

    if args.action == "link-entity-tile":
        if not args.entity or not args.tileset or not args.tile:
            raise SystemExit("link-entity-tile requires --entity, --tileset, and --tile x,y,w,h")
        parts = [int(p.strip()) for p in args.tile.split(",")]
        if len(parts) != 4:
            raise SystemExit("--tile must be x,y,w,h")
        tx = LdtkTransaction(args.ldtk, in_place=args.in_place, output=args.output)
        msg = link_entity_tile(tx.project, args.entity, args.tileset, tuple(parts))
        tx.note_changed([msg])
        out = tx.finish(write_message="wrote {path}")
        if out is None:
            raise SystemExit("link-entity-tile requires --in-place or --output")
        print(f"{msg}; wrote {out}")
        return 0
    return 64


if __name__ == "__main__":
    raise SystemExit(main())
