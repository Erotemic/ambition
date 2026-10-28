#!/usr/bin/env python3
"""Check the selected runtime collection through accepted bundle evidence."""
import subprocess
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
if __name__ == "__main__":
    sys.exit(subprocess.call([str(ROOT / "scripts/regen/sprites.sh"), "--check", *sys.argv[1:]]))
