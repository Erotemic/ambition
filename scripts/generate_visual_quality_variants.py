#!/usr/bin/env python3
"""Generate visual-quality asset variants (smaller textures for weak hardware).

Post-publish helper: full-resolution sprite/background generation stays
unchanged, then this mirrors the installed folders into smaller-resolution
siblings the runtime loads under the Low / Medium / Potato quality profiles:

    sprites/                            -> sprites_0_5x/ sprites_0_25x/ sprites_potato/
    backgrounds/parallax_layers/        -> ..._0_5x/ ..._0_25x/ ..._potato/

Quality is a publish-time asset-generation decision, not an atlas postprocess.
Never resize an already-packed actor/character atlas page to make a quality
variant: doing so lets packed neighbours bleed into frame edges and makes rect /
anchor rounding drift. The primary path asks scale-aware sprite targets to render
source/vector art directly at the tier's texture budget, then packs those frames
into fresh tier-local atlas pages. Sheets whose source target has not yet been
lifted onto that scale-aware protocol use a fallback that crops each authored
frame in isolation, downsamples the isolated crop, and repacks the result.
Background/parallax variants are still whole-image resizes because they are
standalone images, not packed character atlases.

``potato`` is the joke/extreme tier: it aims for 1/16 of the authored size but
floors every frame at ``POTATO_MIN_FRAME_PX`` so atlases stay loadable, and it
uses nearest-neighbour reduction for deliberate crunchy pixels. The effective
factor is per-sheet and is *baked into the variant RON*, so the runtime never
needs to know it — it just reads the smaller rects.

Work selection
--------------
The unit of work is one SHEET (one character/prop), and the tiers are the INNER
loop. That ordering is what makes less-than-everything expressible: a sheet owns
its own destination files rather than a tier owning a directory it just deleted,
so a sheet can be skipped when it is already current, rebuilt on its own, or
(one day) built in parallel with its neighbours. It also parses each sheet's RON
and decodes its page PNGs once for all three tiers instead of three times.

    --target NAME       only sheets / loose PNGs matching NAME (fnmatch,
                        repeatable) — e.g. ``--target patent_clerk``,
                        ``--target 'pirate_*'``
    --tier 0_5x         only that quality tier (repeatable)
    --force             rebuild even when the outputs are newer than the source
    --clean             delete each selected tier root first (whole-tier only)

Without ``--force`` a sheet is rebuilt only when its authored RON, one of its
page PNGs, or this script is newer than the variant it already published. An
unfiltered run also prunes variant files whose source no longer exists, so
dropping the tier-wide ``rmtree`` did not trade freshness for orphans.
"""

from __future__ import annotations

import argparse
import fnmatch
import time
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
import shutil
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))
from publish_safely import break_mirrored_destination  # noqa: E402

from PIL import Image

REPO_ROOT = Path(__file__).resolve().parents[1]
SPRITE_RENDERER_ROOT = REPO_ROOT / "tools" / "ambition_sprite2d_renderer"
if str(SPRITE_RENDERER_ROOT) not in sys.path:
    sys.path.insert(0, str(SPRITE_RENDERER_ROOT))

from ambition_sprite2d_renderer.authoring.packer import FrameInput, pack_frames  # noqa: E402
from ambition_sprite2d_renderer.registry import (  # noqa: E402
    Target,
    discover_all_targets,
)

POTATO_MIN_FRAME_PX = 8
POTATO_SOURCE_RENDER_FLOOR = 0.25


@dataclass(frozen=True)
class Variant:
    suffix: str
    nominal_scale: float
    min_frame_px: int  # per-sheet floor so frames never collapse below this


VARIANTS: tuple[Variant, ...] = (
    Variant("0_5x", 0.5, 1),
    Variant("0_25x", 0.25, 1),
    Variant("potato", 1.0 / 16.0, POTATO_MIN_FRAME_PX),
)

# Editing this file changes every variant's pixels, so its own mtime is part of
# every freshness comparison. Nothing else in the repo is: a target's generator
# code reaches the tiers only through the published sheet, which regen_sprites
# rewrites when that code changes.
_SCRIPT_MTIME = Path(__file__).resolve().stat().st_mtime


# ──────────────────────────────────────────────────────────────────────────
# Phase timing
#
# A counter, not a profiler. The question a slow run raises here is "which
# PHASE, and which CHARACTER, cost the minutes" — `cProfile` answers a
# different one (which call), at a multiple of the runtime, and its call graph
# is dominated by PIL internals shared by all three phases anyway.
# ──────────────────────────────────────────────────────────────────────────


