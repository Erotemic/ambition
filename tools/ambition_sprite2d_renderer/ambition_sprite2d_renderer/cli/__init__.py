"""Command-line interface for the sprite renderer.

Split for navigability; the public API is re-exported here so callers
keep using ``from ...cli import main, draw_all`` without caring about it:

- :mod:`cli.commands` — the ``draw_*`` pipeline functions + ``_cmd_*``
  argparse handlers (all the logic).
- :mod:`cli.parser` — argparse wiring + the ``main()`` entry point.
- :mod:`cli.console` — Rich path-link output helpers.
"""
from __future__ import annotations

from .commands import (
    draw_all,
    draw_canonicals,
    draw_character,
    draw_factions,
    draw_review,
)
from .parser import build_parser, main

__all__ = [
    "build_parser",
    "draw_all",
    "draw_canonicals",
    "draw_character",
    "draw_factions",
    "draw_review",
    "main",
]
