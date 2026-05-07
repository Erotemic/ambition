"""Modal CLI for ambition_sprite2d_renderer.

Subcommands:

    list                       Show registered render targets.
    render <target>            Render target into generated/<target>/.
    preview <target>           Render and print resulting paths.
    install <target>           Copy generated files into the sandbox asset tree.
    render-publish <target>    Render then install in one shot.
"""
from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path

from ambition_sprite2d_renderer.targets import get_target, list_target_names


def package_dir() -> Path:
    return Path(__file__).resolve().parent.parent


def repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


def generated_dir(target_name: str) -> Path:
    return package_dir() / "generated" / target_name


def sandbox_sprites_dir() -> Path:
    return (
        repo_root() / "crates" / "ambition_sandbox" / "assets" / "sprites"
    )


def cmd_list(_args: argparse.Namespace) -> int:
    for name in list_target_names():
        print(name)
    return 0


def _render_target(target_name: str, *, legacy_aliases: bool = False) -> list[Path]:
    target = get_target(target_name)
    out_dir = generated_dir(target_name)
    paths = target.render(out_dir, legacy_aliases=legacy_aliases)
    for p in paths:
        print(p)
    return paths


def cmd_render(args: argparse.Namespace) -> int:
    _render_target(args.target, legacy_aliases=args.legacy_aliases)
    return 0


def cmd_preview(args: argparse.Namespace) -> int:
    paths = _render_target(args.target, legacy_aliases=args.legacy_aliases)
    print(f"\npreview written: {paths[0] if paths else '<none>'}")
    return 0


def _install(target_name: str, dest_root: Path) -> list[Path]:
    """Copy the canonical sheet files for ``target_name`` into ``dest_root``."""
    target = get_target(target_name)
    out_dir = generated_dir(target_name)
    dest_root.mkdir(parents=True, exist_ok=True)
    copied: list[Path] = []
    missing: list[str] = []
    for fname in target.SHEET_FILES:
        src = out_dir / fname
        if not src.exists():
            missing.append(fname)
            continue
        dst = dest_root / fname
        shutil.copy2(src, dst)
        copied.append(dst)
    if missing:
        print(
            f"warning: {target_name} files not yet rendered: {', '.join(missing)}",
            file=sys.stderr,
        )
    for p in copied:
        print(p)
    return copied


def cmd_install(args: argparse.Namespace) -> int:
    copied = _install(args.target, args.dest_root)
    return 0 if copied else 1


def cmd_render_publish(args: argparse.Namespace) -> int:
    _render_target(args.target, legacy_aliases=args.legacy_aliases)
    copied = _install(args.target, args.dest_root)
    return 0 if copied else 1


def add_render_args(p: argparse.ArgumentParser) -> None:
    p.add_argument("target", help="target id (see `list`)")
    p.add_argument(
        "--legacy-aliases",
        action="store_true",
        help="also emit any legacy compatibility sheets the target supports",
    )


def add_install_args(p: argparse.ArgumentParser) -> None:
    p.add_argument("target", help="target id (see `list`)")
    p.add_argument(
        "--dest-root",
        type=Path,
        default=sandbox_sprites_dir(),
        help="install destination (default: crates/ambition_sandbox/assets/sprites)",
    )


def build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(
        prog="ambition_sprite2d_renderer",
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    sub = ap.add_subparsers(dest="command", required=True)

    p_list = sub.add_parser("list", help="Show registered targets")
    p_list.set_defaults(func=cmd_list)

    p_render = sub.add_parser("render", help="Render a target into generated/")
    add_render_args(p_render)
    p_render.set_defaults(func=cmd_render)

    p_preview = sub.add_parser("preview", help="Render and report output paths")
    add_render_args(p_preview)
    p_preview.set_defaults(func=cmd_preview)

    p_install = sub.add_parser("install", help="Copy generated files into sandbox assets")
    add_install_args(p_install)
    p_install.set_defaults(func=cmd_install)

    p_rp = sub.add_parser("render-publish", help="Render, then install")
    add_render_args(p_rp)
    p_rp.add_argument(
        "--dest-root",
        type=Path,
        default=sandbox_sprites_dir(),
    )
    p_rp.set_defaults(func=cmd_render_publish)

    return ap


def main(argv: list[str] | None = None) -> int:
    ap = build_parser()
    args = ap.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
