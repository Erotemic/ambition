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
4. Use ``Use selected as pivot`` or the ``pivot follows`` dropdown when the
   component pivot should be exactly the same location as a named anchor.
5. Watch the larger live spritesheet preview on the right update from the current unsaved metadata.
6. Save.  If ``--rough-metadata`` is supplied, equivalent rough-local points are
   updated so the green-screen refinement pass will preserve the manual edits.

The GUI does not try to solve animation poses.  It makes component-local anchors
visible and editable so the downstream joint-driven compositor has trustworthy
attachment points, and it renders a configured spritesheet after edits so bad
anchor choices are visible immediately.
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
from dataclasses import dataclass, field
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
    path.write_text(
        yaml.safe_dump(data, sort_keys=False, allow_unicode=True), encoding="utf8"
    )


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
                draw.rectangle(
                    [x, y, x + cell - 1, y + cell - 1], fill=(235, 235, 235, 255)
                )
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
class PreviewConfig:
    """Live-preview rendering settings for the editor."""

    config: Optional[Path] = None
    enabled: bool = True
    # The preview pane is intentionally large because a row-oriented sheet can
    # be much wider than it is tall.  The canvas has scrollbars, so these are
    # display raster limits, not hard widget sizes.
    max_width: int = 1400
    max_height: int = 820
    debounce_ms: int = 80
    live_unsaved: bool = True
    fit_to_view: bool = True
    background: str = "black"


def infer_preview_config(
    metadata_path: Path, explicit: Optional[Path] = None
) -> Optional[Path]:
    """Find the default rig job to render inside the GUI."""
    if explicit is not None:
        return explicit.resolve()
    # Most repo layouts are: metadata/<file>.yaml and examples/robot_rig_job.yaml.
    root = metadata_path.resolve().parent.parent
    candidates = [
        root / "examples" / "robot_rig_job.yaml",
        Path.cwd() / "examples" / "robot_rig_job.yaml",
    ]
    for cand in candidates:
        if cand.exists():
            return cand.resolve()
    return None


