#!/usr/bin/env python3
"""Make `mary_o.ldtk`'s entity DEFINITIONS match `mary_o.entities.json`.

⭐ **Safe to re-run, always — this is the half of the authoring workflow that
is.** `author_mary_o_ldtk.py` rebuilds the whole level from mirrored Rust
constants and refuses to run twice, because doing so would discard everything
authored in the editor. Vocabulary still has to be able to change after that
point, so definition sync is a separate script that touches `defs` and never a
level.

GPT 5.6's Mary-O spec asks for exactly this split: *"separate any
schema/definition synchronization from destructive level regeneration."*

    python3 game/ambition_demo_mary_o/tools/sync_mary_o_ldtk_defs.py

⚠ registering a definition is what makes the FIELDS appear in the editor. Without
it an author cannot pick a `kind` at all, which is the difference between
authorable and authorable-by-someone-holding-the-source.

## It used to claim this and not do it

The first version shelled out to `def register-entity`, which refuses an
identifier the project already has, and then matched that refusal's text and
called it success:

    if result.returncode != 0 and tolerate in output:
        print("(already registered — nothing to do)")

So after the very first run, editing the manifest changed nothing — not a
field, not a type, not a default, not the block's size — and the script printed
that it had synchronized. "Already registered" is only "nothing to do" if the
registration is all that was ever wanted, and a sync wants agreement.

It now runs `def upsert-entity`, which reconciles the definitions against the
manifest while preserving the entity's `uid` and each field's `uid` — the
things every placement in the level references — so an existing `MaryOBlock`
whose `kind` is `Quasar` still says `Quasar` afterwards. A run with nothing to
do leaves the file byte-identical.

## What it still refuses

Retiring a field, or changing its type, deletes the values instances carried
for it. When any instance actually holds one, the tool stops, names the field
and the distinct values that would die, and exits non-zero. That is a decision
about Jon's authored level, not a schema edit, and it is made by a human
passing `--drop-instance-values MaryOBlock.<field>` for exactly that run.

⛔ **never commit such a flag into this script.** The opt-in is refused on the
second run — once the values are gone there is nothing left to authorize — so a
standing one turns a re-runnable sync into a red run. That refusal is the
guardrail; do not route around it by widening the script's own flags.

It also never touches a LEVEL: no instances are moved, added or removed, and
the only instance records it deletes are the ones whose definition it was
authorized to remove. `mary_o.ldtk` is authored content — this script exists so
the vocabulary can move without the level being regenerated underneath it.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
TOOLS = REPO / "tools" / "ambition_ldtk_tools"
TARGET = REPO / "game" / "ambition_demo_mary_o" / "assets" / "worlds" / "mary_o.ldtk"
SPEC = TARGET.with_name("mary_o.entities.json")


def run_tool(*args: str) -> int:
    print("::", " ".join(str(a) for a in args))
    return subprocess.run(
        [sys.executable, "-m", "ambition_ldtk_tools", *args],
        cwd=REPO,
        env={"PYTHONPATH": str(TOOLS), "PATH": "/usr/bin:/bin"},
    ).returncode


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="report what would change and write nothing",
    )
    parser.add_argument(
        "--drop-instance-values",
        action="append",
        default=[],
        metavar="MaryOBlock.FIELD",
        help=(
            "Authorize destroying the values the level's instances carry for "
            "this field. For ONE run by a human who has read what dies — never "
            "wired into a script or a CI job."
        ),
    )
    args = parser.parse_args()

    if not TARGET.exists():
        sys.exit(f"{TARGET.relative_to(REPO)} does not exist yet — run author_mary_o_ldtk.py first")

    # `upsert-entity` runs the repair + schema-validate post-pass itself, which
    # is what re-derives the instance-side bookkeeping a definition edit
    # invalidates (`realEditorValues` go stale the moment a default moves). One
    # command, and the file it leaves behind is one LDtk will open.
    argv = [
        "def",
        "upsert-entity",
        str(SPEC),
        "--ldtk",
        str(TARGET),
        "--dry-run" if args.dry_run else "--in-place",
        # Mary-O's noun, not the engine's — see the flag's help.
        "--game-owned",
    ]
    for path in args.drop_instance_values:
        argv.extend(["--drop-instance-values", path])

    rc = run_tool(*argv)
    if rc != 0:
        sys.exit(rc)
    verb = "would sync" if args.dry_run else "synced"
    print(f"{verb} Mary-O's entity definitions into {TARGET.relative_to(REPO)}")


if __name__ == "__main__":
    main()