class PhaseClock:
    """Wall-clock seconds and call counts per named phase, plus per-unit totals."""

    def __init__(self) -> None:
        self.seconds: dict[str, float] = {}
        self.calls: dict[str, int] = {}
        self.units: dict[str, float] = {}
        self.unit_phase: dict[str, str] = {}

    @contextmanager
    def phase(self, name: str, unit: str | None = None):
        started = time.perf_counter()
        try:
            yield
        finally:
            elapsed = time.perf_counter() - started
            self.seconds[name] = self.seconds.get(name, 0.0) + elapsed
            self.calls[name] = self.calls.get(name, 0) + 1
            if unit is not None:
                self.units[unit] = self.units.get(unit, 0.0) + elapsed
                self.unit_phase[unit] = name

    def report(self, *, top: int = 10) -> None:
        if not self.seconds:
            return
        total = sum(self.seconds.values())
        print(f"\n==> phase cost ({total:.1f}s attributed)")
        for name, seconds in sorted(
            self.seconds.items(), key=lambda kv: kv[1], reverse=True
        ):
            share = 100.0 * seconds / total if total else 0.0
            print(
                f"    {seconds:8.1f}s  {share:5.1f}%  {name}"
                f"  ({self.calls[name]} calls)"
            )
        if not self.units:
            return
        ranked = sorted(self.units.items(), key=lambda kv: kv[1], reverse=True)[:top]
        print(f"==> slowest {len(ranked)} units (all selected tiers summed)")
        for unit, seconds in ranked:
            print(f"    {seconds:8.1f}s  {unit}  [{self.unit_phase[unit]}]")


# ──────────────────────────────────────────────────────────────────────────
# Selection
# ──────────────────────────────────────────────────────────────────────────


def selects(name: str, selectors: tuple[str, ...]) -> bool:
    """True when no selector was given, or one matches `name` (fnmatch)."""
    if not selectors:
        return True
    return any(fnmatch.fnmatch(name, pattern) for pattern in selectors)


def resolve_tiers(names: tuple[str, ...]) -> tuple[Variant, ...]:
    if not names:
        return VARIANTS
    by_suffix = {variant.suffix: variant for variant in VARIANTS}
    unknown = [name for name in names if name not in by_suffix]
    if unknown:
        raise SystemExit(
            f"unknown tier(s) {unknown}; choose from {sorted(by_suffix)}"
        )
    return tuple(by_suffix[name] for name in dict.fromkeys(names))


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
            raise ValueError(
                f"trailing RON at offset {self.i}: {self.s[self.i : self.i + 40]!r}"
            )
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
            while self.i < self.n and (
                self.s[self.i].isalnum() or self.s[self.i] == "_"
            ):
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
        while self.i < self.n and (
            self.s[self.i].isdigit() or self.s[self.i] in ".eE+-"
        ):
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
            out.append(
                (key, _scale_int(value, scale, floor=1 if key in ("w", "h") else 0))
            )
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
        return List_(
            [
                _scale_rect_struct(v, scale) if isinstance(v, Struct) else v
                for v in node.items
            ]
        )
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
                            Tuple_(
                                [
                                    _scale_float(c, scale) if isinstance(c, Num) else c
                                    for c in t.items
                                ]
                            )
                            if isinstance(t, Tuple_)
                            else t
                            for t in value.items
                        ]
                    ),
                )
            )
        elif key == "frames" and isinstance(value, List_):
            out.append(
                (
                    key,
                    List_(
                        [
                            _scale_animation_box(v, scale)
                            if isinstance(v, Struct)
                            else v
                            for v in value.items
                        ]
                    ),
                )
            )
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
            out.append(
                (
                    key,
                    Map_(
                        [
                            (k, _scale_animation_metrics(v, scale))
                            for k, v in value.entries
                        ]
                    ),
                )
            )
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
            out.append(
                (
                    key,
                    List_(
                        [
                            _scale_row(v, scale) if isinstance(v, Struct) else v
                            for v in value.items
                        ]
                    ),
                )
            )
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
        (
            min(d for d in _frame_size(r) if d > 0)
            for r in records.items
            if isinstance(r, Struct)
        ),
        default=0,
    )
    floored = variant.nominal_scale
    if smallest > 0 and variant.min_frame_px > 0:
        floored = max(floored, variant.min_frame_px / smallest)
    return min(1.0, floored)


# ──────────────────────────────────────────────────────────────────────────
# PNG resize + folder mirroring
# ──────────────────────────────────────────────────────────────────────────


def resampling_for_variant(variant: Variant) -> Image.Resampling:
    if variant.suffix == "potato":
        return Image.Resampling.NEAREST
    return Image.Resampling.LANCZOS


def resize_png(
    src: Path,
    dst: Path,
    scale: float,
    min_px: int = 1,
    *,
    resampling: Image.Resampling = Image.Resampling.LANCZOS,
) -> None:
    with Image.open(src) as image:
        width = max(min_px, round(image.width * scale))
        height = max(min_px, round(image.height * scale))
        resized = image.resize((width, height), resampling)
        dst.parent.mkdir(parents=True, exist_ok=True)
        break_symlink_before_write(dst)
        resized.save(dst)


