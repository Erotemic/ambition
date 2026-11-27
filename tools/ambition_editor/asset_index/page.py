"""The page's own files: its HTML, scripts and styles, as opposed to the catalog
a build writes. They change only with the inspector's code, so they are copied
in a moment, and `serve` copies them before it shows a previous index: an old
page against a new server asks for routes the server no longer has."""

from __future__ import annotations

import shutil
from pathlib import Path

WEB = Path(__file__).resolve().parents[1] / "web"
FILES = ("index.html", "catalog-loader.js", "app.js", "app.css", "inventory.js", "characters.js", "table_sort.js", "reviews.js", "items.js", "edits.js",
         "preflight.html")


def copy(out: Path) -> None:
    """Write the page's files from `web/` into the index at `out`."""
    for name in FILES:
        shutil.copyfile(WEB / name, out / name)
