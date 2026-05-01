# LDtk hot reload foundation

Ambition now treats the sandbox LDtk project as a live development asset. The
runtime keeps the Ambition gameplay world typed in Rust, but can rebuild that
world from the on-disk LDtk project while the sandbox is running.

Run the sandbox with Bevy file watching enabled during level-design sessions:

```bash
cargo run -p ambition_sandbox --features dev_hot_reload --release
```

The sandbox also polls the LDtk file modification time, so the manual reload
path still works even when file watching is not enabled.

## Controls

- `F11`: validate and apply the current `sandbox.ldtk` file immediately.
- `F12`: toggle automatic apply after a changed LDtk file is detected.
- `F5`: overview camera, useful after moving stitched chunks or resizing areas.

## Reload policy

A reload does all of the following:

1. Reads `crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk` from disk.
2. Runs the Ambition LDtk validator.
3. Rejects the reload if validation fails, leaving the live world intact.
4. Converts LDtk levels into the Ambition room manifest.
5. Rebuilds `RoomSet`, `GameWorld`, feature runtime, moving-platform state, and
   the LDtk runtime index.
6. Preserves the player, abilities, HP, and velocity as much as possible.
7. Repairs the player to the nearest valid spawn if the edited map places the
   previous position inside collision.
8. Despawns and respawns map-authored room visuals / physics mirrors.

## Intentional constraints

Hot reload is allowed to rebuild map-authored runtime state. It should not reset
long-lived player progression, health, or tuning resources. If a future gameplay
system needs persistent map state, it should key that state by stable LDtk IID
instead of transient spawn order.

The validator remains mandatory. LDtk is first-class for authored world data,
but Ambition still owns the gameplay invariants.

## Next steps

- Listen to Bevy `AssetEvent<LdtkProject>` in addition to modification-time
  polling once the exact Bevy 0.18 message-reader API is settled.
- Promote low-risk entities (`PlayerStart`, `DebugLabel`, `LoadingZone`,
  `CameraZone`) to direct `bevy_ecs_ldtk` entity bundles.
- Add raw-LDtk-vs-Ambition runtime debug overlays for every spatial entity.
- Preserve collected chest/pickup state by stable LDtk IID across reloads.
- Add safe policies for moving/deleting the current active area under the player.

## `bevy_ecs_ldtk` loader health

The Ambition validator should catch editor/schema shape problems that would make
`bevy_ecs_ldtk` reject the project. In particular, LDtk `FieldDef` records in
both `defs.entities[*].fieldDefs` and `defs.levelFields` need the first-class
LDtk reference/display keys such as `allowedRefs`, `allowedRefTags`,
`allowOutOfLevelRef`, `autoChainRef`, `editorDisplayScale`, `editorLinkStyle`,
`editorShowInWorld`, `exportToToc`, `searchable`, and `symmetricalRef`.

If Bevy logs an LDtk asset loader error but the Ambition reload path still works,
treat it as a real issue: the typed `LdtkProject` asset is not healthy, and
direct `bevy_ecs_ldtk` spawning/hot reload will be partial or misleading.
Fix the LDtk schema shape and validator before migrating more runtime categories
onto direct LDtk-spawned entities.

The first-class `LdtkWorldBundle` root is intentionally hidden during the
transition. If a room shows large dark rectangles or other LDtk placeholder
visuals, keep the root hidden and migrate the relevant LDtk entity/layer through
a typed Ambition bundle instead of exposing the entire plugin-spawned hierarchy.

## Entity and field definition UID health

`bevy_ecs_ldtk` uses each entity instance's `defUid` to look up its
`EntityDefinition` before computing the spawned Bevy transform. If a hand-authored
or agent-generated LDtk file has entity instances whose `defUid` values do not
match `defs.entities[*].uid`, the plugin can panic during level spawning even
though Ambition's custom JSON adapter can still identify entities by
`__identifier`. The validator must therefore check instance `defUid` values for
entities, level fields, and entity field instances.

When generating or patching `.ldtk` directly, always keep these IDs synchronized:

```text
entity instance defUid == defs.entities[identifier].uid
entity field instance defUid == defs.entities[identifier].fieldDefs[field].uid
level field instance defUid == defs.levelFields[field].uid
```
