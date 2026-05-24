"""Canonical-pose rendering and gallery composition.

Three surfaces all produce a single canonical "idle" frame for art review:

- **Adapter targets** — driven by YAML configs in ``configs/``;
  rendered in-memory via ``adapter.render_canonical(spec, job)``.
- **Tack-on targets** — discovered under ``targets/<category>/``.
  We always render fresh into the gallery's out_dir, never read from
  ``generated/<name>/`` cache. Fast path: target exposes a
  ``render_canonical(out_dir, **opts)`` hook (3 lines wrapping
  [`tackon_sheet.write_canonical`]). Slow fallback: invoke the full
  ``target.render(out_dir)`` and pluck ``{name}_canonical_transparent.png``
  from its outputs.
- **Review NPCs** — toon-adapter jobs under ``configs/review/``,
  enumerated in CLI's ``RUNTIME_REVIEW_NPCS``. Same rendering path as
  adapter targets, just sourced from a different config dir.

[`write_all_canonicals`] is the unified entry point: it renders the
canonical of every target fresh and composes a single gallery image
with consistent transparent backdrop + per-category section headers.

[`draw_canonical_of`] is the single-target equivalent, exposed via
CLI as ``canonical <target>``.
"""
from __future__ import annotations

import math
from pathlib import Path
from typing import Iterable, List, Optional, Tuple

from PIL import Image, ImageDraw

from .adapters import get_adapter
from .config import CharacterJob, load_jobs
from .rendering import load_font
from .target_registry import TackonTarget


# (target-id, display-label, transparent-RGBA-image)
CanonicalTile = Tuple[str, str, Image.Image]


# Gallery backdrop — solid neutral so transparent silhouettes read clearly
# without competing for attention. Slightly darker than the per-tile
# divider so tiles pop against the page.
_GALLERY_BG = (28, 28, 34, 255)
_TILE_BG = (44, 44, 52, 255)
_TILE_BORDER = (72, 72, 84, 255)


def render_canonical(job: CharacterJob) -> Image.Image:
    adapter = get_adapter(job.target)
    spec = adapter.sample_spec(job)
    return adapter.render_canonical(spec, job)


def _autocrop_transparent(img: Image.Image, pad: int = 4) -> Image.Image:
    """Crop ``img`` to its alpha bbox + a small pad so tiles don't carry
    the full original frame canvas (most of which is transparent).

    Adapter canonicals come back at the job's full ``single_width × single_height``
    (often 128×128) but the actual artwork uses ~40% of that. Cropping
    here lets the gallery's max-image-size grid stay compact even when
    one target has a full-frame silhouette.
    """
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


def _collect_adapter_tiles(
    config_dir: Path,
    out_dir: Path,
    *,
    label_prefix: str = "",
) -> List[CanonicalTile]:
    """Render every adapter job's canonical pose, save transparent tile, return."""
    tiles: List[CanonicalTile] = []
    for path, job in load_jobs(config_dir):
        img = render_canonical(job)
        if img.mode != "RGBA":
            img = img.convert("RGBA")
        img = _autocrop_transparent(img)
        stem = job.output_stem(path)
        out = out_dir / f"{stem}_canonical.png"
        img.save(out)
        label = job.name or stem.replace("_", " ").title()
        if label_prefix:
            label = f"{label_prefix}{label}"
        tiles.append((stem, label, img))
    return tiles


def draw_tackon_canonical(target: TackonTarget, out_dir: Path) -> Path:
    """Draw the canonical of one tack-on target and save it to ``out_dir``.

    Fast path: if the target exposes a ``render_canonical(out_dir)``
    hook, call it. That's a thin wrapper around
    [`tackon_sheet.write_canonical`] that renders + crops + saves
    just the canonical frame.

    Slow path: run the target's full ``render(out_dir)`` and locate
    the ``{name}_canonical_transparent.png`` it emits as a side
    effect. Slower (builds the entire sheet) but works for every
    target without per-target opt-in.

    Returns the path to the saved transparent PNG. Raises
    ``FileNotFoundError`` if the slow path completed but didn't emit a
    canonical (typically a bespoke render that bypasses
    [`tackon_sheet.build_sheet`]).
    """
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    if target.render_canonical is not None:
        return Path(target.render_canonical(out_dir))
    # Slow fallback: full render, look for the canonical_transparent the
    # tack-on sheet pipeline writes.
    target.render(out_dir)
    candidate = out_dir / f"{target.name}_canonical_transparent.png"
    if candidate.exists():
        return candidate
    raise FileNotFoundError(
        f"{target.category}/{target.name}: full render completed but "
        f"{candidate.name} is missing — target's render() may not go "
        f"through `tackon_sheet.build_sheet`. Add a `render_canonical` "
        f"hook (see e.g. galwah.py) to fix."
    )


