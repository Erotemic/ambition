#!/usr/bin/env python3
"""Regenerate `music_registry.ron` from published music assets.

Each conventional `audio/music/generated/<cue>/full.ogg` becomes a track unless
explicitly denied. Off-convention assets use `SPECIAL_ENTRIES`; track-level
one-shots require explicit score metadata rather than being inferred from
section loopability.

    python3 scripts/regen_music_registry.py
    python3 scripts/regen_music_registry.py --check"""

from __future__ import annotations

import argparse
import os
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
ASSET_ROOTS_SH = REPO_ROOT / "scripts" / "lib" / "asset_roots.sh"


def _declared_asset_crate() -> str:
    """Read the consuming crate name from the shell declaration.

    Parse the declaration directly so shell and Python tooling share one source
    of truth. Missing or malformed declarations raise instead of guessing.
    """
    text = ASSET_ROOTS_SH.read_text()
    match = re.search(
        r'^AMBITION_ASSET_CRATE="\$\{AMBITION_ASSET_CRATE:-([A-Za-z0-9_]+)\}"',
        text,
        re.MULTILINE,
    )
    if not match:
        raise SystemExit(
            f"cannot read AMBITION_ASSET_CRATE from {ASSET_ROOTS_SH}. That file "
            "is where this repo declares which crate's assets/ ships to the "
            "game; this script will not guess it."
        )
    return match.group(1)


# `scripts/regen/music.sh` sources the declaration and exports this, which is the normal
# path. The fallback keeps a bare `python3 scripts/regen_music_registry.py`
# working for somebody poking at it by hand — and it now READS the declaration
# rather than restating it.
GENERATED_DIR = Path(
    os.environ.get("AMBITION_MUSIC_PUBLISH_ROOT")
    or (
        REPO_ROOT
        / "crates"
        / _declared_asset_crate()
        / "assets"
        / "audio"
        / "music"
        / "generated"
    )
)
ACTIVE_SCORES_DIR = (
    REPO_ROOT / "tools" / "ambition_music_renderer" / "scores" / "active"
)
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


def _plain_yaml_scalar(raw: str) -> str:
    """Parse the small top-level scalar subset used for score metadata."""
    value = raw.strip()
    if value[:1] in {"\"", "'"} and value[-1:] == value[:1]:
        value = value[1:-1]
    return value


