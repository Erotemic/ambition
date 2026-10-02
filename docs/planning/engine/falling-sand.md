# Falling sand — correctness first, then "Don't fuck with Oiler"

**Scope:** the correctness contract of the falling-sand room, the standing
block on fluids, and the parked Oiler design. The room is an engine
correctness customer: it must be deterministic, conserving and settling before
any feature uses it.

**Current shape.** Sand runs on a bespoke deterministic grid
(`ambition_content::falling_sand_sim`, pure core `sand_grid.rs`), which is
built and tested in every `cargo test -p ambition_content`. Water and oil run
on the external crate `bevy_falling_sand` (0.8) in a presentation module behind
the off-by-default `falling_sand` feature (`game/ambition_content/Cargo.toml`;
reached only by `ambition_app`'s own `falling_sand` feature). The crate is not
in the default resolve, so no shipped build contains the fluids.
`LiveSandRoom` finds the live room that instantiates the falling-sand room, so
the sand runs while several rooms are live.

## 1. The correctness contract (the actual engine work)

The falling-sand room is a **bounded, deterministic cellular automaton** with
an explicit conservation law:

- **One representation.** A particle exists in exactly one place: the grid.
  Pooled or settled matter is grid state (or, once compiled, RoomGeometry
  overlay solids), never also a live falling particle. A tile dense enough to
  be a sand solid never also becomes a water region.
- **Conservation:** total matter per material = spawned - despawned, every
  tick, asserted in a headless test that runs the room's real spec.
- **Settle guarantee:** any finite spawn input reaches a fixed point: every
  particle settles, pools or leaves through an authored drain or kill
  boundary.
- **Fluids find level:** water and oil equalize across connected basins (the
  lateral-flow CA rule with stable cell order, double-buffered).
- **Sand-to-geometry compilation** comes only from settled grid state, through
  the RoomGeometry overlay. Falling sand never mutates authored geometry.
  Compiled cells leave the grid; the transfer between owners is atomic.
- **Determinism:** fixed update order, RNG seeded from world state, gravity
  frame from `GravityCtx` (not -y). The module is a self-gating content plugin:
  the engine ships the CA substrate; the room ships as content.

**A spout is an authored placement.** A source that emits matter is an
authored placement in the map, not a hardcoded runtime spawn. It uses the same
world-to-sim lowering seam as any other placement: the map author places a
`spout` (material, rate, direction), `ambition_platformer2d_world` carries it
as an authored record, and the falling-sand content plugin registers the
interpreter that lowers it into the runtime emitter at room load.
`ambition_platformer2d_world` never names the falling-sand runtime. The schema
is falling-sand-specific: `SpoutSpec { material: String, rate: f32, direction:
[f32; 2] }` as a `PlacementSchema::Spout(..)` variant, registered through
`register_placement_interpreter`. Do not author a general "emitter" placement
until a second emitter-shaped placement exists. Today the spouts are a
`SpoutMouth { particle_type, x, y, width }` table of the same shape; the
placement variant does not exist yet.

Guards (`game/ambition_content/src/falling_sand/tests.rs`):
`the_grid_is_the_only_owner_of_matter`, its poison
`the_single_owner_guard_can_detect_a_reintroduced_representation`, and
`a_tile_dense_in_both_sand_and_water_is_owned_by_sand_alone`. Per-frame
conservation: `tally_particles` / `TallyLedger` (`debug_assert` that
per-material tile buckets sum to the ledger column). `SpawnParticleSignal` is
the only way matter enters. `MAX_DYNAMIC_*` truncation warns once.

## Blocker — water and oil are shelved on `bevy_falling_sand`

Jon's ruling: *"If this is impossible with bevy_falling_sand we can shelve it
for the time being... We will have to rewrite bevy_falling_sand if we want
netcode level determinism AND falling sand."*

- **Blocked:** the fluid half of §1 (level-finding, tick-locked stepping,
  determinism) on the external crate. Adaptation is ruled out: the crate's
  movement systems are private `PostUpdate` systems, its step signal fires
  twice, `DirtyAdvance` starves, and its core is parallel, RNG-driven and
  query-ordered. No configuration reaches around these.
- **Not blocked for fixed-tick or headless play:** sand (§4).
- **Blocked for netcode:** the room as a whole. Water and oil are on the
  frame-driven crate, and the sand grid and ledger are not rollback snapshots.
  Gating their advance to an authoritative pass prevents duplicate stepping but
  does not reconstruct historical sand state during a rewind. Do not claim
  falling-sand rollback correctness until an authorized fork or rewrite
  supplies one rollback-owned material model.
- **Until unblocked:** water and oil stay on `bevy_falling_sand` with their
  known defects and take no further correctness work.
- **The unblock is a rewrite decision:** a deterministic fluid CA under §1,
  either grown from the sand grid (lateral-flow rules over the same cell
  substrate) or a fork of the crate. Do not start it without an explicit
  go-ahead.

## 2. Oiler (parked ideas)

The character **Oiler** (Euler) uses the module: a special sprays large
volumes of oil (a spawner as a technique with parameters); pooled oil becomes a
**surface coating** that slows actors in it (a `Contact`-driven movement
modifier; add the coating vocabulary when Oiler lands); a second beat ignites
pooled oil, and flame spreads across connected coated cells, deals damage over
time and consumes the oil (oil -> fire -> gone). Prerequisites: the fluid
contract of §1, the technique parameter seam, and the combat presentation-facts
seam ([`combat-model.md`](combat-model.md)). Oiler is content (a catalog row
and techniques); only the surface-coating movement hook is engine vocabulary.

## 3. Single owner and conservation

The single-owner rule of §1 is the structural fix for matter that had two
homes (CA particles and a separate sprite fleet with its own gravity). The
sprite representation is gone; the guards in §1 defend the rule.

## 4. Sand on a bespoke grid: replace, sand only

`bevy_falling_sand` cannot be driven one step per simulation tick without a
fork. So sand, the material whose settled state becomes world geometry, runs on
the bespoke grid `ambition_content::falling_sand_sim`.

- **One representation, two owners, one door.** Loose sand is
  `SandCell::Sand` in `SandGrid`; settled sand is mass in `SettledSandLedger`
  (its cell becomes `Settled` geometry). `settle_into` is the only transfer,
  atomic per cell.
- **Conservation:** `loose + settled == emitted` (`conserved_with`),
  `debug_assert`ed every simulation tick.
- **Settle guarantee:** a fixed-point test (a finite pour becomes quiescent
  within a budget; ten more ticks move and transfer nothing; the ledger total
  equals the emitted total).
- **One solver step per ordinary simulation tick:** `step_sand_grid` and
  `emit_sand_into_grid` run in the simulation schedule under
  `simulation_pass_is_authoritative`, so a replay pass does not advance them
  twice. This is not a rollback snapshot.
- **Sand-to-geometry:** the ledger contributes bottom-aligned,
  fill-proportional one-way blocks (`falling_sand:settled:<tx>:<ty>`) through
  the overlay each frame. Ledger-owned tiles veto water regions.
- **Determinism:** no RNG, no entity iteration, no hash maps; scan order and
  diagonal preference are pure functions of state and tick; an identical-runs
  test pins it.
- **Authored-room regression:** `app_it::falling_sand_room` enters
  `falling_sand_room` by semantic id, activates the authored sand switch, then
  asserts emission, conservation, bounded settling, overlay ground and
  persistence across 30 rebuilds.
- **Feel constants:** `SETTLED_BLOCK_MIN_CELLS = 64`,
  `FALL_CELLS_PER_TICK = 3`, an emission budget of 120k grains (warns once when
  it closes the spout).

### What §4 does not do

- Water and oil correctness: blocked (see above). If a rewrite lands, the
  sand plumbing left in the presentation module (`MaterialKind::Sand` arms,
  `project_sand`, which sees zero sand particles) goes with it.
- The spout placement schema (§1).
- Re-fluidizing settled sand, drains, gravity frames other than down, Oiler.