def _collect_tackon_tiles(
    targets: Iterable[Tuple[str, TackonTarget]],
    out_dir: Path,
) -> Tuple[List[CanonicalTile], List[str]]:
    """Draw a fresh canonical for each tack-on target into ``out_dir``.

    Always renders fresh — does NOT read from `generated/<name>/`
    cache. See [`draw_tackon_canonical`] for the per-target API and
    the fast/slow path split.
    """
    tiles: List[CanonicalTile] = []
    warnings: List[str] = []
    for name, target in targets:
        try:
            canonical_path = draw_tackon_canonical(target, out_dir)
        except FileNotFoundError as ex:
            warnings.append(str(ex))
            continue
        except Exception as ex:  # noqa: BLE001 - record + continue
            warnings.append(
                f"{target.category}/{name}: render failed "
                f"({type(ex).__name__}: {ex})"
            )
            continue
        img = Image.open(canonical_path).convert("RGBA")
        img = _autocrop_transparent(img)
        # Also save a copy at the gallery-conventional name (matches the
        # adapter + review NPC convention so the per-target PNGs in
        # out_dir are uniformly named).
        gallery_out = out_dir / f"{name}_canonical.png"
        if gallery_out != canonical_path:
            img.save(gallery_out)
        label = name.replace("_", " ").title()
        tiles.append((name, label, img))
    return tiles, warnings


def _collect_review_npc_tiles(
    review_dir: Path,
    out_dir: Path,
    review_npcs: Iterable[str],
) -> Tuple[List[CanonicalTile], List[str]]:
    """Render canonicals for every review NPC listed in ``review_npcs``.

    These are toon-adapter jobs under ``configs/review/``; rendering
    path is identical to [`_collect_adapter_tiles`] but driven by the
    enumerated stem list instead of a directory walk (so we get the
    curated runtime cast, not every art-iteration scratch config).
    """
    tiles: List[CanonicalTile] = []
    warnings: List[str] = []
    for stem in review_npcs:
        cfg = review_dir / f"{stem}.yaml"
        if not cfg.exists():
            warnings.append(f"review/{stem}: config missing at {cfg}")
            continue
        try:
            job = CharacterJob.load(cfg)
            img = render_canonical(job)
        except Exception as ex:  # noqa: BLE001
            warnings.append(
                f"review/{stem}: render failed "
                f"({type(ex).__name__}: {ex})"
            )
            continue
        if img.mode != "RGBA":
            img = img.convert("RGBA")
        img = _autocrop_transparent(img)
        out = out_dir / f"{stem}_canonical.png"
        img.save(out)
        label = job.name or stem.replace("_", " ").title()
        tiles.append((stem, label, img))
    return tiles, warnings


