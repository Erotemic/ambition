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

    world init <target.ldtk>       Scaffold a new .ldtk file by cloning sandbox.ldtk defs.
    world auto-layout <ldtk>      Arrange Free-layout levels by LoadingZone graph
                                   (--strategy greedy/layered/clustered).

    generate hall-of-characters    Rebuild the Hall of Characters area spec from
                                   character_catalog.ron (pedestals, tiers, dialogue ids).

    diff semantic <before> <after> Review semantic LDtk changes without JSON noise.
    policy check|fix <ldtk>        Check/fix agent authoring policies.
    camera audit|auto-cover <ldtk> CameraZone placement and coverage helpers.
    asset catalog <ldtk>          List registered tilesets, entity sprites, and PNGs.
    asset link-entity-tile <ldtk> Point an entity def at a registered tileset tile.
    asset generate-editor-icons   Create placeholder editor icon PNGs.
    asset register-entity-icons   Generate the editor-icon atlas + wire every entity def's icon (one shot).
    asset suggest/apply/validate-manifest
                                  Scaffold and apply visual manifests.

    level set-field [--level ID --set key=value | <spec.yaml>]
                                   Update level-scoped metadata (biome /
                                   music_track / ambient_profile / etc.).

    entity add <spec.yaml>         Add entity instance(s) into a level.
    entity set-field <spec.yaml>   Set field instances on existing entities.
    entity move      <spec.yaml>   Move an existing entity to new px/size.
    entity change-layer <ldtk>     Move selected entities to another Entities layer.
    entity delete    <spec.yaml>   Delete entity instance(s) from a level.
    entity query     [filters]     Read-only: list/query entities by level/type/field.
    entity check     [rect]        Read-only: report overlaps + nearest neighbor for a placement.
    layer check-entity-rules       Read-only: validate entity/layer placement policy.
    layer apply-entity-rules       Set LDtk tag filters so entities are only placeable on intended layers.

    def register-entity <spec>     Register a new entity definition.
    def update-entity <id> <ldtk>  Add new fields to an existing entity def.

    tileset add <ldtk> <png> <grid> Register a tileset def from a PNG.

    link add         [TODO]
    link remove      [TODO]
    link check       [TODO]

    intgrid summarize <level>     Print per-value cell counts + bboxes for a layer.
    intgrid query     <rect>      Read-only: what IntGrid values are at a px/size rect.
    intgrid erase     <rect>      Zero out cells overlapping a px/size rect.
    intgrid paint    <rect>       Set cells overlapping the rect to --value (1=Solid).
    gates audit       <level>      Read-only: switches/lock walls/triggers/breakables + targets.

    room describe     --level X    Read-only: summarize a room for chat/sandbox design.
    room render       --level X    Render a room to SVG/PNG without launching LDtk/game.
    room bundle-debug --level X    Bundle summary + render + traces/specs for upload.

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
    """Run roundtrip + validate, reporting both diagnostics when possible."""
    roundtrip_rc = _delegate("ambition_ldtk_tools.roundtrip", rest)
    validate_rc = _delegate("ambition_ldtk_tools.validate", rest)
    return roundtrip_rc or validate_rc


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


def cmd_world(args, rest):
    if args.world_action == "init":
        return _delegate("ambition_ldtk_tools.world_init", rest)
    if args.world_action == "repack":
        return _delegate(
            "ambition_ldtk_tools.edit.world_repack", [args.world_action, *rest]
        )
    if args.world_action == "auto-layout":
        return _delegate(
            "ambition_ldtk_tools.edit.world_layout", [args.world_action, *rest]
        )
    return _todo(f"world {args.world_action}")


def cmd_generate(args, rest):
    if args.generate_action == "hall-of-characters":
        return _delegate("ambition_ldtk_tools.generate_hall_of_characters", rest)
    return _todo(f"generate {args.generate_action}")


def cmd_dialogue(args, rest):
    if args.dialogue_action == "lint":
        return _delegate("ambition_ldtk_tools.dialogue_lint", rest)
    return _todo(f"dialogue {args.dialogue_action}")


def cmd_diff(args, rest):
    if args.diff_action == "semantic":
        return _delegate("ambition_ldtk_tools.edit.semantic_diff", [args.diff_action, *rest])
    return _todo(f"diff {args.diff_action}")


def cmd_policy(args, rest):
    if args.policy_action in {"check", "fix"}:
        return _delegate("ambition_ldtk_tools.edit.policy", [args.policy_action, *rest])
    return _todo(f"policy {args.policy_action}")


