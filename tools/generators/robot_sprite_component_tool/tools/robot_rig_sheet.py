#!/usr/bin/env python3
"""Assemble extracted robot components into production sprite sheets.

This is a lightweight, Pillow-only compositor intended to sit downstream of
``robot_asset_tool.py``:

    rough metadata -> refined metadata -> transparent slices -> assembled frames

The tool is deliberately data-driven.  The default animation poses below are a
starter procedural rig, not final gameplay tuning.  They are useful because they
prove that the refined component atlas can be assembled into fixed-canvas frames
and sprite-sheet manifests without asking an image model to do layout.
"""
from __future__ import annotations

import argparse
import json
import math
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterable, List, Mapping, Optional, Sequence, Tuple

import yaml
import numpy as np
from PIL import Image, ImageColor, ImageDraw, ImageFont

Point = Tuple[float, float]
RGBA = Tuple[int, int, int, int]


ANIMATIONS: Dict[str, Dict[str, int]] = {
    "idle": {"frames": 8, "duration_ms": 120},
    "walk": {"frames": 8, "duration_ms": 95},
    "run": {"frames": 8, "duration_ms": 75},
    "jump": {"frames": 6, "duration_ms": 95},
    "fall": {"frames": 6, "duration_ms": 95},
    "slash": {"frames": 7, "duration_ms": 75},
    "hit": {"frames": 5, "duration_ms": 90},
    "death": {"frames": 8, "duration_ms": 110},
    "teleport": {"frames": 8, "duration_ms": 62},
    "dash": {"frames": 6, "duration_ms": 65},
}

DEFAULT_ANIMATIONS = list(ANIMATIONS.keys())


@dataclass
class RenderConfig:
    frame_width: int = 192
    frame_height: int = 192
    label_width: int = 54
    border: int = 2
    sheet_background: str = "transparent"
    frame_background: str = "transparent"
    scale: float = 0.275
    root_x: float = 0.0
    root_y: float = -12.0


@dataclass
class RigJob:
    metadata: Path
    slices: Path
    animations: List[str] = field(default_factory=lambda: list(DEFAULT_ANIMATIONS))
    render: RenderConfig = field(default_factory=RenderConfig)
    output_dir: Path = Path("output/assembled")

    @classmethod
    def load(cls, path: str | Path) -> "RigJob":
        path = Path(path)
        data = yaml.safe_load(path.read_text()) or {}
        base = path.parent
        render_data = data.get("render", {}) or {}
        render = RenderConfig(**{k: v for k, v in render_data.items() if hasattr(RenderConfig, k)})
        metadata = Path(data.get("metadata", "../metadata/robot_components.refined.yaml"))
        slices = Path(data.get("slices", "../output/slices"))
        output_dir = Path(data.get("output_dir", "../output/assembled"))
        if not metadata.is_absolute():
            metadata = (base / metadata).resolve()
        if not slices.is_absolute():
            slices = (base / slices).resolve()
        if not output_dir.is_absolute():
            output_dir = (base / output_dir).resolve()
        animations = list(data.get("animations", DEFAULT_ANIMATIONS))
        return cls(metadata=metadata, slices=slices, animations=animations, render=render, output_dir=output_dir)


def clamp(v: float, lo: float, hi: float) -> float:
    return max(lo, min(hi, v))


def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def smoothstep(t: float) -> float:
    t = clamp(t, 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def pingpong(t: float) -> float:
    t = t % 1.0
    return 2.0 * t if t <= 0.5 else 2.0 * (1.0 - t)


def rotate_vec(x: float, y: float, degrees: float) -> Point:
    a = math.radians(degrees)
    ca = math.cos(a)
    sa = math.sin(a)
    return (x * ca - y * sa, x * sa + y * ca)


def parse_bg(value: str) -> RGBA:
    if str(value).lower() == "transparent":
        return (0, 0, 0, 0)
    r, g, b = ImageColor.getrgb(value)
    return (r, g, b, 255)


def font(size: int = 12):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size=size)
        except OSError:
            pass
    return ImageFont.load_default()


def connected_components_bbox(mask: np.ndarray) -> List[Tuple[int, int, int, int, int]]:
    """Return connected-component boxes as (area, x1, y1, x2, y2)."""
    h, w = mask.shape
    seen = np.zeros_like(mask, dtype=bool)
    boxes: List[Tuple[int, int, int, int, int]] = []
    ys, xs = np.nonzero(mask)
    for sx, sy in zip(xs.tolist(), ys.tolist()):
        if seen[sy, sx] or not mask[sy, sx]:
            continue
        stack = [(sx, sy)]
        seen[sy, sx] = True
        area = 0
        x1 = x2 = sx
        y1 = y2 = sy
        while stack:
            x, y = stack.pop()
            area += 1
            if x < x1:
                x1 = x
            if x > x2:
                x2 = x
            if y < y1:
                y1 = y
            if y > y2:
                y2 = y
            for nx, ny in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                if 0 <= nx < w and 0 <= ny < h and mask[ny, nx] and not seen[ny, nx]:
                    seen[ny, nx] = True
                    stack.append((nx, ny))
        boxes.append((area, x1, y1, x2 + 1, y2 + 1))
    boxes.sort(reverse=True)
    return boxes


def find_dark_visor_bbox(head: Image.Image) -> Optional[Tuple[int, int, int, int]]:
    """Find the actual black visor plate in a head sprite.

    The component metadata has only rough sockets.  For expression overlays we
    instead detect the dark visor on the selected head image and fit expression
    strokes inside that detected plate.  This prevents floating/decal-like face
    plates when the head is tilted, squashed, or rotated.
    """
    arr = np.asarray(head.convert("RGBA"))
    rgb = arr[..., :3].astype(np.int16)
    alpha = arr[..., 3] > 64
    # The visor is the large dark rounded rectangle.  Keep the threshold loose
    # enough to include antialiased dark pixels but reject purple antenna parts.
    dark = alpha & (rgb[..., 0] < 45) & (rgb[..., 1] < 55) & (rgb[..., 2] < 70)
    comps = connected_components_bbox(dark)
    h, w = dark.shape
    candidates = []
    for area, x1, y1, x2, y2 in comps:
        bw = x2 - x1
        bh = y2 - y1
        if area < 100:
            continue
        if bw < w * 0.20 or bh < h * 0.10:
            continue
        aspect = bw / max(1, bh)
        if aspect < 1.05:
            continue
        cx = (x1 + x2) / 2.0
        cy = (y1 + y2) / 2.0
        # Prefer central/right dark plates over outlines or antenna shadows.
        score = area + 200.0 * (cx / max(1, w)) - 100.0 * abs((cy / max(1, h)) - 0.45)
        candidates.append((score, x1, y1, x2, y2))
    if not candidates:
        return None
    _, x1, y1, x2, y2 = max(candidates)
    # Inset slightly so repainting does not cover the thick white shell outline.
    pad_x = max(1, int(round((x2 - x1) * 0.03)))
    pad_y = max(1, int(round((y2 - y1) * 0.04)))
    return (x1 + pad_x, y1 + pad_y, x2 - pad_x, y2 - pad_y)


def cyan_bbox(img: Image.Image) -> Optional[Tuple[int, int, int, int]]:
    arr = np.asarray(img.convert("RGBA"))
    r = arr[..., 0].astype(np.int16)
    g = arr[..., 1].astype(np.int16)
    b = arr[..., 2].astype(np.int16)
    a = arr[..., 3] > 32
    cyan = a & (g > 90) & (b > 100) & (b > r + 35) & (g > r + 25)
    ys, xs = np.nonzero(cyan)
    if len(xs) == 0:
        return None
    return (int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1)


def cyan_only(img: Image.Image) -> Image.Image:
    arr = np.array(img.convert("RGBA"))
    r = arr[..., 0].astype(np.int16)
    g = arr[..., 1].astype(np.int16)
    b = arr[..., 2].astype(np.int16)
    a = arr[..., 3]
    cyan = (a > 32) & (g > 90) & (b > 100) & (b > r + 35) & (g > r + 25)
    out = np.zeros_like(arr)
    out[..., :3] = arr[..., :3]
    out[..., 3] = np.where(cyan, a, 0).astype(np.uint8)
    return Image.fromarray(out, "RGBA")


