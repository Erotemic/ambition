"""Modal CLI for ambition_sprite2d_renderer.

Two surfaces live here:

(a) The procedural character lab — formerly ``proc2d_character_lab``.
    Targets defined under ``targets/`` (robot_side, goblin_side, boss_side,
    robot25d) are driven by YAML jobs in ``configs/`` and the
    ``adapters.TARGETS`` registry.

      list-targets               Show registered character adapters.
      draw-all                   Render every job in ``configs/`` to a sheet.
      draw-canonicals            Render canonical poses + contact sheet.
      draw-entities              Render non-character gameplay entity sprites.
      spritesheet <config> <out> Render one job's sheet.
      single <config> <out>      Render one frame.

(b) Tack-on targets (e.g. sandbag) that are not yet folded into the
    adapter system. They expose a ``render(out_dir, **opts) -> list[Path]``
    function and are driven by these subcommands:

      render <target>            Render into ``generated/<target>/``.
      preview <target>           Render and print the resulting paths.
      install <target>           Copy generated files into sandbox assets.
      render-publish <target>    Render then install.

See ``targets/sandbag.py`` for an integration TODO that spells out how to
fold sandbag into the adapter system once the next sandbag-shaped target
appears.
"""
from __future__ import annotations

import argparse
import shutil
import sys
from importlib import import_module
from pathlib import Path
from typing import List

from .adapters import TARGETS, get_adapter
from .canonical import write_canonicals
from .console import print_canonical_outputs, print_paths
from .config import CharacterJob, load_jobs
from .entities import write_entity_sprites
from .sheet import write_spritesheet


def package_dir() -> Path:
    return Path(__file__).resolve().parent.parent


def repo_root() -> Path:
    # tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/cli.py -> repo root.
    return Path(__file__).resolve().parents[3]


# Defaults are computed against the package, not the cwd, so the CLI works
# regardless of where the user runs it from.
DEFAULT_CONFIG_DIR = (
    Path(__file__).resolve().parent / "configs"
)
DEFAULT_ASSET_DIR = (
    package_dir() / "generated"
)


# ---- Tack-on targets registry --------------------------------------------------
#
# Maps target id -> dotted module path. Modules are imported lazily so that
# `list-targets` works even without Pillow installed.

_TACKON_TARGETS: dict[str, str] = {
    "sandbag": "ambition_sprite2d_renderer.targets.sandbag",
}


def _get_tackon_target(name: str):
    try:
        mod_path = _TACKON_TARGETS[name]
    except KeyError as ex:
        raise KeyError(f"unknown tack-on target: {name}") from ex
    return import_module(mod_path)


def sandbox_sprites_dir() -> Path:
    return (
        repo_root() / "crates" / "ambition_sandbox" / "assets" / "sprites"
    )


def generated_dir(target_name: str) -> Path:
    return DEFAULT_ASSET_DIR / target_name


# ---- Adapter (character lab) commands -----------------------------------------

def draw_all(config_dir: str | Path = DEFAULT_CONFIG_DIR, out_dir: str | Path = DEFAULT_ASSET_DIR) -> List[Path]:
    out_dir = Path(out_dir)
    outputs: List[Path] = []
    for path, job in load_jobs(config_dir):
        stem = job.target
        image_out = out_dir / f"{stem}_spritesheet.png"
        manifest_out = out_dir / f"{stem}_spritesheet.yaml"
        outputs.extend(write_spritesheet(job, image_out, manifest_out))
    return outputs


def draw_canonicals(
    config_dir: str | Path = DEFAULT_CONFIG_DIR,
    out_dir: str | Path = DEFAULT_ASSET_DIR / "canonicals",
) -> List[Path]:
    return write_canonicals(config_dir, out_dir)


def draw_entities(out_dir: str | Path = DEFAULT_ASSET_DIR / "entities") -> List[Path]:
    return write_entity_sprites(out_dir)


def _cmd_draw_all(args: argparse.Namespace) -> int:
    print_paths(draw_all(args.config_dir, args.out_dir))
    return 0


def _cmd_draw_canonicals(args: argparse.Namespace) -> int:
    print_canonical_outputs(draw_canonicals(args.config_dir, args.out_dir))
    return 0


def _cmd_draw_entities(args: argparse.Namespace) -> int:
    print_paths(draw_entities(args.out_dir))
    return 0


def _cmd_list_targets(args: argparse.Namespace) -> int:
    print("# adapter targets (driven by configs/*.yaml):")
    for target in sorted(TARGETS):
        adapter = get_adapter(target)
        print(f"  {target}: {', '.join(adapter.default_animations())}")
    print("# tack-on targets (render/install/render-publish):")
    for target in sorted(_TACKON_TARGETS):
        print(f"  {target}")
    return 0


def _cmd_spritesheet(args: argparse.Namespace) -> int:
    job = CharacterJob.load(args.config)
    print_paths(write_spritesheet(job, args.output, args.manifest_out))
    return 0


