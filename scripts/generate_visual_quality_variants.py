#!/usr/bin/env python3
"""Generate visual-quality asset variants (smaller textures for weak hardware).

Post-publish helper: full-resolution sprite/background generation stays
unchanged, then this mirrors the installed folders into smaller-resolution
siblings the runtime loads under the Low / Medium / Potato quality profiles:

    sprites/                            -> sprites_0_5x/ sprites_0_25x/ sprites_potato/
    backgrounds/parallax_layers/        -> ..._0_5x/ ..._0_25x/ ..._potato/

The hard part this script gets right (and the first scaffold did not): a packed
``*_spritesheet.ron`` carries **pixel coordinates** (frame rects, trim offsets,
frame size, body/hit/hurt boxes, feet pixel) that index the PNG. Resizing the
PNG without rescaling those coordinates produces a *broken* atlas. So for each
sheet we rescale the PNG **and** every pixel-space field by one consistent
per-sheet factor, leaving normalized data (feet_anchor_norm, anchors, durations,
collision_scale) untouched.

``potato`` is the joke/extreme tier: it aims for ~1% of the authored size but
floors every frame at ``POTATO_MIN_FRAME_PX`` so atlases stay loadable. The
effective factor is therefore per-sheet and is *baked into the variant RON*, so
the runtime never needs to know it — it just reads the smaller rects.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path

from PIL import Image

POTATO_MIN_FRAME_PX = 8


@dataclass(frozen=True)
class Variant:
    suffix: str
    nominal_scale: float
    min_frame_px: int  # per-sheet floor so frames never collapse below this


VARIANTS: tuple[Variant, ...] = (
    Variant("0_5x", 0.5, 1),
    Variant("0_25x", 0.25, 1),
    Variant("potato", 0.02, POTATO_MIN_FRAME_PX),
)


# ──────────────────────────────────────────────────────────────────────────
# Minimal RON value model + parser.
#
# Handles exactly the constructs the spritesheet manifests use: lists, structs
# `(key: value, ...)`, positional tuples `(a, b)`, maps `{"k": v}`, `Some(..)`,
# `None`, strings, numbers, bools. Numbers keep their raw text so values we do
# NOT scale round-trip byte-for-byte (e.g. the long feet_anchor_norm floats).
# ──────────────────────────────────────────────────────────────────────────


class Struct:
    __slots__ = ("fields",)

    def __init__(self, fields: list[tuple[str, object]]):
        self.fields = fields

    def get(self, key: str):
        for k, v in self.fields:
            if k == key:
                return v
        return None


class Tuple_:
    __slots__ = ("items",)

    def __init__(self, items: list[object]):
        self.items = items


class List_:
    __slots__ = ("items",)

    def __init__(self, items: list[object]):
        self.items = items


class Map_:
    __slots__ = ("entries",)

    def __init__(self, entries: list[tuple[str, object]]):
        self.entries = entries


class Some_:
    __slots__ = ("inner",)

    def __init__(self, inner: object):
        self.inner = inner


class Num:
    __slots__ = ("raw",)

    def __init__(self, raw: str):
        self.raw = raw

    @property
    def is_float(self) -> bool:
        return "." in self.raw or "e" in self.raw or "E" in self.raw


class Str:
    __slots__ = ("value",)

    def __init__(self, value: str):
        self.value = value


class Atom:  # None, true, false, bare idents
    __slots__ = ("text",)

    def __init__(self, text: str):
        self.text = text


class RonParser:
    def __init__(self, text: str):
        self.s = text
        self.i = 0
        self.n = len(text)

    def parse(self) -> object:
        self._ws()
        value = self._value()
        self._ws()
        if self.i != self.n:
            raise ValueError(f"trailing RON at offset {self.i}: {self.s[self.i:self.i+40]!r}")
        return value

    def _ws(self) -> None:
        while self.i < self.n:
            c = self.s[self.i]
            if c in " \t\r\n":
                self.i += 1
            elif c == "/" and self.i + 1 < self.n and self.s[self.i + 1] == "/":
                while self.i < self.n and self.s[self.i] != "\n":
                    self.i += 1
            else:
                break

    def _value(self) -> object:
        self._ws()
        c = self.s[self.i]
        if c == "[":
            return self._list()
        if c == "{":
            return self._map()
        if c == "(":
            return self._paren()
        if c == '"':
            return self._string()
        if c == "-" or c.isdigit():
            return self._number()
        return self._ident()

    def _list(self) -> List_:
        self.i += 1  # [
        items: list[object] = []
        while True:
            self._ws()
            if self.s[self.i] == "]":
                self.i += 1
                return List_(items)
            items.append(self._value())
            self._ws()
            if self.s[self.i] == ",":
                self.i += 1

    def _map(self) -> Map_:
        self.i += 1  # {
        entries: list[tuple[str, object]] = []
        while True:
            self._ws()
            if self.s[self.i] == "}":
                self.i += 1
                return Map_(entries)
            key = self._string().value
            self._ws()
            assert self.s[self.i] == ":", f"map expected ':' at {self.i}"
            self.i += 1
            entries.append((key, self._value()))
            self._ws()
            if self.s[self.i] == ",":
                self.i += 1

    def _paren(self) -> object:
        # Either a struct `(key: v, ...)` or a positional tuple `(a, b)`.
        self.i += 1  # (
        self._ws()
        if self.s[self.i] == ")":
            self.i += 1
            return Tuple_([])
        if self._looks_like_struct():
            return self._struct_body()
        return self._tuple_body()

    def _looks_like_struct(self) -> bool:
        save = self.i
        try:
            if not (self.s[self.i].isalpha() or self.s[self.i] == "_"):
                return False
            while self.i < self.n and (self.s[self.i].isalnum() or self.s[self.i] == "_"):
                self.i += 1
            self._ws()
            return self.i < self.n and self.s[self.i] == ":"
        finally:
            self.i = save

    def _struct_body(self) -> Struct:
        fields: list[tuple[str, object]] = []
        while True:
            self._ws()
            if self.s[self.i] == ")":
                self.i += 1
                return Struct(fields)
            start = self.i
            while self.s[self.i].isalnum() or self.s[self.i] == "_":
                self.i += 1
            key = self.s[start : self.i]
            self._ws()
            assert self.s[self.i] == ":", f"struct expected ':' at {self.i}"
            self.i += 1
            fields.append((key, self._value()))
            self._ws()
            if self.s[self.i] == ",":
                self.i += 1

    def _tuple_body(self) -> Tuple_:
        items: list[object] = []
        while True:
            self._ws()
            if self.s[self.i] == ")":
                self.i += 1
                return Tuple_(items)
            items.append(self._value())
            self._ws()
            if self.s[self.i] == ",":
                self.i += 1

    def _string(self) -> Str:
        assert self.s[self.i] == '"'
        self.i += 1
        out: list[str] = []
        while self.s[self.i] != '"':
            c = self.s[self.i]
            if c == "\\":
                out.append(self.s[self.i : self.i + 2])
                self.i += 2
            else:
                out.append(c)
                self.i += 1
        self.i += 1
        return Str("".join(out))

    def _number(self) -> Num:
        start = self.i
        if self.s[self.i] == "-":
            self.i += 1
        while self.i < self.n and (self.s[self.i].isdigit() or self.s[self.i] in ".eE+-"):
            # Stop the scan if we hit a delimiter that can't be part of a number.
            if self.s[self.i] in "+-" and self.s[self.i - 1] not in "eE":
                break
            self.i += 1
        return Num(self.s[start : self.i])

    def _ident(self) -> object:
        start = self.i
        while self.i < self.n and (self.s[self.i].isalnum() or self.s[self.i] == "_"):
            self.i += 1
        word = self.s[start : self.i]
        self._ws()
        if word == "Some" and self.i < self.n and self.s[self.i] == "(":
            self.i += 1
            inner = self._value()
            self._ws()
            assert self.s[self.i] == ")", f"Some expected ')' at {self.i}"
            self.i += 1
            return Some_(inner)
        return Atom(word)


# ──────────────────────────────────────────────────────────────────────────
# Serializer
# ──────────────────────────────────────────────────────────────────────────


def dump(node: object) -> str:
    if isinstance(node, Struct):
        return "(" + ", ".join(f"{k}: {dump(v)}" for k, v in node.fields) + ")"
    if isinstance(node, Tuple_):
        return "(" + ", ".join(dump(v) for v in node.items) + ")"
    if isinstance(node, List_):
        return "[" + ", ".join(dump(v) for v in node.items) + "]"
    if isinstance(node, Map_):
        return "{" + ", ".join(f'"{k}": {dump(v)}' for k, v in node.entries) + "}"
    if isinstance(node, Some_):
        return f"Some({dump(node.inner)})"
    if isinstance(node, Num):
        return node.raw
    if isinstance(node, Str):
        return f'"{node.value}"'
    if isinstance(node, Atom):
        return node.text
    raise TypeError(f"cannot dump {type(node)}")


# ──────────────────────────────────────────────────────────────────────────
# Pixel-space rescale. Key-driven so we scale rects / sizes / feet pixels but
# never normalized anchors, durations, page indices, or tuning ratios.
# ──────────────────────────────────────────────────────────────────────────

PIXEL_INT_FIELDS = {"label_width", "frame_width", "frame_height", "y_offset"}
RECT_INT_FIELDS = {"x", "y", "w", "h"}


def _scale_int(node: Num, scale: float, floor: int = 0) -> Num:
    return Num(str(max(floor, round(int(node.raw) * scale))))


def _scale_float(node: Num, scale: float) -> Num:
    return Num(repr(float(node.raw) * scale))


def _scale_rect_struct(node: Struct, scale: float) -> Struct:
    # FrameRect / PixelRect / NamedPixelRect: scale x,y,w,h (+ the off trim
    # tuple); pass name, page, anchors (normalized) through untouched.
    out: list[tuple[str, object]] = []
    for key, value in node.fields:
        if key in RECT_INT_FIELDS and isinstance(value, Num):
            out.append((key, _scale_int(value, scale, floor=1 if key in ("w", "h") else 0)))
        elif key == "off" and isinstance(value, Tuple_):
            out.append((key, Tuple_([_scale_int(v, scale) for v in value.items])))
        else:
            out.append((key, value))
    return Struct(out)


def _scale_point_struct(node: Struct, scale: float) -> Struct:
    out: list[tuple[str, object]] = []
    for key, value in node.fields:
        if key in ("x", "y") and isinstance(value, Num):
            out.append((key, _scale_float(value, scale)))
        else:
            out.append((key, value))
    return Struct(out)


def _map_opt(node: object, fn) -> object:
    """Apply `fn` to a struct that may be wrapped in `Some(..)` (or be `None`)."""
    if isinstance(node, Some_):
        return Some_(_map_opt(node.inner, fn))
    if isinstance(node, Struct):
        return fn(node)
    return node  # None / Atom


def _scale_rect_list(node: object, scale: float) -> object:
    if isinstance(node, List_):
        return List_([_scale_rect_struct(v, scale) if isinstance(v, Struct) else v for v in node.items])
    return node


def _scale_animation_box(node: Struct, scale: float) -> Struct:
    out: list[tuple[str, object]] = []
    for key, value in node.fields:
        if key in ("parts",):
            out.append((key, _scale_rect_list(value, scale)))
        elif key == "bbox":
            out.append((key, _map_opt(value, lambda s: _scale_rect_struct(s, scale))))
        elif key == "poly" and isinstance(value, List_):
            out.append(
                (
                    key,
                    List_(
                        [
                            Tuple_([_scale_float(c, scale) if isinstance(c, Num) else c for c in t.items])
                            if isinstance(t, Tuple_)
                            else t
                            for t in value.items
                        ]
                    ),
                )
            )
        elif key == "frames" and isinstance(value, List_):
            out.append((key, List_([_scale_animation_box(v, scale) if isinstance(v, Struct) else v for v in value.items])))
        else:
            out.append((key, value))
    return Struct(out)


def _scale_animation_metrics(node: Struct, scale: float) -> Struct:
    out: list[tuple[str, object]] = []
    for key, value in node.fields:
        if key in ("hurtbox", "hitbox"):
            out.append((key, _map_opt(value, lambda s: _scale_animation_box(s, scale))))
        else:
            out.append((key, value))
    return Struct(out)


def _scale_body_metrics(node: Struct, scale: float) -> Struct:
    out: list[tuple[str, object]] = []
    for key, value in node.fields:
        if key == "body_pixel_bbox":
            out.append((key, _map_opt(value, lambda s: _scale_rect_struct(s, scale))))
        elif key == "body_pixel_parts":
            out.append((key, _scale_rect_list(value, scale)))
        elif key == "feet_pixel":
            out.append((key, _map_opt(value, lambda s: _scale_point_struct(s, scale))))
        elif key == "animations" and isinstance(value, Map_):
            out.append((key, Map_([(k, _scale_animation_metrics(v, scale)) for k, v in value.entries])))
        else:  # feet_anchor_norm + anything else: normalized / untouched
            out.append((key, value))
    return Struct(out)


def _scale_row(node: Struct, scale: float) -> Struct:
    out: list[tuple[str, object]] = []
    for key, value in node.fields:
        if key == "rects":
            out.append((key, _scale_rect_list(value, scale)))
        else:
            out.append((key, value))
    return Struct(out)


def scale_record(record: Struct, scale: float) -> Struct:
    out: list[tuple[str, object]] = []
    for key, value in record.fields:
        if key in PIXEL_INT_FIELDS and isinstance(value, Num):
            out.append((key, _scale_int(value, scale, floor=1)))
        elif key == "body_metrics":
            out.append((key, _map_opt(value, lambda s: _scale_body_metrics(s, scale))))
        elif key == "rows" and isinstance(value, List_):
            out.append((key, List_([_scale_row(v, scale) if isinstance(v, Struct) else v for v in value.items])))
        else:  # target, image, images, tuning, durations: untouched
            out.append((key, value))
    return Struct(out)


def _frame_size(record: Struct) -> tuple[int, int]:
    fw = record.get("frame_width")
    fh = record.get("frame_height")
    fw = int(fw.raw) if isinstance(fw, Num) else 0
    fh = int(fh.raw) if isinstance(fh, Num) else 0
    return fw, fh


def effective_scale(records: List_, variant: Variant) -> float:
    """One factor per sheet: the nominal scale, raised so no frame in the sheet
    falls below `variant.min_frame_px`. Clamped to <= 1.0 (never upscale)."""
    smallest = min(
        (min(d for d in _frame_size(r) if d > 0) for r in records.items if isinstance(r, Struct)),
        default=0,
    )
    floored = variant.nominal_scale
    if smallest > 0 and variant.min_frame_px > 0:
        floored = max(floored, variant.min_frame_px / smallest)
    return min(1.0, floored)


# ──────────────────────────────────────────────────────────────────────────
# PNG resize + folder mirroring
# ──────────────────────────────────────────────────────────────────────────


def resize_png(src: Path, dst: Path, scale: float, min_px: int = 1) -> None:
    with Image.open(src) as image:
        width = max(min_px, round(image.width * scale))
        height = max(min_px, round(image.height * scale))
        resized = image.resize((width, height), Image.Resampling.LANCZOS)
        dst.parent.mkdir(parents=True, exist_ok=True)
        resized.save(dst)


def page_filenames(record: Struct) -> list[str]:
    images = record.get("images")
    if isinstance(images, List_) and images.items:
        return [v.value for v in images.items if isinstance(v, Str)]
    image = record.get("image")
    return [image.value] if isinstance(image, Str) else []


def _resized_dim(original: int, scale: float, min_px: int) -> int:
    return max(min_px, round(original * scale))


def _clamp_rects_to_pages(record: Struct, page_dims: dict[int, tuple[int, int]]) -> None:
    """Clamp every scaled rect into its page's resized bounds. Independent
    per-field rounding can push a rect 1px past the edge; an out-of-bounds atlas
    cell samples garbage (or trips Bevy), so we pull it back in place."""
    rows = record.get("rows")
    if not isinstance(rows, List_):
        return
    for row in rows.items:
        if not isinstance(row, Struct):
            continue
        rects = row.get("rects")
        if not isinstance(rects, List_):
            continue
        for rect in rects.items:
            if not isinstance(rect, Struct):
                continue
            page = rect.get("page")
            page = int(page.raw) if isinstance(page, Num) else 0
            dims = page_dims.get(page)
            if dims is None:
                continue
            w_max, h_max = dims
            vals = {k: int(rect.get(k).raw) for k in RECT_INT_FIELDS if isinstance(rect.get(k), Num)}
            x = min(max(0, vals.get("x", 0)), max(0, w_max - 1))
            y = min(max(0, vals.get("y", 0)), max(0, h_max - 1))
            w = min(max(1, vals.get("w", 1)), w_max - x)
            h = min(max(1, vals.get("h", 1)), h_max - y)
            rect.fields = [
                (k, Num(str({"x": x, "y": y, "w": w, "h": h}[k]))) if k in RECT_INT_FIELDS else (k, v)
                for k, v in rect.fields
            ]


def process_sheet(ron_src: Path, ron_dst: Path, variant: Variant) -> int:
    """Rescale one `*_spritesheet.ron` + its page PNG(s). Returns pages written."""
    records = RonParser(ron_src.read_text()).parse()
    if not isinstance(records, List_):
        # Single-record top-level struct is unusual for these files, but support it.
        records = List_([records])
    scale = effective_scale(records, variant)
    scaled = List_([scale_record(r, scale) if isinstance(r, Struct) else r for r in records.items])

    # Resize page PNGs first so we know each page's exact resized bounds, then
    # clamp the scaled rects into them.
    pages = 0
    page_dims: dict[int, tuple[int, int]] = {}
    seen: set[str] = set()
    for record in records.items:
        if not isinstance(record, Struct):
            continue
        for page_index, fname in enumerate(page_filenames(record)):
            png_src = ron_src.parent / fname
            if not png_src.exists():
                continue
            with Image.open(png_src) as im:
                w0, h0 = im.width, im.height
            page_dims[page_index] = (
                _resized_dim(w0, scale, variant.min_frame_px),
                _resized_dim(h0, scale, variant.min_frame_px),
            )
            if fname not in seen:
                seen.add(fname)
                resize_png(png_src, ron_dst.parent / fname, scale, min_px=variant.min_frame_px)
                pages += 1

    for record in scaled.items:
        if isinstance(record, Struct):
            _clamp_rects_to_pages(record, page_dims)

    ron_dst.parent.mkdir(parents=True, exist_ok=True)
    ron_dst.write_text("[\n" + ",\n".join(dump(r) for r in scaled.items) + "\n]\n")
    return pages


def generate_sprite_variants(asset_root: Path) -> None:
    src = asset_root / "sprites"
    if not src.exists():
        print(f"skip missing sprite root: {src}")
        return
    for variant in VARIANTS:
        dst = asset_root / f"sprites_{variant.suffix}"
        sheet_pngs: set[Path] = set()
        sheets = 0
        for ron in sorted(src.rglob("*_spritesheet.ron")):
            rel = ron.relative_to(src)
            written = process_sheet(ron, dst / rel, variant)
            sheets += 1
            for fname in page_filenames_safe(ron):
                sheet_pngs.add((ron.parent / fname).resolve())
        # Standalone PNGs (entities/, loose) not owned by a sheet.
        loose = 0
        for png in sorted(src.rglob("*.png")):
            if png.resolve() in sheet_pngs:
                continue
            resize_png(png, dst / png.relative_to(src), variant.nominal_scale, min_px=variant.min_frame_px)
            loose += 1
        print(f"sprites {variant.suffix}: {sheets} sheets, {loose} loose png (scale~{variant.nominal_scale}) -> {dst}")


def page_filenames_safe(ron: Path) -> list[str]:
    try:
        records = RonParser(ron.read_text()).parse()
    except Exception:
        return []
    if isinstance(records, Struct):
        records = List_([records])
    if not isinstance(records, List_):
        return []
    names: list[str] = []
    for record in records.items:
        if isinstance(record, Struct):
            names.extend(page_filenames(record))
    return names


def generate_parallax_variants(asset_root: Path) -> None:
    src = asset_root / "backgrounds" / "parallax_layers"
    if not src.exists():
        print(f"skip missing parallax root: {src}")
        return
    for variant in VARIANTS:
        dst = asset_root / "backgrounds" / f"parallax_layers_{variant.suffix}"
        pngs = 0
        for png in sorted(src.rglob("*.png")):
            resize_png(png, dst / png.relative_to(src), variant.nominal_scale, min_px=variant.min_frame_px)
            pngs += 1
        print(f"parallax {variant.suffix}: {pngs} png (scale {variant.nominal_scale}) -> {dst}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--asset-root",
        type=Path,
        default=Path("crates/ambition_gameplay_core/assets"),
        help="gameplay-core asset root containing sprites/ and backgrounds/",
    )
    parser.add_argument("--sprites-only", action="store_true")
    parser.add_argument("--backgrounds-only", action="store_true")
    args = parser.parse_args()

    asset_root = args.asset_root.resolve()
    if not args.backgrounds_only:
        generate_sprite_variants(asset_root)
    if not args.sprites_only:
        generate_parallax_variants(asset_root)


if __name__ == "__main__":
    main()
