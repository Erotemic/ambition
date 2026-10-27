"""Previews: a thumbnail of each image, a strip of frames for each animation
of each sheet, the first frame of each portrait clip, and a composite of each
parallax theme. Every preview is named by a digest of what it is made from,
so a rebuild draws only what changed. At most `jobs` processes (a shared
machine: the default is 4, the cap 6), one sheet each, one page at a time."""

from __future__ import annotations

import hashlib
import json
import math
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
WEBP_MAX_SIDE = 16383


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


def _columns(frames: int, width: int, height: int) -> int:
    """Keep preview grids inside the WebP image extent."""
    frames = max(1, frames)
    columns = min(frames, WEBP_MAX_SIDE // width, max(32, math.ceil(frames / (WEBP_MAX_SIDE // height))))
    if math.ceil(frames / columns) * height > WEBP_MAX_SIDE:
        raise ValueError("The animation has too many frames for one preview image")
    return columns


def subject_camera(image: Image.Image, body: dict | None, feet: dict | None, parts=()) -> dict:
    """Fit the neutral subject once. Keep this camera for all action frames."""
    visible = image.getchannel("A").point(lambda alpha: 255 if alpha > 16 else 0).getbbox()
    boxes = []
    for box in [body, *parts]:
        if box and all(math.isfinite(box.get(key, float("nan"))) for key in ("x", "y", "w", "h")) and box["w"] > 0 and box["h"] > 0:
            boxes.append((box["x"], box["y"], box["x"] + box["w"], box["y"] + box["h"]))
    basis = "body-and-visible" if boxes else "visible" if visible else "empty"
    if feet and all(math.isfinite(feet.get(key, float("nan"))) for key in ("x", "y")):
        anchor, feet_basis = [feet["x"], feet["y"]], "authored"
    elif boxes:
        anchor = [(min(box[0] for box in boxes) + max(box[2] for box in boxes)) / 2, max(box[3] for box in boxes)]
        feet_basis = "body-bounds"
    else:
        anchor = [(visible[0] + visible[2]) / 2, visible[3]] if visible else [image.width / 2, image.height]
        feet_basis = "visible" if visible else "empty"
    if visible:
        boxes.append(visible)
    bounds = [min(box[0] for box in boxes), min(box[1] for box in boxes), max(box[2] for box in boxes), max(box[3] for box in boxes)] if boxes else [0, 0, image.width, image.height]
    authored_anchor = list(anchor)
    anchor[0] = (bounds[0] + bounds[2]) / 2
    # A physical foot anchor can be above a tail or a floating body's art.
    upper_scale = min(0.84 / max(1, bounds[2] - bounds[0]), 0.78 / max(1, anchor[1] - bounds[1]))
    if (bounds[3] - anchor[1]) * upper_scale > 0.08:
        anchor[1] = bounds[3]
        feet_basis = "visible-bottom"
    extents = [anchor[0] - bounds[0], bounds[2] - anchor[0], anchor[1] - bounds[1], bounds[3] - anchor[1]]
    scale = min(room / extent for room, extent in zip((0.42, 0.42, 0.78, 0.08), extents) if extent > 0)
    return {"basis": basis, "feet_basis": feet_basis, "bounds": bounds, "anchor": anchor,
            "authored_anchor": authored_anchor, "scale": scale, "baseline": 0.9}


def subject_frame(image: Image.Image, camera: dict, size: int) -> Image.Image:
    """Draw a logical frame through the sheet's fixed subject camera."""
    scale = camera["scale"] * size
    inverse = (1 / scale, 0, camera["anchor"][0] - size * 0.5 / scale,
               0, 1 / scale, camera["anchor"][1] - size * camera["baseline"] / scale)
    return image.convert("RGBa").transform((size, size), Image.Transform.AFFINE, inverse,
                                           Image.Resampling.BICUBIC).convert("RGBA")


def _frame(page: Image.Image, rect: list, size: list) -> Image.Image:
    x, y, w, h, _page, ox, oy = rect
    image = Image.new("RGBA", tuple(size), (0, 0, 0, 0))
    image.alpha_composite(page.crop((x, y, x + w, y + h)), (int(ox), int(oy)))
    return image


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
        frame_columns = [_columns(len(row["rects"]), cw, ch) for row in spec["rows"]]
        subject_columns = [_columns(len(row["rects"]), CELL, CELL) for row in spec["rows"]]
        strips = [Image.new("RGBA", (cw * cols, ch * math.ceil(max(1, len(row["rects"])) / cols)), (0, 0, 0, 0)) for row, cols in zip(spec["rows"], frame_columns)]
        subjects = [Image.new("RGBA", (CELL * cols, CELL * math.ceil(max(1, len(row["rects"])) / cols)), (0, 0, 0, 0)) for row, cols in zip(spec["rows"], subject_columns)]
        neutral_rect = spec["rows"][spec["cover_row"]]["rects"][spec["cover_frame"]]
        with Image.open(spec["pages"][neutral_rect[4]]) as page:
            cover = _frame(page.convert("RGBA"), neutral_rect, spec["frame"])
        covers = {}
        neutral = cover.copy()
        for row, dest in spec.get("covers", {}).items():
            rects = spec["rows"][int(row)]["rects"]
            rect = rects[len(rects) // 2]
            with Image.open(spec["pages"][rect[4]]) as page:
                covers[dest] = _frame(page.convert("RGBA"), rect, spec["frame"])
            neutral.alpha_composite(covers[dest])
        camera = subject_camera(neutral, spec.get("body"), spec.get("feet"), spec.get("parts", []))
        for page in sorted(by_page):
            if page >= len(spec["pages"]):
                continue
            with Image.open(spec["pages"][page]) as im:
                sheet = im.convert("RGBA")
            for r, i, rect in by_page[page]:
                frame = _frame(sheet, rect, spec["frame"])
                strips[r].alpha_composite(frame.resize((cw, ch), Image.LANCZOS), ((i % frame_columns[r]) * cw, (i // frame_columns[r]) * ch))
                subjects[r].alpha_composite(subject_frame(frame, camera, CELL), ((i % subject_columns[r]) * CELL, (i // subject_columns[r]) * CELL))
            del sheet
        for row, strip, subject in zip(spec["rows"], strips, subjects):
            _save(strip, Path(row["dest"]))
            _save(subject, Path(row["subject_dest"]))
        _save(subject_frame(cover, camera, THUMB), Path(spec["cover_dest"]))
        for dest, image in covers.items():
            _save(subject_frame(image, camera, THUMB), Path(dest))
        Path(spec["framing_dest"]).write_text(json.dumps(camera))
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
    framing_outputs = []
    full_files = {product.file.resolve(): product for product in products.values() if product.root.of is None}
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
            metadata = next(products[key] for key in asset.products
                            if products[key].root.of is None and products[key].role in ("sheet_record", "sheet_layout"))
            pages = [full_files.get((metadata.file.parent / name).resolve()) for name in layout["pages"]]
            used = {rect[4] for row in layout["rows"] for rect in row["rects"]}
            if any(page >= len(pages) or pages[page] is None for page in used):
                asset.data.setdefault("errors", []).append("a page the layout names is not published")
                pages = []
            if pages:
                format_key = "subject-view-v2"
                key = _key(format_key, [p.sha256 if p else None for p in pages], layout["frame"], layout.get("body_bbox"),
                           layout.get("body_parts"), layout.get("feet"), layout["rows"], layout.get("cover_rows", []))
                base = out_dir / "strips" / key
                names = [r["name"] for r in layout["rows"]]
                cover_row = next((names.index(n) for n in COVER_ROWS if n in names), 0)
                cover_frame = len(layout["rows"][cover_row]["rects"]) // 2
                cw, ch, _ = _cell(tuple(layout["frame"]))
                spec_rows = []
                for i, row in enumerate(layout["rows"]):
                    dest = base / f"{i}.webp"
                    row["strip"] = rel(dest)
                    row["strip_columns"] = _columns(len(row["rects"]), cw, ch)
                    row["subject_columns"] = _columns(len(row["rects"]), CELL, CELL)
                    subject_dest = base / f"{i}-subject.webp"
                    row["subject_strip"] = rel(subject_dest)
                    spec_rows.append({"rects": row["rects"], "dest": str(dest), "subject_dest": str(subject_dest)})
                layout["cell"] = [cw, ch]
                framing_dest = base / "framing.json"
                layout["framing"] = {"cell": [CELL, CELL], "metadata": rel(framing_dest)}
                framing_outputs.append((layout["framing"], framing_dest))
                cover_dest = base / "cover.webp"
                covers = {str(names.index(name)): str(base / f"cover-{names.index(name)}.webp")
                          for name in layout.get("cover_rows", []) if name in names}
                layout["covers"] = {names[int(index)]: rel(Path(dest)) for index, dest in covers.items()}
                asset.cover = rel(cover_dest)
                if needed(cover_dest) or needed(framing_dest) or any(needed(Path(dest)) for dest in covers.values()) or any(needed(Path(r[name])) for r in spec_rows for name in ("dest", "subject_dest")):
                    tasks.append((sheet_task, ({
                        "pages": [str(p.file) if p else None for p in pages], "frame": layout["frame"], "rows": spec_rows,
                        "cover_row": cover_row, "cover_frame": cover_frame, "cover_dest": str(cover_dest),
                        "covers": covers,
                        "body": layout.get("body_bbox"), "feet": layout.get("feet"), "parts": layout.get("body_parts", []),
                        "framing_dest": str(framing_dest),
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
    for framing, path in framing_outputs:
        if path.is_file():
            framing.update(json.loads(path.read_text()))
    return errors


def prune(out_dir: Path, products: dict[str, Any], assets: dict[str, Any]) -> int:
    """Delete previews the index no longer names; the count deleted."""
    keep = {getattr(p, "thumb", None) for p in products.values()} | {a.cover for a in assets.values()}
    for a in assets.values():
        keep.update((a.data.get("layout") or {}).get("covers", {}).values())
        for row in (a.data.get("layout") or {}).get("rows", []):
            keep.add(row.get("strip"))
            keep.add(row.get("subject_strip"))
        keep.add((a.data.get("layout") or {}).get("framing", {}).get("metadata"))
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
