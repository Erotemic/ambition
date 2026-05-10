#!/usr/bin/env python3
from __future__ import annotations

import argparse
import shutil
from pathlib import Path

from pirate_sprite_common import ANIMATIONS, BASE_FRAME, build_sheet, draw_character

TARGETS = {
    "pirate_admiral": "pirate_admiral",
    "pirate_raider": "pirate_raider",
}


def find_repo_root(start: Path) -> Path:
    start = start.resolve()
    for path in [start, *start.parents]:
        if (path / "crates" / "ambition_sandbox").exists() and (path / "tools" / "ambition_sprite2d_renderer").exists():
            return path
    return start.parents[1]


def render_target(target: str, out_dir: Path, frame_size=BASE_FRAME):
    out_dir.mkdir(parents=True, exist_ok=True)
    return build_sheet(
        target=target,
        rows=ANIMATIONS,
        render_fn=lambda anim, frame_idx, nframes: draw_character(target, anim, frame_idx, nframes, frame_size=frame_size),
        out_dir=out_dir,
        frame_size=frame_size,
    )


def publish_target(target: str, src_dir: Path, dest_root: Path):
    dest_root.mkdir(parents=True, exist_ok=True)
    copied = []
    for suffix in ["spritesheet.png", "spritesheet.yaml"]:
        src = src_dir / f"{target}_{suffix}"
        dst = dest_root / src.name
        shutil.copy2(src, dst)
        copied.append(dst)
    return copied


def main(argv=None):
    here = Path(__file__).resolve().parent
    repo_root = find_repo_root(here)
    p = argparse.ArgumentParser(description="Render standalone pirate spritesheets for Ambition.")
    p.add_argument("--target", choices=["pirate_admiral", "pirate_raider", "all"], default="all")
    p.add_argument("--out-root", type=Path, default=here / "generated")
    p.add_argument("--frame-width", type=int, default=128)
    p.add_argument("--frame-height", type=int, default=128)
    p.add_argument("--publish", action="store_true")
    p.add_argument("--dest-root", type=Path, default=repo_root / "crates" / "ambition_sandbox" / "assets" / "sprites")
    args = p.parse_args(argv)

    targets = [args.target] if args.target != "all" else ["pirate_admiral", "pirate_raider"]
    frame_size = (args.frame_width, args.frame_height)
    for target in targets:
        out_dir = args.out_root / target
        outputs = render_target(target, out_dir, frame_size=frame_size)
        print(f"Rendered {target}:")
        for pth in outputs.values():
            print(f"  {pth}")
        if args.publish:
            copied = publish_target(target, out_dir, args.dest_root)
            print(f"Published {target}:")
            for pth in copied:
                print(f"  {pth}")


if __name__ == "__main__":
    raise SystemExit(main())
