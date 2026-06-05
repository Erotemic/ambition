from __future__ import annotations

"""Rich console helpers for copy/pasteable scripts.

Every path emitted by the sprite tooling should be printed through these
helpers so terminals that support Rich links expose it as an easy-to-open
``file://`` link.  The displayed text intentionally remains the original path
shape supplied by the caller, while the link target is absolute.
"""

from pathlib import Path
from typing import Iterable, List

from rich import print as rich_print
from rich.markup import escape


def _path_uri(path: Path) -> str:
    return path.resolve().as_uri()


def path_link(path: str | Path) -> str:
    """Return Rich markup for a clickable local file-system path."""
    p = Path(path)
    return f"[link={_path_uri(p)}]{escape(str(path))}[/link]"


def print_path(path: str | Path, *, prefix: str = "", suffix: str = "") -> None:
    """Print one path as a Rich file link."""
    rich_print(f"{prefix}{path_link(path)}{suffix}")


def print_paths(paths: Iterable[str | Path], *, prefix: str = "") -> None:
    """Print paths, one per line, as Rich file links."""
    outputs: List[str | Path] = list(paths)
    for path in outputs:
        print_path(path, prefix=prefix)


def print_canonical_outputs(paths: Iterable[str | Path]) -> None:
    outputs: List[Path] = [Path(path) for path in paths]
    print_paths(outputs)

    contact = next(
        (p for p in outputs if p.name == "canonicals_contact_sheet.png"), None
    )
    if contact is not None:
        print_path(contact, prefix="[bold green]Canonical contact sheet:[/bold green] ")
