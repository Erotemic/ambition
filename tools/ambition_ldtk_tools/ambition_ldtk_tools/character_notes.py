#!/usr/bin/env python3
"""Carry a sprite target's authoring notes through to the character catalog.

A character now arrives from `tools/ambition_sprite2d_renderer` with more than
art: its target module declares an ``ACTOR_METADATA`` dict holding who it
parodies, how it should fight, and lines it might say. None of that reached the
game -- the catalog row was hand-copied, so the prose and the suggested barks
stayed in the renderer and the character shipped mute.

This module is the join. It normalizes ``ACTOR_METADATA`` into the three fields
`CharacterCatalogEntry` carries (`authoring_description`, `gameplay_description`,
`fallback_dialogue`) and reports what a target failed to supply.

# # Why normalization is the whole job

The targets do not agree on a shape, because each was authored separately:

  - ``authoring_description`` is a prose ``str`` in one target and a ``dict`` of
    ``parody_of`` / ``core_joke`` / ``design_notes`` in the next, with the key
    set varying between dict-shaped ones;
  - ``gameplay_description`` is a dict of ``role`` / ``signature_moves`` /
    ... , or absent entirely;
  - dialogue lives under ``dialogue_hints``, whose own keys vary
    (``suggested_barks``, ``barks``, ``fallback_dialogue``, ``fallback_lines``),
    or is missing.

Rejecting the variation would mean rewriting every target; silently accepting it
would mean a character whose notes quietly flatten to `""`. So this flattens
deterministically and **names every gap** (:func:`missing_fields`) so an empty
description is a reported fact rather than an invisible one.

# # What the suggested barks become

EVERY key under ``dialogue_hints`` folds into the catalog's single
``fallback_dialogue`` pool, barks first — not a fixed list of spellings, because
a fixed list dropped six authored lines from two characters without saying so.
That pool is what
`CharacterCatalogEntry::bark` reaches for when a situation has no authored pool,
so a freshly generated character speaks in its own voice on hit, when provoked,
idling, and on its Hall pedestal without anyone writing four pools by hand.
Short barks lead so rotation 0 -- the line a player hears first -- is punchy.
Promoting a line into a real situation pool later silences the fallback for that
situation only.

# # Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools:tools/ambition_sprite2d_renderer \\
python -m ambition_ldtk_tools.character_notes --target marie_curry
```

Prints a RON fragment for manual splice into `character_catalog.ron`, the same
posture as `codegen_character_catalog` -- a hand-curated catalog is not
machine-rewritten, and a human decides where the row lands.
"""

from __future__ import annotations

import argparse
import importlib
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence

DEFAULT_CATALOG = "game/ambition_content/assets/data/character_catalog.ron"
# An existing, stable id keeps the insert deterministic without needing to parse RON.
SPLICE_ANCHOR = '        "npc_carl_stargan": ('
# The safest posture for a character that has art but no authored kit yet.
DEFAULT_BRAIN = "patrol_peaceful"
DEFAULT_ACTION_SET = "striker_swipe"

# Dict keys that carry a single prose sentence, in the order they should read
# once flattened. Anything not listed is appended after these, alphabetically,
# so a target that invents a key still contributes its text instead of losing it.
_PROSE_KEY_ORDER = (
    "parody_of",
    "core_joke",
    "concept",
    "name_origin",
    "role",
    "combat_identity",
    "signature_moves",
    "visual_inspiration",
    "visual_inspirations",
    "gameplay_inspiration",
    "reference_hooks",
    "design_notes",
    "authoring_notes",
    "boundaries",
)

# Keys whose values are context for the reader rather than a claim about the
# character, prefixed so the flattened prose stays readable.
_PROSE_KEY_LABELS = {
    "parody_of": "Parodies",
    "name_origin": "Name origin",
    "visual_inspiration": "Visual inspiration",
    "visual_inspirations": "Visual inspiration",
    "gameplay_inspiration": "Gameplay inspiration",
    "reference_hooks": "Reference hooks",
    "design_notes": "Design notes",
    "authoring_notes": "Authoring notes",
    "boundaries": "Boundaries",
    "role": "Role",
    "combat_identity": "Combat identity",
    "signature_moves": "Signature moves",
}


@dataclass(frozen=True)
class CharacterNotes:
    """The catalog-facing projection of one target's ``ACTOR_METADATA``."""

    character_id: str
    display_name: str
    authoring_description: str = ""
    gameplay_description: str = ""
    fallback_dialogue: tuple[str, ...] = ()
    traits: tuple[str, ...] = ()

    def missing_fields(self) -> tuple[str, ...]:
        """Which catalog-facing fields this target supplied nothing for.

        Reported rather than defaulted: a character with no suggested lines is
        one that will fall through to the engine-generic bark, and that is worth
        seeing at generation time instead of discovering in a playtest.
        """
        gaps = []
        if not self.authoring_description:
            gaps.append("authoring_description")
        if not self.gameplay_description:
            gaps.append("gameplay_description")
        if not self.fallback_dialogue:
            gaps.append("fallback_dialogue")
        return tuple(gaps)


