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
    NOT A PATH  a SCHEMATIC (`provider::local_name` -- the SHAPE of a key) or an
                AUTHORED CONTENT KEY (`smash::duelist`, `versus::versus_duelist`
                -- a catalog fragment, not a Rust item). Both are spelled like
                paths and neither half is ever an item. There is no reliable way
                to tell these from a real path by regex, so they are marked, not
                matched. Expect them: they were a fifth of one sweep's findings.

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

#: ⚠ SAME LINE OR THE NEXT ONE. A wrapped comment routinely puts the citation at
#: the end of one line and the parenthetical reason at the start of the next, so
#: a strict same-line rule quietly ignored half the markers people wrote — three
#: of them mine, and I re-triaged those citations twice before noticing.
#:
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
    # ⛔ `X::mod` IS A PATH IDIOM, NOT A SYMBOL. The repo writes `conversion::mod`
    # and `rendering::mod` to mean "that module's mod.rs"; there is no item named
    # `mod` to find and reporting it is pure noise.
    "mod",
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


def source_text(suffixes: tuple[str, ...] = (".rs", ".py")) -> str:
    """Every tracked source with one of `suffixes`, concatenated once.

    ⛔ THE SUFFIX ARGUMENT IS NOT A CONVENIENCE. Concatenating Rust and Python
    into one index makes a name in either language qualify a citation in the
    other, and they collide: a `class FontSource:` in a font-download script
    made the checker judge Rust's upstream `FontSource::Family` as ours, then
    report its variant missing — four findings from one collision. A citation in
    a `.rs` file is judged against Rust definitions.

    One read beats a `grep -r` per citation: a full planning sweep asks a few
    hundred questions and the tree is large enough that the difference is
    minutes.
    """
    chunks = []
    for rel in repo_files():
        if rel.suffix not in suffixes:
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
#: ⛔ MODIFIERS MUST BE CONSUMED, NOT MATCHED. The first version alternated over
#: the keywords directly, so `pub const fn levelled` matched `const` and captured
#: the NEXT word -- "fn" -- and the real name was never indexed at all. Every
#: `const fn`, `async fn` and `unsafe fn` in the tree was invisible, which shows
#: up as a citation to a function that plainly exists being reported missing.
DEFINITION = re.compile(
    r"\b(?:pub\s*(?:\([^)]*\)\s*)?)?"
    r"(?:default\s+)?(?:const\s+|async\s+|unsafe\s+|extern\s+\"[^\"]*\"\s+)*"
    r"(?:" + "|".join(re.escape(k) for k in DEF_KINDS) + r")\s+([A-Za-z_][A-Za-z0-9_]*)"
)
#: An enum variant or associated item, at the start of a line.
#:
#: ⚠ `=` is in the class for two real declaration styles this missed: a variant
#: with a DISCRIMINANT (`Sequence = 1 << 0,`), and a name declared inside a MACRO
#: arm (`SHOCKWAVE => "shockwave",`). Both reported as missing until measured;
#: both exist. A prose linter should err toward believing the tree.
BARE_ITEM = re.compile(r"^\s*([A-Z][A-Za-z0-9_]*)\s*[,({=]", re.M)
#: A STRUCT FIELD. Without this the checker reports every `Type::field` citation
#: -- `ControlSettings::right_stick_mode` is a field, not a method, and the
#: repository cites fields as often as functions.
FIELD = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?([a-z_][a-z0-9_]*)\s*:", re.M)


#: A qualifier is only OURS if the tree declares it as a type or a module.
#: ⛔ Not "any defined name": field and variant indexing is deliberately loose so
#: TAILS resolve, and reusing it for qualifiers let `Gizmos::text_2d` and
#: `bevy_ggrs::RollbackId` through -- upstream names that happen to collide with
#: something loose. A checker that reports upstream names teaches its reader to
#: skim, which is how a real finding gets missed.
QUALIFIER = re.compile(
    r"\b(?:struct|enum|trait|mod|type|class)\s+([A-Za-z_][A-Za-z0-9_]*)"
)


