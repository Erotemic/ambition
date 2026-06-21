"""Sparse actor-contract sidecars for generated sprite sheets.

The renderer's long-standing ``*_spritesheet.ron`` files describe pixel
layout: image path, frame rectangles, row names, durations, body metrics.
This module emits a *second*, optional sidecar, ``*_actor.ron``, that describes
how those pixels may be used by the game: stable character identity, optional
body/capability hints, sparse sockets, default brain/action presets, and
animation/action bindings.

The sandbox does not consume these files yet. The design goal is to let the
renderer start producing rich, inspectable data without disturbing the current
runtime loader. Every field is intentionally optional or inferred: generated
characters can be zombies with no hands, props with no traversal, bosses with
bespoke sheets, or humanoids with conventional locomotion rows.
"""
from __future__ import annotations

from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path
import re
from typing import Any, Dict, Iterable, Mapping, MutableMapping, Sequence



# ---- Tiny RON emitter -------------------------------------------------------
#
# Kept local for the same reason as the sheet emitters: the shape is small and
# easy to inspect in diffs, and the renderer intentionally avoids a python-ron
# dependency.  These wrapper classes let us distinguish RON structs, maps, and
# Some(...) options while still building the contract as ordinary Python data.


@dataclass(frozen=True)
class RonSome:
    value: Any


@dataclass(frozen=True)
class RonStruct:
    fields: Mapping[str, Any]


@dataclass(frozen=True)
class RonMap:
    values: Mapping[str, Any]


def some(value: Any) -> RonSome:
    return RonSome(value)


def struct(**fields: Any) -> RonStruct:
    return RonStruct({k: v for k, v in fields.items()})


def ron_map(values: Mapping[str, Any] | None = None) -> RonMap:
    return RonMap(dict(values or {}))


def _ron_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def _ron_atom(value: Any, indent: int = 0) -> str:
    pad = " " * indent
    child = indent + 4
    if isinstance(value, RonSome):
        return f"Some({_ron_atom(value.value, indent)})"
    if isinstance(value, RonStruct):
        if not value.fields:
            return "()"
        lines = ["("]
        for key, item in value.fields.items():
            lines.append(f"{' ' * child}{key}: {_ron_atom(item, child)},")
        lines.append(f"{pad})")
        return "\n".join(lines)
    if isinstance(value, RonMap):
        if not value.values:
            return "{}"
        lines = ["{"]
        for key in sorted(value.values):
            lines.append(
                f"{' ' * child}\"{_ron_escape(str(key))}\": {_ron_atom(value.values[key], child)},"
            )
        lines.append(f"{pad}}}")
        return "\n".join(lines)
    if value is None:
        return "None"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        # Always include a decimal point so RON/Rust readers don't infer int.
        text = f"{value:.6f}".rstrip("0").rstrip(".")
        if "." not in text:
            text += ".0"
        return text
    if isinstance(value, str):
        return f'"{_ron_escape(value)}"'
    if isinstance(value, Mapping):
        return _ron_atom(ron_map(value), indent)
    if isinstance(value, (list, tuple)):
        if not value:
            return "[]"
        lines = ["["]
        for item in value:
            lines.append(f"{' ' * child}{_ron_atom(item, child)},")
        lines.append(f"{pad}]")
        return "\n".join(lines)
    raise TypeError(f"unsupported RON value {value!r} ({type(value).__name__})")


def to_ron(value: RonStruct) -> str:
    return (
        "// Auto-emitted optional actor contract. Current sandbox builds ignore\n"
        "// this file; it is the renderer -> engine sidecar for future actor\n"
        "// spec ingestion. Keep fields sparse: capabilities create requirements,\n"
        "// and missing data should be explicit rather than guessed silently.\n"
        f"{_ron_atom(value)}\n"
    )



# ---- Catalog defaults --------------------------------------------------------
#
# The Python renderer can run as a standalone package, so the actor contract
# cannot *require* the sandbox catalog.  When the repo is present, however, the
# catalog is the best source of existing character identity / default brain /
# action-set tags.  Use it as a soft enrichment layer.  Hand-authored YAML /
# module metadata still wins.

@dataclass(frozen=True)
class CatalogProfile:
    character_id: str
    display_name: str | None = None
    spritesheet: str | None = None
    manifest: str | None = None
    body_kind: str | None = None
    default_brain: str | None = None
    default_action_set: str | None = None
    tags: tuple[str, ...] = ()


def _repo_root() -> Path | None:
    for parent in Path(__file__).resolve().parents:
        if (parent / "crates" / "ambition_gameplay_core" / "assets" / "data" / "character_catalog.ron").exists():
            return parent
    return None


def _catalog_path() -> Path | None:
    root = _repo_root()
    if root is None:
        return None
    path = root / "crates" / "ambition_gameplay_core" / "assets" / "data" / "character_catalog.ron"
    return path if path.exists() else None


def _extract_string(block: str, key: str) -> str | None:
    match = re.search(rf"{re.escape(key)}:\s*\"([^\"]*)\"", block)
    return match.group(1) if match else None


def _extract_ident(block: str, key: str) -> str | None:
    match = re.search(rf"{re.escape(key)}:\s*([A-Za-z_][A-Za-z0-9_]*)", block)
    return match.group(1) if match else None


