#!/usr/bin/env python3
"""Interactive anchor editor for robot component metadata.

This is a small Tk/Pillow GUI for hand-tuning sprite-local pivots and anchors.
It intentionally edits metadata, not rendered poses:

    python tools/anchor_editor.py metadata/robot_components.refined.yaml \
        --slices output/slices \
        --rough-metadata metadata/robot_components.rough.yaml

Typical workflow:

1. Select a component on the left.
2. Select ``pivot`` or a named anchor.
3. Click on the enlarged sprite to move that point.
4. Save.  If ``--rough-metadata`` is supplied, equivalent rough-local points are
   updated so the green-screen refinement pass will preserve the manual edits.

The GUI does not try to solve animation poses.  It makes component-local anchors
visible and editable so the downstream joint-driven compositor has trustworthy
attachment points.
"""
from __future__ import annotations

import argparse
import json
import math
import shutil
import sys
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, Iterable, List, Mapping, Optional, Tuple

import yaml
from PIL import Image, ImageDraw, ImageTk

Point = Tuple[float, float]


ANCHOR_COLORS: Dict[str, Tuple[int, int, int]] = {
    "pivot": (255, 255, 255),
    "neck": (0, 196, 255),
    "face_socket": (0, 255, 255),
    "antenna_socket": (160, 80, 255),
    "shoulder_left": (0, 230, 80),
    "shoulder_right": (255, 80, 60),
    "hip_left": (165, 82, 255),
    "hip_right": (255, 150, 0),
    "wrist": (255, 240, 0),
    "hand": (255, 240, 0),
    "elbow": (255, 180, 0),
    "hip": (255, 210, 0),
    "knee": (255, 255, 0),
    "foot": (255, 255, 0),
    "ground": (255, 255, 0),
    "ankle": (255, 230, 0),
}


def color_for_name(name: str) -> Tuple[int, int, int]:
    if name in ANCHOR_COLORS:
        return ANCHOR_COLORS[name]
    # Stable deterministic color for any custom anchor.
    h = 0
    for ch in name:
        h = (h * 131 + ord(ch)) & 0xFFFFFFFF
    return (80 + (h & 127), 80 + ((h >> 8) & 127), 80 + ((h >> 16) & 127))


def load_yaml(path: Path) -> Dict[str, Any]:
    return yaml.safe_load(path.read_text(encoding="utf8")) or {}


def save_yaml(path: Path, data: Mapping[str, Any]) -> None:
    path.write_text(yaml.safe_dump(data, sort_keys=False, allow_unicode=True), encoding="utf8")


def backup_file(path: Path) -> Path:
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    backup = path.with_suffix(path.suffix + f".bak-{stamp}")
    shutil.copy2(path, backup)
    return backup


def as_point(value: Any, default: Point = (0.0, 0.0)) -> Point:
    if isinstance(value, (list, tuple)) and len(value) == 2:
        return (float(value[0]), float(value[1]))
    return default


def point_list(pt: Point) -> List[int]:
    return [int(round(pt[0])), int(round(pt[1]))]


