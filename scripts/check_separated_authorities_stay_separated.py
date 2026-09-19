#!/usr/bin/env python3
"""A `LEGITIMATE_SEPARATION` is a standing claim about the tree, so hold it.

⛔⛤ **HALF THE CONSOLIDATION CENSUS'S FAMILIES CARRY THIS LABEL AND NOTHING
RE-DERIVED ANY OF THEM.** The other two states defend themselves better than
this one does. `RESOLVED` claims work happened, and
`check_collapsed_authorities_stay_collapsed.py` holds it by requiring the
deleted symbol to stay deleted. `OPEN_PRESSURE` asks a maintainer a question
and a question cannot go stale quietly.

`LEGITIMATE_SEPARATION` claims something different and more fragile: that two
similar-looking owners hold genuinely DIFFERENT facts, and that keeping them
apart is the architecture rather than an accident. Nothing re-checks that, and
ONE CONVENIENCE EDIT FALSIFIES IT — a derive added to make a type easier to
reach, a diagnostic read once for a decision. Both documents keep printing
`LEGITIMATE_SEPARATION` afterwards.

⇒ Two of the four reduce to a mechanical fact, and those two are here. The
other two are held by the census's dated review, which names what would change
them.

⚠ **WHAT THIS CANNOT DO, STATED SO A GREEN IS NOT READ AS MORE.** It holds the
two invariants below and says nothing about whether the separations are still
a good idea. A second owner arriving under a different name, or a diagnostic
that starts deciding something through a helper this does not follow, is
invisible here. The semantic half is the review in
`architecture-census.md`, and it is dated.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "scripts"))

import a_rollback_arm_must_refuse_a_frozen_world as frozen  # noqa: E402
import check_collapsed_authorities_stay_collapsed as collapsed  # noqa: E402

sys.path.insert(0, str(REPO / "scripts" / "lib"))
from test_paths import file_is_test_only, is_test_path, strip_test_modules  # noqa: E402

#: A type that must NOT carry a derive, and the family that breaks if it does.
#:
#: ⛔ `RollbackConfirmationState` is the sharpest row in the whole census: the
#: claim is one word. It is *"deliberately not a `Resource`"* — it is DERIVED
#: from `ActiveRollbackAuthority` for a requested session scope, so a stored
#: copy would be a second owner of the confirmation answer, free to disagree
#: with the authority it came from. Adding `Resource` to reach it more easily
#: is a one-word edit that reads like an ergonomics fix.
FORBIDDEN_DERIVE = {
    "RollbackConfirmationState": ("Resource", "DUP-ROLLBACK-CONFIRMATION"),
}

#: A type production may WRITE but must never READ, and the family it belongs
#: to. Writing a diagnostic is what a diagnostic is for; reading one back in
#: production is how it stops being one.
#:
#: ⚠ `LastConstructionVerification` is deliberately NOT here, and the asymmetry
#: is the honest part: it HAS two production reads and both are correct —
#: `transaction.rs` corrects a row only while it is still this room's, and
#: `dev_runtime.rs` builds a cosmetic message beside a decision taken
#: elsewhere. A rule that banned its reads would be wrong, and a rule loose
#: enough to allow them would not hold this one. Its half of the family stays
#: with the dated review.
WRITE_ONLY_IN_PRODUCTION = {
    "LastRoomConstructionCommit": "DUP-CONSTRUCTION-DIAGNOSTICS",
}

#: How a read is spelled. `insert_resource` is a write and `Res<T>` is not: the
#: patterns below are anchored so `insert_resource::<T>` cannot match as one.
READ_SPELLINGS = (
    r"\bRes\s*<\s*{name}\b",
    r"\bResMut\s*<\s*{name}\b",
    r"(?<!insert_)(?<!init_)\bresource\s*::\s*<\s*{name}\b",
    r"\bget_resource(?:_mut)?\s*::\s*<\s*{name}\b",
    r"\bremove_resource\s*::\s*<\s*{name}\b",
)


#: ⚠ MEASURED ON THIS CORPUS, NOT BORROWED. The first draft reused the sibling
#: guard's floor of 1700 and reddened on its own first run: that number is over
#: all 1,917 tracked `.rs` files, and this scan drops test files first, leaving
#: **1,309** production files on 2026-09-19. A floor is only meaningful against
#: the population it guards.
FLOOR = 1150


def production_sources() -> list[tuple[str, str]]:
    """Tracked Rust with test files dropped and `#[cfg(test)]` modules stripped.

    ⚠ THE CORPUS IS THE SIBLING GUARD'S, imported rather than respelled, so
    both checks over this census scan the same tree.
    """
    out = []
    for rel in collapsed._tracked_rust():
        if "/target/" in rel or rel.startswith(".worktrees"):
            continue
        path = REPO / rel
        try:
            raw = path.read_text(errors="replace")
        except OSError:
            continue
        if is_test_path(path) or file_is_test_only(raw):
            continue
        out.append((rel, frozen.code_only(strip_test_modules(raw))))
    return out


def declaration_derives(name: str, sources) -> tuple[str, list[str]] | None:
    """The derive list on `name`'s declaration, or `None` if it is not declared."""
    # ⛔⛤ THE VISIBILITY KEYWORD SITS BETWEEN THE ATTRIBUTES AND THE KEYWORD,
    # AND LEAVING IT OUT MADE THIS RULE TEST NOTHING. The first draft joined the
    # attribute group to `struct|enum` with `\s*`, so `#[derive(..)]\npub enum X`
    # never connected: the derive list parsed EMPTY for every `pub` type, and
    # the forbidden-derive poison PASSED. Found by running it, not by reading it.
    decl = re.compile(
        rf"((?:#\[[^\]]*\]\s*)*)"
        rf"(?:pub\s*(?:\([^)]*\)\s*)?)?"
        rf"\b(?:struct|enum)\s+{re.escape(name)}\b"
    )
    for rel, code in sources:
        match = decl.search(code)
        if match:
            return rel, re.findall(r"\b([A-Z]\w*)\b", match.group(1))
    return None


