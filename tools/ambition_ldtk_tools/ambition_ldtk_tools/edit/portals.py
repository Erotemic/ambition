#!/usr/bin/env python3
"""Author a LINKED portal PAIR in one command.

Two `Portal` entities pair iff their `color` fields are complementary channels.
This is the ergonomic front end: give it a channel + the two placements and it
emits BOTH ends with the right paired colors, a shared id prefix, and a shared
name, then repairs the file — so an author never has to remember the pairing
table or place two entities by hand.

    ambition-ldtk-tools portal pair \\
        --level portal_lab --channel purple \\
        --a 254 891 up --b 554 891 up \\
        --id demo --name "demo gate" --in-place

`--channel` accepts either member of a named pair (purple/yellow, teal/red,
green/magenta, cyan/rose) or a generated channel `cN` (paired with `cN^1`, i.e.
c8↔c9, c10↔c11, …) — there are 128 generated pairs, so a room can hold far more
than the four named ones. `--a`/`--b` are `X Y NORMAL`, NORMAL ∈
{up (floor), down (ceiling), left (right-wall), right (left-wall)} (world y is
down). Each end's box is the 92×18 portal face centered on `X,Y`.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

from ambition_ldtk_tools.area_authoring import (
    build_entity_instance,
    find_entity_def,
    load_project,
    write_project,
)
from ambition_ldtk_tools.edit.entities import find_ambition_layer, find_level

REPO_ROOT = Path(__file__).resolve().parents[4]
DEFAULT_LDTK = (
    REPO_ROOT
    / "crates"
    / "ambition_sandbox"
    / "assets"
    / "ambition"
    / "worlds"
    / "sandbox.ldtk"
)

# The named complementary pairs (mirrors PortalChannelColor::partner in Rust).
NAMED_PARTNER = {
    "purple": "yellow",
    "yellow": "purple",
    "teal": "red",
    "red": "teal",
    "green": "magenta",
    "magenta": "green",
    "cyan": "rose",
    "rose": "cyan",
}
NORMALS = {"up", "down", "left", "right"}


def partner_of(channel: str) -> str:
    """The complementary channel `channel` links to (mirrors the Rust side)."""
    if channel in NAMED_PARTNER:
        return NAMED_PARTNER[channel]
    if channel.startswith("c") and channel[1:].isdigit():
        idx = int(channel[1:])
        if 0 <= idx <= 255:
            return f"c{idx ^ 1}"
    raise SystemExit(
        f"unknown channel '{channel}'. Use a named pair "
        f"({', '.join(sorted(NAMED_PARTNER))}) or a generated 'cN' (0..255)."
    )


def main(argv=None) -> int:
    p = argparse.ArgumentParser(prog="ambition-ldtk-tools portal pair")
    p.add_argument("pair", nargs="?", help=argparse.SUPPRESS)
    p.add_argument("--ldtk", type=Path, default=DEFAULT_LDTK)
    p.add_argument("--level", required=True, help="level identifier (e.g. portal_lab)")
    p.add_argument("--channel", required=True, help="channel of end A (partner is B)")
    p.add_argument(
        "--a", nargs=3, metavar=("X", "Y", "NORMAL"), required=True,
        help="end A: level-local x y and surface normal",
    )
    p.add_argument(
        "--b", nargs=3, metavar=("X", "Y", "NORMAL"), required=True,
        help="end B (partner): level-local x y and surface normal",
    )
    p.add_argument("--id", default="portal", help="id prefix; ends are {id}_{color}")
    p.add_argument("--name", default=None, help="shared name (default: '{id} pair')")
    p.add_argument("--in-place", action="store_true")
    p.add_argument("--output", type=Path, default=None)
    p.add_argument("--backup", action="store_true")
    p.add_argument("--no-repair", action="store_true")
    args = p.parse_args(argv)
    if not args.in_place and args.output is None:
        p.error("choose --in-place or --output <path>")

    chan_a = args.channel.strip().lower()
    chan_b = partner_of(chan_a)
    name = args.name or f"{args.id} pair"

    def make(channel: str, placement) -> dict:
        x, y, normal = placement
        normal = normal.strip().lower()
        if normal not in NORMALS:
            raise SystemExit(f"normal must be one of {sorted(NORMALS)}, got '{normal}'")
        return {
            "type": "Portal",
            "px": [int(x), int(y)],
            "fields": {
                "id": f"{args.id}_{channel}",
                "name": name,
                "color": channel,
                "normal": normal,
            },
        }

    project = load_project(args.ldtk)
    level = find_level(project, args.level)
    layer = find_ambition_layer(level)
    grid = int(project.get("defaultGridSize", 16))
    ent_def = find_entity_def(project, "Portal")
    valid = {f["identifier"] for f in ent_def.get("fieldDefs", [])}

    added = []
    for spec in (make(chan_a, args.a), make(chan_b, args.b)):
        for fname in spec["fields"]:
            if fname not in valid:
                return _fail(f"Portal has no field '{fname}' (known: {sorted(valid)})")
        inst = build_entity_instance(
            project, spec, grid, int(level.get("worldX", 0)), int(level.get("worldY", 0))
        )
        layer.setdefault("entityInstances", []).append(inst)
        added.append(f"{spec['fields']['id']} ({spec['fields']['color']})")

    target = args.output or args.ldtk
    if args.in_place and args.backup:
        backup = args.ldtk.with_suffix(args.ldtk.suffix + ".bak")
        shutil.copy2(args.ldtk, backup)
        print(f"wrote backup: {backup}")
    write_project(target, project)
    print(f"added portal pair to '{args.level}': {' <-> '.join(added)}")

    if args.no_repair:
        return 0
    cmd = [sys.executable, "-m", "ambition_ldtk_tools.repair", str(target), "--in-place"]
    print("$ " + " ".join(cmd))
    return subprocess.call(cmd)


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