def break_symlink_before_write(path: Path) -> None:
    """Replace a symlinked destination with a real file, never write through it.

    ⛔⛔ AN ASSET-MIRRORED WORKTREE POINTS EVERY GENERATED FILE AT THE MAIN
    CHECKOUT'S COPY. `scripts/mirror_assets_for_worktree.py` symlinks file by
    file precisely so that "a regenerated sprite lands as a REAL file in the
    worktree ... and the main checkout never sees it" — but that is only true if
    the writer UNLINKS FIRST. `Image.save()` and `Path.write_text()` open the
    path for writing, and an open-for-write FOLLOWS the symlink: the bytes land
    in the shared main checkout, silently, for every agent regenerating assets
    in a worktree.

    Verified 2026-09-02 rather than reasoned about: a plain
    `open(link, "wb").write(...)` replaced the TARGET's contents and left the
    link in place.

    ⚠ The main checkout is what other sessions build and gate from, so this is
    not a tidiness rule — it is the difference between regenerating your own
    assets and quietly editing everyone else's.

    ⇒ **The implementation now lives in `scripts/lib/publish_safely.py`**, shared
    with every other publisher into a mirrored tree — the SFX bank and its dump
    were writing straight through the link while this generator was already
    safe, and one guarded road out of four is not a guarded invariant. This stays
    as the name the call sites here already use.
    """
    break_mirrored_destination(path)


def page_filenames(record: Struct) -> list[str]:
    images = record.get("images")
    if isinstance(images, List_) and images.items:
        return [v.value for v in images.items if isinstance(v, Str)]
    image = record.get("image")
    return [image.value] if isinstance(image, Str) else []


# Diagnostic filename suffixes / gallery dirs the sprite generators emit next to
# runtime art. Mirrors DIAGNOSTIC_SUFFIXES / DIAGNOSTIC_DIRS in
# scripts/sweep_runtime_diagnostics.py and asset_publish::classify.rs. The
# variant generator's loose-png pass must skip these so downscaled diagnostics
# never leak into the (gitignored) quality-variant roots.
_DIAGNOSTIC_SUFFIXES = (
    "_canonical.png",
    "_canonical_transparent.png",
    "_preview_labeled.png",
    "_parts_debug.png",
    "_debug.png",
)
_DIAGNOSTIC_DIRS = ("canonicals", "diagnostics")


def is_diagnostic_png(rel_path: Path) -> bool:
    """True if a png (relative to the sprites root) is a visual diagnostic."""
    if any(part in _DIAGNOSTIC_DIRS for part in rel_path.parts):
        return True
    name = rel_path.name
    return name == "canonicals_contact_sheet.png" or name.endswith(_DIAGNOSTIC_SUFFIXES)


def _resized_dim(original: int, scale: float, min_px: int) -> int:
    return max(min_px, round(original * scale))


def _set_num_field(node: Struct, key: str, value: int) -> None:
    for idx, (field, old) in enumerate(node.fields):
        if field == key:
            node.fields[idx] = (field, Num(str(value)))
            return
    node.fields.append((key, Num(str(value))))


def _set_tuple_field(node: Struct, key: str, values: tuple[int, int]) -> None:
    new_value = Tuple_([Num(str(values[0])), Num(str(values[1]))])
    for idx, (field, old) in enumerate(node.fields):
        if field == key:
            node.fields[idx] = (field, new_value)
            return
    node.fields.append((key, new_value))


def _set_str_field(node: Struct, key: str, value: str) -> None:
    for idx, (field, old) in enumerate(node.fields):
        if field == key:
            node.fields[idx] = (field, Str(value))
            return
    node.fields.append((key, Str(value)))


def _set_list_field(node: Struct, key: str, values: list[object]) -> None:
    for idx, (field, old) in enumerate(node.fields):
        if field == key:
            node.fields[idx] = (field, List_(values))
            return
    node.fields.append((key, List_(values)))


def _remove_field(node: Struct, key: str) -> None:
    node.fields = [(field, value) for field, value in node.fields if field != key]


def _iter_frame_rects(record: Struct):
    rows = record.get("rows")
    if not isinstance(rows, List_):
        return
    for row_index, row in enumerate(rows.items):
        if not isinstance(row, Struct):
            continue
        rects = row.get("rects")
        if not isinstance(rects, List_):
            continue
        for frame_index, rect in enumerate(rects.items):
            if not isinstance(rect, Struct):
                continue
            yield row_index, frame_index, row, rect


def _rect_value(rect: Struct, key: str, default: int = 0) -> int:
    value = rect.get(key)
    return int(value.raw) if isinstance(value, Num) else default


def _page_for_rect(rect: Struct) -> int:
    page = rect.get("page")
    return int(page.raw) if isinstance(page, Num) else 0


def _off_for_rect(rect: Struct) -> tuple[int, int]:
    off = rect.get("off")
    if isinstance(off, Tuple_) and len(off.items) >= 2:
        x, y = off.items[:2]
        if isinstance(x, Num) and isinstance(y, Num):
            return int(x.raw), int(y.raw)
    return 0, 0


def _set_page_images(records: List_, page_names: list[str]) -> None:
    for record in records.items:
        if not isinstance(record, Struct):
            continue
        _set_str_field(record, "image", page_names[0])
        if len(page_names) > 1:
            _set_list_field(record, "images", [Str(name) for name in page_names])
        else:
            _remove_field(record, "images")


