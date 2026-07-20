# Falling sand — correctness first, then "Don't fuck with Oiler"

**Authored by fable, 2026-07-05.** Low priority relative to the
decomposition/demo tracks, but part of the engine: the module must be
CORRECT (deterministic, conserving, settling) before any feature rides it.
Current observed defects (Jon): water and oil pool on the top platform yet
particles ALSO fall forever below; settling sand becomes world geometry on
both the top and bottom platforms — i.e., particles are being duplicated or
mis-homed between the pooling representation and the falling representation.

## 1. The correctness contract (the actual engine work)

The falling-sand room becomes a **bounded, deterministic cellular
automaton** with an explicit conservation law:

- **One representation.** A particle exists in exactly one place: the grid.
  Pooled/settled matter is grid state (or, once compiled, RoomGeometry
  overlay solids) — never simultaneously a live falling particle. The
  observed double-pooling symptom is by definition a violation of this
  invariant; the fix is structural (single owner), not a patch.
- **Conservation test:** total matter per material = spawned − despawned,
  every tick, asserted in a headless test that runs the room's real spec.
- **Settle guarantee:** any finite spawn input reaches a fixed point (no
  particle falls forever): every particle either settles, pools, or exits
  through an authored drain/kill boundary. Test: spawn N, step until
  quiescent, assert fixed point within a tick budget.
- **Fluids find level:** water/oil equalize across connected basins
  (the standard lateral-flow CA rule with a determinism-safe update order
  — stable cell ordering, double-buffered, per the query-order rule).
- **Sand→geometry compilation** stays, but only from settled grid state,
  through the RoomGeometry OVERLAY (the write-map rule: falling_sand never
  mutates authored geometry) — and compiled cells leave the grid
  (conservation moves them between owners atomically).
- **Determinism:** fixed update order, seeded RNG from world state, C4-safe
  (gravity-frame from `GravityCtx`, not −y). The module remains a
  self-gating CONTENT plugin (architecture ruling) — the engine ships the
  CA substrate, the room ships as content.

Slices ~~FS1 (single-owner refactor + conservation test)~~ ✅ **DONE
2026-07-10 — see §3**, ~~FS2 (settle/level rules + fixed-point test)~~ /
~~FS3 (overlay compilation + atomic ownership transfer)~~ ✅ **DONE for SAND
2026-07-20 — see §4; water/oil level-finding remains open**. The spec above
is the contract; the current code is small and may be boldly restructured to
meet it.

