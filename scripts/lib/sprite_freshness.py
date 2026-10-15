#!/usr/bin/env python3
"""Which targets of a sprite publish batch are fresh: one process for the batch.

`publish_cached_batch` in `scripts/regen/sprites.sh` asked each target in turn:
`unit_key` (a hash of its import closure and of the files its last render
read) and `sheet_output_digests` (a hash of each file it installs), each a few
`sha256sum` processes. The closure files that targets share were hashed again
for each target: 15 s of keys and 5 s of digests for 245 targets, measured
2026-10-15, of a 40 s freshness pass. This computes the same answers with each
file hashed once.

A target is fresh iff its stored key is its key now AND each file it installs
still hashes to what the cache recorded when it wrote them. The outputs are
not taken on trust: on 2026-08-29 `player_robot_v3_spritesheet.png` was
byte-identical to v2's, and a check that asked only whether the file existed
skipped it on every run.

⛔ THE KEY IS THE SAME BYTES AS `unit_key` IN `scripts/lib/sprite_cache_key.sh`.
A stored key is compared as text, so a key that differs in one byte makes every
sheet stale, and the next publish renders all of them again (about an hour).
`test_the_python_unit_key_is_the_bash_unit_key` holds the two together. On the
real tree, compare `--keys` with `unit_key` from the bash library.

Usage (from the renderer directory, as `sprites.sh` runs it):

    sprite_freshness.py --repo-root R --cache-dir C --sprites-dir S --core CORE \\
        --installs INSTALLS.tsv --portraits PORTRAITS.tsv [--force] TARGET...

`INSTALLS.tsv` and `PORTRAITS.tsv` are the `target<TAB>relative path` lines of
`sprite_install_names.py` and `portrait-files --with-target`. Prints
`fresh<TAB>target` or `stale<TAB>target` for each target, in order.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import sys
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Set, Tuple


class Hasher:
    """`sha256sum` of a file, each file read once per process."""

    def __init__(self) -> None:
        self._seen: Dict[Path, Optional[str]] = {}

    def digest(self, path: Path) -> Optional[str]:
        """The hex digest, or `None` when the file cannot be read (where
        `sha256sum` prints an error and nothing on stdout)."""
        if path not in self._seen:
            try:
                with open(path, "rb") as handle:
                    self._seen[path] = hashlib.file_digest(handle, "sha256").hexdigest()
            except OSError:
                self._seen[path] = None
        return self._seen[path]


def _sha256sum_lines(hasher: Hasher, root: Path, rels: Iterable[str]) -> str:
    """What `(cd root && sha256sum rels...)` prints on stdout."""
    out = []
    for rel in rels:
        digest = hasher.digest(root / rel)
        if digest is not None:
            out.append(f"{digest}  {rel}\n")
    return "".join(out)


def _c_sorted_unique(lines: Iterable[str]) -> List[str]:
    """`LC_ALL=C sort -u`: byte order, duplicates removed."""
    return sorted(set(lines), key=lambda line: line.encode())


def closures(closures_file: Path) -> Dict[str, List[str]]:
    """Each target's import closure, in file order (`target_closures.tsv`)."""
    out: Dict[str, List[str]] = {}
    try:
        text = closures_file.read_text()
    except OSError:
        return out
    for line in text.splitlines():
        parts = line.split("\t")
        if len(parts) >= 2:
            out.setdefault(parts[0], []).append(parts[1])
    return out


def hash_reads_text(hasher: Hasher, repo_root: Path, record: Path) -> str:
    """What `hash_reads RECORD` prints."""
    files: List[str] = []
    dirs: List[str] = []
    for line in record.read_text().splitlines():
        fields = line.split()
        if len(fields) >= 2 and fields[0] == "f":
            files.append(fields[1])
        elif len(fields) >= 2 and fields[0] == "d":
            dirs.append(fields[1])
    out = []
    present = []
    for rel in _c_sorted_unique(files):
        if (repo_root / rel).is_file():
            present.append(rel)
        else:
            out.append(f"missing {rel}\n")
    out.append(_sha256sum_lines(hasher, repo_root, present))
    for rel in _c_sorted_unique(dirs):
        out.append(f"directory {rel}\n")
        try:
            names = os.listdir(repo_root / rel)
        except OSError:
            names = []
        out.extend(f"{name}\n" for name in sorted(names, key=lambda name: name.encode()))
    return "".join(out)


