"""Export a rig document as a readable, runnable Python target module.

The generated module mirrors the structure of the hand-written reference
target (``targets/characters/player_robot_fable.py``): a ``Skeleton``
built bone-by-bone, ``Clip`` objects whose channels are ``Channel``
keyframes / plain lambdas / constants, parts painted via
``rigdoc.paint_part``, and the standard ``render`` / ``render_canonical``
hooks so dropping the file under ``targets/characters/`` registers it as
a sheet target.

This is the bridge from GUI-authored content back to code: an agent can
read the generated module to see exactly how a character is rigged and
animated programmatically, then edit it like any other Python target.

Generated modules use absolute imports so they also run standalone (e.g.
imported from a scratch directory in tests).
"""

from __future__ import annotations

from .rigdoc import RigDocument, visible_parts


def _fmt_value(v) -> str:
    if isinstance(v, float):
        s = f"{v:g}"
        return s
    return repr(v)


def _fmt_key(k) -> str:
    parts = [_fmt_value(float(k[0])), _fmt_value(float(k[1]))]
    if len(k) > 2 and k[2] != "smooth":
        parts.append(repr(k[2]))
    return f"({', '.join(parts)})"


def _fmt_channel(spec: dict) -> str:
    if "expr" in spec:
        return f"lambda t: {spec['expr']}"
    if "const" in spec:
        return _fmt_value(float(spec["const"]))
    keys = ", ".join(_fmt_key(k) for k in spec.get("keys", []))
    return f"Channel({keys})"


def _fmt_part(part: dict) -> str:
    keys = (
        "name", "bone", "z", "kind", "points", "radius", "ry", "a", "b",
        "center", "fill", "outline", "outline_w", "opacity_channel",
    )
    items = [f"{k!r}: {part[k]!r}" for k in keys if k in part]
    return "    {" + ", ".join(items) + "},"


