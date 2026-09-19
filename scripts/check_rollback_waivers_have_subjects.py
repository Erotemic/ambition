#!/usr/bin/env python3
"""Every rollback-coverage waiver still has a subject in the source tree.

⛔⛤ **A WAIVER WHOSE SUBJECT IS GONE IS WORSE THAN A MISSING ONE: IT SILENTLY
PRE-APPROVES WHATEVER A FUTURE TYPE OF THAT NAME WRITES.** `rollback_coverage.rs`
asks, for each resource it sweeps, whether some waiver row matches it. No check
asked the converse — whether a row still matches anything. Found 2026-09-18 when
the player-clone relic was deleted: `"::app::player_clone::"` became a waiver for
a module that does not exist, and every arm in that file stayed green.

⛔⛔ **AND THE OBVIOUS PLACE TO ASK IS THE WRONG ONE, MEASURED.** My first repair
was a Rust arm sweeping the shipped composition's live resources and reporting
rows that matched none. It named FIFTEEN, and the premise was wrong rather than
the rows: a resource absent from the world one `update()` after boot is usually
LAZILY INSERTED or composition-specific, not deleted. `::rooms::transaction::`
exists only DURING a room transaction; `SessionMechanics` only while a
generation is active; `ActiveRollbackAuthority` only with a session;
`PlayerManaRegen` only once a mana-having character spawns; and the
`ambition_demo_twintrack`, `ambition_demo_smash` and `ambition_match` rows
belong to compositions that harness never builds.

⇒ *"Is its subject gone?"* is a question about SOURCE, not about a world at one
instant, which is why this lives here and not there. A type declared anywhere in
the tree has a subject, whenever it happens to be inserted.

# # What a match is

A waiver row's spelling already selects its scope in `rollback_coverage.rs`, and
this reuses that rule rather than inventing a second one:

    ends with `::`   a module-family waiver  → the spelling appears in source
    ends with `<`    a generic's prefix      → the spelling appears in source
    bare lower-case  a CRATE-prefix waiver   → that crate directory exists
    otherwise        a full type-path suffix → a type of that name is declared

⚠ The third line is a scope my first rule did not model, and its absence made
this check's first run report `ambition_menu` as a dead row for a live crate.
`RESOURCE_WAIVED`'s own doc block says crate-prefix waivers exist; I read the
spelling rule and not the sentence above it.

⚠ The match is deliberately loose — a declaration anywhere, including in test
code. A waiver for a type that exists only in tests is a different (and much
smaller) problem than a waiver for a type that does not exist at all, and this
check is only about the second.
"""

from __future__ import annotations

import functools
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

REPO = Path(__file__).resolve().parent.parent
COVERAGE = REPO / "game" / "ambition_app" / "tests" / "rollback_coverage.rs"

#: The waiver tables this check reads out of `rollback_coverage.rs`.
TABLES = ("WAIVED", "RESOURCE_WAIVED", "INERT_WAIVED")

#: ⛔ Anti-vacuity: if the table parse breaks, zero rows means zero failures.
FLOORS = {"waiver rows": 60, "declared type names": 2000}

_ROW = re.compile(r'^\s*\(\s*"([^"]+)"\s*,', re.MULTILINE)


@functools.cache
def _lockfile() -> str:
    """`Cargo.lock`, the authority on which crates are in the build at all."""
    return (REPO / "Cargo.lock").read_text(errors="replace")
_TYPE_DECL = re.compile(r"\b(?:struct|enum|type)\s+([A-Z][A-Za-z0-9_]*)")

