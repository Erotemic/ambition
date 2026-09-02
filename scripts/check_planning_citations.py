#!/usr/bin/env python3
"""Check that the symbols and files `docs/planning/` cites actually exist.

⛔⛔ WHY THIS EXISTS. On 2026-09-02 a planning row cited
`grid_backend::focus_key_for_cursor`, a function that has NEVER existed --
`git log -S` finds no commit that ever added it. It was invented, then copied
into a commit message and a doc comment because each reuse copied the previous
one. Nothing we run could see it: it is in prose, so the compiler and rustfmt
never look, and it greps to nothing, which is indistinguishable from code that
MOVED. A reader cannot tell a fabricated citation from a stale one, and the
stale/fabricated distinction is the difference between "fix the row" and "the
row was never true".

⭐ THE POINT IS THE TRIAGE, NOT THE COUNT. Every finding here is one of:

    MOVED       the file or symbol was renamed/deleted -> update the row
    FABRICATED  it never existed -> the row's claim needs re-deriving, not
                re-pointing. `git log -S '<name>' --all` is the discriminator
                and this script prints the command for you.
    QUOTED      a row deliberately quotes a name that is wrong, to record a
                mistake. Mark those (see MARKER below) so they stop reporting.

⚠ THIS IS A LINTER FOR PROSE AND IT WILL HAVE FALSE POSITIVES. It cannot know
about upstream crates, external tools, or names that only appear in generated
code. It is a WORKLIST, not a gate, and it exits 0 unless `--strict` is passed.

USAGE
    scripts/check_planning_citations.py [--strict] [PATH ...]
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

#: Put this on the same line as a citation that is wrong ON PURPOSE -- a row
#: quoting a mistake it is recording. `three classes of comment history`: a
#: quoted mistake outlives the mistake, and must not be silently "fixed".
MARKER = "cite-ok"

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True, text=True, check=True,
    ).stdout.strip()
)

# `path/file.rs:123` or `file.rs:123`
FILE_LINE = re.compile(r"`([A-Za-z0-9_./-]+\.(?:rs|py|ron|toml|sh|md)):(\d+)`")
# `module::thing` / `Type::method` -- at least one `::`, ordinary Rust idents.
SYMBOL = re.compile(r"`([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)+)`")

#: Segments that are never a definition to look for on their own.
NOISE = {
    "self", "crate", "super", "std", "core", "alloc",
    # ⛔ PRIMITIVES ARE NOT OURS. `usize::MAX` and `f32::INFINITY` cite the
    # standard library; the repository happens to define fields and type
    # aliases with these spellings, so the qualifier test alone lets them past.
    "usize", "isize", "bool", "char", "str", "String",
    "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128",
    "f32", "f64",
}


def repo_files() -> list[Path]:
    out = subprocess.run(
        ["git", "ls-files"], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout.split()
    return [Path(p) for p in out]


def source_text() -> str:
    """Every tracked Rust/Python source, concatenated once.

    One read beats a `grep -r` per citation: a full planning sweep asks a few
    hundred questions and the tree is large enough that the difference is
    minutes.
    """
    chunks = []
    for rel in repo_files():
        if rel.suffix not in {".rs", ".py"}:
            continue
        try:
            chunks.append((REPO / rel).read_text(errors="replace"))
        except OSError:
            continue
    return "\n".join(chunks)


DEF_KINDS = (
    "fn", "struct", "enum", "trait", "const", "static", "type", "mod",
    "union", "class", "def", "macro_rules!",
)
#: One pass over the whole tree, collecting every name it DEFINES.
DEFINITION = re.compile(
    r"\b(?:" + "|".join(re.escape(k) for k in DEF_KINDS) + r")\s+([A-Za-z_][A-Za-z0-9_]*)"
)
#: An enum variant or associated item, at the start of a line.
BARE_ITEM = re.compile(r"^\s*([A-Z][A-Za-z0-9_]*)\s*[,({]", re.M)
#: A STRUCT FIELD. Without this the checker reports every `Type::field` citation
#: -- `ControlSettings::right_stick_mode` is a field, not a method, and the
#: repository cites fields as often as functions.
FIELD = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?([a-z_][a-z0-9_]*)\s*:", re.M)


def defined_names(text: str) -> set[str]:
    """Every name the tree DEFINES, indexed once.

    ⛔ MENTIONS MUST NOT COUNT, which is the whole difficulty. The fabricated
    name that prompted this script APPEARS in the repo -- inside the comment
    that cites it -- so a substring search would have called it present. Only
    definition sites go in this set.

    ⚠ And it is built ONCE. The first draft asked a dozen regexes of the whole
    concatenated tree per citation and did not finish in ten minutes; a few
    hundred citations against a set is instant.
    """
    names = set(DEFINITION.findall(text))
    names.update(BARE_ITEM.findall(text))
    names.update(FIELD.findall(text))
    return names


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--strict", action="store_true",
                        help="exit 1 when anything is unresolved")
    parser.add_argument("paths", nargs="*", type=Path,
                        default=[REPO / "docs" / "planning"])
    args = parser.parse_args()

    docs = []
    for p in args.paths:
        p = p if p.is_absolute() else REPO / p
        docs.extend(sorted(p.rglob("*.md")) if p.is_dir() else [p])

    print(f"reading {len(list(repo_files()))} tracked files ...", file=sys.stderr)
    defined = defined_names(source_text())
    print(f"indexed {len(defined)} defined name(s)", file=sys.stderr)
    tracked = {str(p) for p in repo_files()}
    by_suffix: dict[str, list[str]] = {}
    for p in tracked:
        by_suffix.setdefault(Path(p).name, []).append(p)

    findings: list[tuple[str, int, str, str]] = []
    checked = 0
    for doc in docs:
        rel = doc.relative_to(REPO)
        for lineno, line in enumerate(doc.read_text(errors="replace").splitlines(), 1):
            if MARKER in line:
                continue
            for m in FILE_LINE.finditer(line):
                checked += 1
                path, want = m.group(1), int(m.group(2))
                hits = [p for p in by_suffix.get(Path(path).name, [])
                        if p.endswith(path)]
                if not hits:
                    findings.append((str(rel), lineno, m.group(0), "no such file"))
                    continue
                n = len((REPO / hits[0]).read_text(errors="replace").splitlines())
                if want > n:
                    findings.append((str(rel), lineno, m.group(0),
                                     f"{hits[0]} has {n} lines"))
            for m in SYMBOL.finditer(line):
                checked += 1
                parts = m.group(1).split("::")
                head, tail = parts[0], parts[-1]
                if tail in NOISE or len(tail) < 3:
                    continue
                # ⛔ ONLY JUDGE NAMES THIS REPOSITORY OWNS. `FontSize::Rem`,
                # `YarnSpinnerPlugin::with_yarn_sources` and `usize::MAX` are
                # upstream, and a checker that reports them teaches its reader
                # to skim past it -- which is how a real finding gets missed.
                # Requiring the QUALIFIER to be defined here is the cheapest
                # test that keeps `grid_backend::…` and drops the rest.
                if head in NOISE or head not in defined:
                    continue
                if tail not in defined:
                    findings.append((str(rel), lineno, m.group(0),
                                     "nothing DEFINES this name"))

    print(f"\nchecked {checked} citation(s) across {len(docs)} planning file(s)")
    if not findings:
        print("all resolved.")
        return 0
    print(f"{len(findings)} unresolved:\n")
    for rel, lineno, cite, why in findings:
        print(f"  {rel}:{lineno}\n    {cite} -- {why}")
    print(f"""
⇒ TRIAGE EACH ONE. It is MOVED, FABRICATED, or QUOTED-ON-PURPOSE, and the
  three want different fixes. The discriminator between the first two is:

      git log -S '<the name>' --all --oneline

  Output means it existed and was renamed or deleted: repoint the row. NO
  output means it never existed: the row's claim needs re-deriving, because
  whoever wrote it was describing code they had not read.

  For a citation that is wrong deliberately -- a row quoting a mistake it is
  recording -- put `{MARKER}` on that line and it stops reporting.""")
    return 1 if args.strict else 0


if __name__ == "__main__":
    raise SystemExit(main())
