#!/usr/bin/env python3
"""Compatibility shim. Prefer:
    python -m ambition_ldtk_tools area create ...
    python -m ambition_ldtk_tools door free-spots ...
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _ldtk_shim_helper import forward

if __name__ == "__main__":
    raise SystemExit(
        forward(
            ["area", "create"],
            "tools/author_ldtk_area.py -- use `python -m ambition_ldtk_tools area create ...` instead.",
        )
    )