def _extract_tags(block: str) -> tuple[str, ...]:
    match = re.search(r"tags:\s*\[(.*?)\]", block, flags=re.S)
    if not match:
        return ()
    return tuple(re.findall(r'"([^"]*)"', match.group(1)))


@lru_cache(maxsize=1)
def _load_catalog_profiles() -> dict[str, CatalogProfile]:
    """Load sandbox catalog identity defaults if this package is in-repo.

    Keys include both catalog character IDs and spritesheet stems.  The parser is
    intentionally narrow and tolerant: it reads only the flat fields currently
    needed by the actor sidecar and silently returns an empty map when the repo
    catalog is unavailable.
    """
    path = _catalog_path()
    if path is None:
        return {}
    text = path.read_text(encoding="utf8")
    profiles: dict[str, CatalogProfile] = {}
    entry_re = re.compile(r'"([^"]+)"\s*:\s*\((.*?)\n\s{8}\),', flags=re.S)
    for match in entry_re.finditer(text):
        character_id = match.group(1)
        block = match.group(2)
        spritesheet = _extract_string(block, "spritesheet")
        profile = CatalogProfile(
            character_id=character_id,
            display_name=_extract_string(block, "display_name"),
            spritesheet=spritesheet,
            manifest=_extract_string(block, "manifest"),
            body_kind=_extract_ident(block, "body_kind"),
            default_brain=_extract_string(block, "default_brain"),
            default_action_set=_extract_string(block, "default_action_set"),
            tags=_extract_tags(block),
        )
        profiles[character_id] = profile
        if spritesheet:
            stem = _spritesheet_stem(Path(spritesheet).name)
            profiles.setdefault(stem, profile)
    return profiles


def _catalog_profile_for(stem: str, explicit_character_id: str | None = None) -> CatalogProfile | None:
    profiles = _load_catalog_profiles()
    if explicit_character_id and explicit_character_id in profiles:
        return profiles[explicit_character_id]
    return profiles.get(stem)


# ---- Contract inference -----------------------------------------------------


BASE_CHARACTER_IDS = {
    "player_robot": "player",
    "robot": "robot",
    "goblin": "goblin",
    "sandbag": "sandbag",
}

IDLE_CANDIDATES = ("idle", "rest", "front_idle", "side_idle", "opening", "stable")
WALK_CANDIDATES = ("walk", "side_walk", "shamble", "stable")
MELEE_CANDIDATES = (
    "slash",
    "attack_side",
    "bite",
    "floor_slam",
    "side_sweep",
    "stomp",
)
RANGED_CANDIDATES = ("shoot", "aim", "cast", "spike_halo")
HIT_CANDIDATES = ("hit", "hurt")
DEATH_CANDIDATES = ("death",)


def _spritesheet_stem(path: str | Path) -> str:
    stem = Path(path).stem
    return stem[:-12] if stem.endswith("_spritesheet") else stem


def actor_sidecar_path_for_image(image_path: str | Path) -> Path:
    path = Path(image_path)
    return path.with_name(f"{_spritesheet_stem(path)}_actor.ron")


def _humanize(stem: str) -> str:
    return " ".join(part.capitalize() for part in stem.replace("npc_", "").split("_") if part)


def _as_mapping(value: Any) -> Dict[str, Any]:
    return dict(value) if isinstance(value, Mapping) else {}


def _deep_merge(base: Dict[str, Any], overlay: Mapping[str, Any]) -> Dict[str, Any]:
    result = dict(base)
    for key, value in overlay.items():
        if isinstance(value, Mapping) and isinstance(result.get(key), Mapping):
            result[key] = _deep_merge(dict(result[key]), value)
        else:
            result[key] = value
    return result


def _rows_from_manifest(manifest: Mapping[str, Any]) -> list[str]:
    rows = manifest.get("rows")
    if isinstance(rows, list):
        out: list[str] = []
        for row in rows:
            if not isinstance(row, Mapping):
                continue
            name = row.get("animation") or row.get("name") or row.get("id")
            if name:
                out.append(str(name))
        return out
    anims = manifest.get("animations")
    if isinstance(anims, Mapping):
        return [str(name) for name in anims.keys()]
    return []


def _first_present(rows: Iterable[str], candidates: Sequence[str]) -> str | None:
    row_set = set(rows)
    for cand in candidates:
        if cand in row_set:
            return cand
    return None


def _character_id_for(stem: str, target: str | None, explicit: str | None = None) -> str:
    if explicit:
        return explicit
    if stem in BASE_CHARACTER_IDS:
        return BASE_CHARACTER_IDS[stem]
    if stem.startswith("npc_"):
        return stem
    # Base adapter config sheets keep their short IDs. Named/configured
    # characters become catalog-style NPC IDs by default.
    if target and stem == target and target in {"robot", "goblin", "sandbag", "ninja", "boss"}:
        return stem
    return f"npc_{stem}"


