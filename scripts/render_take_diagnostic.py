#!/usr/bin/env python3
"""Draw a recorded take as an SVG contact sheet — with NO rasterizer at all.

⭐⭐ TWO KINDS OF PICTURE, AND THEY MUST NEVER BE CONFUSED. An ENGINE RENDER is
what the production Bevy graph drew (`moveset_render`, which needs a WGPU
adapter — Lavapipe counts). A DIAGNOSTIC RENDER is derived from a recorded take
and simulation observations, and needs nothing. This is the second, and every
sheet it writes says so on its face.

⛔⛔ IT EXISTED ONLY INSIDE A BROWSER CANVAS. The moveset inspector already draws
body boxes, combat volumes and projectiles in JavaScript, so the one machine that
could produce these pictures was one with a browser attached to a running server.
An agent on a restricted box could observe everything and show nothing. A 2026-08-29
review named that: make it an exportable developer ARTIFACT.

⛔ SVG, NOT PNG, AND THAT IS THE POINT. Geometry is what a take records; rendering
it to pixels would need the sprite sheets decoded and a compositor, which is the
work this tool exists to avoid. An SVG is diffable, scalable, and readable in the
same terminal that produced it.

    python3 scripts/render_take_diagnostic.py --takes <takes.json> --out <dir>
    python3 scripts/render_take_diagnostic.py --takes … --character npc_pirate_admiral
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

# How many frames of a take one sheet shows. A take is 150 frames; a strip of
# twelve is a move's shape without being a wall.
DEFAULT_COLUMNS = 12

# ⛔ THE LABEL IS NOT DECORATION. The inspector is careful never to pass a derived
# picture off as an engine render, and an exported file leaves the context that
# made that obvious — so the file carries it.
WATERMARK = "DIAGNOSTIC RENDER — derived from a recorded take, not an engine render"


def sample(frames: list[dict], columns: int) -> list[tuple[int, dict]]:
    """Evenly spaced frames, always including the first.

    ⛔ EVENLY SPACED, NOT THE FIRST N. The first twelve ticks of a 150-tick take
    are the wind-up and nothing else; a strip of them shows a fighter standing
    still and says the move does nothing.
    """
    if not frames:
        return []
    if len(frames) <= columns:
        return list(enumerate(frames))
    step = len(frames) / columns
    return [(int(i * step), frames[int(i * step)]) for i in range(columns)]


def _rect(x: float, y: float, w: float, h: float, fill: str, stroke: str, width: float = 1.0) -> str:
    return (
        f'<rect x="{x:.2f}" y="{y:.2f}" width="{max(w, 0.1):.2f}" '
        f'height="{max(h, 0.1):.2f}" fill="{fill}" stroke="{stroke}" stroke-width="{width}"/>'
    )


def cell(frame: dict, index: int, view: list[float], size: tuple[float, float]) -> str:
    """One frame, drawn in its own coordinate space."""
    vx, vy, vw, vh = view
    w, h = size
    scale = min(w / max(vw, 1.0), h / max(vh, 1.0))
    # ⛔ THE TAKE'S Y IS DOWNWARD-POSITIVE ALREADY (it is the sim's own frame), so
    # the cell maps straight through. Flipping it here would put every fighter on
    # the ceiling, which is the kind of error a derived picture makes look like a
    # physics bug.
    def px(p: list[float]) -> tuple[float, float]:
        return ((p[0] - vx) * scale, (p[1] - vy) * scale)

    parts: list[str] = [f'<g transform="translate(0,0)">']
    parts.append(_rect(0, 0, w, h, "#101014", "#2a2a33"))
    for body in frame.get("bodies", []):
        cx, cy = px(body["pos"])
        hw, hh = body["half"][0] * scale, body["half"][1] * scale
        subject = body.get("seat") == 0
        parts.append(
            _rect(
                cx - hw,
                cy - hh,
                hw * 2,
                hh * 2,
                "#2b4a6b" if subject else "#33333c",
                "#7fb2e5" if subject else "#5a5a66",
                1.5 if subject else 1.0,
            )
        )
    for box in frame.get("hitboxes", []):
        cx, cy = px(box["pos"])
        hw, hh = box["half"][0] * scale, box["half"][1] * scale
        # Whose swing it is decides the colour: a take records BOTH fighters and
        # the move's own statistics count only the subject's.
        mine = box.get("subject_owned")
        parts.append(
            _rect(cx - hw, cy - hh, hw * 2, hh * 2, "none", "#e5554f" if mine else "#8a5f5c", 1.5)
        )
    for shot in frame.get("projectiles", []):
        cx, cy = px(shot["pos"])
        half = shot.get("half", [4.0, 4.0])
        hw, hh = half[0] * scale, half[1] * scale
        parts.append(_rect(cx - hw, cy - hh, hw * 2, hh * 2, "none", "#e0b84f", 1.2))

    caption = f"t{index}"
    for key in ("move", "pose", "clip"):
        value = frame.get(key)
        if value:
            caption += f" · {value}"
    parts.append(
        f'<text x="4" y="{h - 5:.1f}" font-family="monospace" font-size="9" '
        f'fill="#9aa0aa">{_escape(caption)}</text>'
    )
    parts.append("</g>")
    return "".join(parts)


def _escape(text: str) -> str:
    return (
        str(text)
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


def sheet(take: dict, columns: int = DEFAULT_COLUMNS) -> str:
    """One take as one SVG."""
    frames = take.get("frames", [])
    picked = sample(frames, columns)
    view = take.get("view") or [0.0, 0.0, 320.0, 240.0]
    cw, ch = 160.0, 120.0
    width = max(cw * len(picked), cw)
    height = ch + 34

    cells = []
    for column, (index, frame) in enumerate(picked):
        cells.append(f'<g transform="translate({column * cw:.1f},26)">')
        cells.append(cell(frame, index, view, (cw - 2, ch - 2)))
        cells.append("</g>")

    title = f"{take.get('character', '?')} · {take.get('verb', '?')}"
    outcome = take.get("intended_move")
    if outcome:
        title += f" · intended {outcome}"
        if take.get("reached_intended_move") is False:
            title += " (NOT REACHED)"
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0f}" '
        f'height="{height:.0f}" viewBox="0 0 {width:.0f} {height:.0f}">'
        f'<rect width="100%" height="100%" fill="#0b0b0e"/>'
        f'<text x="6" y="15" font-family="monospace" font-size="12" fill="#dfe3e8">'
        f"{_escape(title)}</text>"
        + "".join(cells)
        + f'<text x="6" y="{height - 6:.0f}" font-family="monospace" font-size="9" '
        f'fill="#c07a3e">{_escape(WATERMARK)}</text>'
        "</svg>"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--takes", required=True, type=Path, help="a moveset_takes takes.json")
    parser.add_argument("--out", required=True, type=Path, help="directory for the SVGs")
    parser.add_argument("--character", help="only this character's takes")
    parser.add_argument("--columns", type=int, default=DEFAULT_COLUMNS)
    args = parser.parse_args()

    doc = json.loads(args.takes.read_text(encoding="utf8"))
    takes = doc.get("takes", doc) if isinstance(doc, dict) else doc
    if args.character:
        takes = [t for t in takes if t.get("character") == args.character]
    if not takes:
        print("no takes matched")
        return 1

    args.out.mkdir(parents=True, exist_ok=True)
    for take in takes:
        name = f"{take.get('character', 'unknown')}.{take.get('verb', 'unknown')}.svg"
        (args.out / name).write_text(sheet(take, args.columns), encoding="utf8")
    print(f"wrote {len(takes)} diagnostic sheet(s) to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
