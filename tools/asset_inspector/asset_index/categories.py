"""One top-level category for every image asset, so the page can list each kind of art.

The evidence, strongest first:

1. a specific index family (`assets.py`: effect, held item, entity, terrain skin, ...),
2. the renderer target's own category (`inventory` unit kind: actor, prop, ...),
3. a general index family, then where the file lives (`backgrounds/`, `room_dressing/`, ...).

An asset none of these places is `other`: the overview counts it, so a gap in
classification is visible instead of hidden. A reduced-resolution copy (a file
only in a quality tier root) keeps its original's category and is marked
`tier_copy`, so it is not counted as an asset of its own.
"""

from __future__ import annotations

from collections import Counter

#: (key, label) in the order the page shows them.
CATEGORIES = (
    ("characters", "Characters"),
    ("props", "Props"),
    ("items", "Held items"),
    ("effects", "Effects"),
    ("backgrounds", "Backgrounds"),
    ("terrain", "Terrain & tiles"),
    ("entities", "Entities & pickups"),
    ("icons", "Icons & UI"),
    ("reference", "Reference art"),
    ("other", "Other"),
)

TIER_SUFFIXES = ("_0_5x", "_0_25x", "_potato")
_UNIT_KIND = {"actor": "characters", "prop": "props", "projectile": "effects", "terrain": "terrain", "tile": "terrain",
              "background": "backgrounds", "icon": "icons", "atlas": "other"}
_PATH = (  # first match on the product path wins
    ("sprites/props/", "items"),
    ("sprites/entities/", "entities"),
    ("sprites/icons/", "icons"),
    ("backgrounds/", "backgrounds"),
    ("room_dressing/", "terrain"),
    ("sprite_packs/", "other"),
    ("assets/", "reference"),
)
_FAMILY = {"character": "characters", "prop": "props", "held item": "items", "effect": "effects", "projectile": "effects",
           "background part": "backgrounds", "legacy background": "backgrounds", "terrain skin": "terrain",
           "tileset": "terrain", "entity": "entities", "icon": "icons", "reference": "reference"}
#: Families more specific than a renderer target's category (strike effects are `props` targets).
_SPECIFIC = {"effect", "projectile", "held item", "background part", "legacy background", "entity", "terrain skin", "tileset", "reference"}
_ICON_WORDS = ("icon", "hud_", "_hud", "cursor", "button", "portrait_frame")


def categorize(catalog: dict) -> dict:
    """Set `category` and `tier_copy` on each asset row of `catalog`; return the counts per category."""
    units = (catalog.get("inventory") or {}).get("units", {})
    products = catalog.get("products", {})
    themes = _theme_rooms(catalog)
    counts: dict[str, Counter] = {key: Counter() for key, _ in CATEGORIES}
    for asset in catalog.get("assets", {}).values():
        paths = asset.get("products") or []
        # A tier root can be scanned under a full-tier name (the publication store's `backgrounds`),
        # so a folder named for a tier (`parallax_layers_0_25x`) marks a copy too.
        tiers = {"copy" if any(part.endswith(TIER_SUFFIXES) for part in path.split("/")[:-1])
                 else products.get(path, {}).get("tier", "full") for path in paths}
        asset["tier_copy"] = bool(tiers) and "full" not in tiers
        asset["category"] = _category(asset, units, paths)
        if asset["category"] == ELSEWHERE:
            continue
        if asset["category"] == "backgrounds":
            theme = max((t for t in themes if asset["name"] == t or asset["name"].startswith(t + "_")), key=len, default=None)
            if theme:
                asset["placements"] = [*asset.get("placements", []), *themes[theme]]
        row = counts[asset["category"]]
        row["tier_copies" if asset["tier_copy"] else "assets"] += 1
        if not asset["tier_copy"] and (asset.get("uses") or asset.get("placements")):
            row["used"] += 1
    # `order`: the catalog is written with sorted keys, so the page needs the order spelled out.
    return {key: {"label": label, "order": index, **counts[key]} for index, (key, label) in enumerate(CATEGORIES)}


#: Files that the Music and Fonts views own: not image assets.
ELSEWHERE = "elsewhere"


def _category(asset: dict, units: dict, paths: list[str]) -> str:
    if paths and all(path.startswith(("audio/", "fonts/")) for path in paths):
        return ELSEWHERE
    unit = units.get(asset.get("inspection_unit") or asset.get("generator") or "")
    name = (asset.get("name") or "").lower()
    if any(word in name for word in _ICON_WORDS) and asset.get("family") in ("icon", "image", "tileset", "sheet", "prop"):
        return "icons"
    if asset.get("family") in _SPECIFIC:
        return _FAMILY[asset["family"]]
    if unit and _UNIT_KIND.get(unit.get("kind")):
        kind = _UNIT_KIND[unit["kind"]]
        # A renderer prop drawn to be held is an item; the target category says prop for both.
        return "items" if kind == "props" and asset.get("family") == "held item" else kind
    if asset.get("family") in _FAMILY:
        return _FAMILY[asset["family"]]
    for path in paths:
        bare = path.split("/", 1)[-1] if path.startswith(("sprites_", "backgrounds_")) else path
        for prefix, key in _PATH:
            if path.startswith(prefix) or bare.startswith(prefix):
                return key
    return "other"


def _theme_rooms(catalog: dict) -> dict[str, list[dict]]:
    """The rooms that select each parallax theme, as placements: a room names its backdrop by
    theme (`parallax_theme: "alarm"`), and the theme's layers are named for it (`alarm_far_backplate`)."""
    rooms: dict[str, list[dict]] = {}
    for item in (catalog.get("items") or {}).values():
        if item.get("kind") != "world":
            continue
        world = (item.get("sources") or [""])[0]
        for room in item.get("details", {}).get("rooms", []):
            if room.get("parallax_theme"):
                rooms.setdefault(room["parallax_theme"], []).append(
                    {"count": 1, "field": "parallax_theme", "level": room["id"], "world": world})
    return rooms
