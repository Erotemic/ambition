#!/usr/bin/env python3
"""The music renderer THE SUPERPROJECT PINS must refuse the General-MIDI fallback.

⛔⛔ THIS READS THE PINNED TREE, NOT THE CHECKED-OUT ONE, AND THAT IS THE WHOLE
POINT. `tools/ambition_music_renderer` is a submodule. A developer's working copy
can sit on any branch, so every ordinary check — running its test suite, grepping
its files, importing it — describes the branch that happens to be checked out and
says nothing about what a fresh clone gets.

The failure this exists for, 2026-09-04: a planning entry concluded the standing
"no General-MIDI stand-ins on new machines" requirement was guarded, and reasoned
correctly to get there. `scripts/regen/music.sh` does warn-and-continue, it does
defer to the renderer refusing, and the renderer does refuse — on a branch. The
pinned commit's `cli.py` contained ONE match for the concept and it was a
comment. Every link true, the chain false, because "the renderer" named two
different commits. See
`dev/benchmark-candidates/every-link-true-and-the-chain-false-2026-09-04.md`.

⚠ REPORTS, DOES NOT GATE, and that is temporary rather than a design choice. The
pin does not satisfy the requirement today; making this exit non-zero would paint
CI red for a condition only a maintainer can clear (it needs a submodule
fast-forward and a pointer bump — see
`docs/planning/yardrat-open-measurements.md`). ⇒ **Flip `GATES` to `True` in the
same commit that lands the pointer**, and this becomes the guard that stops the
requirement silently regressing again.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SUBMODULE = "tools/ambition_music_renderer"

#: Flip to True with the pointer bump. See the module docstring.
GATES = False

#: What a refusing renderer must contain. Deliberately the ENV OVERRIDE and not
#: the words "General MIDI": the prose appears in comments on both sides, and a
#: comment is exactly what the pinned tree turned out to have. The override name
#: only exists where the refusal is implemented, because it is the escape hatch
#: the refusal offers.
REQUIRED_TOKEN = "AMBITION_MUSIC_ALLOW_GM_FALLBACK"
REQUIRED_IN = "ambition_music_renderer/cli.py"


def pinned_sha() -> str | None:
    """The commit the SUPERPROJECT records, not the one checked out."""
    out = subprocess.run(
        ["git", "ls-tree", "HEAD", SUBMODULE],
        cwd=ROOT, capture_output=True, text=True,
    ).stdout.split()
    # `160000 commit <sha>\t<path>`
    return out[2] if len(out) >= 3 else None


def file_at(sha: str, path: str) -> str | None:
    proc = subprocess.run(
        ["git", "show", f"{sha}:{path}"],
        cwd=ROOT / SUBMODULE, capture_output=True, text=True,
    )
    return proc.stdout if proc.returncode == 0 else None


def refusal_present(sha: str) -> tuple[bool, str]:
    """Does the tree at `sha` implement the refusal? Returns (yes, why)."""
    body = file_at(sha, REQUIRED_IN)
    if body is None:
        return False, f"{REQUIRED_IN} does not exist at {sha[:9]}"
    hits = body.count(REQUIRED_TOKEN)
    if hits == 0:
        return False, (
            f"{REQUIRED_IN} at {sha[:9]} never names {REQUIRED_TOKEN}, so it has "
            "no refusal to override — it will render General-MIDI stand-ins and "
            "report success"
        )
    return True, f"{REQUIRED_IN} at {sha[:9]} names {REQUIRED_TOKEN} {hits}x"


def main() -> int:
    sha = pinned_sha()
    if sha is None:
        print(f"⛔ could not read the pinned commit for {SUBMODULE}")
        return 1
    ok, why = refusal_present(sha)
    if ok:
        print(f"OK: the PINNED music renderer refuses the GM fallback — {why}")
        return 0
    print(f"⛔ THE PINNED MUSIC RENDERER DOES NOT REFUSE THE GM FALLBACK: {why}")
    print(
        "   A fresh clone renders General-MIDI stand-ins and reports success.\n"
        "   The refusal exists on `agent/sfizz-source-fallback-and-cue-fanout`,\n"
        "   which fast-forwards onto the submodule's main. The fix is that\n"
        "   fast-forward plus a pointer bump — NOT re-pointing at the branch\n"
        "   commit, which is deletable. See docs/planning/"
        "yardrat-open-measurements.md."
    )
    return 1 if GATES else 0


if __name__ == "__main__":
    sys.exit(main())
