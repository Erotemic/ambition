#!/usr/bin/env python3
"""Compare area spec coordinates against a live LDtk file.

The spec under `tools/ambition_ldtk_tools/specs/*.yaml` is the
rebuildable source for a level (`area create --replace-existing`
rewrites the live level from the spec). In long-lived repos the
live LDtk gets edited / moved while the specs sit unchanged, so the
spec's `world_x` / `world_y` / `px_wid` / `px_hei` quietly drift
from the live values.

If you re-apply a drifted spec with `--replace-existing` the tool
obediently authors the level at the spec's stale coordinate and
leaves a duplicate-or-misplaced ghost level. The failure is silent:
tests still pass, validation still passes, but the LDtk editor view
looks weird and adjacent layout operations start overlapping.

This subcommand is the pre-flight check. It walks the named spec
files, finds each one's matching level in the live LDtk (by
`level_id`), and reports any field that disagrees.

Usage:

  python -m ambition_ldtk_tools level diff-specs \\
      --ldtk game/ambition_content/assets/worlds/intro.ldtk \\
      tools/ambition_ldtk_tools/specs/intro_*.yaml

Exit code is 0 if every named spec matches the live LDtk, 1 if any
spec diverges. Pipe through grep / wc -l in CI to enforce a clean
state before bulk re-applies.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[4]


def load_yaml(path: Path) -> dict:
    try:
        import yaml
    except ImportError as ex:  # pragma: no cover - tooling dependency
        raise SystemExit(
            f"PyYAML required to read {path}; "
            f"install with `pip install pyyaml` or include in repo deps. ({ex})"
        )
    return yaml.safe_load(path.read_text())


SPEC_FIELDS = ("world_x", "world_y", "px_wid", "px_hei")
LDTK_FIELDS_BY_SPEC = {
    "world_x": "worldX",
    "world_y": "worldY",
    "px_wid": "pxWid",
    "px_hei": "pxHei",
}


def diff_one(spec: dict, levels_by_id: dict) -> tuple[bool, list[str] | None]:
    """Compare one spec dict against the live LDtk levels.

    Returns (matches, diff_lines). `matches` is True iff every
    coordinate / size field equals the live level. `diff_lines` is
    None when the spec is not an area spec (no world_x / px_wid /
    etc. to diff); callers should treat that as a clean skip
    rather than a failure.
    """
    has_area_fields = any(spec.get(f) is not None for f in SPEC_FIELDS)
    if not has_area_fields:
        # Not an area spec (e.g. a door-add / entity-add spec). The
        # diff tool only owns area-spec coordinate drift; skip
        # quietly.
        return True, None
    level_id = spec.get("level_id") or spec.get("id")
    if not level_id:
        return False, [
            "spec has area-spec coordinates but no `level_id`; cannot match against LDtk."
        ]
    live = levels_by_id.get(level_id)
    if live is None:
        return False, [f"spec level_id {level_id!r} has no matching level in the LDtk."]
    diffs: list[str] = []
    for spec_field in SPEC_FIELDS:
        spec_value = spec.get(spec_field)
        if spec_value is None:
            continue
        live_field = LDTK_FIELDS_BY_SPEC[spec_field]
        live_value = live.get(live_field)
        if int(spec_value) != int(live_value):
            diffs.append(
                f"{level_id}.{spec_field}: spec={spec_value} live={live_value} "
                f"(live LDtk wins; update spec or re-apply with --replace-existing)"
            )
    return not diffs, diffs


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "specs",
        nargs="*",
        type=Path,
        help="One or more spec YAML paths to diff against the live LDtk.",
    )
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=REPO_ROOT
        / "crates"
        / "ambition_actors"
        / "assets"
        / "ambition"
        / "worlds"
        / "sandbox.ldtk",
        help="Path to the live .ldtk file (default: sandbox.ldtk).",
    )
    parser.add_argument(
        "--all",
        action="store_true",
        help=(
            "Diff every *.yaml spec under tools/ambition_ldtk_tools/specs/ "
            "(non-recursive). Convenient for a clean-state CI check."
        ),
    )
    args = parser.parse_args(argv)

    spec_paths: list[Path] = list(args.specs)
    if args.all:
        spec_dir = REPO_ROOT / "tools" / "ambition_ldtk_tools" / "specs"
        spec_paths.extend(sorted(spec_dir.glob("*.yaml")))
        spec_paths.extend(sorted(spec_dir.glob("*.yml")))
    if not spec_paths:
        parser.error("no specs to diff; pass spec paths or --all")

    project = json.loads(args.ldtk.read_text())
    levels_by_id = {lvl["identifier"]: lvl for lvl in project.get("levels", [])}

    all_match = True
    for spec_path in spec_paths:
        spec = load_yaml(spec_path)
        if spec is None:
            print(f"{spec_path}: empty or unreadable YAML")
            all_match = False
            continue
        matches, diffs = diff_one(spec, levels_by_id)
        if matches and diffs is None:
            # Not an area spec; quiet skip.
            print(f"SKIP: {spec_path.name} (not an area spec)")
            continue
        if matches:
            level_id = spec.get("level_id") or spec.get("id") or "?"
            live = levels_by_id.get(level_id, {})
            print(
                f"OK: {spec_path.name} ({level_id}) matches live "
                f"({live.get('worldX')},{live.get('worldY')}) "
                f"{live.get('pxWid')}x{live.get('pxHei')}"
            )
        else:
            print(f"DIFF: {spec_path}")
            for line in diffs or []:
                print(f"  - {line}")
            all_match = False
    return 0 if all_match else 1


if __name__ == "__main__":
    raise SystemExit(main())
