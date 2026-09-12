#!/usr/bin/env python3
"""Attribute every occurrence of a symbol to the Rust `fn` that contains it, and
say whether that `fn` already takes a named parameter.

⭐⭐ WHY THIS EXISTS, AND IT IS NOT A GREP WRAPPER. Collapsing a second authority
in a Bevy codebase costs exactly two things: the number of USE SITES, and the
number of SYSTEM SIGNATURES that must gain access to the surviving owner. A flat
`git grep -c` answers the first and hides the second, and the second is what
decides whether a collapse is bounded or a sweep. Ordering a 65-hit grep by
enclosing function turned "somewhere between a patch and a rewrite" into
"27 rewrites, 4 signatures, one file".

⛔ IT ALSO SEPARATES THE POPULATIONS A COUNT MERGES. Sites inside `#[cfg(test)]`
follow the API for free; sites in helpers that take the value as a PARAMETER never
learn the fact moved at all; only sites that read it off a resource are real work.
Reporting one total for all three is how a carve gets sized at the wrong
granularity.

USAGE
    measure_symbol_sites_by_system.py <symbol> <file.rs> [--needs <param-regex>]

    --needs defaults to `pages:\\s*Res(Mut)?<`, the shape of a Bevy resource
    parameter. Any regex works; it is matched against the function's first 40
    lines, which is where a system's parameter list lives.

⚠ IT IS A HEURISTIC ON `fn` AT COLUMN ZERO, and it says so rather than pretending
otherwise: a nested `fn`, a `fn` inside an `impl` block indented four spaces, or a
macro-generated one is attributed to the enclosing column-zero item. For the
question it answers — which SYSTEM needs a parameter — that is the right
granularity, because systems are column-zero free functions in this tree. Check
the reported line numbers against the file before pricing anything.
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys

FN_AT_COLUMN_ZERO = re.compile(r"(?:pub\(crate\)\s+|pub\s+)?(?:async\s+)?fn\s+(\w+)")
COMMENT = re.compile(r"^\s*(//|/\*|\*)")


def functions(lines: list[str]) -> list[tuple[int, str]]:
    """Every column-zero `fn`, as (1-based line, name), plus an EOF sentinel."""
    found = [
        (i + 1, m.group(1))
        for i, line in enumerate(lines)
        if (m := FN_AT_COLUMN_ZERO.match(line))
    ]
    found.append((len(lines) + 1, "<eof>"))
    return found


def enclosing(fns: list[tuple[int, str]], line: int) -> tuple[int, str]:
    for start, name in reversed(fns[:-1]):
        if start <= line:
            return (start, name)
    # ⚠ A HIT ABOVE THE FIRST `fn` IS A FIELD OR A CONST, NOT A BUG: a struct
    # field declaration is exactly the thing a collapse deletes, so it is
    # reported under `<module>` rather than dropped.
    return (0, "<module>")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("symbol")
    ap.add_argument("path", type=pathlib.Path)
    ap.add_argument("--needs", default=r"pages:\s*Res(Mut)?<")
    ap.add_argument(
        "--window",
        type=int,
        default=40,
        help="how many lines of each fn to scan for --needs (its parameter list)",
    )
    args = ap.parse_args(argv)

    if not args.path.is_file():
        print(f"not a file: {args.path}", file=sys.stderr)
        return 2
    lines = args.path.read_text().splitlines()
    if not lines:
        # ⛔ AN EMPTY CORPUS MUST NOT PRINT A CLEAN BILL OF HEALTH.
        print(f"{args.path} is empty; nothing was measured", file=sys.stderr)
        return 2

    fns = functions(lines)
    needs = re.compile(args.needs)
    sites: dict[tuple[int, str], list[int]] = {}
    commented = 0
    for i, line in enumerate(lines, 1):
        if args.symbol not in line:
            continue
        if COMMENT.match(line):
            commented += 1
            continue
        sites.setdefault(enclosing(fns, i), []).append(i)

    if not sites:
        print(
            f"`{args.symbol}` appears in NO code line of {args.path} "
            f"({commented} comment mentions). Widen the symbol before concluding "
            "absence — a negative result here is a claim about the query.",
            file=sys.stderr,
        )
        return 1

    total = sum(len(v) for v in sites.values())
    print(f"{args.symbol} in {args.path}: {total} code sites, {commented} in comments\n")
    print(f"{'enclosing fn':44} {'has ' + args.needs:26} sites  lines")
    have = missing = 0
    for (start, name), at in sorted(sites.items()):
        # ⛔⛤ THE MODULE-LEVEL ROW IS A DECLARATION, NOT WORK, AND CLASSIFYING IT
        # AS "NEEDS THE PARAM" INFLATED THE COUNT BY ONE — found by this script's
        # own test, which is why it has one. `start` is 0 there, so the body slice
        # below would be `lines[-1:]`: the LAST line of the file, matched against
        # a parameter pattern it has nothing to do with. A struct field is deleted
        # by a collapse, never given a resource.
        if start == 0:
            print(f"{name:44} {'declaration, not a call site':26} {len(at):5}  {at[:8]}")
            continue
        end = next(s for s, _ in fns if s > start)
        body = "\n".join(lines[start - 1 : min(end - 1, start - 1 + args.window)])
        if needs.search(body):
            verdict, kind = "YES", "have"
        elif re.search(rf"{re.escape(args.symbol)}\s*:", body):
            verdict, kind = "takes-it-as-a-param", "param"
        else:
            verdict, kind = "NO", "missing"
        if kind == "have":
            have += len(at)
        elif kind == "missing":
            missing += len(at)
        print(f"{name:44} {verdict:26} {len(at):5}  {at[:8]}")
    print(
        f"\n{have} site(s) in functions that already have it; "
        f"{missing} site(s) in functions that would need it."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