def _derive_body_plan(stem: str, target: str | None, tags: Sequence[str], rows: Sequence[str]) -> str | None:
    hay = " ".join([stem, target or "", *tags]).lower()
    if "boss" in hay or stem in {"boss", "gnu_ton_boss", "mockingbird_boss"}:
        return "BossMultipart"
    if "house" in hay or "portrait" in hay or "prop" in hay or "board" in hay:
        return "PropActor"
    if "slug" in hay:
        return "Crawler"
    if "shark" in hay or "flying" in hay or "fly" in rows or "hover" in rows:
        return "Flyer"
    if target in {"robot", "goblin", "toon", "ninja", "sandbag", "trent_elder"}:
        return "HumanoidBiped"
    if any(row in rows for row in ("walk", "run", "slash", "talk", "interact")):
        return "HumanoidBiped"
    return None


def _derive_body_kind(body_plan: str | None, stem: str, tags: Sequence[str]) -> str | None:
    hay = " ".join([stem, *(tags or [])]).lower()
    if body_plan == "BossMultipart":
        return "Wide"
    if "heavy" in hay or "brute" in hay or "mauler" in hay or "trex" in hay:
        return "Wide"
    if body_plan == "Crawler":
        return "LowProfile"
    if body_plan == "PropActor":
        return "PropLike"
    if body_plan:
        return "Standard"
    return None


def _derive_locomotion(rows: Sequence[str], body_plan: str | None, target: str | None) -> str | None:
    row_set = set(rows)
    if body_plan == "Flyer" or {"fly", "hover", "float_glide"} & row_set:
        return "Fly"
    if body_plan == "Crawler":
        return "Slither"
    if target == "boss" or body_plan == "BossMultipart":
        return "BossKinematic"
    if {"walk", "run", "side_walk", "shamble", "stable"} & row_set:
        return "Walk"
    return None


def _derive_facing_policy(stem: str, target: str | None, tags: Sequence[str]) -> str | None:
    """Best-effort visual angle hint for renderer-emitted actor sidecars."""
    hay = " ".join([stem, target or "", *tags]).lower()
    if target == "ninja" or "ninja" in hay:
        return "three_quarter_front_right"
    if target in {"robot", "goblin", "toon"}:
        return "side_right_three_quarter"
    if any(word in hay for word in ("pirate", "viking", "statesman", "hermit", "galwah")):
        return "side_right_three_quarter"
    return None


def _derive_animation_bindings(rows: Sequence[str]) -> Dict[str, RonStruct]:
    bindings: Dict[str, RonStruct] = {}
    default = _first_present(rows, IDLE_CANDIDATES) or (rows[0] if rows else None)
    if default:
        bindings["default"] = struct(animation=default, events=[])
    walk = _first_present(rows, WALK_CANDIDATES)
    if walk:
        bindings["locomotion.walk"] = struct(animation=walk, events=[])
    if "run" in rows:
        bindings["locomotion.run"] = struct(animation="run", events=[])
    if "hover" in rows:
        bindings["locomotion.hover"] = struct(animation="hover", events=[])
    if "fly" in rows:
        bindings["locomotion.fly"] = struct(animation="fly", events=[])
    melee = _first_present(rows, MELEE_CANDIDATES)
    if melee:
        bindings["action.melee.primary"] = struct(
            animation=melee,
            events=[
                struct(t=0.35, event="hitbox_active_start", source="renderer_default"),
                struct(t=0.55, event="hitbox_active_end", source="renderer_default"),
            ],
        )
    ranged = _first_present(rows, RANGED_CANDIDATES)
    if ranged:
        bindings["action.ranged.primary"] = struct(
            animation=ranged,
            events=[struct(t=0.5, event="projectile_release", source="renderer_default")],
        )
    if "talk" in rows:
        bindings["interaction.talk"] = struct(animation="talk", events=[])
    if "interact" in rows:
        bindings["interaction.use"] = struct(animation="interact", events=[])
    hit = _first_present(rows, HIT_CANDIDATES)
    if hit:
        bindings["damage.hit"] = struct(animation=hit, events=[])
    death = _first_present(rows, DEATH_CANDIDATES)
    if death:
        bindings["lifecycle.death"] = struct(animation=death, events=[])
    return bindings


def _point(x: float, y: float) -> RonStruct:
    return struct(x=float(x), y=float(y))


def _socket(*, source: str, x: float, y: float, animation: str | None = None, frame: int | None = None) -> RonStruct:
    return struct(
        source=source,
        animation=animation,
        frame=frame,
        point=_point(x, y),
    )


def _bbox_from_manifest(manifest: Mapping[str, Any]) -> tuple[float, float, float, float] | None:
    bm = manifest.get("body_metrics")
    if not isinstance(bm, Mapping):
        return None
    bbox = bm.get("body_pixel_bbox")
    if not isinstance(bbox, Mapping) or not all(k in bbox for k in ("x", "y", "w", "h")):
        return None
    return (float(bbox["x"]), float(bbox["y"]), float(bbox["w"]), float(bbox["h"]))


def _normalize_socket_value(value: Any) -> RonStruct | Any:
    if isinstance(value, RonStruct):
        return value
    if not isinstance(value, Mapping):
        return value
    # Common YAML shorthand: {point: {x: 22, y: 26}}. Keep extra fields if
    # present, but normalize nested points to structs instead of maps.
    data = dict(value)
    point = data.get("point")
    if isinstance(point, Mapping) and "x" in point and "y" in point:
        data["point"] = _point(float(point["x"]), float(point["y"]))
    return _mapping_to_struct(data)


