from __future__ import annotations

"""Music-faction character lineup renderer.

This is intentionally a thin layer over the existing per-character YAML jobs.
The near-term goal is to make faction / leader review sprites feel as data-driven
as the music cues without forcing every runtime sprite sheet through a new schema.
"""

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Tuple

import yaml
from PIL import Image, ImageDraw

from ..registry import CharacterJob, RenderConfig
from .rendering import load_font
from .sheet import write_spritesheet
from .canonical import render_canonical


@dataclass(frozen=True)
class FactionCharacter:
    id: str
    display_name: str
    target: str
    archetype: str
    role: str = "npc"
    seed: int = 0
    variant: Optional[str] = None
    held_item: Optional[str] = None
    animations: List[str] = field(default_factory=list)
    render: Dict[str, Any] = field(default_factory=dict)
    notes: str = ""

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "FactionCharacter":
        return cls(
            id=str(data["id"]),
            display_name=str(data.get("display_name") or data["id"]),
            target=str(data["target"]),
            archetype=str(data.get("archetype", "default")),
            role=str(data.get("role", "npc")),
            seed=int(data.get("seed", 0)),
            variant=data.get("variant"),
            held_item=data.get("held_item"),
            animations=list(data.get("animations") or []),
            render=dict(data.get("render") or {}),
            notes=str(data.get("notes", "")),
        )

    def to_job(
        self, faction_id: str, music_cue: str, default_render: Dict[str, Any]
    ) -> CharacterJob:
        render_data = dict(default_render)
        render_data.update(self.render)
        return CharacterJob(
            target=self.target,
            name=self.id,
            seed=self.seed,
            archetype=self.archetype,
            variant=self.variant,
            held_item=self.held_item,
            animations=list(self.animations),
            render=RenderConfig(**render_data),
            faction=faction_id,
            role=self.role,
            music_cue=music_cue,
            tags=["faction", self.role],
        )


@dataclass(frozen=True)
class MusicFaction:
    id: str
    display_name: str
    music_cue: str
    music_source: str
    palette_hint: str = ""
    lair_hook: str = ""
    characters: List[FactionCharacter] = field(default_factory=list)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "MusicFaction":
        chars = [
            FactionCharacter.from_dict(item) for item in data.get("characters", [])
        ]
        return cls(
            id=str(data["id"]),
            display_name=str(data.get("display_name") or data["id"]),
            music_cue=str(data["music_cue"]),
            music_source=str(data.get("music_source", "")),
            palette_hint=str(data.get("palette_hint", "")),
            lair_hook=str(data.get("lair_hook", "")),
            characters=chars,
        )


@dataclass(frozen=True)
class FactionLineup:
    title: str
    default_animations: List[str]
    default_render: Dict[str, Any]
    factions: List[MusicFaction]

    @classmethod
    def load(cls, path: str | Path) -> "FactionLineup":
        path = Path(path)
        with path.open("r", encoding="utf8") as file:
            data = yaml.safe_load(file) or {}
        if not isinstance(data, dict):
            raise TypeError(f"expected mapping in {path}")
        default_animations = list(
            data.get("default_animations") or ["idle", "walk", "talk", "hit", "death"]
        )
        default_render = dict(data.get("default_render") or {})
        factions = [MusicFaction.from_dict(item) for item in data.get("factions", [])]
        if not factions:
            raise ValueError(f"no factions defined in {path}")
        return cls(
            title=str(data.get("title") or path.stem),
            default_animations=default_animations,
            default_render=default_render,
            factions=factions,
        )

    def iter_jobs(
        self,
    ) -> Iterable[Tuple[MusicFaction, FactionCharacter, CharacterJob]]:
        for faction in self.factions:
            for character in faction.characters:
                job = character.to_job(
                    faction.id, faction.music_cue, self.default_render
                )
                if not job.animations:
                    job.animations = list(self.default_animations)
                yield faction, character, job


def _safe_name(value: str) -> str:
    return "".join(
        ch if ch.isalnum() or ch in {"_", "-"} else "_" for ch in value.lower()
    ).strip("_")


