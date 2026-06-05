from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image, ImageOps

from .fit import fit_template, render_template
from .init_template import init_template_from_image


def _print_path(label: str, path: Path) -> None:
    try:
        from rich import print as rprint

        rprint(f"{label}: [link={path.resolve().as_uri()}]{path}[/link]")
    except Exception:
        print(f"{label}: {path}")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="ambition-procfit",
        description="Fit procedural shape templates to reference concept art crops.",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    crop = sub.add_parser(
        "crop",
        help="Crop a region from a larger concept sheet. Use pixels or normalized coordinates.",
    )
    crop.add_argument("--image", required=True, help="Input concept sheet image.")
    crop.add_argument(
        "--box",
        required=True,
        help="Crop box as x0,y0,x1,y1. Values <= 1 are interpreted as normalized.",
    )
    crop.add_argument("--out", required=True, help="Output crop PNG path.")

    init = sub.add_parser(
        "init-template", help="Seed a trainable YAML template from a target image."
    )
    init.add_argument(
        "--target", required=True, help="Reference image or crop to approximate."
    )
    init.add_argument("--out", required=True, help="Output YAML template path.")
    init.add_argument(
        "--size",
        type=int,
        default=192,
        help="Square fitting resolution used by the template.",
    )
    init.add_argument(
        "--rects", type=int, default=32, help="Number of rectangle seeds."
    )
    init.add_argument(
        "--ellipses", type=int, default=8, help="Number of ellipse seeds."
    )
    init.add_argument(
        "--superellipses",
        type=int,
        default=0,
        help="Number of superellipse seeds. These can morph between ellipse-like and rectangle-like geometry during optimization.",
    )
    init.add_argument(
        "--segments", type=int, default=10, help="Number of line segment seeds."
    )

    render = sub.add_parser("render", help="Render a YAML template to an image.")
    render.add_argument("--template", required=True, help="Template YAML path.")
    render.add_argument("--out", required=True, help="Output PNG path.")
    render.add_argument("--width", type=int, default=None)
    render.add_argument("--height", type=int, default=None)
    render.add_argument("--device", default="auto", choices=["auto", "cpu", "cuda"])
    render.add_argument("--supersample", type=int, default=2)
    render.add_argument(
        "--threads",
        type=int,
        default=4,
        help="Torch CPU worker threads; use 0 to preserve PyTorch default.",
    )

    fit = sub.add_parser("fit", help="Optimize a template to a target image crop.")
    fit.add_argument("--template", required=True, help="Initial template YAML path.")
    fit.add_argument(
        "--target", required=True, help="Reference image or crop to approximate."
    )
    fit.add_argument(
        "--out-dir",
        required=True,
        help="Directory for optimized template, render, comparison, metrics.",
    )
    fit.add_argument("--steps", type=int, default=300)
    fit.add_argument("--lr", type=float, default=0.05)
    fit.add_argument("--size", type=int, default=192)
    fit.add_argument("--device", default="auto", choices=["auto", "cpu", "cuda"])
    fit.add_argument("--supersample", type=int, default=1)
    fit.add_argument(
        "--sharpness",
        type=float,
        default=90.0,
        help="Legacy constant sharpness control; also used as a floor for profile-based annealing.",
    )
    fit.add_argument(
        "--sharpness-start",
        type=float,
        default=None,
        help="Initial soft-render sharpness. If omitted, profile defaults are used.",
    )
    fit.add_argument(
        "--sharpness-end",
        type=float,
        default=None,
        help="Final sharpness at the end of optimization. If omitted, profile defaults are used.",
    )
    fit.add_argument("--restarts", type=int, default=1)
    fit.add_argument(
        "--mode",
        default="generic",
        choices=["generic", "background", "sprite"],
        help="Tuning profile. Use sprite for crisper edges and stronger high-frequency matching.",
    )
    fit.add_argument("--seed", type=int, default=0)
    fit.add_argument("--log-every", type=int, default=25)
    fit.add_argument(
        "--save-debug",
        action="store_true",
        help="Write intermediate renders and comparison frames during optimization.",
    )
    fit.add_argument(
        "--debug-every",
        type=int,
        default=50,
        help="Capture a debug frame every N steps.",
    )
    fit.add_argument(
        "--debug-max-frames",
        type=int,
        default=24,
        help="Soft limit on the number of intermediate frames saved per restart.",
    )
    fit.add_argument(
        "--threads",
        type=int,
        default=4,
        help="Torch CPU worker threads; use 0 to preserve PyTorch default.",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command == "crop":
        img = ImageOps.exif_transpose(Image.open(args.image).convert("RGBA"))
        vals = [float(v.strip()) for v in args.box.split(",")]
        if len(vals) != 4:
            raise SystemExit("--box must have four comma-separated values: x0,y0,x1,y1")
        if max(vals) <= 1.0:
            vals = [
                vals[0] * img.width,
                vals[1] * img.height,
                vals[2] * img.width,
                vals[3] * img.height,
            ]
        box = tuple(int(round(v)) for v in vals)
        out = Path(args.out)
        out.parent.mkdir(parents=True, exist_ok=True)
        img.crop(box).save(out)
        _print_path("crop", out)
        print(f"box: {box}")
        return 0
    if args.command == "init-template":
        spec = init_template_from_image(
            args.target,
            args.out,
            size=args.size,
            rects=args.rects,
            ellipses=args.ellipses,
            superellipses=args.superellipses,
            segments=args.segments,
        )
        _print_path("template", Path(args.out))
        print(f"primitives: {len(spec.primitives)}")
        return 0
    if args.command == "render":
        out = render_template(
            args.template,
            args.out,
            width=args.width,
            height=args.height,
            device=args.device,
            supersample=args.supersample,
            threads=args.threads,
        )
        _print_path("render", out)
        return 0
    if args.command == "fit":
        out_dir = Path(args.out_dir)
        out_dir.mkdir(parents=True, exist_ok=True)
        _print_path("output_dir", out_dir)
        if args.save_debug:
            debug_dir = out_dir / "debug"
            debug_dir.mkdir(parents=True, exist_ok=True)
            _print_path("debug_dir", debug_dir)
        result = fit_template(
            args.template,
            args.target,
            out_dir,
            steps=args.steps,
            lr=args.lr,
            size=args.size,
            device=args.device,
            supersample=args.supersample,
            sharpness=args.sharpness,
            sharpness_start=args.sharpness_start,
            sharpness_end=args.sharpness_end,
            restarts=args.restarts,
            seed=args.seed,
            log_every=args.log_every,
            threads=args.threads,
            mode=args.mode,
            save_debug=args.save_debug,
            debug_every=args.debug_every,
            debug_max_frames=args.debug_max_frames,
        )
        print(f"best_loss: {result.best_loss:.6f}")
        _print_path("optimized_template", result.template_path)
        _print_path("optimized_render", result.render_path)
        _print_path("comparison", result.comparison_path)
        _print_path("metrics", result.metrics_path)
        return 0
    parser.error(f"Unhandled command: {args.command}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