def _clean_lines(value: Any) -> list[str]:
    """Every non-empty string in `value`, whether it is one or a sequence."""
    if isinstance(value, str):
        text = value.strip()
        return [text] if text else []
    if isinstance(value, Sequence):
        out = []
        for item in value:
            out.extend(_clean_lines(item))
        return out
    return []


def _sentence(text: str) -> str:
    """Terminate a fragment so flattened prose does not run sentences together."""
    text = text.strip()
    if text and text[-1] not in ".!?":
        return text + "."
    return text


def flatten_prose(value: Any) -> str:
    """Flatten a prose field that may be a string, a dict, or missing.

    Dict-shaped values are emitted in :data:`_PROSE_KEY_ORDER` first, then any
    remaining keys alphabetically, so two runs over the same target produce the
    same text and an unrecognized key is still carried rather than dropped.
    """
    if value is None:
        return ""
    if isinstance(value, str):
        return _sentence(value)
    if not isinstance(value, Mapping):
        return _sentence(" ".join(_clean_lines(value)))

    ordered = [k for k in _PROSE_KEY_ORDER if k in value]
    ordered += sorted(k for k in value if k not in _PROSE_KEY_ORDER)

    parts: list[str] = []
    for key in ordered:
        lines = _clean_lines(value[key])
        if not lines:
            continue
        body = " ".join(_sentence(line) for line in lines)
        label = _PROSE_KEY_LABELS.get(key)
        parts.append(f"{label}: {body}" if label else body)
    return " ".join(parts).strip()


def fallback_pool(dialogue_hints: Any) -> tuple[str, ...]:
    """The catalog's fallback line pool: suggested barks first, then longer
    fallback dialogue, de-duplicated while preserving order."""
    if not isinstance(dialogue_hints, Mapping):
        return ()
    lines: list[str] = []
    # `barks` is the key `publish_character_notes` writes into sheet manifests;
    # `suggested_barks` is what the targets actually author. Accept both rather
    # than making a character mute over a key name.
    #
    # so the rule is now the CAPABILITY, not the name: everything under
    # `dialogue_hints` is dialogue by construction, so every key is read. The
    # known spellings lead, in the authored precedence (short barks before
    # longer fallback prose); anything new follows in sorted order, which keeps
    # the pool deterministic without anyone having to come back here.
    known = ("suggested_barks", "barks", "fallback_dialogue", "fallback_lines")
    for key in known:
        lines.extend(_clean_lines(dialogue_hints.get(key)))
    for key in sorted(k for k in dialogue_hints if k not in known):
        lines.extend(_clean_lines(dialogue_hints.get(key)))
    seen: set[str] = set()
    unique = []
    for line in lines:
        if line not in seen:
            seen.add(line)
            unique.append(line)
    return tuple(unique)


def notes_from_actor_metadata(metadata: Mapping[str, Any]) -> CharacterNotes:
    """Project one target's ``ACTOR_METADATA`` onto the catalog's fields."""
    actor = metadata.get("actor") or {}
    body = metadata.get("body") or {}
    return CharacterNotes(
        character_id=str(actor.get("character_id", "")).strip(),
        display_name=str(actor.get("display_name", "")).strip(),
        authoring_description=flatten_prose(metadata.get("authoring_description")),
        gameplay_description=flatten_prose(metadata.get("gameplay_description")),
        fallback_dialogue=fallback_pool(metadata.get("dialogue_hints")),
        traits=tuple(_clean_lines(body.get("traits"))),
    )


def load_target_metadata(target: str) -> Mapping[str, Any]:
    """Import a renderer character target and hand back its ``ACTOR_METADATA``.

    Importing beats parsing: the dicts reference module-level constants, so
    `ast.literal_eval` cannot read them.
    """
    module = importlib.import_module(
        f"ambition_sprite2d_renderer.targets.characters.{target}"
    )
    metadata = getattr(module, "ACTOR_METADATA", None)
    if not isinstance(metadata, Mapping):
        raise KeyError(f"target '{target}' declares no ACTOR_METADATA")
    return metadata


def _ron_string(text: str) -> str:
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def render_notes_ron(notes: CharacterNotes, indent: str = " " * 12) -> str:
    """The catalog fields for one character, ready to splice into its row."""
    lines = []
    if notes.authoring_description:
        lines.append(
            f"{indent}authoring_description: {_ron_string(notes.authoring_description)},"
        )
    if notes.gameplay_description:
        lines.append(
            f"{indent}gameplay_description: {_ron_string(notes.gameplay_description)},"
        )
    if notes.fallback_dialogue:
        lines.append(f"{indent}fallback_dialogue: [")
        for line in notes.fallback_dialogue:
            lines.append(f"{indent}    {_ron_string(line)},")
        lines.append(f"{indent}],")
    return "\n".join(lines)


