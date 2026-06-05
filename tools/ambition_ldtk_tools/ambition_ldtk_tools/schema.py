#!/usr/bin/env python3
"""Fetch LDtk's official JSON schema for Python jsonschema validation."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from urllib.request import urlopen

DEFAULT_URL = "https://ldtk.io/files/JSON_SCHEMA.json"
DEFAULT_OUTPUT = (
    Path(__file__).resolve().parent.parent / "schemas" / "ldtk" / "JSON_SCHEMA.json"
)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--url", default=DEFAULT_URL, help="LDtk schema URL")
    parser.add_argument(
        "--output", type=Path, default=DEFAULT_OUTPUT, help="Output schema path"
    )
    args = parser.parse_args(argv)

    try:
        with urlopen(args.url, timeout=30) as response:  # noqa: S310 - trusted explicit developer tooling URL
            data = response.read()
    except Exception as ex:  # noqa: BLE001
        print(f"error: failed to fetch {args.url}: {ex}", file=sys.stderr)
        return 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(data)
    print(f"wrote {len(data)} bytes to {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
