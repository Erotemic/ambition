from __future__ import annotations

"""Shared animation vocabulary for procedural 2D character targets.

The Rust runtime currently consumes a compact core grid, but the renderer can
produce richer review sheets before engine integration exists.  Keeping those
names here lets robot, goblin, sandbag, and future character variants agree on
what an animation row means without every target inventing its own spelling.
"""

from typing import Dict, Iterable, List, Mapping

AnimationInfo = Dict[str, int]
AnimationMap = Dict[str, AnimationInfo]

CORE_CHARACTER_ANIMATION_ORDER: List[str] = [
    "idle",
    "walk",
    "run",
    "jump",
    "fall",
    "slash",
    "hit",
    "death",
    "blink_out",
    "blink_in",
    "dash",
]

# Rows for mechanics that already exist or are on the near-term gameplay path
# but do not yet have first-class Rust animation selection everywhere.
EXTENDED_PLAYER_ANIMATION_ORDER: List[str] = [
    "crouch",
    "wall_slide",
    "wall_jump",
    "ledge_grab",
    "climb",
    "swim",
    "interact",
    "talk",
    "block",
]

# Review-only rows for expressive player variants and future character work.
# These deliberately use action-oriented names that can be shared by NPCs,
# dummies, and the player before the Rust runtime chooses a final row order.
ADVANCED_PLAYER_ANIMATION_ORDER: List[str] = [
    "land",
    "roll",
    "slide",
    "crouch_walk",
    "pickup",
    "throw",
    "aim",
    "shoot",
    "charge",
    "cast",
    "celebrate",
    "sit",
    "sleep",
    "hover",
    "stomp",
]

# Traversal polish + directional sword attacks (Marth/Lucina shaped). These
# rows are still review-only at the Rust level; the renderer produces them so
# combat designers can iterate on the swing shapes before the gameplay layer
# binds individual attacks to inputs.
TRAVERSAL_POLISH_ANIMATION_ORDER: List[str] = [
    "dash_startup",
    "land_hard",
    "land_recovery",
    "wall_grab",
    "ledge_climb",
    "ledge_getup",
    # Smash-Bros style ledge options. `ledge_roll` is the
    # invuln-tumble option; `ledge_getup_attack` is the swing-onto-
    # platform option. Runtime selection lives in `pick_player_anim`
    # (`CharacterAnim::LedgeRoll` / `CharacterAnim::LedgeGetupAttack`).
    "ledge_roll",
    "ledge_getup_attack",
    "float_glide",
]

DIRECTIONAL_ATTACK_ANIMATION_ORDER: List[str] = [
    "attack_side",
    "attack_up",
    "attack_down",
    "air_neutral",
    "air_forward",
    "air_back",
    "air_down",
    "air_up",
]

FULL_PLAYER_ANIMATION_ORDER: List[str] = (
    CORE_CHARACTER_ANIMATION_ORDER
    + EXTENDED_PLAYER_ANIMATION_ORDER
    + ADVANCED_PLAYER_ANIMATION_ORDER
    + TRAVERSAL_POLISH_ANIMATION_ORDER
    + DIRECTIONAL_ATTACK_ANIMATION_ORDER
)

DEFAULT_CORE_TIMINGS: AnimationMap = {
    "idle": {"frames": 8, "duration_ms": 120},
    "walk": {"frames": 8, "duration_ms": 95},
    "run": {"frames": 8, "duration_ms": 75},
    "jump": {"frames": 6, "duration_ms": 95},
    "fall": {"frames": 6, "duration_ms": 95},
    "slash": {"frames": 8, "duration_ms": 75},
    "hit": {"frames": 5, "duration_ms": 90},
    "death": {"frames": 8, "duration_ms": 110},
    "blink_out": {"frames": 6, "duration_ms": 62},
    "blink_in": {"frames": 6, "duration_ms": 62},
    "dash": {"frames": 6, "duration_ms": 65},
}

