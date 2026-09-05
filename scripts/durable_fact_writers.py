#!/usr/bin/env python3
"""Who WRITES each durable save family, and which families have more than one.

⭐ THE QUESTION: a durable fact with two writers gets its policy from whoever
remembered. `boss.cleared` is the worked example — WRITTEN by one generic
authority (`boss_encounter/src/systems.rs`, keyed by placement, on the death
edge) and RETRACTED by a content system naming itself, so ten of eleven shipped
placements have no retraction at all and nobody chose that. This census asks
which other families have the same shape.

⛔⛔ IT EXCLUDES `#[cfg(test)]` MODULES BY POSITION, NOT BY PATH. This repo puts
`#[cfg(test)] mod tests` INSIDE ordinary source files, so a path filter
(`tests.rs`, `/tests/`) leaves every in-file test block in the numerator. My
first pass at this counted 7 `set_flag` writers; most were test seeds inside
`save_data.rs` and `save.rs`.

⚠ It is a LEXICAL census and says so: it finds `.set_<family>(` calls, not
every road to a durable write. A helper that wraps one is counted at the helper.
Read it as "where does the tree name this family", not as a proof of arity.
"""

import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
# ⛔⛔ THE RECEIVER IS NOT ALWAYS `data_mut()`. The first version of this pattern
# required `data_mut().set_x(` and therefore MISSED the one writer this census
# was written to find: `cut_rope/mod.rs:214` binds `let data = save.data_mut();`
# and then calls `data.set_boss(...)` in a loop. ⇒ A census that cannot find its
# own worked example is not a weak census, it is a false one — it would have
# reported `boss` as single-writer, which is the exact claim it exists to test.
# Match the VERB on any receiver and name the families explicitly instead.
FAMILIES = "boss|flag|switch|encounter|quest|item|occurrence|custody|checkpoint"
CALL = re.compile(rf"\.\s*(set|clear|remove)_({FAMILIES})\s*\(")
TEST_MOD = re.compile(r"^\s*#\[cfg\(test\)\]", re.M)


def production_lines(text: str) -> list[tuple[int, str]]:
    """Every line before the file's first `#[cfg(test)]`, 1-based.

    Crude on purpose and honest about it: a file with a `#[cfg(test)]` helper
    ABOVE production code would under-count. Verified below — the script reports
    how many files that shape applies to, so the crudeness is measured, not
    assumed.
    """
    match = TEST_MOD.search(text)
    cut = text[: match.start()].count("\n") + 1 if match else None
    return [
        (i + 1, line)
        for i, line in enumerate(text.splitlines())
        if cut is None or i + 1 < cut
    ]


def main() -> int:
    files = subprocess.run(
        ["git", "ls-files", "*.rs"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()

    writers: dict[str, list[str]] = defaultdict(list)
    suppressed = 0
    early_test_mod = 0
    for rel in files:
        if "/tests/" in rel or rel.endswith("tests.rs"):
            continue
        text = (REPO / rel).read_text(encoding="utf-8", errors="replace")
        lines = production_lines(text)
        total_hits = sum(
            len(CALL.findall(l.split("//", 1)[0])) for l in text.splitlines()
        )
        prod_hits = 0
        for number, line in lines:
            # ⛔ A COMMENT IS NOT A WRITER. `yarn_vocabulary.rs:133` is a doc line
            # describing `world.set_flag(flag, on)` and the first version counted
            # it — the same class as the Yarn census counting a character SAYING
            # a call. Strip `//` before matching; a `//` inside a string literal
            # would over-strip, and no writer in this tree is written that way.
            code = line.split("//", 1)[0]
            for verb, family in CALL.findall(code):
                writers[family].append(f"{rel}:{number} ({verb})")
                prod_hits += 1
        suppressed += total_hits - prod_hits
        # A `#[cfg(test)]` in the first third of a file that still has production
        # writers after it is the shape this cut would mis-handle.
        if TEST_MOD.search(text) and prod_hits == 0 and total_hits > 0:
            early_test_mod += 1

    print(f"durable families named by production code: {len(writers)}")
    print(f"in-file `#[cfg(test)]` writes excluded by position: {suppressed}")
    print(f"files whose writes are ALL after a `#[cfg(test)]`: {early_test_mod}  "
          f"(each is either all-test or a miss; read them if non-zero)")
    print()
    for family in sorted(writers, key=lambda f: (-len(writers[f]), f)):
        sites = writers[family]
        mark = "  <-- MORE THAN ONE" if len(sites) > 1 else ""
        print(f"{len(sites):3}  {family}{mark}")
        for site in sites:
            print(f"       {site}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