def _page_image_names(first_image: str, page_count: int) -> list[str]:
    if page_count <= 1:
        return [first_image]
    src = Path(first_image)
    stem = src.stem
    suffix = src.suffix.lstrip(".")
    return [src.name] + [f"{stem}.{page}.{suffix}" for page in range(1, page_count)]


def _copy_non_atlas_metadata(scaled_rect: Struct, placement) -> None:
    _set_num_field(scaled_rect, "x", placement.x)
    _set_num_field(scaled_rect, "y", placement.y)
    _set_num_field(scaled_rect, "w", placement.w)
    _set_num_field(scaled_rect, "h", placement.h)
    _set_num_field(scaled_rect, "page", placement.page)
    _set_tuple_field(scaled_rect, "off", (placement.off_x, placement.off_y))


def _frame_crop_from_rect(
    page_images: dict[int, Image.Image], rect: Struct
) -> Image.Image:
    page_index = _page_for_rect(rect)
    source_page = page_images[page_index]
    x = _rect_value(rect, "x")
    y = _rect_value(rect, "y")
    w = max(1, _rect_value(rect, "w", 1))
    h = max(1, _rect_value(rect, "h", 1))
    return source_page.crop((x, y, x + w, y + h)).convert("RGBA")


def _scaled_frame_crop(
    crop: Image.Image, scale: float, variant: Variant
) -> Image.Image:
    width = _resized_dim(crop.width, scale, variant.min_frame_px)
    height = _resized_dim(crop.height, scale, variant.min_frame_px)
    return crop.resize((width, height), resampling_for_variant(variant))


def _logical_frame_from_crop(
    scaled_crop: Image.Image,
    logical_size: tuple[int, int],
    offset: tuple[int, int],
) -> Image.Image:
    logical = Image.new("RGBA", logical_size, (0, 0, 0, 0))
    ox, oy = offset
    crop = scaled_crop
    if ox < 0 or oy < 0:
        crop = crop.crop((max(0, -ox), max(0, -oy), crop.width, crop.height))
        ox = max(0, ox)
        oy = max(0, oy)
    if ox >= logical.width or oy >= logical.height:
        return logical
    if ox + crop.width > logical.width or oy + crop.height > logical.height:
        crop = crop.crop((0, 0, logical.width - ox, logical.height - oy))
    if crop.width > 0 and crop.height > 0:
        logical.alpha_composite(crop, (ox, oy))
    return logical


class SheetSource:
    """One authored spritesheet, parsed ONCE and shared by every tier.

    This object is the reason the loop is per-sheet outer / tier inner. With
    the tier outer the same RON was parsed three times and the same page PNGs
    decoded three times, and the tier owned a destination directory it had
    just deleted — so no sheet could be skipped, rebuilt alone, or built
    beside its neighbours.
    """

    __slots__ = ("ron", "records", "_pages", "_page_names")

    def __init__(self, ron: Path, records: List_) -> None:
        self.ron = ron
        self.records = records
        self._pages: dict[int, Image.Image] | None = None
        self._page_names: list[str] = []

    @classmethod
    def load(cls, ron: Path) -> "SheetSource":
        records = RonParser(ron.read_text()).parse()
        if not isinstance(records, List_):
            # Single-record top-level struct is unusual here, but support it.
            records = List_([records])
        return cls(ron, records)

    def _decode(self) -> None:
        pages: dict[int, Image.Image] = {}
        names: list[str] = []
        for record in self.records.items:
            if not isinstance(record, Struct):
                continue
            for page_index, fname in enumerate(page_filenames(record)):
                if page_index in pages:
                    continue
                png_src = self.ron.parent / fname
                if png_src.exists():
                    pages[page_index] = Image.open(png_src).convert("RGBA")
                    names.append(fname)
        self._pages = pages
        self._page_names = names

    @property
    def pages(self) -> dict[int, Image.Image]:
        if self._pages is None:
            self._decode()
        assert self._pages is not None
        return self._pages

    @property
    def page_names(self) -> list[str]:
        if self._pages is None:
            self._decode()
        return self._page_names

    def close(self) -> None:
        for image in (self._pages or {}).values():
            image.close()
        self._pages = None


