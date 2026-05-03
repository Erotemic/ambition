from __future__ import annotations

from pathlib import Path
from typing import Optional

import typer
import yaml
from rich.console import Console

from .adapters import TARGETS, get_adapter
from .canonical import render_canonical, render_canonical_contact_sheet
from .config import CharacterJob, RenderConfig, load_job, save_job
from .sheet import render_spritesheet

PROJECT_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONFIG_DIR = PROJECT_ROOT / "proc2d_character_lab" / "configs"
DEFAULT_ASSET_DIR = PROJECT_ROOT / "assets"

app = typer.Typer(help="YAML-driven procedural 2D character rigging and sprite-sheet generation.")
console = Console()


@app.command("list-targets")
def list_targets() -> None:
    for name, adapter in TARGETS.items():
        console.print(f"[bold]{name}[/bold]: animations={', '.join(adapter.default_animations())}")


@app.command("init-config")
def init_config(
    target: str = typer.Option("goblin"),
    out: Path = typer.Option(Path("character.yaml")),
    seed: int = 0,
    archetype: str = "default",
    held_item: Optional[str] = None,
) -> None:
    adapter = get_adapter(target)
    render = RenderConfig()
    render.frame_width = 128
    render.frame_height = 128
    render.single_width = 128
    render.single_height = 128
    render.supersample = 6
    render.downsample = "lanczos"
    render.background = "transparent"
    render.sheet_background = "transparent"
    job = CharacterJob(
        target=target,
        seed=seed,
        archetype=archetype,
        held_item=held_item,
        animations=adapter.default_animations(),
        render=render,
    )
    save_job(job, out)
    console.print(f"wrote {out}")


@app.command("spec")
def spec(config: Path) -> None:
    job = load_job(config)
    adapter = get_adapter(job.target)
    spec_obj = adapter.sample_spec(job)
    console.print(yaml.safe_dump(adapter.spec_dict(spec_obj), sort_keys=False))


@app.command("single")
def single(config: Path, out: Path, animation: str = "idle", frame_index: int = 0) -> None:
    job = load_job(config)
    adapter = get_adapter(job.target)
    if animation not in adapter.animations():
        raise typer.BadParameter(f"unknown animation {animation}; available={sorted(adapter.animations())}")
    spec_obj = adapter.sample_spec(job)
    img = adapter.render_single(spec_obj, animation, frame_index, job)
    out.parent.mkdir(parents=True, exist_ok=True)
    img.save(out)
    console.print(f"wrote {out}")


@app.command("canonical")
def canonical(config: Path, out: Path) -> None:
    job = load_job(config)
    adapter = get_adapter(job.target)
    img, manifest = render_canonical(adapter, job)
    out.parent.mkdir(parents=True, exist_ok=True)
    img.save(out)
    console.print(f"wrote {out}")
    console.print(yaml.safe_dump(manifest, sort_keys=False))


@app.command("spritesheet")
def spritesheet(config: Path, out: Path, manifest_out: Optional[Path] = None) -> None:
    job = load_job(config)
    adapter = get_adapter(job.target)
    manifest = render_spritesheet(adapter, job, out, manifest_out)
    console.print(f"wrote {out}")
    if manifest_out:
        console.print(f"wrote {manifest_out}")
    console.print(f"frames: {len(manifest['frames'])}")


@app.command("draw-all")
def draw_all(
    config_dir: Path = typer.Option(DEFAULT_CONFIG_DIR, help="Directory containing character YAML configs."),
    out_dir: Path = typer.Option(DEFAULT_ASSET_DIR, help="Directory where sprite sheets and manifests are written."),
    pattern: str = typer.Option("*.yaml", help="Glob pattern for character YAML configs."),
) -> None:
    """Render all character YAML configs to the project-local assets directory."""
    if not config_dir.exists():
        raise typer.BadParameter(f"config directory does not exist: {config_dir}")
    out_dir.mkdir(parents=True, exist_ok=True)
    configs = sorted(config_dir.glob(pattern))
    if not configs:
        raise typer.BadParameter(f"no configs matched {pattern!r} in {config_dir}")
    total_frames = 0
    for config in configs:
        job = load_job(config)
        adapter = get_adapter(job.target)
        stem = config.stem
        sheet_out = out_dir / f"{stem}_spritesheet.png"
        manifest_out = out_dir / f"{stem}_spritesheet.yaml"
        manifest = render_spritesheet(adapter, job, sheet_out, manifest_out)
        total_frames += len(manifest["frames"])
        console.print(f"[bold]{stem}[/bold] -> {sheet_out}")
        console.print(f"[dim]{stem} manifest -> {manifest_out} ({len(manifest['frames'])} frames)[/dim]")
    console.print(f"rendered {len(configs)} character configs, {total_frames} frames total")


@app.command("draw-canonicals")
def draw_canonicals(
    config_dir: Path = typer.Option(DEFAULT_CONFIG_DIR, help="Directory containing character YAML configs."),
    out_dir: Path = typer.Option(DEFAULT_ASSET_DIR / "canonicals", help="Directory where canonical frames are written."),
    pattern: str = typer.Option("*.yaml", help="Glob pattern for character YAML configs."),
    make_sheet: bool = typer.Option(True, help="Also emit a small contact sheet."),
) -> None:
    if not config_dir.exists():
        raise typer.BadParameter(f"config directory does not exist: {config_dir}")
    out_dir.mkdir(parents=True, exist_ok=True)
    configs = sorted(config_dir.glob(pattern))
    if not configs:
        raise typer.BadParameter(f"no configs matched {pattern!r} in {config_dir}")
    entries = []
    for config in configs:
        job = load_job(config)
        adapter = get_adapter(job.target)
        image, _ = render_canonical(adapter, job)
        out_path = out_dir / f"{config.stem}_canonical.png"
        image.save(out_path)
        console.print(f"[bold]{config.stem}[/bold] -> {out_path}")
        entries.append((config.stem, image))
    if make_sheet and entries:
        sheet = render_canonical_contact_sheet(entries)
        sheet_path = out_dir / "canonicals_contact_sheet.png"
        sheet.save(sheet_path)
        console.print(f"contact sheet -> {sheet_path}")


@app.command("validate-config")
def validate_config(config: Path) -> None:
    job = load_job(config)
    adapter = get_adapter(job.target)
    missing = [a for a in (job.animations or adapter.default_animations()) if a not in adapter.animations()]
    if missing:
        raise typer.BadParameter(f"unknown animations for {job.target}: {missing}")
    console.print(f"valid: target={job.target}, seed={job.seed}, archetype={job.archetype}")


if __name__ == "__main__":
    app()
