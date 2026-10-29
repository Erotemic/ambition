from __future__ import annotations

import argparse
from pathlib import Path

from .profiles import iter_profiles
from .render import render_profile


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Render Ambition parallax background placeholder layers."
    )
    parser.add_argument(
        "--out",
        type=Path,
        required=True,
        help="Output root. Profiles are written below this directory. The caller selects its content library.",
    )
    parser.add_argument(
        "--profile",
        default="all",
        help="Profile to render, or 'all'. Known profiles: default, hub, lab, basement, cove, skybridge, boss, water, cave.",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    written = []
    for profile in iter_profiles(args.profile):
        written.extend(render_profile(profile, args.out))
    for path in written:
        print(path)
    return 0