def doc_to_python(doc: RigDocument) -> str:
    fr = doc.frame
    lines: list = []
    a = lines.append

    a(f'"""{doc.name} — generated from a rig document by the rig editor.')
    a("")
    a("Structure mirrors targets/characters/player_robot_fable.py (the")
    a("hand-written reference for the bone toolkit). Everything here is")
    a("plain code: edit bones, parts, and clips directly, then publish via")
    a(f"`./regen_sprites.sh --target {doc.name}` after dropping this file")
    a("under targets/characters/. If the source .rig.json is also saved in")
    a("targets/characters/rigged/ under the same name, that registration")
    a('shadows this module — keep one or rename.\n"""')
    a("")
    a("from __future__ import annotations")
    a("")
    a("from math import atan2, cos, exp, floor, pi, sin, sqrt, tan, tau  # noqa: F401 - for lambdas")
    a("from pathlib import Path")
    a("from typing import Dict, List, Tuple")
    a("")
    a("from PIL import Image, ImageDraw")
    a("")
    a("from ambition_sprite2d_renderer.authoring.rig import clamp, lerp, smoothstep  # noqa: F401 - for lambdas")
    a("from ambition_sprite2d_renderer.authoring.rigdoc import paint_part")
    a("from ambition_sprite2d_renderer.authoring.skeleton import Channel, Clip, Skeleton, two_bone_ik")
    a("from ambition_sprite2d_renderer.authoring.tackon_sheet import build_sheet, write_canonical")
    a("")
    a(f"TARGET_NAME = {doc.name!r}")
    a(f"FRAME_W, FRAME_H = {int(fr['width'])}, {int(fr['height'])}")
    a(f"RENDER_SCALE = {max(1, int(fr.get('render_scale', 1)))}")
    a(f"SS = {int(fr.get('supersample', 4))}  # anti-aliasing supersample on top of RENDER_SCALE")
    a(f"GROUND_Y = {float(fr.get('ground_y', 101.0))}")
    a(f"CENTER_X = {float(fr.get('center_x', 64.0))}")
    a(f"ANKLE_H = {float(fr.get('ankle_h', 2.6))}")
    a("")
    a("PALETTE: Dict[str, str] = {")
    for name, value in doc.palette.items():
        a(f"    {name!r}: {value!r},")
    a("}")
    a("")
    a("")
    a("def build_skeleton() -> Skeleton:")
    a("    sk = Skeleton()")
    for b in doc.bones:
        args = [repr(b["name"])]
        if b.get("parent"):
            args.append(f"parent={b['parent']!r}")
        off = b.get("offset", [0.0, 0.0])
        args.append(f"offset=({_fmt_value(float(off[0]))}, {_fmt_value(float(off[1]))})")
        if float(b.get("length", 0.0)):
            args.append(f"length={_fmt_value(float(b['length']))}")
        if float(b.get("rest_angle", 0.0)):
            args.append(f"rest_angle={_fmt_value(float(b['rest_angle']))}")
        a(f"    sk.bone({', '.join(args)})")
    a("    return sk")
    a("")
    a("")
    a("SKEL = build_skeleton()")
    a("")
    a("# Parts are painted back-to-front by z via rigdoc.paint_part — the same")
    a("# vocabulary the GUI edits (polygon/capsule/circle, palette refs,")
    a("# optional opacity_channel). Parts whose `feature` is toggled off in the")
    a("# document's `features` map are dropped at eject time.")
    a("PARTS: List[dict] = [")
    for part in visible_parts(doc.parts, doc.features):
        a(_fmt_part(part))
    a("]")
    a("")
    a("# IK legs: feet are authored as world-space targets via the")
    a("# '<channel_prefix>_x/_lift/_pitch' channels; knees solve via two_bone_ik.")
    a(f"IK_LEGS: List[dict] = {doc.ik_legs!r}")
    a("")
    a("CLIPS: Dict[str, Clip] = {")
    for clip_name, clip in doc.clips.items():
        a(f"    {clip_name!r}: Clip(loop={bool(clip.get('loop', True))!r}, channels={{")
        for ch_name, spec in clip.get("channels", {}).items():
            a(f"        {ch_name!r}: {_fmt_channel(spec)},")
        a("    }),")
    a("}")
    a("")
    a("ROWS: List[Tuple[str, int, int]] = [")
    for name, frames, duration in doc.rows():
        a(f"    ({name!r}, {frames}, {duration}),")
    a("]")
    a("")
    a("")
    a("def _solve(animation: str, t: float):")
    a('    """Sample the clip, run leg IK, return (bone worlds, params)."""')
    a("    sampled = CLIPS[animation].sample(t)")
    a("    root = (CENTER_X + sampled.get('root_x', 0.0), GROUND_Y + sampled.get('root_y', 0.0))")
    a("    angles = {n: v for n, v in sampled.items() if n in SKEL.bones}")
    a("    w0 = SKEL.world(angles, root=root)")
    a("    for leg in IK_LEGS:")
    a("        up, lo = leg['upper'], leg['lower']")
    a("        pre = leg.get('channel_prefix', 'foot')")
    a("        hip = w0[up].origin")
    a("        x = sampled.get(f'{pre}_x', float(leg.get('rest_x', 0.0)))")
    a("        lift = sampled.get(f'{pre}_lift', 0.0)")
    a("        ankle = (CENTER_X + x, GROUND_Y - ANKLE_H - lift)")
    a("        a1, a2 = two_bone_ik(hip, ankle, SKEL.bones[up].length, SKEL.bones[lo].length,")
    a("                             bend=float(leg.get('bend', 1.0)))")
    a("        parent = SKEL.bones[up].parent")
    a("        parent_angle = w0[parent].angle if parent else 0.0")
    a("        angles[up] = a1 - parent_angle - SKEL.bones[up].rest_angle")
    a("        angles[lo] = a2 - a1 - SKEL.bones[lo].rest_angle")
    a("        foot = leg.get('foot')")
    a("        if foot and foot in SKEL.bones:")
    a("            angles[foot] = sampled.get(f'{pre}_pitch', 0.0) - a2 - SKEL.bones[foot].rest_angle")
    a("    return SKEL.world(angles, root=root), sampled")
    a("")
    a("")
    a("def render_frame(animation: str, frame_idx: int, nframes: int) -> Image.Image:")
    a("    clip = CLIPS[animation]")
    a("    t = frame_idx / max(1, nframes) if clip.loop else frame_idx / max(1, nframes - 1)")
    a("    S = float(RENDER_SCALE * SS)")
    a("    img = Image.new('RGBA', (int(FRAME_W * S), int(FRAME_H * S)), (0, 0, 0, 0))")
    a("    draw = ImageDraw.Draw(img)")
    a("    world, params = _solve(animation, t)")
    a("    for part in PARTS:")
    a("        paint_part(img, draw, part, world, S, params, PALETTE)")
    a("    return img.resize((FRAME_W * RENDER_SCALE, FRAME_H * RENDER_SCALE), Image.Resampling.LANCZOS)")
    a("")
    a("")
    a("def render(out_dir: Path, **opts) -> List[Path]:")
    a("    del opts")
    a("    Path(out_dir).mkdir(parents=True, exist_ok=True)")
    a("    outputs = build_sheet(")
    a("        target=TARGET_NAME,")
    a("        rows=ROWS,")
    a("        render_fn=render_frame,")
    a("        out_dir=Path(out_dir),")
    a("        frame_size=(FRAME_W * RENDER_SCALE, FRAME_H * RENDER_SCALE),")
    a("    )")
    a("    keys = ('spritesheet', 'yaml', 'ron', 'actor', 'canonical', 'canonical_transparent', 'preview')")
    a("    return [Path(outputs[k]) for k in keys if outputs.get(k)]")
    a("")
    a("")
    a("def render_canonical(out_dir: Path, **opts) -> Path:")
    a("    del opts")
    a("    return write_canonical(TARGET_NAME, ROWS, render_frame, Path(out_dir))")
    a("")
    return "\n".join(lines)