def _iter_frame_anchors(manifest: Mapping[str, Any]) -> Iterable[tuple[str | None, int | None, str, Mapping[str, Any]]]:
    """Yield anchors exposed by sheet rect metadata.

    `tackon_sheet.build_sheet` already carries per-frame anchor maps through to
    the sheet manifest for weapon/procedural targets.  Promote the first anchor
    of each name into the actor sidecar so the engine has a stable socket bag
    without needing generator-specific parsing.
    """
    rows = manifest.get("rows")
    if not isinstance(rows, list):
        return
    for row in rows:
        if not isinstance(row, Mapping):
            continue
        animation = row.get("animation") or row.get("name") or row.get("id")
        rects = row.get("rects")
        if not isinstance(rects, list):
            continue
        for frame_index, rect in enumerate(rects):
            if not isinstance(rect, Mapping):
                continue
            anchors = rect.get("anchors")
            if not isinstance(anchors, Mapping):
                continue
            for name, pos in anchors.items():
                if isinstance(pos, Mapping) and "x" in pos and "y" in pos:
                    yield (str(animation) if animation else None, frame_index, str(name), pos)


def _derive_sockets(
    manifest: Mapping[str, Any],
    *,
    stem: str,
    target: str | None,
    body_plan: str | None,
    tags: Sequence[str],
    rows: Sequence[str],
    held_item: str | None = None,
    action_preset: str | None = None,
) -> Dict[str, RonStruct]:
    sockets: Dict[str, RonStruct] = {}
    bm = manifest.get("body_metrics")
    if isinstance(bm, Mapping):
        feet = bm.get("feet_pixel")
        if isinstance(feet, Mapping) and "x" in feet and "y" in feet:
            sockets["feet"] = _socket(
                source="body_metrics.feet_pixel",
                x=float(feet["x"]),
                y=float(feet["y"]),
            )
    bbox = _bbox_from_manifest(manifest)
    if bbox is not None:
        x, y, w, h = bbox
        cx = x + w * 0.5
        top = y
        bottom = y + h
        sockets.setdefault("center", _socket(source="body_metrics.body_pixel_bbox", x=cx, y=y + h * 0.5))
        sockets.setdefault("root", _socket(source="body_metrics.body_pixel_bbox", x=cx, y=bottom))
        sockets.setdefault("head", _socket(source="body_metrics.body_pixel_bbox", x=cx, y=top))
        sockets.setdefault("chest", _socket(source="heuristic.body_bbox", x=cx, y=y + h * 0.38))

        hay = " ".join([stem, target or "", held_item or "", action_preset or "", *tags]).lower()
        row_set = set(rows)
        traits = set(str(t).lower() for t in tags)
        no_hands = bool({"no_hands", "prop", "portrait"} & traits or body_plan in {"Crawler", "Flyer", "PropActor"})
        if body_plan == "HumanoidBiped" and not no_hands:
            sockets.setdefault("hand_l", _socket(source="heuristic.body_bbox", x=x + w * 0.28, y=y + h * 0.48))
            sockets.setdefault("hand_r", _socket(source="heuristic.body_bbox", x=x + w * 0.72, y=y + h * 0.48))
        if any(word in hay for word in ("zombie", "ghoul", "slug", "shark", "bear", "raptor", "trex", "bite")):
            sockets.setdefault("mouth", _socket(source="heuristic.body_bbox", x=x + w * 0.68, y=y + h * 0.30))
        if held_item in {"bow", "staff", "gun"} or {"shoot", "aim", "cast", "spike_halo"} & row_set:
            fallback = "hand_r" if "hand_r" in sockets else "center"
            base = sockets.get(fallback)
            if isinstance(base, RonStruct):
                p = base.fields.get("point")
                if isinstance(p, RonStruct):
                    sockets.setdefault("muzzle", _socket(source=f"heuristic.{fallback}", x=float(p.fields["x"]), y=float(p.fields["y"])))
                    sockets.setdefault("projectile_origin", sockets["muzzle"])
        if held_item or ({"slash", "attack_side", "bite", "floor_slam", "side_sweep", "stomp"} & row_set):
            if "hand_r" in sockets:
                hand = sockets["hand_r"].fields.get("point")
                if isinstance(hand, RonStruct):
                    sockets.setdefault("weapon_grip", sockets["hand_r"])
                    sockets.setdefault("weapon_tip", _socket(source="heuristic.hand_r", x=float(hand.fields["x"]) + w * 0.35, y=float(hand.fields["y"])))
            elif "mouth" in sockets:
                sockets.setdefault("weapon_tip", sockets["mouth"])

    for animation, frame_index, name, pos in _iter_frame_anchors(manifest):
        if name not in sockets:
            sockets[name] = _socket(
                source="frame_rect.anchors",
                animation=animation,
                frame=frame_index,
                x=float(pos["x"]),
                y=float(pos["y"]),
            )
    return sockets


