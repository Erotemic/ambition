#!/usr/bin/env python3
"""Check that the named planning control-plane documents still exist.

A past edit emptied status.md without causing another planning check to fail.
This check protects the document identities and their required sections. It does
not enforce document length: a resolved queue or question list may be short.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

# Named documents and headings that identify their purpose. A heading may have
# a descriptive suffix, such as "# The queue — live execution order".
LIVE_CONTROL_PLANE: dict[str, tuple[str, ...]] = {
    "docs/planning/queue.md": ("# The queue",),
    "docs/planning/status.md": ("# Planning status", "## Current execution"),
    "docs/planning/awaiting-maintainer-decision.md": (
        "# Awaiting a maintainer decision",
        "## What actually blocks architecture work today",
    ),
    "docs/planning/maintainer-decisions.md": ("# Maintainer decisions",),
    "docs/planning/README.md": ("# `docs/planning`",),
    "docs/planning/tracks.md": ("# Tracks",),
    "docs/planning/roadmap.md": ("# Roadmap", "## North star"),
    "docs/planning/consolidation/README.md": ("# Architecture consolidation",),
    "docs/planning/consolidation/consolidation-plan.md": (
        "# Architecture consolidation plan",
        "## Priority table",
    ),
    "docs/planning/consolidation/architecture-census.md": (
        "# Architecture consolidation census",
    ),
    "docs/planning/consolidation/campaign-metrics.md": (
        "# Consolidation campaign metrics",
    ),
}


def findings_for(root: Path, documents: dict[str, tuple[str, ...]]) -> list[str]:
    """Report missing documents, required headings, or empty document bodies."""
    findings: list[str] = []
    for rel, required in sorted(documents.items()):
        path = root / rel
        if not path.is_file():
            findings.append(f"  {rel}: file is missing")
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        headings = [line for line in lines if line.startswith("#")]
        missing = [heading for heading in required if not any(
            actual == heading or actual.startswith(heading + " ")
            for actual in headings
        )]
        if missing:
            findings.append(f"  {rel}: missing required heading(s): {missing}")
        # A stub containing only headings is not a useful control-plane page.
        # Do not require a fixed amount of prose or any outstanding tasks.
        if not any(line.strip() and not line.lstrip().startswith(("#", "<!--"))
                   for line in lines):
            findings.append(f"  {rel}: no body content")
    return findings


def main() -> int:
    findings = findings_for(REPO, LIVE_CONTROL_PLANE)
    if findings:
        print("Planning control-plane document(s) are missing or incomplete:\n"
              + "\n".join(findings))
        return 1
    print(f"ok: {len(LIVE_CONTROL_PLANE)} named planning documents have their "
          "required sections and body content")
    return 0


if __name__ == "__main__":
    sys.exit(main())
