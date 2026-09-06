from __future__ import annotations

from pathlib import Path
from typing import Dict, Tuple

import numpy as np
from PIL import Image, ImageDraw, ImageFilter


def _hex_to_rgb01(hex_color: str) -> Tuple[float, float, float]:
    hex_color = hex_color.strip().lstrip("#")
    if len(hex_color) != 6:
        raise ValueError(hex_color)
    return tuple(int(hex_color[i : i + 2], 16) / 255.0 for i in (0, 2, 4))


def _rgb01_to_u8(rgb: Tuple[float, float, float]) -> Tuple[int, int, int]:
    return tuple(max(0, min(255, int(round(c * 255)))) for c in rgb)


def _mix(
    a: Tuple[float, float, float], b: Tuple[float, float, float], t: float
) -> Tuple[float, float, float]:
    return tuple((1.0 - t) * x + t * y for x, y in zip(a, b))


def _make_base_canvas(
    size: int, c0: Tuple[int, int, int], c1: Tuple[int, int, int]
) -> Image.Image:
    yy, xx = np.mgrid[0:size, 0:size].astype(np.float32)
    xx = xx / max(1, size - 1)
    yy = yy / max(1, size - 1)
    grad = (0.72 * xx + 0.28 * yy)[..., None]
    a = np.array(c0, dtype=np.float32)
    b = np.array(c1, dtype=np.float32)
    arr = a[None, None, :] * (1.0 - grad) + b[None, None, :] * grad
    return Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8), mode="RGB")


