#!/usr/bin/env python3
"""**Which public functions in these files have NO call site?** — asked of the
compiler, never of a grep (ledger D105).

Marks every public `fn` in the given files with
`#[deprecated(note = "PROBE_<name>")]`, builds the workspace, and reports the
probe names that never appear in a warning. Those have no caller the compiler can
see — through re-exports, macros, trait impls and cfgs alike. The files are
ALWAYS restored, including when the build fails.

⛔⛔ **THIS EXISTS BECAUSE GREP-BASED CENSUSES HAVE BEEN WRONG SEVEN TIMES IN ONE
RUN**, and every failure had the same shape: reporting an absence that had never
been looked for. Three of them were in earlier hand-rolled versions of THIS
probe, and each is designed out below:

* **`-p <crate>` compiles a crate's DEPENDENCIES, never its DEPENDENTS.** A
  single-package check named five dead functions in `features/enemies`;
  `--workspace` named three. `from_prepared_specs` — called from
  `ambition_content` — was one it would have deleted. ⇒ this always builds the
  whole workspace, and refuses a `-p` shortcut.
* **A `#[cfg(test)]` skip that LATCHES marks nothing after the first one.** A
  flag set and never cleared meant zero functions were marked in a 1,500-line
  file, and the run then reported 21 as dead — including `spec_for_brain`, which
  has callers in three crates. ⇒ the skip is brace-depth scoped, and…
* **…a probe that marked NOTHING reported everything as dead.** ⇒ marking zero
  items is a hard failure. A count of zero is never a clean bill.
* **`--workspace` CANNOT SEE A CONSUMER THE WORKSPACE EXCLUDES**, and this tool
  walked into that on 2026-08-12 — its own eighth trap. Run over
  `features/enemies/mod.rs` it reported exactly one dead function,
  `CharacterRosterFragment::from_ron_at`, whose only caller is
  `fixtures/external_consumer`: `exclude`d in the root manifest, and the only
  in-repo consumer that links the engine from OUTSIDE a shared workspace, which
  is the entire population a public-API census is about. That function had
  already been deleted once for this reason and restored the same morning;
  trusting the output would have deleted it again. ⇒ every excluded consumer is
  built too, with the same markings, and the warnings are UNIONED. It now reports
  25 of 25 on that file, and the case that produced the false positive is the
  case that validates the fix.

⚠ **"no call site" is not "delete it".** Test-only API that pins an
architectural invariant is not dead code, it is the invariant's only witness —
`CharacterRoster::fallback_for_provider` looked dead and is the observation seam
for a cross-provider isolation test. Read every hit before removing it.

⚠ **struct FIELDS are outside this instrument.** `#[deprecated]` does not
usefully cover them; a dead field surfaces as `cargo check`'s "never read", but
only once every reader is gone — which is why fields tend to fall out one
deletion AFTER their accessor.

Usage::

    python3 scripts/probe_dead_public_fns.py crates/foo/src/bar.rs [more.rs ...]
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PUBLIC_FN = re.compile(r"^(\s*)(pub(?:\([a-z:\s]+\))?\s+)fn (\w+)")


# **Consumers the root `Cargo.toml` EXCLUDES**, which a `--workspace` build
# therefore cannot see. They are the point of a public-API census, not an
# afterthought: each one links the engine the way a stranger does, with its own
# workspace, lockfile and feature resolution.
EXCLUDED_CONSUMERS = ("fixtures/external_consumer",)


def mark(path: str) -> tuple[str, list[str]]:
    """Return `(original text, marked names)` and write the marked file."""
    original = open(path, encoding="utf-8").read()
    lines = original.split("\n")
    out: list[str] = []
    names: list[str] = []
    # ⛔ brace-DEPTH scoped, not a latch: a `#[cfg(test)]` module ends, and a
    # flag that never clears silently un-marks the whole rest of the file.
    test_depth: int | None = None
    depth = 0
    for i, line in enumerate(lines):
        if test_depth is None and line.strip().startswith("#[cfg(test)]"):
            test_depth = depth
        match = PUBLIC_FN.match(line)
        in_test = test_depth is not None
        already = i > 0 and lines[i - 1].strip().startswith("#[deprecated")
        if match and not in_test and not already:
            out.append(f'{match.group(1)}#[deprecated(note = "PROBE_{match.group(3)}")]')
            names.append(match.group(3))
        out.append(line)
        depth += line.count("{") - line.count("}")
        if test_depth is not None and depth <= test_depth:
            test_depth = None
    open(path, "w", encoding="utf-8").write("\n".join(out))
    return original, names


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", nargs="+", help="Rust source files to probe")
    args = parser.parse_args()

    originals: dict[str, str] = {}
    marked: list[tuple[str, str]] = []
    try:
        for path in args.files:
            original, names = mark(path)
            originals[path] = original
            marked.extend((os.path.basename(path), name) for name in names)
            print(f"{os.path.basename(path):34s} marked {len(names)}")

        # ⛔ THE ZERO GUARD. A probe that marked nothing finds everything "dead".
        if not marked:
            print(
                "\n⛔ marked ZERO functions. That is a failure, not a clean bill —"
                "\n   check the file paths and whether the public-fn pattern still"
                "\n   matches this file's style."
            )
            return 1

        # ⛔ --workspace, never -p: `-p` cannot see a crate's dependents.
        #
        # ⛔⛔ **AND `--workspace` CANNOT SEE A CONSUMER THE WORKSPACE EXCLUDES**
        # — the eighth trap, and this tool walked straight into it on
        # 2026-08-12. Run over `features/enemies/mod.rs` it reported exactly one
        # dead function, `CharacterRosterFragment::from_ron_at`. That function
        # had been deleted for the same reason six days earlier, restored the
        # same morning, and its only caller is `fixtures/external_consumer` —
        # `exclude`d in the root `Cargo.toml`, and the ONLY in-repo consumer that
        # links the engine from outside a shared workspace, which is precisely
        # the population a public-API census is about. Trusting the output would
        # have deleted it a second time.
        #
        # ⇒ every sub-workspace consumer is built too, with the same markings,
        # and the warnings are UNIONED. A crate that has to be asked separately
        # is exactly the crate a census forgets.
        builds = [(REPO, ["cargo", "check", "--workspace", "--all-targets"])]
        for consumer in EXCLUDED_CONSUMERS:
            directory = os.path.join(REPO, consumer)
            if os.path.exists(os.path.join(directory, "Cargo.toml")):
                builds.append((directory, ["cargo", "check", "--all-targets"]))
            else:
                print(f"⚠ excluded consumer {consumer} is not present; not probed")

        seen: set[str] = set()
        for directory, command in builds:
            result = subprocess.run(
                command, cwd=directory, capture_output=True, text=True
            )
            output = result.stdout + result.stderr
            if "error" in output and "PROBE_" not in output:
                print(f"\n⛔ the build in {directory} FAILED, so no probe result is meaningful:")
                print("\n".join(output.splitlines()[-25:]))
                return 1
            seen |= set(re.findall(r"PROBE_(\w+)", output))
    finally:
        for path, original in originals.items():
            open(path, "w", encoding="utf-8").write(original)

    dead = [(f, n) for f, n in marked if n not in seen]
    print(f"\n{len(marked) - len(dead)} of {len(marked)} have a call site.")
    if not dead:
        print("No public fn in these files is unreferenced.")
        return 0
    print("\nNO CALL SITE IN THE WORKSPACE OR ANY EXCLUDED CONSUMER:")
    for file_name, name in dead:
        print(f"  {file_name:34s} {name}")
    print(
        "\n⚠ read each one before deleting. Test-only API that pins an"
        "\n  architectural invariant is the invariant's only witness, not dead code."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