def build_sheet_variant(source: SheetSource, ron_dst: Path, variant: Variant) -> int:
    """Publish one quality-tier `*_spritesheet.ron` + freshly packed page PNGs.

    Actor/character quality variants are generated from isolated source frames,
    never by resizing the already-packed atlas page.
    """
    records = source.records
    scale = effective_scale(records, variant)
    scaled = List_(
        [scale_record(r, scale) if isinstance(r, Struct) else r for r in records.items]
    )

    source_pages = source.pages
    source_page_names = source.page_names
    if not source_pages:
        return 0

    frames: list[FrameInput] = []
    for record_index, (record, scaled_record) in enumerate(
        zip(records.items, scaled.items)
    ):
        if not isinstance(record, Struct) or not isinstance(scaled_record, Struct):
            continue
        frame_width, frame_height = _frame_size(scaled_record)
        original_rects = list(_iter_frame_rects(record))
        scaled_rects = list(_iter_frame_rects(scaled_record))
        for (
            (row_index, frame_index, _row, rect),
            (_scaled_row_index, _scaled_frame_index, _scaled_row, scaled_rect),
        ) in zip(original_rects, scaled_rects):
            crop = _frame_crop_from_rect(source_pages, rect)
            scaled_crop = _scaled_frame_crop(crop, scale, variant)
            logical_frame = _logical_frame_from_crop(
                scaled_crop,
                (frame_width, frame_height),
                _off_for_rect(scaled_rect),
            )
            key = (record_index, row_index, frame_index)
            frames.append(
                FrameInput(
                    key=key,
                    image=logical_frame,
                    logical_size=(frame_width, frame_height),
                )
            )

    result = pack_frames(frames, max_dim=16384, page_size=4096, padding=1, trim=True)
    for key, placement in result.placements.items():
        record_index, row_index, frame_index = key
        scaled_record = scaled.items[record_index]
        if not isinstance(scaled_record, Struct):
            continue
        for (
            candidate_row_index,
            candidate_frame_index,
            _row,
            scaled_rect,
        ) in _iter_frame_rects(scaled_record):
            if (
                candidate_row_index == row_index
                and candidate_frame_index == frame_index
            ):
                _copy_non_atlas_metadata(scaled_rect, placement)
                break

    first_image = (
        source_page_names[0]
        if source_page_names
        else page_filenames(records.items[0])[0]
    )
    page_names = _page_image_names(first_image, len(result.pages))
    _set_page_images(scaled, page_names)

    ron_dst.parent.mkdir(parents=True, exist_ok=True)
    for page, fname in zip(result.pages, page_names):
        page_dst = ron_dst.parent / fname
        break_symlink_before_write(page_dst)
        page.save(page_dst)

    # the decoded source pages belong to `source`, which the caller reuses for
    # the remaining tiers. Closing them here (as this did while the tier was the
    # outer loop and each sheet was opened afresh) leaves the next tier reading
    # from a closed image.
    break_symlink_before_write(ron_dst)
    ron_dst.write_text("[\n" + ",\n".join(dump(r) for r in scaled.items) + "\n]\n")
    return len(result.pages)


def source_publishable_targets() -> dict[str, Target]:
    """Targets that can render quality tiers directly from vector/source data.

    The quality publisher's primary path is source render -> pack. Today the
    YAML config surface is scale-aware end-to-end; module-authored targets are
    left on the frame-local fallback until their render functions accept the
    same `quality_scale` protocol.

    The renderer unified `TackonTarget`/`AdapterTarget` into one `Target` with a
    `kind` discriminator, so this selects `kind == "config"` — the YAML-authored
    path that `AdapterTarget` used to name.
    """
    report = discover_all_targets()
    return {
        name: target
        for name, target in report.targets.items()
        if target.kind == "config"
    }


def publish_source_quality_target(
    target: Target,
    variant: Variant,
    dst_root: Path,
) -> int:
    out_dir = SPRITE_RENDERER_ROOT / "generated" / f"{target.name}_{variant.suffix}"
    out_dir.mkdir(parents=True, exist_ok=True)
    source_scale = effective_source_quality_scale(variant)
    if variant.suffix == "potato":
        source_scale = max(source_scale, POTATO_SOURCE_RENDER_FLOOR)
    target.render_sheet(
        out_dir,
        quality_scale=source_scale,
        downsample="nearest" if variant.suffix == "potato" else "lanczos",
    )
    if variant.suffix == "potato" and source_scale > variant.nominal_scale:
        ron_src = out_dir / f"{target.name}_spritesheet.ron"
        if not ron_src.exists():
            return 0
        final_variant = Variant(
            variant.suffix,
            variant.nominal_scale / source_scale,
            variant.min_frame_px,
        )
        intermediate = SheetSource.load(ron_src)
        try:
            return build_sheet_variant(
                intermediate,
                dst_root / f"{target.name}_spritesheet.ron",
                final_variant,
            )
        finally:
            intermediate.close()
    copied = target.install(out_dir, dst_root)
    return sum(
        1 for path in copied if path.suffix == ".png" and "_canonical" not in path.name
    )


def effective_source_quality_scale(variant: Variant) -> float:
    """Fraction of a target's normal source render scale for this tier.

    Adapter targets multiply this by their own full-quality `render_scale`, so a
    normal `render_scale=2` target emits `0_5x` at 1.0, `0_25x` at 0.5, and
    `potato` at 0.125.
    """
    return variant.nominal_scale


def sheet_page_claims(src: Path) -> dict[Path, list[Path]]:
    """Every authored manifest under `src`, mapped to the page PNGs it names.

    Parsed once and threaded through the run: the duplicate-page guard, the
    freshness stamps, and the loose-PNG exclusion set all ask this same
    question, and each used to re-parse all 190 manifests to answer it.
    """
    return {
        ron: [ron.parent / fname for fname in page_filenames_safe(ron)]
        for ron in sorted(src.rglob("*_spritesheet.ron"))
    }


