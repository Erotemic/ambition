"""LDtk project load/write helpers."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any


def load_project(path: Path) -> dict[str, Any]:
    """Load an LDtk JSON project as a mutable dictionary."""
    return json.loads(path.read_text())


def uids_in_use(project: dict[str, Any], text: str) -> set[int]:
    """Every integer this project has already spent from its ``nextUid`` counter.

    ⛔ ``<Identifier>-NNNN`` iids are minted from the SAME counter (see
    ``area_authoring.allocate_iid``), so a suffix in use pins it exactly as a
    ``uid`` field does. Four digits minimum, because shorter runs are ordinary
    numbers inside identifiers rather than minted suffixes.
    """
    found: set[int] = set()

    def walk(node: object) -> None:
        if isinstance(node, dict):
            for key, value in node.items():
                if key == "nextUid":
                    continue
                if isinstance(value, int) and (key == "uid" or key.endswith("Uid")):
                    found.add(value)
                else:
                    walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    walk(project)
    found.update(int(m) for m in re.findall(r'"[A-Za-z_]+-(\d{4,})"', text))
    return found


def warn_if_uid_leaked(path: Path, project: dict[str, Any], text: str) -> None:
    """Say so when a write records uids that were allocated and never used.

    ⛔⛤ **THIS EXISTS BECAUSE FOUR AUTHORED WORLDS KEPT ARRIVING DIRTY WITH A
    ONE-LINE DIFF — `nextUid` UP BY EXACTLY ONE AND NOTHING ELSE CHANGED.** An
    LDtk project counts uids monotonically, and ``alloc_uid``/``allocate_iid``
    bump the counter the moment they are CALLED. A command that allocates and
    then discovers the thing it was building already exists spends a uid on a
    no-op and rewrites an authored file to record it.

    ⇒ **THE POINT IS THAT THE CULPRIT NAMES ITSELF.** `54d99e7fb` fixed the two
    allocation sites that were then known, by reusing existing uids; the churn
    came back from a third, and there are twenty-odd call sites. Auditing them is
    a POPULATION and a population rots when somebody adds a twenty-first — so
    this reports the PROPERTY at the moment it is violated, and whoever ran the
    command learns which one it was without anybody guessing.

    ⚠ **A WARNING AND NOT A REFUSAL, BECAUSE A DELETION LEGITIMATELY RAISES THE
    COUNTER ABOVE THE CONTENTS.** Removing the highest-uid entity is not a leaked
    allocation. `scripts/check_ldtk_uid_leak.py` is the gate that refuses one
    reaching a commit; this is the signal that says who made it.
    """
    counter = int(project.get("nextUid", 1))
    highest = max(uids_in_use(project, text), default=0)
    leak = counter - highest - 1
    if leak > 0:
        print(
            f"⚠ {path}: nextUid={counter} but nothing uses more than {highest} — "
            f"{leak} uid(s) allocated and never written. Whatever command just "
            f"ran allocates BEFORE it checks whether the thing already exists; "
            f"see scripts/check_ldtk_uid_leak.py.",
            file=sys.stderr,
        )


def write_project(path: Path, project: dict[str, Any]) -> None:
    """Write an LDtk project using the editor-friendly Ambition formatting.

    The formatter also refreshes derived editor fields when the full validator is
    available.  Feature commands should prefer this over direct ``json.dump`` so
    no command has to decide when to run the editor normalizer itself.
    """
    # ⛔⛤ **THIS USED TO BE `except Exception:` AROUND THE ONLY WRITER OF
    # AUTHORED CONTENT, AND SUCCESS AND FAILURE HANDED THE CALLER THE SAME
    # THING.** Any error at all — a bug in the normalizer, a formatter raising on
    # one odd field — silently fell through to plain JSON, which REFORMATS AN
    # ENTIRE AUTHORED WORLD. The result is a five-thousand-line diff whose cause
    # is unrecoverable, because nothing recorded that the fallback was taken.
    #
    # ⇒ THE FALLBACK IS KEPT AND NARROWED TO WHAT IT EXISTS FOR: `ImportError`,
    # a tiny test or tool install without the validator. Everything else now
    # RAISES, so a bug in the formatter is a traceback naming itself rather than
    # a reserialized world. And the path that IS taken says so, once, on stderr —
    # the same move as `warn_if_uid_leaked`: make the cause announce itself at
    # the moment it happens instead of leaving it to be inferred from a diff.
    try:
        from ambition_ldtk_tools.editor_format import dump_editor_style
        from ambition_ldtk_tools.validate import normalize_project_for_editor
    except ImportError as missing:
        print(
            f"⚠ {path}: writing PLAIN JSON — this install has no editor "
            f"formatter ({missing}). The whole file is reserialized, so expect a "
            f"diff far larger than the edit.",
            file=sys.stderr,
        )
        write_project_json(path, project)
        return

    normalize_project_for_editor(project)
    path.parent.mkdir(parents=True, exist_ok=True)
    rendered = dump_editor_style(project)
    warn_if_uid_leaked(path, project, rendered)
    path.write_text(rendered)


def write_project_json(path: Path, project: dict[str, Any]) -> None:
    """Write a project with plain JSON formatting.

    This is mainly a fallback / test helper.  Tool commands should use
    :func:`write_project` unless they explicitly want raw JSON.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    rendered = json.dumps(project, indent=2, sort_keys=False) + "\n"
    warn_if_uid_leaked(path, project, rendered)
    path.write_text(rendered)
