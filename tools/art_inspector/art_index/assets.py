"""Logical assets: the products grouped as a person thinks of them. One
character is one asset with its pages, layout, portraits, parts and every
quality tier; one parallax theme is one asset with its layers."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from . import sheets
from .scan import Product

CONFIDENCE_ORDER = {"verified": 0, "declared": 1, "derived": 2, "inferred": 3, "reference": 4, "unknown": 5}

#: A registry category, as the family of its sheets.
_CATEGORY_FAMILY = {"characters": "character", "props": "prop", "projectiles": "projectile", "tiles": "tileset", "icons": "icon"}


@dataclass
class Asset:
    id: str
    family: str
    name: str
    products: list[str] = field(default_factory=list)
    data: dict[str, Any] = field(default_factory=dict)
    generator: str | None = None
    confidence: str = "unknown"
    cover: str | None = None


def _longest_unit(name: str, units: list[str]) -> str | None:
    for unit in units:  # longest first
        if name.startswith(unit + "_"):
            return unit
    return None


def _image_family(rel: str) -> str:
    name = Path(rel).name
    if rel.startswith("entities/"):
        return "entity"
    if rel.startswith("props/"):
        return "held item"
    if rel.startswith("gnu_ton_space/") or rel.startswith("backgrounds/"):
        return "background part"
    if "contact_sheet" in name or "preview" in name or "canonical" in name:
        return "review sheet"
    if name.startswith(("ability_", "hud_", "item_")):
        return "icon"
    if "tileset" in name or name.startswith(("editor_icons", "ldtk_editor_art")):
        return "tileset"
    return "image"


def _sheet_family(key: str, category: str | None) -> str:
    base = key.rsplit("/", 1)[-1]
    if base.endswith(("_fx", "_vfx", "_explosions")) or "_fx_" in base or base.startswith("generic_"):
        return "effect"
    return _CATEGORY_FAMILY.get(category or "", "sheet")


def group(products: dict[str, Product], generators: dict[str, dict[str, Any]]) -> dict[str, Asset]:
    """Each product joins one asset. The full-quality root decides the asset;
    a tier copy joins the asset of the file it copies."""
    themes = sorted((g.split(":", 1)[1] for g in generators if g.startswith("theme:")), key=len, reverse=True)
    skins = sorted((g.split(":", 1)[1] for g in generators if g.startswith("skin:")), key=len, reverse=True)
    assets: dict[str, Asset] = {}

    def join(aid: str, family: str, name: str, product: Product) -> None:
        asset = assets.get(aid) or assets.setdefault(aid, Asset(aid, family, name))
        asset.products.append(product.path)
        product.asset = aid

    for product in sorted(products.values(), key=lambda p: (p.root.of is not None, p.path)):
        root = product.root.of or product.root.name
        rel = product.rel
        name = Path(rel).name
        directory = str(Path(rel).parent) if "/" in rel else ""
        if root == "assets":
            parts = rel.split("/")
            folder = "/".join(parts[:2]) if len(parts) > 2 else parts[0] if len(parts) > 1 else "assets"
            join(f"ref:{folder}", "reference", folder, product)
            product.role = "reference"
        elif root == "sprites":
            role = sheets.sheet_role(name)
            if role:
                key = f"{directory}/{role[0]}" if directory else role[0]
                gen = generators.get(f"target:{role[0]}", {})
                join(f"sheet:{key}", _sheet_family(key, gen.get("category")), role[0], product)
                product.role = role[1]
            else:
                join(f"image:sprites/{rel}", _image_family(rel) if product.is_image else "manifest", Path(rel).stem, product)
                product.role = "image" if product.is_image else "metadata"
        elif root == "backgrounds/parallax_layers":
            theme = _longest_unit(name, themes) if name.endswith(".png") else None
            if theme:
                join(f"parallax:{theme}", "parallax", theme, product)
                product.role = f"layer:{name[len(theme) + 1:-4]}"
            else:
                join(f"image:{root}/{rel}", "manifest", Path(rel).stem, product)
                product.role = "metadata"
        elif root == "backgrounds/sky_parts":
            join(f"sky:{Path(rel).stem}", "background part", Path(rel).stem, product)
            product.role = "image"
        elif root == "room_dressing":
            skin = _longest_unit(name, skins)
            unit = skin or name.split("_", 1)[0]
            join(f"skin:{unit}", "terrain skin", unit, product)
            product.role = f"part:{name[len(unit) + 1:-4]}" if name.endswith(".png") else "metadata"
        elif root == "sprite_packs":
            join("pack:ultrapack", "sprite pack", "ultrapack", product)
            product.role = "atlas" if product.is_image else "pack_table"
        elif root.startswith("backgrounds/"):
            profile = root.split("/", 1)[1]
            join(f"legacy_bg:{profile}", "legacy background", profile, product)
            product.role = f"layer:{Path(rel).stem}" if product.is_image else "metadata"
        else:
            join(f"image:{root}/{rel}", "image", Path(rel).stem, product)

    for asset in assets.values():
        rate(asset, products)
    return assets


#: The role of an asset's main file, by preference: its confidence is the asset's.
_PRIMARY = ("sheet_page:0", "image", "layer:sky", "part:fill", "atlas", "reference")


def rate(asset: Asset, products: dict[str, Product]) -> None:
    """The generator and confidence of an asset are its main file's; the
    weakest of its full-quality files is kept beside them."""
    full = [products[p] for p in asset.products if products[p].root.of is None] or [products[p] for p in asset.products]
    ranked = sorted(full, key=lambda p: (_PRIMARY.index(p.role) if p.role in _PRIMARY else len(_PRIMARY), p.path))
    main = next((p for p in ranked if p.is_image), ranked[0])
    asset.confidence = main.provenance.get("confidence", "unknown")
    asset.data["weakest"] = max((p.provenance.get("confidence", "unknown") for p in full), key=CONFIDENCE_ORDER.__getitem__)
    asset.generator = main.provenance.get("generator") or next((p.provenance.get("generator") for p in ranked if p.provenance.get("generator")), None)


def read_sidecars(assets: dict[str, Asset], products: dict[str, Product], cache: "sheets.SidecarCache | None" = None) -> None:
    """Read the layout, actor, portrait, part and rig sidecars of each sheet asset.
    `cache` keeps each reading by file version between builds."""
    cache = cache or sheets.SidecarCache(None)
    for asset in assets.values():
        if not asset.id.startswith("sheet:"):
            continue
        by_role = {products[p].role: products[p] for p in asset.products if products[p].root.of is None}
        layout = by_role.get("sheet_layout")
        try:
            if layout:
                asset.data["layout"] = cache.read(sheets.read_layout, layout.file)
            if "actor" in by_role:
                asset.data["actor"] = cache.read(sheets.read_actor, by_role["actor"].file)
            if "portrait_clips" in by_role:
                asset.data["portraits"] = cache.read(sheets.read_portraits, by_role["portrait_clips"].file)
            if "parts_table" in by_role:
                asset.data["parts"] = cache.read(sheets.read_parts, by_role["parts_table"].file)
            if "body_rig" in by_role:
                asset.data["body_rig"] = cache.read(sheets.read_body_rig, by_role["body_rig"].file)
        except Exception as ex:  # noqa: BLE001 - one bad sidecar is a diagnostic, not a failed build
            asset.data.setdefault("errors", []).append(f"{type(ex).__name__}: {ex}")
