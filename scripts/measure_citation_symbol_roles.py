#!/usr/bin/env python3
"""Count the "population-overstating" citation class — AND REFUTE IT.

⛔⛔ **THIS SCRIPT'S RESULT IS THAT THE RULE IT MEASURES SHOULD NOT BE BUILT.**
It is committed so the proposal is not made a second time.

THE PROPOSED RULE. `--roles` reports a citation that lands inside a
`#[cfg(test)]` region, and the mixed class needs a person to read the prose. Two
findings on 2026-09-10 shared a narrower shape that looked machine-checkable:

  * `open-world-runtime-and-residency.md:623` claims *"every consumer goes
    through it"* and cites `conversion/mod.rs:1603` for `kinematic_path_aliases`
    — a test.
  * `simulation-authority-and-determinism.md:765` lists
    `ambition_match/src/seating.rs:248` as a `smash.match_scoped` site — a test
    function's name, the key's only occurrence in that crate.

⇒ Both rows put a SYMBOL next to a CITATION, so: *report a doc line carrying
both, where every occurrence of that symbol in the cited file is inside a
`#[cfg(test)]` region.*

## WHY IT DOES NOT WORK — three measurements, each killing a version

**1. Pair symbol to citation by LINE: 11 hits, and 3 are tautologies.** When the
symbol IS a test function's name, "every occurrence is inside a test" is true by
construction. A doc naming the test it relies on is the deliberate case, not the
defect.

**2. After excluding tautologies: 8 hits, and 6 are CROSS-PAIRING.** A long
markdown table row carries several citations and many symbols, and pairing every
symbol with every citation on the line manufactures pairs neither the author nor
the reader would connect. MEASURED: `smash-parity-inventory.md:766` cites
`hit_response.rs:98` for `HitReaction` — correctly — and `hit_reaction.rs:293`
for a different claim. `hit_reaction.rs` has its `#[cfg(test)]` at 472, so **the
citation is production and the row is right**; the rule accused it anyway.

**3. Require the symbol within 40 characters of the citation: 2 hits, and it
LOSES one of the two cases that motivated it.** `smash.match_scoped` sits far
from `seating.rs:248` in its table row. Of the two survivors, one is the known
`kinematic_path_aliases` row and the other is a FALSE POSITIVE:
`george_grab_dash` occurs only in a test of `george_booul_moveset.rs` because
the running form is DERIVED by `dash_stance_verb` rather than authored — which
is exactly what that row says.

⇒ **The rule cannot both catch its two motivating examples and avoid
cross-pairing.** Adjacency is what separates a real pair from a manufactured
one, and adjacency is what one of the two real rows does not have.

⭐ **THE HONEST SUMMARY: a population of one, which is a positive control and
nothing else.** A guard written for its own example sits green forever and tells
nobody anything.

## ⛔⛔ AND THE DECISIVE OBJECTION IS STRUCTURAL, NOT ABOUT POPULATION SIZE

The three measurements above kill three VERSIONS of the rule. Yardrat killed the
rule itself, from the other end, by asking a better question: **not "how many
rows match" but "does a CORRECT row also match".** It does.

    docs/planning/tracks.md:125   -- a row that deliberately names its test
      `lifecycle/continuity.rs:674` (`a_consumed_occurrence_is_not_resurrected_by_a_placement`)
      a line citation                                            YES
      an adjacent backticked symbol                              YES
      the symbol's only occurrence in the cited file is a test   YES

⇒ **A row that deliberately names a test function writes it in backticks beside
the citation. So does a row that mis-cites a test as production. THE SHAPE IS
HOW HUMANS WRITE BOTH.** Prose is the only thing that separates them, which is
the conclusion `--roles` already reached — reached again from the opposite
direction by a rule built specifically to need no prose.

⚠ The tautology arm below happens to exclude THAT row, because its symbol is the
test's own `fn` name. **It does not answer the objection.** A deliberate row can
name a production symbol whose only use in the cited file is a test — which is
exactly the shape of `kinematic_path_aliases`, the surviving "real" hit.

⭐ **TWO INDEPENDENT ARGUMENTS FOR THE SAME DESIGN, FROM OPPOSITE ENDS**, is
worth more than two runs of one instrument agreeing. The class is real — two
rows prove it — but it is found by READING, and `--roles` already narrows the
reading to fifteen rows.

Run: python3 scripts/measure_citation_symbol_roles.py [--adjacent]
"""
from __future__ import annotations

