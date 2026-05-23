"""Canonical-pose rendering and contact-sheet composition.

Two surfaces both produce a canonical "idle" frame for art review:

- **Adapter targets** — driven by YAML configs in ``configs/``;
  rendered in-memory via ``adapter.render_canonical(spec, job)``.
- **Tack-on targets** — discovered under ``targets/<category>/``.
  Their ``render()`` writes ``{name}_canonical.png`` to
  ``generated/<name>/`` as a side-effect of building the full sheet
  (see [`tackon_sheet.build_sheet`]).

[`write_all_canonicals`] is the unified entry point: it renders /
collects canonicals from both surfaces and composes a single grid
contact sheet. [`write_canonicals`] is the legacy adapter-only path,
preserved for callers that explicitly want it.
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


# (target-id, display-label, image)
CanonicalTile = Tuple[str, str, Image.Image]


def render_canonical(job: CharacterJob) -> Image.Image:
    adapter = get_adapter(job.target)
    spec = adapter.sample_spec(job)
    return adapter.render_canonical(spec, job)


def _collect_adapter_tiles(
    config_dir: Path,
    out_dir: Path,
) -> List[CanonicalTile]:
    """Render every adapter job's canonical pose, save it, return tiles."""
    tiles: List[CanonicalTile] = []
    for path, job in load_jobs(config_dir):
        img = render_canonical(job)
        stem = job.output_stem(path)
        out = out_dir / f"{stem}_canonical.png"
        img.save(out)
        label = job.name or stem.replace("_", " ").title()
        tiles.append((stem, label, img))
    return tiles


def _collect_tackon_tiles(
    targets: Iterable[Tuple[str, TackonTarget]],
    out_dir: Path,
    generated_root: Path,
    *,
    render_if_missing: bool,
) -> Tuple[List[CanonicalTile], List[str]]:
    """For each tack-on target, pluck (or render-then-pluck) its canonical.

    Each tack-on's ``render()`` writes ``{name}_canonical.png`` to its
    ``generated/<name>/`` dir as a side-effect of the full sheet build.
    Reuse that cached file when present; render the full sheet when not.

    Returns ``(tiles, warnings)`` so callers can surface targets that
    didn't produce a canonical (typically bespoke render paths that
    skip [`tackon_sheet.build_sheet`]).
    """
    tiles: List[CanonicalTile] = []
    warnings: List[str] = []
    for name, target in targets:
        gen_dir = generated_root / name
        canonical = gen_dir / f"{name}_canonical.png"
        if not canonical.exists() and render_if_missing:
            try:
                target.render(gen_dir)
            except Exception as ex:  # noqa: BLE001 - record + continue
                warnings.append(
                    f"{target.category}/{name}: render failed "
                    f"({type(ex).__name__}: {ex})"
                )
                continue
        if not canonical.exists():
            # Some bespoke targets (weird_hermit, mockingbird_boss) don't
            # follow the build_sheet naming convention. Try the first
            # PNG in their sheet_files as a fallback.
            for fname in target.sheet_files:
                candidate = gen_dir / fname
                if candidate.suffix == ".png" and candidate.exists():
                    canonical = candidate
                    break
            else:
                warnings.append(
                    f"{target.category}/{name}: no canonical PNG found "
                    f"in {gen_dir} after render"
                )
                continue
        img = Image.open(canonical).convert("RGBA")
        out = out_dir / f"{name}_canonical.png"
        img.save(out)
        label = name.replace("_", " ").title()
        tiles.append((name, label, img))
    return tiles, warnings


def _grid_contact_sheet(tiles: List[CanonicalTile]) -> Image.Image:
    """Compose a grid contact sheet from a list of canonical tiles.

    Switches from the legacy horizontal strip to a square-ish grid
    because the tack-on roster pushes the strip past 60 tiles wide,
    which doesn't fit on any reasonable display.
    """
    font = load_font(14)
    cell_label_w = max((font.getbbox(label)[2] - font.getbbox(label)[0]) for _, label, _ in tiles)
    cell_w = max(max(img.width for _, _, img in tiles), cell_label_w + 18)
    cell_h = max(img.height for _, _, img in tiles) + 24
    cols = max(1, int(math.ceil(math.sqrt(len(tiles)))))
    rows = max(1, int(math.ceil(len(tiles) / cols)))
    contact = Image.new("RGBA", (cell_w * cols, cell_h * rows), (0, 0, 0, 0))
    draw = ImageDraw.Draw(contact)
    for idx, (_stem, label, img) in enumerate(tiles):
        col = idx % cols
        row = idx // cols
        x0 = col * cell_w
        y0 = row * cell_h
        label_box = font.getbbox(label)
        label_x = x0 + max(8, (cell_w - (label_box[2] - label_box[0])) // 2)
        contact.alpha_composite(img, (x0 + (cell_w - img.width) // 2, y0 + 20))
        draw.text((label_x, y0 + 3), label, fill=(255, 255, 255, 255), font=font)
    return contact


def write_canonicals(config_dir: str | Path, out_dir: str | Path) -> List[Path]:
    """Render every adapter target's canonical pose + a horizontal contact sheet.

    Legacy adapter-only path. Prefer [`write_all_canonicals`] for the
    unified adapter + tack-on output.
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


def write_all_canonicals(
    out_dir: str | Path,
    *,
    config_dir: Optional[str | Path],
    tackons: Iterable[Tuple[str, TackonTarget]],
    generated_root: str | Path,
    render_if_missing: bool = True,
) -> Tuple[List[Path], List[str]]:
    """Render canonicals for every adapter + tack-on target into ``out_dir``.

    Adapter canonicals come from the in-memory adapter pipeline.
    Tack-on canonicals come from each target's ``generated/<name>/<name>_canonical.png``
    (rendered on demand if missing and ``render_if_missing`` is true).

    Returns ``(outputs, warnings)`` — ``outputs`` is the list of files
    written under ``out_dir``; ``warnings`` collects per-target failures
    so the CLI can surface them without aborting the whole run.
    """
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    generated_root = Path(generated_root)

    tiles: List[CanonicalTile] = []
    warnings: List[str] = []

    if config_dir is not None:
        tiles.extend(_collect_adapter_tiles(Path(config_dir), out_dir))

    tackon_tiles, tackon_warnings = _collect_tackon_tiles(
        tackons, out_dir, generated_root, render_if_missing=render_if_missing,
    )
    tiles.extend(tackon_tiles)
    warnings.extend(tackon_warnings)

    outputs: List[Path] = [out_dir / f"{stem}_canonical.png" for stem, _, _ in tiles]
    if tiles:
        contact = _grid_contact_sheet(tiles)
        contact_out = out_dir / "canonicals_contact_sheet.png"
        contact.save(contact_out)
        outputs.append(contact_out)
    return outputs, warnings
