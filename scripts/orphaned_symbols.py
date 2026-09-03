#!/usr/bin/env python3
"""Public functions whose only callers are tests.

⛔ THE ROT THIS FINDS IS THE ONE A CITATION CHECKER STRUCTURALLY CANNOT SEE. A
carve that reroutes around a function but leaves it for the tests keeps every
doc naming it GREEN while making every sentence about it false. Found
2026-09-03: `6c9fb2b58` rerouted the quality convergence onto
`retire_realizations` and left `demote_stale_realizations` in place; three
planning sites still described it as the live mechanism and all three resolved.

⚠ THIS IS A CENSUS, NOT A DEAD-CODE LIST, and the difference matters:

* the engine is a LIBRARY, so a `pub fn` with no in-repo caller may be intended
  for downstream consumers;
* most of the 157 found on 2026-09-03 are legitimate (test-support helpers,
  builders, thin projections like `evaluate_character_ai` -> `..._output(x).mode`);
* it is a GREP, so trait-dispatched, macro-generated and re-exported calls are
  invisible to it.

⇒ Which is why there is deliberately NO ratchet here. The LEVEL is noise; the
DELTA across a carve is the signal. Run it before and after, diff the JSON:

    python3 scripts/orphaned_symbols.py --json > /tmp/before.json
    # ... carve lands ...
    python3 scripts/orphaned_symbols.py --json > /tmp/after.json
    python3 -c "import json,sys; a,b=[set(json.load(open(p))) for p in sys.argv[1:]]; print('\\n'.join(sorted(b-a)))" /tmp/before.json /tmp/after.json

Anything that appears is a symbol the carve just orphaned: grep the docs for it
before the sentences describing it go quietly false.

⭐ A SECOND SHAPE WORTH A LOOK (--overloads): a bare `fn` that production never
calls because a `_with_...` sibling took over, so the TESTS EXERCISE ONE OVERLOAD
AND THE PROGRAM ANOTHER. Twelve of the 157. Most are benign wrappers; one was
real -- `system_rows` vs `system_rows_with_quality_prompt`, where every test
passed `None` and the two rows the `Some` branch inserts mid-list had never been
built by anything.
"""

from __future__ import annotations

import argparse
import collections
import json
import pathlib
import re
import subprocess
import sys

DEF_RE = re.compile(r"^\s*pub(?:\([^)]*\))?\s+(?:async\s+)?fn\s+([a-z_][a-z0-9_]*)", re.M)
WORD_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
#: `///` and `//!` only. A NAME IN PROSE IS NOT A CALL, and leaving these in
#: made a function look live purely because its own doc block mentioned it.
DOC_RE = re.compile(r"^\s*//[/!].*$", re.M)


def repo_root() -> pathlib.Path:
    out = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True
    )
    return pathlib.Path(out.stdout.strip())


def is_test_path(path: str) -> bool:
    name = pathlib.Path(path).name
    return (
        name == "tests.rs"
        or name.endswith("_tests.rs")
        or "/tests/" in path
        or "/benches/" in path
    )


def census(root: pathlib.Path) -> tuple[dict[str, str], collections.Counter, collections.Counter]:
    listing = subprocess.run(
        ["git", "ls-files"], cwd=root, capture_output=True, text=True, check=True
    ).stdout.split()
    files = [f for f in listing if f.endswith(".rs")]

    definitions: dict[str, str] = {}
    for path in files:
        if is_test_path(path):
            continue
        for match in DEF_RE.finditer((root / path).read_text(errors="replace")):
            definitions.setdefault(match.group(1), path)

    names = set(definitions)
    production: collections.Counter = collections.Counter()
    tests: collections.Counter = collections.Counter()
    for path in files:
        text = DOC_RE.sub("", (root / path).read_text(errors="replace"))
        counts = collections.Counter(w for w in WORD_RE.findall(text) if w in names)
        target = tests if is_test_path(path) else production
        for name, n in counts.items():
            target[name] += n
    return definitions, production, tests


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="emit the bare name list")
    parser.add_argument(
        "--overloads",
        action="store_true",
        help="only the bare/_with_ splits: tests take one overload, production another",
    )
    args = parser.parse_args()

    root = repo_root()
    definitions, production, tests = census(root)

    # A definition is one production mention of ITSELF, so <= 1 means no caller.
    orphans = sorted(n for n in definitions if production[n] <= 1 and tests[n] >= 1)

    if args.overloads:
        rows = []
        for name in orphans:
            siblings = sorted(
                (m for m in definitions if m != name and m.startswith(name + "_") and production[m] > 1),
                key=lambda m: -production[m],
            )
            if siblings:
                rows.append((name, definitions[name], tests[name], siblings))
        if args.json:
            json.dump([r[0] for r in rows], sys.stdout, indent=1)
            print()
            return 0
        for name, path, n_tests, siblings in rows:
            print(f"{name}  ({path}, {n_tests} test mentions)")
            for sibling in siblings[:3]:
                print(f"    production takes {sibling}  ({production[sibling] - 1} mentions)")
        print(f"\n{len(rows)} of {len(orphans)} orphans are overload splits")
        return 0

    if args.json:
        json.dump(orphans, sys.stdout, indent=1)
        print()
        return 0

    by_crate: dict[str, list[str]] = collections.defaultdict(list)
    for name in orphans:
        parts = definitions[name].split("/")
        by_crate["/".join(parts[:2]) if len(parts) > 1 else parts[0]].append(name)
    for crate, names in sorted(by_crate.items(), key=lambda kv: (-len(kv[1]), kv[0])):
        print(f"{len(names):4d}  {crate}")
        print(f"        {', '.join(names)}")
    print(
        f"\n{len(orphans)} of {len(definitions)} pub fns have no production caller."
        "\n⚠ A CENSUS, NOT A DEFECT LIST -- the level is noise, the delta across a carve"
        "\n  is the signal. See this file's docstring before acting on a name."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
