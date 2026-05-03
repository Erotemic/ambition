from __future__ import annotations

import os
from pathlib import Path
from typing import Optional

import typer
import yaml
from PIL import Image, ImageDraw, ImageFont
from rich.console import Console
from rich.markup import escape

from .adapters import TARGETS, get_adapter
from .canonical import render_canonical, render_canonical_contact_sheet
from .config import CharacterJob, RenderConfig, load_job, save_job
from .sheet import render_spritesheet
from .textures import generate_texture_pack

DEFAULT_GEN3D_ROOT = Path(os.environ.get("GEN3D_BLENDER_LAB_ROOT", Path(__file__).resolve().parents[1]))
DEFAULT_CONFIG_DIR = Path(os.environ.get("GEN3D_BLENDER_LAB_CONFIG_DIR", DEFAULT_GEN3D_ROOT / "gen3d_blender_lab" / "configs"))
DEFAULT_ASSET_DIR = Path(os.environ.get("GEN3D_BLENDER_LAB_ASSET_DIR", DEFAULT_GEN3D_ROOT / "assets"))

app = typer.Typer(help="Blender-first procedural character sprite generation.")
console = Console()


def _texture_preview_sheet(texture_groups: list[tuple[str, dict[str, str]]], out_path: Path) -> Path:
    if not texture_groups:
        return out_path
    tile = 128
    gutter = 14
    header_h = 34
    label_h = 22
    max_cols = max(len(paths) for _, paths in texture_groups)
    width = 180 + max_cols * (tile + gutter) + gutter
    height = gutter + len(texture_groups) * (header_h + tile + label_h + gutter)
    sheet = Image.new("RGBA", (width, height), (245, 247, 250, 255))
    draw = ImageDraw.Draw(sheet)
    font_big = ImageFont.load_default()
    font_small = ImageFont.load_default()
    y = gutter
    for stem, texmap in texture_groups:
        draw.text((gutter, y), stem, fill=(24, 26, 34, 255), font=font_big)
        x = 180
        y_img = y + header_h
        for key, path in sorted(texmap.items()):
            img = Image.open(path).convert("RGBA").resize((tile, tile))
            sheet.alpha_composite(img, (x, y_img))
            draw.rectangle((x, y_img, x + tile, y_img + tile), outline=(80, 84, 96, 255), width=1)
            draw.text((x + 4, y_img + tile + 2), key, fill=(40, 42, 52, 255), font=font_small)
            x += tile + gutter
        y += header_h + tile + label_h + gutter
    out_path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out_path)
    return out_path


def _file_link(path: Path, label: str | None = None) -> str:
    resolved = Path(path).expanduser().resolve()
    text = escape(label or str(resolved))
    return f"[link=file://{resolved}]{text}[/link]"


@app.command("list-targets")
def list_targets() -> None:
    for name, adapter in TARGETS.items():
        console.print(f"[bold]{name}[/bold]: animations={', '.join(adapter.default_animations())}")


@app.command("init-config")
def init_config(target: str = typer.Option("goblin"), out: Path = typer.Option(Path("character.yaml")), seed: int = 0, archetype: str = "default", held_item: Optional[str] = None) -> None:
    adapter = get_adapter(target)
    render = RenderConfig()
    if target == "robot":
        render.frame_width = 128
        render.frame_height = 128
        render.single_width = 640
        render.single_height = 640
        render.background = "transparent"
        render.sheet_background = "#F4F4F4"
        render.label_width = 112
    job = CharacterJob(target=target, seed=seed, archetype=archetype, held_item=held_item, animations=adapter.default_animations(), render=render)
    save_job(job, out)
    console.print(f"wrote {out}")


@app.command("spec")
def spec(config: Path) -> None:
    job = load_job(config)
    adapter = get_adapter(job.target)
    spec_obj = adapter.sample_spec(job)
    console.print(yaml.safe_dump(adapter.spec_dict(spec_obj), sort_keys=False))


@app.command("canonical")
def canonical(config: Path, out: Path) -> None:
    job = load_job(config)
    adapter = get_adapter(job.target)
    info = render_canonical(adapter, job, out)
    out_path = Path(info["out"])
    console.print(f"[bold green]Canonical image:[/bold green] {_file_link(out_path)}")