def _write_contact_sheet(tiles: List[Tuple[str, str, Image.Image]], out: Path) -> None:
    if not tiles:
        return
    font = load_font(12)
    small = load_font(10)
    probe = Image.new("RGBA", (1, 1), (0, 0, 0, 0))
    draw_probe = ImageDraw.Draw(probe)
    tile_w = 178
    tile_h = 170
    cols = min(6, max(1, len(tiles)))
    rows = (len(tiles) + cols - 1) // cols
    contact = Image.new("RGBA", (cols * tile_w, rows * tile_h), (18, 20, 27, 255))
    draw = ImageDraw.Draw(contact)
    for idx, (name, subtitle, img) in enumerate(tiles):
        col = idx % cols
        row = idx // cols
        x = col * tile_w
        y = row * tile_h
        draw.rounded_rectangle(
            (x + 5, y + 5, x + tile_w - 5, y + tile_h - 5),
            radius=10,
            fill=(28, 32, 43, 255),
            outline=(70, 78, 104, 255),
            width=1,
        )
        # Fit the canonical image into the card without distorting it.
        max_w, max_h = 112, 108
        scale = min(max_w / img.width, max_h / img.height, 1.0)
        if scale < 1.0:
            shown = img.resize(
                (max(1, int(img.width * scale)), max(1, int(img.height * scale))),
                Image.Resampling.LANCZOS,
            )
        else:
            shown = img
        contact.alpha_composite(shown, (x + (tile_w - shown.width) // 2, y + 35))
        for text, ty, fnt, fill in [
            (name, 11, font, (245, 247, 255, 255)),
            (subtitle, 146, small, (190, 198, 220, 255)),
        ]:
            bbox = draw_probe.textbbox((0, 0), text, font=fnt)
            tw = bbox[2] - bbox[0]
            draw.text(
                (x + max(8, (tile_w - tw) // 2), y + ty), text, font=fnt, fill=fill
            )
    out.parent.mkdir(parents=True, exist_ok=True)
    contact.save(out)


def write_faction_lineup(config_path: str | Path, out_dir: str | Path) -> List[Path]:
    lineup = FactionLineup.load(config_path)
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs: List[Path] = []
    manifest: Dict[str, Any] = {
        "title": lineup.title,
        "factions": [],
    }
    tiles: List[Tuple[str, str, Image.Image]] = []
    for faction, character, job in lineup.iter_jobs():
        stem = _safe_name(character.id)
        image_out = out_dir / f"{stem}_spritesheet.png"
        manifest_out = out_dir / f"{stem}_spritesheet.yaml"
        outputs.extend(write_spritesheet(job, image_out, manifest_out))
        img = render_canonical(job)
        canonical_out = out_dir / f"{stem}_canonical.png"
        img.save(canonical_out)
        outputs.append(canonical_out)
        tiles.append((character.display_name, faction.display_name, img))
        manifest["factions"].append(
            {
                "faction_id": faction.id,
                "display_name": faction.display_name,
                "music_cue": faction.music_cue,
                "music_source": faction.music_source,
                "palette_hint": faction.palette_hint,
                "lair_hook": faction.lair_hook,
                "character": {
                    "id": character.id,
                    "display_name": character.display_name,
                    "target": character.target,
                    "archetype": character.archetype,
                    "role": character.role,
                    "held_item": character.held_item,
                    "animations": list(job.animations),
                    "notes": character.notes,
                },
                "outputs": {
                    "spritesheet": image_out.name,
                    "manifest": manifest_out.name,
                    "canonical": canonical_out.name,
                },
            }
        )
    manifest_out = out_dir / "faction_lineup_manifest.yaml"
    with manifest_out.open("w", encoding="utf8") as file:
        yaml.safe_dump(manifest, file, sort_keys=False)
    outputs.append(manifest_out)
    contact_out = out_dir / "faction_leaders_contact_sheet.png"
    _write_contact_sheet(tiles, contact_out)
    outputs.append(contact_out)
    return outputs
