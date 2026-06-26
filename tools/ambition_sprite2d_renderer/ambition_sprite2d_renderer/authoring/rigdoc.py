"""Serializable rig documents — data-defined characters for the bone toolkit.

A rig document is plain JSON describing a complete character: bones,
drawable parts, palette, IK leg bindings, and animation clips whose
channels are keyframes, math expressions, or constants. This module turns
documents into rendered frames and spritesheets using the same
``skeleton`` + ``tackon_sheet`` machinery as the Python-coded targets.

Consumers:

- ``ambition_sprite2d_renderer.gui`` — the PySide6 character editor;
  documents are its file format.
- ``targets/characters/rigged.py`` — auto-registers every document under
  ``targets/characters/rigged/`` as a sheet target, so GUI-authored
  characters publish through ``./regen_sprites.sh --target <name>`` like
  everything else.

Document shape (all geometry in base-frame pixels, y down, facing +x)::

    {
      "name": "my_bot",
      "frame": {"width": 128, "height": 128, "supersample": 4,
                "ground_y": 101.0, "center_x": 64.0, "ankle_h": 2.6},
      "palette": {"shell": "#FDFDFB", ...},
      "bones": [{"name": "pelvis", "parent": null, "offset": [0, -20.5],
                 "length": 0.0, "rest_angle": 0.0}, ...],
      "parts": [
        {"name": "torso", "bone": "torso", "z": 40, "kind": "polygon",
         "points": [[-9.8, -13.5], ...], "radius": 3.6,
         "fill": "shell", "outline": "outline", "outline_w": 1.15},
        {"kind": "capsule", "a": [0, 0], "b": null, "radius": 2.3, ...},
        {"kind": "circle", "center": [8, 0], "radius": 3.2, ...},
        ... optional "opacity_channel": "slash_vis" on any part ...
        ... optional "feature": "hairpin" tags a part as an optional accessory ...
      ],
      "features": {"hairpin": false, "glasses": true},  # toggle optional parts

      "ik_legs": [{"upper": "near_leg_u", "lower": "near_leg_l",
                   "foot": "near_foot", "channel_prefix": "near_foot",
                   "rest_x": 5.0, "bend": 1.0}],
      "clips": {
        "idle": {"loop": true, "frames": 8, "duration_ms": 120,
                 "channels": {
                   "torso": {"expr": "2.8*sin(tau*t)"},
                   "near_arm_u": {"keys": [[0, 8, "smooth"], [0.5, -4]]},
                   "near_foot_x": {"const": 5.0}}}}
    }

Channel conventions match the Python targets: names that are bones become
pose angles (degrees); ``root_x``/``root_y`` offset the root from
``(center_x, ground_y)``; ``<prefix>_x`` / ``_lift`` / ``_pitch`` drive IK
feet (x is offset from ``center_x`` in WORLD space so planted feet stay
put); anything else is a free parameter (e.g. an ``opacity_channel``).

Colors are palette keys, ``#RRGGBB``, or ``#RRGGBBAA``. Translucent parts
are painted on a scratch layer and alpha-composited (the gnu_ton rule).
"""

from __future__ import annotations

import json
import math
from pathlib import Path
from typing import Dict, List, Optional, Tuple

from PIL import Image, ImageDraw

from .common_draw import draw_capsule
from .rig import clamp, lerp, smoothstep
from .skeleton import (
    BoneWorld,
    Channel,
    Skeleton,
    draw_polygon,
    rounded_polygon,
    two_bone_ik,
)

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]

PART_KINDS = ("polygon", "capsule", "circle", "sprite")
EASE_NAMES = ("linear", "smooth", "out", "in", "sine")

# Restricted namespace for expression channels. Documents are local,
# Jon-authored content; this keeps expressions to math, not a sandbox.
_EXPR_GLOBALS = {
    "__builtins__": {},
    "sin": math.sin,
    "cos": math.cos,
    "tan": math.tan,
    "atan2": math.atan2,
    "sqrt": math.sqrt,
    "exp": math.exp,
    "floor": math.floor,
    "pi": math.pi,
    "tau": math.tau,
    "abs": abs,
    "min": min,
    "max": max,
    "clamp": clamp,
    "lerp": lerp,
    "smoothstep": smoothstep,
}
_EXPR_CACHE: Dict[str, object] = {}


