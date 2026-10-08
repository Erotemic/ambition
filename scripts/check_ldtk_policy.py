#!/usr/bin/env python3
"""Every LDtk world in the map submodule passes the authoring policy.

`ambition_ldtk_tools policy check` existed and no gate ran it, so the policy
drifted unseen: on 2026-10-08 five CameraZones (four in `sandbox.ldtk`, one in
the generated Hall) sat on `Ambition` instead of `AmbitionCameras`. This runs
the same check (`edit.policy.collect_policy_issues` with the default rules)
over every `.ldtk` under `game/ambition_map_assets`.

Run it with the LDtk tool's interpreter, which has the package:
`scripts/run_tests.py` does (`tool_python`).
"""

from __future__ import annotations

import argparse
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "roots",
        nargs="*",
        type=pathlib.Path,
        default=[REPO / "game" / "ambition_map_assets"],
        help="Directories or .ldtk files to check (default: the map submodule).",
    )
    args = parser.parse_args(argv)

    from ambition_ldtk_tools.edit.policy import collect_policy_issues, parse_rules
    from ambition_ldtk_tools.ldtk import format_issue_lines, load_project

    files: list[pathlib.Path] = []
    for root in args.roots:
        files.extend(sorted(root.rglob("*.ldtk")) if root.is_dir() else [root])
    if not files:
        # An empty corpus is a failure: the check was aimed at nothing.
        print(f"no .ldtk world under {[str(r) for r in args.roots]}", file=sys.stderr)
        return 2

    rules = parse_rules([], include_defaults=True)
    failed = 0
    for path in files:
        issues = [i for i in collect_policy_issues(load_project(path), rules) if i.severity == "error"]
        print(f"  {path.relative_to(REPO) if path.is_relative_to(REPO) else path}: {len(issues)} error(s)")
        if issues:
            print(format_issue_lines(issues, title="    policy issues:", empty=""), end="")
        failed += bool(issues)
    if failed:
        print(f"{failed} world(s) break the LDtk authoring policy; `ambition_ldtk_tools policy fix --in-place <world>` moves each entity to its layer")
        return 1
    print(f"{len(files)} world(s) checked; each passes the LDtk authoring policy.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
