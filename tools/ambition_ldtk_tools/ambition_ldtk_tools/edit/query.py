#!/usr/bin/env python3
"""Query entity instances in an LDtk level — read-only, prints a
table of matches so authors don't have to grep the JSON to find an
iid, position, or field value.

Common workflows this unblocks:

  * "where is the intro entry door?" — query the level for
    `identifier=LoadingZone` and read the px column.
  * "what's the iid I need for entity move/delete?" — query and
    read the iid column.
  * "how many EnemySpawns are in this room?" — count rows.

Examples (all read-only; no file is modified):

  # All entities in a level
  python -m ambition_ldtk_tools entity query \\
    --ldtk <ldtk> --level central_hub_basement

  # Only LoadingZones in a level
  python -m ambition_ldtk_tools entity query \\
    --ldtk <ldtk> --level central_hub_basement --identifier LoadingZone

  # LoadingZones whose target_room matches a value
  python -m ambition_ldtk_tools entity query \\
    --ldtk <ldtk> --level central_hub_basement \\
    --identifier LoadingZone --field target_room=intro_wake_room

  # Across every level (skip --level)
  python -m ambition_ldtk_tools entity query \\
    --ldtk <ldtk> --identifier EnemySpawn

  # Look up a known iid (anywhere in the project)
  python -m ambition_ldtk_tools entity query \\
    --ldtk <ldtk> --iid LoadingZone-4346

Default output is a compact text table. `--format json` emits a
structured JSON array suitable for piping into jq or another tool.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/query.py -> repo root.
REPO_ROOT = Path(__file__).resolve().parents[4]

from ambition_ldtk_tools.area_authoring import load_project  # noqa: E402


def _entity_field_dict(entity: dict) -> dict:
    out: dict[str, object] = {}
    for fi in entity.get("fieldInstances", []):
        ident = fi.get("__identifier")
        if ident is None:
            continue
        out[ident] = fi.get("__value")
    return out


def _entity_field_value(entity: dict, name: str):
    for fi in entity.get("fieldInstances", []):
        if fi.get("__identifier") == name:
            return fi.get("__value")
    return None


def _matches_field_filter(entity: dict, filters: list[tuple[str, str]]) -> bool:
    for fname, expected in filters:
        actual = _entity_field_value(entity, fname)
        # Compare as strings (CLI arg values are always strings); fall
        # through to exact equality if both sides are non-string.
        if isinstance(actual, str):
            if actual != expected:
                return False
        elif str(actual) != expected:
            return False
    return True


def _parse_field_filter(raw: str) -> tuple[str, str]:
    if "=" not in raw:
        raise SystemExit(f"--field expects NAME=VALUE; got {raw!r}")
    name, _, value = raw.partition("=")
    name = name.strip()
    value = value.strip()
    if not name:
        raise SystemExit(f"--field {raw!r} has an empty name")
    return name, value


def collect(
    project: dict,
    level_filter: str | None,
    identifier_filter: str | None,
    field_filters: list[tuple[str, str]],
    iid_filter: str | None,
) -> list[dict]:
    rows: list[dict] = []
    for level in project.get("levels") or []:
        level_id = level.get("identifier", "<unnamed>")
        if level_filter and level_id != level_filter:
            continue
        for layer in level.get("layerInstances") or []:
            for entity in layer.get("entityInstances") or []:
                iid = entity.get("iid", "<no-iid>")
                if iid_filter and iid != iid_filter:
                    continue
                identifier = entity.get("__identifier", "<unknown>")
                if identifier_filter and identifier != identifier_filter:
                    continue
                if field_filters and not _matches_field_filter(entity, field_filters):
                    continue
                rows.append(
                    {
                        "level": level_id,
                        "layer": layer.get("__identifier"),
                        "identifier": identifier,
                        "iid": iid,
                        "px": entity.get("px"),
                        "size": [entity.get("width"), entity.get("height")],
                        "fields": _entity_field_dict(entity),
                    }
                )
    return rows


def print_table(rows: list[dict], summary_fields: list[str] | None) -> None:
    if not rows:
        print("(no matches)")
        return
    # Pick a default set of summary fields per identifier — for
    # LoadingZone the id + target are the most useful, for NpcSpawn
    # it's name + dialogue_id, etc. The CLI flag overrides this.
    DEFAULTS = {
        "LoadingZone": ["id", "name", "activation", "target_room", "target_zone"],
        "NpcSpawn": ["name", "dialogue_id"],
        "EnemySpawn": ["name", "brain"],
        "Switch": ["id", "name", "action", "target_encounter"],
        "DebugLabel": ["name", "text"],
        "CameraZone": ["id", "name", "mode"],
        "PlayerStart": ["name"],
        "Prop": ["name", "kind"],
    }
    headers = ["level", "identifier", "iid", "px", "size", "summary"]
    widths = {h: len(h) for h in headers}
    formatted: list[list[str]] = []
    for row in rows:
        ident = row["identifier"]
        summary_keys = summary_fields or DEFAULTS.get(ident) or sorted(row["fields"])
        summary_bits: list[str] = []
        for key in summary_keys:
            if key in row["fields"]:
                summary_bits.append(f"{key}={row['fields'][key]!r}")
        summary = "  ".join(summary_bits)
        cols = [
            str(row["level"]),
            str(ident),
            str(row["iid"]),
            f"({row['px'][0]},{row['px'][1]})" if row["px"] else "?",
            f"{row['size'][0]}x{row['size'][1]}" if row["size"][0] is not None else "?",
            summary,
        ]
        formatted.append(cols)
        for h, c in zip(headers, cols):
            widths[h] = max(widths[h], len(c))
    header_line = "  ".join(h.ljust(widths[h]) for h in headers)
    print(header_line)
    print("-" * len(header_line))
    for cols in formatted:
        print("  ".join(c.ljust(widths[h]) for c, h in zip(cols, headers)))


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=REPO_ROOT
        / "crates"
        / "ambition_sandbox"
        / "assets"
        / "ambition"
        / "worlds"
        / "sandbox.ldtk",
    )
    parser.add_argument(
        "--level",
        default=None,
        help="restrict to one level identifier (default: all levels)",
    )
    parser.add_argument(
        "--identifier",
        default=None,
        help="restrict to one entity identifier (e.g. LoadingZone, NpcSpawn)",
    )
    parser.add_argument(
        "--field",
        action="append",
        default=[],
        help="filter by `name=value` (repeatable; combined with AND)",
    )
    parser.add_argument(
        "--iid",
        default=None,
        help="look up a single entity by iid (overrides other filters)",
    )
    parser.add_argument(
        "--summary",
        default=None,
        help=(
            "comma-separated list of field names to show in the summary "
            "column (default: per-identifier sensible picks)"
        ),
    )
    parser.add_argument(
        "--format",
        choices=["table", "json"],
        default="table",
    )
    args = parser.parse_args(argv)

    project = load_project(args.ldtk)
    field_filters = [_parse_field_filter(raw) for raw in args.field]
    rows = collect(
        project,
        args.level,
        args.identifier,
        field_filters,
        args.iid,
    )

    if args.format == "json":
        json.dump(rows, sys.stdout, indent=2)
        sys.stdout.write("\n")
        return 0

    summary_fields = (
        [s.strip() for s in args.summary.split(",") if s.strip()]
        if args.summary
        else None
    )
    print_table(rows, summary_fields)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