def eval_expr(expr: str, t: float) -> float:
    code = _EXPR_CACHE.get(expr)
    if code is None:
        code = compile(expr, "<channel expr>", "eval")
        _EXPR_CACHE[expr] = code
    return float(eval(code, _EXPR_GLOBALS, {"t": t}))


def sample_channel_spec(spec: dict, t: float, loop: bool) -> float:
    """Sample one channel spec ({"const"} | {"expr"} | {"keys"}) at t."""
    if "const" in spec:
        return float(spec["const"])
    if "expr" in spec:
        return eval_expr(spec["expr"], t % 1.0 if loop else clamp(t, 0.0, 1.0))
    keys = spec.get("keys") or []
    if not keys:
        return 0.0
    ch = Channel(*[tuple(k) for k in keys])
    return ch.sample(t, loop)


def part_visible(part: dict, features: Dict[str, bool]) -> bool:
    """Whether a part renders under the document's ``features`` toggles.

    A part with no ``feature`` tag always renders. A part tagged
    ``"feature": "hairpin"`` renders unless ``features["hairpin"]`` is set
    false — so a character can carry optional accessories (hairpin, glasses,
    hat, …) and toggle each one on/off without editing the parts list. An
    unlisted feature defaults to visible, so existing rigs are unaffected.
    """
    feature = part.get("feature")
    if feature is None:
        return True
    return bool(features.get(feature, True))


def visible_parts(parts: List[dict], features: Dict[str, bool]) -> List[dict]:
    """Parts to paint, back-to-front by ``z``, with disabled features dropped."""
    ordered = sorted(parts, key=lambda p: float(p.get("z", 0.0)))
    return [p for p in ordered if part_visible(p, features)]


def parse_color(value, palette: Dict[str, str], opacity: float = 1.0) -> Optional[Color]:
    """Resolve a palette key / #RRGGBB / #RRGGBBAA into RGBA ints."""
    if value is None:
        return None
    s = palette.get(value, value)
    s = str(s).lstrip("#")
    if len(s) == 8:
        r, g, b, a = (int(s[i : i + 2], 16) for i in (0, 2, 4, 6))
    else:
        r, g, b = (int(s[i : i + 2], 16) for i in (0, 2, 4))
        a = 255
    return (r, g, b, int(a * clamp(opacity, 0.0, 1.0)))


