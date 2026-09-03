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
2026-07-20 — see §4; water/oil are SHELVED behind the hard blocker below**.
The spec above is the contract; the current code is small and may be boldly
restructured to meet it.

> **RE-MEASURED against `64adb1a8b` (2026-09-03), six weeks on: ✔ EVERY STATUS
> ON THIS PAGE IS STILL TRUE, AND THE ONE THAT MATTERS IS GUARDED.** Recorded
> rather than re-dated, because "still accurate" is a result a later reader
> should not have to re-derive.
>
> - **FS1's single-owner invariant is not just done, it is defended.**
>   `game/ambition_content/src/falling_sand/tests.rs` carries
>   `the_grid_is_the_only_owner_of_matter` **and**
>   `the_single_owner_guard_can_detect_a_reintroduced_representation` — a poison
>   test for the guard itself. ⇒ Jon's original symptom (matter pooling AND
>   falling at once) cannot silently return; something has to defeat a guard that
>   is itself proven to bite.
> - The sibling assertions cover the rest of the §1 bullet: every particle lands
>   in exactly one ledger column, a tile dense in both sand and water is owned by
>   sand alone, and thin matter *"projects nothing but is not lost"*.
> - **The hard blocker stands.** `bevy_falling_sand` is still the dependency
>   (`0.8`, `game/ambition_content/Cargo.toml:105`), so Jon's 2026-07-20 ruling
>   has not been overtaken by anything in this tree.
>
> ⓘ **One fact worth having beside the blocker: none of this is in a shipped
> build.** The room is behind an off-by-default feature all the way up —
> `ambition_content`'s `default = []` with `falling_sand = ["dep:bevy_falling_sand"]`,
> reached only by `ambition_app`'s own `falling_sand` feature. ⇒ Shelving costs
> nothing at runtime, and the determinism problem cannot reach a player today.

## ⛔ HARD BLOCKER — water/oil SHELVED on `bevy_falling_sand` (Jon, 2026-07-20)

Jon's ruling, on reading §4's evidence: *"If this is impossible with
bevy_falling_sand we can shelve it for the time being... We will have to
rewrite bevy_falling_sand if we want netcode level determinism AND falling
sand."*

- **What is blocked:** the fluid half of §1's contract — water/oil
  level-finding, tick-locked stepping, determinism — on the external crate.
  Adaptation is ruled out, not deferred: the §4 findings (private `PostUpdate`
  movement systems, a step signal that fires twice, `DirtyAdvance`
  starvation, parallel+RNG+query-ordered core) are structural, and no amount
  of configuration reaches around them.
- **What is NOT blocked for ordinary fixed-tick/headless play:** sand. It
  already left the crate and meets the conservation/settling/geometry contract
  on the bespoke grid (`falling_sand_sim`, §4).
- **What remains blocked for netcode:** the room as a whole. Water/oil still
  live on the external frame-driven crate, and the bespoke sand grid/ledger are
  not rollback snapshots. Gating their advancement to an authoritative pass
  prevents duplicate stepping but does not reconstruct historical sand state
  during a rewind. Do not claim falling-sand rollback correctness until the
  explicitly authorized fork/rewrite supplies one rollback-owned material
  model.
- **Standing until unblocked:** water/oil stay on `bevy_falling_sand` in the
  feature-gated presentation module exactly as they are — known defects and
  all — and take **no further correctness work** on that path. Fixing them
  there would be sunk cost against a dead end.
- **The unblock is a rewrite decision, not a task:** if/when Jon wants
  netcode-level fluids, the work is a deterministic fluid CA under Ambition's
  contract — either grown from the sand grid (which is the proof-of-shape:
  lateral-flow rules over the same cell substrate, double-buffered per §1) or
  a ground-up fork of the crate. Do not start it without an explicit
  go-ahead; do not "improve" the bfs path in the meantime.

**The SPOUT is the canonical authored-placement example (Jon, 2026-07-06).**
A falling-sand spout (a source that emits matter) is an **authored PLACEMENT
in the map**, not a hardcoded runtime spawn — it rides the same
world→sim lowering seam as any other contentful placement (architecture
§4b): the map author drops a `spout` placement (a Tier-0 authored schema:
material, rate, direction), `ambition_platformer2d_world` carries it as an authored
record, and the falling-sand CONTENT plugin registers the interpreter that
lowers it into the runtime emitter at room-load. So the same-tier deps hold:
`ambition_platformer2d_world` never names the falling-sand runtime; the content plugin
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

Root cause: `emit_falling_sand_spouts` wrote `SpawnParticleSignal`s into the CA
grid *and* spawned a parallel fleet of `FallingSandStreamParticle` sprites with
their own hardcoded gravity (on `Res<Time>`, not `WorldTime`) that ignored room
geometry and despawned at an invented floor. Matter had two homes: the sprites
fell straight through the platforms the real CA particles were pooling on —
Jon's report, verbatim: *"water and oil pool on the top platform yet particles
ALSO fall forever below."*

