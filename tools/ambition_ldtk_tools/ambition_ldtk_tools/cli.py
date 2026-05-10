"""Modal CLI for ambition_ldtk_tools.

Subcommands (those marked [TODO] are not yet wired and will print a hint):

    validate                       Validate the Ambition LDtk file.
    repair                         Repair editor metadata for round-trip.
    roundtrip                      Non-mutating round-trip smoke check.
    doctor                         Run roundtrip + validate.
    compact                        Re-format JSON arrays to LDtk editor style.
    list-metadata                  Print biome/music/ambient metadata per level.
    schema fetch                   Fetch the official LDtk JSON schema.
    schema validate                Run schema-only validation against an LDtk.

    area create <spec.yaml>        Author a new area / level from a YAML spec.
    door free-spots <room>         List free 48x96 door slots in a level.

    entity add <spec.yaml>         Add entity instance(s) into a level.
    entity set-field [TODO]        Set a field on an existing entity.
    entity move      [TODO]        Move an existing entity.
    entity delete    [TODO]        Delete an entity instance.

    def register-entity <spec>     Register a new entity definition.

    link add         [TODO]
    link remove      [TODO]
    link check       [TODO]

    intgrid paint    [TODO]
    intgrid erase    [TODO]
    intgrid summarize [TODO]

The TODO subcommands are placeholders — the package was migrated from
several standalone scripts and these slots are reserved so the surface
stays stable while we backfill them.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path


def _todo(name: str) -> int:
    print(
        f"[TODO] '{name}' is not implemented yet. The CLI surface is reserved "
        f"so callers do not break when it lands. Please file or pick this up.",
        file=sys.stderr,
    )
    return 64  # EX_USAGE-ish


def _delegate(module_path: str, argv: list[str]) -> int:
    """Import a script-style module and call its main(argv)."""
    from importlib import import_module

    mod = import_module(module_path)
    if not hasattr(mod, "main"):
        print(f"internal error: module {module_path} has no main()", file=sys.stderr)
        return 70
    return int(mod.main(argv))


# ---- Command implementations (most are thin wrappers around legacy mains) ----

def cmd_validate(args, rest):
    return _delegate("ambition_ldtk_tools.validate", rest)


def cmd_repair(args, rest):
    return _delegate("ambition_ldtk_tools.repair", rest)


def cmd_roundtrip(args, rest):
    return _delegate("ambition_ldtk_tools.roundtrip", rest)


def cmd_doctor(args, rest):
    """Run roundtrip + validate sequentially."""
    rc = _delegate("ambition_ldtk_tools.roundtrip", rest)
    if rc != 0:
        return rc
    return _delegate("ambition_ldtk_tools.validate", rest)


def cmd_compact(args, rest):
    return _delegate("ambition_ldtk_tools.compact", rest)


def cmd_list_metadata(args, rest):
    return _delegate("ambition_ldtk_tools.list_metadata", rest)


def cmd_schema(args, rest):
    if args.schema_action == "fetch":
        return _delegate("ambition_ldtk_tools.schema", rest)
    if args.schema_action == "validate":
        # Delegate to validate with --schema flag handling.
        return _delegate("ambition_ldtk_tools.validate", rest)
    return _todo(f"schema {args.schema_action}")


def cmd_area(args, rest):
    if args.area_action == "create":
        return _delegate("ambition_ldtk_tools.area_authoring", rest)
    return _todo(f"area {args.area_action}")


def cmd_door(args, rest):
    if args.door_action == "free-spots":
        # area_authoring exposes --list-free-spots <room>; forward through.
        return _delegate("ambition_ldtk_tools.area_authoring", ["--list-free-spots", *rest])
    if args.door_action == "snap":
        # area_authoring exposes --snap-to-surface; forward through.
        return _delegate("ambition_ldtk_tools.area_authoring", ["--snap-to-surface", *rest])
    return _todo(f"door {args.door_action}")


def cmd_entity(args, rest):
    if args.entity_action == "add":
        return _delegate("ambition_ldtk_tools.edit.entities", rest)
    if args.entity_action == "even-space":
        # area_authoring exposes --even-space-entities; forward through.
        return _delegate(
            "ambition_ldtk_tools.area_authoring",
            ["--even-space-entities", *rest],
        )
    return _todo(f"entity {args.entity_action}")


def cmd_def(args, rest):
    if args.def_action == "register-entity":
        return _delegate("ambition_ldtk_tools.edit.defs", rest)
    return _todo(f"def {args.def_action}")


def cmd_link(args, rest):
    return _todo(f"link {args.link_action}")


def cmd_intgrid(args, rest):
    return _todo(f"intgrid {args.intgrid_action}")


# ---- Parser construction ------------------------------------------------------

def build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(
        prog="ambition_ldtk_tools",
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    sub = ap.add_subparsers(dest="command", required=True)

    sp_validate = sub.add_parser("validate", help="Validate the Ambition LDtk file")
    sp_validate.set_defaults(func=cmd_validate)

    sp_repair = sub.add_parser("repair", help="Repair editor metadata in the LDtk file")
    sp_repair.set_defaults(func=cmd_repair)

    sp_roundtrip = sub.add_parser("roundtrip", help="Non-mutating round-trip smoke check")
    sp_roundtrip.set_defaults(func=cmd_roundtrip)

    sp_doctor = sub.add_parser("doctor", help="Run roundtrip + validate")
    sp_doctor.set_defaults(func=cmd_doctor)

    sp_compact = sub.add_parser("compact", help="Compact LDtk JSON arrays to editor style")
    sp_compact.set_defaults(func=cmd_compact)

    sp_list_metadata = sub.add_parser(
        "list-metadata",
        help="Print biome/music/ambient metadata per level",
    )
    sp_list_metadata.set_defaults(func=cmd_list_metadata)

    # schema {fetch,validate}
    sp_schema = sub.add_parser("schema", help="Schema fetch/validate helpers")
    schema_sub = sp_schema.add_subparsers(dest="schema_action", required=True)
    schema_sub.add_parser("fetch", help="Fetch the official LDtk JSON schema")
    schema_sub.add_parser("validate", help="Run schema-only validation against an LDtk")
    sp_schema.set_defaults(func=cmd_schema)

    # area create
    sp_area = sub.add_parser("area", help="Area authoring")
    area_sub = sp_area.add_subparsers(dest="area_action", required=True)
    area_sub.add_parser("create", help="Create an area/level from a spec")
    sp_area.set_defaults(func=cmd_area)

    # door free-spots / door snap
    sp_door = sub.add_parser("door", help="Door helpers")
    door_sub = sp_door.add_subparsers(dest="door_action", required=True)
    door_sub.add_parser("free-spots", help="List free 48x96 door slots in a level")
    door_sub.add_parser(
        "snap",
        help="Snap a door to the nearest Collision surface; forwards to "
        "area-authoring's --snap-to-surface flag. Usage: door snap <room> --x N "
        "[--door-w 48] [--door-h 96] [--prefer-y N]",
    )
    sp_door.set_defaults(func=cmd_door)

    # entity {add,set-field,move,delete,even-space}
    sp_entity = sub.add_parser("entity", help="Entity instance edits")
    entity_sub = sp_entity.add_subparsers(dest="entity_action", required=True)
    entity_sub.add_parser("add", help="Add entity instance(s)")
    entity_sub.add_parser("set-field", help="[TODO] Set a field on an existing entity")
    entity_sub.add_parser("move", help="[TODO] Move an existing entity")
    entity_sub.add_parser("delete", help="[TODO] Delete an entity instance")
    entity_sub.add_parser(
        "even-space",
        help="Even-space entities of one type along x in a level. "
        "Usage: entity even-space <room> [--entity-type ID] [--y-row Y] "
        "[--strategy preserve-ends|fit] [--start-x N --end-x N]",
    )
    sp_entity.set_defaults(func=cmd_entity)

    # def register-entity
    sp_def = sub.add_parser("def", help="Definition edits")
    def_sub = sp_def.add_subparsers(dest="def_action", required=True)
    def_sub.add_parser("register-entity", help="Register a new entity definition")
    sp_def.set_defaults(func=cmd_def)

    # link {add,remove,check}
    sp_link = sub.add_parser("link", help="[TODO] Entity link helpers")
    link_sub = sp_link.add_subparsers(dest="link_action", required=True)
    link_sub.add_parser("add")
    link_sub.add_parser("remove")
    link_sub.add_parser("check")
    sp_link.set_defaults(func=cmd_link)

    # intgrid {paint,erase,summarize}
    sp_intgrid = sub.add_parser("intgrid", help="[TODO] IntGrid edits")
    intgrid_sub = sp_intgrid.add_subparsers(dest="intgrid_action", required=True)
    intgrid_sub.add_parser("paint")
    intgrid_sub.add_parser("erase")
    intgrid_sub.add_parser("summarize")
    sp_intgrid.set_defaults(func=cmd_intgrid)

    return ap


def main(argv: list[str] | None = None) -> int:
    if argv is None:
        argv = sys.argv[1:]
    ap = build_parser()
    # parse_known_args lets us forward leftover flags to the delegated
    # legacy mains without re-declaring every flag here.
    args, rest = ap.parse_known_args(argv)
    return args.func(args, rest)


if __name__ == "__main__":
    raise SystemExit(main())
