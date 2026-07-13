#!/usr/bin/env python3
"""Regenerate ``music_registry.ron`` from the rendered-OGG asset tree.

The music registry is a *projection* of what music actually exists: every
``audio/music/generated/<cue>/full.ogg`` becomes a radio track unless it is
denied (backing-layer stems, superseded mixes). This is the elegant
counterpart to the now-vestigial hand-authored list — registration is an
invariant, not a chore. Run it standalone or via ``regen_music.sh`` (which
calls it after publishing renders).

A registry entry is intentionally trivial: just ``id`` + ``display_name``.
``asset_path`` is omitted for the conventional
``audio/music/generated/<id>/full.ogg`` layout (the Rust ``MusicTrack``
derives it from ``id``) and only spelled out for off-convention assets
(``SPECIAL_ENTRIES``). No tempo/arrangement metadata: the OGG is what plays.

Usage:
    python3 scripts/regen_music_registry.py            # rewrite the registry
    python3 scripts/regen_music_registry.py --check     # fail if out of date
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
ASSETS_ROOT = REPO_ROOT / "crates" / "ambition_actors" / "assets"
GENERATED_DIR = ASSETS_ROOT / "audio" / "music" / "generated"
REGISTRY_PATH = REPO_ROOT / "game" / "ambition_content" / "assets" / "audio" / "music_registry.ron"

# Track played at startup / when no radio station is selected.
DEFAULT_TRACK = "long_lofi_drift"

# Off-convention entries: id != directory, or a non-``full.ogg`` source.
# Each is (id, display_name, asset_path). The directories they consume are
# listed in CLAIMED_DIRS so the scan does not also emit a plain entry.
SPECIAL_ENTRIES = [
    (
        "original_lofi_loop",
        "Original Lofi Loop",
        "audio/music/generated/lofi_study_loop/full.ogg",
    ),
    (
        "first_goblin_tune_v2_radio",
        "First Goblin Tune v2 — Wave 1 Radio Mix",
        "audio/music/generated/first_goblin_tune_v2/adaptive/wave1/wave1.full.ogg",
    ),
]
CLAIMED_DIRS = {"lofi_study_loop", "first_goblin_tune_v2"}

# Generated dirs that exist but should NOT appear as radio songs.
#   - ``flying_spaghetti_monster_fight``: superseded by the ``roots`` family
#     (the boss now uses ``flying_spaghetti_monster_roots_boss_choir_backing``).
# NOTE: ``*_choir_backing`` cues are NOT stems despite the name — they are the
# fuller boss/stage arrangements (boss + choir) and the cues with live scores
# under scores/active, so they ARE registered. Add an id here to retire a cue
# from the radio without deleting its render.
DENY_IDS = {
    "flying_spaghetti_monster_fight",
}


def is_denied(cue: str) -> bool:
    return cue in DENY_IDS


# Curated display names. Anything not listed is title-cased from its id
# (see ``title_case``). Keep guild/faction cues to their short stage name.
DISPLAY_NAMES = {
    "artists_guild_chiaroscuro": "Chiaroscuro",
    "elves_faction_silverleaf_reverie": "Silverleaf Reverie",
    "env_advocacy_solace": "Env Advocacy — Solace",
    "fighters_guild_oath_of_steel": "Oath of Steel",
    "for_emmy_forever_ago": "For Emmy, Forever Ago",
    "for_emmy_forever_ago_extended": "For Emmy, Forever Ago (Extended)",
    "luddites_guild_loom_and_liberty": "Loom and Liberty",
    "mages_guild_arcane_lanterns": "Arcane Lanterns",
    "mathematicians_guild_proof_by_moonlight": "Proof by Moonlight",
    "ninja_guild_shadow_kata": "Shadow Kata",
    "physicists_guild_event_horizon_waltz": "Event Horizon Waltz",
    "pirates_guild_black_flag_jig": "Black Flag Jig",
    "raid_enforcer_theme": "Black Pennant March",
    "thieves_guild_cobblestone_whisper": "Cobblestone Whisper",
    "crooked_ascent_boss": "Crooked Ascent (Boss)",
    "dinosaur_liberators_long": "Dinosaur Liberators (Long)",
    "fast_paced_violin_boss": "Fast-Paced Violin (Boss)",
    "solo_soar_9m08_loud": "Solo Soar (Loud, 9m)",
    "smirking_behemoth_boss": "You Have To Cut The Rope",
    "standing_on_shoulders": "Standing on Shoulders (GNU-ton)",
    "the_algorithm_knows_youre_lonely": "The Algorithm Knows You're Lonely",
    "series_a_bloodbath": "Series A Bloodbath",
    "flying_spaghetti_monster_stage": "Flying Spaghetti Monster (Stage)",
    "flying_spaghetti_monster_pastafarian_fight": "Flying Spaghetti Monster (Pastafarian, Fight)",
    "flying_spaghetti_monster_pastafarian_stage": "Flying Spaghetti Monster (Pastafarian, Stage)",
    "flying_spaghetti_monster_roots_boss": "Flying Spaghetti Monster (Roots, Boss)",
    "flying_spaghetti_monster_roots_boss_brimstone": "Flying Spaghetti Monster (Roots, Boss — Brimstone)",
    "flying_spaghetti_monster_roots_boss_choir_backing": "Flying Spaghetti Monster (Roots, Boss — Choir)",
    "flying_spaghetti_monster_roots_stage": "Flying Spaghetti Monster (Roots, Stage)",
    "flying_spaghetti_monster_roots_stage_choir_backing": "Flying Spaghetti Monster (Roots, Stage — Choir)",
}

# Lowercased in the middle of a title (never first word).
SMALL_WORDS = {"a", "an", "and", "the", "of", "to", "for", "by", "on", "in", "is"}


def title_case(cue: str) -> str:
    words = cue.split("_")
    out = []
    for i, word in enumerate(words):
        if i != 0 and word in SMALL_WORDS:
            out.append(word)
        else:
            out.append(word[:1].upper() + word[1:])
    return " ".join(out)


def display_name(cue: str) -> str:
    return DISPLAY_NAMES.get(cue, title_case(cue))


def ron_escape(text: str) -> str:
    return text.replace("\\", "\\\\").replace('"', '\\"')


def discover_cues() -> list[str]:
    cues = []
    for child in sorted(GENERATED_DIR.iterdir()):
        if not child.is_dir():
            continue
        if not (child / "full.ogg").is_file():
            continue
        cue = child.name
        if cue in CLAIMED_DIRS or is_denied(cue):
            continue
        cues.append(cue)
    return cues


def render_registry() -> str:
    cues = discover_cues()
    lines = [
        "// Ambition music registry — radio + room music asset pointers.",
        "//",
        "// GENERATED by scripts/regen_music_registry.py (run via regen_music.sh).",
        "// Do NOT hand-edit: re-rendering music overwrites this file. To change",
        "// what ships, edit the generator's DENY_IDS / DISPLAY_NAMES / special",
        "// entries. Each track maps to audio/music/generated/<id>/full.ogg unless",
        "// an explicit asset_path is given.",
        "(",
        f'    default_track: "{ron_escape(DEFAULT_TRACK)}",',
        "    tracks: [",
    ]

    for track_id, name, asset_path in SPECIAL_ENTRIES:
        lines.append("        (")
        lines.append(f'            id: "{ron_escape(track_id)}",')
        lines.append(f'            display_name: "{ron_escape(name)}",')
        lines.append(f'            asset_path: Some("{ron_escape(asset_path)}"),')
        lines.append("        ),")

    for cue in cues:
        lines.append(
            f'        (id: "{ron_escape(cue)}", display_name: "{ron_escape(display_name(cue))}"),'
        )

    lines.append("    ],")
    lines.append(")")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero if the registry is out of date instead of writing it",
    )
    args = parser.parse_args()

    if not GENERATED_DIR.is_dir():
        print(f"error: generated music dir not found: {GENERATED_DIR}", file=sys.stderr)
        return 1

    content = render_registry()
    track_count = content.count("\n            id:") + content.count("        (id:")

    if args.check:
        current = REGISTRY_PATH.read_text(encoding="utf8") if REGISTRY_PATH.exists() else ""
        if current != content:
            print(
                f"music_registry.ron is out of date — run scripts/regen_music_registry.py "
                f"(would register {track_count} tracks)",
                file=sys.stderr,
            )
            return 1
        print(f"music_registry.ron is up to date ({track_count} tracks)")
        return 0

    REGISTRY_PATH.write_text(content, encoding="utf8")
    print(f"wrote {REGISTRY_PATH.relative_to(REPO_ROOT)} ({track_count} tracks)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