def render_catalog_row(
    notes: CharacterNotes,
    target: str,
    brain: str = DEFAULT_BRAIN,
    action_set: str = DEFAULT_ACTION_SET,
) -> str:
    """A COMPLETE catalog row for one character, notes included.

    The brain/action-set defaults are a starting posture, not a claim about the
    character: a peaceful patroller with the generic swipe kit is the safest
    thing to put on a Hall pedestal, and retuning one row afterwards is a
    one-line edit. Getting the character INTO the game is the hard part.
    """
    tags = ", ".join(_ron_string(t) for t in notes.traits)
    row = [
        f'        "{notes.character_id}": (',
        f"            display_name: {_ron_string(notes.display_name)},",
        f'            spritesheet: "sprites/{target}_spritesheet.png",',
        f'            manifest: "sprites/{target}_spritesheet.ron",',
        "            tier: MainHall,",
        "            body_kind: Standard,",
        "            composition: None,",
        f"            default_brain: {_ron_string(brain)},",
        f"            default_action_set: {_ron_string(action_set)},",
        f"            tags: [{tags}],",
    ]
    body = render_notes_ron(notes)
    if body:
        row.append(body)
    row.append("        ),")
    return "\n".join(row) + "\n"


def discover_targets() -> list[str]:
    """Every renderer character target module name, sorted.

    Discovered from the package directory rather than a hand-kept list, so a
    character that was just dropped in is visible to this tool immediately —
    the failure this whole join exists to prevent is a character that exists in
    one place and is invisible in the next.
    """
    package = importlib.import_module("ambition_sprite2d_renderer.targets.characters")
    roots = [Path(p) for p in getattr(package, "__path__", [])]
    names = {
        path.stem
        for root in roots
        for path in root.glob("*.py")
        if not path.stem.startswith("_")
    }
    return sorted(names)


def splice_rows(catalog_path: Path, rows: Mapping[str, str]) -> list[str]:
    """Insert each row whose character_id is absent, before `SPLICE_ANCHOR`.

    Idempotent by character id: re-running adds nothing and never rewrites a row
    a human has since edited. That matters more than it sounds — this catalog is
    hand-curated and gets restored from snapshots, so a tool that rewrites rows
    it did not author would quietly undo real authoring.
    """
    text = catalog_path.read_text()
    pending = [row for cid, row in rows.items() if f'"{cid}": (' not in text]
    if not pending:
        return []
    if text.count(SPLICE_ANCHOR) != 1:
        raise ValueError(
            f"cannot locate a unique splice anchor {SPLICE_ANCHOR!r} in {catalog_path}"
        )
    catalog_path.write_text(text.replace(SPLICE_ANCHOR, "".join(pending) + SPLICE_ANCHOR, 1))
    return [cid for cid, row in rows.items() if row in pending]


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument(
        "--target",
        action="append",
        help="renderer character target module (repeatable). Omit to scan them all.",
    )
    parser.add_argument(
        "--splice",
        action="store_true",
        help="write missing rows into the character catalog instead of printing them",
    )
    parser.add_argument(
        "--catalog",
        type=Path,
        default=Path(DEFAULT_CATALOG),
        help=f"catalog path (default {DEFAULT_CATALOG})",
    )
    parser.add_argument("--brain", default=DEFAULT_BRAIN)
    parser.add_argument("--action-set", default=DEFAULT_ACTION_SET)
    args = parser.parse_args(list(argv) if argv is not None else None)

    targets = args.target or discover_targets()
    status = 0
    rows: dict[str, str] = {}
    for target in targets:
        try:
            notes = notes_from_actor_metadata(load_target_metadata(target))
        except Exception as exc:  # a target may be art-only, or fail to import
            if args.target:  # explicitly asked for -> a real error
                print(f"# {target}: {exc}", file=sys.stderr)
                status = 1
            continue
        if not notes.character_id:
            if args.target:
                print(f"# {target}: ACTOR_METADATA names no character_id", file=sys.stderr)
                status = 1
            continue
        rows[notes.character_id] = render_catalog_row(
            notes, target, args.brain, args.action_set
        )
        gaps = notes.missing_fields()
        if gaps:
            print(f"# {target}: no {', '.join(gaps)}", file=sys.stderr)

    if not args.splice:
        for row in rows.values():
            print(row, end="")
        return status

    # `--splice` writes only what was ASKED for. A blanket scan over the whole
    # renderer sweeps up things that are targets but not characters -- variant
    # rigs (`npc_pirate_heavy`, whose real rows are its three named variants),
    # candidate art, and `*_v2` experiments -- and the catalog deliberately has
    # no row for those. Naming the target is the author saying "this one is a
    # character," which is a judgement no directory listing can make.
    if not args.target:
        print(
            "refusing to splice a whole-renderer scan: name the targets you mean\n"
            "  (the listing above is a report; not every render target is a character)",
            file=sys.stderr,
        )
        for row in rows.values():
            print(row, end="")
        return 1

    added = splice_rows(args.catalog, rows)
    if added:
        print(f"added {len(added)} row(s) to {args.catalog}: {', '.join(added)}")
        print("next: regenerate the hall so they get pedestals ->")
        print(
            "  PYTHONPATH=tools/ambition_ldtk_tools "
            "python3 -m ambition_ldtk_tools.generate_hall_of_characters"
        )
    else:
        print(f"{args.catalog}: every discovered character already has a row")
    return status


if __name__ == "__main__":  # pragma: no cover - CLI entry
    raise SystemExit(main())
