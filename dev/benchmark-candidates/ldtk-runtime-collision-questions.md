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

## Q: Implement a "ledge grab + climb-up" snap mechanic that cannot teleport the player out of the world

### Context

You're adding a ledge-grab mechanic to a 2D platformer. The player can wall-cling, and while clinging the engine each frame calls a probe routine that asks: *"is there a grabbable ledge at head height on the side I'm clinging to?"* If yes, the probe returns:

* `anchor` — where the player's center should sit while hanging.
* `climb_target` — where the player should snap when they press Up + Jump to climb up.

The world is a finite rect; coordinates are top-left (Y grows downward), and the playable area is `[Vec2::ZERO, world.size]`. The world contains `Solid` blocks. Levels are authored in an external editor (LDtk) and may include ceiling tiles, floor tiles, and arbitrary geometry; the probe's caller does not pre-filter blocks by location, only by `BlockKind`.

A natural first cut of the probe checks two things for each candidate Solid:

1. The ledge top sits within a "shoulder height" band around the player's head.
2. The space directly above the ledge is clear of *other* solid blocks (probe an AABB the player's size sitting on top of the ledge; reject if it overlaps another Solid).

This passes every unit test the engine ships, and the player can grab and climb most ledges in the level.

### Failure

In one room the player can grab a ledge near the ceiling and then snaps *above* the world's top edge after pressing Up + Jump. The engine's collision-correction or a follow-up movement step bounces them back to the wall, the wall-cling state re-fires the probe, the probe returns the same ledge, and the player is stuck oscillating between two positions ~450 px apart — visually a flickering teleport, mechanically a soft-lock that traps them inside an arena lock-wall encounter.

The only signal in an F8 trace dump is two repeating `CollisionCorrection :: <large>px (vel-budget <small>px)` events alternating directions, with no `InputEdge` events between them.

### Expected answer

The probe is missing a world-bounds rejection. "Clear of other blocks" is necessary but not sufficient — the climbed-onto AABB must also stay inside the world rect. With top-left coordinates and a player half-extent `half`, the probe AABB whose center is `(target_x, target_y)` must satisfy:

```rust
target_y - half.y >= 0.0
target_x - half.x >= 0.0
target_x + half.x <= world.size.x
```

These three checks belong inside the probe alongside the existing `body_overlaps_any` clearance check, not in the caller. Splitting them invites a future call site to skip the bounds check; co-locating them keeps "is this snap target physically valid" as one decision.

The deeper invariant: **any movement primitive that snaps the player to a computed position (ledge climb, blink, dash-through, mantle, teleport) must validate the destination against world bounds before the snap commits, even when local clearance looks fine.** Recovery systems (collision-correction, last-safe-pos) are not a substitute — they fight the snap each frame and produce visible teleport-loops rather than fail closed.

### Diagnostic hint

The fingerprint of "physics fight between a bad snap and a recovery system" in an F8 trace is paired `CollisionCorrection` events with equal-and-opposite deltas and no `InputEdge` between them. When you see that pattern, look for snap mechanics — not for collision bugs in the sweep code.

### Validation

* Unit test: a world with a `Solid` whose top edge is at `y ≈ 1` (so the player sitting on top would have their head above `y = 0`) — the probe must return `None`. Without the bounds check the probe returns a `LedgeContact` whose `climb_target.y` is negative.
* In-engine: kicking off a wall-cling against a near-ceiling tile should not produce a teleport, period.

### Tags

`game-physics`, `world-bounds`, `snap-mechanic`, `record-replay`,
`silent-failure`, `feedback-loop`, `architecture-seam`