class RigDocument:
    """A mutable rig document: thin helpers over the plain ``data`` dict.

    The GUI edits ``data`` in place; everything here derives from it on
    demand (documents are editor-scale, tens of bones, so no caching)."""

    def __init__(self, data: dict, source_path=None) -> None:
        self.data = data
        # Where this document was loaded from. ``sprite`` parts resolve their
        # SVG relative to this, and the per-resolution sprite raster cache is
        # keyed per instance (so a sheet rasterizes each part once).
        self.source_path = Path(source_path) if source_path is not None else None
        self._sprite_cache: Dict[Tuple[str, int], Tuple[Image.Image, Point]] = {}

    # ---- I/O -------------------------------------------------------------

    @classmethod
    def load(cls, path) -> "RigDocument":
        return cls(
            json.loads(Path(path).read_text(encoding="utf8")), source_path=path
        )

    def save(self, path) -> None:
        Path(path).write_text(
            json.dumps(self.data, indent=1) + "\n", encoding="utf8"
        )

    @classmethod
    def new_empty(cls, name: str = "new_character") -> "RigDocument":
        return cls(
            {
                "name": name,
                "frame": {
                    "width": 128,
                    "height": 128,
                    "supersample": 4,
                    "ground_y": 101.0,
                    "center_x": 64.0,
                    "ankle_h": 2.6,
                },
                "palette": {
                    "shell": "#FDFDFB",
                    "outline": "#17191F",
                    "glow": "#0CEBFF",
                    "accent": "#C58AFF",
                },
                "bones": [
                    {"name": "pelvis", "parent": None, "offset": [0.0, -20.5], "length": 0.0, "rest_angle": 0.0},
                    {"name": "torso", "parent": "pelvis", "offset": [0.0, -4.0], "length": 0.0, "rest_angle": 0.0},
                ],
                "parts": [
                    {"name": "torso", "bone": "torso", "z": 40, "kind": "polygon",
                     "points": [[-10, -14], [10, -14], [9, 2], [-9, 2]], "radius": 3.5,
                     "fill": "shell", "outline": "outline", "outline_w": 1.15},
                ],
                "ik_legs": [],
                "clips": {
                    "idle": {"loop": True, "frames": 8, "duration_ms": 120, "channels": {}},
                },
            }
        )

    # ---- Accessors ---------------------------------------------------------

    @property
    def name(self) -> str:
        return str(self.data.get("name", "unnamed"))

    @property
    def frame(self) -> dict:
        return self.data["frame"]

    @property
    def palette(self) -> Dict[str, str]:
        return self.data.setdefault("palette", {})

    @property
    def bones(self) -> List[dict]:
        return self.data.setdefault("bones", [])

    @property
    def parts(self) -> List[dict]:
        return self.data.setdefault("parts", [])

    @property
    def clips(self) -> Dict[str, dict]:
        return self.data.setdefault("clips", {})

    @property
    def ik_legs(self) -> List[dict]:
        return self.data.setdefault("ik_legs", [])

    @property
    def features(self) -> Dict[str, bool]:
        """Optional-part toggles: ``{feature_name: enabled}``. A part tagged
        with a ``feature`` only renders when its entry here is truthy (or
        absent — features default to on)."""
        return self.data.setdefault("features", {})

    @property
    def svg_source(self) -> Dict[str, object]:
        """Optional source-SVG binding for ``sprite`` parts::

            {"path": "rel/to/this.rig.json/art.svg", "view": "VIEW-front-right",
             "ref_dpi": 96.0, "scale": 0.1845}

        ``sprite`` parts name SVG element ids and a ``pivot`` in *reference
        pixels* (the SVG rendered at ``ref_dpi``); ``scale`` is base-frame units
        per reference pixel, so the same art drives both the bone geometry and
        the rendered raster."""
        return self.data.setdefault("svg_source", {})

    @property
    def sprite_tuning(self) -> Dict[str, float]:
        """Optional in-game sheet tuning, emitted to the RON's ``tuning`` field
        and read by the runtime ``SheetRegistry``:

        - ``collision_scale`` — the in-game display SIZE driver
          (height = collision * collision_scale). Raise it to make a character
          render bigger/taller without touching its gameplay collision box.
        - ``frame_sample_inset`` — pixels trimmed off each atlas cell edge.
        - ``feet_anchor_y_override`` — explicit feet anchor.

        Absent → the runtime's ``DEFAULT_TUNING`` (collision_scale 1.5). This is
        how a rig specifies its own defaults instead of inheriting the fallback."""
        return self.data.setdefault("sprite_tuning", {})

    def bone(self, name: str) -> Optional[dict]:
        for b in self.bones:
            if b["name"] == name:
                return b
        return None

    def rows(self) -> List[Tuple[str, int, int]]:
        return [
            (name, int(c.get("frames", 8)), int(c.get("duration_ms", 100)))
            for name, c in self.clips.items()
        ]

    def ik_bone_names(self) -> set:
        out = set()
        for leg in self.ik_legs:
            out.update({leg.get("upper"), leg.get("lower"), leg.get("foot")})
        out.discard(None)
        return out

    def foot_leg_for_bone(self, bone_name: str) -> Optional[dict]:
        for leg in self.ik_legs:
            if bone_name in (leg.get("foot"), leg.get("lower"), leg.get("upper")):
                return leg
        return None

    # ---- Evaluation ----------------------------------------------------------

    def build_skeleton(self) -> Skeleton:
        sk = Skeleton()
        for b in self.bones:
            sk.bone(
                b["name"],
                parent=b.get("parent"),
                offset=tuple(b.get("offset", (0.0, 0.0))),
                length=float(b.get("length", 0.0)),
                rest_angle=float(b.get("rest_angle", 0.0)),
            )
        return sk

    def sample(self, clip_name: str, t: float) -> Dict[str, float]:
        clip = self.clips.get(clip_name) or {"channels": {}}
        loop = bool(clip.get("loop", True))
        return {
            name: sample_channel_spec(spec, t, loop)
            for name, spec in clip.get("channels", {}).items()
        }

    def solve(self, clip_name: str, t: float):
        """Sample channels, run IK legs, return (bone worlds, params)."""
        s = self.sample(clip_name, t)
        fr = self.frame
        cx = float(fr.get("center_x", fr["width"] / 2))
        gy = float(fr.get("ground_y", fr["height"] - 2))
        ankle_h = float(fr.get("ankle_h", 0.0))
        root = (cx + s.get("root_x", 0.0), gy + s.get("root_y", 0.0))
        sk = self.build_skeleton()
        angles = {n: v for n, v in s.items() if n in sk.bones}
        w0 = sk.world(angles, root=root)
        for leg in self.ik_legs:
            up, lo = leg["upper"], leg["lower"]
            if up not in sk.bones or lo not in sk.bones:
                continue
            pre = leg.get("channel_prefix", "foot")
            hip = w0[up].origin
            x = s.get(f"{pre}_x", float(leg.get("rest_x", 0.0)))
            lift = s.get(f"{pre}_lift", 0.0)
            ankle = (cx + x, gy - ankle_h - lift)
            a1, a2 = two_bone_ik(
                hip, ankle, sk.bones[up].length, sk.bones[lo].length,
                bend=float(leg.get("bend", 1.0)),
            )
            parent = sk.bones[up].parent
            parent_angle = w0[parent].angle if parent else 0.0
            angles[up] = a1 - parent_angle - sk.bones[up].rest_angle
            angles[lo] = a2 - a1 - sk.bones[lo].rest_angle
            foot = leg.get("foot")
            if foot and foot in sk.bones:
                pitch = s.get(f"{pre}_pitch", 0.0)
                angles[foot] = pitch - a2 - sk.bones[foot].rest_angle
        return sk.world(angles, root=root), s

    # ---- Sprite parts (rasterized SVG subsets) ------------------------------

    def _svg_path(self) -> Optional[Path]:
        src = self.svg_source.get("path")
        if not src:
            return None
        p = Path(str(src))
        if not p.is_absolute() and self.source_path is not None:
            p = (self.source_path.parent / p).resolve()
        return p

    def sprite_image(self, part: dict, S: float) -> Optional[Tuple[Image.Image, Point]]:
        """Rasterize a ``sprite`` part's SVG subset at composite scale ``S``.

        Returns ``(cropped RGBA, pivot_in_crop_px)`` — the pivot is the part's
        joint (``pivot``, in reference px) located inside the cropped raster.
        Cached per ``(part, round(S))`` so a whole sheet rasterizes each part
        once. ``None`` if the SVG is unavailable or the subset renders empty."""
        svg_path = self._svg_path()
        if svg_path is None or not svg_path.exists():
            return None
        key = (part.get("name", ""), int(round(S * 256)))
        cached = self._sprite_cache.get(key)
        if cached is not None:
            return cached
        from .svg_parts import rasterize_subset

        src = self.svg_source
        ref_dpi = float(src.get("ref_dpi", 96.0))
        scale = float(src.get("scale", 1.0))  # base-frame units per reference px
        view = str(src.get("view", ""))
        # 1 ref-px -> scale*S composite px, so render the SVG at this dpi.
        dpi = ref_dpi * scale * S
        img, (off_x, off_y), _ppu = rasterize_subset(
            svg_path, view, list(part.get("include", [])), dpi
        )
        if img is None:
            return None
        px, py = part.get("pivot", (0.0, 0.0))
        pivot = (px * scale * S - off_x, py * scale * S - off_y)
        self._sprite_cache[key] = (img, pivot)
        return img, pivot

    # ---- Painting -----------------------------------------------------------

    def render_at(
        self,
        clip_name: str,
        t: float,
        supersample: Optional[int] = None,
        scale: Optional[int] = None,
    ) -> Image.Image:
        """Render one frame at normalized time ``t`` (continuous — the GUI
        scrubs with this). Output size is ``(width*scale, height*scale)``;
        ``scale`` defaults to the document's ``frame.render_scale`` (1), so
        a doc can publish at 2x/4x resolution while geometry stays authored
        in base-frame units."""
        fr = self.frame
        w, h = int(fr["width"]), int(fr["height"])
        rs = int(scale if scale is not None else fr.get("render_scale", 1))
        rs = max(1, rs)
        ss = max(1, int(supersample if supersample is not None else fr.get("supersample", 4)))
        S = float(rs * ss)
        img = Image.new("RGBA", (int(w * S), int(h * S)), (0, 0, 0, 0))
        draw = ImageDraw.Draw(img)
        world, params = self.solve(clip_name, t)
        for part in visible_parts(self.parts, self.features):
            sprite = self.sprite_image(part, S) if part.get("kind") == "sprite" else None
            paint_part(img, draw, part, world, S, params, self.palette, sprite=sprite)
        if ss == 1:
            return img
        return img.resize((w * rs, h * rs), Image.Resampling.LANCZOS)

    def frame_time(self, clip_name: str, frame_idx: int, nframes: Optional[int] = None) -> float:
        """Normalized time for a frame index under the loop conventions:
        loops sample i/n (no duplicated end frame), one-shots i/(n-1)."""
        clip = self.clips.get(clip_name) or {}
        n = int(nframes or clip.get("frames", 8))
        if bool(clip.get("loop", True)):
            return frame_idx / max(1, n)
        return frame_idx / max(1, n - 1)

    def render_frame(self, clip_name: str, frame_idx: int, nframes: int) -> Image.Image:
        return self.render_at(clip_name, self.frame_time(clip_name, frame_idx, nframes))