def load_robot_rig_sheet_module():
    """Import the sibling compositor without requiring package installation."""
    tool_path = Path(__file__).with_name("robot_rig_sheet.py")
    spec = importlib.util.spec_from_file_location(
        "robot_rig_sheet_for_anchor_editor", tool_path
    )
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Cannot import robot_rig_sheet.py from {tool_path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def preview_fit_image(
    img: Image.Image,
    max_width: int,
    max_height: int,
    bg: str = "black",
    *,
    fit_to_view: bool = True,
    allow_upscale: bool = False,
) -> Image.Image:
    """Composite a live preview image for Tk display.

    When ``fit_to_view`` is true, the rendered sheet is scaled down to the
    preview raster limit.  When false, the preview is kept at native size and
    the preview canvas scrollbars expose the whole image.
    """
    img = img.convert("RGBA")
    if bg == "checker":
        base = checkerboard(img.size, cell=12)
    elif bg == "white":
        base = Image.new("RGBA", img.size, (255, 255, 255, 255))
    else:
        base = Image.new("RGBA", img.size, (0, 0, 0, 255))
    base.alpha_composite(img)
    if not fit_to_view:
        return base
    scale = min(max_width / max(1, base.width), max_height / max(1, base.height))
    if not allow_upscale:
        scale = min(scale, 1.0)
    out_size = (
        max(1, int(round(base.width * scale))),
        max(1, int(round(base.height * scale))),
    )
    if out_size == base.size:
        return base
    return base.resize(out_size, Image.Resampling.LANCZOS)


def render_preview_image(
    metadata: Mapping[str, Any],
    preview_config: Path,
    *,
    max_width: int = 1400,
    max_height: int = 820,
    bg: str = "black",
    fit_to_view: bool = True,
) -> Image.Image:
    """Render the configured sprite sheet using unsaved in-memory metadata.

    The editor writes a temporary metadata file from the in-memory metadata
    dictionary.  This means the preview reflects current GUI edits even before
    the YAML is saved to disk.
    """
    rig = load_robot_rig_sheet_module()
    metadata_snapshot = copy.deepcopy(dict(metadata))
    with tempfile.NamedTemporaryFile(
        "w", suffix=".yaml", encoding="utf8", delete=False
    ) as temp:
        temp_path = Path(temp.name)
        yaml.safe_dump(metadata_snapshot, temp, sort_keys=False, allow_unicode=True)
        temp.flush()
    try:
        job = rig.RigJob.load(preview_config)
        job.metadata = temp_path
        sheet, _manifest = rig.build_spritesheet(job)
        return preview_fit_image(
            sheet,
            max_width=max_width,
            max_height=max_height,
            bg=bg,
            fit_to_view=fit_to_view,
        )
    finally:
        try:
            temp_path.unlink()
        except OSError:
            pass


@dataclass
class EditorPaths:
    metadata: Path
    slices: Path
    rough_metadata: Optional[Path] = None


class AnchorEditorApp:
    def __init__(
        self,
        root: "tk.Tk",
        paths: EditorPaths,
        *,
        zoom: int = 5,
        background: str = "checker",
        show_names: bool = False,
        preview: Optional[PreviewConfig] = None,
    ):
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
        self._updating_pivot_combo = False
        self._photo = None
        self._preview_photo = None
        self._preview_after_id = None
        self.preview = preview or PreviewConfig(
            config=infer_preview_config(paths.metadata)
        )
        self._status = tk.StringVar(value="Ready")
        self._selected_sprite = tk.StringVar(value="")
        self._selected_anchor = tk.StringVar(value="pivot")
        self._x_var = tk.StringVar(value="0")
        self._y_var = tk.StringVar(value="0")
        self._filter_var = tk.StringVar(value="")
        self._bg_var = tk.StringVar(value=background)
        self._zoom_var = tk.IntVar(value=self.zoom)
        self._pivot_source_var = tk.StringVar(value="__custom__")
        self._preview_status = tk.StringVar(value="Preview not rendered yet")
        self._preview_enabled_var = tk.BooleanVar(value=self.preview.enabled)
        self._preview_fit_var = tk.BooleanVar(value=self.preview.fit_to_view)
        self._preview_live_var = tk.BooleanVar(value=self.preview.live_unsaved)
        self._preview_bg_var = tk.StringVar(value=self.preview.background)
        self._preview_width_var = tk.IntVar(value=self.preview.max_width)
        self._preview_height_var = tk.IntVar(value=self.preview.max_height)

        self.metadata = load_yaml(paths.metadata)
        self.rough_metadata = (
            load_yaml(paths.rough_metadata) if paths.rough_metadata else None
        )
        self.sprites: Dict[str, Dict[str, Any]] = self.metadata.get("sprites", {})
        if not self.sprites:
            raise RuntimeError(f"No sprites found in {paths.metadata}")
        self.sprite_names = sorted(self.sprites.keys())

        self.root.title(f"Robot Anchor Editor - {paths.metadata.name}")
        self._build_ui()
        self._bind_events()
        self._populate_sprite_list()
        self.select_sprite(self.sprite_names[0])
        self.schedule_preview_update(force=True)

    def _build_ui(self) -> None:
        tk = self.tk
        ttk = self.ttk
        root = self.root

        root.geometry("1900x1050")
        root.minsize(1300, 760)
        root.columnconfigure(1, weight=1)
        root.columnconfigure(2, weight=3)
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
        ttk.Button(anchor_frame, text="Add", command=self.add_anchor).grid(
            row=1, column=1, pady=(4, 0)
        )
        ttk.Button(anchor_frame, text="Delete", command=self.delete_anchor).grid(
            row=1, column=2, pady=(4, 0)
        )
        ttk.Button(
            anchor_frame,
            text="Select pivot",
            command=lambda: self.select_anchor("pivot"),
        ).grid(row=1, column=3, pady=(4, 0))
        ttk.Button(
            anchor_frame,
            text="Use selected as pivot",
            command=self.use_selected_as_pivot,
        ).grid(row=2, column=0, columnspan=2, sticky="ew", pady=(4, 0))
        ttk.Label(anchor_frame, text="pivot follows").grid(
            row=2, column=2, sticky="e", pady=(4, 0)
        )
        self.pivot_combo = ttk.Combobox(
            anchor_frame,
            textvariable=self._pivot_source_var,
            width=14,
            state="readonly",
        )
        self.pivot_combo.grid(row=2, column=3, sticky="ew", pady=(4, 0))

        xy = ttk.Frame(left)
        xy.grid(row=4, column=0, sticky="ew", pady=(8, 0))
        ttk.Label(xy, text="x").grid(row=0, column=0)
        ttk.Entry(xy, textvariable=self._x_var, width=6).grid(row=0, column=1)
        ttk.Label(xy, text="y").grid(row=0, column=2, padx=(8, 0))
        ttk.Entry(xy, textvariable=self._y_var, width=6).grid(row=0, column=3)
        ttk.Button(xy, text="Apply", command=self.apply_xy).grid(
            row=0, column=4, padx=(8, 0)
        )

        opts = ttk.Frame(left)
        opts.grid(row=5, column=0, sticky="ew", pady=(8, 0))
        ttk.Label(opts, text="Zoom").grid(row=0, column=0, sticky="w")
        ttk.Spinbox(
            opts,
            from_=1,
            to=12,
            textvariable=self._zoom_var,
            width=4,
            command=self.set_zoom_from_var,
        ).grid(row=0, column=1, sticky="w")
        ttk.Checkbutton(
            opts, text="show names", variable=self.show_names, command=self.redraw
        ).grid(row=0, column=2, sticky="w", padx=(8, 0))
        ttk.Label(opts, text="bg").grid(row=1, column=0, sticky="w", pady=(4, 0))
        bg = ttk.Combobox(
            opts,
            textvariable=self._bg_var,
            width=10,
            values=["checker", "black", "white"],
            state="readonly",
        )
        bg.grid(row=1, column=1, sticky="w", pady=(4, 0))

        buttons = ttk.Frame(left)
        buttons.grid(row=6, column=0, sticky="ew", pady=(10, 0))
        ttk.Button(buttons, text="Save", command=self.save).grid(
            row=0, column=0, sticky="ew"
        )
        ttk.Button(buttons, text="Backup", command=self.backup).grid(
            row=0, column=1, sticky="ew", padx=(4, 0)
        )
        ttk.Button(buttons, text="Reload", command=self.reload).grid(
            row=0, column=2, sticky="ew", padx=(4, 0)
        )

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

        right = ttk.LabelFrame(
            root,
            text="Live spritesheet preview (unsaved in-memory metadata)",
            padding=6,
        )
        right.grid(row=0, column=2, sticky="nsew", padx=(0, 6), pady=6)
        right.columnconfigure(0, weight=1)
        right.rowconfigure(2, weight=1)

        preview_label = ttk.Label(right, textvariable=self._preview_status, anchor="w")
        preview_label.grid(row=0, column=0, sticky="ew")

        preview_opts = ttk.Frame(right)
        preview_opts.grid(row=1, column=0, sticky="ew", pady=(4, 6))
        ttk.Checkbutton(
            preview_opts,
            text="live unsaved",
            variable=self._preview_live_var,
            command=self._on_preview_controls_changed,
        ).grid(row=0, column=0, sticky="w")
        ttk.Checkbutton(
            preview_opts,
            text="enable",
            variable=self._preview_enabled_var,
            command=self._on_preview_controls_changed,
        ).grid(row=0, column=1, sticky="w", padx=(8, 0))
        ttk.Checkbutton(
            preview_opts,
            text="fit",
            variable=self._preview_fit_var,
            command=self._on_preview_controls_changed,
        ).grid(row=0, column=2, sticky="w", padx=(8, 0))
        ttk.Label(preview_opts, text="max w").grid(
            row=0, column=3, sticky="e", padx=(12, 2)
        )
        ttk.Spinbox(
            preview_opts,
            from_=320,
            to=4096,
            increment=80,
            textvariable=self._preview_width_var,
            width=6,
            command=self._on_preview_controls_changed,
        ).grid(row=0, column=4, sticky="w")
        ttk.Label(preview_opts, text="max h").grid(
            row=0, column=5, sticky="e", padx=(8, 2)
        )
        ttk.Spinbox(
            preview_opts,
            from_=160,
            to=4096,
            increment=80,
            textvariable=self._preview_height_var,
            width=6,
            command=self._on_preview_controls_changed,
        ).grid(row=0, column=6, sticky="w")
        ttk.Label(preview_opts, text="bg").grid(
            row=0, column=7, sticky="e", padx=(8, 2)
        )
        ttk.Combobox(
            preview_opts,
            textvariable=self._preview_bg_var,
            width=8,
            values=["black", "checker", "white"],
            state="readonly",
        ).grid(row=0, column=8, sticky="w")
        ttk.Button(
            preview_opts,
            text="Render now",
            command=lambda: self.update_preview(force=True),
        ).grid(row=0, column=9, sticky="e", padx=(12, 0))

        preview_area = ttk.Frame(right)
        preview_area.grid(row=2, column=0, sticky="nsew")
        preview_area.columnconfigure(0, weight=1)
        preview_area.rowconfigure(0, weight=1)
        self.preview_canvas = tk.Canvas(
            right, background="#101010", highlightthickness=0, width=1100, height=720
        )
        self.preview_canvas.grid(row=2, column=0, sticky="nsew")
        p_xscroll = ttk.Scrollbar(
            right, orient="horizontal", command=self.preview_canvas.xview
        )
        p_yscroll = ttk.Scrollbar(
            right, orient="vertical", command=self.preview_canvas.yview
        )
        p_xscroll.grid(row=3, column=0, sticky="ew")
        p_yscroll.grid(row=2, column=1, sticky="ns")
        self.preview_canvas.configure(
            xscrollcommand=p_xscroll.set, yscrollcommand=p_yscroll.set
        )

        status = ttk.Label(root, textvariable=self._status, anchor="w", padding=(6, 3))
        status.grid(row=1, column=0, columnspan=3, sticky="ew")

    def _on_preview_controls_changed(self) -> None:
        self.preview.enabled = bool(self._preview_enabled_var.get())
        self.preview.live_unsaved = bool(self._preview_live_var.get())
        self.preview.fit_to_view = bool(self._preview_fit_var.get())
        try:
            self.preview.max_width = max(1, int(self._preview_width_var.get()))
            self.preview.max_height = max(1, int(self._preview_height_var.get()))
        except Exception:
            pass
        self.preview.background = self._preview_bg_var.get()
        self.schedule_preview_update(force=True)

    def _bind_events(self) -> None:
        self.sprite_list.bind("<<ListboxSelect>>", self._on_sprite_list_select)
        self.anchor_list.bind("<<ListboxSelect>>", self._on_anchor_list_select)
        self._filter_var.trace_add("write", lambda *_: self._populate_sprite_list())
        self._bg_var.trace_add("write", lambda *_: self.redraw())
        self._pivot_source_var.trace_add("write", self._on_pivot_source_changed)
        self._preview_bg_var.trace_add(
            "write", lambda *_: self._on_preview_controls_changed()
        )
        self.canvas.bind("<Button-1>", self.on_canvas_down)
        self.canvas.bind("<B1-Motion>", self.on_canvas_drag)
        self.canvas.bind("<ButtonRelease-1>", self.on_canvas_up)
        self.root.bind("<Control-s>", lambda event: self.save())
        self.root.bind("<Control-r>", lambda event: self.update_preview(force=True))
        self.root.bind("<Control-p>", lambda event: self.use_selected_as_pivot())
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
            raise FileNotFoundError(
                f"Missing slice {path} and no image.file in metadata"
            )
        img_path = (self.paths.metadata.parent / img_file).resolve()
        atlas = Image.open(img_path).convert("RGBA")
        x, y, w, h = sprite["rect"]
        return atlas.crop((x, y, x + w, y + h))

    def select_sprite(self, name: str) -> None:
        if name not in self.sprites:
            return
        self._selected_sprite.set(name)
        self._populate_anchor_list()
        self._populate_pivot_combo()
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

    def _anchor_only_names(self) -> List[str]:
        return sorted((self._current_sprite().get("anchors") or {}).keys())

    def _pivot_source(self) -> str:
        sprite = self._current_sprite()
        value = str(sprite.get("pivot_anchor") or "__custom__")
        if value != "__custom__" and value not in (sprite.get("anchors") or {}):
            # Keep old metadata usable if an anchor was deleted outside the GUI.
            sprite.pop("pivot_anchor", None)
            value = "__custom__"
        return value

    def _sync_pivot_from_alias(self, sprite: Optional[Dict[str, Any]] = None) -> None:
        sprite = sprite if sprite is not None else self._current_sprite()
        alias = sprite.get("pivot_anchor")
        anchors = sprite.get("anchors") or {}
        if alias and alias in anchors:
            sprite["pivot"] = point_list(as_point(anchors[alias]))

    def _sync_all_pivots_from_aliases(self) -> None:
        for sprite in self.sprites.values():
            self._sync_pivot_from_alias(sprite)

    def _populate_pivot_combo(self) -> None:
        values = ["__custom__"] + self._anchor_only_names()
        if hasattr(self, "pivot_combo"):
            self.pivot_combo.configure(values=values)
        current = self._pivot_source()
        self._updating_pivot_combo = True
        try:
            self._pivot_source_var.set(current)
        finally:
            self._updating_pivot_combo = False

    def _on_pivot_source_changed(self, *_args) -> None:
        # This trace may fire during initialization before sprites are selected.
        if self._updating_pivot_combo:
            return
        if (
            not self._current_sprite_name()
            or self._current_sprite_name() not in self.sprites
        ):
            return
        src = self._pivot_source_var.get()
        self.set_pivot_source(src)

    def set_pivot_source(self, src: str) -> None:
        sprite = self._current_sprite()
        anchors = sprite.setdefault("anchors", {})
        if src == "__custom__":
            if "pivot_anchor" in sprite:
                sprite.pop("pivot_anchor", None)
                self.unsaved = True
                self._status.set("Pivot is now an independent custom point")
        elif src in anchors:
            old_alias = sprite.get("pivot_anchor")
            old_pivot = (
                point_list(as_point(sprite.get("pivot"))) if "pivot" in sprite else None
            )
            new_pivot = point_list(as_point(anchors[src]))
            sprite["pivot_anchor"] = src
            sprite["pivot"] = new_pivot
            if old_alias != src or old_pivot != new_pivot:
                self.unsaved = True
            self._status.set(f"Pivot now follows anchor {src!r}")
        else:
            self._status.set(f"Cannot set pivot to missing anchor {src!r}")
            return
        self._sync_xy_vars()
        self.redraw()
        self.schedule_preview_update()

    def use_selected_as_pivot(self) -> None:
        name = self._selected_anchor.get()
        if name == "pivot":
            self._status.set("Select a named anchor first, then make pivot follow it")
            return
        self._pivot_source_var.set(name)
        self.set_pivot_source(name)

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
        anchors = sprite.get("anchors") or {}
        if name == "pivot":
            alias = sprite.get("pivot_anchor")
            if alias and alias in anchors:
                return as_point(anchors[alias], (0, 0))
            return as_point(sprite.get("pivot"), (0, 0))
        return as_point(anchors.get(name), (0, 0))

    def set_anchor_point(self, name: str, pt: Point) -> None:
        sprite = self._current_sprite()
        img = self._load_image(self._current_sprite_name())
        x = max(0, min(img.width - 1, pt[0]))
        y = max(0, min(img.height - 1, pt[1]))
        new_pt = point_list((x, y))
        alias = sprite.get("pivot_anchor")
        if name == "pivot":
            if alias and alias in sprite.get("anchors", {}):
                sprite.setdefault("anchors", {})[alias] = new_pt
                sprite["pivot"] = new_pt
            else:
                sprite.pop("pivot_anchor", None)
                sprite["pivot"] = new_pt
                self._populate_pivot_combo()
        else:
            anchors = sprite.setdefault("anchors", {})
            anchors[name] = new_pt
            if alias == name:
                sprite["pivot"] = new_pt
        self.unsaved = True
        self._sync_xy_vars()
        self.schedule_preview_update()

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
        # If two points overlap (common when pivot follows a named anchor), keep
        # the current selection when possible.  This avoids the old trap where
        # clicking the same location always selected the other point.
        threshold = max_dist_px / self.zoom
        current = self._selected_anchor.get()
        if current in self._anchor_names():
            ax, ay = self.get_anchor_point(current)
            if math.hypot(pt[0] - ax, pt[1] - ay) < threshold:
                return current
        best = None
        best_d = threshold
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
        self._populate_pivot_combo()
        self.select_anchor(name)
        self.schedule_preview_update()

    def delete_anchor(self) -> None:
        name = self._selected_anchor.get()
        if name == "pivot":
            self._status.set("Cannot delete pivot")
            return
        anchors = self._current_sprite().setdefault("anchors", {})
        if name in anchors:
            del anchors[name]
            if self._current_sprite().get("pivot_anchor") == name:
                self._current_sprite().pop("pivot_anchor", None)
            self.unsaved = True
            self._selected_anchor.set("pivot")
            self._populate_anchor_list()
            self._populate_pivot_combo()
            self.redraw()
            self.schedule_preview_update()

    def redraw(self) -> None:
        name = self._current_sprite_name()
        if not name:
            return
        self.background = self._bg_var.get()
        img = composited_sprite_preview(self._load_image(name), self.background)
        scaled = img.resize(
            (img.width * self.zoom, img.height * self.zoom), Image.Resampling.NEAREST
        )
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
                draw.ellipse(
                    (x - r - 4, y - r - 4, x + r + 4, y + r + 4),
                    outline=(255, 255, 255, 255),
                    width=2,
                )
            if self.show_names.get():
                draw.text((x + r + 3, y - r), aname, fill=(255, 255, 255, 255))
        self._photo = ImageTk.PhotoImage(scaled)
        self.canvas.delete("all")
        self.canvas.create_image(0, 0, image=self._photo, anchor="nw")
        self.canvas.configure(scrollregion=(0, 0, scaled.width, scaled.height))

    def schedule_preview_update(self, force: bool = False) -> None:
        if not self.preview.enabled or self.preview.config is None:
            self._preview_status.set("Live preview disabled or no preview config found")
            return
        if not self.preview.live_unsaved and not force:
            self._preview_status.set(
                "Preview stale; live unsaved preview is off. Press Render now / Ctrl+R."
            )
            return
        if force:
            if self._preview_after_id is not None:
                try:
                    self.root.after_cancel(self._preview_after_id)
                except Exception:
                    pass
                self._preview_after_id = None
            self.update_preview(force=True)
            return
        if self._preview_after_id is not None:
            try:
                self.root.after_cancel(self._preview_after_id)
            except Exception:
                pass
        self._preview_after_id = self.root.after(
            self.preview.debounce_ms, self.update_preview
        )

    def update_preview(self, force: bool = False) -> None:
        if not self.preview.enabled or self.preview.config is None:
            self._preview_status.set("Live preview disabled")
            return
        self._preview_after_id = None
        try:
            self._sync_all_pivots_from_aliases()
            img = render_preview_image(
                self.metadata,
                self.preview.config,
                max_width=self.preview.max_width,
                max_height=self.preview.max_height,
                bg=self.preview.background,
                fit_to_view=self.preview.fit_to_view,
            )
            self._preview_photo = ImageTk.PhotoImage(img)
            self.preview_canvas.delete("all")
            self.preview_canvas.create_image(
                0, 0, image=self._preview_photo, anchor="nw"
            )
            self.preview_canvas.configure(scrollregion=(0, 0, img.width, img.height))
            live = "live-unsaved" if self.preview.live_unsaved else "manual-unsaved"
            dirty = "dirty" if self.unsaved else "saved"
            fit = "fit" if self.preview.fit_to_view else "native"
            self._preview_status.set(
                f"{live} preview: {self.preview.config.name} ({img.width}x{img.height}, {fit}, {dirty})"
            )
        except Exception as ex:
            self._preview_status.set(f"Preview render failed: {ex}")

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
            if not (
                isinstance(refined_rect, list)
                and len(refined_rect) == 4
                and isinstance(rough_rect, list)
                and len(rough_rect) == 4
            ):
                continue
            rx, ry = refined_rect[0], refined_rect[1]
            ox, oy = rough_rect[0], rough_rect[1]

            def refined_local_to_rough_local(pt: Any) -> List[int]:
                p = as_point(pt)
                # The refinement step uses: new_local = old_rect + old_local - new_rect.
                # Invert that relation so the next refine preserves the edited refined point.
                return point_list((p[0] + rx - ox, p[1] + ry - oy))

            if refined_sprite.get("pivot_anchor"):
                rough_sprite["pivot_anchor"] = refined_sprite.get("pivot_anchor")
            elif "pivot_anchor" in rough_sprite:
                rough_sprite.pop("pivot_anchor", None)
            if "pivot" in refined_sprite:
                rough_sprite["pivot"] = refined_local_to_rough_local(
                    refined_sprite["pivot"]
                )
            if "anchors" in refined_sprite:
                rough_anchors = rough_sprite.setdefault("anchors", {})
                for aname, pt in refined_sprite.get("anchors", {}).items():
                    rough_anchors[aname] = refined_local_to_rough_local(pt)
        self.rough_metadata["metadata_quality"] = self.rough_metadata.get(
            "metadata_quality", {}
        )
        notes = self.rough_metadata["metadata_quality"].setdefault("notes", [])
        note = "anchors manually synced from refined metadata by tools/anchor_editor.py"
        if note not in notes:
            notes.append(note)

    def save(self) -> None:
        self._sync_all_pivots_from_aliases()
        if not self.unsaved:
            self._status.set("No anchor changes to save")
            return
        backup_file(self.paths.metadata)
        self.metadata["metadata_quality"] = self.metadata.get("metadata_quality", {})
        self.metadata["metadata_quality"]["anchor_precision"] = (
            "manually_adjusted_with_anchor_editor"
        )
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
            self._status.set(
                "Unsaved changes present; save or restart to reload safely"
            )
            return
        self.metadata = load_yaml(self.paths.metadata)
        self.rough_metadata = (
            load_yaml(self.paths.rough_metadata) if self.paths.rough_metadata else None
        )
        self.sprites = self.metadata.get("sprites", {})
        self.sprite_names = sorted(self.sprites.keys())
        self._populate_sprite_list()
        self.select_sprite(
            self._current_sprite_name()
            if self._current_sprite_name() in self.sprites
            else self.sprite_names[0]
        )

    def on_close(self) -> None:
        if self.unsaved:
            # Keep this minimal: avoid modal prompts when running from odd desktop
            # environments.  Users can hit Save explicitly or close again after
            # reading the status.
            self._status.set(
                "Unsaved changes. Press Save first, or close again after saving."
            )
            self.unsaved = False
            return
        self.root.destroy()


def write_anchor_report(
    metadata_path: Path,
    slices_path: Path,
    output: Path,
    sprites: Optional[List[str]] = None,
) -> None:
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
        rows.append(
            {
                "sprite": sid,
                "image": str(image_path),
                "size": size,
                "rect": sprite.get("rect"),
                "pivot": sprite.get("pivot"),
                "pivot_anchor": sprite.get("pivot_anchor"),
                "anchors": sprite.get("anchors", {}),
            }
        )
    output.write_text(
        json.dumps({"metadata": str(metadata_path), "sprites": rows}, indent=2),
        encoding="utf8",
    )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Manually place robot component pivots and anchors in a Tk GUI."
    )
    parser.add_argument(
        "metadata",
        type=Path,
        help="Metadata YAML to edit, usually metadata/robot_components.refined.yaml",
    )
    parser.add_argument(
        "--slices",
        type=Path,
        default=Path("output/slices"),
        help="Directory containing extracted component PNGs",
    )
    parser.add_argument(
        "--rough-metadata",
        type=Path,
        default=None,
        help="Optional rough YAML to keep in sync when saving",
    )
    parser.add_argument(
        "--zoom", type=int, default=5, help="Initial integer zoom factor"
    )
    parser.add_argument(
        "--background",
        choices=["checker", "black", "white"],
        default="checker",
        help="Preview background",
    )
    parser.add_argument(
        "--show-names", action="store_true", help="Show anchor names on the canvas"
    )
    parser.add_argument(
        "--preview-config",
        type=Path,
        default=None,
        help="Rig job YAML to render live in the right-side preview pane; defaults to examples/robot_rig_job.yaml when available",
    )
    parser.add_argument(
        "--no-live-preview",
        action="store_true",
        help="Disable live spritesheet preview rendering",
    )
    parser.add_argument(
        "--render-preview",
        type=Path,
        default=None,
        help="Render the preview spritesheet once and exit instead of opening the GUI",
    )
    parser.add_argument(
        "--preview-max-width",
        type=int,
        default=1400,
        help="Maximum live preview display width",
    )
    parser.add_argument(
        "--preview-max-height",
        type=int,
        default=820,
        help="Maximum live preview display height",
    )
    parser.add_argument(
        "--preview-native",
        action="store_true",
        help="Show the live preview at native rendered size with scrollbars instead of fitting to preview limits",
    )
    parser.add_argument(
        "--manual-preview",
        action="store_true",
        help="Do not auto-render on every edit; press Ctrl+R / Render now to preview unsaved metadata",
    )
    parser.add_argument(
        "--anchor-report",
        type=Path,
        default=None,
        help="Write a JSON anchor report and exit instead of opening the GUI",
    )
    parser.add_argument(
        "--sprites",
        nargs="*",
        default=None,
        help="Sprite ids to include in --anchor-report",
    )
    return parser


