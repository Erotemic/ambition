#!/usr/bin/env python3
from __future__ import annotations

"""Render every configured procedural 2D character spritesheet.

Run this from the project root, for example:

    python draw_all_character_spritesheets.py

By default it reads configs from ``./proc2d_character_lab/configs`` and writes
spritesheets and manifests to ``./assets``.
"""

from pathlib import Path

import typer

from proc2d_character_lab.adapters import get_adapter
from proc2d_character_lab.cli import DEFAULT_ASSET_DIR, DEFAULT_CONFIG_DIR
from proc2d_character_lab.config import load_job
from proc2d_character_lab.sheet import render_spritesheet

app = typer.Typer(help="Draw all procedural 2D character sprite sheets.")


@app.command()
def main(
    config_dir: Path = typer.Option(DEFAULT_CONFIG_DIR, help="Directory containing character YAML configs."),
    out_dir: Path = typer.Option(DEFAULT_ASSET_DIR, help="Output directory for PNG/YAML assets."),
    pattern: str = typer.Option("*.yaml", help="Glob for character config files."),
) -> None:
    if not config_dir.exists():
        raise typer.BadParameter(f"config directory does not exist: {config_dir}")
    out_dir.mkdir(parents=True, exist_ok=True)
    configs = sorted(config_dir.glob(pattern))
    if not configs:
        raise typer.BadParameter(f"no configs matched {pattern!r} in {config_dir}")
    for config in configs:
        job = load_job(config)
        adapter = get_adapter(job.target)
        sheet_out = out_dir / f"{config.stem}_spritesheet.png"
        manifest_out = out_dir / f"{config.stem}_spritesheet.yaml"
        render_spritesheet(adapter, job, sheet_out, manifest_out)
        print(f"wrote {sheet_out}")
        print(f"wrote {manifest_out}")


if __name__ == "__main__":
    app()