# ---- Part painting -----------------------------------------------------------
# Module-level so generated Python targets (rigdoc_codegen) can paint the
# same part vocabulary without carrying a RigDocument around.


def blit_rotated(
    canvas: Image.Image,
    sprite: Image.Image,
    pivot: Point,
    world_px: Point,
    delta_deg: float,
    opacity: float = 1.0,
) -> None:
    """Rotate ``sprite`` about its ``pivot`` by ``delta_deg`` and composite it so
    the pivot lands at ``world_px``. The sprite is padded into a square centered
    on the pivot so the rotation keeps the pivot fixed."""
    px, py = pivot
    w, h = sprite.size
    radius = max(
        math.hypot(cx - px, cy - py) for cx in (0, w) for cy in (0, h)
    )
    R = int(math.ceil(radius)) + 2
    pad = Image.new("RGBA", (2 * R, 2 * R), (0, 0, 0, 0))
    pad.alpha_composite(sprite, (R - int(round(px)), R - int(round(py))))
    # PIL rotates counter-clockwise in image space; with +y down that matches the
    # toolkit's clockwise-positive screen angle, so pass delta straight through.
    rot = pad.rotate(delta_deg, resample=Image.Resampling.BICUBIC, center=(R, R))
    if opacity < 1.0:
        rot.putalpha(rot.getchannel("A").point(lambda v: int(v * opacity)))
    canvas.alpha_composite(
        rot, (int(round(world_px[0])) - R, int(round(world_px[1])) - R)
    )


