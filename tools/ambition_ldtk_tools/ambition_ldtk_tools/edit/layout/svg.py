"""SVG preview rendering for LDtk world auto-layout."""

from __future__ import annotations

import math
from pathlib import Path

from ambition_ldtk_tools.edit.layout.model import GroupInfo, LayoutResult, LevelInfo, Point, Rect
from ambition_ldtk_tools.edit.layout.graph import level_top_left_relative_to_group

def _svg_escape(value: object) -> str:
    return (
        str(value)
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


def _layout_bounds(result: LayoutResult, *, margin: int = 512) -> Rect:
    rects = [group.rect.translated(result.placements[group_id]) for group_id, group in result.groups.items()]
    if not rects:
        return Rect(-margin, -margin, margin * 2, margin * 2)
    x0 = min(r.x for r in rects) - margin
    y0 = min(r.y for r in rects) - margin
    x1 = max(r.x2 for r in rects) + margin
    y1 = max(r.y2 for r in rects) + margin
    return Rect(x0, y0, x1 - x0, y1 - y0)


def _level_rect_at_placement(level: LevelInfo, group: GroupInfo, placement: Point) -> Rect:
    rel = level_top_left_relative_to_group(level, group)
    return Rect(placement.x + rel.x, placement.y + rel.y, level.rect.w, level.rect.h)


def render_layout_svg(result: LayoutResult, *, max_width: int = 1800) -> str:
    """Render a proposed world auto-layout as a standalone SVG."""

    bounds = _layout_bounds(result)
    scale = min(1.0, max_width / max(bounds.w, 1))
    width = max(1, int(math.ceil(bounds.w * scale)))
    height = max(1, int(math.ceil(bounds.h * scale)))

    def sx(x: int | float) -> float:
        return (float(x) - bounds.x) * scale

    def sy(y: int | float) -> float:
        return (float(y) - bounds.y) * scale

    def sw(w: int | float) -> float:
        return max(1.0, float(w) * scale)

    font = max(9, min(14, int(12 * max(0.75, scale))))
    parts: list[str] = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect x="0" y="0" width="100%" height="100%" fill="#0f172a"/>',
        '<style>text{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}</style>',
    ]

    # Draw resolved links first so rooms sit on top of them.
    for edge in result.edges:
        if edge.target is None or edge.target_group_id not in result.groups:
            continue
        if edge.source.group_id not in result.placements or edge.target_group_id not in result.placements:
            continue
        src = result.placements[edge.source.group_id] + edge.source.center_rel_group
        dst = result.placements[edge.target_group_id] + edge.target.center_rel_group
        color = "#64748b" if edge.source.group_id != edge.target_group_id else "#334155"
        dash = "" if edge.weight >= 1.0 else ' stroke-dasharray="8 6"'
        parts.append(
            f'<line x1="{sx(src.x):.1f}" y1="{sy(src.y):.1f}" '
            f'x2="{sx(dst.x):.1f}" y2="{sy(dst.y):.1f}" '
            f'stroke="{color}" stroke-width="{max(1.0, 2.0 * scale):.1f}" opacity="0.55"{dash}/>'
        )

    for group_id in sorted(result.groups):
        group = result.groups[group_id]
        placement = result.placements[group_id]
        grect = group.rect.translated(placement)
        locked = group_id in result.locked_groups
        fill = "#1e293b" if not locked else "#3b2f1e"
        stroke = "#38bdf8" if not locked else "#f59e0b"
        parts.append(
            f'<rect x="{sx(grect.x):.1f}" y="{sy(grect.y):.1f}" '
            f'width="{sw(grect.w):.1f}" height="{sw(grect.h):.1f}" '
            f'fill="{fill}" stroke="{stroke}" stroke-width="2" opacity="0.94" rx="6"/>'
        )
        parts.append(
            f'<text x="{sx(grect.x) + 6:.1f}" y="{sy(grect.y) + font + 4:.1f}" '
            f'fill="{stroke}" font-size="{font}" paint-order="stroke" stroke="#0f172a" stroke-width="4">'
            f'{_svg_escape(group_id)}{" 🔒" if locked else ""}</text>'
        )
        for level in sorted(group.levels, key=lambda li: li.identifier):
            rect = _level_rect_at_placement(level, group, placement)
            parts.append(
                f'<rect x="{sx(rect.x):.1f}" y="{sy(rect.y):.1f}" '
                f'width="{sw(rect.w):.1f}" height="{sw(rect.h):.1f}" '
                f'fill="#0f172a" stroke="#94a3b8" stroke-width="1" opacity="0.32"/>'
            )
            if rect.w * scale >= 90 and rect.h * scale >= 24:
                parts.append(
                    f'<text x="{sx(rect.x) + 5:.1f}" y="{sy(rect.y) + font * 2 + 8:.1f}" '
                    f'fill="#cbd5e1" font-size="{max(8, font - 1)}">{_svg_escape(level.identifier)}</text>'
                )

    if result.unresolved_edges:
        y = 22
        parts.append(f'<text x="16" y="{y}" fill="#fca5a5" font-size="14">unresolved links: {len(result.unresolved_edges)}</text>')
    parts.append(
        f'<text x="16" y="{height - 16}" fill="#94a3b8" font-size="12">'
        f'auto-layout strategy={_svg_escape(result.strategy)} · padding {result.packing_padding}px</text>'
    )
    parts.append("</svg>\n")
    return "\n".join(parts)


def write_svg_report(path: Path, result: LayoutResult, *, max_width: int = 1800) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(render_layout_svg(result, max_width=max_width))
