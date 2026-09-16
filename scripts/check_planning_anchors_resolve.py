#!/usr/bin/env python3
"""A pointer into a planning heading must still find that heading.

⛔⛤ **A HEADING RENAME SILENTLY BREAKS EVERY POINTER TO IT, AND NOTHING IN THE
GATE SAW IT.** MEASURED 2026-09-16: marking the A10 row done — appending
`— ✅ DONE, DEMOLITION CLOSED 2026-09-16` to its `###` heading — broke
`status.md`'s only link to it. `--maintenance` stayed 10/10 across four runs
either side of that edit, because the doc-link job checks the agent KB and the
citation checkers resolve PATHS and SYMBOLS, not anchors.

⇒ The control plane is a set of documents that point at each other. A pointer
that lands on nothing sends a reader to the top of a 1,700-line file to search by
eye, which is the same cost as having no pointer at all — except that it looks
like one.

⚠ **WHAT THIS DOES NOT CHECK.** Only links WITHIN `docs/planning` and only into
files in that tree. A link out to `docs/concepts` or into source is another
checker's job, and a bare `#anchor` with no file is a same-file link this does
not resolve either.
"""

from __future__ import annotations

import pathlib
import re
import sys
import urllib.parse

ROOT = pathlib.Path(__file__).resolve().parents[1]
PLANNING = ROOT / "docs/planning"

#: GitHub's slug: lowercase, drop everything that is not word/space/hyphen,
#: spaces to hyphens. Emoji and punctuation vanish; the hyphens they sat between
#: remain, which is why `— ✅ DONE` becomes `--done`.
LINK = re.compile(r"\]\(([\w./-]*?\.md)#([^)]+)\)")


def slug(heading: str) -> str:
    text = heading.lstrip("#").strip()
    text = re.sub(r"[^\w\s-]", "", text.lower())
    return text.replace(" ", "-")


def anchors(path: pathlib.Path) -> set[str]:
    return {
        slug(line)
        for line in path.read_text(encoding="utf-8").split("\n")
        if line.startswith("#")
    }


def main() -> int:
    by_name: dict[str, set[str]] = {}
    files = sorted(PLANNING.rglob("*.md"))
    # ⛔ ANTI-VACUITY. An empty scan root reports clean, which is also what a
    # healthy corpus reports.
    if len(files) < 20:
        print(f"⛔⛔ only {len(files)} planning document(s) found; the scan is broken")
        return 1
    for path in files:
        by_name.setdefault(path.name, set()).update(anchors(path))

    checked = 0
    findings: list[str] = []
    for path in files:
        for number, line in enumerate(path.read_text(encoding="utf-8").split("\n"), 1):
            for match in LINK.finditer(line):
                target = pathlib.Path(match.group(1)).name
                if target not in by_name:
                    continue  # out of this tree; another checker's population
                checked += 1
                anchor = urllib.parse.unquote(match.group(2))
                # ⚠ Compare on the SLUG of the decoded anchor, so a link written
                # with percent-escaped emoji matches a heading that contains one.
                if anchor not in by_name[target] and slug(anchor) not in by_name[target]:
                    findings.append(
                        f"  {path.relative_to(ROOT)}:{number}\n"
                        f"     -> {target}#{match.group(2)}  (no such heading)"
                    )

    if checked < 20:
        print(
            f"⛔⛔ only {checked} intra-planning anchor link(s) checked; a clean "
            "verdict would be a claim about the scan"
        )
        return 1

    if findings:
        print(f"⛔ {len(findings)} planning pointer(s) land on no heading:\n")
        print("\n".join(findings))
        print(
            "\n⇒ A heading rename breaks every pointer to it and the compiler, the\n"
            "  citation checkers and the doc-link job all stay green. Repoint the\n"
            "  link, or rename the heading back."
        )
        return 1

    print(
        f"Every intra-planning pointer resolves: {checked} anchor link(s) across "
        f"{len(files)} document(s)."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
