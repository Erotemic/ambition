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
import math
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


def key_frames(take: dict, columns: int) -> list[tuple[int, dict, str]]:
    """The ticks that MEAN something, each labelled with why it was picked.

    ⭐⭐ AN EVEN STRIP SAMPLES THE CLOCK, NOT THE MOVE. A jab is live for five of
    a hundred and fifty ticks, so twelve evenly spaced frames will usually miss
    every one of them and show a fighter standing still — which is exactly the
    picture that makes somebody conclude the move does nothing. These are the
    moments an authoring question is about, taken from the measurements the
    runtime published rather than from a stride.

    Falls back to an even strip when a take carries none of the fields (an older
    recording), because half a picture beats a refusal.
    """
    frames = take.get("frames") or []
    if not frames:
        return []
    m = _measure(take)
    picks: dict[int, str] = {}

    def note(tick: int, label: str, *, only_if_new: bool = False) -> None:
        # ⭐ LABELS ACCUMULATE. The first live volume and the first contact are
        # often the same tick, and a cell that named only one of them would hide
        # the coincidence that answers "did it connect the moment it went live".
        if tick in picks and picks[tick]:
            if only_if_new or label in picks[tick]:
                return
            picks[tick] = f"{picks[tick]} · {label}"
        else:
            picks[tick] = label

    note(0, "start")
    startup, active = m.get("startup"), m.get("active")
    if startup:
        note(startup["last_tick"], "last startup")
    if m.get("first_active_tick") is not None:
        note(m["first_active_tick"], "first live volume")
    if m.get("first_contact_tick") is not None:
        note(m["first_contact_tick"], "FIRST CONTACT")
    if m.get("max_reach_tick") is not None:
        note(m["max_reach_tick"], f"max reach {m['max_reach_px']:g}px")
    for spawn in m.get("spawns") or []:
        note(spawn["tick"], f"{spawn['kind']} spawned")
    if active:
        note(active["last_tick"], "last active")
    recovery = m.get("recovery")
    if recovery:
        note(recovery["last_tick"], "end of recovery")
    note(len(frames) - 1, "end", only_if_new=True)

    chosen = sorted(tick for tick in picks if 0 <= tick < len(frames))
    # ⛔ THE MEANINGFUL FRAMES COME FIRST. When there is room left, fill it with
    # an even spread so the strip still reads as a motion; when there is not,
    # the semantic ones are the ones that survive.
    if len(chosen) < columns:
        for tick, _ in sample(frames, columns):
            if len(chosen) >= columns:
                break
            if tick not in picks:
                picks[tick] = ""
                chosen = sorted(picks)
    if len(chosen) > columns:
        # Keep the labelled ones; an unlabelled filler is the first to go.
        labelled = [t for t in chosen if picks[t]]
        chosen = sorted(labelled[:columns])
    return [(tick, frames[tick], picks[tick]) for tick in chosen]


def _measure(take: dict) -> dict:
    """The measurements, from the one tool that owns them.

    ⛔ NOT A SECOND IMPLEMENTATION. `moveset_report` derives every one of these
    from the runtime's published observation; a filmstrip that computed "first
    contact" its own way would be a second answer to a question that has one.
    """
    import importlib.util
    import sys

    if "moveset_report" not in sys.modules:
        spec = importlib.util.spec_from_file_location(
            "moveset_report", Path(__file__).resolve().parent / "moveset_report.py"
        )
        module = importlib.util.module_from_spec(spec)
        sys.modules["moveset_report"] = module
        spec.loader.exec_module(module)
    return sys.modules["moveset_report"].measure(take)


def _rect(x: float, y: float, w: float, h: float, fill: str, stroke: str, width: float = 1.0) -> str:
    return (
        f'<rect x="{x:.2f}" y="{y:.2f}" width="{max(w, 0.1):.2f}" '
        f'height="{max(h, 0.1):.2f}" fill="{fill}" stroke="{stroke}" stroke-width="{width}"/>'
    )


