#!/usr/bin/env python3
from __future__ import annotations

"""Render every configured Blender character spritesheet.

This script is intended to be run from any working directory as long as this
`gen3d` directory is installed or is on PYTHONPATH.
"""

from gen3d_blender_lab.cli import app

if __name__ == "__main__":
    app(["draw-all"])