**The SPOUT is the canonical authored-placement example (Jon, 2026-07-06).**
A falling-sand spout (a source that emits matter) is an **authored PLACEMENT
in the map**, not a hardcoded runtime spawn — it rides the same
world→sim lowering seam as any other contentful placement (architecture
§4b): the map author drops a `spout` placement (a Tier-0 authored schema:
material, rate, direction), `ambition_world` carries it as an authored
record, and the falling-sand CONTENT plugin registers the interpreter that
lowers it into the runtime emitter at room-load. So the same-tier deps hold:
`ambition_world` never names the falling-sand runtime; the content plugin
depends on world + reads the schema. **✅ RULED (fable, 2026-07-06 night, closing the last open schema
question):** the spout schema is **falling-sand-SPECIFIC** —
`SpoutSpec { material: String, rate: f32, direction: [f32; 2] }` as a
`PlacementSchema::Spout(...)` variant in the Tier-0 placements module
([W-a]). Do NOT author a general "emitter" placement: narrow specific
types beat wide generic ones (the closed, editor-visible schema is
Jon's stated preference, §4b.3), and generalization waits for a SECOND
emitter-shaped placement to actually land (grow-don't-mint). The
falling-sand content plugin registers the `Spout` interpreter through
the [W-b] `register_placement_interpreter` API — the canonical CONTENT
interpreter, and the W-queue step-3 proof case.

## 2. Oiler (ideas parked here deliberately — feel-pass era work)

The character **Oiler** (Euler) weaponizes the module: a special that
sprays large volumes of oil (FS-spawner as a technique with params —
the A-track technique seam), pooled oil becomes a **surface coating** that
slows actors moving through it (a `Contact`-driven movement modifier — the
surface-coating vocabulary may also serve ice/goo later; add it when Oiler
lands, not before), and a second beat IGNITES pooled oil: flame propagates
across connected coated cells, dealing damage volumes over time and
consuming the oil (conservation: oil → fire → gone). After enough pooling,
the arena is a trap; hence the brainstorm's line. Prerequisites: FS1–FS3,
the technique params seam (landed), CM5 presentation events. Home when
built: Oiler is CONTENT (a catalog row + techniques); only the surface-
coating movement hook is engine vocabulary.

---

## 3. FS1 — single owner + conservation (opus, 2026-07-10)

**The reported defect was not subtle, and it was not in the cellular automaton.**
`emit_falling_sand_spouts` did two things at once: it wrote `SpawnParticleSignal`s
into the CA grid *and* spawned a parallel fleet of Ambition-side
`FallingSandStreamParticle` sprites, with the comment *"so the player gets
immediate visual feedback that the spout opened."*

Those sprites were matter's second home. They fell on their own hardcoded
gravity (`vel.y += 90.0 * dt`, on `Res<Time>` rather than `WorldTime` — a
time-domains violation too), ignored every block in the room, and despawned at an
invented `world.size.y - 64` floor. So they poured straight **through** the
platforms the real particles were pooling on and rained down below. That is Jon's
report, verbatim: *"water and oil pool on the top platform yet particles ALSO
fall forever below."*

§1 says the fix is **structural (single owner), not a patch**. So:

- **The stream representation is deleted.** `SpawnParticleSignal` is now the only
  way matter enters. `bevy_falling_sand`'s own `render` feature draws the falling
  matter; `sync_material_visuals` draws what has settled. **One owner, two views.**
- **The spouts are a table.** `SpoutMouth { particle_type, x, y, width }` +
  `open_spouts(switch_state)` — the same data, in the same shape, that the ruled
  `PlacementSchema::Spout { material, rate, direction }` will carry. One `const`
  away from being read off the map instead of typed into the source, which is what
  [W-a]/[W-b] turn it into.
- **The conservation law is a ledger, and it is tested.** `tally_particles` walks
  the grid once and lands every particle in exactly one `TallyLedger` column:
  `sand`, `water`, `oil`, `outside_world`, or `unmodelled` (walls are geometry, not
  matter). `total()` equals the particles walked, and a `debug_assert` on every
  real frame checks that each material's tile buckets sum to its ledger column —
  so a particle counted twice, or lost between the query and the tile map, is a
  panic in a dev build rather than a drifting pool.
- **Single owner per TILE, too.** A tile dense enough to be a sand solid never also
  becomes a water region (you cannot swim inside a block), and the visual agrees
  with the collision. Pinned by `a_tile_dense_in_both_sand_and_water_is_owned_by_sand_alone`.
- **No silent caps.** `MAX_DYNAMIC_*` truncation now warns once, because a
  truncated frame is indistinguishable from a settled one from the outside — a
  pool simply stops growing, and that reads as a physics bug.

`the_grid_is_the_only_owner_of_matter` guards the definitions (not mentions of the
names — the doc comments say them out loud so the next reader knows what was
removed, and an occurrence-counting lint would fight its own explanation). Its
poison test assembles the needles at runtime so they never appear as literals in
the file, and checks the guard both fires on a reintroduction and stays quiet on
the module as it stands.

### What FS1 did NOT do

- **The CA is unaudited for conservation.** `tally_particles` proves the
  *projection* creates and loses nothing. Whether `bevy_falling_sand` itself
  conserves matter (§1's *"total per material = spawned − despawned, every tick"*
  against the room's real spec) needs a headless test that steps the CA, and
  `FallingSandPlugin` pulls the `render` feature. That is FS2's job, alongside the
  settle guarantee it already needs a stepping harness for.
- **The remaining pile-up weirdness is FS2's.** With the second representation
  gone, whatever is left is the CA's rules — settle, lateral flow, level-finding —
  which is exactly the slice named for them.

---

## 4. FS2+FS3 sand slice — adapt-vs-replace ruled: REPLACE, sand only (fable, 2026-07-20)

Jon's directive: *"Repair falling sand by landing one deterministic sand-only
FS2/FS3 vertical slice. Drive exactly one solver step per simulation tick;
prove finite settling and conservation; transfer settled sand into persistent
collision ownership atomically; and add a regression in the authored
falling-sand room."* The one-CA-step-per-sim-tick experiment from the GPT
round-6 queue resolved during design, from source, before any code:

**`bevy_falling_sand` 0.7.0 cannot be driven one step per sim tick without a
fork.** The evidence, all [root-caused] at the crate's source:

- The movement systems (`par_handle_movement_by_chunks` etc.) are **private**
  (`movement/processing/mod.rs` — `mod systems`, no re-export) and pinned to
  `PostUpdate`, so they cannot be re-homed into the sim schedule. One
  `PostUpdate` pass per render frame can never equal N sim ticks under
  fixed-tick catch-up or a GGRS replay — and it happily steps while the game
  is paused, because a render frame is not a sim tick.
- The single-step hook fires twice. `SimulationStepSignal` is consumed by
  nothing; the run condition `condition_msg_simulation_step_received` only
  peeks (`!is_empty()`), so with message double-buffering one signal keeps the
  gate open for TWO `PostUpdate` passes.
- `ChunkSystems::DirtyAdvance` gates on the free-run resource
  (`resource_exists::<ParticleSimulationRun>`), not the step signal, and an
  unpaired advance DROPS dirty state (`advance_frame` overwrites `current`) —
  so signal-driving starves the chunk movement path entirely.
- Determinism: parallel checkerboard chunk iteration by default, per-particle
  `MovementRng`, and Bevy `Query` iteration order underneath — three strikes
  against ADR 0023 in the crate's core loop.

So SAND — the material whose settled state becomes world geometry, i.e. the
one that must be correct — moved onto a bespoke deterministic grid CA:
`ambition_content::falling_sand_sim` (UNGATED, so its proofs run in every
`cargo test -p ambition_content`) with `sand_grid.rs` as the pure core.
**Water and oil stay on `bevy_falling_sand`** in the feature-gated
presentation module until their own slice; their known defects
(frame-locked stepping, no level-finding) are unchanged and explicitly out of
scope per the directive.

What landed, against §1's contract:

- **One representation, two owners, one door.** Loose sand = `SandCell::Sand`
  in `SandGrid`; settled sand = mass in `SettledSandLedger` (its cell becomes
  `Settled` geometry). `settle_into` is the only transfer and is atomic per
  cell — §1's *"compiled cells leave the grid (conservation moves them between
  owners atomically)"*, now literally a function.
- **Conservation:** `loose + settled == emitted`, checked by
  `conserved_with`, `debug_assert`ed every sim tick, asserted every tick in
  the unit tests and every observed tick in the room regression.
- **Settle guarantee:** proved as a fixed-point test (finite pour → quiescent
  within budget → ten further ticks move and transfer nothing → ledger total
  == emitted). The transfer condition — all three lower neighbors static —
  can only ever fossilize a grain the CA rules could never move again.
- **One solver step per sim tick:** `step_sand_grid` runs in the sim schedule,
  emission/stepping gated `simulation_pass_is_authoritative` (a replayed
  rollback frame must not double-advance un-registered state), while the
  ledger→overlay projection runs on EVERY pass so replayed player physics
  stands on the same ground.
- **Sand→geometry compilation:** the ledger contributes bottom-aligned,
  fill-proportional one-way blocks (`falling_sand:settled:<tx>:<ty>`) through
  the overlay each frame — the LEDGER is the persistence, the overlay stays a
  per-frame composition, which kills the transient-projection flicker (a
  truncated/thin frame can no longer un-ground a pile). Ledger-owned tiles
  veto water regions (single owner per tile, across representations).
- **Determinism:** no RNG, no entity iteration, no hash maps; scan order and
  diagonal preference are pure functions of (state, tick); pinned by an
  identical-runs test.
- **Authored-room regression** (`app_it::falling_sand_room`): enters
  `falling_sand_room` by semantic id, activates the authored sand switch by
  its authored id (no coordinates), then asserts emission → conservation →
  bounded-time settling → overlay ground → persistence across 30 rebuilds.
- **The visual ships with the slice** (draw-blind rule): a room-sized texture
  redrawn on grid ticks — loose grains in the old three-tone palette, settled
  ground a deeper tone. Blind feel constants, said so at their definitions:
  `SETTLED_BLOCK_MIN_CELLS = 64`, `FALL_CELLS_PER_TICK = 3`, emission budget
  120k grains (warned once when it closes the spout, never silent).

### What §4 deliberately did NOT do

- Water/oil correctness (level-finding, tick-locking) — still on the external
  crate, still FS2's open half. When that slice lands, the bfs-side sand
  plumbing left in the presentation module (`MaterialKind::Sand` arms,
  `project_sand`) dies with it; it currently sees zero sand particles.
- The spout-placement schema ([W-a]/[W-b]) — the mouth table moved crates but
  kept its shape.
- Re-fluidizing settled sand, drains, C4 gravity frames, Oiler.
