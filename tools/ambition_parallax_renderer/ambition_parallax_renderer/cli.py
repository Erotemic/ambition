"""CLI for Ambition background/parallax generation."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Iterable

from .parallax_layers import write_background_layers


def package_dir() -> Path:
    return Path(__file__).resolve().parent.parent


def repo_root() -> Path:
    # tools/ambition_parallax_renderer/ambition_parallax_renderer/cli.py -> repo root.
    return Path(__file__).resolve().parents[3]


DEFAULT_OUT_DIR = (
    repo_root()
    / "crates"
    / "ambition_gameplay_core"
    / "assets"
    / "backgrounds"
    / "parallax_layers"
)


def _print_paths(paths: Iterable[Path]) -> None:
    for path in paths:
        print(path)


def _cmd_draw_backgrounds(args: argparse.Namespace) -> int:
    _print_paths(write_background_layers(args.out_dir))
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="ambition_parallax_renderer",
        description="Generate Ambition biome sky/backdrop and parallax atmosphere PNGs.",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser(
        "draw-backgrounds", help="Render opaque skies plus transparent parallax layers"
    )
    p.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    p.set_defaults(func=_cmd_draw_backgrounds)

    # Compatibility alias for the previous command name. This renderer still
    # produces backgrounds; the alias avoids breaking terminal history.
    p = sub.add_parser("draw-parallax-layers", help="Alias for draw-backgrounds")
    p.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    p.set_defaults(func=_cmd_draw_backgrounds)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))