def main(argv: Optional[List[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    metadata = args.metadata.resolve()
    slices = args.slices.resolve()
    rough = args.rough_metadata.resolve() if args.rough_metadata else None
    preview_config = infer_preview_config(metadata, args.preview_config)
    if args.anchor_report:
        write_anchor_report(
            metadata, slices, args.anchor_report.resolve(), args.sprites
        )
        print(f"Wrote {args.anchor_report}")
        return 0
    if args.render_preview:
        if preview_config is None:
            print(
                "ERROR: no preview config found; pass --preview-config", file=sys.stderr
            )
            return 2
        meta = load_yaml(metadata)
        img = render_preview_image(
            meta,
            preview_config,
            max_width=args.preview_max_width,
            max_height=args.preview_max_height,
            bg="black",
            fit_to_view=not args.preview_native,
        )
        args.render_preview.parent.mkdir(parents=True, exist_ok=True)
        img.save(args.render_preview)
        print(f"Wrote {args.render_preview}")
        return 0
    # Import tkinter only when launching the GUI; this keeps CI/headless checks usable.
    try:
        import tkinter as tk
    except Exception as ex:  # pragma: no cover - environment dependent
        print(f"ERROR: tkinter is required for the GUI: {ex}", file=sys.stderr)
        return 2
    root = tk.Tk()
    preview = PreviewConfig(
        config=preview_config,
        enabled=not args.no_live_preview,
        max_width=args.preview_max_width,
        max_height=args.preview_max_height,
        live_unsaved=not args.manual_preview,
        fit_to_view=not args.preview_native,
    )
    AnchorEditorApp(
        root,
        EditorPaths(metadata=metadata, slices=slices, rough_metadata=rough),
        zoom=args.zoom,
        background=args.background,
        show_names=args.show_names,
        preview=preview,
    )
    root.mainloop()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
