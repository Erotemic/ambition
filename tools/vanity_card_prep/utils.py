"""Shared utilities for the vanity-card pipeline."""

import os
import numpy as np
from PIL import Image

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(SCRIPT_DIR))
CONFIG_PATH = os.path.join(SCRIPT_DIR, "config.yaml")


def load_config() -> dict:
    import yaml

    with open(CONFIG_PATH) as f:
        return yaml.safe_load(f)


def repo_path(config: dict, section: str, *parts: str) -> str:
    return os.path.join(REPO, config["paths"][section], *parts)


def out_path(config: dict, *parts: str) -> str:
    return repo_path(config, "output", *parts)


def src_path(config: dict, *parts: str) -> str:
    return repo_path(config, "concept_art", *parts)


# ── Chroma key ────────────────────────────────────────────────────────────────


def chroma_key(
    img: Image.Image, inner: float = 25.0, outer: float = 70.0, spill: bool = True
) -> Image.Image:
    """
    Remove a green-screen background, adapting to the actual background colour
    sampled from image corners.  Returns an RGBA image.
    """
    rgba = img.convert("RGBA")
    arr = np.array(rgba, dtype=np.float32)
    h, w = arr.shape[:2]

    corners = [
        arr[0, 0, :3],
        arr[0, w - 1, :3],
        arr[h - 1, 0, :3],
        arr[h - 1, w - 1, :3],
    ]
    greenish = [p for p in corners if p[1] - p[0] > 20 and p[1] - p[2] > 20]
    bg = np.median(greenish if greenish else corners, axis=0)

    dist = np.sqrt(((arr[:, :, :3] - bg) ** 2).sum(axis=2))
    strength = np.clip(1.0 - (dist - inner) / (outer - inner), 0.0, 1.0)

    if spill:
        g = arr[:, :, 1]
        arr[:, :, 1] = np.clip(g - strength * 0.5 * g, 0.0, 255.0)

    result = arr.astype(np.uint8).copy()
    result[:, :, 3] = np.clip(arr[:, :, 3] * (1.0 - strength), 0, 255).astype(np.uint8)
    return Image.fromarray(result, "RGBA")


def cleanup_green_residue(img: Image.Image) -> Image.Image:
    """
    Secondary green-screen cleanup pass.

    The primary chroma_key samples a bright lime-green corner (~RGB 0,230,0).
    Shadow-side or unlit green pixels (~RGB 0,100-160,0) sit far enough from
    the sampled bg that they survive with full or partial alpha.  This pass
    zeroes any pixel whose colour profile is unambiguously green-screen:
        R < 50  AND  B < 50  AND  G > (R + 60)  AND  G > (B + 60)
    This threshold is safe because real character colours (skin, blue shirt,
    white/grey robot, red hair) all have balanced or high-blue channels.
    """
    arr = np.array(img.convert("RGBA"))
    R = arr[:, :, 0].astype(int)
    G = arr[:, :, 1].astype(int)
    B = arr[:, :, 2].astype(int)
    A = arr[:, :, 3]
    mask = (A > 0) & (R < 50) & (B < 50) & (G > R + 60) & (G > B + 60)
    arr[mask, 3] = 0
    return Image.fromarray(arr, "RGBA")


# ── Flat background removal ───────────────────────────────────────────────────


def remove_flat_bg(img: Image.Image, tolerance: int = 25) -> Image.Image:
    """
    Remove a uniform flat background by BFS flood-fill from the four borders.
    Returns RGBA with background pixels set to alpha=0.
    """
    from collections import deque

    rgba = img.convert("RGBA")
    arr = np.array(rgba)

    corners = [arr[0, 0, :3], arr[0, -1, :3], arr[-1, 0, :3], arr[-1, -1, :3]]
    bg = np.median(corners, axis=0).astype(np.uint8)

    diff = np.abs(arr[:, :, :3].astype(int) - bg.astype(int)).max(axis=2)
    candidate = diff < tolerance

    h, w = candidate.shape
    visited = np.zeros((h, w), dtype=bool)

    queue = deque()
    for x in range(w):
        for y in (0, h - 1):
            if candidate[y, x] and not visited[y, x]:
                visited[y, x] = True
                queue.append((y, x))
    for y in range(h):
        for x in (0, w - 1):
            if candidate[y, x] and not visited[y, x]:
                visited[y, x] = True
                queue.append((y, x))

    while queue:
        y, x = queue.popleft()
        for dy, dx in ((-1, 0), (1, 0), (0, -1), (0, 1)):
            ny, nx = y + dy, x + dx
            if (
                0 <= ny < h
                and 0 <= nx < w
                and not visited[ny, nx]
                and candidate[ny, nx]
            ):
                visited[ny, nx] = True
                queue.append((ny, nx))

    result = arr.copy()
    result[visited, 3] = 0
    return Image.fromarray(result, "RGBA")


# ── Content span detection ────────────────────────────────────────────────────


def find_content_spans(
    has_content: np.ndarray, min_gap: int = 15, min_size: int = 40
) -> list:
    """
    Return list of (start, end) pairs for runs of True values in has_content.
    Gaps shorter than min_gap are bridged; spans shorter than min_size dropped.
    """
    arr = has_content.astype(np.uint8).copy()

    # Bridge small gaps
    i = 0
    while i < len(arr):
        if arr[i] == 0:
            j = i
            while j < len(arr) and arr[j] == 0:
                j += 1
            if j - i < min_gap:
                arr[i:j] = 1
            i = j
        else:
            i += 1

    spans = []
    in_span = False
    start = 0
    for i, v in enumerate(arr):
        if v and not in_span:
            in_span, start = True, i
        elif not v and in_span:
            if i - start >= min_size:
                spans.append((start, i))
            in_span = False
    if in_span and len(arr) - start >= min_size:
        spans.append((start, len(arr)))
    return spans


# ── Misc image helpers ────────────────────────────────────────────────────────


def tight_crop(img: Image.Image, margin: int = 4) -> Image.Image:
    bbox = img.getbbox()
    if not bbox:
        return img
    l, t, r, b = bbox
    l, t = max(0, l - margin), max(0, t - margin)
    r, b = min(img.width, r + margin), min(img.height, b + margin)
    return img.crop((l, t, r, b))


def has_transparency(img: Image.Image, threshold: float = 0.05) -> bool:
    """Return True if the image has meaningful alpha transparency."""
    rgba = img.convert("RGBA")
    alpha = np.array(rgba)[:, :, 3]
    return bool((alpha < 250).mean() > threshold)


def save(img: Image.Image, path: str) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    img.save(path)
    rel = os.path.relpath(path, REPO)
    print(f"  wrote {rel}  ({img.size[0]}×{img.size[1]})")