def _derive_presets(target: str | None, archetype: str | None, role: str | None, tags: Sequence[str], held_item: str | None) -> tuple[str | None, str | None]:
    words = {str(x).lower() for x in [target, archetype, role, held_item, *tags] if x}
    if "training_dummy" in words or target == "sandbag":
        return "stand_still", "sandbag_punch"
    if "boss" in words:
        return "stand_still", "peaceful"
    if "enemy" in words or target in {"goblin", "ninja"}:
        if held_item in {"bow", "staff"} or "ranger" in words or "shaman" in words:
            return "skirmisher_ranger", "ranger_arrow"
        if "brute" in words or "hammer" in words or "guardian" in words or "heavy" in words:
            return "melee_brute_brute", "brute_lunge"
        return "melee_brute_striker", "striker_swipe"
    if target == "robot" and ("runner" in words or "guardian" in words):
        return "melee_brute_striker", "striker_swipe"
    return "patrol_peaceful", "peaceful"


def _traversal_defaults(rows: Sequence[str], body_plan: str | None) -> Dict[str, Any]:
    row_set = set(rows)
    walk = bool({"walk", "run", "side_walk", "shamble", "stable"} & row_set or body_plan in {"HumanoidBiped", "Crawler"})
    fly = bool(body_plan == "Flyer" or {"fly", "hover", "float_glide"} & row_set)
    return {
        "walk": walk if walk else None,
        "jump": {"height_px": None, "distance_px": None, "source": "animation_rows"} if {"jump", "wall_jump"} & row_set else None,
        "climb": True if {"climb", "ledge_climb", "ledge_grab", "wall_grab"} & row_set else None,
        "fly": True if fly else None,
        "swim": True if "swim" in row_set else None,
        "crawl": None,
        "use_lifts": None,
        "door_access": [],
    }


def _traversal(rows: Sequence[str], body_plan: str | None) -> RonStruct:
    return _capability_override_struct(_traversal_defaults(rows, body_plan))


def _interaction_defaults(rows: Sequence[str], stem: str, tags: Sequence[str]) -> Dict[str, Any]:
    hay = " ".join([stem, *tags]).lower()
    return {
        "talk": True if "talk" in rows else None,
        "trade": True if "merchant" in hay or "shop" in hay else None,
        "carry": None,
        "open_doors": [],
    }


def _interactions(rows: Sequence[str], stem: str, tags: Sequence[str]) -> RonStruct:
    return _capability_override_struct(_interaction_defaults(rows, stem, tags))


def _derive_collision(body_override: Mapping[str, Any], manifest: Mapping[str, Any], body_plan: str | None) -> Any:
    explicit = body_override.get("collision")
    if isinstance(explicit, Mapping):
        return some(_mapping_to_struct(explicit))
    if explicit is not None:
        return explicit
    bbox = _bbox_from_manifest(manifest)
    if bbox is None or body_plan == "BossMultipart":
        return None
    _, _, w, h = bbox
    return some(struct(
        w_px=w,
        h_px=h,
        source="sheet.body_metrics.body_pixel_bbox",
        confidence="derived",
    ))


def _derive_hurtbox(body_override: Mapping[str, Any], collision: Any, body_plan: str | None) -> Any:
    explicit = body_override.get("hurtbox")
    if isinstance(explicit, Mapping):
        return some(_mapping_to_struct(explicit))
    if explicit is not None:
        return explicit
    if body_plan in {"PropActor", "BossMultipart"}:
        return None
    if isinstance(collision, RonSome) and isinstance(collision.value, RonStruct):
        fields = dict(collision.value.fields)
        fields.setdefault("source", "derived_from_collision")
        fields.setdefault("confidence", "fallback")
        return some(struct(**fields))
    return None


def _derive_mass_class(body_override: Mapping[str, Any], body_plan: str | None, body_kind: str | None, tags: Sequence[str]) -> str | None:
    explicit = body_override.get("mass_class")
    if explicit:
        return str(explicit)
    hay = " ".join([body_plan or "", body_kind or "", *tags]).lower()
    if "boss" in hay or body_plan == "BossMultipart":
        return "Boss"
    if "heavy" in hay or "wide" in hay or "brute" in hay:
        return "Heavy"
    if body_plan == "PropActor":
        return "Static"
    if body_plan in {"Crawler", "Flyer"}:
        return "Light"
    if body_plan:
        return "Medium"
    return None


def _binding_has(bindings: Mapping[str, Any], prefix: str) -> bool:
    return any(str(key).startswith(prefix) for key in bindings)


def _has_any(mapping: Mapping[str, Any], names: Sequence[str]) -> bool:
    return any(name in mapping for name in names)


def _missing_information(
    *,
    collision: Any,
    hurtbox: Any,
    sockets: Mapping[str, Any],
    capabilities: RonStruct,
    animation_bindings: Mapping[str, Any],
    action_preset: str | None,
    body_plan: str | None,
    authoring_missing: Sequence[Any],
) -> list[str]:
    missing: list[str] = []
    if collision is None:
        missing.append("collision: not authored or derivable; engine should fall back to LDtk AABB")
    if hurtbox is None and body_plan not in {"PropActor", "BossMultipart"}:
        missing.append("hurtbox: not authored; fallback to collision/body metrics")

    wants_melee = _binding_has(animation_bindings, "action.melee") or bool(action_preset and action_preset not in {"peaceful", "ranger_arrow"})
    if wants_melee and not _has_any(sockets, ("weapon_tip", "hand_r", "mouth", "center")):
        missing.append("melee origin socket: no weapon_tip/hand_r/mouth/center socket available; action must use a geometry fallback")

    wants_ranged = _binding_has(animation_bindings, "action.ranged") or bool(action_preset and action_preset in {"ranger_arrow", "boss_bolt"})
    if wants_ranged and not _has_any(sockets, ("muzzle", "projectile_origin", "hand_r", "center")):
        missing.append("ranged origin socket: no muzzle/projectile_origin/hand_r/center socket available; action must use a body fallback")

    trav = capabilities.fields.get("traversal") if isinstance(capabilities, RonStruct) else None
    if isinstance(trav, RonSome) and isinstance(trav.value, RonStruct):
        jump = trav.value.fields.get("jump")
        if isinstance(jump, RonSome) and isinstance(jump.value, RonStruct):
            if jump.value.fields.get("height_px") is None or jump.value.fields.get("distance_px") is None:
                missing.append("jump numbers: traversal jump is present but height_px/distance_px are not measured")

    missing.extend(str(x) for x in (authoring_missing or []))
    # Preserve order but collapse duplicates from profile + explicit authoring.
    return list(dict.fromkeys(missing))