def assert_one_manifest_per_page(claims: dict[Path, list[Path]], src: Path) -> None:
    """Refuse to publish while two manifests name the same page PNG.

    ⛔ this is silent corruption, not a tidiness rule. `build_sheet_variant`
    writes its freshly packed page to the filename the RON's `image` field names, so
    two `*_spritesheet.ron` claiming one PNG means whichever sorts LAST
    overwrites the other's page — while the loser's rects stay in its own RON,
    describing a packing that no longer exists. Nothing fails: both RONs parse,
    every file is present, and the asset contract is satisfied. It surfaces on
    a device as one character cropping garbage out of a correct-looking sheet.


    ⚠ counts distinct FILES, not records: one multi-record manifest sharing a
    page across its own records (the lab props) is the legitimate case.
    """
    claimants: dict[Path, set[Path]] = {}
    for ron, pages in claims.items():
        for page in pages:
            claimants.setdefault(page.resolve(), set()).add(ron)
    contested = {png: rons for png, rons in claimants.items() if len(rons) > 1}
    if not contested:
        return
    lines = ["a page PNG is claimed by more than one spritesheet manifest:"]
    for png, rons in sorted(contested.items()):
        lines.append(f"  {png.relative_to(src)}")
        for ron in sorted(rons):
            lines.append(f"      claimed by {ron.relative_to(src)}")
    lines.append(
        "Each manifest must name its own page. A manifest left behind by a "
        "renamed or split target is the usual cause — delete it rather than "
        "renaming its page, since nothing regenerates it."
    )
    raise SystemExit("\n".join(lines))


@dataclass(frozen=True)
class SheetUnit:
    """One sheet's worth of work, across every selected tier."""

    stem: str
    ron: Path
    rel: Path
    pages: tuple[Path, ...]
    target: Target | None  # not None => render this tier from source art


def _mtime(path: Path) -> float | None:
    try:
        return path.stat().st_mtime
    except OSError:
        return None


def _input_stamp(inputs: tuple[Path, ...]) -> float:
    """Newest mtime among a unit's inputs (this script always counts)."""
    stamps = [_SCRIPT_MTIME]
    stamps.extend(m for m in (_mtime(path) for path in inputs) if m is not None)
    return max(stamps)


def _published_and_current(ron_dst: Path, stamp: float) -> bool:
    """True when this tier's RON and every page it names post-date `stamp`."""
    published = _mtime(ron_dst)
    if published is None or published < stamp:
        return False
    names = page_filenames_safe(ron_dst)
    if not names:
        return False
    for name in names:
        page = _mtime(ron_dst.parent / name)
        if page is None or page < stamp:
            return False
    return True


def _source_counterpart(rel: Path) -> Path:
    """The authored file a variant file mirrors.

    Almost always the same relative path; a multi-page atlas publishes
    `<stem>.<n>.<ext>` siblings of `<stem>.<ext>`, and those trace back to the
    same authored sheet.
    """
    parts = rel.name.split(".")
    if len(parts) >= 3 and parts[-2].isdigit():
        return rel.with_name(".".join(parts[:-2] + parts[-1:]))
    return rel


def prune_orphans(src: Path, dst: Path) -> int:
    """Delete variant files whose authored counterpart is gone.

    This is what the tier-wide `rmtree` used to accomplish incidentally, at the
    cost of making every run a full rebuild. Deleting a character now removes
    its three variants on the next unfiltered run, and nothing else moves.
    """
    if not dst.exists():
        return 0
    removed = 0
    for path in sorted(dst.rglob("*")):
        if not path.is_file():
            continue
        if (src / _source_counterpart(path.relative_to(dst))).exists():
            continue
        path.unlink()
        removed += 1
    return removed


def verify_published_sheets(
    asset_root: Path,
    units: list["SheetUnit"],
    tiers: tuple[Variant, ...],
) -> None:
    """Postcondition, DERIVED from the plan this run just executed.

    ⛔ the shape matters more than the check. Both this script's wrapper and
    `scripts/regen/sprites.sh` used to assert a hand-written list of filenames; both
    lists drifted off the target list (`player_robot_spritesheet.*` has had no
    producer for months, and the wrapper spent weeks asserting the same dead
    stem), and a list that names a file nothing publishes can only ever fail —
    aborting real work over an entry nobody maintains. A postcondition
    generated from the units actually planned cannot drift, because there is
    no second list to drift from.
    """
    missing: list[Path] = []
    for unit in units:
        for variant in tiers:
            ron_dst = asset_root / f"sprites_{variant.suffix}" / unit.rel
            if not ron_dst.is_file():
                missing.append(ron_dst)
                continue
            names = page_filenames_safe(ron_dst)
            if not names:
                missing.append(ron_dst)
                continue
            missing.extend(
                ron_dst.parent / name
                for name in names
                if not (ron_dst.parent / name).is_file()
            )
    if not missing:
        return
    lines = [f"{len(missing)} planned quality-variant file(s) were not published:"]
    lines.extend(f"    {path}" for path in missing[:20])
    if len(missing) > 20:
        lines.append(f"    … and {len(missing) - 20} more")
    raise SystemExit("\n".join(lines))


