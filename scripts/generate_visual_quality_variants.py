#!/usr/bin/env python3
"""Generate quality products with the selected sprite toolchain."""
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools/ambition_sprite2d_renderer"))
from ambition_sprite2d_renderer.quality import main

if __name__ == "__main__":
    main()
