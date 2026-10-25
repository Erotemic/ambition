"""Previews: a thumbnail of each image, a strip of frames for each animation
of each sheet, the first frame of each portrait clip, and a composite of each
parallax theme. Every preview is named by a digest of what it is made from,
so a rebuild draws only what changed. At most `jobs` processes (a shared
machine: the default is 4, the cap 6), one sheet each, one page at a time."""

from __future__ import annotations

import hashlib
import json
import os
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path
from typing import Any

from PIL import Image

Image.MAX_IMAGE_PIXELS = None

#: Bump to redraw every preview.
VERSION = 3
THUMB = 192
CELL = 128
MAX_JOBS = 6


def _key(*parts: Any) -> str:
    return hashlib.sha256(json.dumps([VERSION, *parts], sort_keys=True).encode()).hexdigest()[:24]


def _save(image: Image.Image, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    partial = path.with_suffix(f".{os.getpid()}.partial.webp")
    image.save(partial, "WEBP", quality=82, method=4)
    partial.replace(path)


def _thumb(image: Image.Image, size: int) -> Image.Image:
    image = image.convert("RGBA")
    image.thumbnail((size, size), Image.LANCZOS)
    return image


def image_task(src: str, dest: str) -> str | None:
    try:
        with Image.open(src) as im:
            _save(_thumb(im, THUMB), Path(dest))
        return None
    except Exception as ex:  # noqa: BLE001 - reported per file
        return f"{src}: {type(ex).__name__}: {ex}"


def _cell(frame: tuple[int, int]) -> tuple[int, int, float]:
    fw, fh = max(1, frame[0] or 1), max(1, frame[1] or 1)
    scale = min(CELL / fw, CELL / fh)
    return max(1, round(fw * scale)), max(1, round(fh * scale)), scale


def sheet_task(spec: dict[str, Any]) -> str | None:
    """Draw the strips of one sheet: `spec` has the page files, the frame
    size, each row's rects and the destination of each strip."""
    try:
        cw, ch, scale = _cell(tuple(spec["frame"]))
        pages: dict[int, Image.Image] = {}
        by_page: dict[int, list[tuple[int, int, list]]] = {}
        for r, row in enumerate(spec["rows"]):
            for i, rect in enumerate(row["rects"]):
                by_page.setdefault(rect[4], []).append((r, i, rect))
        strips = [Image.new("RGBA", (cw * max(1, len(row["rects"])), ch), (0, 0, 0, 0)) for row in spec["rows"]]
        cover = None
        for page in sorted(by_page):
            if page >= len(spec["pages"]):
                continue
            with Image.open(spec["pages"][page]) as im:
                sheet = im.convert("RGBA")
            for r, i, (x, y, w, h, _p, ox, oy) in by_page[page]:
                frame = Image.new("RGBA", (spec["frame"][0], spec["frame"][1]), (0, 0, 0, 0))
                frame.alpha_composite(sheet.crop((x, y, x + w, y + h)), (int(ox), int(oy)))
                if r == spec["cover_row"] and i == spec["cover_frame"]:
                    cover = frame
                strips[r].alpha_composite(frame.resize((cw, ch), Image.LANCZOS), (i * cw, 0))
            del sheet
        for row, strip in zip(spec["rows"], strips):
            _save(strip, Path(row["dest"]))
        if cover is not None:
            _save(_thumb(cover, THUMB), Path(spec["cover_dest"]))
        return None
    except Exception as ex:  # noqa: BLE001
        return f"{spec['pages'][0] if spec['pages'] else '?'}: {type(ex).__name__}: {ex}"


def portrait_task(spec: dict[str, Any]) -> str | None:
    try:
        with Image.open(spec["image"]) as im:
            sheet = im.convert("RGBA")
        for clip in spec["clips"]:
            x, y, w, h = clip["rect"]
            _save(_thumb(sheet.crop((x, y, x + w, y + h)), THUMB), Path(clip["dest"]))
        return None
    except Exception as ex:  # noqa: BLE001
        return f"{spec['image']}: {type(ex).__name__}: {ex}"


def composite_task(layers: list[str], dest: str) -> str | None:
    try:
        out = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        for layer in layers:
            with Image.open(layer) as im:
                out.alpha_composite(im.convert("RGBA").resize((512, 512), Image.LANCZOS))
        _save(_thumb(out, 256), Path(dest))
        return None
    except Exception as ex:  # noqa: BLE001
        return f"{dest}: {type(ex).__name__}: {ex}"


COVER_ROWS = ("idle", "rest", "stand", "default", "neutral")


def plan_and_draw(out_dir: Path, products: dict[str, Any], assets: dict[str, Any], jobs: int, draw: bool = True) -> list[str]:
    """Name every preview, draw the ones not drawn yet, and set the `thumb`,
    `strips`, `cover` and portrait previews on products and assets."""
    jobs = max(1, min(MAX_JOBS, jobs))
    tasks: list[tuple[Any, tuple]] = []

    def rel(path: Path) -> str:
        return path.relative_to(out_dir).as_posix()

    def needed(path: Path) -> bool:
        return draw and not path.exists()

    planned: set[Path] = set()
    for product in products.values():
        if product.root.of is None and product.file.suffix.lower() in (".png", ".webp", ".jpg", ".jpeg", ".gif"):
            dest = out_dir / "thumbs" / f"{product.sha256[:2]}" / f"{product.sha256}.webp"
            product.thumb = rel(dest)
            # Two files with the same bytes share one thumbnail, drawn once.
            if needed(dest) and dest not in planned:
                planned.add(dest)
                tasks.append((image_task, (str(product.file), str(dest))))

    for asset in assets.values():
        layout = asset.data.get("layout")
        if layout and layout["rows"]:
            pages_by_name = {Path(products[p].rel).name: products[p] for p in asset.products if products[p].root.of is None}
            pages = [pages_by_name.get(name) for name in layout["pages"]]
            if not all(pages):
                asset.data.setdefault("errors", []).append("a page the layout names is not published")
                pages = [p for p in pages if p]
            if pages:
                key = _key([p.sha256 for p in pages], layout["rows"])
                base = out_dir / "strips" / key
                names = [r["name"] for r in layout["rows"]]
                cover_row = next((names.index(n) for n in COVER_ROWS if n in names), 0)
                cover_frame = len(layout["rows"][cover_row]["rects"]) // 2
                cw, ch, _ = _cell(tuple(layout["frame"]))
                spec_rows = []
                for i, row in enumerate(layout["rows"]):
                    dest = base / f"{i}.webp"
                    row["strip"] = rel(dest)
                    spec_rows.append({"rects": row["rects"], "dest": str(dest)})
                layout["cell"] = [cw, ch]
                cover_dest = base / "cover.webp"
                asset.cover = rel(cover_dest)
                if needed(cover_dest) or any(needed(Path(r["dest"])) for r in spec_rows):
                    tasks.append((sheet_task, ({
                        "pages": [str(p.file) for p in pages], "frame": layout["frame"], "rows": spec_rows,
                        "cover_row": cover_row, "cover_frame": cover_frame, "cover_dest": str(cover_dest),
                    },)))
        portraits = asset.data.get("portraits")
        if portraits:
            image = next((products[p] for p in asset.products if products[p].role == "portraits"), None)
            if image is not None:
                clips = []
                for name, clip in portraits["clips"].items():
                    dest = out_dir / "portraits" / _key(image.sha256, name, clip["rects"][0]) / "f.webp"
                    clip["thumb"] = rel(dest)
                    clips.append({"rect": clip["rects"][0], "dest": str(dest)})
                if any(needed(Path(c["dest"])) for c in clips):
                    tasks.append((portrait_task, ({"image": str(image.file), "clips": clips},)))
        if asset.id.startswith("parallax:"):
            order = ("sky", "far_backplate", "near_background", "foreground_atmosphere", "foreground")
            layers = sorted(
                (products[p] for p in asset.products if products[p].root.of is None and products[p].role.startswith("layer:") and products[p].role[6:] in order),
                key=lambda p: order.index(p.role[6:]),
            )
            if layers:
                dest = out_dir / "composites" / f"{_key([p.sha256 for p in layers])}.webp"
                asset.cover = rel(dest)
                if needed(dest):
                    tasks.append((composite_task, ([str(p.file) for p in layers], str(dest))))
        if asset.cover is None:
            images = [products[p] for p in asset.products if getattr(products[p], "thumb", None)]
            pick = next((p for p in images if p.role in ("image", "sheet_page:0", "portraits", "reference") or p.role.startswith(("layer:", "part:fill"))), None) or (images[0] if images else None)
            if pick is not None:
                asset.cover = pick.thumb

    errors: list[str] = []
    if tasks:
        print(f"previews: drawing {len(tasks)} task(s) on {jobs} process(es)", flush=True)
        with ProcessPoolExecutor(max_workers=jobs) as pool:
            futures = [pool.submit(fn, *args) for fn, args in tasks]
            for n, future in enumerate(futures, 1):
                result = future.result()
                if result:
                    errors.append(result)
                if n % 200 == 0:
                    print(f"  {n}/{len(tasks)}", flush=True)
    return errors


def prune(out_dir: Path, products: dict[str, Any], assets: dict[str, Any]) -> int:
    """Delete previews the index no longer names; the count deleted."""
    keep = {getattr(p, "thumb", None) for p in products.values()} | {a.cover for a in assets.values()}
    for a in assets.values():
        for row in (a.data.get("layout") or {}).get("rows", []):
            keep.add(row.get("strip"))
        for clip in (a.data.get("portraits") or {}).get("clips", {}).values():
            keep.add(clip.get("thumb"))
    removed = 0
    for folder in ("thumbs", "strips", "portraits", "composites"):
        for dirpath, _dirs, files in os.walk(out_dir / folder):
            for name in files:
                path = Path(dirpath) / name
                if path.relative_to(out_dir).as_posix() not in keep:
                    path.unlink()
                    removed += 1
    return removed
