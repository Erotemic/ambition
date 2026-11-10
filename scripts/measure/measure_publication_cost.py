#!/usr/bin/env python3
"""Measure what one focused publication costs, against stores of two sizes.

ASSET-PUBLISH asks for a small and a many-bundle baseline of time, bytes read,
bytes hashed and stat calls. This builds a synthetic store of `--bundles`
accepted bundles (`--files` members of `--bytes` each), selects them, then
measures the two steps of a focused publication of ONE new unit:

* accept: copy the new unit's members into an immutable bundle;
* publish: select it onto the existing selection.

Then the explicit deep `audit` of the whole selection, which is where payload
digests are read now; before 2026-11-10 `publish` did that work itself.

Counters: wall time; bytes read by this process (`/proc/self/io` rchar, Linux
only, cached reads included); bytes passed to SHA-256 (including the small
JSON identities); calls to `os.stat` (every `Path.stat`, `is_file`, `exists`).

    python3 scripts/measure/measure_publication_cost.py --bundles 4
    python3 scripts/measure/measure_publication_cost.py --bundles 200
"""
from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import sys
import tempfile
import time

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "tools/ambition_sprite2d_renderer"))
from ambition_sprite2d_renderer import publication_store  # noqa: E402
from ambition_sprite2d_renderer.publication_store import Store  # noqa: E402


class Counters:
    def __init__(self):
        self.hashed = 0
        self.stats = 0

    def install(self):
        counters = self
        real_sha256, real_stat = hashlib.sha256, os.stat

        class Counted:
            def __init__(self, data=b""):
                self._inner = real_sha256()
                self.update(data)

            def update(self, data):
                counters.hashed += len(data)
                self._inner.update(data)

            def hexdigest(self):
                return self._inner.hexdigest()

        def stat(*args, **kwargs):
            counters.stats += 1
            return real_stat(*args, **kwargs)

        publication_store.hashlib.sha256 = Counted
        os.stat = stat
        return lambda: (setattr(publication_store.hashlib, "sha256", real_sha256), setattr(os, "stat", real_stat))


def rchar() -> int | None:
    try:
        return int(next(line for line in Path("/proc/self/io").read_text().splitlines() if line.startswith("rchar:")).split()[1])
    except (OSError, StopIteration):
        return None


def measure(label: str, step):
    counters = Counters()
    restore = counters.install()
    read, started = rchar(), time.perf_counter()
    try:
        result = step()
    finally:
        elapsed = time.perf_counter() - started
        restore()
    after = rchar()
    read_text = f"{(after - read) / 1e6:9.2f} MB read" if read is not None and after is not None else "  read n/a"
    print(f"{label:<28} {elapsed * 1e3:9.1f} ms  {read_text}  {counters.hashed / 1e6:9.2f} MB hashed  {counters.stats:7d} stat calls")
    return result


def members(directory: Path, unit: int, files: int, size: int) -> dict[str, tuple[Path, str, str]]:
    found = {}
    for index in range(files):
        logical = f"sprites/unit{unit:04d}/member{index:03d}.png"
        path = directory / logical
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(os.urandom(size))
        found[logical] = (path, "runtime", "full")
    return found


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--bundles", type=int, default=4)
    parser.add_argument("--files", type=int, default=8)
    parser.add_argument("--bytes", type=int, default=256 * 1024)
    args = parser.parse_args()

    with tempfile.TemporaryDirectory(prefix="publication-cost-") as temporary:
        root = Path(temporary)
        store = Store(root / "store")
        accepted = {f"target:unit{unit:04d}": store.accept(f"target:unit{unit:04d}",
                                                            members(root / "sources", unit, args.files, args.bytes), None)
                    for unit in range(args.bundles)}
        selection = store.publish(accepted, store.selected(), replace=True)
        total = args.bundles * args.files * args.bytes
        print(f"store: {args.bundles} selected bundles x {args.files} members x {args.bytes // 1024} KiB "
              f"= {total / 1e6:.1f} MB selected; one new unit of {args.files * args.bytes / 1e6:.1f} MB")

        new = members(root / "sources", args.bundles, args.files, args.bytes)
        fresh = Store(store.root)
        key = measure("accept one new unit", lambda: fresh.accept(f"target:unit{args.bundles:04d}", new, None))
        measure("publish it", lambda: Store(store.root).publish({f"target:unit{args.bundles:04d}": key}, selection))
        defects = measure("audit (explicit, deep)", lambda: Store(store.root).audit())
        assert not defects, defects

if __name__ == "__main__":
    main()
