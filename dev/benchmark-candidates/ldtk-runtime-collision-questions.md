# LDtk entity → runtime collision questions

## Q: When does an `entity add`-inserted Solid LDtk entity NOT acquire runtime collision in the Ambition sandbox?

### Context

`tools/ambition_ldtk_tools` exposes two mutating commands:

* `area create <spec.yaml>` — authors a whole new level. Among other
  things it walks every static-collision LDtk entity (`Solid`,
  `OneWayPlatform`, `BlinkWall`, …) and **lowers** them into the
  Ambition `Ambition` IntGrid layer at the appropriate cells. The
  runtime's `int_grid_value_to_block` path is what turns IntGrid
  cells into `ae::Block` collision; the entity-layer `Solid` shape
  is just an authoring affordance.

* `entity add <spec.yaml>` — surgically inserts one or more entity
  instances into an existing level. It updates `entityInstances` and
  re-runs the LDtk repair + validate pipeline so the file stays
  editor-clean.

Symptom (real bug from the 2026-05-10 mockingbird arena work): an
agent added a `Solid` LDtk entity via `entity add` to seal an arena
exit. The LDtk file validated cleanly, the new entity appeared in
the editor, but at runtime the player walked straight through it as
if it weren't there.

### Failure

The added Solid entity has zero collision in-game. Nothing fails at
build time; nothing fails at validate time. The only signal is that
the player can ignore the wall.

### Expected answer

`entity add` does not lower newly-inserted static-collision entities
into the IntGrid layer. The runtime collision path reads IntGrid
cells via `int_grid_value_to_block`, not the raw entity-layer
shapes. Without the lowering pass that `area create` runs at level
authoring time, the new Solid is invisible to collision.

The fix is one of:

1. **Re-author the level** so `area create`'s lowering pass runs for
   the new entity. Cost: any other surgical edits in the level have
   to be redone.
2. **Extend the tool** with an `intgrid paint` (already reserved as a
   placeholder in the CLI) or a dedicated `entity add --lower` flag
   that runs the same lowering pass `area create` does, in place,
   for the entities just inserted.
3. **Use a different mechanism for the gameplay intent.** A static
   wall in an arena is a poor model for "lock the player in during a
   boss fight" anyway — the natural primitive is an authored
   `LockWall` LDtk entity plus an `EncounterTrigger`, both of which
   the encounter system already understands and which keep the
   "appears at fight start, disappears on victory" semantics.

The diagnostic surface area is what makes this interesting: the
mistake passes every automated check the toolchain offers (LDtk
validate is clean, repair is no-op, `cargo test` is green) and only
manifests when a human walks through the wall. A model that merely
"ran the validator" cannot tell the difference between a working
fix and a no-op fix here.

### Validation

The bug was diagnosed by adding a player-vs-block collision overlay
(or by trying to use the wall in-game). A unit test that asserts
`ae::World::blocks_at(player_aabb)` finds a block at the inserted
entity's footprint would catch it.

### Tags

`ldtk`, `tooling`, `bevy-collision`, `silent-failure`,
`tool-vs-runtime-mismatch`