# ⭐⭐ THE SEMANTIC ROLE DECIDES THE COLOUR, and the role is also written as
# text. A sheet read months later, or by a model that cannot ask, must not need
# a legend to say which fighter the move belongs to.
ROLE_FILL = {
    "subject": "#2b4a6b",
    "target": "#6b4a2b",
    "subject_owned": "#2b6b52",
    "target_owned": "#6b6b2b",
    "other": "#33333c",
}
ROLE_STROKE = {
    "subject": "#7fb2e5",
    "target": "#e5a54f",
    "subject_owned": "#5fc9a4",
    "target_owned": "#c9c05f",
    "other": "#5a5a66",
}
ROLE_TAG = {"subject": "SUBJECT", "target": "TARGET"}


def role_of(row: dict, take: dict) -> str:
    """What this row IS, with a fallback for takes recorded before roles existed.

    ⛔ A v1 take carried a seat index and a boolean; an old artifact must still
    draw. What an old file may contain does not define what a new one emits.
    """
    role = row.get("role")
    if role:
        return role
    owned = row.get("subject_owned")
    if owned is True:
        return "subject_owned"
    if owned is False:
        return "other"
    seat = row.get("seat")
    if seat is not None:
        return "subject" if seat == take.get("seat", 0) else "target"
    return "other"


def _volume(volume: dict, px, scale: float, fill: str, stroke: str, width: float = 1.0) -> str:
    """One combat volume, in its REAL shape.

    ⛔⛔ A rotated box, a disc and a convex arc are all merely CONTAINED by the
    axis-aligned box around them, and for a sweeping arc that box is a great deal
    larger than the thing that can actually hit you. The recorded `shape` is
    drawn where there is one; the AABB is the honest fallback for an older take.
    """
    shape = volume.get("shape") or {}
    kind = shape.get("kind")
    if kind == "circle":
        cx, cy = px(shape["center"])
        return (
            f'<circle cx="{cx:.2f}" cy="{cy:.2f}" r="{shape["radius"] * scale:.2f}" '
            f'fill="{fill}" stroke="{stroke}" stroke-width="{width}"/>'
        )
    if kind == "convex" and len(shape.get("points") or []) > 2:
        points = " ".join(f"{x:.2f},{y:.2f}" for x, y in (px(p) for p in shape["points"]))
        return (
            f'<polygon points="{points}" fill="{fill}" stroke="{stroke}" '
            f'stroke-width="{width}"/>'
        )
    if kind == "obb":
        cx, cy = shape["center"]
        hx, hy = shape["half"]
        cos, sin = math.cos(shape["rotation"]), math.sin(shape["rotation"])
        corners = [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)]
        world = [(cx + ox * cos - oy * sin, cy + ox * sin + oy * cos) for ox, oy in corners]
        points = " ".join(f"{x:.2f},{y:.2f}" for x, y in (px(list(p)) for p in world))
        return (
            f'<polygon points="{points}" fill="{fill}" stroke="{stroke}" '
            f'stroke-width="{width}"/>'
        )
    cx, cy = px(volume["pos"])
    hw, hh = volume["half"][0] * scale, volume["half"][1] * scale
    return _rect(cx - hw, cy - hh, hw * 2, hh * 2, fill, stroke, width)


