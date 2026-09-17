#!/usr/bin/env python3
"""Does a `path:N` citation still point at the LINE it was written against?

⛔⛤ **`check_planning_citations.py` CANNOT ANSWER THIS, AND SAYS SO.** Its
`FILE_LINE` rule verifies that the path resolves, that `N` is not past the end of
the file, and that the path is unambiguous -- never that line `N` still holds
what the sentence claims. A line number inside a living file is the one citation
form that rots in total silence: the file grows above the cited line and the
coordinate now addresses a different statement, with every check still green.

⛔ **AND THE OBVIOUS INSTRUMENT WAS MEASURED AND REJECTED FIRST.**
`scripts/citation_line_content_feasibility.py` records the NEGATIVE: pairing a
`file:line` with the prose tokens around it recovers the content check for only
23% of citations at +/-30 lines, and a clause-bound, uniquely-defined-name
narrowing was WORSE (32% miss at +/-20). Guessing the intended content from prose
does not work.

⭐⭐ **SO DO NOT GUESS IT -- READ IT.** `git blame` gives the commit that last
wrote the DOC line holding the citation. `git show <that commit>:<path>` gives
the cited file exactly as the author saw it. Compare line `N` then against line
`N` now. That is text against text, with no inference at any step, and it is
exact in both directions:

    same       the coordinate still addresses the same statement
    moved      that statement is now at a different line, found EXACTLY once
    ambiguous  the old text occurs several times now; a human picks
    gone       the old text is not in the file at all -- the code changed, and
               the citation is a claim about something that no longer exists
    no-history the path did not exist at that commit (a file move, usually)
    uncommitted the citing doc line is not committed, so it has no reference
               point yet -- commit and re-run

`--fix` repoints the `moved` ones, which is the whole population that can be
repaired without a judgement call.

⚠ **THE REFERENCE POINT IS THE DOC LINE'S LAST WRITE, NOT THE CITATION'S BIRTH.**
Editing a sentence for an unrelated reason moves its blame forward and re-bases
its citation against a tree where the coordinate was already wrong. That makes
this check CONSERVATIVE -- it under-reports drift and never invents it -- which
is the right direction for a tool that rewrites files.

⛔ **NOT A GATE BY DEFAULT.** The first run over `docs/planning` found 103 of 295
citations no longer addressing their line. A check that fails on a third of the
corpus is a report wearing a gate's costume; `--strict` exists for a lane that
has repaired its own.
"""

from __future__ import annotations

import argparse
import collections
import importlib.util
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent


