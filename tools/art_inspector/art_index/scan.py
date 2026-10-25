"""The products: each file under the scanned roots, with its size, digest
and, for a PNG, its pixel size (read from the header, not decoded)."""

from __future__ import annotations

import hashlib
import json
import os
import struct
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .paths import IMAGES, LISTED, Root


@dataclass
class Product:
    #: The path the index names it by: `<root>/<relative path>`.
    path: str
    root: Root
    rel: str
    file: Path
    bytes: int
    mtime: float
    sha256: str
    size: tuple[int, int] | None = None
    #: Filled by the later passes.
    asset: str | None = None
    role: str = "file"
    provenance: dict[str, Any] = field(default_factory=dict)
    thumb: str | None = None

    @property
    def is_image(self) -> bool:
        return self.file.suffix.lower() in IMAGES


class HashCache:
    """sha256 by (real path, mtime, size), kept between builds so a rebuild
    reads only the files that changed."""

    def __init__(self, path: Path) -> None:
        self.path = path
        try:
            self.data: dict[str, list] = json.loads(path.read_text())
        except (OSError, ValueError):
            self.data = {}
        self.used: dict[str, list] = {}

    def digest(self, file: Path) -> str:
        real = os.path.realpath(file)
        st = os.stat(real)
        key = [st.st_mtime_ns, st.st_size]
        hit = self.data.get(real)
        if hit and hit[:2] == key:
            self.used[real] = hit
            return hit[2]
        h = hashlib.sha256()
        with open(real, "rb") as f:
            for block in iter(lambda: f.read(1 << 20), b""):
                h.update(block)
        self.used[real] = [*key, h.hexdigest()]
        return h.hexdigest()

    def save(self) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        partial = self.path.with_suffix(".partial")
        partial.write_text(json.dumps(self.used))
        partial.replace(self.path)


def png_size(file: Path) -> tuple[int, int] | None:
    """The width and height in a PNG's IHDR chunk."""
    try:
        with open(file, "rb") as f:
            head = f.read(24)
    except OSError:
        return None
    if len(head) == 24 and head[:8] == b"\x89PNG\r\n\x1a\n" and head[12:16] == b"IHDR":
        return struct.unpack(">II", head[16:24])
    return None


def scan(roots: list[Root], hashes: HashCache) -> dict[str, Product]:
    """Every listed file under `roots`. A directory scanned as its own root
    (`backgrounds/hub` under `backgrounds`) is listed once, by the deeper root."""
    claimed = {r.path.resolve() for r in roots}
    out: dict[str, Product] = {}
    for root in roots:
        if not root.path.is_dir():
            continue
        for dirpath, dirnames, filenames in os.walk(root.path, followlinks=True):
            here = Path(dirpath)
            dirnames[:] = sorted(d for d in dirnames if not d.startswith(".") and (here / d).resolve() not in claimed)
            for name in sorted(filenames):
                file = here / name
                if name.startswith(".") or file.suffix.lower() not in LISTED or not file.is_file():
                    continue
                rel = file.relative_to(root.path).as_posix()
                st = file.stat()
                product = Product(
                    path=f"{root.name}/{rel}",
                    root=root,
                    rel=rel,
                    file=file,
                    bytes=st.st_size,
                    mtime=st.st_mtime,
                    sha256=hashes.digest(file),
                )
                if file.suffix.lower() == ".png":
                    product.size = png_size(file)
                out[product.path] = product
    return out
