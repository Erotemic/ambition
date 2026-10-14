#!/usr/bin/env python3
"""Name the samples of an Android profile, on a host that has no simpleperf.

    python3 scripts/symbolize_android_profile.py PROFILE_DIR [PROFILE_DIR ...] [--top N]

A profile from ``scripts/profile_android.sh`` has the device's report
(``simpleperf-device-basic.txt``: each sampled address and its share), and,
when the app library was stripped, no Rust names in it: an address of the app
is ``libambition_app.so[+OFFSET]``. The profile's ``symfs/`` has the
unstripped library with the same build id. This script names each app address
with ``llvm-symbolizer`` against that library, adds the shares up by function
and by crate, and prints a table for each profile, and a table of the change
between the first profile and each other one.

It needs ``llvm-symbolizer`` (any version; ``llvm-symbolizer-18`` is tried
too). Shares are of all samples of the app's process, all threads.
"""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from typing import Dict, List, Tuple

LINE = re.compile(r"^\s*([0-9.]+)%\s+(\S+)\s+(.*?)\s*$")
APP = re.compile(r"libambition_app\.so\[\+([0-9a-f]+)\]")


def symbolizer() -> str:
    for name in ("llvm-symbolizer", "llvm-symbolizer-18", "llvm-symbolizer-17", "llvm-symbolizer-19"):
        found = shutil.which(name)
        if found:
            return found
    sys.exit("no llvm-symbolizer on PATH")


def app_library(profile: Path) -> Path:
    found = sorted(profile.glob("symfs/**/libambition_app.so"))
    if not found:
        sys.exit(f"{profile}: no symfs/**/libambition_app.so (was the profile made with symfs on?)")
    return found[0]


def samples(profile: Path) -> List[Tuple[float, str, str]]:
    """``(share, shared object, symbol)`` of each line of the device's basic report."""
    out = []
    for line in (profile / "simpleperf-device-basic.txt").read_text(errors="replace").splitlines():
        match = LINE.match(line)
        if match:
            share, dso, symbol = match.groups()
            out.append((float(share), dso, symbol))
    return out


def name_addresses(library: Path, offsets: List[str]) -> Dict[str, str]:
    """The function at each address of the app library."""
    if not offsets:
        return {}
    proc = subprocess.run(
        [symbolizer(), f"--obj={library}", "--functions=linkage", "--no-inlines", "--demangle", "--output-style=GNU"],
        input="\n".join(f"0x{offset}" for offset in offsets) + "\n",
        capture_output=True,
        text=True,
        check=True,
    )
    names = [line for i, line in enumerate(proc.stdout.splitlines()) if i % 2 == 0]
    return {offset: (name or "??") for offset, name in zip(offsets, names)}


def short(name: str) -> str:
    """A function name without its generic arguments and hash."""
    name = re.sub(r"::h[0-9a-f]{16}$", "", name)
    depth, out = 0, []
    for ch in name:
        if ch == "<":
            depth += 1
            if depth == 1:
                out.append("<…>")
            continue
        if ch == ">":
            depth = max(0, depth - 1)
            continue
        if depth == 0:
            out.append(ch)
    return "".join(out)


def crate(name: str) -> str:
    """The crate a function is in, as far as its name says."""
    name = name.lstrip("<").lstrip("&").strip()
    match = re.match(r"([A-Za-z_][A-Za-z0-9_]*)::", name)
    if match:
        return match.group(1)
    if name.startswith("mi_") or name.startswith("_mi_"):
        return "mimalloc"
    return "(other)"


def table(profile: Path) -> Tuple[Dict[str, float], Dict[str, float], Dict[str, float]]:
    rows = samples(profile)
    library = app_library(profile)
    offsets = sorted({m.group(1) for _s, _d, symbol in rows if (m := APP.search(symbol))})
    names = name_addresses(library, offsets)
    by_function: Dict[str, float] = defaultdict(float)
    by_crate: Dict[str, float] = defaultdict(float)
    by_object: Dict[str, float] = defaultdict(float)
    for share, dso, symbol in rows:
        match = APP.search(symbol)
        obj = Path(dso).name if dso.startswith("/") else dso
        by_object[obj] += share
        if match:
            name = names.get(match.group(1), "??")
            by_function[short(name)] += share
            by_crate[crate(name)] += share
        else:
            by_function[f"[{obj}] {symbol[:60]}"] += share
    return by_function, by_crate, by_object


def show(title: str, values: Dict[str, float], top: int) -> None:
    print(f"\n### {title}")
    for name, share in sorted(values.items(), key=lambda kv: -kv[1])[:top]:
        print(f"{share:6.2f}%  {name}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("profiles", nargs="+", type=Path)
    parser.add_argument("--top", type=int, default=30)
    args = parser.parse_args()
    tables = []
    for profile in args.profiles:
        by_function, by_crate, by_object = table(profile)
        tables.append((profile, by_function, by_crate, by_object))
        print(f"\n## {profile.name}")
        show("shared objects", by_object, 12)
        show("crates of the app", by_crate, 20)
        show("functions", by_function, args.top)
    if len(tables) > 1:
        base_name, base_fn, base_crate, _ = tables[0]
        for profile, by_function, by_crate, _ in tables[1:]:
            print(f"\n## {profile.name} minus {base_name.name} (points of share)")
            for label, now, before in (("crates", by_crate, base_crate), ("functions", by_function, base_fn)):
                change = {k: now.get(k, 0.0) - before.get(k, 0.0) for k in set(now) | set(before)}
                show(f"{label} that grew", {k: v for k, v in change.items() if v > 0}, args.top if label == "functions" else 12)
                show(f"{label} that shrank", {k: -v for k, v in change.items() if v < 0}, 12)


if __name__ == "__main__":
    main()
