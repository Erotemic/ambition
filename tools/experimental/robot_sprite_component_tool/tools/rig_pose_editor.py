#!/usr/bin/env python3
"""Integrated anchor / pose / z-order editor for the robot rig.

This editor is intentionally repo-specific and sits above ``robot_rig_sheet.py``.
It edits two data layers:

* component-local anchors in ``metadata/robot_components.refined.yaml``
* logical per-frame pose overrides in ``metadata/robot_pose_overrides.yaml``

Unlike ``anchor_editor.py``, this GUI is organized around animation frames and
logical part instances (front_arm, back_arm, front_leg, etc.).  One art asset can
be reused by many instances with different pivots, rotations, z-order, and
per-frame endpoint targets.
"""
from __future__ import annotations

import argparse
import copy
import importlib.util
import json
import math
import shutil
import sys
import tempfile
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, Iterable, List, Mapping, Optional, Sequence, Tuple

import yaml
from PIL import Image, ImageDraw, ImageTk

Point = Tuple[float, float]

ANCHOR_COLORS: Dict[str, Tuple[int, int, int]] = {
    "pivot": (255, 255, 255),
    "neck": (0, 196, 255),
    "face_socket": (0, 255, 255),
    "antenna_socket": (160, 80, 255),
    "shoulder": (255, 120, 80),
    "shoulder_left": (0, 230, 80),
    "shoulder_right": (255, 80, 60),
    "wrist": (255, 240, 0),
    "hip": (255, 210, 0),
    "hip_left": (165, 82, 255),
    "hip_right": (255, 150, 0),
    "ground": (255, 255, 255),
}

ROLE_TO_SPRITE_FIELD = {
    "torso": "torso_sprite",
    "head": "head_sprite",
    "front_arm": "front_arm_sprite",
    "back_arm": "back_arm_sprite",
    "front_hand": "front_hand_sprite",
    "back_hand": "back_hand_sprite",
    "front_leg": "front_leg_sprite",
    "back_leg": "back_leg_sprite",
    "front_foot": "front_foot_sprite",
    "back_foot": "back_foot_sprite",
    "face": "face_sprite",
}

ROLE_TO_ANGLE_FIELD = {
    "torso": "torso_angle",
    "head": "head_angle",
    "front_arm": "front_arm_angle",
    "back_arm": "back_arm_angle",
    "front_hand": "front_hand_angle",
    "back_hand": "back_hand_angle",
    "front_leg": "front_leg_angle",
    "back_leg": "back_leg_angle",
    "front_foot": "front_foot_angle",
    "back_foot": "back_foot_angle",
}

ROLE_TO_DELTA_FIELD = {
    # Godot-like direct manipulation channels. These fields are sparse
    # keyframe offsets/endpoints, not component-local anchor edits.
    "torso": "torso_offset",
    "head": "head_offset",
    "front_arm": "front_wrist_delta",
    "back_arm": "back_wrist_delta",
    "front_hand": "front_hand_offset",
    "back_hand": "back_hand_offset",
    "front_leg": "front_ground_delta",
    "back_leg": "back_ground_delta",
    "front_foot": "front_foot_offset",
    "back_foot": "back_foot_offset",
}

ROLE_TO_SCALE_FIELD = {
    "torso": "torso_scale",
    "head": "head_scale",
    "front_arm": "front_arm_scale",
    "back_arm": "back_arm_scale",
    "front_hand": "front_hand_scale",
    "back_hand": "back_hand_scale",
    "front_leg": "front_leg_scale",
    "back_leg": "back_leg_scale",
    "front_foot": "front_foot_scale",
    "back_foot": "back_foot_scale",
    "face": "face_scale",
}

DEFAULT_Z_ORDER = [
    "fx_behind", "back_foot", "back_leg", "back_hand", "back_arm", "torso",
    "front_leg", "front_foot", "front_arm", "front_hand", "head", "fx_front",
]


def load_yaml(path: Path) -> Dict[str, Any]:
    if not path.exists():
        return {}
    return yaml.safe_load(path.read_text(encoding="utf8")) or {}