def discover_one_shot_ids() -> set[str]:
    """Read explicit score-level one-shot declarations without YAML inference."""
    # The scores live in the `ambition_music_renderer` SUBMODULE. An uninitialized
    # submodule is an empty directory, and globbing it would quietly yield zero
    # one-shot ids -- rewriting the committed registry to loop every sting again,
    # with a cheerful "wrote ... (72 tracks)" and no warning. The registry is
    # generated-but-committed, so regenerating it from an incomplete checkout must
    # FAIL rather than produce plausible wrong output.
    scores = sorted(ACTIVE_SCORES_DIR.glob("*.music.yaml"))
    if not scores:
        raise SystemExit(
            f"no scores found under {ACTIVE_SCORES_DIR}.\n"
            "That directory is the `ambition_music_renderer` submodule, and the\n"
            "score-level `one_shot:` flags live only there -- regenerating without\n"
            "them would silently un-mark every one-shot sting as looping.\n"
            "Run: git submodule update --init tools/ambition_music_renderer"
        )
    one_shot_ids: set[str] = set()
    for score_path in scores:
        score_id: str | None = None
        one_shot = False
        for line in score_path.read_text(encoding="utf8").splitlines():
            # Only column-zero keys are score-level. Nested section
            # ``loopable`` values intentionally do not participate.
            if line.startswith("id:"):
                score_id = _plain_yaml_scalar(line.split(":", 1)[1])
            elif line.startswith("one_shot:"):
                value = _plain_yaml_scalar(line.split(":", 1)[1]).lower()
                if value not in {"true", "false"}:
                    raise ValueError(
                        f"{score_path}: one_shot must be true or false, got {value!r}"
                    )
                one_shot = value == "true"
        if one_shot:
            if not score_id:
                raise ValueError(f"{score_path}: one_shot score has no top-level id")
            one_shot_ids.add(score_id)
    return one_shot_ids


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
    one_shot_ids = discover_one_shot_ids()
    missing = sorted(one_shot_ids.difference(cues))
    if missing:
        raise ValueError(
            "one-shot score ids have no generated full.ogg registry entry: "
            + ", ".join(missing)
        )
    lines = [
        "// Ambition music registry — radio + room music asset pointers.",
        "//",
        "// GENERATED by scripts/regen_music_registry.py (run via scripts/regen/music.sh).",
        "// Do NOT hand-edit: re-rendering music overwrites this file. To change",
        "// what ships, edit the generator's DENY_IDS / DISPLAY_NAMES / special",
        "// entries / score-level one_shot metadata. Each track maps to",
        "// audio/music/generated/<id>/full.ogg unless an explicit asset_path is given.",
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
        fields = [
            f'id: "{ron_escape(cue)}"',
            f'display_name: "{ron_escape(display_name(cue))}"',
        ]
        if cue in one_shot_ids:
            fields.append("one_shot: true")
        lines.append(f"        ({', '.join(fields)}),")

    lines.append("    ],")
    lines.append(")")
    return "\n".join(lines) + "\n"


def tracked_ids(text: str) -> set[str]:
    """Every ``id:`` in an existing registry, without parsing RON properly.

    A regex is enough and deliberate: this is a SAFETY check on the file we are
    about to overwrite, so it must work on whatever is on disk — including a
    file some future edit shaped slightly differently.
    """
    return set(re.findall(r'id:\s*"([^"]+)"', text))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero if the registry is out of date instead of writing it",
    )
    parser.add_argument(
        "--allow-removals",
        action="store_true",
        help=(
            "permit the regeneration to DELETE tracks the committed registry has. "
            "Required whenever a cue is genuinely retired, and refused otherwise "
            "so a partial asset tree cannot silently shrink the shipped registry."
        ),
    )
    parser.add_argument(
        "--generated-dir",
        type=Path,
        default=None,
        help=(
            "the published-cue directory to project from. `scripts/regen/music.sh` "
            "passes the same value it hands the renderer, so the two cannot "
            "drift apart; see scripts/lib/asset_roots.sh."
        ),
    )
    args = parser.parse_args()
    if args.generated_dir is not None:
        global GENERATED_DIR
        GENERATED_DIR = args.generated_dir.expanduser().resolve()

    if not GENERATED_DIR.is_dir():
        print(f"error: generated music dir not found: {GENERATED_DIR}", file=sys.stderr)
        return 1

    content = render_registry()
    track_count = content.count("\n            id:") + content.count("        (id:")

    # A REMOVAL GUARD, on both paths.
    #
    # The registry is a projection of `audio/music/generated/<cue>/full.ogg`, and those OGGs are
    # build artifacts nobody commits. So regenerating on a machine that has not rendered every cue
    # does not report a difference of opinion about the roster — it deletes real content, and says
    # "regenerated" while doing it.
    #
    # Adding is still free. This generator's own docstring calls registration an
    # invariant rather than a chore, and it stays one — it is REMOVAL that needs
    # somebody to have meant it.
    current = REGISTRY_PATH.read_text(encoding="utf8") if REGISTRY_PATH.exists() else ""
    removed = sorted(tracked_ids(current) - tracked_ids(content))
    if removed and not args.allow_removals:
        listed = "\n  ".join(removed)
        print(
            f"error: regenerating would DELETE {len(removed)} track(s) from "
            f"{REGISTRY_PATH.relative_to(REPO_ROOT)}:\n  {listed}\n\n"
            "The registry projects the rendered-OGG tree, and those OGGs are build "
            "artifacts that are not committed — so this usually means THIS checkout "
            "has not rendered them, not that they are gone. Render the missing cues "
            "(scripts/scripts/regen/music.sh), or pass --allow-removals if they are really "
            "retired.",
            file=sys.stderr,
        )
        return 1

    if args.check:
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
