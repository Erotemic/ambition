#!/usr/bin/env python3
"""Which authored BOOLEAN parameter modes has no shipped content ever turned on?

⭐ THE QUESTION, and it is not "does this field exist". A technique parameter can
be fully built, guarded, and rollback-registered while every authored customer
leaves it `false` — the behaviour behind it then never runs in the shipped game
and no test notices, because the tests set it themselves. Two such modes turned
up by accident on 2026-09-05 (`TeleportParams::behind_nearest_foe`, and Sing's
engine earlier the same day), which is what this census is for: a catalog that
only lists what EXISTS cannot find them; one that joins the type against
authored content can.

⛔ IT COUNTS THREE STATES, and the middle one is the finding:
  live     — set `true` somewhere in authored content
  dormant  — NAMED in authored content but never set true (the expensive case:
             it looks used, and a grep for the field says "used")
  unnamed  — never mentioned at all

⚠ THE AUTHORED CORPUS INCLUDES RUST. This repo authors content in `.ron`,
`.ldtk`, `.yarn` AND in the content crates' own `.rs`. A first version of this
scan globbed only data files, called `behind_nearest_foe` unauthored, and was
wrong about which half of the tree does the authoring.
⛔ AND `git ls-files 'game/x/src/**/*.rs'` DOES NOT MATCH `src/*.rs` — it matched
only subdirectories and silently dropped every top-level content module, which is
where most authoring lives. The filtering is done in Python for that reason.

⚠ NOT A GATE, and it must not become one: content breadth is not gated in this
repository. The number moves whenever content lands — it moved by one while this
script was being written — so a run is a snapshot and prints its commit.
"""

import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

#: Crates whose `.rs` IS authored content rather than engine code.
CONTENT_CRATES = (
    "game/ambition_content/",
    "game/ambition_demo_smash/",
    "game/ambition_demo_mary_o/",
    "game/ambition_demo_sanic/",
    "game/ambition_demo_pocket/",
    "game/ambition_demo_twintrack/",
)

#: Deserializable types that are NOT authored content: user settings, dev
#: toggles, per-frame input, and budgets/tuning the engine defaults.
NOT_AUTHORED = re.compile(r"Settings|DeveloperTools|ControlFrame|Budget|Tuning|AbilitySet")

STRUCT = re.compile(
    r"#\[derive\([^)]*Deserialize[^)]*\)\]\s*(?:#\[[^\]]*\]\s*)*pub struct (\w+)\s*\{(.*?)\n\}",
    re.S,
)
BOOL_FIELD = re.compile(r"\n\s+pub (\w+):\s*bool\b")


def git(*args: str) -> list[str]:
    return subprocess.run(
        ["git", *args], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout.split()


def authored_corpus() -> tuple[str, int]:
    files = [
        f
        for f in git("ls-files")
        if (f.endswith((".ron", ".ldtk", ".yarn")) and f.startswith("game/"))
        or (f.endswith(".rs") and f.startswith(CONTENT_CRATES))
    ]
    text = "\n".join(
        (REPO / f).read_text(encoding="utf-8", errors="replace") for f in files
    )
    return text, len(files)


def modes() -> list[tuple[str, str, str]]:
    out = []
    for f in git("grep", "-l", "Deserialize", "--", "crates"):
        if "/tests" in f or f.endswith("tests.rs"):
            continue
        text = (REPO / f).read_text(encoding="utf-8", errors="replace")
        for m in STRUCT.finditer(text):
            name, body = m.group(1), m.group(2)
            if NOT_AUTHORED.search(name):
                continue
            for fm in BOOL_FIELD.finditer(body):
                out.append((name, fm.group(1), f))
    return out


def main() -> int:
    corpus, n_files = authored_corpus()
    # ⛔ ANTI-VACUITY: a corpus that missed the content crates would report
    # almost everything as unnamed and read as a catastrophe. This field is
    # authored in Rust, so its absence means the corpus is broken, not the tree.
    if "behind_nearest_foe" not in corpus:
        print(
            "authored corpus does not contain a field known to be authored in Rust "
            "(`behind_nearest_foe`) — the file selection is broken, and every "
            "'unnamed' below would be an artefact.",
            file=sys.stderr,
        )
        return 1

    head = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"], cwd=REPO, capture_output=True, text=True
    ).stdout.strip()
    fields = modes()
    live, dormant, unnamed = [], [], []
    for name, field, where in fields:
        if re.search(re.escape(field) + r"\s*[:=]\s*true\b", corpus):
            live.append((name, field, where))
        elif re.search(r"\b" + re.escape(field) + r"\b", corpus):
            dormant.append((name, field, where))
        else:
            unnamed.append((name, field, where))

    print(f"AUTHORED BOOLEAN PARAMETER MODES  (at {head}, corpus {n_files} files)\n")
    print(f"  technique/placement bool modes      {len(fields)}")
    print(f"  LIVE     — set true in content      {len(live)}")
    print(f"  DORMANT  — named, never set true    {len(dormant)}")
    print(f"  UNNAMED  — never mentioned          {len(unnamed)}")
    for label, rows in (("DORMANT", dormant), ("UNNAMED", unnamed)):
        if not rows:
            continue
        print(f"\n{label}:")
        for name, field, where in sorted(rows):
            print(f"  {name}.{field}\n      {where}")
    print(
        "\nⓘ A dormant mode is not a defect. It is a built behaviour with no "
        "authored customer — decide whether to author one or to delete it."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
