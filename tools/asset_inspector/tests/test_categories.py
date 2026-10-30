"""Every image asset gets one top-level category, and what no rule places is counted as other."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from asset_index import categories


def asset(aid, family, products, **extra):
    return {"id": aid, "name": aid.split(":")[-1].split("/")[-1].removesuffix(".png"), "family": family, "products": products, "uses": [], "placements": [], **extra}


def test_each_asset_gets_one_category_and_tier_copies_fold_into_their_originals():
    catalog = {
        "inventory": {"units": {"target:goblin": {"kind": "actor"}, "target:crate": {"kind": "prop"}, "target:fx": {"kind": "prop"}}},
        "products": {"backgrounds/parallax_layers/alarm_sky.png": {"tier": "full"}, "backgrounds/parallax_layers_0_25x/alarm_sky.png": {"tier": "full"}},
        "items": {"world:hub": {"kind": "world", "sources": ["worlds/hub.ldtk"], "details": {"rooms": [{"id": "lobby", "parallax_theme": "alarm"}]}}},
        "assets": {a["id"]: a for a in [
            asset("sheet:goblin", "character", ["sprites/goblin_actor.ron"], generator="target:goblin"),
            asset("sheet:crate", "prop", ["sprites/crate_actor.ron"], generator="target:crate", uses=[{"path": "x"}]),
            asset("sheet:strike_fx", "effect", ["sprites/strike_fx_actor.ron"], generator="target:fx"),
            asset("image:backgrounds/parallax_layers/alarm_sky.png", "manifest", ["backgrounds/parallax_layers/alarm_sky.png"]),
            asset("image:backgrounds/parallax_layers_0_25x/alarm_sky.png", "image", ["backgrounds/parallax_layers_0_25x/alarm_sky.png"]),
            asset("image:sprites/entities/pickup_health.png", "entity", ["sprites/entities/pickup_health.png"]),
            asset("image:audio/music/theme.json", "image", ["audio/music/theme.json"]),
            asset("image:sprites/mystery.json", "image", ["sprites/mystery.json"]),
        ]}}
    summary = categories.categorize(catalog)
    got = {a["id"]: a["category"] for a in catalog["assets"].values()}
    assert got["sheet:goblin"] == "characters" and got["sheet:crate"] == "props"
    assert got["sheet:strike_fx"] == "effects", "a specific family beats the target's category"
    assert got["image:sprites/entities/pickup_health.png"] == "entities"
    assert got["image:audio/music/theme.json"] == categories.ELSEWHERE
    assert got["image:sprites/mystery.json"] == "other" and summary["other"]["assets"] == 1
    copy = catalog["assets"]["image:backgrounds/parallax_layers_0_25x/alarm_sky.png"]
    assert copy["category"] == "backgrounds" and copy["tier_copy"]
    assert {k: v for k, v in summary["backgrounds"].items() if k != "order"} == {"label": "Backgrounds", "assets": 1, "tier_copies": 1, "used": 1}
    assert catalog["assets"]["image:backgrounds/parallax_layers/alarm_sky.png"]["placements"][0]["level"] == "lobby"
    assert summary["props"]["used"] == 1