def _grid_contact_sheet(
    tiles: List[CanonicalTile],
    *,
    sections: Optional[List[Tuple[str, int]]] = None,
) -> Image.Image:
    """Compose a gallery from a list of canonical tiles.

    Every tile is composited onto the same neutral backdrop so the
    gallery reads as one consistent piece regardless of which source
    each canonical came from. Tiles are centered within a uniform
    cell sized to the largest source image, so silhouette-vs-silhouette
    proportions stay readable.

    ``sections`` is an optional list of ``(section_title, tile_count)``
    headers — when present, the gallery is split into labeled sections
    in order, with a header row between groups. When absent, all tiles
    flow into a single square-ish grid.
    """
    font = load_font(14)
    header_font = load_font(18)
    max_label_w = max((font.getbbox(label)[2] - font.getbbox(label)[0]) for _, label, _ in tiles)
    cell_w = max(max(img.width for _, _, img in tiles), max_label_w + 18) + 16
    cell_h = max(img.height for _, _, img in tiles) + 32

    if sections is None:
        sections = [("", len(tiles))]

    cols = max(1, min(8, int(math.ceil(math.sqrt(len(tiles))))))
    pad = 10
    header_h = 32

    # Pre-compute layout: rows-per-section + total height.
    section_rows: List[int] = []
    total_grid_rows = 0
    for _title, count in sections:
        if count <= 0:
            section_rows.append(0)
            continue
        rows_in_section = int(math.ceil(count / cols))
        section_rows.append(rows_in_section)
        total_grid_rows += rows_in_section

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
                # Tile backdrop + border so transparent silhouettes have
                # a consistent surface to read against.
                draw.rectangle(
                    (x0 + 2, y0 + 2, x0 + cell_w - 6, y0 + cell_h - 4),
                    fill=_TILE_BG, outline=_TILE_BORDER, width=1,
                )
                # Center the canonical image within the tile.
                img_x = x0 + (cell_w - img.width) // 2
                img_y = y0 + 4 + (cell_h - 24 - img.height) // 2
                contact.alpha_composite(img, (img_x, img_y))
                # Label centered at the bottom of the tile.
                label_box = font.getbbox(label)
                label_x = x0 + max(4, (cell_w - (label_box[2] - label_box[0])) // 2)
                draw.text(
                    (label_x, y0 + cell_h - 20),
                    label, fill=(228, 228, 240, 255), font=font,
                )
            else:
                continue
            # Inner loop broke (out of tiles); break the outer loop too.
            break
        y += rows_in_section * cell_h
    return contact


def write_canonicals(config_dir: str | Path, out_dir: str | Path) -> List[Path]:
    """Render every adapter target's canonical pose + a contact sheet.

    Legacy adapter-only path. Prefer [`write_all_canonicals`] for the
    unified gallery covering adapter targets, tack-ons, and review NPCs.
    """
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    tiles = _collect_adapter_tiles(Path(config_dir), out_dir)
    outputs: List[Path] = [out_dir / f"{stem}_canonical.png" for stem, _, _ in tiles]
    if tiles:
        contact = _grid_contact_sheet(tiles)
        contact_out = out_dir / "canonicals_contact_sheet.png"
        contact.save(contact_out)
        outputs.append(contact_out)
    return outputs


def draw_canonical_of(
    target_id: str,
    out_dir: str | Path,
    *,
    tackons: dict[str, TackonTarget],
    config_dir: Optional[str | Path] = None,
    review_config_dir: Optional[str | Path] = None,
) -> Path:
    """Draw the canonical of a single target (any surface) into ``out_dir``.

    Resolves ``target_id`` across all three surfaces — adapter configs,
    tack-on registry, review-NPC configs — and renders one canonical
    PNG to ``out_dir/{target_id}_canonical.png``. Single-target CLI
    entrypoint (``canonical <target>``) wraps this.
    """
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    # Tack-on hit?
    if target_id in tackons:
        target = tackons[target_id]
        canonical_path = draw_tackon_canonical(target, out_dir)
        img = Image.open(canonical_path).convert("RGBA")
        img = _autocrop_transparent(img)
        gallery_out = out_dir / f"{target_id}_canonical.png"
        if gallery_out != canonical_path:
            img.save(gallery_out)
        return gallery_out
    # Adapter config hit (configs/*.yaml or configs/review/*.yaml)?
    for cfg_dir in (config_dir, review_config_dir):
        if cfg_dir is None:
            continue
        cfg = Path(cfg_dir) / f"{target_id}.yaml"
        if cfg.exists():
            job = CharacterJob.load(cfg)
            img = render_canonical(job)
            if img.mode != "RGBA":
                img = img.convert("RGBA")
            img = _autocrop_transparent(img)
            out = out_dir / f"{target_id}_canonical.png"
            img.save(out)
            return out
    raise KeyError(
        f"unknown target {target_id!r}; not in tack-on registry and no "
        f"{target_id}.yaml under configs/ or configs/review/"
    )


def write_all_canonicals(
    out_dir: str | Path,
    *,
    config_dir: Optional[str | Path],
    tackons: Iterable[Tuple[str, TackonTarget]],
    review_config_dir: Optional[str | Path] = None,
    review_npcs: Iterable[str] = (),
) -> Tuple[List[Path], List[str]]:
    """Draw the full canonical gallery: adapters + tack-ons + review NPCs.

    Every tile is drawn fresh — does NOT read from any cached
    ``generated/<name>/`` files. The gallery composites all tiles onto
    a consistent transparent-backed gallery so it reads as one unified
    review piece, with per-category section headers.

    Returns ``(outputs, warnings)`` — outputs is the list of files
    written; warnings collects per-target failures so the CLI can
    surface them without aborting the whole run.
    """
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    warnings: List[str] = []
    sections: List[Tuple[str, List[CanonicalTile]]] = []

    if config_dir is not None:
        adapter_tiles = _collect_adapter_tiles(Path(config_dir), out_dir)
        if adapter_tiles:
            sections.append(("Adapter targets", adapter_tiles))

    if review_config_dir is not None and review_npcs:
        review_tiles, review_warnings = _collect_review_npc_tiles(
            Path(review_config_dir), out_dir, review_npcs,
        )
        warnings.extend(review_warnings)
        if review_tiles:
            sections.append(("Review NPCs", review_tiles))

    tackons_list = list(tackons)
    tackon_tiles, tackon_warnings = _collect_tackon_tiles(
        tackons_list, out_dir,
    )
    warnings.extend(tackon_warnings)
    # Split tack-ons by category so characters / props / tiles / icons each
    # get their own labeled section.
    by_cat: dict[str, List[CanonicalTile]] = {}
    cat_of: dict[str, str] = {name: tgt.category for name, tgt in tackons_list}
    for tile in tackon_tiles:
        cat = cat_of.get(tile[0], "tack-on")
        by_cat.setdefault(cat, []).append(tile)
    for cat in ("characters", "props", "tiles", "icons"):
        if cat in by_cat:
            sections.append((f"Tack-on {cat}", by_cat[cat]))

    all_tiles: List[CanonicalTile] = []
    section_headers: List[Tuple[str, int]] = []
    for title, tiles in sections:
        all_tiles.extend(tiles)
        section_headers.append((title, len(tiles)))

    outputs: List[Path] = [out_dir / f"{stem}_canonical.png" for stem, _, _ in all_tiles]
    if all_tiles:
        contact = _grid_contact_sheet(all_tiles, sections=section_headers)
        contact_out = out_dir / "canonicals_contact_sheet.png"
        contact.save(contact_out)
        outputs.append(contact_out)
    return outputs, warnings
