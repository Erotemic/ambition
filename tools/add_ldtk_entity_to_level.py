#!/usr/bin/env python3
"""Compatibility shim. Prefer:
    python -m ambition_ldtk_tools entity add ...
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _ldtk_shim_helper import forward

if __name__ == "__main__":
    raise SystemExit(
        forward(
            ["entity", "add"],
            "tools/add_ldtk_entity_to_level.py -- use `python -m ambition_ldtk_tools entity add` instead.",
        )
    )