def _draw_robot_texture(
    size: int,
    base_hex: str,
    shadow_hex: str,
    accent_hex: str,
    out: Path,
    dark: bool = False,
) -> Path:
    base = _hex_to_rgb01(base_hex)
    shadow = _hex_to_rgb01(shadow_hex)
    accent = _hex_to_rgb01(accent_hex)
    c0 = _rgb01_to_u8(_mix(base, shadow, 0.12 if not dark else 0.28))
    c1 = _rgb01_to_u8(_mix(base, shadow, 0.34 if not dark else 0.56))
    img = _make_base_canvas(size, c0, c1)
    draw = ImageDraw.Draw(img, "RGBA")

    # Large, visible panel blocks.
    panel_fill = _rgb01_to_u8(_mix(base, shadow, 0.28 if not dark else 0.12)) + (155,)
    edge = _rgb01_to_u8(
        _mix(shadow, (0.03, 0.03, 0.04), 0.52 if not dark else 0.34)
    ) + (185,)
    blocks = [
        (0.06, 0.08, 0.44, 0.44),
        (0.52, 0.12, 0.90, 0.38),
        (0.14, 0.56, 0.54, 0.88),
        (0.60, 0.54, 0.90, 0.84),
    ]
    for x0, y0, x1, y1 in blocks:
        rect = (size * x0, size * y0, size * x1, size * y1)
        draw.rounded_rectangle(
            rect,
            radius=size * 0.05,
            fill=panel_fill,
            outline=edge,
            width=max(1, size // 64),
        )

    # Accent stripe and small bright insert.
    stripe = _rgb01_to_u8(_mix(accent, base, 0.12)) + (200,)
    draw.rounded_rectangle(
        (size * 0.10, size * 0.70, size * 0.90, size * 0.80),
        radius=size * 0.04,
        fill=stripe,
    )
    draw.rounded_rectangle(
        (size * 0.68, size * 0.18, size * 0.84, size * 0.30),
        radius=size * 0.02,
        fill=(255, 255, 255, 170),
    )
    draw.line(
        (size * 0.10, size * 0.22, size * 0.90, size * 0.22),
        fill=(255, 255, 255, 90),
        width=max(1, size // 64),
    )
    draw.line(
        (size * 0.16, size * 0.54, size * 0.86, size * 0.54),
        fill=(255, 255, 255, 70),
        width=max(1, size // 72),
    )

    # Bolts.
    bolt_color = _rgb01_to_u8(_mix(shadow, (0.0, 0.0, 0.0), 0.35)) + (88,)
    r = max(2, size // 26)
    for ox, oy in [(0.14, 0.14), (0.86, 0.16), (0.16, 0.86), (0.84, 0.84)]:
        x = int(size * ox)
        y = int(size * oy)
        draw.ellipse((x - r, y - r, x + r, y + r), fill=bolt_color)
        draw.line(
            (x - r + 1, y, x + r - 1, y),
            fill=(255, 255, 255, 120),
            width=max(1, size // 128),
        )
        draw.line(
            (x, y - r + 1, x, y + r - 1),
            fill=(255, 255, 255, 120),
            width=max(1, size // 128),
        )

    # Subtle noise and scratches.
    rng = np.random.default_rng(0 if dark else 1)
    arr = np.asarray(img).astype(np.float32)
    arr += rng.normal(0.0, 2.5 if dark else 3.0, size=(size, size, 1))
    arr = np.clip(arr, 0, 255)
    img = Image.fromarray(arr.astype(np.uint8), mode="RGB").filter(
        ImageFilter.GaussianBlur(radius=max(0.25, size / 360))
    )
    draw = ImageDraw.Draw(img, "RGBA")
    for _ in range(max(5, size // 28)):
        x0 = int(rng.integers(0, size - 1))
        y0 = int(rng.integers(0, size - 1))
        x1 = min(size - 1, x0 + int(rng.integers(size // 24, size // 8)))
        y1 = min(size - 1, y0 + int(rng.integers(1, size // 26)))
        draw.line((x0, y0, x1, y1), fill=(255, 255, 255, 24), width=max(1, size // 100))

    out.parent.mkdir(parents=True, exist_ok=True)
    img.save(out)
    return out


def _draw_goblin_skin_texture(
    size: int, base_hex: str, shadow_hex: str, eye_hex: str, out: Path
) -> Path:
    base = _hex_to_rgb01(base_hex)
    shadow = _hex_to_rgb01(shadow_hex)
    eye = _hex_to_rgb01(eye_hex)
    img = _make_base_canvas(
        size,
        _rgb01_to_u8(_mix(base, shadow, 0.10)),
        _rgb01_to_u8(_mix(base, shadow, 0.30)),
    )
    draw = ImageDraw.Draw(img, "RGBA")
    rng = np.random.default_rng(3)

    for _ in range(max(28, size // 5)):
        x = int(rng.integers(0, size))
        y = int(rng.integers(0, size))
        r = int(rng.integers(max(2, size // 36), max(4, size // 18)))
        col = _rgb01_to_u8(_mix(shadow, base, 0.18)) + (rng.integers(60, 110),)
        draw.ellipse((x - r, y - r, x + r, y + r), fill=col)

    tint = _rgb01_to_u8(_mix(eye, shadow, 0.46)) + (88,)
    draw.polygon(
        [
            (size * 0.06, size * 0.36),
            (size * 0.44, size * 0.28),
            (size * 0.72, size * 0.38),
            (size * 0.20, size * 0.46),
        ],
        fill=tint,
    )
    draw.polygon(
        [
            (size * 0.12, size * 0.58),
            (size * 0.54, size * 0.50),
            (size * 0.82, size * 0.60),
            (size * 0.26, size * 0.68),
        ],
        fill=tint,
    )

    arr = np.asarray(img).astype(np.float32)
    arr += rng.normal(0.0, 2.0, size=(size, size, 1))
    arr = np.clip(arr, 0, 255)
    img = Image.fromarray(arr.astype(np.uint8), mode="RGB").filter(
        ImageFilter.GaussianBlur(radius=max(0.4, size / 300))
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    img.save(out)
    return out


def _draw_goblin_cloth_texture(
    size: int, base_hex: str, shadow_hex: str, accent_hex: str, out: Path
) -> Path:
    base = _hex_to_rgb01(base_hex)
    shadow = _hex_to_rgb01(shadow_hex)
    accent = _hex_to_rgb01(accent_hex)
    img = _make_base_canvas(
        size,
        _rgb01_to_u8(_mix(base, shadow, 0.06)),
        _rgb01_to_u8(_mix(base, shadow, 0.30)),
    )
    draw = ImageDraw.Draw(img, "RGBA")
    rng = np.random.default_rng(7)

    band = _rgb01_to_u8(_mix(accent, base, 0.30)) + (110,)
    spacing = max(14, size // 6)
    for offset in range(-size, size * 2, spacing):
        draw.line((offset, 0, offset + size, size), fill=band, width=max(1, size // 34))
    for y in range(max(12, size // 7), size, max(12, size // 7)):
        draw.line((0, y, size, y), fill=(255, 255, 255, 56), width=max(1, size // 90))
    patch = _rgb01_to_u8(_mix(base, shadow, 0.22)) + (120,)
    draw.rectangle((size * 0.10, size * 0.18, size * 0.32, size * 0.36), fill=patch)
    draw.rectangle((size * 0.58, size * 0.62, size * 0.84, size * 0.82), fill=patch)

    arr = np.asarray(img).astype(np.float32)
    arr += rng.normal(0.0, 2.5, size=(size, size, 1))
    arr = np.clip(arr, 0, 255)
    img = Image.fromarray(arr.astype(np.uint8), mode="RGB").filter(
        ImageFilter.GaussianBlur(radius=max(0.20, size / 420))
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    img.save(out)
    return out


def generate_texture_pack(
    out_dir: Path, spec: Dict[str, object], target: str, size: int = 128
) -> Dict[str, str]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    result: Dict[str, str] = {}
    if target == "robot":
        result["primary"] = str(
            _draw_robot_texture(
                size,
                str(spec["primary_color"]),
                str(spec["primary_shadow"]),
                str(spec["accent_color"]),
                out_dir / "robot_primary.png",
                dark=False,
            )
        )
        result["dark"] = str(
            _draw_robot_texture(
                size,
                str(spec["dark_color"]),
                "#050509",
                str(spec["accent2_color"]),
                out_dir / "robot_dark.png",
                dark=True,
            )
        )
        result["metal"] = str(
            _draw_robot_texture(
                size,
                str(spec["metal_color"]),
                "#77808D",
                str(spec["accent_color"]),
                out_dir / "robot_metal.png",
                dark=False,
            )
        )
    elif target == "goblin":
        result["skin"] = str(
            _draw_goblin_skin_texture(
                size,
                str(spec["skin_color"]),
                str(spec["skin_shadow"]),
                str(spec["eye_color"]),
                out_dir / "goblin_skin.png",
            )
        )
        result["cloth"] = str(
            _draw_goblin_cloth_texture(
                size,
                str(spec["cloth_color"]),
                str(spec["cloth_shadow"]),
                str(spec["accent_color"]),
                out_dir / "goblin_cloth.png",
            )
        )
        result["accent"] = str(
            _draw_robot_texture(
                size,
                str(spec["accent_color"]),
                str(spec["accent2_color"]),
                str(spec["eye_color"]),
                out_dir / "goblin_accent.png",
                dark=False,
            )
        )
        result["metal"] = str(
            _draw_robot_texture(
                size,
                str(spec["metal_color"]),
                "#7C74A2",
                str(spec["accent2_color"]),
                out_dir / "goblin_metal.png",
                dark=False,
            )
        )
    else:
        raise KeyError(target)
    return result