def checkerboard(size: Tuple[int, int], cell: int = 8) -> Image.Image:
    w, h = size
    img = Image.new("RGBA", size, (210, 210, 210, 255))
    draw = ImageDraw.Draw(img)
    for y in range(0, h, cell):
        for x in range(0, w, cell):
            if ((x // cell) + (y // cell)) % 2 == 0:
                draw.rectangle([x, y, x + cell - 1, y + cell - 1], fill=(235, 235, 235, 255))
    return img


def composited_sprite_preview(sprite: Image.Image, bg: str = "checker") -> Image.Image:
    sprite = sprite.convert("RGBA")
    if bg == "black":
        base = Image.new("RGBA", sprite.size, (0, 0, 0, 255))
    elif bg == "white":
        base = Image.new("RGBA", sprite.size, (255, 255, 255, 255))
    else:
        base = checkerboard(sprite.size)
    base.alpha_composite(sprite)
    return base


@dataclass
class EditorPaths:
    metadata: Path
    slices: Path
    rough_metadata: Optional[Path] = None


class AnchorEditorApp:
    def __init__(self, root: "tk.Tk", paths: EditorPaths, *, zoom: int = 5, background: str = "checker", show_names: bool = False):
        # Tk imports are intentionally local so ``--help`` and tests do not need
        # a display server.
        import tkinter as tk
        from tkinter import ttk

        self.tk = tk
        self.ttk = ttk
        self.root = root
        self.paths = paths
        self.zoom = max(1, int(zoom))
        self.background = background
        self.show_names = tk.BooleanVar(value=show_names)
        self.unsaved = False
        self._dragging_anchor: Optional[str] = None
        self._photo = None
        self._status = tk.StringVar(value="Ready")
        self._selected_sprite = tk.StringVar(value="")
        self._selected_anchor = tk.StringVar(value="pivot")
        self._x_var = tk.StringVar(value="0")
        self._y_var = tk.StringVar(value="0")
        self._filter_var = tk.StringVar(value="")
        self._bg_var = tk.StringVar(value=background)
        self._zoom_var = tk.IntVar(value=self.zoom)

        self.metadata = load_yaml(paths.metadata)
        self.rough_metadata = load_yaml(paths.rough_metadata) if paths.rough_metadata else None
        self.sprites: Dict[str, Dict[str, Any]] = self.metadata.get("sprites", {})
        if not self.sprites:
            raise RuntimeError(f"No sprites found in {paths.metadata}")
        self.sprite_names = sorted(self.sprites.keys())

        self.root.title(f"Robot Anchor Editor - {paths.metadata.name}")
        self._build_ui()
        self._bind_events()
        self._populate_sprite_list()
        self.select_sprite(self.sprite_names[0])

    def _build_ui(self) -> None:
        tk = self.tk
        ttk = self.ttk
        root = self.root

        root.columnconfigure(1, weight=1)
        root.rowconfigure(0, weight=1)

        left = ttk.Frame(root, padding=6)
        left.grid(row=0, column=0, sticky="ns")
        left.rowconfigure(2, weight=1)

        ttk.Label(left, text="Sprite filter").grid(row=0, column=0, sticky="w")
        filt = ttk.Entry(left, textvariable=self._filter_var, width=28)
        filt.grid(row=1, column=0, sticky="ew", pady=(0, 6))

        self.sprite_list = tk.Listbox(left, exportselection=False, width=32, height=28)
        self.sprite_list.grid(row=2, column=0, sticky="nsew")
        scroll = ttk.Scrollbar(left, command=self.sprite_list.yview)
        scroll.grid(row=2, column=1, sticky="ns")
        self.sprite_list.configure(yscrollcommand=scroll.set)

        anchor_frame = ttk.LabelFrame(left, text="Anchors", padding=6)
        anchor_frame.grid(row=3, column=0, sticky="ew", pady=(8, 0))
        anchor_frame.columnconfigure(0, weight=1)
        self.anchor_list = tk.Listbox(anchor_frame, exportselection=False, height=8)
        self.anchor_list.grid(row=0, column=0, columnspan=4, sticky="ew")
        self.anchor_entry = ttk.Entry(anchor_frame, width=18)
        self.anchor_entry.grid(row=1, column=0, sticky="ew", pady=(4, 0))
        ttk.Button(anchor_frame, text="Add", command=self.add_anchor).grid(row=1, column=1, pady=(4, 0))
        ttk.Button(anchor_frame, text="Delete", command=self.delete_anchor).grid(row=1, column=2, pady=(4, 0))
        ttk.Button(anchor_frame, text="Pivot", command=lambda: self.select_anchor("pivot")).grid(row=1, column=3, pady=(4, 0))

        xy = ttk.Frame(left)
        xy.grid(row=4, column=0, sticky="ew", pady=(8, 0))
        ttk.Label(xy, text="x").grid(row=0, column=0)
        ttk.Entry(xy, textvariable=self._x_var, width=6).grid(row=0, column=1)
        ttk.Label(xy, text="y").grid(row=0, column=2, padx=(8, 0))
        ttk.Entry(xy, textvariable=self._y_var, width=6).grid(row=0, column=3)
        ttk.Button(xy, text="Apply", command=self.apply_xy).grid(row=0, column=4, padx=(8, 0))

        opts = ttk.Frame(left)
        opts.grid(row=5, column=0, sticky="ew", pady=(8, 0))
        ttk.Label(opts, text="Zoom").grid(row=0, column=0, sticky="w")
        ttk.Spinbox(opts, from_=1, to=12, textvariable=self._zoom_var, width=4, command=self.set_zoom_from_var).grid(row=0, column=1, sticky="w")
        ttk.Checkbutton(opts, text="show names", variable=self.show_names, command=self.redraw).grid(row=0, column=2, sticky="w", padx=(8, 0))
        ttk.Label(opts, text="bg").grid(row=1, column=0, sticky="w", pady=(4, 0))
        bg = ttk.Combobox(opts, textvariable=self._bg_var, width=10, values=["checker", "black", "white"], state="readonly")
        bg.grid(row=1, column=1, sticky="w", pady=(4, 0))

        buttons = ttk.Frame(left)
        buttons.grid(row=6, column=0, sticky="ew", pady=(10, 0))
        ttk.Button(buttons, text="Save", command=self.save).grid(row=0, column=0, sticky="ew")
        ttk.Button(buttons, text="Backup", command=self.backup).grid(row=0, column=1, sticky="ew", padx=(4, 0))
        ttk.Button(buttons, text="Reload", command=self.reload).grid(row=0, column=2, sticky="ew", padx=(4, 0))

        center = ttk.Frame(root, padding=6)
        center.grid(row=0, column=1, sticky="nsew")
        center.columnconfigure(0, weight=1)
        center.rowconfigure(0, weight=1)

        self.canvas = tk.Canvas(center, background="#202020", highlightthickness=0)
        self.canvas.grid(row=0, column=0, sticky="nsew")
        xscroll = ttk.Scrollbar(center, orient="horizontal", command=self.canvas.xview)
        yscroll = ttk.Scrollbar(center, orient="vertical", command=self.canvas.yview)
        xscroll.grid(row=1, column=0, sticky="ew")
        yscroll.grid(row=0, column=1, sticky="ns")
        self.canvas.configure(xscrollcommand=xscroll.set, yscrollcommand=yscroll.set)

        status = ttk.Label(root, textvariable=self._status, anchor="w", padding=(6, 3))
        status.grid(row=1, column=0, columnspan=2, sticky="ew")

    def _bind_events(self) -> None:
        self.sprite_list.bind("<<ListboxSelect>>", self._on_sprite_list_select)
        self.anchor_list.bind("<<ListboxSelect>>", self._on_anchor_list_select)
        self._filter_var.trace_add("write", lambda *_: self._populate_sprite_list())
        self._bg_var.trace_add("write", lambda *_: self.redraw())
        self.canvas.bind("<Button-1>", self.on_canvas_down)
        self.canvas.bind("<B1-Motion>", self.on_canvas_drag)
        self.canvas.bind("<ButtonRelease-1>", self.on_canvas_up)
        self.root.bind("<Control-s>", lambda event: self.save())
        self.root.bind("<Left>", lambda event: self.nudge(-1, 0))
        self.root.bind("<Right>", lambda event: self.nudge(1, 0))
        self.root.bind("<Up>", lambda event: self.nudge(0, -1))
        self.root.bind("<Down>", lambda event: self.nudge(0, 1))
        self.root.protocol("WM_DELETE_WINDOW", self.on_close)

    def _populate_sprite_list(self) -> None:
        filt = self._filter_var.get().strip().lower()
        names = [n for n in self.sprite_names if filt in n.lower()]
        self.sprite_list.delete(0, self.tk.END)
        for name in names:
            self.sprite_list.insert(self.tk.END, name)
        if self._selected_sprite.get() in names:
            self.sprite_list.selection_clear(0, self.tk.END)
            idx = names.index(self._selected_sprite.get())
            self.sprite_list.selection_set(idx)
            self.sprite_list.see(idx)

    def _on_sprite_list_select(self, event=None) -> None:
        sel = self.sprite_list.curselection()
        if sel:
            self.select_sprite(self.sprite_list.get(sel[0]))

    def _on_anchor_list_select(self, event=None) -> None:
        sel = self.anchor_list.curselection()
        if sel:
            self.select_anchor(self.anchor_list.get(sel[0]))

    def _current_sprite_name(self) -> str:
        return self._selected_sprite.get()

    def _current_sprite(self) -> Dict[str, Any]:
        return self.sprites[self._current_sprite_name()]

    def _sprite_path(self, name: str) -> Path:
        return self.paths.slices / f"{name}.png"

    def _load_image(self, name: str) -> Image.Image:
        path = self._sprite_path(name)
        if path.exists():
            return Image.open(path).convert("RGBA")
        # Fallback: crop from atlas image if slices are missing.
        sprite = self.sprites[name]
        img_file = self.metadata.get("image", {}).get("file")
        if not img_file:
            raise FileNotFoundError(f"Missing slice {path} and no image.file in metadata")
        img_path = (self.paths.metadata.parent / img_file).resolve()
        atlas = Image.open(img_path).convert("RGBA")
        x, y, w, h = sprite["rect"]
        return atlas.crop((x, y, x + w, y + h))

    def select_sprite(self, name: str) -> None:
        if name not in self.sprites:
            return
        self._selected_sprite.set(name)
        self._populate_anchor_list()
        current = self._selected_anchor.get()
        anchors = self._anchor_names()
        if current not in anchors:
            self._selected_anchor.set(anchors[0])
        self._sync_xy_vars()
        self.redraw()
        self._status.set(f"Selected sprite {name}")

    def _anchor_names(self) -> List[str]:
        sprite = self._current_sprite()
        names = ["pivot"]
        names.extend(sorted((sprite.get("anchors") or {}).keys()))
        return names

    def _populate_anchor_list(self) -> None:
        self.anchor_list.delete(0, self.tk.END)
        for name in self._anchor_names():
            self.anchor_list.insert(self.tk.END, name)
        self._select_anchor_list_item(self._selected_anchor.get())

    def _select_anchor_list_item(self, name: str) -> None:
        names = self._anchor_names()
        if name in names:
            idx = names.index(name)
            self.anchor_list.selection_clear(0, self.tk.END)
            self.anchor_list.selection_set(idx)
            self.anchor_list.see(idx)

    def select_anchor(self, name: str) -> None:
        if name not in self._anchor_names():
            return
        self._selected_anchor.set(name)
        self._select_anchor_list_item(name)
        self._sync_xy_vars()
        self.redraw()

    def get_anchor_point(self, name: str) -> Point:
        sprite = self._current_sprite()
        if name == "pivot":
            return as_point(sprite.get("pivot"), (0, 0))
        return as_point((sprite.get("anchors") or {}).get(name), (0, 0))

    def set_anchor_point(self, name: str, pt: Point) -> None:
        sprite = self._current_sprite()
        img = self._load_image(self._current_sprite_name())
        x = max(0, min(img.width - 1, pt[0]))
        y = max(0, min(img.height - 1, pt[1]))
        if name == "pivot":
            sprite["pivot"] = point_list((x, y))
        else:
            anchors = sprite.setdefault("anchors", {})
            anchors[name] = point_list((x, y))
        self.unsaved = True
        self._sync_xy_vars()

    def _sync_xy_vars(self) -> None:
        pt = self.get_anchor_point(self._selected_anchor.get())
        self._x_var.set(str(int(round(pt[0]))))
        self._y_var.set(str(int(round(pt[1]))))

    def canvas_to_image(self, event) -> Point:
        x = self.canvas.canvasx(event.x) / self.zoom
        y = self.canvas.canvasy(event.y) / self.zoom
        return (x, y)

    def image_to_canvas(self, pt: Point) -> Point:
        return (pt[0] * self.zoom, pt[1] * self.zoom)

    def nearest_anchor(self, pt: Point, max_dist_px: float = 10.0) -> Optional[str]:
        best = None
        best_d = max_dist_px / self.zoom
        for name in self._anchor_names():
            ax, ay = self.get_anchor_point(name)
            d = math.hypot(pt[0] - ax, pt[1] - ay)
            if d < best_d:
                best = name
                best_d = d
        return best

    def on_canvas_down(self, event) -> None:
        pt = self.canvas_to_image(event)
        nearby = self.nearest_anchor(pt)
        if nearby is not None:
            self.select_anchor(nearby)
        name = self._selected_anchor.get()
        self._dragging_anchor = name
        self.set_anchor_point(name, pt)
        self.redraw()

    def on_canvas_drag(self, event) -> None:
        if self._dragging_anchor:
            self.set_anchor_point(self._dragging_anchor, self.canvas_to_image(event))
            self.redraw()

    def on_canvas_up(self, event) -> None:
        self._dragging_anchor = None

    def apply_xy(self) -> None:
        try:
            x = float(self._x_var.get())
            y = float(self._y_var.get())
        except ValueError:
            self._status.set("Invalid x/y")
            return
        self.set_anchor_point(self._selected_anchor.get(), (x, y))
        self.redraw()

    def nudge(self, dx: int, dy: int) -> None:
        name = self._selected_anchor.get()
        x, y = self.get_anchor_point(name)
        self.set_anchor_point(name, (x + dx, y + dy))
        self.redraw()

    def set_zoom_from_var(self) -> None:
        self.zoom = max(1, int(self._zoom_var.get()))
        self.redraw()

    def add_anchor(self) -> None:
        name = self.anchor_entry.get().strip()
        if not name or name == "pivot":
            self._status.set("Enter a non-empty anchor name other than pivot")
            return
        sprite = self._current_sprite()
        anchors = sprite.setdefault("anchors", {})
        if name not in anchors:
            img = self._load_image(self._current_sprite_name())
            anchors[name] = [img.width // 2, img.height // 2]
            self.unsaved = True
        self._populate_anchor_list()
        self.select_anchor(name)

    def delete_anchor(self) -> None:
        name = self._selected_anchor.get()
        if name == "pivot":
            self._status.set("Cannot delete pivot")
            return
        anchors = self._current_sprite().setdefault("anchors", {})
        if name in anchors:
            del anchors[name]
            self.unsaved = True
            self._selected_anchor.set("pivot")
            self._populate_anchor_list()
            self.redraw()

    def redraw(self) -> None:
        name = self._current_sprite_name()
        if not name:
            return
        self.background = self._bg_var.get()
        img = composited_sprite_preview(self._load_image(name), self.background)
        scaled = img.resize((img.width * self.zoom, img.height * self.zoom), Image.Resampling.NEAREST)
        # Draw anchors after scaling for crisp markers.
        draw = ImageDraw.Draw(scaled)
        selected = self._selected_anchor.get()
        for aname in self._anchor_names():
            pt = self.get_anchor_point(aname)
            x, y = self.image_to_canvas(pt)
            col = color_for_name(aname)
            r = 6 if aname == selected else 4
            fill = (*col, 255)
            outline = (255, 255, 255, 255) if aname != "pivot" else (0, 0, 0, 255)
            # Crosshair + small circle; no text unless explicitly enabled.
            draw.line((x - r * 2, y, x + r * 2, y), fill=fill, width=2)
            draw.line((x, y - r * 2, x, y + r * 2), fill=fill, width=2)
            draw.ellipse((x - r, y - r, x + r, y + r), outline=outline, width=2)
            if aname == selected:
                draw.ellipse((x - r - 4, y - r - 4, x + r + 4, y + r + 4), outline=(255, 255, 255, 255), width=2)
            if self.show_names.get():
                draw.text((x + r + 3, y - r), aname, fill=(255, 255, 255, 255))
        self._photo = ImageTk.PhotoImage(scaled)
        self.canvas.delete("all")
        self.canvas.create_image(0, 0, image=self._photo, anchor="nw")
        self.canvas.configure(scrollregion=(0, 0, scaled.width, scaled.height))

    def backup(self) -> None:
        b1 = backup_file(self.paths.metadata)
        msg = f"Backed up {b1.name}"
        if self.paths.rough_metadata and self.paths.rough_metadata.exists():
            b2 = backup_file(self.paths.rough_metadata)
            msg += f" and {b2.name}"
        self._status.set(msg)

    def _sync_to_rough(self) -> None:
        if self.rough_metadata is None or self.paths.rough_metadata is None:
            return
        rough_sprites = self.rough_metadata.setdefault("sprites", {})
        for sid, refined_sprite in self.sprites.items():
            rough_sprite = rough_sprites.get(sid)
            if not rough_sprite:
                continue
            refined_rect = refined_sprite.get("rect")
            rough_rect = rough_sprite.get("rect")
            if not (isinstance(refined_rect, list) and len(refined_rect) == 4 and isinstance(rough_rect, list) and len(rough_rect) == 4):
                continue
            rx, ry = refined_rect[0], refined_rect[1]
            ox, oy = rough_rect[0], rough_rect[1]

            def refined_local_to_rough_local(pt: Any) -> List[int]:
                p = as_point(pt)
                # The refinement step uses: new_local = old_rect + old_local - new_rect.
                # Invert that relation so the next refine preserves the edited refined point.
                return point_list((p[0] + rx - ox, p[1] + ry - oy))

            if "pivot" in refined_sprite:
                rough_sprite["pivot"] = refined_local_to_rough_local(refined_sprite["pivot"])
            if "anchors" in refined_sprite:
                rough_anchors = rough_sprite.setdefault("anchors", {})
                for aname, pt in refined_sprite.get("anchors", {}).items():
                    rough_anchors[aname] = refined_local_to_rough_local(pt)
        self.rough_metadata["metadata_quality"] = self.rough_metadata.get("metadata_quality", {})
        notes = self.rough_metadata["metadata_quality"].setdefault("notes", [])
        note = "anchors manually synced from refined metadata by tools/anchor_editor.py"
        if note not in notes:
            notes.append(note)

    def save(self) -> None:
        if not self.unsaved:
            self._status.set("No anchor changes to save")
            return
        backup_file(self.paths.metadata)
        self.metadata["metadata_quality"] = self.metadata.get("metadata_quality", {})
        self.metadata["metadata_quality"]["anchor_precision"] = "manually_adjusted_with_anchor_editor"
        save_yaml(self.paths.metadata, self.metadata)
        msg = f"Saved {self.paths.metadata}"
        if self.paths.rough_metadata and self.rough_metadata is not None:
            backup_file(self.paths.rough_metadata)
            self._sync_to_rough()
            save_yaml(self.paths.rough_metadata, self.rough_metadata)
            msg += f" and synced {self.paths.rough_metadata}"
        self.unsaved = False
        self._status.set(msg)

    def reload(self) -> None:
        if self.unsaved:
            self._status.set("Unsaved changes present; save or restart to reload safely")
            return
        self.metadata = load_yaml(self.paths.metadata)
        self.rough_metadata = load_yaml(self.paths.rough_metadata) if self.paths.rough_metadata else None
        self.sprites = self.metadata.get("sprites", {})
        self.sprite_names = sorted(self.sprites.keys())
        self._populate_sprite_list()
        self.select_sprite(self._current_sprite_name() if self._current_sprite_name() in self.sprites else self.sprite_names[0])

    def on_close(self) -> None:
        if self.unsaved:
            # Keep this minimal: avoid modal prompts when running from odd desktop
            # environments.  Users can hit Save explicitly or close again after
            # reading the status.
            self._status.set("Unsaved changes. Press Save first, or close again after saving.")
            self.unsaved = False
            return
        self.root.destroy()


def write_anchor_report(metadata_path: Path, slices_path: Path, output: Path, sprites: Optional[List[str]] = None) -> None:
    """Write a JSON summary of sprite sizes and anchors for non-GUI inspection."""
    meta = load_yaml(metadata_path)
    rows = []
    for sid, sprite in sorted((meta.get("sprites") or {}).items()):
        if sprites and sid not in sprites:
            continue
        image_path = slices_path / f"{sid}.png"
        size = None
        if image_path.exists():
            with Image.open(image_path) as img:
                size = list(img.size)
        rows.append({
            "sprite": sid,
            "image": str(image_path),
            "size": size,
            "rect": sprite.get("rect"),
            "pivot": sprite.get("pivot"),
            "anchors": sprite.get("anchors", {}),
        })
    output.write_text(json.dumps({"metadata": str(metadata_path), "sprites": rows}, indent=2), encoding="utf8")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Manually place robot component pivots and anchors in a Tk GUI.")
    parser.add_argument("metadata", type=Path, help="Metadata YAML to edit, usually metadata/robot_components.refined.yaml")
    parser.add_argument("--slices", type=Path, default=Path("output/slices"), help="Directory containing extracted component PNGs")
    parser.add_argument("--rough-metadata", type=Path, default=None, help="Optional rough YAML to keep in sync when saving")
    parser.add_argument("--zoom", type=int, default=5, help="Initial integer zoom factor")
    parser.add_argument("--background", choices=["checker", "black", "white"], default="checker", help="Preview background")
    parser.add_argument("--show-names", action="store_true", help="Show anchor names on the canvas")
    parser.add_argument("--anchor-report", type=Path, default=None, help="Write a JSON anchor report and exit instead of opening the GUI")
    parser.add_argument("--sprites", nargs="*", default=None, help="Sprite ids to include in --anchor-report")
    return parser


def main(argv: Optional[List[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    metadata = args.metadata.resolve()
    slices = args.slices.resolve()
    rough = args.rough_metadata.resolve() if args.rough_metadata else None
    if args.anchor_report:
        write_anchor_report(metadata, slices, args.anchor_report.resolve(), args.sprites)
        print(f"Wrote {args.anchor_report}")
        return 0
    # Import tkinter only when launching the GUI; this keeps CI/headless checks usable.
    try:
        import tkinter as tk
    except Exception as ex:  # pragma: no cover - environment dependent
        print(f"ERROR: tkinter is required for the GUI: {ex}", file=sys.stderr)
        return 2
    root = tk.Tk()
    AnchorEditorApp(root, EditorPaths(metadata=metadata, slices=slices, rough_metadata=rough), zoom=args.zoom, background=args.background, show_names=args.show_names)
    root.mainloop()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
