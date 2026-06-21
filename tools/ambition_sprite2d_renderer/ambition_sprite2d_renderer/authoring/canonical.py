"""Canonical-pose rendering and gallery composition.

Single entry point: iterate the unified target registry, call
``target.render_canonical(out_dir)`` on each, compose a labeled
gallery image with per-category section headers.

Every Target (tack-on or YAML-adapter) implements the same
[`render_canonical`] protocol method, so this module no longer needs
per-surface collectors. The slow/fast fallback for tack-on targets
without a dedicated canonical hook lives inside [`TackonTarget`]
itself.
"""

from __future__ import annotations

import math
from pathlib import Path
from typing import Iterable, List, Optional, Tuple

from PIL import Image, ImageDraw

from .adapters import get_adapter
from ..registry import CharacterJob
from .rendering import load_font
from ..registry import Target


# (target-id, display-label, transparent-RGBA-image)
CanonicalTile = Tuple[str, str, Image.Image]


_GALLERY_BG = (28, 28, 34, 255)
_TILE_BG = (44, 44, 52, 255)
_TILE_BORDER = (72, 72, 84, 255)


def render_canonical(job: CharacterJob) -> Image.Image:
    """Legacy in-memory adapter canonical renderer.

    Kept for backwards compat with callers (the ``canonical.py`` API
    has shipped this name for a while). New code should construct an
    `AdapterTarget` and call ``target.render_canonical(out_dir)``.
    """
    adapter = get_adapter(job.target)
    spec = adapter.sample_spec(job)
    return adapter.render_canonical(spec, job)


def _autocrop_transparent(img: Image.Image, pad: int = 4) -> Image.Image:
    """Crop ``img`` to its alpha bbox + a small pad."""
    if img.mode != "RGBA":
        img = img.convert("RGBA")
    bbox = img.getchannel("A").getbbox()
    if bbox is None:
        return img
    x1, y1, x2, y2 = bbox
    x1 = max(0, x1 - pad)
    y1 = max(0, y1 - pad)
    x2 = min(img.width, x2 + pad)
    y2 = min(img.height, y2 + pad)
    return img.crop((x1, y1, x2, y2))


def draw_canonical_of(
    target: Target,
    out_dir: str | Path,
    **opts,
) -> Path:
    """Draw the canonical of one Target into ``out_dir``.

    Single-target API. Returns the path to the saved
    ``{name}_canonical.png``. The transparent source PNG that the
    target's renderer writes
    (``{name}_canonical_transparent.png``) is normalized to a single
    consistent filename + auto-cropped here.
    """
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    source = target.render_canonical(out_dir, **opts)
    img = Image.open(source).convert("RGBA")
    img = _autocrop_transparent(img)
    gallery_out = out_dir / f"{target.name}_canonical.png"
    if gallery_out != source:
        img.save(gallery_out)
    return gallery_out


def _collect_tiles(
    targets: Iterable[Target],
    out_dir: Path,
) -> Tuple[List[CanonicalTile], List[str]]:
    """Draw every target's canonical into ``out_dir`` and return tiles + warnings."""
    tiles: List[CanonicalTile] = []
    warnings: List[str] = []
    for target in targets:
        try:
            path = draw_canonical_of(target, out_dir)
        except FileNotFoundError as ex:
            warnings.append(str(ex))
            continue
        except Exception as ex:  # noqa: BLE001 - record + continue
            warnings.append(
                f"{target.category}/{target.name}: render failed "
                f"({type(ex).__name__}: {ex})"
            )
            continue
        img = Image.open(path).convert("RGBA")
        label = target.name.replace("_", " ").title()
        tiles.append((target.name, label, img))
    return tiles, warnings