def cmd_camera(args, rest):
    if args.camera_action in {"audit", "auto-cover"}:
        return _delegate("ambition_ldtk_tools.edit.camera", [args.camera_action, *rest])
    return _todo(f"camera {args.camera_action}")


def cmd_asset(args, rest):
    if args.asset_action in {
        "catalog",
        "link-entity-tile",
        "generate-editor-icons",
        "register-entity-icons",
        "suggest-manifest",
        "apply-manifest",
        "validate-manifest",
        "preview-manifest",
    }:
        return _delegate("ambition_ldtk_tools.edit.assets", [args.asset_action, *rest])
    return _todo(f"asset {args.asset_action}")


def cmd_door(args, rest):
    if args.door_action == "free-spots":
        # area_authoring exposes --list-free-spots <room>; forward through.
        return _delegate(
            "ambition_ldtk_tools.area_authoring", ["--list-free-spots", *rest]
        )
    if args.door_action == "snap":
        # area_authoring exposes --snap-to-surface; forward through.
        return _delegate(
            "ambition_ldtk_tools.area_authoring", ["--snap-to-surface", *rest]
        )
    return _todo(f"door {args.door_action}")


def cmd_level(args, rest):
    if args.level_action == "set-field":
        return _delegate("ambition_ldtk_tools.edit.level_set_field", rest)
    if args.level_action == "diff-specs":
        return _delegate("ambition_ldtk_tools.edit.spec_diff", rest)
    if args.level_action == "delete":
        return _delegate("ambition_ldtk_tools.edit.level_delete", rest)
    return _todo(f"level {args.level_action}")


def cmd_entity(args, rest):
    if args.entity_action == "add":
        return _delegate("ambition_ldtk_tools.edit.entities", rest)
    if args.entity_action == "set-field":
        return _delegate("ambition_ldtk_tools.edit.set_field", rest)
    if args.entity_action == "move":
        return _delegate("ambition_ldtk_tools.edit.move", rest)
    if args.entity_action == "change-layer":
        return _delegate("ambition_ldtk_tools.edit.entity_layer_rules", [args.entity_action, *rest])
    if args.entity_action == "delete":
        return _delegate("ambition_ldtk_tools.edit.delete", rest)
    if args.entity_action == "query":
        return _delegate("ambition_ldtk_tools.edit.query", rest)
    if args.entity_action == "measure":
        return _delegate("ambition_ldtk_tools.edit.measure", rest)
    if args.entity_action == "check":
        return _delegate("ambition_ldtk_tools.edit.check", rest)
    if args.entity_action == "snap-to-floor":
        return _delegate("ambition_ldtk_tools.edit.snap_to_floor", rest)
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
    if args.def_action == "update-entity":
        # update_entity's argparse owns its surface; prepend the
        # action verb so the existing per-tool parser sees it
        # positionally (same pattern as `tileset add`).
        return _delegate(
            "ambition_ldtk_tools.edit.update_entity",
            [args.def_action, *rest],
        )
    return _todo(f"def {args.def_action}")


def cmd_mount(args, rest):
    if args.mount_action == "split":
        return _delegate("ambition_ldtk_tools.mount_split", rest)
    return _todo(f"mount {args.mount_action}")


def cmd_layer(args, rest):
    if args.layer_action == "split-entities":
        return _delegate(
            "ambition_ldtk_tools.edit.layer_split_entities",
            [args.layer_action, *rest],
        )
    if args.layer_action in {"check-entity-rules", "apply-entity-rules"}:
        return _delegate(
            "ambition_ldtk_tools.edit.entity_layer_rules",
            [args.layer_action, *rest],
        )
    return _todo(f"layer {args.layer_action}")


def cmd_tileset(args, rest):
    if args.tileset_action == "add":
        # The tileset module's argparse owns its surface; prepend the
        # action verb so the existing per-tool parser sees it
        # positionally (matches the `intgrid summarize/erase` pattern).
        return _delegate(
            "ambition_ldtk_tools.edit.tilesets", [args.tileset_action, *rest]
        )
    if args.tileset_action == "add-layer":
        return _delegate(
            "ambition_ldtk_tools.edit.tile_layers", [args.tileset_action, *rest]
        )
    if args.tileset_action == "paint":
        return _delegate(
            "ambition_ldtk_tools.edit.tile_paint", [args.tileset_action, *rest]
        )
    return _todo(f"tileset {args.tileset_action}")