import argparse
import importlib.util
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
_spec = importlib.util.spec_from_file_location(
    "_c", REPO / "scripts" / "check_planning_citations.py"
)
_m = importlib.util.module_from_spec(_spec)
sys.modules["_c"] = _m
_spec.loader.exec_module(_m)

#: A backticked token long enough to be a symbol or an authored key.
SYM = re.compile(r"`([A-Za-z_][A-Za-z0-9_]{3,}(?:[.:][A-Za-z0-9_]+)*)`")
NOISE = {"cfg", "test", "cite", "true", "false", "None", "Some", "self"}
#: How close a symbol must sit to a citation to be plausibly its subject.
ADJACENT_CHARS = 40


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--adjacent", action="store_true",
                    help="require the symbol within %d chars of the citation "
                         "(measurement 3)" % ADJACENT_CHARS)
    args = ap.parse_args()

    by_suffix: dict[str, list[str]] = {}
    for rel in _m.repo_files():
        by_suffix.setdefault(pathlib.Path(rel).name, []).append(str(rel))

    regions: dict[str, list[tuple[int, int]]] = {}
    source: dict[str, list[str]] = {}

    hits, tautologies, pairs = [], [], 0
    for doc in sorted((REPO / "docs").rglob("*.md")):
        for n, line in enumerate(doc.read_text(errors="replace").splitlines(), 1):
            cites = list(_m.FILE_LINE.finditer(line))
            if not cites:
                continue
            on_line = {s for s in SYM.findall(line)
                       if s not in NOISE
                       and not re.search(r"\.(rs|py|md|ron|toml|sh|json)$", s)}
            for mo in cites:
                path = mo.group(1)
                found = sorted(p for p in by_suffix.get(pathlib.Path(path).name, [])
                               if p.endswith(path))
                if len(found) != 1:
                    continue
                rel = found[0]
                if rel not in regions:
                    regions[rel] = _m.cfg_test_regions(REPO / rel)
                    source[rel] = (REPO / rel).read_text(errors="replace").splitlines()
                if not regions[rel]:
                    continue
                subjects = on_line
                if args.adjacent:
                    lo = max(0, mo.start() - ADJACENT_CHARS)
                    hi = min(len(line), mo.end() + ADJACENT_CHARS)
                    subjects = {s for s in SYM.findall(line[lo:hi]) if s in on_line}
                for sym in subjects:
                    bare = sym.split("::")[-1].split(".")[-1]
                    occ = [i + 1 for i, l in enumerate(source[rel]) if bare in l]
                    if not occ:
                        continue
                    pairs += 1
                    if not all(any(a <= o <= b for a, b in regions[rel]) for o in occ):
                        continue
                    if any(re.search(rf"\bfn\s+{re.escape(bare)}\b", source[rel][o - 1])
                           for o in occ):
                        tautologies.append((doc, n, sym))
                        continue
                    hits.append((doc, n, mo.group(0), sym, len(occ)))

    mode = "ADJACENT" if args.adjacent else "SAME LINE"
    print(f"pairing mode: {mode}")
    print(f"symbol/citation pairs in files that have test regions: {pairs}")
    print(f"\nevery occurrence of the symbol in the cited file is a test: {len(hits)}")
    for d, n, c, s, k in hits:
        print(f"   {d.relative_to(REPO)}:{n}")
        print(f"      cite {c}  symbol `{s}`  ({k} occurrence(s))")
    print(f"\nexcluded as TAUTOLOGIES (the symbol IS a test fn name): {len(tautologies)}")
    for d, n, s in tautologies:
        print(f"   {d.relative_to(REPO)}:{n}  `{s}`")
    print("\n⛔ READ THIS SCRIPT'S DOCSTRING BEFORE ACTING ON THESE ROWS. The rule "
          "they come from was measured and refuted; the counts are the evidence.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