def cell(
    frame: dict,
    index: int,
    view: list[float],
    size: tuple[float, float],
    take: dict,
    event: str = "",
) -> str:
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
        role = role_of(body, take)
        cx, cy = px(body["pos"])
        hw, hh = body["half"][0] * scale, body["half"][1] * scale
        parts.append(
            _rect(
                cx - hw,
                cy - hh,
                hw * 2,
                hh * 2,
                ROLE_FILL.get(role, ROLE_FILL["other"]),
                ROLE_STROKE.get(role, ROLE_STROKE["other"]),
                1.5 if role == "subject" else 1.0,
            )
        )
        # ⭐⭐ DAMAGEABLE GEOMETRY, WHICH IS HALF THE INTERACTION. A sheet showing
        # only attack volumes cannot explain why an attack missed, or whether an
        # apparent overlap could have connected at all.
        for hurt in body.get("hurtboxes") or []:
            parts.append(_volume(hurt, px, scale, "none", "#49c8d8", 1.0))
        # ⛔ AN EMPTY LIST IS A DECISION, NOT A GAP. A body nothing can hit this
        # frame looks identical to one the recording failed to resolve.
        if body.get("hurtbox_source") == "intangible":
            parts.append(
                f'<text x="{cx - hw:.1f}" y="{cy + hh + 9:.1f}" font-family="monospace" '
                f'font-size="7" fill="#49c8d8">INTANGIBLE</text>'
            )
        tag = ROLE_TAG.get(role)
        if tag:
            parts.append(
                f'<text x="{cx - hw:.1f}" y="{cy - hh - 3:.1f}" font-family="monospace" '
                f'font-size="7" fill="{ROLE_STROKE.get(role)}">{tag}</text>'
            )
    for box in frame.get("hitboxes", []):
        # Whose swing it is decides the colour: a take records BOTH fighters and
        # the move's own statistics count only the subject's.
        mine = role_of(box, take) == "subject_owned"
        parts.append(
            _volume(box, px, scale, "none", "#e5554f" if mine else "#8a5f5c", 1.5)
        )
    for shot in frame.get("projectiles", []):
        cx, cy = px(shot["pos"])
        half = shot.get("half", [4.0, 4.0])
        hw, hh = half[0] * scale, half[1] * scale
        mine = role_of(shot, take) == "subject_owned"
        parts.append(
            _rect(cx - hw, cy - hh, hw * 2, hh * 2, "none", "#e0b84f" if mine else "#8a7f4f", 1.2)
        )

    caption = f"t{index}"
    for key in ("move", "pose", "clip"):
        value = frame.get(key)
        if value:
            caption += f" · {value}"
    # ⭐ THE MOVE CLOCK. "a red box appeared on t37" is not frame data; the
    # authored window the clock is inside is.
    subject = next(
        (b for b in frame.get("bodies", []) if role_of(b, take) == "subject"), None
    )
    move_state = (subject or {}).get("move_state") or {}
    if move_state.get("phase"):
        caption += f" · {move_state['phase']}"
    parts.append(
        f'<text x="4" y="{h - 5:.1f}" font-family="monospace" font-size="9" '
        f'fill="#9aa0aa">{_escape(caption)}</text>'
    )
    # ⭐ WHY THIS FRAME IS IN THE STRIP. A key-frame sheet whose cells did not say
    # what they were chosen for is an even strip with gaps.
    if event:
        parts.append(
            f'<text x="4" y="11" font-family="monospace" font-size="8" '
            f'fill="#e8c15a">{_escape(event)}</text>'
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


def sheet(take: dict, columns: int = DEFAULT_COLUMNS, select: str = "key") -> str:
    """One take as one SVG."""
    frames = take.get("frames", [])
    if select == "key":
        picked = key_frames(take, columns)
    else:
        picked = [(tick, frame, "") for tick, frame in sample(frames, columns)]
    view = take.get("view") or [0.0, 0.0, 320.0, 240.0]
    cw, ch = 160.0, 120.0
    width = max(cw * len(picked), cw)
    height = ch + 34

    cells = []
    for column, (index, frame, event) in enumerate(picked):
        cells.append(f'<g transform="translate({column * cw:.1f},26)">')
        cells.append(cell(frame, index, view, (cw - 2, ch - 2), take, event))
        cells.append("</g>")

    title = f"{take.get('character', '?')} · {take.get('verb', '?')}"
    # ⭐ WHO IT WAS PERFORMED AGAINST, and how that target behaved. The same move
    # against a live opponent and against a passive one are two measurements.
    if take.get("target"):
        title += f" · vs {take['target']}"
        if take.get("target_behavior"):
            title += f" ({take['target_behavior']})"
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
    parser.add_argument(
        "--select",
        choices=("key", "even"),
        default="key",
        help="key: the ticks that mean something (default). even: a fixed stride.",
    )
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
        (args.out / name).write_text(
            sheet(take, args.columns, args.select), encoding="utf8"
        )
    print(f"wrote {len(takes)} diagnostic sheet(s) to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