def _grid_contact_sheet(
    tiles: List[CanonicalTile],
    *,
    sections: Optional[List[Tuple[str, int]]] = None,
) -> Image.Image:
    """Compose a gallery from a list of canonical tiles."""
    font = load_font(14)
    header_font = load_font(18)
    max_label_w = max(
        (font.getbbox(label)[2] - font.getbbox(label)[0]) for _, label, _ in tiles
    )
    cell_w = max(max(img.width for _, _, img in tiles), max_label_w + 18) + 16
    cell_h = max(img.height for _, _, img in tiles) + 32

    if sections is None:
        sections = [("", len(tiles))]

    cols = max(1, min(8, int(math.ceil(math.sqrt(len(tiles))))))
    pad = 10
    header_h = 32

    section_rows: List[int] = []
    for _title, count in sections:
        if count <= 0:
            section_rows.append(0)
            continue
        section_rows.append(int(math.ceil(count / cols)))

    sheet_w = cell_w * cols + pad * 2
    sheet_h = pad * 2 + sum(
        (header_h if title else 0) + rows_in_section * cell_h
        for (title, _), rows_in_section in zip(sections, section_rows)
    )
    sheet_h = max(sheet_h, cell_h + pad * 2)

    contact = Image.new("RGBA", (sheet_w, sheet_h), _GALLERY_BG)
    draw = ImageDraw.Draw(contact)

    tile_iter = iter(tiles)
    y = pad
    for (title, count), rows_in_section in zip(sections, section_rows):
        if count <= 0:
            continue
        if title:
            draw.text((pad, y + 6), title, fill=(220, 220, 230, 255), font=header_font)
            y += header_h
        for r in range(rows_in_section):
            for c in range(cols):
                try:
                    stem, label, img = next(tile_iter)
                except StopIteration:
                    break
                x0 = pad + c * cell_w
                y0 = y + r * cell_h
                draw.rectangle(
                    (x0 + 2, y0 + 2, x0 + cell_w - 6, y0 + cell_h - 4),
                    fill=_TILE_BG,
                    outline=_TILE_BORDER,
                    width=1,
                )
                img_x = x0 + (cell_w - img.width) // 2
                img_y = y0 + 4 + (cell_h - 24 - img.height) // 2
                contact.alpha_composite(img, (img_x, img_y))
                label_box = font.getbbox(label)
                label_x = x0 + max(4, (cell_w - (label_box[2] - label_box[0])) // 2)
                draw.text(
                    (label_x, y0 + cell_h - 20),
                    label,
                    fill=(228, 228, 240, 255),
                    font=font,
                )
            else:
                continue
            break
        y += rows_in_section * cell_h
    return contact


def write_canonicals(config_dir: str | Path, out_dir: str | Path) -> List[Path]:
    """Legacy adapter-only path: walk YAML configs in ``config_dir``.

    Preserved for callers (notably ``draw_review``) that explicitly
    want to scope the canonical pass to a specific config dir without
    pulling in the unified target registry.
    """
    from ..registry import load_jobs

    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    tiles: List[CanonicalTile] = []
    for path, job in load_jobs(Path(config_dir)):
        img = render_canonical(job)
        if img.mode != "RGBA":
            img = img.convert("RGBA")
        img = _autocrop_transparent(img)
        stem = job.output_stem(path)
        out = out_dir / f"{stem}_canonical.png"
        img.save(out)
        label = job.name or stem.replace("_", " ").title()
        tiles.append((stem, label, img))
    outputs: List[Path] = [out_dir / f"{stem}_canonical.png" for stem, _, _ in tiles]
    if tiles:
        contact = _grid_contact_sheet(tiles)
        contact_out = out_dir / "canonicals_contact_sheet.png"
        contact.save(contact_out)
        outputs.append(contact_out)
    return outputs


def write_gallery(
    out_dir: str | Path,
    targets: Iterable[Target],
    *,
    section_order: Tuple[str, ...] = (
        "characters",
        "props",
        "tiles",
        "icons",
        "review_npcs",
    ),
) -> Tuple[List[Path], List[str]]:
    """Draw a labeled gallery from any iterable of [`Target`] instances.

    Group tiles by `target.category` in ``section_order`` so the
    gallery reads as one unified piece with consistent backdrop +
    per-category headers. Categories not in ``section_order`` come
    last in alphabetical order.

    Returns ``(outputs, warnings)`` — outputs is the list of files
    written; warnings collects per-target failures.
    """
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    targets_list = list(targets)
    # Group by category for section headers.
    by_cat: dict[str, List[Target]] = {}
    for target in targets_list:
        by_cat.setdefault(target.category, []).append(target)
    ordered_cats: List[str] = []
    for cat in section_order:
        if cat in by_cat:
            ordered_cats.append(cat)
    for cat in sorted(by_cat):
        if cat not in section_order:
            ordered_cats.append(cat)

    all_tiles: List[CanonicalTile] = []
    section_headers: List[Tuple[str, int]] = []
    warnings: List[str] = []
    for cat in ordered_cats:
        cat_targets = by_cat[cat]
        cat_tiles, cat_warnings = _collect_tiles(cat_targets, out_dir)
        warnings.extend(cat_warnings)
        if cat_tiles:
            all_tiles.extend(cat_tiles)
            section_headers.append((cat.replace("_", " ").title(), len(cat_tiles)))

    outputs: List[Path] = [
        out_dir / f"{stem}_canonical.png" for stem, _, _ in all_tiles
    ]
    if all_tiles:
        contact = _grid_contact_sheet(all_tiles, sections=section_headers)
        contact_out = out_dir / "canonicals_contact_sheet.png"
        contact.save(contact_out)
        outputs.append(contact_out)
    return outputs, warnings
