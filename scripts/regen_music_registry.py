#!/usr/bin/env python3
"""Build the selected library music registry from authored scores and policy.

Local rendered files do not define registry membership. The result stays in
an ignored generated root.
"""

from __future__ import annotations

import argparse
import os
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
from lib.content_libraries import selected_manifest, generated_root, producer, relative

GENERATED_DIR = Path(os.environ.get("AMBITION_MUSIC_PUBLISH_ROOT") or generated_root(REPO_ROOT) / "audio/music/generated")
ACTIVE_SCORES_DIR = (
    REPO_ROOT / "tools" / "ambition_music_renderer" / "scores" / "active"
)
REGISTRY_PATH = generated_root(REPO_ROOT) / relative(producer(REPO_ROOT, "music")["registry"])
_policy_path = selected_manifest(REPO_ROOT).parent / relative(producer(REPO_ROOT, "music")["policy"])
_policy = json.loads(_policy_path.read_text())

# Track played at startup / when no radio station is selected.
DEFAULT_TRACK = _policy["DEFAULT_TRACK"]

# Off-convention entries: id != directory, or a non-``full.ogg`` source.
# Each is (id, display_name, asset_path). The directories they consume are
# listed in CLAIMED_DIRS so the scan does not also emit a plain entry.
SPECIAL_ENTRIES = _policy["SPECIAL_ENTRIES"]
CLAIMED_DIRS = _policy["CLAIMED_DIRS"]

# Generated dirs that exist but should NOT appear as radio songs.
#   - ``flying_spaghetti_monster_fight``: superseded by the ``roots`` family
#     (the boss now uses ``flying_spaghetti_monster_roots_boss_choir_backing``).
#   - ``dinosaur_liberators_adaptive``: the layers of the primeval trail's
#     adaptive cue (``music_cues.ron``). The radio track is ``dinosaur_liberators``.
# NOTE: ``*_choir_backing`` cues are NOT stems despite the name — they are the
# fuller boss/stage arrangements (boss + choir) and the cues with live scores
# under scores/active, so they ARE registered. Add an id here to retire a cue
# from the radio without deleting its render.
DENY_IDS = _policy["DENY_IDS"]


def is_denied(cue: str) -> bool:
    return cue in DENY_IDS


# Curated display names. Anything not listed is title-cased from its id
# (see ``title_case``). Keep guild/faction cues to their short stage name.
DISPLAY_NAMES = _policy["DISPLAY_NAMES"]

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


def discover_active_scores() -> dict[str, bool]:
    """Read score identities and explicit one-shot declarations."""
    # Missing source scores cannot define a complete registry.
    scores = sorted({*ACTIVE_SCORES_DIR.glob("*.yaml"), *ACTIVE_SCORES_DIR.glob("*.yml")})
    if not scores:
        raise SystemExit(
            f"no scores found under {ACTIVE_SCORES_DIR}.\n"
            "That directory is the `ambition_music_renderer` submodule, and the\n"
            "score-level `one_shot:` flags live only there -- regenerating without\n"
            "them would silently un-mark every one-shot sting as looping.\n"
            "Run: git submodule update --init tools/ambition_music_renderer"
        )
    active_scores: dict[str, bool] = {}
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
        if not score_id:
            raise ValueError(f"{score_path}: score has no top-level id")
        if score_id in active_scores:
            raise ValueError(f"{score_path}: duplicate score id {score_id}")
        active_scores[score_id] = one_shot
    return active_scores


def generation_cues() -> list[str]:
    return sorted({*discover_active_scores(), *_policy["EXTRA_CUES"], *CLAIMED_DIRS})


def discover_cues() -> list[str]:
    return [cue for cue in _policy["EXTRA_CUES"] if cue not in CLAIMED_DIRS and not is_denied(cue)]


def render_registry() -> str:
    active = discover_active_scores()
    cues = sorted({*discover_cues(), *(cue for cue in active if cue not in CLAIMED_DIRS and not is_denied(cue))})
    one_shot_ids = {cue for cue, one_shot in active.items() if one_shot}
    lines = [
        "// Ambition music registry — radio + room music asset pointers.",
        "//",
        "// GENERATED by scripts/regen_music_registry.py (run via scripts/regen/music.sh).",
        "// Do NOT hand-edit: re-rendering music overwrites this file. To change",
        "// The selected library owns track names and selection policy.",
        "// Score metadata supplies the one_shot flag. Each track maps to",
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
    parser.add_argument("--list-cues", action="store_true", help="list selected source cue IDs as JSON")
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero if the registry is out of date instead of writing it",
    )
    parser.add_argument(
        "--generated-dir",
        type=Path,
        default=None,
        help=(
            "the published-cue directory to project from. `scripts/regen/music.sh` "
            "passes the same value it hands the renderer, so the two cannot "
            "drift apart; see the selected content library."
        ),
    )
    args = parser.parse_args()
    if args.list_cues:
        print(json.dumps(generation_cues()))
        return 0
    if args.generated_dir is not None:
        global GENERATED_DIR
        GENERATED_DIR = args.generated_dir.expanduser().resolve()

    content = render_registry()
    track_count = content.count("\n            id:") + content.count("        (id:")

    current = REGISTRY_PATH.read_text(encoding="utf8") if REGISTRY_PATH.exists() else ""
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

    REGISTRY_PATH.parent.mkdir(parents=True, exist_ok=True)
    REGISTRY_PATH.write_text(content, encoding="utf8")
    print(f"wrote {REGISTRY_PATH} ({track_count} tracks)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
