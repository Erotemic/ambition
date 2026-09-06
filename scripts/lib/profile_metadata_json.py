#!/usr/bin/env python3
"""Turn a profiling bundle's `metadata.txt` into `metadata.json`.

The text file stays authoritative and human-readable; this is the machine-
readable face of the same facts, so a tool reading a bundle does not have to
re-implement the key=value parse (including the multi-line git status block).
"""

from __future__ import annotations

import json
import os
import sys


def parse_metadata(path: str) -> dict:
    data: dict[str, object] = {}
    dirty: list[str] = []
    in_status = False
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            line = line.rstrip("\n")
            if line == "git_status_porcelain_begin":
                in_status = True
                continue
            if line == "git_status_porcelain_end":
                in_status = False
                continue
            if in_status:
                if line.strip():
                    dirty.append(line)
                continue
            key, sep, value = line.partition("=")
            if sep:
                data[key] = value
    data["git_dirty_files"] = dirty
    data["git_clean"] = not dirty
    return data


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: profile_metadata_json.py <bundle-dir>", file=sys.stderr)
        return 2
    out_dir = argv[1]
    source = os.path.join(out_dir, "metadata.txt")
    if not os.path.exists(source):
        return 0
    data = parse_metadata(source)
    with open(os.path.join(out_dir, "metadata.json"), "w", encoding="utf-8") as handle:
        json.dump(data, handle, indent=2, sort_keys=True)
        handle.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