def _profile_defaults(stem: str, target: str | None, job_data: Mapping[str, Any], tags: Sequence[str]) -> Dict[str, Any]:
    """Return sparse family defaults used before hand-authored overrides.

    This is intentionally conservative: it enriches recurring character families
    with known capabilities/body traits while still letting YAML/tack-on metadata
    override any field.  The goal is fewer anonymous gaps in `_actor.ron`, not a
    rigid taxonomy.
    """
    archetype = str(job_data.get("archetype") or "").lower()
    role = str(job_data.get("role") or "").lower()
    held = str(job_data.get("held_item") or "").lower()
    hay = " ".join([stem, target or "", archetype, role, held, *tags]).lower()
    out: Dict[str, Any] = {"body": {}, "capabilities": {"traversal": {}, "interactions": {}}, "sockets": {}, "animation_bindings": {}}

    def traits(*items: str) -> None:
        cur = list(out["body"].get("traits") or [])
        for item in items:
            if item and item not in cur:
                cur.append(item)
        out["body"]["traits"] = cur

    if target in {"robot", "goblin", "toon", "ninja"} or any(w in hay for w in ("viking", "pirate", "statesman", "creator", "lord", "hermit", "galwah")):
        out["body"].setdefault("body_plan", "HumanoidBiped")
        out["body"].setdefault("body_kind", "Standard")
        out["capabilities"]["traversal"].setdefault("walk", True)
    if target == "robot" or "robot" in hay:
        traits("robot")
        out["capabilities"]["traversal"].setdefault("jump", {"height_px": None, "distance_px": None, "source": "robot_profile"})
        out["capabilities"]["traversal"].setdefault("use_lifts", True)
    if target == "goblin" or "goblin" in hay:
        traits("enemy", "goblin")
        out["capabilities"]["traversal"].setdefault("jump", None)
    if "merchant" in hay or "shop" in hay:
        traits("merchant")
        out["capabilities"]["interactions"].setdefault("talk", True)
        out["capabilities"]["interactions"].setdefault("trade", True)
    if target == "sandbag" or "sandbag" in hay:
        out["body"].update({"body_plan": "TrainingDummy", "body_kind": "Standard", "mass_class": "Static"})
        traits("training")
        out["capabilities"]["traversal"].update({"walk": False, "jump": None, "climb": None, "fly": None})
    if "boss" in hay or target == "boss":
        out["body"].update({"body_plan": "BossMultipart", "body_kind": "Wide", "mass_class": "Boss"})
        traits("boss")
        out["capabilities"]["traversal"].setdefault("walk", False)
    if any(w in hay for w in ("shark", "flying", "mockingbird")):
        out["body"].setdefault("body_plan", "Flyer")
        traits("flying")
        out["capabilities"]["traversal"].update({"walk": False, "fly": True})
    if any(w in hay for w in ("slug", "crawler")):
        out["body"].setdefault("body_plan", "Crawler")
        traits("crawler", "no_hands")
        out["capabilities"]["traversal"].setdefault("walk", True)
    if any(w in hay for w in ("portrait", "house", "prop")):
        out["body"].update({"body_plan": "PropActor", "body_kind": "PropLike", "mass_class": "Static"})
        traits("prop", "no_hands")
        out["capabilities"]["traversal"].update({"walk": False, "jump": None, "climb": None, "fly": None})
    if role == "enemy" or "enemy" in tags:
        traits("enemy")
    # Action bindings are only emitted when the sheet actually contains a row
    # for them; profile data can influence action presets/capabilities but should
    # not invent an animation row that the renderer did not draw.
    return out

def _normalize_authoring_block(block: Mapping[str, Any] | None) -> Dict[str, Any]:
    return dict(block or {})


def _mapping_to_struct_map(values: Mapping[str, Any]) -> RonMap:
    converted: Dict[str, Any] = {}
    for key, value in values.items():
        if isinstance(value, RonStruct):
            converted[key] = value
        elif isinstance(value, Mapping):
            converted[key] = _mapping_to_struct(value)
        else:
            converted[key] = value
    return ron_map(converted)



def _capability_override_struct(value: Mapping[str, Any]) -> RonStruct:
    fields = {}
    for key, item in value.items():
        if isinstance(item, Mapping):
            fields[key] = some(_capability_override_struct(item))
        elif item is None:
            fields[key] = None
        elif isinstance(item, list):
            fields[key] = item
        else:
            fields[key] = some(item)
    return struct(**fields)