def generate_sprite_variants(
    asset_root: Path,
    *,
    tiers: tuple[Variant, ...] = VARIANTS,
    selectors: tuple[str, ...] = (),
    force: bool = False,
    clean: bool = False,
    clock: PhaseClock | None = None,
) -> None:
    """Publish each selected sheet into each selected tier.

    ⛔ the loop is per-sheet OUTER, tier INNER, and that is not a preference:
    the tier-outer form owned a destination directory it had just deleted, so
    "rebuild just the patent clerk" could not be expressed at all. Ultrapack's
    loop stays tier-outer for the opposite reason — its pages are shared across
    characters, so no page can be finalized until every character is known.
    """
    clock = clock or PhaseClock()
    src = asset_root / "sprites"
    if not src.exists():
        print(f"skip missing sprite root: {src}")
        return

    with clock.phase("manifest scan"):
        claims = sheet_page_claims(src)
        assert_one_manifest_per_page(claims, src)
        source_targets = source_publishable_targets()

    units: list[SheetUnit] = []
    for ron, pages in claims.items():
        stem = ron.name.removesuffix("_spritesheet.ron")
        if not selects(stem, selectors):
            continue
        target = source_targets.get(stem)
        units.append(
            SheetUnit(
                stem=stem,
                ron=ron,
                rel=ron.relative_to(src),
                pages=tuple(pages),
                target=target if (target is not None and ron.parent == src) else None,
            )
        )

    sheet_pngs = {page.resolve() for pages in claims.values() for page in pages}
    loose_srcs: list[Path] = []
    for png in sorted(src.rglob("*.png")):
        rel = png.relative_to(src)
        if png.resolve() in sheet_pngs or is_diagnostic_png(rel):
            continue  # a sheet page, or a human-only diagnostic that never ships
        if selects(rel.with_suffix("").as_posix(), selectors) or selects(
            png.stem, selectors
        ):
            loose_srcs.append(png)

    if selectors and not units and not loose_srcs:
        raise SystemExit(
            f"no sprite sheet or loose PNG under {src} matched {list(selectors)}"
        )

    if clean:
        for variant in tiers:
            tier_root = asset_root / f"sprites_{variant.suffix}"
            if tier_root.exists():
                shutil.rmtree(tier_root)

    built = {variant.suffix: 0 for variant in tiers}
    skipped = {variant.suffix: 0 for variant in tiers}

    for unit in units:
        source: SheetSource | None = None
        stamp = _input_stamp((unit.ron,) + unit.pages)
        try:
            for variant in tiers:
                dst_root = asset_root / f"sprites_{variant.suffix}"
                ron_dst = dst_root / unit.rel
                if not force and _published_and_current(ron_dst, stamp):
                    skipped[variant.suffix] += 1
                    continue
                # the phases are DISJOINT — nesting the parse inside the build
                # would bill its seconds to both, and a phase table that sums to
                # more than the run is a table nobody can reason from.
                if unit.target is not None:
                    with clock.phase("source-render", unit=unit.stem):
                        publish_source_quality_target(unit.target, variant, dst_root)
                else:
                    if source is None:
                        with clock.phase("sheet parse + decode", unit=unit.stem):
                            source = SheetSource.load(unit.ron)
                            source.pages  # decode here so this phase owns the cost
                    with clock.phase("frame-local fallback", unit=unit.stem):
                        build_sheet_variant(source, ron_dst, variant)
                built[variant.suffix] += 1
        finally:
            if source is not None:
                source.close()

    loose_built = {variant.suffix: 0 for variant in tiers}
    for png in loose_srcs:
        rel = png.relative_to(src)
        stamp = _input_stamp((png,))
        for variant in tiers:
            dst = asset_root / f"sprites_{variant.suffix}" / rel
            dst_mtime = _mtime(dst)
            if not force and dst_mtime is not None and dst_mtime >= stamp:
                continue
            # Loose props/icons still load as standalone images. A future prop
            # atlas should be added at this seam and reuse the renderer packer,
            # but character-sheet variants above must remain pre-pack frame
            # generation, not post-pack atlas resizing.
            with clock.phase("loose png", unit=rel.as_posix()):
                resize_png(
                    png,
                    dst,
                    variant.nominal_scale,
                    min_px=variant.min_frame_px,
                    resampling=resampling_for_variant(variant),
                )
            loose_built[variant.suffix] += 1

    pruned = {variant.suffix: 0 for variant in tiers}
    if not selectors:
        with clock.phase("prune orphans"):
            for variant in tiers:
                pruned[variant.suffix] = prune_orphans(
                    src, asset_root / f"sprites_{variant.suffix}"
                )

    for variant in tiers:
        dst = asset_root / f"sprites_{variant.suffix}"
        orphans = f", {pruned[variant.suffix]} orphan(s) pruned" if pruned[variant.suffix] else ""
        print(
            f"sprites {variant.suffix}: {built[variant.suffix]} sheet(s) built, "
            f"{skipped[variant.suffix]} already current; "
            f"{loose_built[variant.suffix]}/{len(loose_srcs)} loose png "
            f"(scale~{variant.nominal_scale}){orphans} -> {dst}"
        )

    verify_published_sheets(asset_root, units, tiers)


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


