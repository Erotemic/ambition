#!/usr/bin/env python3
"""Probe selected Rust files for public functions with no call site.

This is a static cleanup aid, not a liveness proof: generated calls, macros, external
consumers, or dynamic registration may make an apparently unreferenced function
intentional. Results are candidates for review rather than automatic deletion."""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PUBLIC_FN = re.compile(r"^(\s*)(pub(?:\([a-z:\s]+\))?\s+)fn (\w+)")


# Consumers the root `Cargo.toml` EXCLUDES, which a `--workspace` build
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
    # brace-DEPTH scoped, not a latch: a `#[cfg(test)]` module ends, and a
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

        # THE ZERO GUARD. A probe that marked nothing finds everything "dead".
        if not marked:
            print(
                "\n⛔ marked ZERO functions. That is a failure, not a clean bill —"
                "\n   check the file paths and whether the public-fn pattern still"
                "\n   matches this file's style."
            )
            return 1

        # --workspace, never -p: `-p` cannot see a crate's dependents.
        #
        #  every sub-workspace consumer is built too, with the same markings,
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
