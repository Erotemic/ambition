from __future__ import annotations

"""Small console helpers for copy/pasteable scripts.

The scripts should still work in plain terminals, but when Rich is available the
canonical contact sheet is printed as a clickable ``file://`` link.
"""

from pathlib import Path
from typing import Iterable, List


def _path_uri(path: Path) -> str:
    return path.resolve().as_uri()


def print_paths(paths: Iterable[str | Path]) -> None:
    outputs: List[Path] = [Path(path) for path in paths]
    try:
        from rich import print as rich_print
    except Exception:  # pragma: no cover - fallback when Rich is not installed
        for path in outputs:
            print(path)
        return

    for path in outputs:
        uri = _path_uri(path)
        rich_print(f"[link={uri}]{path}[/link]")


def print_canonical_outputs(paths: Iterable[str | Path]) -> None:
    outputs: List[Path] = [Path(path) for path in paths]
    try:
        from rich import print as rich_print
    except Exception:  # pragma: no cover - fallback when Rich is not installed
        for path in outputs:
            print(path)
        return

    print_paths(outputs)

    contact = next((p for p in outputs if p.name == "canonicals_contact_sheet.png"), None)
    if contact is not None:
        uri = _path_uri(contact)
        rich_print(f"[bold green]Canonical contact sheet:[/bold green] [link={uri}]{contact}[/link]")