def save_yaml(path: Path, data: Mapping[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(yaml.safe_dump(data, sort_keys=False, allow_unicode=True), encoding="utf8")


def backup(path: Path) -> Optional[Path]:
    if not path.exists():
        return None
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    out = path.with_suffix(path.suffix + f".bak-{stamp}")
    shutil.copy2(path, out)
    return out


def point(value: Any, default: Point = (0.0, 0.0)) -> Point:
    if isinstance(value, (list, tuple)) and len(value) == 2:
        return (float(value[0]), float(value[1]))
    return default


def point_list(pt: Point) -> List[int]:
    return [int(round(pt[0])), int(round(pt[1]))]


def color_for_name(name: str) -> Tuple[int, int, int]:
    if name in ANCHOR_COLORS:
        return ANCHOR_COLORS[name]
    h = 0
    for ch in name:
        h = (h * 131 + ord(ch)) & 0xFFFFFFFF
    return (80 + (h & 127), 80 + ((h >> 8) & 127), 80 + ((h >> 16) & 127))


def checkerboard(size: Tuple[int, int], cell: int = 8) -> Image.Image:
    w, h = size
    img = Image.new("RGBA", size, (210, 210, 210, 255))
    draw = ImageDraw.Draw(img)
    for y in range(0, h, cell):
        for x in range(0, w, cell):
            if ((x // cell) + (y // cell)) % 2 == 0:
                draw.rectangle([x, y, x + cell - 1, y + cell - 1], fill=(235, 235, 235, 255))
    return img


def composite_bg(img: Image.Image, bg: str = "black") -> Image.Image:
    img = img.convert("RGBA")
    if bg == "checker":
        base = checkerboard(img.size, 12)
    elif bg == "white":
        base = Image.new("RGBA", img.size, (255, 255, 255, 255))
    else:
        base = Image.new("RGBA", img.size, (0, 0, 0, 255))
    base.alpha_composite(img)
    return base


def import_rig_module():
    tool_path = Path(__file__).with_name("robot_rig_sheet.py")
    spec = importlib.util.spec_from_file_location("robot_rig_sheet_for_pose_editor", tool_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Cannot import {tool_path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@dataclass
class EditorPaths:
    job: Path
    metadata: Path
    slices: Path
    pose_overrides: Path


def resolve_job_paths(job_path: Path, metadata: Optional[Path], slices: Optional[Path], pose_overrides: Optional[Path]) -> EditorPaths:
    rig = import_rig_module()
    job = rig.RigJob.load(job_path)
    return EditorPaths(
        job=job_path.resolve(),
        metadata=(metadata.resolve() if metadata else job.metadata.resolve()),
        slices=(slices.resolve() if slices else job.slices.resolve()),
        pose_overrides=(pose_overrides.resolve() if pose_overrides else (job.pose_overrides or job_path.parent.parent / "metadata" / "robot_pose_overrides.yaml").resolve()),
    )


class PoseModel:
    """Small wrapper around the pose override YAML."""

    def __init__(self, data: Optional[Dict[str, Any]] = None):
        self.data = data or {"version": "0.2", "animations": {}}
        self.data.setdefault("animations", {})

    def anim(self, name: str) -> Dict[str, Any]:
        return self.data.setdefault("animations", {}).setdefault(name, {})

    def frame(self, name: str, idx: int) -> Dict[str, Any]:
        anim = self.anim(name)
        frames = anim.setdefault("frame_overrides", None)
        # Backward compatibility: early files used ``frames`` both for frame
        # count and frame override mapping.  Prefer frame_overrides, but read
        # mapping-style frames when present.
        if frames is None:
            frames = anim.get("frames") if isinstance(anim.get("frames"), dict) else None
        if frames is None:
            frames = {}
            anim["frame_overrides"] = frames
        return frames.setdefault(str(idx), {})

    def clean_for_save(self) -> Dict[str, Any]:
        out = copy.deepcopy(self.data)
        for anim, adata in (out.get("animations") or {}).items():
            # Keep frame_count distinct from the frame override mapping.  The
            # compositor still reads mapping-style ``frames`` for compatibility,
            # but this file is easier to edit when the semantics are explicit.
            if isinstance(adata.get("frames"), dict):
                adata.setdefault("frame_overrides", adata.pop("frames"))
            frames = adata.get("frame_overrides")
            if isinstance(frames, dict):
                for key in list(frames.keys()):
                    if not frames[key]:
                        frames.pop(key, None)
                if not frames:
                    adata.pop("frame_overrides", None)
        return out


def write_temp_yaml(data: Mapping[str, Any]) -> Path:
    tmp = tempfile.NamedTemporaryFile("w", suffix=".yaml", encoding="utf8", delete=False)
    with tmp:
        yaml.safe_dump(data, tmp, sort_keys=False, allow_unicode=True)
        tmp.flush()
    return Path(tmp.name)


def build_preview(job_path: Path, metadata: Mapping[str, Any], pose_data: Mapping[str, Any], *, animations: Optional[List[str]] = None, debug: bool = False, highlight: Optional[Dict[str, Any]] = None, bg: str = "black") -> Tuple[Image.Image, Dict[str, Any]]:
    rig = import_rig_module()
    meta_path = write_temp_yaml(metadata)
    pose_path = write_temp_yaml(pose_data)
    try:
        job = rig.RigJob.load(job_path)
        job.metadata = meta_path
        job.pose_overrides = pose_path
        if animations:
            job.animations = animations
        sheet, manifest = rig.build_spritesheet(job, debug_parts=debug)
        if highlight:
            draw_highlight(sheet, manifest, highlight)
        return composite_bg(sheet, bg), manifest
    finally:
        for p in [meta_path, pose_path]:
            try:
                p.unlink()
            except OSError:
                pass


def draw_highlight(sheet: Image.Image, manifest: Mapping[str, Any], highlight: Mapping[str, Any]) -> None:
    role = highlight.get("role")
    sprite = highlight.get("sprite")
    animation = highlight.get("animation")
    frame_index = highlight.get("frame_index")
    d = ImageDraw.Draw(sheet)
    for anim, adata in (manifest.get("animations") or {}).items():
        if animation and anim != animation:
            continue
        for frame in adata.get("frames", []):
            if frame_index is not None and int(frame.get("index", -1)) != int(frame_index):
                continue
            x0, y0 = int(frame["x"]), int(frame["y"])
            matched = False
            for comp in (frame.get("pose") or {}).get("components", []):
                if role and comp.get("role") != role:
                    continue
                if sprite and str(comp.get("sprite", "")).split("@")[0] != str(sprite).split("@")[0]:
                    continue
                tx, ty = comp.get("target", [None, None])
                if tx is None:
                    continue
                cx, cy = x0 + float(tx), y0 + float(ty)
                matched = True
                d.ellipse((cx - 8, cy - 8, cx + 8, cy + 8), outline=(255, 255, 0, 255), width=3)
                d.line((cx - 13, cy, cx + 13, cy), fill=(255, 255, 0, 255), width=2)
                d.line((cx, cy - 13, cx, cy + 13), fill=(255, 255, 0, 255), width=2)
            if matched:
                d.rectangle((x0 + 1, y0 + 1, x0 + int(frame["w"]) - 2, y0 + int(frame["h"]) - 2), outline=(255, 240, 0, 255), width=2)


def relevant_animations(job_path: Path, metadata: Mapping[str, Any], pose_data: Mapping[str, Any], selected_sprite: str) -> List[str]:
    rig = import_rig_module()
    job = rig.RigJob.load(job_path)
    overrides = pose_data or {}
    selected_base = selected_sprite.split("@")[0]
    found: List[str] = []
    for anim in job.animations:
        info = rig.animation_info(anim, overrides)
        for idx in range(info["frames"]):
            pose = rig.animation_pose(anim, idx, info["frames"], job.render.scale)
            pose = rig.apply_pose_overrides(pose, anim, idx, overrides)
            refs = rig.pose_sprite_refs(pose)
            if any(rig.base_sprite_name(v) == selected_base for v in refs.values()):
                found.append(anim)
                break
    return found or list(job.animations[:1])


class RigPoseEditor:
    def __init__(self, root: "tk.Tk", paths: EditorPaths, *, zoom: int = 5, bg: str = "checker"):
        import tkinter as tk
        from tkinter import ttk

        self.tk = tk
        self.ttk = ttk
        self.root = root
        self.paths = paths
        self.zoom = max(1, int(zoom))
        self.bg = bg
        self.rig = import_rig_module()
        self.job = self.rig.RigJob.load(paths.job)
        self.job.metadata = paths.metadata
        self.job.slices = paths.slices
        self.job.pose_overrides = paths.pose_overrides
        self.metadata = load_yaml(paths.metadata)
        self.pose_model = PoseModel(load_yaml(paths.pose_overrides))
        self.sprites = self.metadata.setdefault("sprites", {})
        self.sprite_names = sorted(self.sprites.keys())
        self.selected_sprite = self.sprite_names[0]
        self.selected_anchor = "pivot"
        self.selected_instance: Optional[Dict[str, Any]] = None
        self.current_manifest: Dict[str, Any] = {}
        self._component_photo = None
        self._preview_photo = None
        self._anim_photo = None
        self._preview_after = None
        self._playing = False
        self._play_index = 0
        self.dirty_meta = False
        self.dirty_pose = False

        # Tk variables
        self.sprite_filter_var = tk.StringVar(value="")
        self.anchor_x = tk.StringVar(value="0")
        self.anchor_y = tk.StringVar(value="0")
        self.pose_anim = tk.StringVar(value=self.job.animations[0] if self.job.animations else "run")
        self.pose_frame = tk.IntVar(value=0)
        self.pose_role = tk.StringVar(value="front_arm")
        self.pose_art = tk.StringVar(value="")
        self.pose_angle = tk.DoubleVar(value=0.0)
        self.pose_dx = tk.DoubleVar(value=0.0)
        self.pose_dy = tk.DoubleVar(value=0.0)
        self.anim_frame_count = tk.IntVar(value=8)
        self.anim_duration_ms = tk.IntVar(value=75)
        self.preview_relevant_only = tk.BooleanVar(value=True)
        self.preview_debug = tk.BooleanVar(value=False)
        self.preview_fit = tk.BooleanVar(value=True)
        self.preview_bg = tk.StringVar(value="black")
        self.status = tk.StringVar(value="Ready")

        self._build_ui()
        self._bind()
        self.populate_sprite_list()
        self.populate_animation_tree()
        self.select_sprite(self.selected_sprite)
        self.refresh_preview(force=True)

    # UI -----------------------------------------------------------------
    def _build_ui(self) -> None:
        tk, ttk = self.tk, self.ttk
        root = self.root
        root.title("Robot Rig Pose Editor")
        root.geometry("2100x1180")
        root.minsize(1300, 800)
        root.rowconfigure(0, weight=1)
        root.columnconfigure(0, weight=1)

        main = ttk.Panedwindow(root, orient="horizontal")
        main.grid(row=0, column=0, sticky="nsew")

        left = ttk.Frame(main, padding=6)
        mid = ttk.Frame(main, padding=6)
        right = ttk.Frame(main, padding=6)
        main.add(left, weight=1)
        main.add(mid, weight=2)
        main.add(right, weight=3)

        # Left: animation/frame/part tree and pose controls.
        left.rowconfigure(2, weight=1)
        left.columnconfigure(0, weight=1)
        ttk.Label(left, text="Animation / frame / part tree").grid(row=0, column=0, sticky="w")
        tree_frame = ttk.Frame(left)
        tree_frame.grid(row=2, column=0, sticky="nsew")
        tree_frame.rowconfigure(0, weight=1)
        tree_frame.columnconfigure(0, weight=1)
        self.tree = ttk.Treeview(tree_frame, columns=("sprite",), show="tree headings", height=16)
        self.tree.heading("#0", text="Instance")
        self.tree.heading("sprite", text="Art")
        self.tree.column("#0", width=210)
        self.tree.column("sprite", width=150)
        self.tree.grid(row=0, column=0, sticky="nsew")
        ttk.Scrollbar(tree_frame, orient="vertical", command=self.tree.yview).grid(row=0, column=1, sticky="ns")
        self.tree.configure(yscrollcommand=lambda *a: None)

        controls = ttk.LabelFrame(left, text="Selected instance / frame controls", padding=6)
        controls.grid(row=3, column=0, sticky="ew", pady=(8, 0))
        for c in range(4): controls.columnconfigure(c, weight=1)
        ttk.Label(controls, text="anim").grid(row=0, column=0, sticky="w")
        ttk.Entry(controls, textvariable=self.pose_anim, width=10).grid(row=0, column=1, sticky="ew")
        ttk.Label(controls, text="frame").grid(row=0, column=2, sticky="w")
        ttk.Spinbox(controls, from_=0, to=999, textvariable=self.pose_frame, width=6, command=self.on_pose_controls_changed).grid(row=0, column=3, sticky="ew")
        ttk.Label(controls, text="role").grid(row=1, column=0, sticky="w")
        ttk.Entry(controls, textvariable=self.pose_role, width=16).grid(row=1, column=1, sticky="ew")
        ttk.Label(controls, text="art").grid(row=1, column=2, sticky="w")
        self.art_combo = ttk.Combobox(controls, textvariable=self.pose_art, values=self.sprite_names, state="normal")
        self.art_combo.grid(row=1, column=3, sticky="ew")
        ttk.Label(controls, text="angle").grid(row=2, column=0, sticky="w")
        ttk.Spinbox(controls, from_=-180, to=180, increment=1, textvariable=self.pose_angle, width=8, command=self.apply_pose_edit).grid(row=2, column=1, sticky="ew")
        ttk.Label(controls, text="delta x").grid(row=2, column=2, sticky="w")
        ttk.Spinbox(controls, from_=-120, to=120, increment=1, textvariable=self.pose_dx, width=8, command=self.apply_pose_edit).grid(row=2, column=3, sticky="ew")
        ttk.Label(controls, text="delta y").grid(row=3, column=0, sticky="w")
        ttk.Spinbox(controls, from_=-120, to=120, increment=1, textvariable=self.pose_dy, width=8, command=self.apply_pose_edit).grid(row=3, column=1, sticky="ew")
        ttk.Button(controls, text="Apply pose edit", command=self.apply_pose_edit).grid(row=3, column=2, columnspan=2, sticky="ew")
        ttk.Button(controls, text="Navigate: joint -> connected part", command=self.navigate_connected_part).grid(row=4, column=0, columnspan=4, sticky="ew", pady=(4,0))

        animctl = ttk.LabelFrame(left, text="Animation length / timing", padding=6)
        animctl.grid(row=4, column=0, sticky="ew", pady=(8, 0))
        ttk.Label(animctl, text="frames").grid(row=0, column=0)
        ttk.Spinbox(animctl, from_=1, to=64, textvariable=self.anim_frame_count, width=6, command=self.apply_anim_settings).grid(row=0, column=1)
        ttk.Label(animctl, text="ms/frame").grid(row=0, column=2)
        ttk.Spinbox(animctl, from_=10, to=500, textvariable=self.anim_duration_ms, width=6, command=self.apply_anim_settings).grid(row=0, column=3)
        ttk.Button(animctl, text="Add frame", command=self.add_frame).grid(row=1, column=0, columnspan=2, sticky="ew", pady=(4,0))
        ttk.Button(animctl, text="Remove last", command=self.remove_frame).grid(row=1, column=2, columnspan=2, sticky="ew", pady=(4,0))

        zbox = ttk.LabelFrame(left, text="Frame z-order (bottom -> top)", padding=6)
        zbox.grid(row=5, column=0, sticky="ew", pady=(8, 0))
        self.z_list = tk.Listbox(zbox, height=10, exportselection=False)
        self.z_list.grid(row=0, column=0, rowspan=4, sticky="ew")
        ttk.Button(zbox, text="Up", command=lambda: self.move_z(-1)).grid(row=0, column=1, sticky="ew")
        ttk.Button(zbox, text="Down", command=lambda: self.move_z(1)).grid(row=1, column=1, sticky="ew")
        ttk.Button(zbox, text="Reset", command=self.reset_z_order).grid(row=2, column=1, sticky="ew")

        # Middle: component anchor editor.
        mid.rowconfigure(3, weight=1)
        mid.columnconfigure(0, weight=1)
        sf = ttk.Frame(mid)
        sf.grid(row=0, column=0, sticky="ew")
        sf.columnconfigure(1, weight=1)
        ttk.Label(sf, text="Sprite filter").grid(row=0, column=0, sticky="w")
        ttk.Entry(sf, textvariable=self.sprite_filter_var).grid(row=0, column=1, sticky="ew", padx=(4,0))
        self.sprite_list = tk.Listbox(mid, height=10, exportselection=False)
        self.sprite_list.grid(row=1, column=0, sticky="ew", pady=(4, 4))
        af = ttk.Frame(mid)
        af.grid(row=2, column=0, sticky="ew")
        af.columnconfigure(0, weight=1)
        self.anchor_list = tk.Listbox(af, height=8, exportselection=False)
        self.anchor_list.grid(row=0, column=0, sticky="ew")
        xy = ttk.Frame(mid)
        xy.grid(row=4, column=0, sticky="ew", pady=(4,0))
        ttk.Label(xy, text="anchor x").grid(row=0, column=0)
        ttk.Entry(xy, textvariable=self.anchor_x, width=7).grid(row=0, column=1)
        ttk.Label(xy, text="y").grid(row=0, column=2)
        ttk.Entry(xy, textvariable=self.anchor_y, width=7).grid(row=0, column=3)
        ttk.Button(xy, text="Apply anchor", command=self.apply_anchor_xy).grid(row=0, column=4, padx=(8,0))
        ttk.Button(xy, text="Pivot follows selected", command=self.pivot_follows_selected).grid(row=0, column=5, padx=(4,0))
        self.component_canvas = tk.Canvas(mid, background="#202020", highlightthickness=0)
        self.component_canvas.grid(row=3, column=0, sticky="nsew")

        # Right: live sheet and animated preview.
        right.rowconfigure(2, weight=1)
        right.rowconfigure(4, weight=1)
        right.columnconfigure(0, weight=1)
        opts = ttk.Frame(right)
        opts.grid(row=0, column=0, sticky="ew")
        ttk.Checkbutton(opts, text="relevant only", variable=self.preview_relevant_only, command=self.refresh_preview).grid(row=0, column=0, sticky="w")
        ttk.Checkbutton(opts, text="debug", variable=self.preview_debug, command=self.refresh_preview).grid(row=0, column=1, sticky="w")
        ttk.Checkbutton(opts, text="fit", variable=self.preview_fit, command=self.refresh_preview).grid(row=0, column=2, sticky="w")
        ttk.Combobox(opts, textvariable=self.preview_bg, values=["black", "checker", "white"], width=8, state="readonly").grid(row=0, column=3)
        ttk.Button(opts, text="Render now", command=lambda: self.refresh_preview(force=True)).grid(row=0, column=4, padx=(8,0))
        ttk.Button(opts, text="Play/stop selected animation", command=self.toggle_play).grid(row=0, column=5, padx=(8,0))
        self.preview_canvas = tk.Canvas(right, background="#101010", highlightthickness=0)
        self.preview_canvas.grid(row=2, column=0, sticky="nsew")
        self.anim_canvas = tk.Canvas(right, background="#101010", height=260, highlightthickness=0)
        self.anim_canvas.grid(row=4, column=0, sticky="nsew", pady=(8,0))

        savebar = ttk.Frame(root)
        savebar.grid(row=1, column=0, sticky="ew")
        savebar.columnconfigure(1, weight=1)
        ttk.Button(savebar, text="Save metadata + pose overrides", command=self.save).grid(row=0, column=0, padx=6, pady=4)
        ttk.Label(savebar, textvariable=self.status).grid(row=0, column=1, sticky="ew")

    def _bind(self) -> None:
        self.sprite_filter_var.trace_add("write", lambda *_: self.populate_sprite_list())
        self.sprite_list.bind("<<ListboxSelect>>", self.on_sprite_select)
        self.anchor_list.bind("<<ListboxSelect>>", self.on_anchor_select)
        self.component_canvas.bind("<Button-1>", self.on_component_click)
        self.tree.bind("<<TreeviewSelect>>", self.on_tree_select)
        for var in [self.pose_art, self.pose_angle, self.pose_dx, self.pose_dy]:
            try:
                var.trace_add("write", lambda *_: self._defer_pose_edit())
            except Exception:
                pass
        self.preview_bg.trace_add("write", lambda *_: self.refresh_preview())
        self.root.bind("<Control-s>", lambda e: self.save())
        self.root.bind("<Control-r>", lambda e: self.refresh_preview(force=True))

    # Selection + tree ----------------------------------------------------
    def populate_sprite_list(self) -> None:
        f = self.sprite_filter_var.get().lower().strip()
        self.sprite_list.delete(0, "end")
        for name in self.sprite_names:
            if not f or f in name.lower():
                self.sprite_list.insert("end", name)

    def populate_animation_tree(self) -> None:
        self.tree.delete(*self.tree.get_children())
        # Build a lightweight manifest from current state to expose actual
        # per-frame part instances and art choices.
        try:
            _img, manifest = build_preview(self.paths.job, self.metadata, self.pose_model.clean_for_save(), animations=list(self.job.animations), debug=False, bg="black")
            self.current_manifest = manifest
        except Exception as ex:
            self.status.set(f"Tree render failed: {ex}")
            return
        for anim, adata in (self.current_manifest.get("animations") or {}).items():
            aid = f"anim:{anim}"
            self.tree.insert("", "end", aid, text=anim, values=(f"{adata.get('duration_ms')}ms",), open=(anim == self.pose_anim.get()))
            for frame in adata.get("frames", []):
                idx = int(frame["index"])
                fid = f"frame:{anim}:{idx}"
                self.tree.insert(aid, "end", fid, text=f"frame {idx}", values=("",), open=False)
                for comp in (frame.get("pose") or {}).get("components", []):
                    role = comp.get("role")
                    sid = f"part:{anim}:{idx}:{role}"
                    self.tree.insert(fid, "end", sid, text=str(role), values=(comp.get("sprite", ""),))

    def on_tree_select(self, _event=None) -> None:
        sel = self.tree.selection()
        if not sel:
            return
        iid = sel[0]
        parts = iid.split(":")
        if parts[0] == "anim":
            self.pose_anim.set(parts[1])
            self.update_anim_fields()
            self.refresh_preview()
            return
        if parts[0] == "frame":
            self.pose_anim.set(parts[1])
            self.pose_frame.set(int(parts[2]))
            self.update_anim_fields()
            self.refresh_preview()
            return
        if parts[0] == "part":
            anim, idx, role = parts[1], int(parts[2]), parts[3]
            comp = self.find_component(anim, idx, role)
            if comp:
                self.selected_instance = {"animation": anim, "frame_index": idx, "role": role, "sprite": comp.get("sprite")}
                self.pose_anim.set(anim)
                self.pose_frame.set(idx)
                self.pose_role.set(role)
                self.pose_art.set(str(comp.get("sprite", "")))
                self.load_pose_fields_from_model(role, anim, idx, comp)
                base = str(comp.get("sprite", "")).split("@")[0]
                if base in self.sprites:
                    self.select_sprite(base)
                self.populate_z_order()
                self.refresh_preview(force=True)

    def find_component(self, anim: str, idx: int, role: str) -> Optional[Dict[str, Any]]:
        adata = (self.current_manifest.get("animations") or {}).get(anim) or {}
        for frame in adata.get("frames", []):
            if int(frame.get("index", -1)) == int(idx):
                for comp in (frame.get("pose") or {}).get("components", []):
                    if comp.get("role") == role:
                        return comp
        return None

    def select_sprite(self, name: str) -> None:
        if name not in self.sprites:
            return
        self.selected_sprite = name
        self.populate_anchor_list()
        self.draw_component()
        for i in range(self.sprite_list.size()):
            if self.sprite_list.get(i) == name:
                self.sprite_list.selection_clear(0, "end")
                self.sprite_list.selection_set(i)
                self.sprite_list.see(i)
                break

    def on_sprite_select(self, _event=None) -> None:
        sel = self.sprite_list.curselection()
        if sel:
            self.select_sprite(self.sprite_list.get(sel[0]))
            self.refresh_preview()

    # Anchor editor ------------------------------------------------------
    def anchor_names(self) -> List[str]:
        s = self.sprites.get(self.selected_sprite, {})
        return ["pivot"] + sorted((s.get("anchors") or {}).keys())

    def populate_anchor_list(self) -> None:
        self.anchor_list.delete(0, "end")
        for name in self.anchor_names():
            self.anchor_list.insert("end", name)
        self.select_anchor(self.selected_anchor if self.selected_anchor in self.anchor_names() else "pivot")

    def select_anchor(self, name: str) -> None:
        self.selected_anchor = name
        for i in range(self.anchor_list.size()):
            if self.anchor_list.get(i) == name:
                self.anchor_list.selection_clear(0, "end")
                self.anchor_list.selection_set(i)
                break
        pt = self.get_anchor_point(name)
        self.anchor_x.set(str(int(round(pt[0]))))
        self.anchor_y.set(str(int(round(pt[1]))))
        self.draw_component()

    def on_anchor_select(self, _event=None) -> None:
        sel = self.anchor_list.curselection()
        if sel:
            self.select_anchor(self.anchor_list.get(sel[0]))

    def get_anchor_point(self, name: str) -> Point:
        s = self.sprites.get(self.selected_sprite, {})
        if name == "pivot":
            pa = s.get("pivot_anchor")
            if pa and pa in (s.get("anchors") or {}):
                return point(s["anchors"][pa])
            return point(s.get("pivot"), (0, 0))
        return point((s.get("anchors") or {}).get(name), (0, 0))

    def set_anchor_point(self, name: str, pt: Point) -> None:
        s = self.sprites[self.selected_sprite]
        if name == "pivot":
            s.pop("pivot_anchor", None)
            s["pivot"] = point_list(pt)
        else:
            s.setdefault("anchors", {})[name] = point_list(pt)
            if s.get("pivot_anchor") == name:
                s["pivot"] = point_list(pt)
        self.dirty_meta = True

    def draw_component(self) -> None:
        path = self.paths.slices / f"{self.selected_sprite}.png"
        if not path.exists():
            return
        img = Image.open(path).convert("RGBA")
        base = composite_bg(img, self.bg)
        z = self.zoom
        scaled = base.resize((base.width * z, base.height * z), Image.Resampling.NEAREST)
        d = ImageDraw.Draw(scaled)
        for name in self.anchor_names():
            x, y = self.get_anchor_point(name)
            x *= z; y *= z
            col = color_for_name(name)
            r = 7 if name == self.selected_anchor else 5
            d.line((x - 14, y, x + 14, y), fill=(*col, 255), width=2)
            d.line((x, y - 14, x, y + 14), fill=(*col, 255), width=2)
            d.ellipse((x-r, y-r, x+r, y+r), outline=(255,255,255,255), width=2)
        self._component_photo = ImageTk.PhotoImage(scaled)
        self.component_canvas.delete("all")
        self.component_canvas.create_image(0, 0, image=self._component_photo, anchor="nw")
        self.component_canvas.configure(scrollregion=(0, 0, scaled.width, scaled.height))

    def on_component_click(self, event) -> None:
        x = self.component_canvas.canvasx(event.x) / self.zoom
        y = self.component_canvas.canvasy(event.y) / self.zoom
        self.set_anchor_point(self.selected_anchor, (x, y))
        self.select_anchor(self.selected_anchor)
        self.populate_animation_tree()
        self.refresh_preview()

    def apply_anchor_xy(self) -> None:
        self.set_anchor_point(self.selected_anchor, (float(self.anchor_x.get()), float(self.anchor_y.get())))
        self.draw_component()
        self.populate_animation_tree()
        self.refresh_preview()

    def pivot_follows_selected(self) -> None:
        if self.selected_anchor == "pivot":
            return
        s = self.sprites[self.selected_sprite]
        s["pivot_anchor"] = self.selected_anchor
        s["pivot"] = point_list(self.get_anchor_point(self.selected_anchor))
        self.dirty_meta = True
        self.select_anchor("pivot")
        self.refresh_preview()

    # Pose editing -------------------------------------------------------
    def current_frame_override(self) -> Dict[str, Any]:
        return self.pose_model.frame(self.pose_anim.get(), int(self.pose_frame.get()))

    def load_pose_fields_from_model(self, role: str, anim: str, idx: int, comp: Mapping[str, Any]) -> None:
        fr = self.pose_model.frame(anim, idx)
        angle_field = ROLE_TO_ANGLE_FIELD.get(role)
        sprite_field = ROLE_TO_SPRITE_FIELD.get(role)
        delta_field = ROLE_TO_DELTA_FIELD.get(role)
        if sprite_field:
            self.pose_art.set(str(fr.get(sprite_field, comp.get("sprite", ""))))
        if angle_field:
            self.pose_angle.set(float(fr.get(angle_field, comp.get("angle", 0.0))))
        if delta_field:
            default_delta = fr.get(delta_field)
            if default_delta is None:
                endpoint = comp.get("endpoint")
                target = comp.get("target")
                if endpoint and target:
                    default_delta = [float(endpoint[0]) - float(target[0]), float(endpoint[1]) - float(target[1])]
            dx, dy = point(default_delta, (0.0, 0.0))
            self.pose_dx.set(dx); self.pose_dy.set(dy)
        self.update_anim_fields()

    def _defer_pose_edit(self) -> None:
        # Avoid applying during initialization before a selected instance exists.
        if self.selected_instance is None:
            return
        self.root.after(50, self.apply_pose_edit)

    def on_pose_controls_changed(self) -> None:
        """Handle animation/frame selector changes in the Tk fallback UI.

        The PySide6 editor is the primary editor now, but this legacy Tk
        fallback is still useful as a lightweight/headless core module.  The
        frame spinbox was wired to this callback before the method existed,
        which caused direct ``python tools/rig_pose_editor.py ...`` launches to
        crash during UI construction.
        """
        try:
            self._play_index = max(0, int(self.pose_frame.get()))
        except Exception:
            self._play_index = 0
        try:
            self.update_anim_fields()
        except Exception:
            pass
        self.populate_z_order()
        self.refresh_preview()

    def apply_pose_edit(self) -> None:
        if self.selected_instance is None:
            return
        role = self.pose_role.get()
        fr = self.current_frame_override()
        sprite_field = ROLE_TO_SPRITE_FIELD.get(role)
        angle_field = ROLE_TO_ANGLE_FIELD.get(role)
        delta_field = ROLE_TO_DELTA_FIELD.get(role)
        if sprite_field and self.pose_art.get():
            fr[sprite_field] = self.pose_art.get()
        if angle_field:
            try:
                fr[angle_field] = float(self.pose_angle.get())
            except Exception:
                pass
        if delta_field:
            try:
                fr[delta_field] = [float(self.pose_dx.get()), float(self.pose_dy.get())]
            except Exception:
                pass
        self.dirty_pose = True
        self.populate_animation_tree()
        self.refresh_preview()

    def update_anim_fields(self) -> None:
        anim = self.pose_anim.get()
        info = self.rig.animation_info(anim, self.pose_model.clean_for_save())
        self.anim_frame_count.set(int(info["frames"]))
        self.anim_duration_ms.set(int(info["duration_ms"]))
        self.populate_z_order()

    def apply_anim_settings(self) -> None:
        adata = self.pose_model.anim(self.pose_anim.get())
        adata["frames"] = int(self.anim_frame_count.get())
        adata["duration_ms"] = int(self.anim_duration_ms.get())
        self.dirty_pose = True
        self.populate_animation_tree()
        self.refresh_preview()

    def add_frame(self) -> None:
        self.anim_frame_count.set(int(self.anim_frame_count.get()) + 1)
        self.apply_anim_settings()

    def remove_frame(self) -> None:
        self.anim_frame_count.set(max(1, int(self.anim_frame_count.get()) - 1))
        self.apply_anim_settings()

    def populate_z_order(self) -> None:
        self.z_list.delete(0, "end")
        fr = self.current_frame_override()
        order = fr.get("z_order") or DEFAULT_Z_ORDER
        for role in order:
            self.z_list.insert("end", role)

    def move_z(self, delta: int) -> None:
        sel = self.z_list.curselection()
        if not sel:
            return
        i = sel[0]
        j = max(0, min(self.z_list.size() - 1, i + delta))
        if i == j:
            return
        order = [self.z_list.get(k) for k in range(self.z_list.size())]
        item = order.pop(i)
        order.insert(j, item)
        self.current_frame_override()["z_order"] = order
        self.dirty_pose = True
        self.populate_z_order()
        self.z_list.selection_set(j)
        self.refresh_preview()

    def reset_z_order(self) -> None:
        self.current_frame_override().pop("z_order", None)
        self.dirty_pose = True
        self.populate_z_order()
        self.refresh_preview()

    def navigate_connected_part(self) -> None:
        # Select the part instance that is connected to the currently selected
        # sprite+anchor in the current animation/frame.  This is the torso
        # shoulder -> arm workflow requested by the user.
        anim = self.pose_anim.get()
        idx = int(self.pose_frame.get())
        anchor = self.selected_anchor
        for comp in (self.find_frame_components(anim, idx) or []):
            conn = comp.get("connects_to") or {}
            if conn.get("sprite") == self.selected_sprite and conn.get("anchor") == anchor:
                iid = f"part:{anim}:{idx}:{comp.get('role')}"
                if self.tree.exists(iid):
                    self.tree.selection_set(iid)
                    self.tree.see(iid)
                    self.on_tree_select()
                    self.status.set(f"Navigated {self.selected_sprite}.{anchor} -> {comp.get('role')} ({comp.get('sprite')})")
                    return
        self.status.set(f"No component in {anim}[{idx}] is connected to {self.selected_sprite}.{anchor}")

    def find_frame_components(self, anim: str, idx: int) -> Optional[List[Dict[str, Any]]]:
        adata = (self.current_manifest.get("animations") or {}).get(anim) or {}
        for frame in adata.get("frames", []):
            if int(frame.get("index", -1)) == int(idx):
                return (frame.get("pose") or {}).get("components", [])
        return None

    # Preview ------------------------------------------------------------
    def preview_animations(self) -> List[str]:
        if self.preview_relevant_only.get() and self.selected_sprite:
            return relevant_animations(self.paths.job, self.metadata, self.pose_model.clean_for_save(), self.selected_sprite)
        return list(self.job.animations)

    def refresh_preview(self, force: bool = False) -> None:
        if self._preview_after and not force:
            try:
                self.root.after_cancel(self._preview_after)
            except Exception:
                pass
        if not force:
            self._preview_after = self.root.after(100, lambda: self.refresh_preview(force=True))
            return
        self._preview_after = None
        try:
            highlight = None
            if self.selected_instance:
                highlight = dict(self.selected_instance)
            img, manifest = build_preview(
                self.paths.job,
                self.metadata,
                self.pose_model.clean_for_save(),
                animations=self.preview_animations(),
                debug=self.preview_debug.get(),
                highlight=highlight,
                bg=self.preview_bg.get(),
            )
            self.current_manifest = manifest
            maxw = max(400, self.preview_canvas.winfo_width() or 1200)
            maxh = max(300, self.preview_canvas.winfo_height() or 800)
            show = img
            if self.preview_fit.get():
                scale = min(maxw / max(1, img.width), maxh / max(1, img.height), 1.0)
                if scale != 1.0:
                    show = img.resize((int(img.width * scale), int(img.height * scale)), Image.Resampling.LANCZOS)
            self._preview_photo = ImageTk.PhotoImage(show)
            self.preview_canvas.delete("all")
            self.preview_canvas.create_image(0, 0, image=self._preview_photo, anchor="nw")
            self.preview_canvas.configure(scrollregion=(0,0,show.width,show.height))
            self.status.set(f"Preview {show.width}x{show.height}; metadata={'dirty' if self.dirty_meta else 'clean'}, pose={'dirty' if self.dirty_pose else 'clean'}")
            self.render_anim_frame()
        except Exception as ex:
            self.status.set(f"Preview failed: {ex}")

    def render_anim_frame(self) -> None:
        anim = self.pose_anim.get()
        idx = self._play_index % max(1, self.rig.animation_info(anim, self.pose_model.clean_for_save())["frames"])
        try:
            rig = self.rig
            meta_path = write_temp_yaml(self.metadata)
            pose_path = write_temp_yaml(self.pose_model.clean_for_save())
            try:
                job = rig.RigJob.load(self.paths.job)
                job.metadata = meta_path
                job.pose_overrides = pose_path
                atlas = rig.ComponentAtlas(job.metadata, job.slices)
                asm = rig.RobotAssembler(atlas, job.render, pose_overrides=rig.load_pose_overrides(job.pose_overrides))
                img, _ = asm.render_frame(anim, idx, debug_parts=self.preview_debug.get())
            finally:
                meta_path.unlink(missing_ok=True); pose_path.unlink(missing_ok=True)
            show = composite_bg(img, self.preview_bg.get())
            scale = min((self.anim_canvas.winfo_width() or 320) / show.width, (self.anim_canvas.winfo_height() or 240) / show.height, 2.0)
            show = show.resize((int(show.width*scale), int(show.height*scale)), Image.Resampling.NEAREST)
            self._anim_photo = ImageTk.PhotoImage(show)
            self.anim_canvas.delete("all")
            self.anim_canvas.create_image(10, 10, image=self._anim_photo, anchor="nw")
            self.anim_canvas.create_text(10, show.height + 20, text=f"{anim} frame {idx}", fill="white", anchor="nw")
        except Exception as ex:
            self.anim_canvas.delete("all")
            self.anim_canvas.create_text(10, 10, text=f"animated preview failed: {ex}", fill="white", anchor="nw")

    def toggle_play(self) -> None:
        self._playing = not self._playing
        if self._playing:
            self._tick_play()

    def _tick_play(self) -> None:
        if not self._playing:
            return
        self._play_index += 1
        self.render_anim_frame()
        info = self.rig.animation_info(self.pose_anim.get(), self.pose_model.clean_for_save())
        self.root.after(int(info["duration_ms"]), self._tick_play)

    # Save ---------------------------------------------------------------
    def save(self) -> None:
        if self.dirty_meta:
            backup(self.paths.metadata)
            save_yaml(self.paths.metadata, self.metadata)
            self.dirty_meta = False
        if self.dirty_pose:
            backup(self.paths.pose_overrides)
            save_yaml(self.paths.pose_overrides, self.pose_model.clean_for_save())
            self.dirty_pose = False
        self.status.set(f"Saved {self.paths.metadata.name} and {self.paths.pose_overrides.name}")


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description="GUI editor for robot sprite anchors, instances, poses, z-order, and live previews.")
    p.add_argument("job", type=Path, nargs="?", default=Path("examples/robot_rig_job.yaml"), help="Rig job YAML")
    p.add_argument("--metadata", type=Path, default=None)
    p.add_argument("--slices", type=Path, default=None)
    p.add_argument("--pose-overrides", type=Path, default=None)
    p.add_argument("--zoom", type=int, default=5)
    p.add_argument("--render-preview", type=Path, default=None, help="Headless render of current preview and exit")
    p.add_argument("--anchor-report", type=Path, default=None, help="Headless JSON report of relevant instance connections and exit")
    return p


def write_anchor_report(paths: EditorPaths, output: Path) -> None:
    meta = load_yaml(paths.metadata)
    pose = load_yaml(paths.pose_overrides)
    img, manifest = build_preview(paths.job, meta, pose, animations=None, debug=False, highlight=None, bg="black")
    rows = []
    for anim, adata in (manifest.get("animations") or {}).items():
        for frame in adata.get("frames", []):
            for comp in (frame.get("pose") or {}).get("components", []):
                rows.append({"animation": anim, "frame": frame.get("index"), **comp})
    output.write_text(json.dumps({"components": rows}, indent=2), encoding="utf8")


def main_tk_legacy(argv: Optional[Sequence[str]] = None) -> int:
    """Run the original Tk editor path.

    The Tk editor remains available for emergency fallback and for code that
    imports this module as the core data/model layer, but the default command
    now delegates to the PySide6 implementation.
    """
    args = build_parser().parse_args(argv)
    paths = resolve_job_paths(args.job.resolve(), args.metadata, args.slices, args.pose_overrides)
    if args.anchor_report:
        write_anchor_report(paths, args.anchor_report.resolve())
        print(f"Wrote {args.anchor_report}")
        return 0
    if args.render_preview:
        meta = load_yaml(paths.metadata)
        pose = load_yaml(paths.pose_overrides)
        img, _ = build_preview(paths.job, meta, pose, animations=None, debug=False, highlight=None, bg="black")
        args.render_preview.parent.mkdir(parents=True, exist_ok=True)
        img.save(args.render_preview)
        print(f"Wrote {args.render_preview}")
        return 0
    try:
        import tkinter as tk
    except Exception as ex:  # pragma: no cover
        print(f"ERROR: tkinter is required for GUI: {ex}", file=sys.stderr)
        return 2
    root = tk.Tk()
    RigPoseEditor(root, paths, zoom=args.zoom)
    root.mainloop()
    return 0


def _load_pyside_main():
    """Load the PySide6 editor entrypoint from either package or script mode."""
    try:  # package / installed path
        from tools.rig_pose_editor_pyside import main as pyside_main
        return pyside_main
    except Exception:
        _path = Path(__file__).with_name("rig_pose_editor_pyside.py")
        _spec = importlib.util.spec_from_file_location("rig_pose_editor_pyside", _path)
        if _spec is None or _spec.loader is None:
            raise
        _module = importlib.util.module_from_spec(_spec)
        sys.modules[_spec.name] = _module
        _spec.loader.exec_module(_module)
        return _module.main


def main(argv: Optional[Sequence[str]] = None) -> int:
    """Default entrypoint: run the PySide6 editor.

    This preserves the user-facing command::

        python tools/rig_pose_editor.py examples/robot_rig_job.yaml

    while avoiding the stale Tk GUI path that caused the missing-callback crash.
    Use ``main_tk_legacy`` for the old Tk fallback.
    """
    raw = list(argv) if argv is not None else sys.argv[1:]
    return int(_load_pyside_main()(raw))


if __name__ == "__main__":
    raise SystemExit(main())