def _mapping_to_struct(value: Mapping[str, Any]) -> RonStruct:
    fields = {}
    for key, item in value.items():
        if isinstance(item, Mapping):
            fields[key] = _mapping_to_struct(item)
        elif isinstance(item, list):
            fields[key] = [_mapping_to_struct(x) if isinstance(x, Mapping) else x for x in item]
        else:
            fields[key] = item
    return struct(**fields)


def build_actor_contract(
    *,
    stem: str,
    target: str | None,
    image: str,
    sheet_manifest: str,
    manifest: Mapping[str, Any],
    job_data: Mapping[str, Any] | None = None,
    authoring: Mapping[str, Any] | None = None,
) -> RonStruct:
    """Build the sparse actor contract for one rendered sheet.

    ``job_data`` is the flattened CharacterJob-style metadata; ``authoring`` is
    the optional nested override block from YAML or a tack-on target. Both are
    deliberately loose dictionaries so old configs stay compatible.
    """
    job_data = dict(job_data or {})
    # Actor metadata now lives with the character authoring file itself:
    # YAML configs pass their blocks via `CharacterJob`, and Python tack-ons pass
    # local `ACTOR_METADATA` / per-target `actor_metadata`. No central profile
    # table participates here.
    authoring = dict(authoring or {})
    actor_block = _normalize_authoring_block(authoring.get("actor"))
    rows = _rows_from_manifest(manifest)
    explicit_character_id = actor_block.get("character_id") or authoring.get("character_id")
    catalog_profile = _catalog_profile_for(stem, explicit_character_id)
    tags = list(dict.fromkeys([
        *(job_data.get("tags") or []),
        *((catalog_profile.tags if catalog_profile else ()) or ()),
        *(actor_block.get("tags") or authoring.get("tags") or []),
    ]))
    profile = _profile_defaults(stem, target, job_data, tags)
    character_id = explicit_character_id or (catalog_profile.character_id if catalog_profile else None) or _character_id_for(stem, target, None)
    display_name = (
        actor_block.get("display_name")
        or authoring.get("display_name")
        or job_data.get("name")
        or (catalog_profile.display_name if catalog_profile else None)
        or _humanize(stem)
    )

    body_override = _deep_merge(_as_mapping(profile.get("body")), _as_mapping(authoring.get("body")))
    body_plan = actor_block.get("body_plan") or body_override.get("body_plan") or _derive_body_plan(stem, target, tags, rows)
    body_kind = body_override.get("body_kind") or (catalog_profile.body_kind if catalog_profile else None) or _derive_body_kind(body_plan, stem, tags)
    locomotion = body_override.get("locomotion_hint") or _derive_locomotion(rows, body_plan, target)

    brain_preset, action_preset = _derive_presets(
        target,
        job_data.get("archetype"),
        job_data.get("role"),
        tags,
        job_data.get("held_item"),
    )
    if catalog_profile is not None:
        brain_preset = catalog_profile.default_brain or brain_preset
        action_preset = catalog_profile.default_action_set or action_preset
    explicit_brain = _as_mapping(authoring.get("brain")).get("default_preset") or authoring.get("brain")
    explicit_actions = _as_mapping(authoring.get("actions")).get("default_preset") or authoring.get("actions")
    resolved_brain = str(explicit_brain or brain_preset) if (explicit_brain or brain_preset) else None
    resolved_action = str(explicit_actions or action_preset) if (explicit_actions or action_preset) else None

    anim_bindings = _derive_animation_bindings(rows)
    anim_bindings = _deep_merge(anim_bindings, _as_mapping(profile.get("animation_bindings")))
    anim_bindings = _deep_merge(anim_bindings, _as_mapping(authoring.get("animation_bindings")))

    sockets = _derive_sockets(
        manifest,
        stem=stem,
        target=target,
        body_plan=body_plan,
        tags=[*tags, *list(body_override.get("traits") or [])],
        rows=rows,
        held_item=job_data.get("held_item"),
        action_preset=resolved_action,
    )
    sockets = _deep_merge(sockets, _as_mapping(profile.get("sockets")))
    sockets = _deep_merge(sockets, _as_mapping(authoring.get("sockets")))
    sockets = {key: _normalize_socket_value(value) for key, value in sockets.items()}

    capabilities_profile = _as_mapping(profile.get("capabilities"))
    capabilities_override = _as_mapping(authoring.get("capabilities"))
    traversal_plain = _deep_merge(
        _deep_merge(
            _traversal_defaults(rows, body_plan),
            _as_mapping(capabilities_profile.get("traversal")),
        ),
        _as_mapping(capabilities_override.get("traversal")),
    )
    interaction_plain = _deep_merge(
        _deep_merge(
            _interaction_defaults(rows, stem, tags),
            _as_mapping(capabilities_profile.get("interactions")),
        ),
        _as_mapping(capabilities_override.get("interactions")),
    )
    capabilities = struct(
        traversal=some(_capability_override_struct(traversal_plain)),
        interactions=some(_capability_override_struct(interaction_plain)),
    )

    visual_override = _as_mapping(authoring.get("visual"))
    facing_policy = visual_override.get("facing_policy") or _derive_facing_policy(stem, target, tags)
    visual = struct(
        sheet_id=visual_override.get("sheet_id", stem),
        spritesheet=visual_override.get("spritesheet", image),
        sheet_manifest=visual_override.get("sheet_manifest", sheet_manifest),
        default_pose=some(visual_override.get("default_pose") or _first_present(rows, IDLE_CANDIDATES) or (rows[0] if rows else "")),
        facing_policy=some(facing_policy) if facing_policy else None,
        coordinate_system=some(struct(origin="top_left", x_axis="right", y_axis="down", units="pixels")),
        up_axis=some("negative_y"),
        scale=visual_override.get("scale", None),
    )

    collision = _derive_collision(body_override, manifest, body_plan)
    hurtbox = _derive_hurtbox(body_override, collision, body_plan)
    mass_class = _derive_mass_class(body_override, body_plan, body_kind, [*tags, *list(body_override.get("traits") or [])])
    body = struct(
        body_kind=some(body_kind) if body_kind else None,
        body_plan=some(body_plan) if body_plan else None,
        collision=collision,
        hurtbox=hurtbox,
        mass_class=some(mass_class) if mass_class else None,
        locomotion_hint=some(locomotion) if locomotion else None,
        body_metrics_source=some("sheet.body_metrics") if manifest.get("body_metrics") else None,
        traits=list(dict.fromkeys(body_override.get("traits") or [])),
    )
    missing = _missing_information(
        collision=collision,
        hurtbox=hurtbox,
        sockets=sockets,
        capabilities=capabilities,
        animation_bindings=anim_bindings,
        action_preset=resolved_action,
        body_plan=body_plan,
        authoring_missing=authoring.get("missing_information") or [],
    )

    return struct(
        schema_version=1,
        character_id=character_id,
        actor_id=actor_block.get("actor_id", None),
        display_name=some(str(display_name)) if display_name else None,
        provenance=some(struct(
            surface=job_data.get("surface", "adapter" if job_data else "tackon"),
            renderer_target=target or stem,
            output_stem=stem,
            seed=job_data.get("seed", None),
            archetype=job_data.get("archetype", None),
            variant=job_data.get("variant", None),
            held_item=job_data.get("held_item", None),
            source_config=job_data.get("source_config", None),
        )),
        visual=some(visual),
        body=some(body),
        capabilities=some(capabilities),
        brain=some(struct(default_preset=some(resolved_brain) if resolved_brain else None)),
        actions=some(struct(default_preset=some(resolved_action) if resolved_action else None)),
        animation_bindings=_mapping_to_struct_map(anim_bindings),
        sockets=_mapping_to_struct_map(sockets),
        tags=tags,
        missing_information=missing,
    )

