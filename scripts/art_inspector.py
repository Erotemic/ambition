#!/usr/bin/env python3
"""The old name of `scripts/asset_inspector.py`, which it runs with the same arguments."""

import os
import sys
from pathlib import Path

script = Path(__file__).with_name("asset_inspector.py")
os.execv(sys.executable, [sys.executable, str(script), *sys.argv[1:]])
