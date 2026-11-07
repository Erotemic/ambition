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
checker's job.

⛔⛤ **A BARE `#anchor` WAS OUTSIDE THE POPULATION UNTIL 2026-09-17, AND IT IS THE
EASIEST ONE TO GET WRONG.** A same-file pointer is written by hand from a heading
the author is looking at, so it carries every punctuation trap the slug rule has
— and this checker printed *"every intra-planning pointer resolves"* over a file
that had just grown six of them, unexamined. MEASURED when the rule was added:
**15 same-file links across `docs/planning`, all resolving** — so the widening
cost nothing and closed a blind spot that reads exactly like coverage.
"""

from __future__ import annotations

import pathlib
import re
import sys
import urllib.parse

ROOT = pathlib.Path(__file__).resolve().parents[2]
PLANNING = ROOT / "docs/planning"

#: GitHub's slug: lowercase, drop everything that is not word/space/hyphen,
#: spaces to hyphens. Emoji and punctuation vanish; the hyphens they sat between
#: remain, which is why `— ✅ DONE` becomes `--done`.
LINK = re.compile(r"\]\(([\w./-]*?\.md)#([^)]+)\)")
#: The same pointer with the file left out: `](#some-heading)`, resolved against
#: the page it appears in.
SAME_FILE_LINK = re.compile(r"\]\(#([^)]+)\)")

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


#: The two trees whose contract is that every page is reachable: planning
#: carries the control plane, recipes carry procedures with an index that
#: promises to list them. Other trees (`docs/brainstorms`, `docs/storylines`)
#: hold pages that are deliberately not linked from anywhere, and sweeping them
#: would buy an amnesty list — which is how you hide what it exempts.
TREE_ENTRY_POINTS = {
    "docs/planning": "README.md",
    "docs/recipes": "index.md",
}
REACHABLE_TREES = tuple(TREE_ENTRY_POINTS)


def orphans() -> list[pathlib.Path]:
    """Pages in those trees that NO document links to.

    ⛔⛤ **MEASURED 2026-09-16: TWO OPEN ROWS WITH NAMED NEXT STEPS HAD BEEN
    REACHABLE FROM NOTHING FOR SIX DAYS, AND BOTH WERE LIVE.** One recorded the
    same `app_it` failure signature that hit a different arm that night. The
    consolidation PLAN and its metrics page were in the same state — their own
    README named them in backticks rather than links — so the campaign tree was
    navigable only by knowing the filenames.

    ⇒ A page nobody can reach is the dual of a pointer that lands nowhere, and
    the same guard should see both. ⚠ `README.md` and `index.md` are exempt:
    they are the entry points, and a tree's top README is reached by being the
    top README.
    """
    docs = ROOT / "docs"
    pages = list(docs.rglob("*.md"))
    linked: set[pathlib.Path] = set()
    for page in pages:
        text = page.read_text(encoding="utf-8", errors="replace")
        for match in re.finditer(r"\]\(([^)#\s]+\.md)", text):
            linked.add((page.parent / match.group(1)).resolve())
    # Select documents by repository-relative path, not absolute path.
    stranded = [
        page
        for page in sorted(pages)
        if any(
            str(page.relative_to(ROOT)).startswith(tree) for tree in REACHABLE_TREES
        )
        and page.name not in ("README.md", "index.md")
        and page.resolve() not in linked
    ]
    return stranded


def main() -> int:
    by_path: dict[pathlib.Path, set[str]] = {}
    files = sorted(PLANNING.rglob("*.md"))
    # Check the declared entry points, not a historical population count.
    # The corpus is allowed to shrink when plans are completed and deleted.
    for tree, entry in TREE_ENTRY_POINTS.items():
        root = ROOT / tree
        if not root.is_dir() or not (root / entry).is_file():
            print(f"Missing documentation tree or entry point: {tree}/{entry}")
            return 1
    for path in files:
        by_path[path.resolve()] = anchors(path)

    checked = same_file = 0
    findings: list[str] = []
    for path in files:
        own = by_path[path.resolve()]
        for number, line in enumerate(path.read_text(encoding="utf-8").split("\n"), 1):
            for match in SAME_FILE_LINK.finditer(line):
                same_file += 1
                anchor = urllib.parse.unquote(match.group(1))
                if anchor not in own and slug(anchor) not in own:
                    findings.append(
                        f"  {path.relative_to(ROOT)}:{number}\n"
                        f"     -> #{match.group(1)}  (no such heading in this file)"
                    )
            for match in LINK.finditer(line):
                target = (path.parent / match.group(1)).resolve()
                if target not in by_path:
                    if target.is_relative_to(PLANNING.resolve()):
                        findings.append(
                            f"  {path.relative_to(ROOT)}:{number}\n"
                            f"     -> {match.group(1)} (planning target file is missing)"
                        )
                    continue  # links outside planning have another owner
                checked += 1
                anchor = urllib.parse.unquote(match.group(2))
                # ⚠ Compare on the SLUG of the decoded anchor, so a link written
                # with percent-escaped emoji matches a heading that contains one.
                if anchor not in by_path[target] and slug(anchor) not in by_path[target]:
                    findings.append(
                        f"  {path.relative_to(ROOT)}:{number}\n"
                        f"     -> {target.relative_to(ROOT)}#{match.group(2)}  (no such heading)"
                    )

    stranded = orphans()
    if stranded:
        print(
            f"⛔ {len(stranded)} page(s) under {' and '.join(REACHABLE_TREES)} are "
            "linked from NO document:\n"
        )
        for page in stranded:
            print(f"  {page.relative_to(ROOT)}")
        print(
            "\n⇒ A page nobody can reach is a diagnosis nobody has. Link it from\n"
            "  its directory's README/index, or from the row that depends on it.\n"
            "  Two open rows sat unreachable for six days and both were live."
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
        f"Every intra-planning pointer resolves: {checked} cross-file plus "
        f"{same_file} same-file anchor link(s) across "
        f"{len(files)} document(s); every page under "
        f"{' and '.join(REACHABLE_TREES)} is linked from somewhere."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
