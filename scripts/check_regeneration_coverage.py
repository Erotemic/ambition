#!/usr/bin/env python3
"""Check required assets against the regeneration plans and published files."""

from __future__ import annotations

import argparse
import json
import os
import re
import struct
import subprocess
import sys
from pathlib import Path
from typing import Callable

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from lib.content_libraries import selected_manifest, generated_root, producer, runtime_files


ProductReader = Callable[[str], Path | None]


def product_reader(root: Path | None, plans_only: bool) -> ProductReader:
    if plans_only:
        return lambda logical: None
    files = runtime_files(ROOT) if root is None else None
    def read(logical: str) -> Path | None:
        path = root / logical if root is not None else files.get(logical)
        return path if path is not None and path.is_file() else None
    return read


def tool_json(tool: str, override: str, code: str):
    command = '''source "$1/scripts/lib/tool_python.sh"
project="$1/tools/$2"
interpreter="$(ambition_select_tool_python "$project" "$3")"
cd "$project"
"$interpreter" -c "$4"
'''
    result = subprocess.run(
        ["bash", "-c", command, "coverage", str(ROOT), tool, override, code],
        check=True, capture_output=True, text=True,
    )
    return json.loads(result.stdout)


def authored_sources():
    paths = subprocess.run(
        ["git", "ls-files", "-z", "game/**/*.ron", "game/**/*.rs"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout.split("\0")
    for relative in paths:
        if not relative or "tests" in Path(relative).parts:
            continue
        text = (ROOT / relative).read_text(encoding="utf8")
        if relative.endswith(".rs"):
            text = re.split(r"#\[cfg\(test\)\]", text, maxsplit=1)[0]
        text = re.sub(r'"(?:\\.|[^"\\])*"|//[^\n]*|/\*.*?\*/',
                      lambda match: match[0] if match[0].startswith('"') else " ",
                      text, flags=re.S)
        yield relative, text


def referenced_files(sources, pattern: str) -> dict[str, set[str]]:
    found: dict[str, set[str]] = {}
    for source, text in sources:
        for asset in re.findall(pattern, text):
            found.setdefault(asset, set()).add(source)
    return found


def check_sprites(read_product: ProductReader, sources, plans_only: bool) -> list[str]:
    result = subprocess.run(
        [str(ROOT / "scripts/regen/sprites.sh"), "--list-outputs"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    )
    planned = {f"sprites/{name}" for name in result.stdout.splitlines() if name}
    required = referenced_files(sources, r'"(sprites/[^"\n]+_spritesheet\.(?:ron|png))"')
    failures = []
    for asset, owners in sorted(required.items()):
        if asset not in planned:
            failures.append(f"sprites: no regen target publishes {asset} (used by {', '.join(sorted(owners))})")
        elif not plans_only and not read_product(asset):
            failures.append(f"sprites: missing {asset}; run scripts/regen/sprites.sh")
    print(f"sprites: {len(required)} required sheet files, {len(planned)} planned outputs")
    return failures


def check_music(read_product: ProductReader, plans_only: bool) -> list[str]:
    import regen_music_registry as music_registry
    cues = music_registry.generation_cues()
    if not cues:
        raise ValueError("music discovery returned no cues")
    if plans_only:
        registry_text = music_registry.render_registry()
    else:
        registry = read_product(producer(ROOT, "music")["registry"])
        if registry is None:
            raise ValueError("The selected library has no accepted music registry")
        registry_text = registry.read_text()
    registered = music_registry.tracked_ids(registry_text)
    failures = []
    aliases = {entry[0] for entry in music_registry.SPECIAL_ENTRIES}
    for track in sorted(registered - set(cues) - aliases):
        failures.append(f"music: registered track {track} has no regeneration cue")
    for cue in cues:
        if cue not in music_registry.CLAIMED_DIRS and not music_registry.is_denied(cue) and cue not in registered:
            failures.append(f"music: {cue} has a score but no registry entry; run scripts/regen/music.sh")
        if not plans_only and read_product(f"audio/music/generated/{cue}/full.ogg") is None:
            failures.append(f"music: missing {cue}/full.ogg; run scripts/regen/music.sh")
    for track, _, asset in music_registry.SPECIAL_ENTRIES:
        if track not in registered:
            failures.append(f"music: missing registry entry {track}")
        if not plans_only and read_product(asset) is None:
            failures.append(f"music: missing {asset}; run scripts/regen/music.sh")
    print(f"music: {len(cues)} planned cues, {len(registered)} registered tracks")
    return failures


def bank_names(path: Path) -> set[str]:
    with path.open("rb") as bank:
        magic, version, count, _, _, names_offset = struct.unpack("<8sIIQQQ", bank.read(40))
        if magic != b"AMBNDSFX" or version != 1:
            raise ValueError(f"unsupported SFX bank: {path}")
        bank.seek(names_offset)
        names = set()
        for _ in range(count):
            length, = struct.unpack("<H", bank.read(2))
            names.add(bank.read(length).decode("utf8"))
        return names


def check_sfx(read_product: ProductReader, plans_only: bool) -> list[str]:
    cues = tool_json(
        "ambition_sfx_renderer", "AMBITION_SFX_PYTHON",
        "import json; from ambition_sfx_renderer.schema import iter_cue_files, load_cue; "
        f"print(json.dumps([load_cue(path).cue_id for path in iter_cue_files(group={producer(ROOT, 'sfx')['collection']!r})]))",
    )
    if not cues:
        raise ValueError("SFX discovery returned no cues")
    failures = []
    if not plans_only:
        path = read_product("audio/sfx.bank")
        if path is None:
            raise ValueError("The selected library has no accepted SFX bank")
        bank = bank_names(path)
        for cue in sorted(set(cues) - bank):
            failures.append(f"sfx: {cue} is absent from sfx.bank; run scripts/regen/sfx.sh")
    print(f"sfx: {len(cues)} authored cues")
    return failures


def check_backgrounds(read_product: ProductReader, sources, plans_only: bool) -> list[str]:
    required = referenced_files(sources, r'"((?:backgrounds|room_dressing)/[^"{}\n]+\.png)"')
    planned = set(tool_json(
        "ambition_sprite2d_renderer", "AMBITION_SPRITE_PYTHON",
        """import json
import os
from ambition_sprite2d_renderer.backgrounds.scenes import THEMES, LAYERS
from ambition_sprite2d_renderer.backgrounds.sky_parts import PARTS
from ambition_sprite2d_renderer.terrain import skins, motes, decor, doors, shimmer, water_backdrops
paths = [f'backgrounds/parallax_layers/{theme}_{layer}.png' for theme in THEMES for layer in LAYERS]
paths += [f'backgrounds/sky_parts/{part}.png' for part in PARTS]
paths += [f'room_dressing/{skin.key}_{part}.png' for skin in skins.SKINS for part in skins.PARTS]
paths += [f'room_dressing/{theme}_water_backdrop.png' for theme in water_backdrops.BACKDROPS]
for registry, parts in [(motes.MOTES, ['motes']), (motes.SHADOWS, ['shadow']), (decor.DECOR, ['decor', 'decor_glow']), (doors.DOORS, ['door']), (shimmer.SHIMMERS, ['shimmer'])]:
    paths += [f'room_dressing/{key}_{part}.png' for key in registry for part in parts]
print(json.dumps(paths))
""",
    ))
    planned.update(tool_json(
        "ambition_background_renderer", "AMBITION_BACKGROUND_PYTHON",
        "import json; from ambition_background_renderer.profiles import iter_profiles; "
        "print(json.dumps([f'backgrounds/{profile.name}/{layer.name}.png' for profile in iter_profiles('all') for layer in profile.layers]))",
    ))
    failures = []
    for asset in sorted(set(required) - planned):
        failures.append(f"backgrounds: no regen target publishes {asset}")
    if not plans_only:
        for asset in sorted(planned):
            if read_product(asset) is None:
                failures.append(f"backgrounds: missing {asset}; run scripts/regen/backgrounds.sh")
    print(f"backgrounds: {len(planned)} planned files, {len(required)} literal references")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--family", action="append", choices=("sprites", "music", "sfx", "backgrounds"))
    parser.add_argument("--plans-only", action="store_true", help="check coverage without requiring generated files")
    parser.add_argument("--asset-root", type=Path, default=None)
    args = parser.parse_args()
    root = args.asset_root
    if root is None and os.environ.get("AMBITION_PRODUCER_WORKER"):
        root = generated_root(ROOT)
    try:
        read_product = product_reader(root, args.plans_only)
    except (OSError, ValueError) as error:
        print(f"regeneration coverage: FAILED: {error}", file=sys.stderr)
        return 1
    sources = list(authored_sources())
    if not sources:
        parser.error("no authored game sources found")
    failures = []
    declared = json.loads(selected_manifest(ROOT).read_text()).get("producers", {})
    for family in args.family or [name for name in ("sprites", "music", "sfx", "backgrounds") if name in declared]:
        try:
            if family == "sprites":
                failures.extend(check_sprites(read_product, sources, args.plans_only))
            elif family == "backgrounds":
                failures.extend(check_backgrounds(read_product, sources, args.plans_only))
            elif family == "music":
                failures.extend(check_music(read_product, args.plans_only))
            else:
                failures.extend(check_sfx(read_product, args.plans_only))
        except (OSError, ValueError, struct.error, subprocess.CalledProcessError) as error:
            detail = error.stderr if isinstance(error, subprocess.CalledProcessError) else str(error)
            failures.append(f"{family}: cannot check coverage: {detail}")
    for failure in failures:
        print(failure, file=sys.stderr)
    print(f"regeneration coverage: {'FAILED' if failures else 'ok'} ({len(failures)} finding(s))")
    return int(bool(failures))


if __name__ == "__main__":
    raise SystemExit(main())
