from __future__ import annotations

from pathlib import Path

import numpy as np
import torch
from PIL import Image, ImageChops, ImageDraw, ImageOps


def load_target_tensor(
    path: str | Path, size: tuple[int, int], *, background=(0.04, 0.045, 0.055)
) -> torch.Tensor:
    """Load an RGB/RGBA image as HWC float tensor in [0, 1]."""
    img = Image.open(path).convert("RGBA")
    img = ImageOps.exif_transpose(img)
    img = ImageOps.contain(img, size, method=Image.Resampling.LANCZOS)
    canvas = Image.new("RGBA", size, tuple(int(v * 255) for v in (*background, 1.0)))
    canvas.alpha_composite(
        img, ((size[0] - img.width) // 2, (size[1] - img.height) // 2)
    )
    arr = np.asarray(canvas.convert("RGB"), dtype=np.float32) / 255.0
    return torch.from_numpy(arr)


def tensor_to_image(tensor: torch.Tensor) -> Image.Image:
    arr = tensor.detach().cpu().clamp(0, 1).numpy()
    if arr.ndim == 3 and arr.shape[-1] == 3:
        pass
    elif arr.ndim == 3 and arr.shape[0] == 3:
        arr = np.moveaxis(arr, 0, -1)
    else:
        raise ValueError(f"Expected RGB tensor, got shape={arr.shape}")
    return Image.fromarray(np.round(arr * 255).astype(np.uint8), mode="RGB")


def save_tensor_image(tensor: torch.Tensor, path: str | Path) -> None:
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    tensor_to_image(tensor).save(path)


def write_comparison(
    target: torch.Tensor,
    render: torch.Tensor,
    path: str | Path,
    *,
    title: str = "target / render / abs diff",
) -> None:
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    tgt = tensor_to_image(target)
    ren = tensor_to_image(render)
    diff = ImageChops.difference(tgt, ren)
    # Boost subtle differences enough to read in a thumbnail.
    diff = diff.point(lambda x: min(255, int(x * 3)))
    pad = 12
    header_h = 28
    w = tgt.width + ren.width + diff.width + pad * 4
    h = max(tgt.height, ren.height, diff.height) + pad * 2 + header_h
    out = Image.new("RGB", (w, h), (20, 22, 26))
    draw = ImageDraw.Draw(out)
    draw.text((pad, 6), title, fill=(230, 230, 230))
    x = pad
    y = header_h + pad
    for label, img in [("target", tgt), ("render", ren), ("abs diff x3", diff)]:
        draw.text((x, header_h - 14), label, fill=(210, 210, 210))
        out.paste(img, (x, y))
        x += img.width + pad
    out.save(path)


def write_loss_curve(losses: list[float], path: str | Path) -> None:
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    width = 720
    height = 220
    pad = 28
    out = Image.new("RGB", (width, height), (248, 248, 248))
    draw = ImageDraw.Draw(out)
    draw.rectangle([pad, pad, width - pad, height - pad], outline=(80, 80, 80))
    if len(losses) >= 2:
        vals = np.asarray(losses, dtype=np.float64)
        lo = float(vals.min())
        hi = float(vals.max())
        if hi <= lo:
            hi = lo + 1.0
        xs = np.linspace(pad, width - pad, len(vals))
        ys = (height - pad) - (vals - lo) / (hi - lo) * (height - pad * 2)
        pts = [(float(x), float(y)) for x, y in zip(xs, ys)]
        draw.line(pts, fill=(30, 30, 30), width=2)
        draw.text(
            (pad + 4, 8), f"loss {vals[0]:.6f} -> {vals[-1]:.6f}", fill=(30, 30, 30)
        )
    out.save(path)


def write_debug_gif(
    image_paths: list[str | Path], path: str | Path, *, duration_ms: int = 220
) -> None:
    image_paths = [Path(p) for p in image_paths]
    if not image_paths:
        return
    frames = [Image.open(p).convert("RGB") for p in image_paths]
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    frames[0].save(
        path,
        save_all=True,
        append_images=frames[1:],
        duration=duration_ms,
        loop=0,
        optimize=False,
    )