def generate_parallax_variants(
    asset_root: Path,
    *,
    tiers: tuple[Variant, ...] = VARIANTS,
    selectors: tuple[str, ...] = (),
    force: bool = False,
    clean: bool = False,
    clock: PhaseClock | None = None,
) -> None:
    """Parallax layers are standalone images, so these ARE whole-image resizes.

    (The no-resizing-a-packed-page rule is about atlases; nothing here is
    packed.) Same per-image outer / tier inner shape as the sheets, for the
    same reason: one image's three tiers are its own business.
    """
    clock = clock or PhaseClock()
    src = asset_root / "backgrounds" / "parallax_layers"
    if not src.exists():
        print(f"skip missing parallax root: {src}")
        return

    if clean:
        for variant in tiers:
            tier_root = asset_root / "backgrounds" / f"parallax_layers_{variant.suffix}"
            if tier_root.exists():
                shutil.rmtree(tier_root)

    layers = [
        png
        for png in sorted(src.rglob("*.png"))
        if selects(png.relative_to(src).with_suffix("").as_posix(), selectors)
        or selects(png.stem, selectors)
    ]
    built = {variant.suffix: 0 for variant in tiers}
    missing: list[Path] = []
    for png in layers:
        rel = png.relative_to(src)
        stamp = _input_stamp((png,))
        for variant in tiers:
            dst = asset_root / "backgrounds" / f"parallax_layers_{variant.suffix}" / rel
            dst_mtime = _mtime(dst)
            if force or dst_mtime is None or dst_mtime < stamp:
                with clock.phase("parallax png", unit=rel.as_posix()):
                    resize_png(
                        png,
                        dst,
                        variant.nominal_scale,
                        min_px=variant.min_frame_px,
                        resampling=resampling_for_variant(variant),
                    )
                built[variant.suffix] += 1
            if not dst.is_file():
                missing.append(dst)

    for variant in tiers:
        dst_root = asset_root / "backgrounds" / f"parallax_layers_{variant.suffix}"
        print(
            f"parallax {variant.suffix}: {built[variant.suffix]} built, "
            f"{len(layers) - built[variant.suffix]} already current "
            f"(scale {variant.nominal_scale}) -> {dst_root}"
        )
    if missing:
        raise SystemExit(
            f"{len(missing)} planned parallax variant file(s) were not published:\n"
            + "\n".join(f"    {path}" for path in missing[:20])
        )


def main() -> None:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--asset-root",
        type=Path,
        default=Path("crates/ambition_platformer2d_actor_monolith/assets"),
        help="gameplay-core asset root containing sprites/ and backgrounds/",
    )
    parser.add_argument("--sprites-only", action="store_true")
    parser.add_argument("--backgrounds-only", action="store_true")
    parser.add_argument(
        "--tier",
        action="append",
        default=[],
        metavar="SUFFIX",
        help=(
            "only this quality tier (repeatable): "
            + ", ".join(variant.suffix for variant in VARIANTS)
        ),
    )
    parser.add_argument(
        "--target",
        action="append",
        default=[],
        metavar="PATTERN",
        help=(
            "only sheets / loose images whose stem matches PATTERN "
            "(fnmatch, repeatable), e.g. patent_clerk or 'pirate_*'"
        ),
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="rebuild even when the published variant is newer than its source",
    )
    parser.add_argument(
        "--clean",
        action="store_true",
        help="delete each selected tier root first (whole tiers only)",
    )
    args = parser.parse_args()

    if args.sprites_only and args.backgrounds_only:
        raise SystemExit("--sprites-only and --backgrounds-only are exclusive")
    if args.clean and args.target:
        raise SystemExit(
            "--clean empties a whole tier, so it cannot be combined with "
            "--target; use --target with --force to rebuild a selection."
        )

    tiers = resolve_tiers(tuple(args.tier))
    selectors = tuple(args.target)
    asset_root = args.asset_root.resolve()
    clock = PhaseClock()
    started = time.perf_counter()

    if not args.backgrounds_only:
        generate_sprite_variants(
            asset_root,
            tiers=tiers,
            selectors=selectors,
            force=args.force,
            clean=args.clean,
            clock=clock,
        )
    if not args.sprites_only:
        generate_parallax_variants(
            asset_root,
            tiers=tiers,
            selectors=selectors,
            force=args.force,
            clean=args.clean,
            clock=clock,
        )

    clock.report()
    print(f"\n==> quality variants finished in {time.perf_counter() - started:.1f}s")
    for variant in tiers:
        if not args.backgrounds_only:
            print(f"    file://{asset_root}/sprites_{variant.suffix}")
        if not args.sprites_only:
            print(
                "    file://"
                f"{asset_root}/backgrounds/parallax_layers_{variant.suffix}"
            )
    print(f"    file://{asset_root}")


if __name__ == "__main__":
    main()
