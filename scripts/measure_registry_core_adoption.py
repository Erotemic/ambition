#!/usr/bin/env python3
"""Census: which `*Registry` types answer the four registry questions on purpose?

`ambition_registry_core` exists because 31 `*Registry` types were each deciding
independently what counts as identity, whether a second registration of the same
key is a no-op or a refusal, what enters a fingerprint, and whether a conflict
leaves the old registry untouched. Its crate doc states the escape hatch: a
registry whose policy is genuinely different "says so by not using `classify` —
and then has to say why in place."

⇒ So there are THREE states, not two, and the third is the one that matters:

    ADOPTED       manifest dependency AND a non-comment reference
    JUSTIFIED     names `registry_core` only in prose — the documented opt-out
    UNREFERENCED  no reference at all

⛔⛔ THE THIRD BUCKET USED TO BE CALLED `SILENT`, AND THE NAME ASSERTED SOMETHING
THIS SCRIPT DOES NOT MEASURE. Its docstring said "a registry in it is not a
registry that chose a different policy; it is one where the question was never
asked" — and that is false for at least two of the twenty-three. MEASURED
2026-09-07: `FrontendAudioRegistry` (`ambition_audio/src/selection.rs:201`) states
*"Later declarations of the same route replace earlier ones"*, and
`CombatBanterRegistry` (`ambition_conversation/src/banter.rs:37`) states
*"Overwrites any existing entry for that name"*. Both answered the question in
place, neither mentions this crate, and both sat in a bucket named for not having
answered.

⇒ What the script can see is a REFERENCE, so that is what the bucket is now named
for. Whether a registry stated its policy lives in a doc comment on its `register`
function, which no reliable grep reads — the 2026-09-02 inventory
(`docs/planning/triage/ambition-registry-core.md`) did it by hand, and its own
seven-row table had TWO stale rows when re-read on 2026-09-07.

⇒ **UNREFERENCED IS AN UPPER BOUND on work to do, not a list of it.** Read the
`register` function before treating a row as a task.

⛔ WHY THE MANIFEST CHECK IS NOT OPTIONAL. Grepping the source for
`registry_core` alone reports 8/31, and three of those hits are prose explaining
why the crate does NOT adopt. A crate whose manifest lacks the dependency cannot
be calling anything in it, so the manifest is what separates a use from a mention.
That over-count is this script's whole reason for existing.

⚠ AND THE COMMENT FILTER IS LINE-WISE, which is the honest limit: a doc comment
that wraps `registry_core` onto a line with no `//` prefix would read as code.
Checked by hand at the eight hits; re-check if the count moves without a diff.
"""

from __future__ import annotations

import pathlib
import re
import subprocess
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent
CRATE = "ambition_registry_core"


def depends_on_registry_core(crate_dir: pathlib.Path) -> bool:
    manifest = crate_dir / "Cargo.toml"
    if not manifest.exists():
        return False
    data = tomllib.load(manifest.open("rb"))
    return CRATE in set(data.get("dependencies", {}))


def classify_registries() -> dict[str, list[tuple[str, str]]]:
    out = subprocess.run(
        ["git", "grep", "-n", r"pub struct [A-Za-z]*Registry\b", "--", "*.rs"],
        cwd=ROOT, capture_output=True, text=True, check=False,
    ).stdout.splitlines()

    buckets: dict[str, list[tuple[str, str]]] = {"ADOPTED": [], "JUSTIFIED": [], "UNREFERENCED": []}
    for row in out:
        path, _, text = row.split(":", 2)
        if re.search(r"tests?\.rs$", path):
            continue
        name = re.search(r"pub struct (\w+)", text).group(1)
        crate_dir = ROOT / path.split("/src/")[0]
        src = (ROOT / path).read_text()
        code_use = any(
            CRATE in line and not line.strip().startswith(("//", "///", "//!", "*"))
            for line in src.splitlines()
        )
        crate = crate_dir.name
        if depends_on_registry_core(crate_dir) and code_use:
            buckets["ADOPTED"].append((name, crate))
        elif CRATE in src:
            buckets["JUSTIFIED"].append((name, crate))
        else:
            buckets["UNREFERENCED"].append((name, crate))
    return buckets


def main() -> int:
    b = classify_registries()
    total = sum(len(v) for v in b.values())
    print(f"{total} `*Registry` types outside tests")
    for state in ("ADOPTED", "JUSTIFIED", "UNREFERENCED"):
        print(f"\n{state}: {len(b[state])}")
        for name, crate in sorted(b[state]):
            print(f"  {name:36} {crate}")
    print(
        "\n⚠ UNREFERENCED means exactly that: no mention of `ambition_registry_core`."
        "\n  It does NOT mean the registry never decided its conflict policy — two of"
        "\n  them state it in place without naming this crate. Read the `register`"
        "\n  function before treating a row as work."
    )
    # An empty corpus prints a clean report; say so instead.
    if total == 0:
        print("\nNO `*Registry` TYPES FOUND — the pattern was renamed, or this ran "
              "outside the repo. This is not a clean result.", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
