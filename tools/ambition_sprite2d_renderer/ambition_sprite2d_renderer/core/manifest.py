"""The output contract: the sprite-sheet manifest the game reads.

These dataclasses mirror, field-for-field, the RON schema the game deserializes
in ``crates/ambition_sprite_sheet/src/lib.rs`` (``SheetRecord`` & friends). They
are the *target shape* for the single manifest emitter that will replace the two
divergent RON writers (``sheet.py`` and ``tackon_sheet.py``).

Write format is **RON** (the game is already RON-native), produced with the
standard library only — **no YAML in the write path** (Jon, 2026-06-21). The
``to_ron`` emitter itself is intentionally NOT added here yet: it lands when the
two existing emitters are consolidated onto the core, at which point it is
guarded by a Rust-side parse test (Python RON writers are looser than Rust's
``ron`` — see the parser-drift lesson). Defining the schema now means that
consolidation targets one fixed shape instead of reverse-engineering two.

A manifest file is always a *list* of ``SheetRecord`` (length 1 for most sheets;
one record per sub-target for packed shared sheets like the lab props).

Pillow + stdlib only.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List, Optional


@dataclass
class PixelRect:
    x: int
    y: int
    w: int
    h: int


@dataclass
class NamedPixelRect:
    name: str
    x: int
    y: int
    w: int
    h: int


@dataclass
class Point:
    x: float
    y: float


@dataclass
class AnimationBoxFrame:
    """Per-frame gameplay box; lets a hit/hurt box move with the drawn pose."""

    parts: List[NamedPixelRect] = field(default_factory=list)
    bbox: Optional[PixelRect] = None


@dataclass
class AnimationBox:
    """One animation's hit-or-hurt box: multi-rect parts, a bbox fallback,
    and optional per-frame samples (the key to melee boxes that *agree* with
    the animation — the box is sampled from the same posed frames)."""

    parts: List[NamedPixelRect] = field(default_factory=list)
    bbox: Optional[PixelRect] = None
    frames: List[AnimationBoxFrame] = field(default_factory=list)


@dataclass
class AnimationMetrics:
    frame_duration_secs: Optional[float] = None
    hurtbox: Optional[AnimationBox] = None
    hitbox: Optional[AnimationBox] = None


@dataclass
class BodyMetrics:
    """Geometry the renderer *measures* from the rendered art."""

    body_pixel_bbox: Optional[PixelRect] = None
    body_pixel_parts: List[NamedPixelRect] = field(default_factory=list)
    animations: Dict[str, AnimationMetrics] = field(default_factory=dict)
    feet_pixel: Optional[Point] = None
    feet_anchor_norm: Optional[Point] = None


@dataclass
class SheetTuningSpec:
    """Gameplay tuning embedded in the manifest. ``collision_scale`` is
    currently a Rust constant; the renderer can derive it from the measured
    body fraction (the ``TODO(gen2d-collision-aware)``) and emit it here so the
    Rust constant retires. Deferred work, but the field is part of the shape."""

    collision_scale: float
    frame_sample_inset: int = 1


@dataclass
class FrameRect:
    x: int
    y: int
    w: int
    h: int
    # Per-frame named anchor points in atlas pixels (hand/muzzle/…).
    anchors: Dict[str, Point] = field(default_factory=dict)


@dataclass
class SheetRow:
    animation: str
    row_index: int
    frame_count: int
    duration_ms: int
    duration_secs: float
    rects: List[FrameRect] = field(default_factory=list)


@dataclass
class SheetRecord:
    target: str
    image: str
    label_width: int
    frame_width: int
    frame_height: int
    rows: List[SheetRow]
    y_offset: int = 0
    body_metrics: Optional[BodyMetrics] = None
    tuning: Optional[SheetTuningSpec] = None
