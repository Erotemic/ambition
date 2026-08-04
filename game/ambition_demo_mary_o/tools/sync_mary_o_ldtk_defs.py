#!/usr/bin/env python3
"""Register Mary-O's LDtk entity DEFINITIONS into `mary_o.ldtk`.

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
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
TOOLS = REPO / "tools" / "ambition_ldtk_tools"
TARGET = REPO / "game" / "ambition_demo_mary_o" / "assets" / "worlds" / "mary_o.ldtk"
SPEC = Path(__file__).resolve().parent / "mary_o_entities.json"


def run_tool(*args: str, tolerate: str | None = None) -> None:
    print("::", " ".join(str(a) for a in args))
    result = subprocess.run(
        [sys.executable, "-m", "ambition_ldtk_tools", *args],
        cwd=REPO,
        env={"PYTHONPATH": str(TOOLS), "PATH": "/usr/bin:/bin"},
        capture_output=tolerate is not None,
        text=True,
    )
    if tolerate is not None:
        output = (result.stdout or "") + (result.stderr or "")
        print(output, end="")
        # ⚠ **already-registered is SUCCESS for a sync.** `def register-entity`
        # refuses a duplicate identifier, which is right for a one-shot command
        # and wrong for a script whose whole promise is that re-running it is
        # safe. Idempotence is the property being claimed; it has to be true.
        if result.returncode != 0 and tolerate in output:
            print("(already registered — nothing to do)")
            return
    if result.returncode != 0:
        sys.exit(f"tool step failed: {' '.join(args)}")


def main() -> None:
    if not TARGET.exists():
        sys.exit(f"{TARGET.relative_to(REPO)} does not exist yet — run author_mary_o_ldtk.py first")
    run_tool(
        "def",
        "register-entity",
        str(SPEC),
        "--ldtk",
        str(TARGET),
        "--in-place",
        # Mary-O's noun, not the engine's — see the flag's help.
        "--game-owned",
        tolerate="already exists in the project",
    )
    run_tool("repair", str(TARGET), "--in-place")
    run_tool("validate", str(TARGET))
    print(f"synced Mary-O's entity definitions into {TARGET.relative_to(REPO)}")


if __name__ == "__main__":
    main()
