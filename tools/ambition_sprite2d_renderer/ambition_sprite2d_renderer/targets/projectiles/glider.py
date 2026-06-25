"""Conway's Game of Life *glider* projectile.

The glider is the smallest spaceship in Conway's Life: a five-cell
pattern that, left to evolve, returns to its original shape every four
generations — translated one cell diagonally. So the projectile's
animation is not stylised; it is the real automaton. We seed the
canonical south-east glider and step the actual Life rules to produce
the four animation frames.

Orientation: the seed is the south-east glider, whose natural Life
travel is down-and-to-the-RIGHT — a 45° diagonal. A glider is inherently
a *diagonal* spaceship; it does not move along an axis-aligned grid
direction. So after rendering each Life phase we ROTATE the frame by 45°,
mapping that diagonal travel vector onto the sprite's +x axis. The
projectile then flies "nose-first" along its velocity: fired rightward,
the sprite reads as a glider genuinely travelling the way it would on a
real Life grid, just with its diagonal lattice tipped onto the screen
horizontal. The runtime flips the sheet horizontally to face travel, so
a leftward shot mirrors correctly.

The four phases are each re-centred to a fixed 3×3 window before the
rotation, so the sprite animates in place (the projectile entity supplies
the actual world translation) and loops seamlessly. No board grid — only
the live cells are drawn.
"""
from __future__ import annotations

from pathlib import Path
from typing import FrozenSet, List, Set, Tuple

from PIL import Image, ImageDraw

from ...authoring.tackon_sheet import build_sheet, write_canonical
from ...core.draw import rgba

Cell = Tuple[int, int]

TARGET_NAME = "glider"

# Authored at this canvas, then auto-cropped by build_sheet.
FRAME_W, FRAME_H = 96, 96
SUPERSAMPLE = 4

# One row: the glider's flight cycle. Four genuine Life generations,
# ~90 ms each → a brisk, legible roll for a projectile.
ROWS: List[Tuple[str, int, int]] = [("fly", 4, 90)]

# The glider is a 3×3 pattern; centre it in the frame, then rotate.
CELL = 16.0        # logical px per cell
GLIDER_SPAN = 3    # the glider's bounding box is 3×3 cells
MARGIN = (FRAME_W - GLIDER_SPAN * CELL) / 2.0  # centres the 3×3 in the frame

# A glider travels at 45° (its diagonal lattice). Rotate the rendered
# frame counter-clockwise by this so the south-east travel vector lands
# on the sprite's +x axis (screen right).
ROTATE_DEG = 45.0

# Canonical south-east glider (travels down-right). Coordinates are
# (x, y) with +x right and +y down — i.e. screen space.
#   . # .
#   . . #
#   # # #
_SEED: Set[Cell] = {(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)}

# Energised-cell palette (ties into the cellular-automaton content theme).
_DARK = "#04140F"
_CELL_RIM = "#1E8F5A"
_CELL_CORE = "#39E06E"
_GLINT = "#CFFFE0"


def _life_step(cells: Set[Cell]) -> Set[Cell]:
    """One generation of Conway's Game of Life over a sparse cell set."""
    counts: dict[Cell, int] = {}
    for (x, y) in cells:
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                if dx or dy:
                    counts[(x + dx, y + dy)] = counts.get((x + dx, y + dy), 0) + 1
    return {
        pos
        for pos, n in counts.items()
        if n == 3 or (n == 2 and pos in cells)
    }


def _normalize(cells: Set[Cell]) -> FrozenSet[Cell]:
    """Translate a pattern so its bounding box starts at (0, 0), so every
    phase is drawn in the same fixed window (animation in place)."""
    min_x = min(x for x, _ in cells)
    min_y = min(y for _, y in cells)
    return frozenset((x - min_x, y - min_y) for (x, y) in cells)


def _glider_phases(n: int = 4) -> List[FrozenSet[Cell]]:
    phases: List[FrozenSet[Cell]] = []
    cells: Set[Cell] = set(_SEED)
    for _ in range(n):
        phases.append(_normalize(cells))
        cells = _life_step(cells)
    return phases


_PHASES: List[FrozenSet[Cell]] = _glider_phases(4)


def _cell_box(col: float, row: float, inset: float, s: float):
    x0 = (MARGIN + col * CELL + inset) * s
    y0 = (MARGIN + row * CELL + inset) * s
    x1 = (MARGIN + (col + 1) * CELL - inset) * s
    y1 = (MARGIN + (row + 1) * CELL - inset) * s
    return (x0, y0, x1, y1)


def _draw_phase(phase: FrozenSet[Cell]) -> Image.Image:
    s = float(SUPERSAMPLE)
    img = Image.new("RGBA", (FRAME_W * SUPERSAMPLE, FRAME_H * SUPERSAMPLE), (0, 0, 0, 0))
    d = ImageDraw.Draw(img, "RGBA")
    dark = rgba(_DARK)

    # Live cells: energised squares with a bright core + specular glint.
    for (x, y) in phase:
        col, row = float(x), float(y)
        bx0, by0, bx1, by1 = _cell_box(col, row, -3, s)
        d.rounded_rectangle((bx0, by0, bx1, by1), radius=5 * s, fill=rgba(_CELL_CORE, 60))
        x0, y0, x1, y1 = _cell_box(col, row, 2, s)
        d.rounded_rectangle((x0, y0, x1, y1), radius=4 * s, fill=rgba(_CELL_RIM), outline=dark, width=max(1, int(2 * s)))
        cx0, cy0, cx1, cy1 = _cell_box(col, row, 5, s)
        d.rounded_rectangle((cx0, cy0, cx1, cy1), radius=3 * s, fill=rgba(_CELL_CORE))
        d.ellipse((cx0, cy0, (cx0 + cx1) / 2, (cy0 + cy1) / 2), fill=rgba(_GLINT, 230))

    # Tip the 45° diagonal travel onto the sprite's +x axis.
    img = img.rotate(ROTATE_DEG, resample=Image.BICUBIC, expand=False)
    return img.resize((FRAME_W, FRAME_H), Image.LANCZOS)


def render_frame(animation: str, frame_idx: int, nframes: int) -> Image.Image:
    del animation, nframes
    return _draw_phase(_PHASES[frame_idx % len(_PHASES)])


def render(out_dir: Path, **opts) -> List[Path]:
    del opts
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=render_frame,
        out_dir=Path(out_dir),
        frame_size=(FRAME_W, FRAME_H),
        label_width=100,
        auto_crop=True,
    )
    keys = ("spritesheet", "yaml", "ron", "actor", "canonical", "canonical_transparent", "preview")
    return [Path(outputs[k]) for k in keys if outputs.get(k)]


def render_canonical(out_dir: Path, **opts) -> Path:
    del opts
    return write_canonical(TARGET_NAME, ROWS, render_frame, Path(out_dir), frame_size=(FRAME_W, FRAME_H))