def write_actor_contract(path: str | Path, contract: RonStruct) -> Path:
    out = Path(path)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(to_ron(contract), encoding="utf8")
    return out


def write_actor_contract_for_adapter(
    *,
    image_out: str | Path,
    sheet_ron_out: str | Path,
    manifest: Mapping[str, Any],
    job: Any,
    source_config: str | Path | None = None,
) -> Path:
    image_out = Path(image_out)
    stem = _spritesheet_stem(image_out)
    job_data = {
        "surface": "adapter",
        "source_config": str(source_config) if source_config is not None else None,
        "name": getattr(job, "name", None),
        "seed": getattr(job, "seed", None),
        "archetype": getattr(job, "archetype", None),
        "variant": getattr(job, "variant", None),
        "held_item": getattr(job, "held_item", None),
        "role": getattr(job, "role", None),
        "tags": list(getattr(job, "tags", []) or []),
    }
    authoring = {
        "actor": getattr(job, "actor", {}) or {},
        "visual": getattr(job, "visual", {}) or {},
        "body": getattr(job, "body", {}) or {},
        "capabilities": getattr(job, "capabilities", {}) or {},
        "brain": getattr(job, "brain", {}) or {},
        "actions": getattr(job, "actions", {}) or {},
        "animation_bindings": getattr(job, "animation_bindings", {}) or {},
        "sockets": getattr(job, "sockets", {}) or {},
        "missing_information": getattr(job, "missing_information", []) or [],
    }
    contract = build_actor_contract(
        stem=stem,
        target=str(getattr(job, "target", stem)),
        image=image_out.name,
        sheet_manifest=Path(sheet_ron_out).name,
        manifest=manifest,
        job_data=job_data,
        authoring=authoring,
    )
    return write_actor_contract(actor_sidecar_path_for_image(image_out), contract)


def write_actor_contract_for_tackon(
    *,
    target: str,
    image_out: str | Path,
    sheet_ron_out: str | Path,
    manifest: Mapping[str, Any],
    actor_metadata: Mapping[str, Any] | None = None,
) -> Path:
    image_out = Path(image_out)
    stem = _spritesheet_stem(image_out)
    contract = build_actor_contract(
        stem=stem,
        target=target,
        image=image_out.name,
        sheet_manifest=Path(sheet_ron_out).name,
        manifest=manifest,
        job_data={"surface": "tackon", "tags": []},
        authoring=actor_metadata or {},
    )
    return write_actor_contract(actor_sidecar_path_for_image(image_out), contract)