def _load_citation_checker():
    """⛔ ONE RESOLVER, NOT A SECOND AUTHORITY.

    `path_resolves` already knows this repository's abbreviation habit -- a doc
    writes `control/queries.rs` for a path four components long. A private copy
    of that rule here would drift from the one the gate uses, and the first
    version of this script did exactly that: a plain `Path.is_file()` reported
    203 of 329 citations as pointing at nothing, every one of them a real file
    under an abbreviated name.
    """
    spec = importlib.util.spec_from_file_location(
        "check_planning_citations", REPO / "scripts" / "check_planning_citations.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _git(*args: str, root: pathlib.Path = REPO) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *args], cwd=root, capture_output=True, text=True)


class Corpus:
    """⛔ `root` EXISTS SO THE TOOL CAN BE POISONED ON A FIXTURE.

    A checker that can only run against the repository it lives in cannot be
    shown to fail: every arm would be asserting today's corpus back at itself.
    With a root, a test builds two commits in a temp repo, moves a line, and
    watches the verdict flip — which is the only arm that fails for the right
    reason.
    """

    def __init__(self, cpc, root: pathlib.Path = REPO):
        self.cpc = cpc
        self.root = root
        self.by_name: dict[str, list[str]] = collections.defaultdict(list)
        names = (
            [str(rel) for rel in cpc.repo_files()]
            if root == REPO
            else _git("ls-files", root=root).stdout.split()
        )
        for name in names:
            self.by_name[pathlib.Path(name).name].append(name)
        self._old: dict[tuple[str, str], list[str] | None] = {}
        self._now: dict[str, list[str]] = {}

    def resolve(self, cite: str) -> str | None:
        """The tracked path this citation names, by the gate's own rule."""
        parts = cite.split("/")
        for candidate in self.by_name.get(parts[-1], []):
            have = candidate.split("/")
            i = 0
            for want in parts:
                while i < len(have) and not self.cpc._same_component(have[i], want):
                    i += 1
                if i == len(have):
                    break
                i += 1
            else:
                return candidate
        return None

    def at(self, sha: str, path: str) -> list[str] | None:
        key = (sha, path)
        if key not in self._old:
            got = _git("show", f"{sha}:{path}", root=self.root)
            self._old[key] = got.stdout.splitlines() if got.returncode == 0 else None
        return self._old[key]

    def now(self, path: str) -> list[str]:
        if path not in self._now:
            self._now[path] = (self.root / path).read_text(errors="replace").splitlines()
        return self._now[path]


def blame_shas(doc: pathlib.Path, root: pathlib.Path = REPO) -> dict[int, str]:
    """`{doc line: the commit that last wrote it}`."""
    out = _git(
        "blame", "--line-porcelain", "--", str(doc), root=root
    ).stdout.splitlines()
    shas: dict[int, str] = {}
    line = 0
    for row in out:
        head = re.match(r"^([0-9a-f]{40}) \d+ (\d+)", row)
        if head:
            line = int(head.group(2))
            shas[line] = head.group(1)
    return shas


def _shown(doc: pathlib.Path) -> str:
    """⛔ A DOC PASSED ON THE COMMAND LINE IS OFTEN ALREADY RELATIVE, and
    `Path.relative_to` RAISES on that rather than returning it — which turned a
    report into a traceback the first time this was run with an explicit path.
    The sibling checker carries the same warning at its own `relative_to`."""
    try:
        return str(doc.resolve().relative_to(REPO))
    except ValueError:
        return str(doc)


Finding = collections.namedtuple(
    "Finding", "doc docline cite lineno path verdict was now suggestion"
)


def examine(docs: list[pathlib.Path], corpus: Corpus) -> list[Finding]:
    findings: list[Finding] = []
    for doc in docs:
        text = doc.read_text().splitlines()
        hits = [
            (i + 1, m)
            for i, line in enumerate(text)
            for m in corpus.cpc.FILE_LINE.finditer(line)
        ]
        if not hits:
            continue
        shas = blame_shas(doc, corpus.root)
        for docline, match in hits:
            cite, lineno = match.group(1), int(match.group(2))
            path = corpus.resolve(cite)
            sha = shas.get(docline)
            if path is None or sha is None:
                continue
            if sha == "0" * 40:
                # ⛔⛤ AN UNCOMMITTED DOC LINE HAS NO REFERENCE POINT, AND SAYING
                # "no-history" ABOUT IT WOULD BE A CLAIM ABOUT THE CODE. `git
                # blame` gives an all-zero sha for a working-tree edit, so the
                # first `--fix` run turned its own 67 repairs into 67 phantom
                # findings on the very next run. Commit, then re-read.
                findings.append(
                    Finding(doc, docline, cite, lineno, path, "uncommitted", "", "", None)
                )
                continue
            old = corpus.at(sha, path)
            if old is None:
                findings.append(
                    Finding(doc, docline, cite, lineno, path, "no-history", "", "", None)
                )
                continue
            was = (old[lineno - 1] if 0 < lineno <= len(old) else "").strip()
            current = corpus.now(path)
            now = (current[lineno - 1] if 0 < lineno <= len(current) else "").strip()
            if was == now:
                findings.append(
                    Finding(doc, docline, cite, lineno, path, "same", was, now, None)
                )
                continue
            if not was:
                # The cited line was blank when the sentence was written, so
                # there is nothing to track. Not a drift; not repairable either.
                findings.append(
                    Finding(doc, docline, cite, lineno, path, "blank-origin", was, now, None)
                )
                continue
            occurrences = [i + 1 for i, line in enumerate(current) if line.strip() == was]
            if len(occurrences) == 1:
                findings.append(
                    Finding(doc, docline, cite, lineno, path, "moved", was, now, occurrences[0])
                )
            elif occurrences:
                findings.append(
                    Finding(doc, docline, cite, lineno, path, "ambiguous", was, now, None)
                )
            else:
                findings.append(
                    Finding(doc, docline, cite, lineno, path, "gone", was, now, None)
                )
    return findings


def repoint(findings: list[Finding]) -> int:
    """Rewrite every `moved` citation to the line its text is on now.

    ⛔ PER OCCURRENCE, NOT PER LINE. One doc line can carry two citations of the
    same file at different lines, and a blind `str.replace` would rewrite both.
    """
    by_doc: dict[pathlib.Path, list[Finding]] = collections.defaultdict(list)
    for f in findings:
        if f.verdict == "moved":
            by_doc[f.doc].append(f)
    fixed = 0
    for doc, rows in by_doc.items():
        lines = doc.read_text().splitlines(keepends=True)
        for f in sorted(rows, key=lambda r: -r.docline):
            old_token = f"`{f.cite}:{f.lineno}`"
            new_token = f"`{f.cite}:{f.suggestion}`"
            index = f.docline - 1
            if old_token not in lines[index]:
                # The anchor is gone: another repair in this run already moved
                # it, or the doc changed under us. Skip loudly rather than
                # rewrite the wrong token.
                print(
                    f"  ⚠ skipped {doc}:{f.docline} — {old_token} is not on that line",
                    file=sys.stderr,
                )
                continue
            lines[index] = lines[index].replace(old_token, new_token, 1)
            fixed += 1
        doc.write_text("".join(lines))
    return fixed


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "paths",
        nargs="*",
        type=pathlib.Path,
        help="docs to examine (default: every .md under docs/planning)",
    )
    parser.add_argument(
        "--fix",
        action="store_true",
        help="repoint every `moved` citation; leaves ambiguous and gone alone",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="exit 1 when anything but `same` and `blank-origin` remains",
    )
    parser.add_argument("--quiet", action="store_true", help="counts only")
    parser.add_argument(
        "--root",
        type=pathlib.Path,
        default=REPO,
        help="repository to read from (a test fixture; defaults to this one)",
    )
    args = parser.parse_args(argv)

    cpc = _load_citation_checker()
    corpus = Corpus(cpc, args.root)
    if args.paths:
        docs = sorted(p for p in args.paths if p.suffix == ".md")
    else:
        docs = sorted((args.root / "docs" / "planning").rglob("*.md"))

    findings = examine(docs, corpus)
    counts = collections.Counter(f.verdict for f in findings)

    # ⛔ THE ANTI-VACUITY FLOOR. An import failure, a bad pathspec or a `git
    # blame` that returns nothing all produce an empty finding list, and "0
    # drifted" reads exactly like a clean corpus.
    if not findings:
        print(
            f"no `path:N` citation was examined across {len(docs)} document(s). "
            f"That is this checker reporting on ITSELF, not on the corpus — it "
            f"means the resolver, the blame read or the document list came back "
            f"empty.",
            file=sys.stderr,
        )
        return 2

    if not args.quiet:
        for verdict in ("gone", "ambiguous", "moved", "no-history"):
            rows = [f for f in findings if f.verdict == verdict]
            if not rows:
                continue
            print(f"\n{verdict.upper()} ({len(rows)}):")
            for f in rows:
                where = f"{_shown(f.doc)}:{f.docline}"
                target = f"{f.cite}:{f.lineno}"
                arrow = f" -> :{f.suggestion}" if f.suggestion else ""
                print(f"  {where}  cites {target}{arrow}")
                print(f"      then: {f.was[:100]}")
                print(f"      now : {f.now[:100]}")

    fixed = repoint(findings) if args.fix else 0

    print(
        f"\n{len(findings)} `path:N` citation(s) across {len(docs)} document(s): "
        + ", ".join(f"{n} {name}" for name, n in sorted(counts.items()))
    )
    if args.fix:
        print(f"repointed {fixed}")
    if counts["uncommitted"]:
        print(
            f"  ⚠ {counts['uncommitted']} citation(s) sit on UNCOMMITTED doc lines "
            f"and have no reference point yet — commit and re-run to judge them."
        )
    drifted = (
        len(findings)
        - counts["same"]
        - counts["blank-origin"]
        - counts["uncommitted"]
        - fixed
    )
    if args.strict and drifted:
        print(
            f"\n{drifted} citation(s) no longer address the line they were written "
            f"against. `--fix` repoints the unambiguous ones; `gone` and "
            f"`ambiguous` want a reader.",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