def production_reads(name: str, sources) -> list[str]:
    patterns = [re.compile(s.format(name=re.escape(name))) for s in READ_SPELLINGS]
    hits = []
    for rel, code in sources:
        if name not in code:
            continue
        for pattern in patterns:
            for match in pattern.finditer(code):
                hits.append(f"{rel}:{code.count(chr(10), 0, match.start()) + 1}")
    return sorted(set(hits))


def main() -> int:
    sources = production_sources()
    if len(sources) < FLOOR:
        print(
            f"⛔⛔ scanned {len(sources)} production Rust file(s), below the floor "
            f"of {FLOOR}; that is a claim about this scan."
        )
        return 1

    bad = []
    for name, (derive, family) in sorted(FORBIDDEN_DERIVE.items()):
        found = declaration_derives(name, sources)
        if found is None:
            # ⛔ ANTI-VACUITY PER ROW. A renamed or deleted subject makes this
            # rule pass by having nothing to test, which is the one way a
            # green here would be a lie.
            bad.append(f"`{name}` ({family}) is declared nowhere in production source")
            continue
        rel, derives = found
        if derive in derives:
            bad.append(
                f"`{name}` ({rel}) now derives `{derive}`, which makes it a stored "
                f"second owner of an answer it is supposed to DERIVE — {family} is no "
                "longer a legitimate separation"
            )

    for name, family in sorted(WRITE_ONLY_IN_PRODUCTION.items()):
        if declaration_derives(name, sources) is None:
            bad.append(f"`{name}` ({family}) is declared nowhere in production source")
            continue
        reads = production_reads(name, sources)
        if reads:
            bad.append(
                f"`{name}` ({family}) is read by production at {', '.join(reads)}; it is "
                "recorded as developer/test evidence, and a production read is how a "
                "diagnostic stops being one"
            )

    if bad:
        print("a census `LEGITIMATE_SEPARATION` no longer holds:")
        for line in bad:
            print(f"  {line}")
        return 1

    print(
        f"ok: {len(FORBIDDEN_DERIVE)} forbidden-derive and "
        f"{len(WRITE_ONLY_IN_PRODUCTION)} write-only separation(s) hold across "
        f"{len(sources)} production file(s)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