def _cmd_single(args: argparse.Namespace) -> int:
    job = CharacterJob.load(args.config)
    adapter = get_adapter(job.target)
    spec = adapter.sample_spec(job)
    img = adapter.render_single(spec, args.animation, args.frame_index, job)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    img.save(output)
    print_paths([output])
    return 0


# ---- Tack-on commands ---------------------------------------------------------

def _render_tackon(target_name: str, *, legacy_aliases: bool = False) -> List[Path]:
    target = _get_tackon_target(target_name)
    out_dir = generated_dir(target_name)
    paths = target.render(out_dir, legacy_aliases=legacy_aliases)
    for p in paths:
        print(p)
    return paths


def _install_tackon(target_name: str, dest_root: Path) -> List[Path]:
    target = _get_tackon_target(target_name)
    out_dir = generated_dir(target_name)
    dest_root.mkdir(parents=True, exist_ok=True)
    copied: List[Path] = []
    missing: List[str] = []
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


def _cmd_render(args: argparse.Namespace) -> int:
    _render_tackon(args.target, legacy_aliases=args.legacy_aliases)
    return 0


def _cmd_preview(args: argparse.Namespace) -> int:
    paths = _render_tackon(args.target, legacy_aliases=args.legacy_aliases)
    print(f"\npreview written: {paths[0] if paths else '<none>'}")
    return 0


def _cmd_install(args: argparse.Namespace) -> int:
    copied = _install_tackon(args.target, args.dest_root)
    return 0 if copied else 1


def _cmd_render_publish(args: argparse.Namespace) -> int:
    _render_tackon(args.target, legacy_aliases=args.legacy_aliases)
    copied = _install_tackon(args.target, args.dest_root)
    return 0 if copied else 1


def _add_tackon_render_args(p: argparse.ArgumentParser) -> None:
    p.add_argument("target", choices=list(_TACKON_TARGETS))
    p.add_argument(
        "--legacy-aliases",
        action="store_true",
        help="also emit any legacy compatibility sheets the target supports",
    )


def _add_tackon_install_args(p: argparse.ArgumentParser) -> None:
    p.add_argument("target", choices=list(_TACKON_TARGETS))
    p.add_argument(
        "--dest-root",
        type=Path,
        default=sandbox_sprites_dir(),
        help="install destination (default: crates/ambition_sandbox/assets/sprites)",
    )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="ambition_sprite2d_renderer",
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    sub = parser.add_subparsers(dest="command", required=True)

    # Adapter (character lab) commands.
    p = sub.add_parser("draw-all", help="Render all default sprite sheets")
    p.add_argument("--config-dir", default=str(DEFAULT_CONFIG_DIR))
    p.add_argument("--out-dir", default=str(DEFAULT_ASSET_DIR))
    p.set_defaults(func=_cmd_draw_all)

    p = sub.add_parser("draw-canonicals", help="Render canonical images + contact sheet")
    p.add_argument("--config-dir", default=str(DEFAULT_CONFIG_DIR))
    p.add_argument("--out-dir", default=str(DEFAULT_ASSET_DIR / "canonicals"))
    p.set_defaults(func=_cmd_draw_canonicals)

    p = sub.add_parser("draw-entities", help="Render non-character gameplay entity sprites")
    p.add_argument("--out-dir", default=str(DEFAULT_ASSET_DIR / "entities"))
    p.set_defaults(func=_cmd_draw_entities)

    p = sub.add_parser("list-targets", help="Show registered targets (adapter + tack-on)")
    p.set_defaults(func=_cmd_list_targets)
    sub.add_parser("list", help="alias of list-targets").set_defaults(func=_cmd_list_targets)

    p = sub.add_parser("spritesheet", help="Render one job's sheet")
    p.add_argument("config")
    p.add_argument("output")
    p.add_argument("--manifest-out", default=None)
    p.set_defaults(func=_cmd_spritesheet)

    p = sub.add_parser("single", help="Render one frame from a job")
    p.add_argument("config")
    p.add_argument("output")
    p.add_argument("--animation", default="idle")
    p.add_argument("--frame-index", type=int, default=0)
    p.set_defaults(func=_cmd_single)

    # Tack-on commands.
    p = sub.add_parser("render", help="Render a tack-on target into generated/")
    _add_tackon_render_args(p)
    p.set_defaults(func=_cmd_render)

    p = sub.add_parser("preview", help="Render a tack-on target and report paths")
    _add_tackon_render_args(p)
    p.set_defaults(func=_cmd_preview)

    p = sub.add_parser("install", help="Copy a tack-on target's files into sandbox assets")
    _add_tackon_install_args(p)
    p.set_defaults(func=_cmd_install)

    p = sub.add_parser("render-publish", help="Render then install a tack-on target")
    _add_tackon_render_args(p)
    p.add_argument(
        "--dest-root",
        type=Path,
        default=sandbox_sprites_dir(),
    )
    p.set_defaults(func=_cmd_render_publish)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args) or 0)


if __name__ == "__main__":
    raise SystemExit(main())