def compose_head_expression(head: Image.Image, expr: Optional[Image.Image], expr_name: str = "") -> Image.Image:
    """Bake a face expression into the selected head sprite.

    The output has the same size/anchors as the original head, so rotations and
    pivots remain correct.  Only cyan expression strokes are transferred; the
    expression sprite's black plate is discarded and the detected visor on the
    head is repainted in place.
    """
    if expr is None or expr_name in {"", "face_eyes_open"}:
        return head
    bbox = find_dark_visor_bbox(head)
    cb = cyan_bbox(expr)
    if bbox is None or cb is None:
        return head
    x1, y1, x2, y2 = bbox
    vw = max(1, x2 - x1)
    vh = max(1, y2 - y1)
    out = head.copy().convert("RGBA")
    draw = ImageDraw.Draw(out)
    radius = max(4, int(round(vh * 0.24)))
    # Repaint the visor in-place to remove native eyes before adding the new
    # expression.  Keep it slightly inside the detected plate to preserve the
    # original outline/anti-aliasing.
    draw.rounded_rectangle([x1, y1, x2 - 1, y2 - 1], radius=radius, fill=(8, 12, 16, 255))
    strokes = cyan_only(expr).crop(cb)
    if expr_name == "face_teleport_scan":
        fit_w, fit_h = int(vw * 0.92), int(vh * 0.82)
    elif expr_name in {"face_dead_x", "face_angry", "face_happy", "face_sad"}:
        fit_w, fit_h = int(vw * 0.78), int(vh * 0.66)
    else:
        fit_w, fit_h = int(vw * 0.78), int(vh * 0.48)
    if strokes.width <= 0 or strokes.height <= 0:
        return out
    scale = min(fit_w / strokes.width, fit_h / strokes.height)
    sw = max(1, int(round(strokes.width * scale)))
    sh = max(1, int(round(strokes.height * scale)))
    strokes = strokes.resize((sw, sh), Image.Resampling.LANCZOS)
    px = int(round(x1 + vw / 2 - sw / 2))
    py = int(round(y1 + vh / 2 - sh / 2))
    out.alpha_composite(strokes, (px, py))
    return out


class ComponentAtlas:
    """Load refined metadata and transparent component slices."""

    def __init__(self, metadata_path: str | Path, slice_dir: str | Path):
        self.metadata_path = Path(metadata_path)
        self.slice_dir = Path(slice_dir)
        self.meta = yaml.safe_load(self.metadata_path.read_text())
        self.sprites: Dict[str, Dict[str, Any]] = self.meta.get("sprites", {})
        self._images: Dict[str, Image.Image] = {}
        missing = []
        for name in self.sprites:
            p = self.slice_dir / f"{name}.png"
            if not p.exists():
                missing.append(str(p))
        if missing:
            preview = "\n".join(missing[:8])
            extra = "" if len(missing) <= 8 else f"\n... {len(missing) - 8} more"
            raise FileNotFoundError(f"missing extracted slice pngs:\n{preview}{extra}")

    @staticmethod
    def _split_variant(name: str) -> Tuple[str, Tuple[str, ...]]:
        """Return (base_name, variant_flags) for virtual sprite variants.

        The extracted atlas only stores one physical PNG per component, but the
        rig frequently needs left/right hand variants.  Virtual flags keep this
        in metadata/pose data instead of duplicating files on disk.  Currently
        supported suffix: ``@flip_x``.
        """
        parts = str(name).split("@")
        return parts[0], tuple(p for p in parts[1:] if p)

    def image(self, name: str) -> Image.Image:
        base, flags = self._split_variant(name)
        key = "@".join((base, *flags)) if flags else base
        if key not in self._images:
            if base not in self.sprites:
                raise KeyError(f"unknown sprite {name!r}; available={sorted(self.sprites)}")
            img = Image.open(self.slice_dir / f"{base}.png").convert("RGBA")
            if "flip_x" in flags:
                img = img.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
            self._images[key] = img
        return self._images[key]

    def info(self, name: str) -> Dict[str, Any]:
        base, flags = self._split_variant(name)
        try:
            base_info = self.sprites[base]
        except KeyError as ex:
            raise KeyError(f"unknown sprite {name!r}; available={sorted(self.sprites)}") from ex
        if "flip_x" not in flags:
            return base_info
        # Return a shallow transformed copy with horizontal anchors mirrored.
        # Coordinates are point coordinates in image-local space, so x mirrors
        # around the image width.
        w = self.image(name).width
        info = dict(base_info)
        if "pivot" in base_info:
            px, py = base_info["pivot"]
            info["pivot"] = [w - float(px), float(py)]
        anchors = {}
        for k, pt in (base_info.get("anchors") or {}).items():
            anchors[k] = [w - float(pt[0]), float(pt[1])]
        if anchors:
            info["anchors"] = anchors
        return info

    def anchor(self, name: str, anchor: str | None = None) -> Point:
        info = self.info(name)
        if anchor:
            anchors = info.get("anchors") or {}
            if anchor in anchors:
                x, y = anchors[anchor]
                return float(x), float(y)
        x, y = info.get("pivot", [self.image(name).width / 2, self.image(name).height / 2])
        return float(x), float(y)


def alpha_multiply(img: Image.Image, opacity: float) -> Image.Image:
    opacity = clamp(opacity, 0.0, 1.0)
    if opacity >= 0.999:
        return img
    img = img.copy()
    a = img.getchannel("A").point(lambda v: int(v * opacity))
    img.putalpha(a)
    return img


def solidify_alpha(img: Image.Image, color: RGBA) -> Image.Image:
    """Return a solid-color silhouette using the input alpha channel."""
    rgba = img.convert("RGBA")
    out = Image.new("RGBA", rgba.size, color)
    out.putalpha(rgba.getchannel("A"))
    return out


def draw_anchor_marker(draw: ImageDraw.ImageDraw, point: Point, color: RGBA, label: str | None = None) -> None:
    """Draw a compact anchor marker for debug views.

    Debug sheets intentionally avoid per-component text labels; labels add
    visual noise and make it harder to reason about exact placement.
    """
    x, y = point
    r = 3
    draw.ellipse((x - r, y - r, x + r, y + r), fill=color, outline=(255, 255, 255, 255), width=1)
    draw.line((x - 5, y, x + 5, y), fill=(255, 255, 255, 220), width=1)
    draw.line((x, y - 5, x, y + 5), fill=(255, 255, 255, 220), width=1)


DEBUG_COLORS: Dict[str, RGBA] = {
    "torso": (240, 220, 40, 210),
    "head": (70, 170, 255, 220),
    "back_arm": (60, 220, 110, 220),
    "front_arm": (255, 110, 60, 220),
    "back_hand": (20, 160, 70, 235),
    "front_hand": (255, 60, 60, 235),
    "back_leg": (170, 80, 255, 220),
    "front_leg": (255, 150, 40, 220),
    "fx": (50, 240, 255, 170),
}


def paste_transformed(
    base: Image.Image,
    part: Image.Image,
    target: Point,
    local_anchor: Point,
    scale: float = 1.0,
    angle: float = 0.0,
    opacity: float = 1.0,
) -> None:
    """Paste a part so local_anchor maps to target, with scale and rotation.

    Rotation is around the local anchor.  A large temporary canvas keeps the math
    simple and stable; the returned frame remains fixed-size.
    """
    if part.mode != "RGBA":
        part = part.convert("RGBA")
    if scale != 1.0:
        w = max(1, int(round(part.width * scale)))
        h = max(1, int(round(part.height * scale)))
        part = part.resize((w, h), Image.Resampling.LANCZOS)
        local_anchor = (local_anchor[0] * scale, local_anchor[1] * scale)
    part = alpha_multiply(part, opacity)
    diag = int(math.ceil(math.hypot(part.width, part.height)))
    size = max(16, diag + 32)
    # Ensure the anchor has enough room in every direction after rotation.
    size = max(size, int(max(local_anchor[0], local_anchor[1], part.width - local_anchor[0], part.height - local_anchor[1]) * 2 + 48))
    layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    center = (size / 2.0, size / 2.0)
    paste_xy = (int(round(center[0] - local_anchor[0])), int(round(center[1] - local_anchor[1])))
    layer.alpha_composite(part, paste_xy)
    if abs(angle) > 0.001:
        layer = layer.rotate(angle, resample=Image.Resampling.BICUBIC, center=center)
    base_xy = (int(round(target[0] - center[0])), int(round(target[1] - center[1])))
    base.alpha_composite(layer, base_xy)


