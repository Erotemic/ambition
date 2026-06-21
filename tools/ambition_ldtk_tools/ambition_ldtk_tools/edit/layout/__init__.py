"""Auto-layout implementation modules for LDtk Free-layout worlds.

The public CLI remains ``ambition_ldtk_tools.edit.world_layout``.  This package
holds separable layout concerns so strategies, graph construction, SVG reports,
and writeback can be migrated out of the historical monolith incrementally.
"""

from .model import GroupInfo, LayoutEdge, LayoutResult, LevelInfo, Point, Rect, ZoneRef

__all__ = [
    "GroupInfo",
    "LayoutEdge",
    "LayoutResult",
    "LevelInfo",
    "Point",
    "Rect",
    "ZoneRef",
]