def paint_part(
    img: Image.Image,
    draw: ImageDraw.ImageDraw,
    part: dict,
    world: Dict[str, BoneWorld],
    S: float,
    params: Dict[str, float],
    palette: Dict[str, str],
    sprite: Optional[Tuple[Image.Image, Point]] = None,
) -> None:
    bone_name = part.get("bone")
    if bone_name not in world:
        return
    opacity = 1.0
    oc = part.get("opacity_channel")
    if oc:
        # Default 0: a part bound to an opacity channel is HIDDEN in clips
        # that don't drive that channel (a blade only shows in clips that
        # animate slash_vis).
        opacity = clamp(params.get(oc, 0.0), 0.0, 1.0)
        if opacity <= 0.01:
            return
    if part.get("kind") == "sprite":
        if sprite is None:
            return
        bw = world[bone_name]
        spr_img, pivot = sprite
        delta = bw.angle - float(part.get("rest_angle", 0.0))
        blit_rotated(
            img, spr_img, pivot, (bw.origin[0] * S, bw.origin[1] * S), delta, opacity
        )
        return
    fill = parse_color(part.get("fill", "#FFFFFF"), palette, opacity)
    outline = parse_color(part.get("outline"), palette, opacity)
    ow = float(part.get("outline_w", 0.0)) * S
    translucent = (fill is not None and fill[3] < 255) or (
        outline is not None and outline[3] < 255
    )
    if translucent:
        # gnu_ton rule: translucent shapes composite via a scratch layer;
        # drawing them directly would replace destination alpha.
        target = Image.new("RGBA", img.size, (0, 0, 0, 0))
        tdraw = ImageDraw.Draw(target)
    else:
        target, tdraw = img, draw
    bw = world[bone_name]
    kind = part.get("kind", "polygon")
    if kind == "polygon":
        pts = [
            (p[0] * S, p[1] * S)
            for p in (bw.to_world(tuple(q)) for q in part.get("points", []))
        ]
        if len(pts) >= 3:
            radius = float(part.get("radius", 0.0)) * S
            poly = rounded_polygon(pts, radius) if radius > 0 else pts
            draw_polygon(tdraw, poly, fill, outline, ow)
    elif kind == "capsule":
        a_local = tuple(part.get("a", (0.0, 0.0)))
        b_local = part.get("b")
        if b_local is None:
            b_local = (bw.length, 0.0)
        a = bw.to_world(a_local)
        b = bw.to_world(tuple(b_local))
        r = float(part.get("radius", 2.0)) * S
        draw_capsule(
            tdraw,
            (a[0] * S, a[1] * S),
            (b[0] * S, b[1] * S),
            r,
            fill,
            outline if outline is not None else fill,
            ow * 0.5,
        )
    elif kind == "circle":
        # Optional "ry" stretches the circle into an ellipse (eyes).
        c = bw.to_world(tuple(part.get("center", (0.0, 0.0))))
        rx = float(part.get("radius", 2.0)) * S
        ry = float(part.get("ry", part.get("radius", 2.0))) * S
        box = (c[0] * S - rx, c[1] * S - ry, c[0] * S + rx, c[1] * S + ry)
        if outline is not None and ow > 0:
            tdraw.ellipse(box, fill=fill, outline=outline, width=max(1, int(ow)))
        else:
            tdraw.ellipse(box, fill=fill)
    if translucent:
        img.alpha_composite(target)


