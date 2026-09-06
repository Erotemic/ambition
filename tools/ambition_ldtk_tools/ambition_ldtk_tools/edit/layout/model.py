"""Data model for LDtk world auto-layout strategies."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class Point:
    x: int
    y: int

    def __add__(self, other: "Point") -> "Point":
        return Point(self.x + other.x, self.y + other.y)

    def __sub__(self, other: "Point") -> "Point":
        return Point(self.x - other.x, self.y - other.y)


@dataclass(frozen=True)
class Rect:
    x: int
    y: int
    w: int
    h: int

    @property
    def x2(self) -> int:
        return self.x + self.w

    @property
    def y2(self) -> int:
        return self.y + self.h

    @property
    def center(self) -> Point:
        return Point(self.x + self.w // 2, self.y + self.h // 2)

    def translated(self, p: Point) -> "Rect":
        return Rect(self.x + p.x, self.y + p.y, self.w, self.h)

    def intersects(self, other: "Rect", *, gap: int = 0) -> bool:
        return (
            self.x < other.x2 + gap
            and self.x2 + gap > other.x
            and self.y < other.y2 + gap
            and self.y2 + gap > other.y
        )


@dataclass
class LevelInfo:
    identifier: str
    active_area: str
    level: dict
    rect: Rect


@dataclass
class GroupInfo:
    id: str
    levels: list[LevelInfo] = field(default_factory=list)
    anchor: Point = Point(0, 0)
    rect: Rect = Rect(0, 0, 0, 0)


@dataclass(frozen=True)
class ZoneRef:
    group_id: str
    level_id: str
    zone_id: str
    center_rel_group: Point
    center_rel_level: Point
    level_rect_rel_group: Rect
    activation: str


@dataclass(frozen=True)
class LayoutEdge:
    source: ZoneRef
    target: ZoneRef | None
    target_group_id: str
    target_room_raw: str
    target_zone_raw: str
    direction: str
    weight: float = 1.0


@dataclass
class LayoutResult:
    placements: dict[str, Point]
    groups: dict[str, GroupInfo]
    edges: list[LayoutEdge]
    unresolved_edges: list[LayoutEdge]
    moved_levels: int
    updated_entities: int
    report: str
    locked_groups: set[str] = field(default_factory=set)
    packing_padding: int = 0
    strategy: str = "greedy"
    clusters: list[list[str]] = field(default_factory=list)
