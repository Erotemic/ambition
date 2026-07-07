#!/usr/bin/env python3
"""Normalize a Rust crate's module layout to the `<mod>/mod.rs` convention.

Rust supports two equivalent layouts for a module `foo` that has submodules:

  (A) sidecar style   :  foo.rs        + foo/bar.rs        (foo.rs declares `mod bar;`)
  (B) mod.rs style    :  foo/mod.rs    + foo/bar.rs

Both resolve identically (a submodule `bar` of `foo` lives at `foo/bar.rs` either
way), so converting (A) -> (B) is a pure file move with ZERO source edits: for every
`foo.rs` that has a sibling `foo/` directory, `git mv foo.rs foo/mod.rs`.

This repo standardizes on (B) (the `mod.rs` style) — agents reach for it and the
owner prefers it. This script performs A->B across a crate, recursively.

Usage:
    python3 normalize_rust_crate.py <crate-or-src-dir> [more dirs ...] [--dry-run]
    python3 normalize_rust_crate.py crates/ambition_actors/src --dry-run

Notes:
- `lib.rs` and `main.rs` are crate roots, never sidecars — always skipped.
- Files under a `bin/` directory are separate binary roots — skipped.
- Idempotent: an already-normalized crate yields no moves.
- Uses `git mv` so history follows the file. Run from inside the git repo.
- The inverse (B->A) is intentionally NOT implemented; if ever needed it is the
  symmetric move (`git mv foo/mod.rs foo.rs` when foo/ has other contents).
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys

SKIP_STEMS = {"lib", "main"}


def find_pairs(root: str, exclude: frozenset[str] = frozenset()) -> list[tuple[str, str]]:
    """Return [(sidecar_rs, dest_mod_rs)] for every `foo.rs` with a sibling `foo/`.

    `exclude` is a set of module/dir names to leave untouched: any `<name>.rs`
    sidecar is skipped AND the walk does not descend into a `<name>/` directory
    (useful to fence off a module another worker owns).
    """
    pairs: list[tuple[str, str]] = []
    for dirpath, dirnames, filenames in os.walk(root):
        # Don't descend into a `bin/` dir's binary roots, or excluded subtrees.
        if os.path.basename(dirpath) == "bin":
            dirnames[:] = []
            continue
        dirnames[:] = [d for d in dirnames if d not in exclude]
        dirset = set(dirnames)
        for fn in filenames:
            if not fn.endswith(".rs"):
                continue
            stem = fn[:-3]
            if stem in SKIP_STEMS or stem in exclude:
                continue
            if stem in dirset:  # sibling directory `stem/` exists
                sidecar = os.path.join(dirpath, fn)
                dest = os.path.join(dirpath, stem, "mod.rs")
                if os.path.exists(dest):
                    print(f"  SKIP (dest exists): {sidecar} -> {dest}", file=sys.stderr)
                    continue
                pairs.append((sidecar, dest))
    # Deepest paths first so nested conversions never race a parent's move.
    pairs.sort(key=lambda p: p[0].count(os.sep), reverse=True)
    return pairs


def git_mv(src: str, dst: str) -> None:
    subprocess.run(["git", "mv", src, dst], check=True)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("dirs", nargs="+", help="crate root(s) or src dir(s) to normalize")
    ap.add_argument("--dry-run", action="store_true", help="print moves, change nothing")
    ap.add_argument("--exclude", action="append", default=[], metavar="NAME",
                    help="module/dir name to leave untouched (repeatable)")
    args = ap.parse_args()

    exclude = frozenset(args.exclude)
    total = 0
    for d in args.dirs:
        # Accept either a crate dir (use its src/) or a src dir directly.
        root = os.path.join(d, "src") if os.path.isdir(os.path.join(d, "src")) else d
        if not os.path.isdir(root):
            print(f"not a directory: {root}", file=sys.stderr)
            return 2
        pairs = find_pairs(root, exclude)
        print(f"# {root}: {len(pairs)} sidecar module(s) to normalize")
        for src, dst in pairs:
            print(f"  {'(dry) ' if args.dry_run else ''}git mv {src} {dst}")
            if not args.dry_run:
                git_mv(src, dst)
            total += 1
    print(f"# {'would move' if args.dry_run else 'moved'} {total} file(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