# ---- Sheet / GIF export ----------------------------------------------------


def render_sheet_for_doc(doc: RigDocument, out_dir: Path) -> List[Path]:
    """Render the document's full spritesheet bundle (PNG + YAML + RON +
    canonical + preview) via the standard tack-on sheet builder."""
    from .tackon_sheet import build_sheet

    fr = doc.frame
    rs = max(1, int(fr.get("render_scale", 1)))
    outputs = build_sheet(
        target=doc.name,
        rows=doc.rows(),
        render_fn=doc.render_frame,
        out_dir=Path(out_dir),
        frame_size=(int(fr["width"]) * rs, int(fr["height"]) * rs),
        sheet_tuning=doc.sprite_tuning or None,
    )
    keys = ("spritesheet", "yaml", "ron", "actor", "canonical", "canonical_transparent", "preview")
    return [Path(outputs[k]) for k in keys if outputs.get(k)]


def render_gifs_for_doc(doc: RigDocument, out_dir: Path, scale: int = 2) -> List[Path]:
    """One looping GIF per clip, rendered at ``scale``x base size."""
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    fr = doc.frame
    w, h = int(fr["width"]) * scale, int(fr["height"]) * scale
    paths: List[Path] = []
    for name, frames, duration_ms in doc.rows():
        imgs = []
        for i in range(frames):
            frame = doc.render_frame(name, i, frames)
            bg = Image.new("RGBA", frame.size, (43, 33, 40, 255))
            bg.alpha_composite(frame)
            imgs.append(bg.convert("P").resize((w, h), Image.Resampling.NEAREST))
        path = out_dir / f"{doc.name}_{name}.gif"
        imgs[0].save(
            path, save_all=True, append_images=imgs[1:],
            duration=duration_ms, loop=0, disposal=2,
        )
        paths.append(path)
    return paths
