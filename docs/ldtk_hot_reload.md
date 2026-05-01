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