def dependency_crates() -> set[str]:
    """Every crate named as a dependency anywhere in the workspace.

    A qualifier that names one is upstream by construction, whatever our source
    happens to contain.
    """
    names: set[str] = set()
    for rel in repo_files():
        if rel.name != "Cargo.toml":
            continue
        try:
            body = (REPO / rel).read_text(errors="replace")
        except OSError:
            continue
        names.update(re.findall(r"^\s*([a-z][a-z0-9_-]*)\s*=", body, re.M))
    return {n.replace("-", "_") for n in names}


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
    parser.add_argument(
        "--comments", action="store_true",
        help="also check backticked citations in Rust COMMENTS (the fabricated "
             "name that prompted this script reached one, where nothing looks)",
    )
    parser.add_argument("paths", nargs="*", type=Path,
                        default=[REPO / "docs" / "planning"])
    args = parser.parse_args()

    docs = []
    for p in args.paths:
        p = p if p.is_absolute() else REPO / p
        docs.extend(sorted(p.rglob("*.md")) if p.is_dir() else [p])

    print(f"reading {len(list(repo_files()))} tracked files ...", file=sys.stderr)
    text = source_text()
    defined = defined_names(text)
    deps = dependency_crates()
    ours = set(QUALIFIER.findall(text)) - deps
    # A citation inside a `.rs` file is judged against RUST names only; see
    # `source_text`. Planning prose cites both languages, so it keeps the union.
    rust_text = source_text((".rs",))
    rust_defined = defined_names(rust_text)
    rust_ours = set(QUALIFIER.findall(rust_text)) - deps
    print(f"indexed {len(defined)} defined name(s), {len(ours)} usable qualifier(s)",
          file=sys.stderr)
    tracked = {str(p) for p in repo_files()}
    by_suffix: dict[str, list[str]] = {}
    for p in tracked:
        by_suffix.setdefault(Path(p).name, []).append(p)

    findings: list[tuple[str, int, str, str]] = []
    checked = 0
    if args.comments:
        # ⭐ SAME RULE, WIDER TARGET. A comment citation is judged exactly like a
        # planning one -- there is no reason a name in prose is more real for
        # sitting next to the code. MEASURED 2026-09-02: 2,847 judged, 45
        # distinct names unresolved in 59 places, about 2%.
        for rel in repo_files():
            if rel.suffix != ".rs":
                continue
            src_lines = (REPO / rel).read_text(errors="replace").splitlines()
            for lineno, line in enumerate(src_lines, 1):
                stripped = line.lstrip()
                if not stripped.startswith("//"):
                    continue
                if MARKER in line or (
                    lineno < len(src_lines) and MARKER in src_lines[lineno]
                ):
                    continue
                for m in SYMBOL.finditer(line):
                    parts = m.group(1).split("::")
                    head, tail = parts[0], parts[-1]
                    if tail in NOISE or len(tail) < 3:
                        continue
                    if head in NOISE or head not in rust_ours:
                        continue
                    checked += 1
                    if tail not in rust_defined:
                        findings.append((str(rel), lineno, m.group(0),
                                         "nothing DEFINES this name"))
    for doc in docs:
        # ⛔ A DOC OUTSIDE THE REPO IS A LEGITIMATE TARGET -- a fixture under
        # pytest's tmp_path, or a file being checked before it is committed.
        # `relative_to` RAISES on those, which crashed the checker rather than
        # reporting anything, and a crash reads as "no findings" to any caller
        # that only looks at the output.
        try:
            rel = doc.relative_to(REPO)
        except ValueError:
            rel = doc
        doc_lines = doc.read_text(errors="replace").splitlines()
        for lineno, line in enumerate(doc_lines, 1):
            if MARKER in line or (
                lineno < len(doc_lines) and MARKER in doc_lines[lineno]
            ):
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
                if head in NOISE or head not in ours:
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
