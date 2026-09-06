"""Shared post-write repair/validate helpers for mutating LDtk commands."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


def run_repair_and_validate(target: Path, schema: Path | None = None) -> int:
    """Run the standard LDtk repair pass followed by validation."""

    cmd = [
        sys.executable,
        "-m",
        "ambition_ldtk_tools.repair",
        str(target),
        "--in-place",
    ]
    print("$ " + " ".join(cmd))
    if subprocess.run(cmd).returncode != 0:
        return 1
    cmd = [sys.executable, "-m", "ambition_ldtk_tools.validate", str(target)]
    if schema and schema.exists():
        cmd.extend(["--schema", str(schema), "--require-schema"])
    print("$ " + " ".join(cmd))
    return subprocess.run(cmd).returncode
