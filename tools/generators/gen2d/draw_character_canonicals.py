#!/usr/bin/env python3
from __future__ import annotations

"""Render one canonical 2D look-dev frame per configured character.

Run this from the project root, for example:

    python draw_character_canonicals.py

By default it reads configs from ``./proc2d_character_lab/configs`` and writes
canonical renders to ``./assets/canonicals``.
"""

from pathlib import Path

import typer

from proc2d_character_lab.adapters import get_adapter
from proc2d_character_lab.canonical import render_canonical, render_canonical_contact_sheet
from proc2d_character_lab.cli import DEFAULT_ASSET_DIR, DEFAULT_CONFIG_DIR
from proc2d_character_lab.config import load_job

app = typer.Typer(help="Draw one canonical render for each configured character.")


@app.command()
def main(
    config_dir: Path = typer.Option(DEFAULT_CONFIG_DIR, help="Directory containing character YAML configs."),
    out_dir: Path = typer.Option(DEFAULT_ASSET_DIR / "canonicals", help="Output directory for PNG canonical renders."),
    pattern: str = typer.Option("*.yaml", help="Glob for character config files."),
    make_sheet: bool = typer.Option(True, help="Also emit a quick contact sheet."),
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
        print(f"wrote {out_path}")
        entries.append((config.stem, image))
    if make_sheet and entries:
        sheet = render_canonical_contact_sheet(entries)
        sheet_path = out_dir / "canonicals_contact_sheet.png"
        sheet.save(sheet_path)
        print(f"wrote {sheet_path}")


if __name__ == "__main__":
    app()