@app.command("canonical-all")
def canonical_all(
    config_dir: Path = typer.Option(DEFAULT_CONFIG_DIR, help="Directory containing character YAML configs."),
    out_dir: Path = typer.Option(DEFAULT_ASSET_DIR, help="Directory for canonical outputs."),
    pattern: str = typer.Option("*.yaml", help="Glob pattern for character YAML configs."),
    contact_sheet: bool = typer.Option(True, help="Also draw a quick review contact sheet."),
) -> None:
    if not config_dir.exists():
        raise typer.BadParameter(f"config directory does not exist: {config_dir}")
    out_dir.mkdir(parents=True, exist_ok=True)
    configs = sorted(config_dir.glob(pattern))
    if not configs:
        raise typer.BadParameter(f"no configs matched {pattern!r} in {config_dir}")
    items = []
    for config in configs:
        job = load_job(config)
        adapter = get_adapter(job.target)
        out = out_dir / f"{config.stem}_canonical.png"
        info = render_canonical(adapter, job, out)
        items.append(info)
        console.print(f"[bold]{config.stem}[/bold] -> {out}")
    if contact_sheet:
        sheet_out = out_dir / "character_canonicals.png"
        render_canonical_contact_sheet(items, sheet_out)
        console.print(f"[bold green]Canonical contact sheet:[/bold green] {_file_link(sheet_out)}")


@app.command("spritesheet")
def spritesheet(config: Path, out: Path, manifest_out: Optional[Path] = None) -> None:
    job = load_job(config)
    adapter = get_adapter(job.target)
    if manifest_out is None:
        manifest_out = out.with_suffix(".yaml")
    manifest = render_spritesheet(adapter, job, out, manifest_out)
    console.print(f"[bold green]Sprite sheet:[/bold green] {_file_link(out)}")
    console.print(f"[bold green]Manifest:[/bold green] {_file_link(manifest_out)}")
    console.print(f"frames: {len(manifest['frames'])}")


@app.command("draw-all")
def draw_all(
    config_dir: Path = typer.Option(DEFAULT_CONFIG_DIR, help="Directory containing character YAML configs."),
    out_dir: Path = typer.Option(DEFAULT_ASSET_DIR, help="Directory where sprite sheets and manifests are written."),
    pattern: str = typer.Option("*.yaml", help="Glob pattern for character YAML configs."),
) -> None:
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
    canonical_path = out_dir / "character_canonicals.png"
    if canonical_path.exists():
        console.print(f"[bold green]Canonical contact sheet:[/bold green] {_file_link(canonical_path)}")
    else:
        console.print("[dim]Canonical contact sheet not found yet. Run: python -m gen3d_blender_lab.cli canonical-all[/dim]")


@app.command("textures-all")
def textures_all(
    config_dir: Path = typer.Option(DEFAULT_CONFIG_DIR, help="Directory containing character YAML configs."),
    out_dir: Path = typer.Option(DEFAULT_ASSET_DIR / "textures", help="Directory where generated texture previews are written."),
    pattern: str = typer.Option("*.yaml", help="Glob pattern for character YAML configs."),
) -> None:
    if not config_dir.exists():
        raise typer.BadParameter(f"config directory does not exist: {config_dir}")
    out_dir.mkdir(parents=True, exist_ok=True)
    configs = sorted(config_dir.glob(pattern))
    if not configs:
        raise typer.BadParameter(f"no configs matched {pattern!r} in {config_dir}")
    texture_groups = []
    for config in configs:
        job = load_job(config)
        adapter = get_adapter(job.target)
        spec = adapter.spec_dict(adapter.sample_spec(job))
        tex_dir = out_dir / config.stem
        texture_paths = generate_texture_pack(tex_dir, spec, adapter.target)
        texture_groups.append((config.stem, texture_paths))
        console.print(f"[bold]{config.stem}[/bold] textures -> {tex_dir}")
        for key, path in sorted(texture_paths.items()):
            console.print(f"[dim]{key}: {path}[/dim]")
    preview_out = out_dir / "texture_previews.png"
    _texture_preview_sheet(texture_groups, preview_out)
    console.print(f"[bold green]Texture preview sheet:[/bold green] {_file_link(preview_out)}")


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