def unit_key(
    hasher: Hasher,
    *,
    target: str,
    core: str,
    repo_root: Path,
    renderer_dir: Path,
    reads_dir: Path,
    closure_of: Dict[str, List[str]],
) -> Optional[str]:
    """`unit_key TARGET`, or `None` for a target with no read record (whose bash
    key is random, so no stored key can equal it)."""
    record = reads_dir / f"{target}.reads"
    if not record.is_file():
        return None
    leaf_input = _sha256sum_lines(hasher, renderer_dir, closure_of.get(target, []))
    leaf_input += hash_reads_text(hasher, repo_root, record)
    leaf = hashlib.sha256(leaf_input.encode()).hexdigest()
    return hashlib.sha256(f"{core}:{leaf}".encode()).hexdigest()


def output_digests(hasher: Hasher, sprites_dir: Path, rels: List[str]) -> Optional[Set[Tuple[str, str]]]:
    """`sheet_output_digests`, as a set of `(digest, rel)`: `None` when no
    declared output is present (where the bash function fails)."""
    present = [rel for rel in rels if rel and (sprites_dir / rel).is_file()]
    if not present:
        return None
    out = set()
    for rel in present:
        digest = hasher.digest(sprites_dir / rel)
        if digest is not None:
            out.add((digest, rel))
    return out


def stored_entry(path: Path) -> Tuple[Optional[str], Set[Tuple[str, str]]]:
    """A cache entry's key line and its digest block."""
    try:
        lines = path.read_text().splitlines()
    except OSError:
        return None, set()
    if not lines:
        return None, set()
    digests = set()
    for line in lines[1:]:
        digest, _, rel = line.partition(" ")
        if rel:
            digests.add((digest, rel))
    return lines[0], digests


def records_of(tsv: Optional[Path]) -> Dict[str, List[str]]:
    out: Dict[str, List[str]] = {}
    if tsv is None:
        return out
    for line in tsv.read_text().splitlines():
        target, _, rel = line.partition("\t")
        if target:
            out.setdefault(target, []).append(rel)
    return out


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--renderer-dir", type=Path, default=Path.cwd())
    parser.add_argument("--cache-dir", type=Path, required=True)
    parser.add_argument("--sprites-dir", type=Path, required=True)
    parser.add_argument("--core", required=True, help="the shared core fingerprint")
    parser.add_argument("--installs", type=Path, default=None)
    parser.add_argument("--portraits", type=Path, default=None)
    parser.add_argument("--portraits-failed", action="store_true", help="the portrait records could not be resolved")
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--keys", action="store_true", help="print `target<TAB>unit key` instead")
    parser.add_argument("targets", nargs="*")
    args = parser.parse_args(argv)

    hasher = Hasher()
    closure_of = closures(args.cache_dir / "target_closures.tsv")
    installs = records_of(args.installs)
    portraits = records_of(args.portraits)
    for target in args.targets:
        key = unit_key(
            hasher,
            target=target,
            core=args.core,
            repo_root=args.repo_root,
            renderer_dir=args.renderer_dir,
            reads_dir=args.cache_dir / "reads",
            closure_of=closure_of,
        )
        if args.keys:
            print(f"{target}\t{key or ''}")
            continue
        fresh = not args.force and not args.portraits_failed and key is not None
        if fresh:
            stored_key, stored_digests = stored_entry(args.cache_dir / "sheets" / target)
            now = output_digests(hasher, args.sprites_dir, installs.get(target, []))
            fresh = stored_key == key and now is not None and bool(stored_digests) and stored_digests == now
        if fresh:
            fresh = all((args.sprites_dir / rel).is_file() for rel in portraits.get(target, []) if rel)
        print(f"{'fresh' if fresh else 'stale'}\t{target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