Fix (structural, per §1's single-owner rule): the stream-sprite representation
is deleted; `SpawnParticleSignal` is the only way matter enters. Spouts are now
a `SpoutMouth { particle_type, x, y, width }` table — the same shape the ruled
`PlacementSchema::Spout` will carry ([W-a]/[W-b]) <!-- cite-ok: PROPOSED variant, does not exist yet -->. Tile ownership is exclusive
too: a tile dense enough to be a sand solid never also becomes a water region,
pinned by `a_tile_dense_in_both_sand_and_water_is_owned_by_sand_alone`. Silent
truncation is gone — `MAX_DYNAMIC_*` truncation now warns once instead of
reading as a pool that mysteriously stopped growing.

Guard: `the_grid_is_the_only_owner_of_matter` poison test. Conservation is
checked every real frame via `tally_particles`/`TallyLedger` (`debug_assert`
that per-material tile buckets sum to the ledger column).

Not done by FS1: whether `bevy_falling_sand` itself conserves matter tick-over-tick,
and the settle/lateral-flow/level-finding rules — that's FS2, §4.

---

## 4. FS2+FS3 sand slice — adapt-vs-replace ruled: REPLACE, sand only (fable, 2026-07-20)

Jon's directive: *"Repair falling sand by landing one deterministic sand-only
FS2/FS3 vertical slice. Drive exactly one solver step per simulation tick;
prove finite settling and conservation; transfer settled sand into persistent
collision ownership atomically; and add a regression in the authored
falling-sand room."*

`bevy_falling_sand` 0.7.0 cannot be driven one step per sim tick without a
fork — root-caused at the crate's source; see the hard-blocker evidence
above. So SAND — the material whose settled state becomes world geometry —
moved onto a bespoke deterministic grid CA: `ambition_content::falling_sand_sim`
(UNGATED, runs in every `cargo test -p ambition_content`) with `sand_grid.rs`
as the pure core. Water and oil stay on `bevy_falling_sand` per the hard
blocker; their known defects are unchanged and out of scope for this slice.

What landed, against §1's contract:

- **One representation, two owners, one door.** Loose sand = `SandCell::Sand`
  in `SandGrid`; settled sand = mass in `SettledSandLedger` (its cell becomes
  `Settled` geometry). `settle_into` is the only transfer, atomic per cell.
- **Conservation:** `loose + settled == emitted`, checked by `conserved_with`,
  `debug_assert`ed every sim tick.
- **Settle guarantee:** proved as a fixed-point test (finite pour → quiescent
  within budget → ten further ticks move and transfer nothing → ledger total
  == emitted).
- **One solver step per ordinary sim tick:** `step_sand_grid` runs in the sim
  schedule, gated by `simulation_pass_is_authoritative` to prevent duplicate
  advancement on a replay pass. This is explicitly **not** a rollback
  snapshot — the room remains outside netcode acceptance under the hard
  blocker above.
- **Sand→geometry compilation:** the ledger contributes bottom-aligned,
  fill-proportional one-way blocks (`falling_sand:settled:<tx>:<ty>`) through
  the overlay each frame; ledger-owned tiles veto water regions (single owner
  per tile, across representations).
- **Determinism:** no RNG, no entity iteration, no hash maps; scan order and
  diagonal preference are pure functions of (state, tick); pinned by an
  identical-runs test.
- **Authored-room regression** (`app_it::falling_sand_room`): enters
  `falling_sand_room` by semantic id, activates the authored sand switch by
  its authored id, then asserts emission → conservation → bounded-time
  settling → overlay ground → persistence across 30 rebuilds.
- **Visual:** a room-sized texture redrawn on grid ticks. Feel constants:
  `SETTLED_BLOCK_MIN_CELLS = 64`, `FALL_CELLS_PER_TICK = 3`, emission budget
  120k grains (warns once when it closes the spout, never silent).

### What §4 deliberately did NOT do

- Water/oil correctness (level-finding, tick-locking) — still on the external
  crate, and now **SHELVED behind the hard blocker above** (Jon, 2026-07-20):
  no further work on the bfs path; the unblock is a rewrite decision. When a
  rewrite ever lands, the bfs-side sand plumbing left in the presentation
  module (`MaterialKind::Sand` arms, `project_sand`) dies with it; it
  currently sees zero sand particles.
- The spout-placement schema ([W-a]/[W-b]) — the mouth table moved crates but
  kept its shape.
- Re-fluidizing settled sand, drains, C4 gravity frames, Oiler.