def cmd_link(args, rest):
    return _todo(f"link {args.link_action}")


def cmd_intgrid(args, rest):
    if args.intgrid_action in {"summarize", "query", "erase", "paint"}:
        # Forward the action + leftover argv (the --layer / --level /
        # --px / --size / --value flags) to the dedicated module so its
        # own argparse owns the surface.
        return _delegate(
            "ambition_ldtk_tools.edit.intgrid",
            [args.intgrid_action, *rest],
        )
    return _todo(f"intgrid {args.intgrid_action}")


def cmd_gates(args, rest):
    if args.gates_action == "audit":
        return _delegate("ambition_ldtk_tools.edit.gates", rest)
    return _todo(f"gates {args.gates_action}")


def cmd_portal(args, rest):
    if args.portal_action == "pair":
        return _delegate("ambition_ldtk_tools.edit.portals", rest)
    return _todo(f"portal {args.portal_action}")


def cmd_room(args, rest):
    if args.room_action in {"describe", "render", "bundle-debug"}:
        # `room.py` owns its argparse surface. The modal parser only captures
        # the action, so normalize `room describe --ldtk FILE ...` into the
        # delegated parser's expected `--ldtk FILE describe ...` shape.
        before_action: list[str] = []
        after_action: list[str] = []
        it = iter(rest)
        for item in it:
            if item == "--ldtk":
                try:
                    value = next(it)
                except StopIteration:
                    print("room: --ldtk requires a path", file=sys.stderr)
                    return 64
                before_action.extend(["--ldtk", value])
            else:
                after_action.append(item)
        return _delegate(
            "ambition_ldtk_tools.room",
            [*before_action, args.room_action, *after_action],
        )
    if args.room_action == "compile-spec":
        return _delegate("ambition_ldtk_tools.edit.room_spec", ["compile", *rest])
    return _todo(f"room {args.room_action}")


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

    sp_roundtrip = sub.add_parser(
        "roundtrip", help="Non-mutating round-trip smoke check"
    )
    sp_roundtrip.set_defaults(func=cmd_roundtrip)

    sp_doctor = sub.add_parser("doctor", help="Run roundtrip + validate")
    sp_doctor.set_defaults(func=cmd_doctor)

    sp_compact = sub.add_parser(
        "compact", help="Compact LDtk JSON arrays to editor style"
    )
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

    # world init
    sp_world = sub.add_parser("world", help="Multi-file world helpers")
    world_sub = sp_world.add_subparsers(dest="world_action", required=True)
    world_sub.add_parser(
        "init",
        help="Scaffold a new .ldtk file by cloning sandbox.ldtk defs",
    )
    world_sub.add_parser(
        "repack",
        help=(
            "Re-pack levels into an edge-adjacent horizontal chain "
            "(GridVania-friendly). Usage: world repack <ldtk> "
            "[--start-x N] [--start-y N] [--order L1,L2,...] "
            "(--in-place | --output PATH)"
        ),
    )
    world_sub.add_parser(
        "auto-layout",
        help=(
            "Arrange Free-layout levels by LoadingZone graph while preserving "
            "activeArea groups. Usage: world auto-layout <ldtk> "
            "[--start central_hub_main] [--origin 0,0] [--gap 256] [--padding N] [--svg-report PATH] "
            "(--dry-run | --in-place | --output PATH)"
        ),
    )
    sp_world.set_defaults(func=cmd_world)

    # generate {hall-of-characters}
    sp_generate = sub.add_parser(
        "generate",
        help="Regenerate procedurally-authored levels from their source-of-truth data",
    )
    generate_sub = sp_generate.add_subparsers(dest="generate_action", required=True)
    generate_sub.add_parser(
        "hall-of-characters",
        help=(
            "Rebuild the Hall of Characters area spec from character_catalog.ron "
            "(pedestals, tiers, dialogue ids). Usage: generate hall-of-characters "
            "[--catalog PATH] [--out PATH] [--print-summary]"
        ),
    )
    sp_generate.set_defaults(func=cmd_generate)

    # dialogue {lint}
    sp_dialogue = sub.add_parser("dialogue", help="Yarn dialogue helpers")
    dialogue_sub = sp_dialogue.add_subparsers(dest="dialogue_action", required=True)
    dialogue_sub.add_parser(
        "lint",
        help=(
            "Lint Yarn files for malformed markup tags (e.g. a bracketed stage "
            "direction the runtime parses as a tag and panics on). Usage: "
            "dialogue lint [--root DIR]"
        ),
    )
    sp_dialogue.set_defaults(func=cmd_dialogue)

    # diff {semantic}
    sp_diff = sub.add_parser("diff", help="Semantic LDtk diffs")
    diff_sub = sp_diff.add_subparsers(dest="diff_action", required=True)
    diff_sub.add_parser("semantic", help="Review semantic LDtk changes without raw JSON noise")
    sp_diff.set_defaults(func=cmd_diff)

    # policy {check,fix}
    sp_policy = sub.add_parser("policy", help="Agent/CI LDtk policy checks")
    policy_sub = sp_policy.add_subparsers(dest="policy_action", required=True)
    policy_sub.add_parser("check", help="Check LDtk authoring policy rules")
    policy_sub.add_parser("fix", help="Apply safe policy fixes such as entity layer moves")
    sp_policy.set_defaults(func=cmd_policy)

    # camera {audit,auto-cover}
    sp_camera = sub.add_parser("camera", help="CameraZone audit and auto-cover helpers")
    camera_sub = sp_camera.add_subparsers(dest="camera_action", required=True)
    camera_sub.add_parser("audit", help="Check CameraZone layer placement and coverage")
    camera_sub.add_parser("auto-cover", help="Create/update a CameraZone covering a level play rect")
    sp_camera.set_defaults(func=cmd_camera)

    # asset helpers
    sp_asset = sub.add_parser("asset", help="Asset/tileset/entity-sprite helpers")
    asset_sub = sp_asset.add_subparsers(dest="asset_action", required=True)
    asset_sub.add_parser("catalog", help="List registered tilesets, editor sprites, and PNG assets")
    asset_sub.add_parser("link-entity-tile", help="Point an entity def at a registered tileset tileRect")
    asset_sub.add_parser("generate-editor-icons", help="Create placeholder editor icon PNGs")
    asset_sub.add_parser("register-entity-icons", help="One shot: generate the editor-icon atlas + wire every entity def to its icon")
    asset_sub.add_parser("suggest-manifest", help="Print or write a draft visual manifest")
    asset_sub.add_parser("apply-manifest", help="Register tilesets and entity editor icons from a visual manifest")
    asset_sub.add_parser("validate-manifest", help="Validate LDtk visual refs against a manifest")
    asset_sub.add_parser("preview-manifest", help="Render a visual manifest HTML preview")
    sp_asset.set_defaults(func=cmd_asset)

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

    # level set-field
    sp_level = sub.add_parser("level", help="Level (room) metadata edits")
    level_sub = sp_level.add_subparsers(dest="level_action", required=True)
    level_sub.add_parser(
        "set-field",
        help=(
            "Set level field instances (biome / music_track / "
            "ambient_profile / visual_theme / etc.) on existing levels"
        ),
    )
    level_sub.add_parser(
        "diff-specs",
        help=(
            "Read-only: compare each named area spec's "
            "world_x/world_y/px_wid/px_hei against the live LDtk and "
            "report any drift. Use --ldtk <file>. Exits non-zero if "
            "any spec disagrees (CI-friendly)."
        ),
    )
    level_sub.add_parser(
        "delete",
        help=(
            "Delete a whole level (room) from an LDtk file. Usage: "
            "level delete <level_id> [--ldtk PATH] (--in-place | --output PATH) "
            "[--backup]. Use when relocating a level to its own secondary world."
        ),
    )
    sp_level.set_defaults(func=cmd_level)

    # entity {add,set-field,move,delete,even-space}
    # portal pair — author a linked pair in one command
    sp_portal = sub.add_parser("portal", help="Portal authoring")
    portal_sub = sp_portal.add_subparsers(dest="portal_action", required=True)
    portal_sub.add_parser(
        "pair",
        help=(
            "Place a LINKED portal pair. Usage: portal pair --level <id> "
            "--channel <color|cN> --a X Y NORMAL --b X Y NORMAL [--id PREFIX] "
            "[--name NAME] (--in-place | --output PATH). NORMAL ∈ "
            "{up,down,left,right}; the partner color is assigned automatically."
        ),
    )
    sp_portal.set_defaults(func=cmd_portal)

    sp_entity = sub.add_parser("entity", help="Entity instance edits")
    entity_sub = sp_entity.add_subparsers(dest="entity_action", required=True)
    entity_sub.add_parser("add", help="Add entity instance(s)")
    entity_sub.add_parser("set-field", help="Set field instances on existing entities")
    entity_sub.add_parser("move", help="Move an existing entity")
    entity_sub.add_parser(
        "change-layer",
        help=(
            "Move selected entity instances to another Entities layer. "
            "Usage: entity change-layer <ldtk> --identifier CameraZone "
            "--to-layer AmbitionCameras [--from-layer Ambition] "
            "(--dry-run | --in-place | --output PATH)"
        ),
    )
    entity_sub.add_parser("delete", help="Delete entity instance(s) from a level")
    entity_sub.add_parser(
        "query",
        help=(
            "Read-only listing of entities by level/type/field. "
            "Use --level, --identifier, --field NAME=VALUE, --iid. "
            "Default output is a table; --format json for structured."
        ),
    )
    entity_sub.add_parser(
        "measure",
        help=(
            "Read-only: an entity's size + center + nearest Solid in each "
            "direction (placement context). Use --level, --identifier / --iid, "
            "optionally --layer."
        ),
    )
    entity_sub.add_parser(
        "check",
        help=(
            "Read-only: report overlaps and nearest-neighbor distance for "
            "a hypothetical placement. Pair with `door snap` to verify a "
            "slot is safe before `entity add`. Use --level, --px X,Y, "
            "--size W,H, optionally --against / --ignore / --ignore-iid "
            "/ --min-spacing. Exits non-zero on overlap or under-spaced."
        ),
    )
    entity_sub.add_parser(
        "even-space",
        help="Even-space entities of one type along x in a level. "
        "Usage: entity even-space <room> [--entity-type ID] [--y-row Y] "
        "[--strategy preserve-ends|fit] [--start-x N --end-x N]",
    )
    entity_sub.add_parser(
        "snap-to-floor",
        help=(
            "Drop an entity onto the nearest Solid/OneWayUp surface beneath "
            "its x-span (treats one-way platforms as floor, unlike measure). "
            "Use --level, --iid / --identifier [--match k=v], optionally --x "
            "to reposition first, --prefer-y, --dry-run."
        ),
    )
    sp_entity.set_defaults(func=cmd_entity)

    # def {register-entity, update-entity}
    sp_def = sub.add_parser("def", help="Definition edits")
    def_sub = sp_def.add_subparsers(dest="def_action", required=True)
    def_sub.add_parser("register-entity", help="Register a new entity definition")
    def_sub.add_parser(
        "update-entity",
        help=(
            "Add fields to an existing entity def. "
            "Usage: def update-entity <identifier> <ldtk> "
            "--add-field name:type[:default] [--add-field ...] "
            "(--in-place | --output PATH)"
        ),
    )
    sp_def.set_defaults(func=cmd_def)

    # mount split — ADR 0020 fused-composite → linked mount+rider pair migration
    sp_mount = sub.add_parser("mount", help="Mount authoring (ADR 0020)")
    mount_sub = sp_mount.add_subparsers(dest="mount_action", required=True)
    mount_sub.add_parser(
        "split",
        help=(
            "Split fused composite EnemySpawns (pirate_on_shark / "
            "pirate_heavy_on_shark) into linked mount+rider pairs. "
            "Usage: mount split <ldtk> (--in-place | --output PATH)"
        ),
    )
    sp_mount.set_defaults(func=cmd_mount)

    # tileset add
    sp_tileset = sub.add_parser("tileset", help="Tileset definition edits")
    tileset_sub = sp_tileset.add_subparsers(dest="tileset_action", required=True)
    tileset_sub.add_parser(
        "add",
        help=(
            "Register a tileset def from a PNG. "
            "Usage: tileset add <ldtk> <png> <grid_size> "
            "[--identifier NAME] [--padding N] [--spacing N] "
            "(--in-place | --output PATH)"
        ),
    )
    tileset_sub.add_parser(
        "add-layer",
        help=(
            "Add a Tiles layer def + empty per-level instances. "
            "Usage: tileset add-layer <ldtk> <tileset_identifier> "
            "[--layer-identifier NAME] [--display-opacity F] "
            "(--in-place | --output PATH)"
        ),
    )
    tileset_sub.add_parser(
        "paint",
        help=(
            "Paint tiles into a Tiles layer instance from an IntGrid "
            "source layer or a recipe. Usage: tileset paint <ldtk> "
            "<level> <layer> --from-intgrid SRC --map VALUE=TILE "
            "[--map VALUE=TILE ...] (--in-place | --output PATH)"
        ),
    )
    sp_tileset.set_defaults(func=cmd_tileset)

    # link {add,remove,check}
    sp_link = sub.add_parser("link", help="[TODO] Entity link helpers")
    link_sub = sp_link.add_subparsers(dest="link_action", required=True)
    link_sub.add_parser("add")
    link_sub.add_parser("remove")
    link_sub.add_parser("check")
    sp_link.set_defaults(func=cmd_link)

    # layer {split-entities,check-entity-rules,apply-entity-rules}
    sp_layer = sub.add_parser("layer", help="Layer-level edits")
    layer_sub = sp_layer.add_subparsers(dest="layer_action", required=True)
    layer_sub.add_parser(
        "split-entities",
        help=(
            "Move all entities of one `__identifier` into their own "
            "Entities-type layer so the editor can lock/hide that layer "
            "independently. Usage: layer split-entities <ldtk> "
            "--type CameraZone --to-layer AmbitionCameras "
            "[--from-layer Ambition] (--in-place | --output PATH)"
        ),
    )
    layer_sub.add_parser(
        "check-entity-rules",
        help=(
            "Read-only: validate that restricted entity types live on their "
            "intended layer. Defaults include CameraZone=AmbitionCameras; "
            "override/repeat with --rule Entity=Layer."
        ),
    )
    layer_sub.add_parser(
        "apply-entity-rules",
        help=(
            "Update LDtk entity tags + layer required/excluded tags so the "
            "editor only offers restricted entity types on intended layers. "
            "Usage: layer apply-entity-rules <ldtk> --type CameraZone "
            "--to-layer AmbitionCameras [--from-layer Ambition] "
            "(--dry-run | --in-place | --output PATH)"
        ),
    )
    sp_layer.set_defaults(func=cmd_layer)

    # intgrid {summarize,erase,paint}
    sp_intgrid = sub.add_parser("intgrid", help="IntGrid layer edits")
    intgrid_sub = sp_intgrid.add_subparsers(dest="intgrid_action", required=True)
    intgrid_sub.add_parser(
        "summarize",
        help="Print per-value cell counts + bboxes for an IntGrid layer",
    )
    intgrid_sub.add_parser(
        "query",
        help="Read-only: list the IntGrid values present in a px/size rect "
        "(what collision/hazard/etc. is at a location)",
    )
    intgrid_sub.add_parser(
        "erase",
        help="Zero out every IntGrid cell overlapping a given px/size rect",
    )
    intgrid_sub.add_parser(
        "paint",
        help="Set every IntGrid cell overlapping a given px/size rect to "
        "--value (1=Solid, 2=OneWayPlatform, etc.)",
    )
    sp_intgrid.set_defaults(func=cmd_intgrid)

    # gates {audit}
    sp_gates = sub.add_parser("gates", help="Audit gating / destructible elements")
    gates_sub = sp_gates.add_subparsers(dest="gates_action", required=True)
    gates_sub.add_parser(
        "audit",
        help="Read-only: list a level's switches / lock walls / encounter "
        "triggers / breakables and what each switch targets",
    )
    sp_gates.set_defaults(func=cmd_gates)

    # room {describe,render,bundle-debug}
    sp_room = sub.add_parser("room", help="Room inspection/render/debug-bundle helpers")
    room_sub = sp_room.add_subparsers(dest="room_action", required=True)
    room_sub.add_parser(
        "describe",
        help=(
            "Read-only: summarize a level's size, IntGrid values, entities, "
            "gravity zones, loading zones, moving platforms, cameras, and "
            "static review notes. Usage: room describe --level <id> "
            "[--ldtk FILE] [--format text|json] [--entities]"
        ),
    )
    room_sub.add_parser(
        "render",
        help=(
            "Read-only: render a level to .svg or .png without launching LDtk "
            "or the game. Usage: room render --level <id> --out /tmp/room.svg"
        ),
    )
    room_sub.add_parser(
        "bundle-debug",
        help=(
            "Read-only: create a .tar.gz containing room_describe.txt/json, "
            "a room render, matching specs, and debug trace JSONs. Usage: "
            "room bundle-debug --level <id> --out /tmp/room_debug.tar.gz"
        ),
    )
    room_sub.add_parser(
        "compile-spec",
        help=(
            "Compile a compact JSON/RON room edit spec into IntGrid/entity/camera "
            "edits. Usage: room compile-spec specs/foo.json --ldtk sandbox.ldtk --dry-run"
        ),
    )
    sp_room.set_defaults(func=cmd_room)

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
