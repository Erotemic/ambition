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

FULL_PLAYER_ANIMATION_ORDER: List[str] = (
    CORE_CHARACTER_ANIMATION_ORDER
    + EXTENDED_PLAYER_ANIMATION_ORDER
    + ADVANCED_PLAYER_ANIMATION_ORDER
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


def ordered_subset(source: Mapping[str, AnimationInfo], order: Iterable[str]) -> AnimationMap:
    """Return ``source`` in the requested order, skipping missing names."""

    return {name: dict(source[name]) for name in order if name in source}


def merge_animation_maps(*maps: Mapping[str, AnimationInfo]) -> AnimationMap:
    merged: AnimationMap = {}
    for mapping in maps:
        for name, info in mapping.items():
            merged[name] = dict(info)
    return merged
