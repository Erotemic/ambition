#!/usr/bin/env python3
"""Which fields of the mechanical registries does a SIMULATION system read?

`Q122` asks which fields of `PreparedCharacterOverrides`, `SheetRecord` and
`BossCatalog` a rollback timeline's identity should bind, and the architecture
review's instruction is to answer it **from CONSUMERS, not from taste**. This is
that measurement.

⛔⛔⛔ **RUN IT AND READ THE VERDICT FIRST: THE `.field` QUERY THIS SCRIPT USES
CANNOT ANSWER `Q122`, AND THE SCRIPT EXISTS TO PROVE THAT RATHER THAN TO ANSWER
IT.** MEASURED 2026-09-13: **50 of 50 fields classify as MECHANICAL and ZERO as
presentation** — including `portrait`, `voice`, `ranged_vfx`, `dream_seed` and
`sprite_filenames`, which are the five `Q122` itself names as presentation by
their own doc comments.

⇒ **THE CAUSE IS THAT A FIELD NAME IS NOT AN IDENTITY.** `git grep -F ".portrait"`
matches every struct in the workspace with a field spelled that way, so `.id`,
`.body`, `.sheet`, `.key`, `.target`, `.rows` and `.image` each collect the whole
tree. The query answers *"does any crate use this WORD"*, which is not the
question, and it answers it with a table that looks complete.

⭐⭐ **THE CONTROL IS WHAT MAKES THIS A RESULT INSTEAD OF A WRONG ANSWER.** The
five fields above are printed as a REFUTATION section at the end of every run:
their classification is known in advance, the instrument disagrees with all five,
and a version of this script that ever agrees with them is the one that can be
trusted. Without that arm the table above reads as a finished census.

⇒ **WHAT WOULD ACTUALLY ANSWER IT — two roads, neither a grep:**
1. **rust-analyzer references** on each field DECLARATION (`textDocument/references`),
   which is type-aware and returns the real reader set. The workspace already has
   a rust-analyzer MCP surface.
2. **SEAL THE FIELD AND READ THE COMPILER.** Make it private, or rename it, and
   every genuine reader becomes an error with a file and a line. This repository
   has used that method before and it does not depend on a spelling.

⚠ And note the subject may not be these structs at all: `PreparedCharacterOverrides`
is already private to its module, so its fields have almost no DIRECT readers —
the values flow into `PreparedCharacter` and `CharacterDefinition`, and those are
what a reader census has to be about. Establishing that is the first step of
whoever picks `Q122` up.

## The classification below, and why its direction is the safe one

⛔⛤ **A FIELD IS MECHANICAL IF ANY SIMULATION SYSTEM READS IT.** Not if it
*sounds* mechanical, and not if its doc comment says "presentation" — a doc
comment is a claim about intent and the reader list is a fact about the build.
`Q122`'s own row names the trap: *"do not fix it by hand-listing exclusions —
that is a population that rots, and a new presentation field added later would be
included silently."* A hand-list is taste; this is the derivation.

⛔⛔ **A FIELD WITH NO READERS IS `UNRESOLVED`, NEVER `presentation`.** A negative
grep is a claim about the QUERY, not about the code: a field consumed through
destructuring, a `..` rest pattern, `serde` round-tripping, or a rename is
invisible to a `.field` search. Reporting those as "presentation" would quietly
license dropping them from the identity, which is the UNDER-sensitive direction —
the one that restores a snapshot into the wrong world. They are printed in their
own section so the count of things this instrument cannot see is visible rather
than folded into an answer.

⚠ **UNKNOWN CRATE ⇒ MECHANICAL.** The classification below lists PRESENTATION
crates explicitly and treats everything else as simulation. A new crate therefore
defaults to the over-sensitive direction, which `Q122` measures as the safe one:
*"over-sensitivity refuses a legitimate restore; under-sensitivity restores a
snapshot into the wrong world."*

    python3 scripts/measure_identity_field_consumers.py
    python3 scripts/measure_identity_field_consumers.py --json out.json
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# ⭐ THE SUBJECTS ARE NAMED WITH THEIR FILE, so a struct that moves is a LOUD
# failure ("no fields parsed") rather than a silently empty census.
SUBJECTS = [
    ("PreparedCharacterOverrides", "crates/ambition_characters/src/prepared.rs"),
    ("SheetRecord", None),
    ("BossCatalog", None),
]

# ⛔ PRESENTATION CRATES, LISTED; EVERYTHING ELSE IS SIMULATION. The asymmetry is
# deliberate — see the module doc. Matched as a whole path segment so
# `ambition_render` does not also claim `ambition_render_probe`.
PRESENTATION_CRATES = {
    "ambition_render",
    "ambition_sim_view",
    "ambition_vfx",
    "ambition_sprite_fx",
    "ambition_sprite_sheet",
    "ambition_character_sprites",
    "ambition_menu",
    "ambition_menu_kaleidoscope",
    "ambition_inventory_ui",
    "ambition_load_presentation",
    "ambition_portal2d_presentation",
    "ambition_audio",
    "ambition_sfx",
    "ambition_sfx_bank",
    "ambition_dialog",
    "ambition_touch_input",
}


def struct_fields(name: str, hint: str | None) -> tuple[list[str], str]:
    """Every field of `name`, and the file it was read from.

    ⚠ THE BRACE DEPTH IS TRACKED, not assumed: a field whose type is itself a
    braced literal would otherwise end the struct early and truncate the list —
    the same shape as a census that reports a narrower population and reads as
    healthy.
    """
    if hint:
        candidates = [ROOT / hint]
    else:
        found = subprocess.run(
            ["git", "grep", "-l", "-E", rf"(pub )?struct {name}\b"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        ).stdout.split()
        candidates = [ROOT / p for p in found if p.endswith(".rs")]
    for path in candidates:
        if not path.exists():
            continue
        text = path.read_text()
        match = re.search(rf"^\s*(?:pub(?:\([^)]*\))? )?struct {name}\b[^{{;]*{{", text, re.M)
        if not match:
            continue
        depth = 0
        fields: list[str] = []
        for line in text[match.end() - 1 :].splitlines():
            depth += line.count("{") - line.count("}")
            if depth <= 0 and fields:
                break
            field = re.match(r"\s*(?:pub(?:\([^)]*\))? )?([a-z_][a-z0-9_]*)\s*:", line)
            if field and depth == 1:
                fields.append(field.group(1))
            if depth <= 0:
                break
        if fields:
            return fields, str(path.relative_to(ROOT))
    return [], ""


def crate_of(path: str) -> str:
    parts = Path(path).parts
    for i, part in enumerate(parts):
        if part in ("crates", "game", "tools", "dev") and i + 1 < len(parts):
            return parts[i + 1]
    return parts[0] if parts else "?"


def readers(field: str) -> dict[str, set[str]]:
    """Files that READ `.field`, keyed by crate.

    ⛔ PRODUCTION ONLY IS NOT ATTEMPTED HERE and the reason is stated rather than
    hidden: stripping `#[cfg(test)]` regions needs a parser, and a truncating
    approximation is how this repository lost a whole region of one file to a
    guard before. Test files are EXCLUDED BY PATH, which is honest about what it
    catches, and inline `mod tests` hits are visible in the per-file list.
    """
    out: dict[str, set[str]] = defaultdict(set)
    result = subprocess.run(
        ["git", "grep", "-l", "-F", f".{field}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    for path in result.stdout.split():
        if not path.endswith(".rs"):
            continue
        if "/tests/" in path or path.endswith("tests.rs") or path.endswith("_tests.rs"):
            continue
        out[crate_of(path)].add(path)
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", type=Path, help="also write the rows here")
    args = parser.parse_args()

    report: dict[str, object] = {}
    total_fields = 0
    for name, hint in SUBJECTS:
        fields, source = struct_fields(name, hint)
        # ⛔ AN EMPTY FIELD LIST IS A BROKEN INSTRUMENT, NOT AN EMPTY STRUCT.
        if not fields:
            print(f"⛔ {name}: no fields parsed — the struct moved or was renamed.")
            print("   This is an INSTRUMENT failure; do not read the sections below as complete.")
            report[name] = {"error": "no fields parsed"}
            continue
        total_fields += len(fields)
        print(f"\n=== {name}  ({len(fields)} fields, from {source})")
        mechanical: list[tuple[str, list[str]]] = []
        presentation: list[tuple[str, list[str]]] = []
        unresolved: list[str] = []
        for field in fields:
            by_crate = readers(field)
            if not by_crate:
                unresolved.append(field)
                continue
            sim = sorted(c for c in by_crate if c not in PRESENTATION_CRATES)
            if sim:
                mechanical.append((field, sim))
            else:
                presentation.append((field, sorted(by_crate)))

        print(f"\n  MECHANICAL — a simulation crate reads it ({len(mechanical)}):")
        for field, crates in mechanical:
            shown = ", ".join(crates[:4]) + (" …" if len(crates) > 4 else "")
            print(f"    {field:<28} {len(crates):>2} sim crate(s): {shown}")
        print(f"\n  PRESENTATION — only presentation crates read it ({len(presentation)}):")
        for field, crates in presentation:
            print(f"    {field:<28} {', '.join(crates)}")
        print(f"\n  ⛔ UNRESOLVED — this query found NO reader ({len(unresolved)}):")
        print("     A negative grep is a claim about the QUERY. Destructuring, a `..`")
        print("     rest pattern, a serde rename or a re-export are all invisible to it.")
        print("     These are NOT presentation; they are unanswered.")
        for field in unresolved:
            print(f"    {field}")
        report[name] = {
            "source": source,
            "mechanical": {f: c for f, c in mechanical},
            "presentation": {f: c for f, c in presentation},
            "unresolved": unresolved,
        }

    # ⛔⛔⛔ THE REFUTATION ARM. `Q122` names these five as presentation by their
    # own doc comments. A `.field` grep calls every one of them mechanical,
    # because a field NAME is not an identity — `.portrait` matches every struct
    # in the workspace spelled that way. Printed on every run so the table above
    # can never be quoted as a finished census.
    print("\n" + "=" * 72)
    print("⛔⛔⛔ CONTROL — the five fields `Q122` itself calls PRESENTATION:")
    known_presentation = ["portrait", "voice", "ranged_vfx", "dream_seed", "sprite_filenames"]
    disagreed = []
    for field in known_presentation:
        by_crate = readers(field)
        sim = sorted(c for c in by_crate if c not in PRESENTATION_CRATES)
        verdict = "MECHANICAL" if sim else ("presentation" if by_crate else "UNRESOLVED")
        if verdict == "MECHANICAL":
            disagreed.append(field)
        print(f"    {field:<20} this instrument says: {verdict}")
    if disagreed:
        print(
            f"\n⇒ THE INSTRUMENT DISAGREES WITH {len(disagreed)} OF "
            f"{len(known_presentation)}, SO THE TABLE ABOVE IS NOT AN ANSWER.\n"
            "  A field NAME is not an identity: `.portrait` matches every struct in the\n"
            "  workspace spelled that way. `Q122` needs a TYPE-AWARE reader set —\n"
            "  rust-analyzer references on each field declaration, or sealing the field\n"
            "  and reading the compiler errors. Do not hand-list from this output."
        )
    else:
        print(
            "\n⭐ The instrument agrees with all five, which is the first time it has.\n"
            "  Re-read the classification above; it may now be worth acting on."
        )
    print("=" * 72)

    print(f"\n{total_fields} field(s) examined across {len(SUBJECTS)} subject(s).")
    print(
        "⚠ UNKNOWN CRATE ⇒ MECHANICAL by construction, which is the over-sensitive\n"
        "  direction. `Q122` measures that as the safe one: over-sensitivity refuses a\n"
        "  legitimate restore, under-sensitivity restores a snapshot into the wrong world."
    )
    if args.json:
        args.json.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
        print(f"wrote {args.json}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
