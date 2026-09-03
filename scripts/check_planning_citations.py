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

# ⛔⛔ A PATH WITHOUT A LINE NUMBER WAS NOT BEING CHECKED AT ALL. `FILE_LINE`
# requires the `:123` suffix, and almost no planning row writes one: prose cites
# `features/ecs/footstool.rs`, not `…:168`. MEASURED 2026-09-02: the sweep that
# reported "all 353 citations resolve" was true and judged NONE of the 319 bare
# paths in the same files, one of which had been dead since `00030e603`.
#
# ⭐ THE HARD PART IS THAT THE REPOSITORY ABBREVIATES ON PURPOSE, so a plain
# existence test reports the abbreviation instead of the rot — the "teaches its
# reader to skim" failure this file already warns about, which would be worse
# than no check. Three conventions are all correct and all common:
#
#     platformer2d_core/src/abilities.rs   crates/ambition_platformer2d_core/…
#     actor_monolith/src/…/grapple.rs      …ambition_platformer2d_actor_monolith
#     core/draw.py                         inside the sprite-renderer SUBMODULE
#
# ⭐ SO THE RULE IS ORDERED CONTAINMENT, not a suffix match: the basename must
# name a real file, and every component the citation writes must appear in that
# file's path IN ORDER, where a component matches either exactly or as the tail
# of an `ambition_`-prefixed crate directory. All three above resolve;
# `features/ecs/footstool.rs` does not, because `footstool.rs` lives in
# `crates/ambition_combat/src/` and that path contains neither `features` nor
# `ecs`. That is the finding, and it is what a reader following the row hits.
PATH_SUFFIXES = ("rs", "py", "ron", "toml", "sh", "wgsl", "ldtk", "json", "md")
PATH_CITE = re.compile(
    r"`([A-Za-z0-9_][A-Za-z0-9_./-]*/[A-Za-z0-9_.-]+\.(?:"
    + "|".join(PATH_SUFFIXES)
    + r"))`(?!:)"
)
#: ⚠ A BUILD OUTPUT IS NOT ROT. Rows cite `target/run_tests_status.json` by its
#: real path, which is the correct way to name one; it is absent because nothing
#: has been built in this checkout yet, not because the row is wrong.
GENERATED_PREFIXES = ("target/", "dist/", "build/")

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


MISSING_TRACKED: list[Path] = []


def repo_files() -> list[Path]:
    """Tracked files that are ACTUALLY ON DISK.

    ⛔ `git ls-files` LISTS A FILE THAT HAS BEEN DELETED IN THE WORKTREE and not
    yet committed, and that broke this checker two ways at once. The loud one:
    `--comments` reads every tracked `.rs` and died with `FileNotFoundError`
    mid-sweep, so the run reported nothing at all. The quiet one is worse — the
    deleted path stays in the name index, so a citation to a file someone just
    removed still RESOLVES, and the checker's whole job is to catch that.

    ⇒ Filter here rather than at the read sites: one gate keeps the index and
    every reader honest, and a `try/except OSError` at a read would have fixed
    only the crash while leaving the citation passing.

    The skipped paths are counted and reported rather than dropped in silence —
    a large count means a half-finished rename, not a clean tree.

    ⛔ **THE PREDICATE IS `exists()`, AND `is_file()` IS WRONG HERE.** It called
    seven healthy paths deleted on the first run: the five submodule gitlinks,
    which `git ls-files` reports as entries and which are directories on disk,
    plus `game/ambition_content/assets/sprites` and the Mary-O demo's twin —
    tracked SYMLINKS into the monolith's sprite directory. All seven exist; none
    is readable as a file; only `exists()` tells the three cases apart.
    """
    out = subprocess.run(
        ["git", "ls-files"], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout.split()
    kept = []
    for rel in (Path(p) for p in out):
        if (REPO / rel).exists():
            kept.append(rel)
        else:
            MISSING_TRACKED.append(rel)
    return kept


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


def submodule_files() -> list[str]:
    """Every file a SUBMODULE tracks, as a path from the superproject root.

    ⛔ THE SUBMODULE'S OWN ROOT IS THE CONVENTION its docs cite from.
    `engine/sprite-renderer.md` documents the renderer and writes `core/draw.py`,
    which is exactly right from inside `tools/ambition_sprite2d_renderer/` and
    names nothing at all when joined to the superproject. Six such citations were
    the largest single group in the first path sweep; they are correct prose and
    must not report. Ordered containment does the rest — the components resolve
    against the real path once the real path is in the index.
    """
    listing = subprocess.run(
        ["git", "submodule", "--quiet", "foreach", "--recursive",
         'git ls-files | sed "s|^|$displaypath/|"'],
        cwd=REPO, capture_output=True, text=True,
    )
    return listing.stdout.split()


def _same_component(have: str, want: str) -> bool:
    """One path component, allowing the crate-directory abbreviation.

    `ambition_characters` IS `characters` for citation purposes, and
    `ambition_platformer2d_actor_monolith` is `actor_monolith`: prose drops the
    vendor prefix and as much of the crate path as still reads unambiguously.
    Anchored on `_` so it stays a word boundary — `combat` must not match
    `ambition_precombat`.
    """
    return have == want or have.endswith("_" + want)


def path_resolves(cite: str, by_name: dict[str, list[str]]) -> bool:
    """Does `cite` name a real file, allowing the repository's abbreviations?"""
    parts = cite.split("/")
    for candidate in by_name.get(parts[-1], []):
        have = candidate.split("/")
        i = 0
        for want in parts:
            while i < len(have) and not _same_component(have[i], want):
                i += 1
            if i == len(have):
                break
            i += 1
        else:
            return True
    return False


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
    if MISSING_TRACKED:
        # ⚠ Not a failure — an uncommitted deletion is a normal working state.
        # But say it, because every citation to one of these paths is now
        # correctly reported as dead, and that is surprising until you know why.
        shown = ", ".join(sorted({str(p) for p in MISSING_TRACKED})[:3])
        print(f"skipped {len(set(MISSING_TRACKED))} tracked file(s) missing from "
              f"the worktree (deleted, not yet committed): {shown}"
              f"{' ...' if len(set(MISSING_TRACKED)) > 3 else ''}", file=sys.stderr)
    tracked = {str(p) for p in repo_files()}
    by_suffix: dict[str, list[str]] = {}
    for p in tracked:
        by_suffix.setdefault(Path(p).name, []).append(p)
    # ⛔ SUBMODULE FILES BELONG IN THE PATH INDEX AND NOT IN `by_suffix`.
    # `FILE_LINE` opens the file it resolves to and counts its lines; the path
    # check only asks whether it exists. Keeping the wider index separate means
    # adding submodules cannot change what the line-number pass already reports.
    by_path_name: dict[str, list[str]] = {}
    for p in list(tracked) + submodule_files():
        by_path_name.setdefault(Path(p).name, []).append(p)

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
            for m in PATH_CITE.finditer(line):
                cite = m.group(1)
                # An elided path (`tools/.../specs/x.ron`) names a shape, not a
                # file, and a build output is absent by construction.
                if cite.startswith(GENERATED_PREFIXES) or "..." in cite:
                    continue
                checked += 1
                if not path_resolves(cite, by_path_name):
                    findings.append((str(rel), lineno, m.group(0),
                                     "no file at this path"))
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