def transformed_point(local_point: Point, local_anchor: Point, target: Point, scale: float, angle: float) -> Point:
    dx = (local_point[0] - local_anchor[0]) * scale
    dy = (local_point[1] - local_anchor[1]) * scale
    rx, ry = rotate_vec(dx, dy, angle)
    return (target[0] + rx, target[1] + ry)


def midpoint(a: Point, b: Point) -> Point:
    return ((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0)


@dataclass
class RobotPose:
    scale: float
    # Per-part scale multipliers keep the component rig cute/stubby.  The
    # extracted source parts are not all drawn at the same semantic scale, so
    # applying the global scale uniformly makes hands too large and limbs too
    # tall.  These multipliers are deliberately exposed in pose data so future
    # YAML-driven poses can tune them without changing the compositor.
    torso_scale: float = 0.94
    head_scale: float = 1.32
    face_scale: float = 1.0
    arm_scale: float = 0.62
    hand_scale: float = 0.56
    leg_scale: float = 0.58
    root_offset: Point = (0.0, 0.0)
    torso_sprite: str = "torso_front"
    torso_angle: float = 0.0
    torso_offset: Point = (0.0, 0.0)
    head_sprite: str = "head_front"
    head_angle: float = 0.0
    head_offset: Point = (0.0, 0.0)
    # Fraction of torso rotation inherited by the head.  Run/dash need a
    # connected neck mount, but inheriting 100% of the torso angle makes the
    # robot look like a rigid plank.  Keep this explicit per pose.
    head_inherit_torso: float = 0.0
    face_sprite: str = "face_eyes_open"
    front_arm_sprite: str = "arm_capsule_vertical"
    back_arm_sprite: str = "arm_capsule_vertical"
    front_hand_sprite: str = "hand_open_down"
    back_hand_sprite: str = "hand_fist"
    front_arm_angle: float = 0.0
    back_arm_angle: float = 0.0
    front_hand_angle: float = 0.0
    back_hand_angle: float = 0.0
    hand_follow: float = 0.78
    # Local torso-space offsets applied to shoulder sockets before mounting arms.
    # This seats arms under the purple shoulder pods instead of pinning them to
    # the exact pod center, and it benefits every animation that uses the rig.
    front_shoulder_offset: Point = (12.0, 36.0)
    back_shoulder_offset: Point = (-12.0, 36.0)
    # Local torso-space hip offsets applied before mounting legs.  The source
    # torso hip anchors are close together; spreading them slightly keeps the
    # run/walk legs from collapsing into a colored knot under the body and
    # benefits all generated animations.
    front_hip_offset: Point = (18.0, 4.0)
    back_hip_offset: Point = (-18.0, 4.0)
    # Optional endpoint-solved arm targets in final frame pixels, relative to
    # the corrected shoulder target.  When provided, the compositor computes
    # the arm scale/angle so the source arm's shoulder and wrist anchors map
    # exactly to the shoulder and wrist targets.  This is preferred over
    # eyeballed arm angles for locomotion.
    front_wrist_delta: Point | None = None
    back_wrist_delta: Point | None = None
    # Local hand mount correction in final frame pixels after wrist solving.
    front_hand_offset: Point = (0.0, 0.0)
    back_hand_offset: Point = (0.0, 0.0)
    front_leg_sprite: str = "leg_straight_right"
    back_leg_sprite: str = "leg_straight_left"
    front_leg_angle: float = 0.0
    back_leg_angle: float = 0.0
    opacity: float = 1.0
    fx_behind: List[Dict[str, Any]] = field(default_factory=list)
    fx_front: List[Dict[str, Any]] = field(default_factory=list)


def animation_pose(name: str, idx: int, frames: int, base_scale: float) -> RobotPose:
    """Return the starter procedural pose for one frame.

    These are intentionally conservative and rig-friendly.  They prioritize:
    stable roots, small hands, grounded feet, distinct hit/death/teleport
    semantics, and visual separation between run and dash.
    """
    t = 0.0 if frames <= 1 else idx / float(frames - 1)
    cycle = idx / float(max(1, frames))
    wave = math.sin(cycle * math.tau)
    cwave = math.cos(cycle * math.tau)
    alt = 1.0 if idx % 2 == 0 else -1.0
    p = RobotPose(scale=base_scale)

    # Pleasant default side-scroller stance.  Hands are deliberately small and
    # relaxed unless an action specifically calls for a fist/blaster pose.
    p.front_hand_sprite = "hand_open_down"
    p.back_hand_sprite = "hand_fist"
    p.front_arm_angle = -20
    p.back_arm_angle = 20
    p.front_leg_angle = 2
    p.back_leg_angle = -2

    if name == "idle":
        bob = math.sin(cycle * math.tau)
        p.root_offset = (0, -1.2 * abs(bob))
        p.torso_angle = bob * 1.4
        p.head_angle = -bob * 1.8
        p.front_arm_angle = -16 + bob * 3
        p.back_arm_angle = 12 - bob * 3
        p.front_leg_angle = 1.5 * bob
        p.back_leg_angle = -1.5 * bob
        p.face_sprite = "face_blink" if idx == 4 else "face_eyes_open"

    elif name == "walk":
        p.root_offset = (0, -2.0 * abs(wave))
        p.torso_angle = -wave * 2.5
        p.head_angle = wave * 1.4
        p.front_arm_angle = -10 - 26 * wave
        p.back_arm_angle = 10 + 26 * wave
        p.front_leg_angle = 32 * wave
        p.back_leg_angle = -32 * wave
        p.front_leg_sprite = "leg_bent_right" if wave < -0.15 else "leg_straight_right"
        p.back_leg_sprite = "leg_bent_left" if wave > 0.15 else "leg_straight_left"
        p.front_hand_sprite = "hand_open_down"
        p.hand_scale = 0.48

    elif name == "run":
        # Run is rhythmic locomotion.  It gets only a modest heel streak so it
        # remains visually distinct from dash.
        p.root_offset = (3, -5.5 * abs(wave))
        p.torso_sprite = "torso_lean_forward"
        p.torso_angle = -14 - wave * 3
        # Seat the head into the forward-lean torso instead of letting the
        # bottom of the head float above the neck socket.  The head keeps a
        # little independent counter-motion but inherits enough torso rotation
        # to look attached during the stride.
        p.head_offset = (16, 12)
        p.head_angle = -0.5 + wave * 0.4
        p.head_inherit_torso = 0.12
        # Keep the run pose readable by preserving left/right separation.
        # Negative rotation pushes the right/front limb outward; positive
        # rotation pushes the left/back limb outward.  Earlier versions used
        # opposite leg signs, causing the legs to cross through the torso and
        # turn the run into a component pile-up.
        # Keep side limbs outside their body half.  Big expressive run poses
        # looked better mathematically but collapsed visually at this small
        # sprite size.  This compact swing is easier to debug and prevents
        # arms, hands, and legs from crossing through the torso center.
        p.front_arm_angle = -8 - 4 * wave
        p.back_arm_angle = 8 + 4 * wave
        p.front_leg_angle = -16 * wave
        p.back_leg_angle = 16 * wave
        p.front_leg_sprite = "leg_bent_right" if wave > 0.30 else "leg_straight_right"
        p.back_leg_sprite = "leg_bent_left" if wave < -0.30 else "leg_straight_left"
        p.front_hand_sprite = "hand_fist@flip_x"
        p.back_hand_sprite = "hand_fist"
        p.arm_scale = 0.45
        p.hand_scale = 0.62
        p.hand_follow = 0.75
        # v23: the run torso now has semantic side-specific sockets.  Avoid
        # global compensating offsets; let the anchors themselves define where
        # the shoulder/hip chains attach.
        p.front_hip_offset = (0.0, 0.0)
        p.back_hip_offset = (0.0, 0.0)
        p.front_shoulder_offset = (0.0, 0.0)
        p.back_shoulder_offset = (0.0, 0.0)
        # Endpoint targets are final-frame pixels relative to the corrected
        # shoulder.  Keep the run silhouette simple at this tiny sprite size:
        # the near/right arm stays outside the body, while the far/left arm is
        # short and partly hidden behind the torso.
        p.front_wrist_delta = (14.0 + 2.0 * wave, 15.0 - 1.5 * abs(wave))
        p.back_wrist_delta = (-10.0 - 1.5 * wave, 13.0 - 1.0 * abs(wave))
        p.front_hand_offset = (0.0, 0.0)
        p.back_hand_offset = (0.0, 0.0)
        if idx % 2 == 1:
            p.fx_behind.append({"sprite": "fx_dash_streak", "target_offset": (-31, -20), "scale": 0.13, "opacity": 0.28})

    elif name == "jump":
        # Sheet-locked jump poses: the in-game bounding box should move along
        # the jump arc, not the sprite pixels inside the cell.  Keep the root
        # stable in every frame and communicate crouch/launch/apex/land only
        # through pose changes.
        p.root_offset = (0, 0)
        if idx == 0:
            # Anticipation crouch.  The forward-lean torso needs the same
            # mounted/down-forward head seating as run/dash, otherwise the
            # large cute head floats behind the body.
            p.torso_sprite = "torso_lean_forward"
            p.torso_angle = -10
            p.head_offset = (14, 11)
            p.head_angle = 1.0
            p.head_inherit_torso = 0.08
            p.front_leg_sprite = "leg_bent_right"
            p.back_leg_sprite = "leg_bent_left"
            p.front_leg_angle = 30
            p.back_leg_angle = -30
            p.front_arm_angle = -32
            p.back_arm_angle = 28
        elif idx == 1:
            # Launch: still compressed but starting to open up.
            p.torso_sprite = "torso_lean_forward"
            p.torso_angle = -6
            p.head_offset = (12, 10)
            p.head_angle = 0.5
            p.head_inherit_torso = 0.06
            p.front_leg_sprite = "leg_bent_right"
            p.back_leg_sprite = "leg_bent_left"
            p.front_leg_angle = 12
            p.back_leg_angle = -18
            p.front_arm_angle = -36
            p.back_arm_angle = 30
        elif idx in {2, 3}:
            # Airborne tuck.  Do not raise the root in the sheet.
            p.torso_sprite = "torso_front"
            p.torso_angle = -1 if idx == 2 else 1
            p.head_offset = (1, 7)
            p.head_angle = 0.5 if idx == 2 else -0.5
            p.head_inherit_torso = 0.04
            p.front_leg_sprite = "leg_bent_right"
            p.back_leg_sprite = "leg_bent_left"
            p.front_leg_angle = -10 if idx == 2 else 8
            p.back_leg_angle = 12 if idx == 2 else -10
            p.front_arm_angle = -42 if idx == 2 else 30
            p.back_arm_angle = 34 if idx == 2 else -30
        elif idx == frames - 1:
            # Landing squash/recovery.
            p.torso_sprite = "torso_front"
            p.torso_angle = 2
            p.head_offset = (1, 8)
            p.head_angle = -0.5
            p.head_inherit_torso = 0.03
            p.front_leg_sprite = "leg_bent_right"
            p.back_leg_sprite = "leg_bent_left"
            p.front_leg_angle = 20
            p.back_leg_angle = -20
            p.front_arm_angle = -14
            p.back_arm_angle = 14
        else:
            # Descending preparation.
            p.torso_sprite = "torso_front"
            p.torso_angle = 3
            p.head_offset = (1, 7)
            p.head_angle = -1.0
            p.head_inherit_torso = 0.04
            p.front_leg_sprite = "leg_bent_right"
            p.back_leg_sprite = "leg_bent_left"
            p.front_leg_angle = 16
            p.back_leg_angle = -14
            p.front_arm_angle = 20
            p.back_arm_angle = -22

    elif name == "fall":
        p.root_offset = (0, lerp(-20, 6, t))
        p.torso_sprite = "torso_lean_back"
        p.torso_angle = lerp(2, 16, t)
        p.head_angle = lerp(-6, 9, t)
        p.front_leg_sprite = "leg_bent_right"
        p.back_leg_sprite = "leg_bent_left"
        p.front_leg_angle = lerp(-12, 24, t)
        p.back_leg_angle = lerp(14, -18, t)
        p.front_arm_angle = lerp(32, -8, t)
        p.back_arm_angle = lerp(-30, 18, t)
        p.front_hand_sprite = "hand_open_down"

    elif name == "slash":
        a = smoothstep(t)
        swing = math.sin(math.pi * clamp(t, 0, 1))
        p.torso_sprite = "torso_lean_forward"
        p.torso_angle = lerp(-8, 8, a)
        p.head_angle = lerp(1, -5, swing)
        p.front_arm_sprite = "arm_capsule_vertical"
        p.front_hand_sprite = "hand_fist@flip_x"
        p.front_arm_angle = lerp(-56, 50, a)
        p.front_hand_angle = lerp(-40, 26, a)
        p.back_arm_angle = 18
        p.front_leg_sprite = "leg_bent_right"
        p.back_leg_sprite = "leg_straight_left"
        p.front_leg_angle = 12
        p.back_leg_angle = -6
        p.hand_scale = 0.50
        p.hand_follow = 0.88
        if 1 <= idx <= frames - 2:
            p.fx_front.append({
                "sprite": "fx_slash_arc",
                "target_offset": (22, -54),
                "scale": 0.17 + 0.04 * swing,
                "angle": lerp(-14, 14, a),
                "opacity": 0.45 + 0.45 * swing,
            })

    elif name == "hit":
        # Hit is only recoil/stagger; it intentionally does not become death.
        recoil = math.sin(math.pi * t)
        p.torso_sprite = "torso_lean_back"
        p.root_offset = (-7 * recoil, -2 * recoil)
        p.torso_angle = 15 * recoil
        p.head_angle = 20 * recoil
        p.face_sprite = "face_sad" if idx < frames - 1 else "face_eyes_open"
        p.front_arm_angle = 28 * recoil - 10
        p.back_arm_angle = -26 * recoil + 12
        p.front_hand_sprite = "hand_open_down"
        p.front_leg_sprite = "leg_bent_right" if recoil > 0.30 else "leg_straight_right"
        p.back_leg_sprite = "leg_bent_left" if recoil > 0.30 else "leg_straight_left"
        p.front_leg_angle = -10 * recoil
        p.back_leg_angle = 14 * recoil
        if idx <= 2:
            p.fx_front.append({"sprite": "fx_hit_particles", "target_offset": (24, -66), "scale": 0.15, "opacity": 0.70 * max(0.0, 1.0 - t)})

    elif name == "death":
        # Stumble with normal/sad face, then collapse with dead face.  The root
        # drifts down but the final frame stays within the same cell.
        d = smoothstep(t)
        p.hand_scale = 0.43
        p.front_hand_sprite = "hand_open_down"
        p.face_sprite = "face_sad" if idx < 3 else "face_dead_x"
        p.head_sprite = "head_tilt_right" if idx >= 2 else "head_front"
        p.torso_sprite = "torso_prone" if d > 0.62 else "torso_lean_back"
        p.root_offset = (lerp(0, 12, d), lerp(0, 12, d))
        p.torso_angle = lerp(0, 78, d)
        p.head_angle = lerp(0, 86, d)
        p.front_arm_angle = lerp(-10, 62, d)
        p.back_arm_angle = lerp(8, 102, d)
        p.front_leg_sprite = "leg_bent_right"
        p.back_leg_sprite = "leg_bent_left"
        p.front_leg_angle = lerp(5, 72, d)
        p.back_leg_angle = lerp(-5, 45, d)
        if idx >= frames - 2:
            p.opacity = 0.95

    elif name == "teleport":
        # Ambition precision-blink/teleport.  Unlike hit, this has a vanish
        # hold and reappear phase using alpha, scan face, and particle bloom.
        p.face_sprite = "face_teleport_scan"
        p.front_hand_sprite = "hand_open_down"
        if idx <= 1:
            charge = idx / 1.0
            p.root_offset = (0, -1.5 * charge)
            p.fx_front.append({"sprite": "fx_hit_particles", "target_offset": (2, -60), "scale": 0.13 + 0.03 * charge, "opacity": 0.40 + 0.25 * charge})
        elif idx == 2:
            p.opacity = 0.55
            p.head_sprite = "head_squash_blink"
            p.fx_front.append({"sprite": "fx_hit_particles", "target_offset": (0, -56), "scale": 0.23, "opacity": 0.85})
        elif idx in {3, 4}:
            p.opacity = 0.0
            p.fx_front.append({"sprite": "fx_hit_particles", "target_offset": (0, -54), "scale": 0.24 if idx == 3 else 0.18, "opacity": 0.90 if idx == 3 else 0.70})
        elif idx == 5:
            p.opacity = 0.45
            p.root_offset = (0, -1)
            p.head_sprite = "head_squash_blink"
            p.fx_front.append({"sprite": "fx_hit_particles", "target_offset": (0, -56), "scale": 0.21, "opacity": 0.70})
        else:
            p = animation_pose("idle", idx, frames, base_scale)
            p.face_sprite = "face_eyes_open"

    elif name == "dash":
        # Dash is the high-speed burst: low silhouette and a strong back streak.
        p.root_offset = (9, 2)
        p.torso_sprite = "torso_lean_forward"
        p.torso_angle = -24
        # Dash uses the same forward-lean torso as run, but the head needs to
        # be pulled down and forward so it stays planted on the neck while the
        # body compresses into the burst.
        p.head_offset = (18, 16)
        p.head_angle = -0.5
        p.head_inherit_torso = 0.10
        p.front_leg_sprite = "leg_bent_right"
        p.back_leg_sprite = "leg_bent_left"
        p.front_leg_angle = 54
        p.back_leg_angle = -34
        p.front_arm_angle = -58
        p.back_arm_angle = 34
        p.front_hand_sprite = "hand_fist@flip_x"
        p.hand_scale = 0.50
        p.hand_follow = 0.88
        p.fx_behind.append({"sprite": "fx_dash_streak", "target_offset": (-38, -22), "scale": 0.22, "opacity": 0.70})

    else:
        raise KeyError(f"unknown animation {name!r}; available={sorted(ANIMATIONS)}")

    return p


class RobotAssembler:
    def __init__(self, atlas: ComponentAtlas, render: RenderConfig):
        self.atlas = atlas
        self.render = render
        self._head_face_cache: Dict[Tuple[str, str], Image.Image] = {}

    def _component_anchor_worlds(self, sprite: str, local_anchor: Point, target: Point, scale: float, angle: float) -> Dict[str, Point]:
        info = self.atlas.info(sprite)
        worlds = {}
        for name, pt in (info.get("anchors") or {}).items():
            worlds[name] = transformed_point((float(pt[0]), float(pt[1])), local_anchor, target, scale, angle)
        worlds["pivot"] = transformed_point(tuple(map(float, info.get("pivot", local_anchor))), local_anchor, target, scale, angle)
        return worlds

    def _paste_sprite(
        self,
        frame: Image.Image,
        sprite: str,
        target: Point,
        anchor: str | None,
        scale: float,
        angle: float = 0.0,
        opacity: float = 1.0,
        debug_color: RGBA | None = None,
        debug_label: str | None = None,
    ) -> Dict[str, Point]:
        img = self.atlas.image(sprite)
        if debug_color is not None:
            img = solidify_alpha(img, debug_color)
        local_anchor = self.atlas.anchor(sprite, anchor)
        paste_transformed(frame, img, target, local_anchor, scale, angle, opacity)
        worlds = self._component_anchor_worlds(sprite, local_anchor, target, scale, angle)
        if debug_color is not None:
            draw = ImageDraw.Draw(frame)
            draw_anchor_marker(draw, target, debug_color, debug_label)
        return worlds

    def _head_image(self, head_sprite: str, face_sprite: str | None) -> Image.Image:
        face_sprite = face_sprite or "face_eyes_open"
        key = (head_sprite, face_sprite)
        if key not in self._head_face_cache:
            head = self.atlas.image(head_sprite)
            expr = None if face_sprite == "face_eyes_open" else self.atlas.image(face_sprite)
            self._head_face_cache[key] = compose_head_expression(head, expr, face_sprite)
        return self._head_face_cache[key]

    @staticmethod
    def _offset_from_torso(point: Point, offset: Point, torso_scale: float, torso_angle: float) -> Point:
        ox, oy = rotate_vec(offset[0] * torso_scale, offset[1] * torso_scale, torso_angle)
        return (point[0] + ox, point[1] + oy)

    @staticmethod
    def _offset_from_part(point: Point, offset: Point, part_scale: float, part_angle: float) -> Point:
        ox, oy = rotate_vec(offset[0] * part_scale, offset[1] * part_scale, part_angle)
        return (point[0] + ox, point[1] + oy)

    def _solve_arm_endpoint(
        self,
        sprite: str,
        shoulder_target: Point,
        wrist_delta: Point,
        torso_angle: float,
        default_scale: float,
        default_angle: float,
    ) -> Tuple[float, float, Point]:
        """Return (scale, angle, wrist_target) with shoulder/wrist anchors tied.

        ``wrist_delta`` is specified in final-frame pixels in the local torso
        orientation.  The source arm vector from shoulder->wrist is measured
        from component-local anchors.  The returned scale and angle make the
        arm's source wrist land exactly on the desired world wrist target.
        """
        shoulder_local = self.atlas.anchor(sprite, "shoulder")
        wrist_local = self.atlas.anchor(sprite, "wrist")
        src = (wrist_local[0] - shoulder_local[0], wrist_local[1] - shoulder_local[1])
        src_len = max(1e-6, math.hypot(src[0], src[1]))
        dx, dy = rotate_vec(float(wrist_delta[0]), float(wrist_delta[1]), torso_angle)
        wrist_target = (shoulder_target[0] + dx, shoulder_target[1] + dy)
        dst_len = max(1e-6, math.hypot(dx, dy))
        src_angle = math.degrees(math.atan2(src[1], src[0]))
        dst_angle = math.degrees(math.atan2(dy, dx))
        scale = clamp(dst_len / src_len, default_scale * 0.55, default_scale * 1.80)
        angle = dst_angle - src_angle
        if not math.isfinite(scale) or not math.isfinite(angle):
            return default_scale, default_angle, shoulder_target
        return scale, angle, wrist_target

    def render_frame(self, animation: str, frame_index: int, debug_parts: bool = False) -> Tuple[Image.Image, Dict[str, Any]]:
        if animation not in ANIMATIONS:
            raise KeyError(f"unsupported animation {animation!r}; available={sorted(ANIMATIONS)}")
        info = ANIMATIONS[animation]
        idx = frame_index % info["frames"]
        pose = animation_pose(animation, idx, info["frames"], self.render.scale)
        w, h = self.render.frame_width, self.render.frame_height
        frame = Image.new("RGBA", (w, h), parse_bg(self.render.frame_background))
        root = (w / 2.0 + self.render.root_x + pose.root_offset[0], h + self.render.root_y + pose.root_offset[1])
        S = pose.scale
        torsoS = S * pose.torso_scale
        headS = S * pose.head_scale
        faceS = S * pose.face_scale
        armS = S * pose.arm_scale
        handS = S * pose.hand_scale
        legS = S * pose.leg_scale

        for fx in pose.fx_behind:
            self._draw_fx(frame, fx, root, S, debug_parts=debug_parts)

        # Use the selected leg assets to decide where the hips should sit above
        # the root/ground.  This locks every frame to one root baseline and
        # prevents center-based jitter.
        front_hip = self.atlas.anchor(pose.front_leg_sprite, "hip")
        front_ground = self.atlas.anchor(pose.front_leg_sprite, "ground")
        back_hip = self.atlas.anchor(pose.back_leg_sprite, "hip")
        back_ground = self.atlas.anchor(pose.back_leg_sprite, "ground")
        hip_to_ground = max(front_ground[1] - front_hip[1], back_ground[1] - back_hip[1]) * legS
        hip_target = (root[0] + pose.torso_offset[0], root[1] - hip_to_ground + pose.torso_offset[1])

        torso_info = self.atlas.info(pose.torso_sprite)
        hip_l = tuple(map(float, torso_info["anchors"]["hip_left"]))
        hip_r = tuple(map(float, torso_info["anchors"]["hip_right"]))
        torso_hip_anchor = midpoint(hip_l, hip_r)
        torso_anchors = self._component_anchor_worlds(pose.torso_sprite, torso_hip_anchor, hip_target, torsoS, pose.torso_angle)

        # Arm mount points are nudged down/inward from the raw shoulder-pod
        # centers.  This keeps the arms visually socketed under the pod while
        # avoiding the pasted-on look caused by anchoring directly to the pod
        # center.  These corrected targets are used by every animation.
        back_shoulder_target = self._offset_from_torso(torso_anchors["shoulder_left"], pose.back_shoulder_offset, torsoS, pose.torso_angle)
        front_shoulder_target = self._offset_from_torso(torso_anchors["shoulder_right"], pose.front_shoulder_offset, torsoS, pose.torso_angle)

        # Draw order follows the visible side-view robot anatomy:
        # - left/back leg is behind the body
        # - left/back hand is behind the left/back arm
        # - torso sits over the far-side limbs
        # - right/front leg is in front of the body
        # - right/front hand is in front of the right/front arm
        # This keeps the run pose from turning into a pile-up while preserving
        # tied anchor constraints for every limb chain.
        back_hip_target = self._offset_from_torso(torso_anchors["hip_left"], pose.back_hip_offset, torsoS, pose.torso_angle)
        front_hip_target = self._offset_from_torso(torso_anchors["hip_right"], pose.front_hip_offset, torsoS, pose.torso_angle)

        self._paste_sprite(frame, pose.back_leg_sprite, back_hip_target, "hip", legS, pose.back_leg_angle, pose.opacity, DEBUG_COLORS["back_leg"] if debug_parts else None, "back_leg" if debug_parts else None)

        if pose.back_wrist_delta is not None:
            back_arm_scale, back_arm_angle, back_wrist_target = self._solve_arm_endpoint(
                pose.back_arm_sprite, back_shoulder_target, pose.back_wrist_delta, pose.torso_angle, armS, pose.back_arm_angle
            )
        else:
            back_arm_scale, back_arm_angle, back_wrist_target = armS, pose.back_arm_angle, None
        # Compute the solved back wrist before drawing so the hand can be
        # composited behind the arm while still snapping to the same endpoint.
        back_arm_preview = self._component_anchor_worlds(pose.back_arm_sprite, self.atlas.anchor(pose.back_arm_sprite, "shoulder"), back_shoulder_target, back_arm_scale, back_arm_angle)
        if back_wrist_target is None:
            back_wrist_target = back_arm_preview.get("wrist", back_shoulder_target)
        back_wrist = (back_wrist_target[0] + pose.back_hand_offset[0], back_wrist_target[1] + pose.back_hand_offset[1])
        self._paste_sprite(frame, pose.back_hand_sprite, back_wrist, "wrist", handS, pose.back_hand_angle + back_arm_angle * pose.hand_follow, pose.opacity, DEBUG_COLORS["back_hand"] if debug_parts else None, "back_wr" if debug_parts else None)
        self._paste_sprite(frame, pose.back_arm_sprite, back_shoulder_target, "shoulder", back_arm_scale, back_arm_angle, pose.opacity, DEBUG_COLORS["back_arm"] if debug_parts else None, "back_sh" if debug_parts else None)

        # Torso.
        torso_img = self.atlas.image(pose.torso_sprite)
        if debug_parts:
            torso_img = solidify_alpha(torso_img, DEBUG_COLORS["torso"])
        paste_transformed(frame, torso_img, hip_target, torso_hip_anchor, torsoS, pose.torso_angle, pose.opacity)
        if debug_parts:
            draw_anchor_marker(ImageDraw.Draw(frame), hip_target, DEBUG_COLORS["torso"], "torso_hips")

        # The right/front leg should read as the near-side leg, so it draws on
        # top of the body after the torso shell.
        self._paste_sprite(frame, pose.front_leg_sprite, front_hip_target, "hip", legS, pose.front_leg_angle, pose.opacity, DEBUG_COLORS["front_leg"] if debug_parts else None, "front_leg" if debug_parts else None)

        if pose.front_wrist_delta is not None:
            front_arm_scale, front_arm_angle, front_wrist_target = self._solve_arm_endpoint(
                pose.front_arm_sprite, front_shoulder_target, pose.front_wrist_delta, pose.torso_angle, armS, pose.front_arm_angle
            )
        else:
            front_arm_scale, front_arm_angle, front_wrist_target = armS, pose.front_arm_angle, None
        front_arm_anchors = self._paste_sprite(frame, pose.front_arm_sprite, front_shoulder_target, "shoulder", front_arm_scale, front_arm_angle, pose.opacity, DEBUG_COLORS["front_arm"] if debug_parts else None, "front_sh" if debug_parts else None)
        if front_wrist_target is None:
            front_wrist_target = front_arm_anchors.get("wrist", front_shoulder_target)
        front_wrist = (front_wrist_target[0] + pose.front_hand_offset[0], front_wrist_target[1] + pose.front_hand_offset[1])
        self._paste_sprite(frame, pose.front_hand_sprite, front_wrist, "wrist", handS, pose.front_hand_angle + front_arm_angle * pose.hand_follow, pose.opacity, DEBUG_COLORS["front_hand"] if debug_parts else None, "front_wr" if debug_parts else None)

        if debug_parts:
            d = ImageDraw.Draw(frame)
            # Skeleton links: no text, only component colors.  Hollow white dots
            # mark raw torso sockets; filled colored dots mark corrected rig
            # targets after the global offset policy is applied.
            for a, b, c in [
                (back_shoulder_target, back_wrist, DEBUG_COLORS["back_arm"]),
                (front_shoulder_target, front_wrist, DEBUG_COLORS["front_arm"]),
                (back_hip_target, root, DEBUG_COLORS["back_leg"]),
                (front_hip_target, root, DEBUG_COLORS["front_leg"]),
            ]:
                d.line((a[0], a[1], b[0], b[1]), fill=c, width=2)
            for pt in [torso_anchors["shoulder_left"], torso_anchors["shoulder_right"], torso_anchors["hip_left"], torso_anchors["hip_right"], torso_anchors["neck"]]:
                x, y = pt
                d.ellipse((x - 3, y - 3, x + 3, y + 3), outline=(255, 255, 255, 230), width=1)
            for pt, col in [(back_hip_target, DEBUG_COLORS["back_leg"]), (front_hip_target, DEBUG_COLORS["front_leg"]), (root, (255,255,255,240))]:
                draw_anchor_marker(d, pt, col)

        # Head with baked expression.  Expressions are fitted to the detected
        # visor in the selected head sprite, then transformed as one unit; this
        # keeps hurt/dead/teleport/blink overlays locked to tilted heads.
        torso_neck = torso_anchors["neck"]
        head_target = (torso_neck[0] + pose.head_offset[0], torso_neck[1] + pose.head_offset[1])
        head_world_angle = pose.head_angle + pose.torso_angle * pose.head_inherit_torso
        head_img = self._head_image(pose.head_sprite, pose.face_sprite)
        if debug_parts:
            head_img = alpha_multiply(solidify_alpha(head_img, DEBUG_COLORS["head"]), 0.58)
        head_local_anchor = self.atlas.anchor(pose.head_sprite, "neck")
        paste_transformed(frame, head_img, head_target, head_local_anchor, headS, head_world_angle, pose.opacity)
        head_anchors = self._component_anchor_worlds(pose.head_sprite, head_local_anchor, head_target, headS, head_world_angle)
        if debug_parts:
            draw_anchor_marker(ImageDraw.Draw(frame), head_target, DEBUG_COLORS["head"], "head_neck")

        for fx in pose.fx_front:
            self._draw_fx(frame, fx, root, S, debug_parts=debug_parts)

        manifest = {
            "animation": animation,
            "index": idx,
            "duration_ms": info["duration_ms"],
            "root": [round(root[0], 2), round(root[1], 2)],
            "pose": {
                "scale": pose.scale,
                "torso_sprite": pose.torso_sprite,
                "head_sprite": pose.head_sprite,
                "face_sprite": pose.face_sprite,
                "opacity": round(pose.opacity, 3),
                "debug_parts": bool(debug_parts),
                "head_mount": {
                    "torso_neck": [round(torso_neck[0], 2), round(torso_neck[1], 2)],
                    "head_target": [round(head_target[0], 2), round(head_target[1], 2)],
                    "head_offset": [round(pose.head_offset[0], 2), round(pose.head_offset[1], 2)],
                    "head_local_angle": round(pose.head_angle, 2),
                    "head_world_angle": round(head_world_angle, 2),
                    "head_inherit_torso": round(pose.head_inherit_torso, 3),
                },
                "part_scales": {
                    "torso": round(pose.torso_scale, 3),
                    "head": round(pose.head_scale, 3),
                    "face": round(pose.face_scale, 3),
                    "arm": round(pose.arm_scale, 3),
                    "hand": round(pose.hand_scale, 3),
                    "leg": round(pose.leg_scale, 3),
                },
                "arm_mounts": {
                    "back_shoulder_raw": [round(torso_anchors["shoulder_left"][0], 2), round(torso_anchors["shoulder_left"][1], 2)],
                    "front_shoulder_raw": [round(torso_anchors["shoulder_right"][0], 2), round(torso_anchors["shoulder_right"][1], 2)],
                    "back_shoulder_target": [round(back_shoulder_target[0], 2), round(back_shoulder_target[1], 2)],
                    "front_shoulder_target": [round(front_shoulder_target[0], 2), round(front_shoulder_target[1], 2)],
                    "back_wrist": [round(back_wrist[0], 2), round(back_wrist[1], 2)],
                    "front_wrist": [round(front_wrist[0], 2), round(front_wrist[1], 2)],
                    "back_arm_effective_scale": round(back_arm_scale, 4),
                    "front_arm_effective_scale": round(front_arm_scale, 4),
                    "back_arm_effective_angle": round(back_arm_angle, 2),
                    "front_arm_effective_angle": round(front_arm_angle, 2),
                    "back_wrist_delta": None if pose.back_wrist_delta is None else [round(pose.back_wrist_delta[0], 2), round(pose.back_wrist_delta[1], 2)],
                    "front_wrist_delta": None if pose.front_wrist_delta is None else [round(pose.front_wrist_delta[0], 2), round(pose.front_wrist_delta[1], 2)],
                    "hand_follow": round(pose.hand_follow, 3),
                    "z_order": ["back_leg", "back_hand", "back_arm", "torso", "front_leg", "front_arm", "front_hand", "head", "fx_front"],
                },
                "leg_mounts": {
                    "back_hip_raw": [round(torso_anchors["hip_left"][0], 2), round(torso_anchors["hip_left"][1], 2)],
                    "front_hip_raw": [round(torso_anchors["hip_right"][0], 2), round(torso_anchors["hip_right"][1], 2)],
                    "back_hip_target": [round(back_hip_target[0], 2), round(back_hip_target[1], 2)],
                    "front_hip_target": [round(front_hip_target[0], 2), round(front_hip_target[1], 2)],
                    "back_hip_offset": [round(pose.back_hip_offset[0], 2), round(pose.back_hip_offset[1], 2)],
                    "front_hip_offset": [round(pose.front_hip_offset[0], 2), round(pose.front_hip_offset[1], 2)],
                },
            },
        }
        return frame, manifest

    def _draw_fx(self, frame: Image.Image, fx: Mapping[str, Any], root: Point, base_scale: float, debug_parts: bool = False) -> None:
        sprite = str(fx["sprite"])
        offset = fx.get("target_offset", (0.0, 0.0))
        target = (root[0] + float(offset[0]), root[1] + float(offset[1]))
        scale = float(fx.get("scale", base_scale))
        angle = float(fx.get("angle", 0.0))
        opacity = float(fx.get("opacity", 1.0))
        self._paste_sprite(frame, sprite, target, None, scale, angle, opacity, DEBUG_COLORS["fx"] if debug_parts else None, "fx" if debug_parts else None)



def frame_alpha_bbox(frame: Image.Image) -> Optional[Tuple[int, int, int, int]]:
    """Return the non-transparent bounds for a rendered frame."""
    return frame.getchannel("A").getbbox()


def bbox_margins(bbox: Optional[Tuple[int, int, int, int]], width: int, height: int) -> Dict[str, int]:
    if bbox is None:
        return {"left": width, "top": height, "right": width, "bottom": height}
    x1, y1, x2, y2 = bbox
    return {"left": x1, "top": y1, "right": width - x2, "bottom": height - y2}


def bbox_warnings(bbox: Optional[Tuple[int, int, int, int]], width: int, height: int, min_margin: int = 3) -> List[str]:
    if bbox is None:
        return ["empty_frame"]
    margins = bbox_margins(bbox, width, height)
    warnings = []
    for side, value in margins.items():
        if value < min_margin:
            warnings.append(f"near_{side}_edge:{value}px")
    return warnings

def build_spritesheet(job: RigJob, debug_parts: bool = False) -> Tuple[Image.Image, Dict[str, Any]]:
    atlas = ComponentAtlas(job.metadata, job.slices)
    assembler = RobotAssembler(atlas, job.render)
    selected = [a for a in job.animations if a in ANIMATIONS]
    missing = [a for a in job.animations if a not in ANIMATIONS]
    if missing:
        raise KeyError(f"unsupported animations: {missing}; available={sorted(ANIMATIONS)}")
    fw, fh = job.render.frame_width, job.render.frame_height
    label_w = max(0, job.render.label_width)
    border = max(0, job.render.border)
    max_frames = max(ANIMATIONS[a]["frames"] for a in selected)
    sheet_w = label_w + border + max_frames * (fw + border)
    sheet_h = border + len(selected) * (fh + border)
    sheet = Image.new("RGBA", (sheet_w, sheet_h), parse_bg(job.render.sheet_background))
    draw = ImageDraw.Draw(sheet)
    manifest: Dict[str, Any] = {
        "target": "component_robot",
        "metadata": str(job.metadata),
        "slices": str(job.slices),
        "frame_width": fw,
        "frame_height": fh,
        "label_width": label_w,
        "border": border,
        "animations": {},
        "qa_warnings": [],
        "notes": [
            "assembled from refined component sprites, not directly AI-generated as a sheet",
            "hit is recoil/stagger only and intentionally does not transition into death",
            "teleport is the Ambition precision-blink action; face blink is only an expression",
        ],
    }
    for row, animation in enumerate(selected):
        info = ANIMATIONS[animation]
        y = border + row * (fh + border)
        if label_w:
            draw.text((8, y + 8), animation, fill=(255, 255, 255, 255), font=font(12))
            draw.text((8, y + 23), f"{info['frames']}f/{info['duration_ms']}ms", fill=(190, 190, 190, 255), font=font(10))
        frames: List[Dict[str, Any]] = []
        for frame_index in range(info["frames"]):
            x = label_w + border + frame_index * (fw + border)
            frame, frame_meta = assembler.render_frame(animation, frame_index, debug_parts=debug_parts)
            bbox = frame_alpha_bbox(frame)
            margins = bbox_margins(bbox, fw, fh)
            warnings = bbox_warnings(bbox, fw, fh)
            if warnings:
                manifest["qa_warnings"].append({"animation": animation, "frame": frame_index, "warnings": warnings})
            sheet.alpha_composite(frame, (x, y))
            frames.append({
                "index": frame_index,
                "x": x,
                "y": y,
                "w": fw,
                "h": fh,
                "duration_ms": info["duration_ms"],
                "root": frame_meta["root"],
                "bbox": list(bbox) if bbox else None,
                "margins": margins,
                "warnings": warnings,
                "pose": frame_meta["pose"],
            })
        manifest["animations"][animation] = {"duration_ms": info["duration_ms"], "frames": frames}
    return sheet, manifest


def write_spritesheet(job: RigJob, image_out: str | Path, manifest_out: str | Path | None = None, debug_parts: bool = False) -> Tuple[Path, Path]:
    image_out = Path(image_out)
    manifest_out = Path(manifest_out) if manifest_out is not None else image_out.with_suffix(".yaml")
    image_out.parent.mkdir(parents=True, exist_ok=True)
    manifest_out.parent.mkdir(parents=True, exist_ok=True)
    sheet, manifest = build_spritesheet(job, debug_parts=debug_parts)
    sheet.save(image_out)
    manifest_out.write_text(yaml.safe_dump(manifest, sort_keys=False), encoding="utf8")
    return image_out, manifest_out


def write_single(job: RigJob, output: str | Path, animation: str, frame_index: int, debug_parts: bool = False) -> Path:
    atlas = ComponentAtlas(job.metadata, job.slices)
    assembler = RobotAssembler(atlas, job.render)
    image, manifest = assembler.render_frame(animation, frame_index, debug_parts=debug_parts)
    output = Path(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    image.save(output)
    output.with_suffix(".json").write_text(json.dumps(manifest, indent=2), encoding="utf8")
    return output


def write_debug_frame(job: RigJob, output: str | Path, animation: str, frame_index: int, zoom: int = 5, pad: int = 30, background: str = "black") -> Path:
    """Render a large cropped single-frame debug view with tied anchors visible."""
    atlas = ComponentAtlas(job.metadata, job.slices)
    assembler = RobotAssembler(atlas, job.render)
    image, manifest = assembler.render_frame(animation, frame_index, debug_parts=True)
    bbox = frame_alpha_bbox(image)
    if bbox is not None:
        x1, y1, x2, y2 = bbox
        x1 = max(0, x1 - pad)
        y1 = max(0, y1 - pad)
        x2 = min(image.width, x2 + pad)
        y2 = min(image.height, y2 + pad)
        image = image.crop((x1, y1, x2, y2))
        manifest["debug_crop"] = [x1, y1, x2, y2]
    bg = parse_bg(background)
    if bg[3] > 0:
        canvas = Image.new("RGBA", image.size, bg)
        canvas.alpha_composite(image)
        image = canvas
    if zoom and zoom != 1:
        image = image.resize((image.width * zoom, image.height * zoom), Image.Resampling.NEAREST)
        manifest["debug_zoom"] = zoom
    output = Path(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    image.save(output)
    output.with_suffix(".json").write_text(json.dumps(manifest, indent=2), encoding="utf8")
    return output


def draw_default(root: str | Path = ".") -> List[Path]:
    root = Path(root).resolve()
    job_path = root / "examples" / "robot_rig_job.yaml"
    job = RigJob.load(job_path)
    out = job.output_dir / "robot_assembled_spritesheet.png"
    return list(write_spritesheet(job, out))


def build_component_anchor_sheet(job: RigJob, sprites: Sequence[str], cell: int = 160) -> Image.Image:
    """Draw each component in local space with its pivot/anchors.

    No text is drawn: this sheet is meant for visual geometric debugging.
    Marker colors are deterministic by anchor name so the same semantic anchor
    can be compared across components without noisy labels.
    """
    atlas = ComponentAtlas(job.metadata, job.slices)
    cols = max(1, min(6, len(sprites)))
    rows = int(math.ceil(len(sprites) / cols))
    out = Image.new("RGBA", (cols * cell, rows * cell), (0, 0, 0, 255))
    anchor_palette = {
        "pivot": (255, 255, 255, 255),
        "neck": (80, 180, 255, 255),
        "face_socket": (0, 255, 255, 255),
        "antenna_socket": (200, 120, 255, 255),
        "shoulder": (255, 90, 70, 255),
        "shoulder_left": (70, 230, 110, 255),
        "shoulder_right": (255, 90, 70, 255),
        "wrist": (255, 255, 80, 255),
        "elbow": (255, 150, 60, 255),
        "hip": (255, 190, 70, 255),
        "hip_left": (180, 90, 255, 255),
        "hip_right": (255, 150, 40, 255),
        "ankle": (255, 255, 120, 255),
        "ground": (255, 255, 255, 255),
    }
    for idx, sprite in enumerate(sprites):
        r, c = divmod(idx, cols)
        x0, y0 = c * cell, r * cell
        img = atlas.image(sprite)
        info = atlas.info(sprite)
        scale = min((cell - 24) / max(1, img.width), (cell - 24) / max(1, img.height), 1.0)
        w = max(1, int(round(img.width * scale)))
        h = max(1, int(round(img.height * scale)))
        simg = img.resize((w, h), Image.Resampling.LANCZOS) if scale != 1.0 else img.copy()
        px = x0 + int(round((cell - w) / 2))
        py = y0 + int(round((cell - h) / 2))
        out.alpha_composite(simg, (px, py))
        d = ImageDraw.Draw(out)
        def mark(pt: Sequence[float], color: RGBA, radius: int = 4):
            x = px + float(pt[0]) * scale
            y = py + float(pt[1]) * scale
            d.ellipse((x-radius, y-radius, x+radius, y+radius), outline=color, fill=(0,0,0,0), width=2)
            d.line((x-7, y, x+7, y), fill=color, width=1)
            d.line((x, y-7, x, y+7), fill=color, width=1)
        if "pivot" in info:
            mark(info["pivot"], anchor_palette["pivot"], 5)
        for name, pt in (info.get("anchors") or {}).items():
            mark(pt, anchor_palette.get(name, (255, 255, 255, 255)), 4)
    return out


def write_component_anchors(job: RigJob, output: str | Path, sprites: Sequence[str], cell: int = 160) -> Path:
    output = Path(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    img = build_component_anchor_sheet(job, sprites, cell=cell)
    img.save(output)
    return output


def _cmd_list(args: argparse.Namespace) -> int:
    for name, info in ANIMATIONS.items():
        print(f"{name}: {info['frames']}f/{info['duration_ms']}ms")
    return 0


def _cmd_spritesheet(args: argparse.Namespace) -> int:
    job = RigJob.load(args.config)
    image_out, manifest_out = write_spritesheet(job, args.output, args.manifest_out)
    print(image_out)
    print(manifest_out)
    return 0


def _cmd_debug_spritesheet(args: argparse.Namespace) -> int:
    job = RigJob.load(args.config)
    if not getattr(args, "keep_sheet_labels", False):
        # Debug output is for geometric reasoning; omit all text by default.
        job.render.label_width = 0
    image_out, manifest_out = write_spritesheet(job, args.output, args.manifest_out, debug_parts=True)
    print(image_out)
    print(manifest_out)
    return 0


def _cmd_single(args: argparse.Namespace) -> int:
    job = RigJob.load(args.config)
    out = write_single(job, args.output, args.animation, args.frame_index)
    print(out)
    print(out.with_suffix(".json"))
    return 0


def _cmd_debug_frame(args: argparse.Namespace) -> int:
    job = RigJob.load(args.config)
    out = write_debug_frame(job, args.output, args.animation, args.frame_index, zoom=args.zoom, pad=args.pad, background=args.background)
    print(out)
    print(out.with_suffix(".json"))
    return 0


def _cmd_draw_default(args: argparse.Namespace) -> int:
    for p in draw_default(args.root):
        print(p)
    return 0


def _cmd_component_anchors(args: argparse.Namespace) -> int:
    job = RigJob.load(args.config)
    out = write_component_anchors(job, args.output, args.sprites, cell=args.cell)
    print(out)
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="robot-rig-sheet", description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("list-animations", help="List built-in starter animation rows.")
    p.set_defaults(func=_cmd_list)

    p = sub.add_parser("spritesheet", help="Build a spritesheet from a rig job YAML.")
    p.add_argument("config")
    p.add_argument("output")
    p.add_argument("--manifest-out", default=None)
    p.set_defaults(func=_cmd_spritesheet)

    p = sub.add_parser("debug-spritesheet", help="Build a flat-color anchor/debug spritesheet from a rig job YAML.")
    p.add_argument("config")
    p.add_argument("output")
    p.add_argument("--manifest-out", default=None)
    p.add_argument("--keep-sheet-labels", action="store_true", help="Keep animation labels in the debug sheet. Component labels are never drawn.")
    p.set_defaults(func=_cmd_debug_spritesheet)

    p = sub.add_parser("single", help="Render one assembled frame.")
    p.add_argument("config")
    p.add_argument("output")
    p.add_argument("--animation", default="idle")
    p.add_argument("--frame-index", type=int, default=0)
    p.set_defaults(func=_cmd_single)

    p = sub.add_parser("debug-frame", help="Render one enlarged cropped debug frame with solid components and anchors.")
    p.add_argument("config")
    p.add_argument("output")
    p.add_argument("--animation", default="run")
    p.add_argument("--frame-index", type=int, default=0)
    p.add_argument("--zoom", type=int, default=5)
    p.add_argument("--pad", type=int, default=30)
    p.add_argument("--background", default="black")
    p.set_defaults(func=_cmd_debug_frame)

    p = sub.add_parser("component-anchors", help="Draw component-local anchor markers with no text.")
    p.add_argument("config")
    p.add_argument("output")
    p.add_argument("sprites", nargs="+")
    p.add_argument("--cell", type=int, default=160)
    p.set_defaults(func=_cmd_component_anchors)

    p = sub.add_parser("draw-default", help="Build examples/robot_rig_job.yaml outputs.")
    p.add_argument("--root", default=".")
    p.set_defaults(func=_cmd_draw_default)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args) or 0)


if __name__ == "__main__":
    raise SystemExit(main())