#: An inline `mod name {` — a module with no file of its own.
_INLINE_MOD = re.compile(r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?mod[ \t]+([a-z_][a-z_0-9]*)[ \t]*\{", re.MULTILINE)


def waiver_rows() -> dict[str, list[str]]:
    """`{table: [needle, ..]}` for each waiver table in `rollback_coverage.rs`."""
    text = COVERAGE.read_text(errors="replace")
    out: dict[str, list[str]] = {}
    for table in TABLES:
        decl = f"const {table}: &[(&str, &str)] = &["
        start = text.find(decl)
        if start < 0:
            continue
        # ⚠ The array's `[` is the LAST one in the declaration, not the first:
        # `&[(&str, &str)]` contains two before it, and starting the brace
        # scan at `text.index("[", start)` closed on the type signature and
        # parsed every table as empty — caught only by this check's own floor.
        depth, index = 0, start + len(decl) - 1
        end = index
        while end < len(text):
            if text[end] == "[":
                depth += 1
            elif text[end] == "]":
                depth -= 1
                if depth == 0:
                    break
            end += 1
        out[table] = _ROW.findall(text[index : end + 1])
    return out


def _tree_facts() -> tuple[set[str], set[str]]:
    """`(declared type names, module paths)` over the whole Rust tree.

    ⛔⛤ **THE MODULE PATHS ARE DERIVED FROM FILE PATHS, NOT SEARCHED FOR IN
    SOURCE**, and the difference is the whole rule. My first version looked for
    the literal spelling `a::b::c` in the corpus and reported NINE live modules
    as dead: Rust source writes `mod c;` and `use crate::b::c`, so the full path
    a waiver spells almost never appears anywhere. `crates/X/src/a/b.rs` IS the
    module `X::a::b`, and that is a fact about the filesystem.

    ⛔⛤ **AND THE FILE THE WAIVERS LIVE IN IS NOT EVIDENCE FOR THEM.** Poisoned
    2026-09-18 by restoring the dead `"::app::player_clone::"` row this check was
    built for — it PASSED, because the row's own string literal was in the
    corpus a substring search was reading. A guard whose haystack contains its
    own subject answers *"did somebody write this down"*.
    """
    names: set[str] = set()
    modules: set[str] = set()
    for root in ("crates", "game", "tools", "dev"):
        base = REPO / root
        if not base.is_dir():
            continue
        for path in base.rglob("*.rs"):
            if any(part == "target" for part in path.parts):
                continue
            if path == COVERAGE:
                continue
            text = path.read_text(errors="replace")
            names.update(_TYPE_DECL.findall(text))
            rel = path.relative_to(base)
            parts = list(rel.parts)
            if len(parts) < 2 or parts[1] != "src":
                continue
            crate, tail = parts[0], parts[2:]
            if tail and tail[-1] in ("mod.rs", "lib.rs"):
                tail = tail[:-1]
            elif tail:
                tail = tail[:-1] + [tail[-1][: -len(".rs")]]
            own = "::".join([crate, *tail])
            modules.add(own)
            # ⛔⛤ **786 INLINE `mod x { .. }` DECLARATIONS EXIST, AND A
            # FILESYSTEM-ONLY DERIVATION CANNOT SEE ONE.** A waiver naming an
            # inline module would read as dead. Measured 2026-09-18 while
            # chasing the one row this check does flag — the gap was real even
            # though that row turned out not to be an instance of it.
            for inline in _INLINE_MOD.findall(text):
                modules.add(f"{own}::{inline}" if own else inline)
    return names, modules


def module_is_live(prefix: str, modules: set[str]) -> bool:
    """Does a waiver's module path name a module the tree actually has?

    ⚠ **SUFFIX MATCH, BECAUSE WAIVERS SPELL THE PATH TWO WAYS** and both are
    legitimate: fully qualified from a crate root
    (`ambition_platformer2d_runtime::room_transition::loading::X`) and relative
    with a leading `::` (`::room_transition::loading::X`). MEASURED 2026-09-18:
    37 of the 127 qualified subjects use the relative form, so an exact match
    would report all 37 as dead.

    ⚠ **WHAT THIS STILL CANNOT SEE**, stated rather than left to be
    discovered: a re-export. A type declared in `a::b` and published as
    `crate::T` is legitimately waivable under either spelling, so this asks
    only that the named module EXISTS — not that the leaf is declared in it.
    Requiring that would redden every `pub use`.

    ⭐ MEASURED 2026-09-18 ACROSS THE SHIPPED TABLES: 0 of 127 qualified
    subjects name a module the tree does not have, so this lands as a ratchet
    rather than a repair.
    """
    prefix = prefix.strip(":")
    if not prefix:
        return True
    return any(m == prefix or m.endswith(f"::{prefix}") for m in modules)


def subjectless() -> tuple[dict[str, list[str]], dict[str, int]]:
    rows = waiver_rows()
    names, modules = _tree_facts()
    dead: dict[str, list[str]] = {}
    for table, needles in rows.items():
        for needle in needles:
            if needle.endswith("::"):
                # A module family: some module path must END with this one's
                # segments. `::world_flow::room_transition_loading::` is alive
                # iff a file makes `..::world_flow::room_transition_loading` a
                # module.
                want = needle.strip(":")
                if want and any(
                    m == want or m.endswith(f"::{want}") for m in modules
                ):
                    continue
                # ⛤ A FOURTH SCOPE: an EXTERNAL crate's module family. `bevy_asset::`
                # and `bevy_state::` waive types this workspace never declares, and a
                # workspace-only corpus calls them dead. `Cargo.lock` is the right
                # authority for "this crate is in the build".
                head = want.split("::", 1)[0]
                if head and f'name = "{head}"' in _lockfile():
                    continue
            elif needle.endswith("<"):
                # A generic's prefix: its leaf is a declared type.
                leaf = needle.rstrip("<").rsplit("::", 1)[-1]
                if leaf in names:
                    continue
            elif "::" not in needle and needle.islower():
                # ⛤ A THIRD SCOPE MY FIRST RULE DID NOT MODEL: a bare
                # lower-case entry is a CRATE-prefix waiver, which
                # `RESOURCE_WAIVED`'s own doc block mentions ("crate-prefix
                # waivers from `WAIVED` apply here too"). There is exactly one
                # — `ambition_menu` — and reading it as a type path made this
                # check's first run report a dead row for a live crate.
                if (REPO / "crates" / needle).is_dir() or (REPO / "game" / needle).is_dir():
                    continue
            else:
                # ⛔⛤ **THE LEAF WAS THE WHOLE TEST UNTIL 2026-09-18, AND A DEAD
                # MODULE PATH STAYED GREEN.** `a::dead::path::LiveType` passed
                # because `LiveType` is declared SOMEWHERE, so a waiver could
                # keep pointing at a module that had been carved away and this
                # check would never say so — exactly the rot it exists to find,
                # one path segment up from where it was looking. Found by
                # review.
                prefix, _, leaf = needle.rpartition("::")
                if leaf in names and module_is_live(prefix, modules):
                    continue
            dead.setdefault(table, []).append(needle)
    sizes = {
        "waiver rows": sum(len(v) for v in rows.values()),
        "declared type names": len(names),
    }
    return dead, sizes


def main() -> int:
    dead, sizes = subjectless()
    thin = {k: v for k, v in sizes.items() if v < FLOORS[k]}
    if thin:
        print("FAIL: this check's own population collapsed, so its OK would mean nothing")
        for name, value in sorted(thin.items()):
            print(f"  {name}: {value} < floor {FLOORS[name]}")
        print(f"  ⇒ suspect the table parse in {COVERAGE.name}, not a clean tree")
        return 1

    if dead:
        print("FAIL: waiver rows whose subject is not declared anywhere in the tree:")
        for table, needles in sorted(dead.items()):
            for needle in needles:
                print(f"  {table}: {needle}")
        print(
            "  ⇒ Delete the row. It no longer waives anything, and it PRE-APPROVES any "
            "future type of that name — which is how a deleted module's waiver becomes a "
            "silent exemption for its replacement."
        )
        return 1

    print(
        f"ok: all {sizes['waiver rows']} rollback-coverage waiver row(s) across "
        f"{len(waiver_rows())} table(s) name a subject declared in the tree "
        f"({sizes['declared type names']} type names scanned)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
