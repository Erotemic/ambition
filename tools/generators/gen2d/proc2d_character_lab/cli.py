from __future__ import annotations

import argparse
from pathlib import Path
from typing import Iterable, List

from .adapters import TARGETS, get_adapter
from .canonical import write_canonicals
from .console import print_canonical_outputs
from .config import CharacterJob, load_jobs
from .sheet import write_spritesheet

DEFAULT_CONFIG_DIR = Path("proc2d_character_lab/configs")
DEFAULT_ASSET_DIR = Path("assets")


def draw_all(config_dir: str | Path = DEFAULT_CONFIG_DIR, out_dir: str | Path = DEFAULT_ASSET_DIR) -> List[Path]:
    out_dir = Path(out_dir)
    outputs: List[Path] = []
    for path, job in load_jobs(config_dir):
        stem = job.target
        image_out = out_dir / f"{stem}_spritesheet.png"
        manifest_out = out_dir / f"{stem}_spritesheet.yaml"
        outputs.extend(write_spritesheet(job, image_out, manifest_out))
    return outputs


def draw_canonicals(config_dir: str | Path = DEFAULT_CONFIG_DIR, out_dir: str | Path = DEFAULT_ASSET_DIR / "canonicals") -> List[Path]:
    return write_canonicals(config_dir, out_dir)


def _cmd_draw_all(args: argparse.Namespace) -> int:
    for out in draw_all(args.config_dir, args.out_dir):
        print(out)
    return 0


def _cmd_draw_canonicals(args: argparse.Namespace) -> int:
    print_canonical_outputs(draw_canonicals(args.config_dir, args.out_dir))
    return 0


def _cmd_list_targets(args: argparse.Namespace) -> int:
    for target in sorted(TARGETS):
        adapter = get_adapter(target)
        print(f"{target}: {', '.join(adapter.default_animations())}")
    return 0


def _cmd_spritesheet(args: argparse.Namespace) -> int:
    job = CharacterJob.load(args.config)
    write_spritesheet(job, args.output, args.manifest_out)
    print(args.output)
    return 0


def _cmd_single(args: argparse.Namespace) -> int:
    job = CharacterJob.load(args.config)
    adapter = get_adapter(job.target)
    spec = adapter.sample_spec(job)
    img = adapter.render_single(spec, args.animation, args.frame_index, job)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    img.save(output)
    print(output)
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="proc2d-character-lab")
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("draw-all", help="Render all default sprite sheets.")
    p.add_argument("--config-dir", default=str(DEFAULT_CONFIG_DIR))
    p.add_argument("--out-dir", default=str(DEFAULT_ASSET_DIR))
    p.set_defaults(func=_cmd_draw_all)

    p = sub.add_parser("draw-canonicals", help="Render default canonical images and contact sheet.")
    p.add_argument("--config-dir", default=str(DEFAULT_CONFIG_DIR))
    p.add_argument("--out-dir", default=str(DEFAULT_ASSET_DIR / "canonicals"))
    p.set_defaults(func=_cmd_draw_canonicals)

    p = sub.add_parser("list-targets")
    p.set_defaults(func=_cmd_list_targets)

    p = sub.add_parser("spritesheet")
    p.add_argument("config")
    p.add_argument("output")
    p.add_argument("--manifest-out", default=None)
    p.set_defaults(func=_cmd_spritesheet)

    p = sub.add_parser("single")
    p.add_argument("config")
    p.add_argument("output")
    p.add_argument("--animation", default="idle")
    p.add_argument("--frame-index", type=int, default=0)
    p.set_defaults(func=_cmd_single)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args) or 0)


if __name__ == "__main__":
    raise SystemExit(main())