DEFAULT_EXTENDED_TIMINGS: AnimationMap = {
    "crouch": {"frames": 5, "duration_ms": 95},
    "wall_slide": {"frames": 6, "duration_ms": 95},
    "wall_jump": {"frames": 6, "duration_ms": 85},
    "ledge_grab": {"frames": 6, "duration_ms": 100},
    "climb": {"frames": 8, "duration_ms": 100},
    "swim": {"frames": 8, "duration_ms": 105},
    "interact": {"frames": 6, "duration_ms": 90},
    "talk": {"frames": 8, "duration_ms": 110},
    "block": {"frames": 6, "duration_ms": 85},
}

DEFAULT_ADVANCED_TIMINGS: AnimationMap = {
    "land": {"frames": 6, "duration_ms": 72},
    "roll": {"frames": 8, "duration_ms": 58},
    "slide": {"frames": 6, "duration_ms": 70},
    "crouch_walk": {"frames": 8, "duration_ms": 88},
    "pickup": {"frames": 7, "duration_ms": 82},
    "throw": {"frames": 7, "duration_ms": 72},
    "aim": {"frames": 6, "duration_ms": 100},
    "shoot": {"frames": 6, "duration_ms": 58},
    "charge": {"frames": 8, "duration_ms": 76},
    "cast": {"frames": 8, "duration_ms": 80},
    "celebrate": {"frames": 8, "duration_ms": 92},
    "sit": {"frames": 5, "duration_ms": 120},
    "sleep": {"frames": 8, "duration_ms": 130},
    "hover": {"frames": 8, "duration_ms": 78},
    "stomp": {"frames": 6, "duration_ms": 70},
}

DEFAULT_TRAVERSAL_POLISH_TIMINGS: AnimationMap = {
    "dash_startup": {"frames": 4, "duration_ms": 50},
    "land_hard": {"frames": 8, "duration_ms": 95},
    "land_recovery": {"frames": 6, "duration_ms": 75},
    "wall_grab": {"frames": 6, "duration_ms": 110},
    "ledge_climb": {"frames": 6, "duration_ms": 100},
    # `ledge_getup` previously ran 8 × 75 = 600 ms — 2.5x longer than
    # the engine's `LEDGE_CLIMB_TIME = 0.24 s` transition, so the
    # sprite only played its first three frames before the player
    # snapped onto the platform. Retuned to 6 × 40 = 240 ms so frame
    # playback completes exactly when the engine getup ends.
    "ledge_getup": {"frames": 6, "duration_ms": 40},
    # 8 × 37 = 296 ms ≈ `LEDGE_ROLL_TIME = 0.30 s`.
    "ledge_roll": {"frames": 8, "duration_ms": 37},
    # 8 × 37 = 296 ms ≈ `LEDGE_GETUP_ATTACK_TIME = 0.30 s`.
    # The engine fires `MovementOp::Slash` at the START of the
    # transition; sprite should peak the swing mid-animation (frames
    # 4-5) so visual + hitbox read as one beat.
    "ledge_getup_attack": {"frames": 8, "duration_ms": 37},
    "float_glide": {"frames": 8, "duration_ms": 110},
}

DEFAULT_DIRECTIONAL_ATTACK_TIMINGS: AnimationMap = {
    "attack_side": {"frames": 8, "duration_ms": 65},
    "attack_up": {"frames": 8, "duration_ms": 65},
    "attack_down": {"frames": 8, "duration_ms": 65},
    "air_neutral": {"frames": 8, "duration_ms": 60},
    "air_forward": {"frames": 7, "duration_ms": 62},
    "air_back": {"frames": 7, "duration_ms": 62},
    "air_down": {"frames": 7, "duration_ms": 70},
    "air_up": {"frames": 7, "duration_ms": 62},
}


def ordered_subset(
    source: Mapping[str, AnimationInfo], order: Iterable[str]
) -> AnimationMap:
    """Return ``source`` in the requested order, skipping missing names."""

    return {name: dict(source[name]) for name in order if name in source}


def merge_animation_maps(*maps: Mapping[str, AnimationInfo]) -> AnimationMap:
    merged: AnimationMap = {}
    for mapping in maps:
        for name, info in mapping.items():
            merged[name] = dict(info)
    return merged
