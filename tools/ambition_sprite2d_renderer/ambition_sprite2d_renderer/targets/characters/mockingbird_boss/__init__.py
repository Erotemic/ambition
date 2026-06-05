"""Mockingbird boss character — multi-file tack-on target.

A multi-file character package: the boss ships a manifest + per-part
frames assembled by :mod:`.sprite_generator`, with part-config YAML
files alongside the renderer (`mockingbird_boss_parts.yaml`,
`mockingbird_boss_scene.yaml`, `mockingbird_boss_legacy_parts.yaml`)
and a Tk-based part editor (:mod:`.part_editor`) for tuning the rig.

The package layout exists so adding the next multi-file character is a
copy-this-directory operation: drop ``targets/characters/<name>/`` with
the same ``__init__.py`` shape, and discovery picks it up.
"""

from __future__ import annotations

from pathlib import Path
from typing import List

from . import sprite_generator

TARGET_NAME = sprite_generator.TARGET_NAME
SHEET_FILES = list(sprite_generator.OUTPUT_FILES) + [f"{TARGET_NAME}_actor.ron"]

ACTOR_METADATA = {
    "actor": {"character_id": f"npc_{TARGET_NAME}"},
    "body": {
        "body_plan": "BossMultipart",
        "body_kind": "Wide",
        "traits": ["boss", "multipart"],
    },
    "brain": {"default_preset": "stand_still"},
    "actions": {"default_preset": "peaceful"},
    "tags": ["boss", "multipart"],
    "missing_information": [
        "multipart sockets: boss-specific part anchors are still in the JSON manifest/parts files, not normalized into actor sockets",
        "boss schedule/action specials: not authored in the sprite actor contract yet",
    ],
}


def render(out_dir: str | Path, **opts) -> List[Path]:
    return list(
        sprite_generator.render_outputs(
            outdir=out_dir,
            quick=bool(opts.get("quick", False)),
        )
    )


def install(render_dir: str | Path, dest_root: str | Path) -> List[Path]:
    render_dir = Path(render_dir)
    install_dir = Path(dest_root) / TARGET_NAME
    copied = list(
        sprite_generator.install_outputs(
            render_dir=render_dir,
            install_dir=install_dir,
        )
    )
    # Optional actor-contract sidecar emitted by target_registry's post-render
    # hook. Custom boss installers bypass the default copy helper, so carry it
    # explicitly when present.
    actor_src = render_dir / f"{TARGET_NAME}_actor.ron"
    if actor_src.exists():
        install_dir.mkdir(parents=True, exist_ok=True)
        actor_dst = install_dir / actor_src.name
        import shutil

        shutil.copy2(actor_src, actor_dst)
        copied.append(actor_dst)
    return copied
