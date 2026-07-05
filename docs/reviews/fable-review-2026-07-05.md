# Fable review — 2026-07-05: space, momentum, and the mounted giant

**Authored by fable** from Jon's 2026-07-05 direction (three directives, below)
plus three fresh deep audits of the code as it stands today: the collision
kernel + every AABB/global-axis assumption site, the post-R3 world seam, and
the mount/gnuton/moveset/brain surfaces.

**What this doc is:** the execution extension of
[`fable-review-2026-07-04.md`](fable-review-2026-07-04.md). That doc remains
live for R1–R6 (R1–R3 landed; R4–R6 stand). This doc adds adjudications
AJ8–AJ12 and phases **R7–R10** in the same numbering space, and **reshapes
R4b** (the `ambition_world` carve) — see AJ9. Jon's design input is captured
verbatim in
[`../planning/engine/spatial-model.md`](../planning/engine/spatial-model.md)
(binding, ADR-0020-style); this doc turns it into slices.

**Executor rules:** every slice carries a difficulty flag. **[opus]** =
executable by an opus-quality agent from this doc alone. **[★fable]** = design
risk or kernel-deep work — Jon schedules these with fable first; opus agents
must NOT attempt them (a wrong contact model is expensive to unwind). The
handoff rules of the 07-04 doc (§6) apply unchanged: commit each verified
slice, explicit paths, BLIND-marked feel commits, C4 scenario for every new
reaction seam, keep this doc's log current.

---

## 1. Jon's three directives (the inputs)

1. **Authoring-backend-agnostic space.** LDtk is the current map backend, not
   the engine ontology. Ambition owns a canonical spatial model; editor
   backends (LDtk first and only, for now) lower into it. AABB stays the
   protected fast path. Full text: `spatial-model.md`.
2. **Momentum-based locomotion, expedited.** "Sanic" (Sonic parody) enters the
   sandbox with a real momentum/surface moveset — slopes, loops, contact
   frames. This answers roadmap **Q6 (slopes): YES** — recorded in
   `roadmap.md`; Sonic's demo-matrix row is upgraded. The engine upgrades must
   not fork the engine and must coexist with (and help) the R4 decomposition.
3. **Gnuton becomes a mounted boss.** The giant gnu is the MOUNT (with the
   crazy multi-limb ability set); gnuton is the RIDER driving it through the
   ADR 0020 control grant. Today they are one authored sprite; the plan splits
   them. This absorbs and reframes the
   [`multi-limb-bosses.md`](../planning/engine/multi-limb-bosses.md) draft.

## 2. THE STATE delta since 07-04 (measured today)

- **Mount cutover is essentially DONE**, ahead of the 07-04 doc's "Phase B
  remaining" note: B1 (`5e4d6448`, shark encounters as linked mount+rider
  pairs in the `.ldtk`s) and B2 (`16c057d9`, fused-composite path deleted)
  are committed. Grep confirms zero live `composite_visual` /
  `spawn_composite_mount_rider` / `is_composite` code. The working tree
  carries further uncommitted mount cleanup (deleted
  `mounted_rider_brain_and_action_set`, archetype row removal) — an in-flight
  slice by another agent; this review does not touch it. **M5
  (player-piloting) is LANDED (2026-07-05, opus)** — the mount coupling
  (`sync_riders_to_mounts` / `enforce_mount_rider_link`) was made
  controller-agnostic by dropping its player-centric `is_hostile()` gate (a
  mount that "only obeys enemies" violated the relativity principle); coupling
  now keys on structural liveness + role components, so a `Brain::Player` rider
  pilots the mount identically to an AI rider. Two tests pin it: a deterministic
  mount-module test (`a_player_controlled_rider_pilots_the_mount_agnostically`)
  and an end-to-end sim test (`player_pilots_mount_end_to_end.rs`: possess-style
  brain handover → `move_x` drives the MOUNT, home avatar stays put, rider stays
  welded). The R10.6 payoff is now unblocked.
- The R3 exit greps and R4a-1 stand as logged in the 07-04 doc. `ambition_world`
  still does not exist; R4b remains scouted-not-started — which is exactly the
  luck this review needs (AJ9 reshapes it before anyone carves).
- One correction to the standing folklore: **gnuton does not run the smash
  brain.** It is `Brain::StateMachine(BossPattern)` with a `Scripted` pattern
  (`boss_profiles.ron:325`); "smash" is the goblin/PCA template. The PCA
  encounter plan ("EXTEND the smash brain") is unaffected; this doc's gnuton
  plan targets the scripted `BossPattern`.

## 3. FABLE ADJUDICATIONS (continuing the 07-04 series)

### AJ8. Q6 answered: momentum/slopes/loops are IN; "Sanic" is the stress vector

Roadmap Q6 asked whether slopes/curved terrain are in the engine's 1.0
capability set. Jon's answer (2026-07-05): **yes, expedited** — with the
crucial reframe from `spatial-model.md`: this is not "add slopes to the AABB
kernel," it is "grow a geometry/contact layer in which AABBs and richer
surfaces coexist." The demo-matrix Sonic row moves from Edge tier to an
active stress vector; the near-term deliverable is deliberately small (one
Sanic body, one sandbox room, slopes + a loop, coexistence with a knight-like
body, debug overlays) — a proof of possibility and elegance, not a game.

Two protections are binding:

- **AABB is protected, not absolute.** Every current world and body keeps the
  existing axis-role swept-AABB path, byte-identical and fast. Richer geometry
  is opt-in per body and per room. R8.4 pins this with tests.
- **No engine fork.** The momentum body is a typed per-body policy inside the
  ONE actor pipeline (the `Perception::Omniscient/Sighted` precedent from
  R1.2b — a policy enum, not a parallel system), never a second movement
  engine.

### AJ9. The spatial IR: name what exists, split the backend out of the carve

**The IR already exists in embryo — the work is to NAME it, complete it, and
put LDtk on the far side of it.** Today's pipeline is already two-stage
(audit result):

```text
LDtk JSON → parse types (ldtk_world/project.rs — hand-rolled serde)
         → converter registry (identifier → fn(&LdtkEntityCtx) → RuntimeEntityEmission)   [R3.1a]
         → RuntimeEntityEmission { blocks, zones, water, platforms, camera_zones,
                                   props, portals, shrines, gravity_zones, hazards,
                                   interactables, pickups, chests, breakables,
                                   enemy_spawns, boss_spawns, mount_links, … }
         → RoomSpec / RoomSet (+ Authored<T> families)                                     [the IR]
         → staging → ECS
```

`RoomSpec`/`RoomSet`/`Authored<T>`/`Block`/`World` contain **no LDtk types**.
The only genuinely LDtk-shaped things are the parse types, the converter ctx,
the IntGrid emitters, hot-reload, and the `bevy_ecs_ldtk` tile-render spine.
So the manifesto's "canonical spatial IR" is ~80% a boundary-drawing exercise,
not new invention. Decisions (binding):

- **The IR keeps its current names.** `RoomSpec`, `RoomSet`, `Authored<T>`,
  the spec structs, `Block`/`World`/`RoomGeometry` (engine_core) ARE the
  canonical vocabulary — do not rename for ceremony. `RuntimeEntityEmission`
  moves IR-side (it is backend-neutral output) and may rename to
  `RoomEmission` in the move; `LdtkEntityCtx` + the registry implementation
  stay backend-side. A future Tiled backend ships its own ctx + registry that
  emits the same emission type.
- **R4b is reshaped: the carve produces TWO crates, not one.**
  `ambition_world` = the canonical spatial model + room graph + staging +
  platforms + manifest core + validators (Tier 2, no LDtk dep).
  `ambition_ldtk_map` = the LDtk backend (parser, converter registry impl,
  IntGrid emitters, hot-reload, the `bevy_ecs_ldtk` render spine, LDtk-source
  manifest rows) — an optional crate the GAME brings in (content/app dep,
  never an `ambition_world` dep). This is the manifesto's
  `ambition_ldtk_map / ambition_tiled_map / ambition_godot_map` shape with
  only the first implemented. **Do not carve `ambition_world` to the old
  one-crate shape and split later — carve once, to this shape**
  (reorganize-don't-adapt).
- **`WorldManifest` generalizes by one notch, no more.** `WorldSource` today
  assumes `.ldtk` text. Add a backend key (a `format: String` row field
  resolved through a small installed importer registry, same OnceLock
  pattern as everything else) with exactly two entries for now: `ldtk` and
  `ron-room` (native IR rooms serialized as RON — this is what makes
  "generated IR for sandbox tests" and the Sanic demo's hard-coded-geometry
  option real, and it costs almost nothing because `RoomSpec` is plain data).
- **Provenance (source metadata) is a field, not a system.** Authored rows and
  `Block`s gain a cheap `SpatialSource` tag (`Ldtk{iid}`, `Generated{tool}`,
  `Native{file}` — a small enum with a string payload). Debug overlays and
  validators read it. This also kills the one render-side leak found by
  audit: `ambition_render/src/rendering/world.rs:407` sniffs
  `block.name.starts_with("ldtk ")` to detect IntGrid blocks — replace the
  string sniff with the provenance field.
- **The leakage ratchet** (audit list, the R7.4 exit): outside the importer,
  gameplay_core still touches `crate::ldtk_world::*` in `encounter/loading.rs`
  (reads raw `LdtkProject` for `EncounterTrigger`/`StitchedBoundary`/
  `LockWall` — finish their converters so encounters consume emissions),
  `menu/map/systems.rs` + `encounter/systems.rs` (`SandboxLdtkProject`),
  `session/setup.rs` (`LdtkRuntimeIndex`), `persistence/settings`
  (`LdtkHotReloadState`/`LdtkAutoApply` — generalize to "world source
  hot-reload"), `assets/*` (manifest iteration — fine once the manifest is
  backend-keyed), plus the schedule-set name `LdtkRuntimeSpine`. Each is a
  bounded inversion; the exit grep is `rg -i 'ldtk'` over engine crates
  hitting only `ambition_ldtk_map` + install sites.
- **ADR:** this adjudication + the manifesto spawn **ADR 0021 —
  authoring-backend-agnostic space** (refines ADR 0017's "LDtk is for space"
  to "map authoring backends are sources of spatial content; Ambition owns
  the canonical spatial model"). Write it when R7.3 lands, citing both docs.

### AJ10. The geometry/contact kernel: contacts first, chains second, one new solver

This is the deep one. The audit's headline findings make it much cheaper than
feared:

- **The narrowphase already computes real contact normals and throws them
  away.** parry's `cast_shapes` fills `AabbSweepHit.normal1`
  (`engine_core/src/geometry.rs:82`) → `SweepHit.normal1` (`world.rs:305`),
  and the resolvers ignore it, recomputing axis-aligned face pushes
  (`movement/collision.rs:46`, `kinematic.rs:292`). The only consumer today is
  `apply_side_contact` (wall-normal x).
- **parry2d is already a dependency of the movement path itself** (not just
  combat) — `geometry.rs` sweeps are parry casts; `CombatVolume` already does
  circle/OBB/convex narrow-phase. No new dependency, no new math library.
- **`AccelerationFrame` already supports arbitrary-angle frames**
  (`reference_frame.rs:228`); `cardinalized` (`:243`) is an opt-in policy
  layer. The four-cardinal-cones rule is a choice the AABB solver makes, not
  a kernel limit.
- **The assumption inventory is bounded.** The global-axis/AABB sites are:
  `Axis{X,Y}`/`gravity_axis` cardinal collapse + `perpendicular_overlap`
  (`collision_semantics.rs:49–176`), the per-axis sweep order
  (`movement/integration.rs:141`), `axis_face_resolution`/`axis_span`
  (`movement/collision.rs`), `cardinal_gravity` (`kinematic.rs:186`),
  `to_world_half`'s cardinal-exact bound, `aabb_oriented`'s width/height swap,
  one-way-on-gravity-axis (`collision_semantics.rs:97–121`), `water_at`/
  `climbable_at` global-axis scalars (`world.rs:339–380`), ledge world-bounds,
  the moving-platform carry projection, and `PortalFrame`'s cardinal normals
  (explicit `FIXME(portal-api)`, `pieces.rs:24`). **None of these get
  rewritten in R8** — they are the protected AABB path. R8 builds the general
  vocabulary BESIDE them.

**The binding design (three layers, in dependency order):**

1. **Contact vocabulary** (`collision_semantics.rs`, the shared pure kernel —
   both sweeps inherit it). A `Contact` struct is the lingua franca:

   ```rust
   pub struct Contact {
       pub point: Vec2,
       pub normal: Vec2,          // unit, out of the surface
       pub tangent: Vec2,         // normal rotated -90°, consistent winding
       pub toi: f32,              // along the attempted motion
       pub surface_velocity: Vec2,// the surface's own frame motion (Block.velocity precedent)
       pub source: ContactSource, // Block index/name | Chain { id, segment, s }
   }
   ```

   Slice one routes the EXISTING sweeps' parry normals into `Contact`s
   without changing any resolution (observability first, byte-identical —
   resolvers still act on axis faces). The manifesto's principle lands here:
   *the world exposes coherent contact information; bodies decide what it
   means.* The knight body's floor/wall/ceiling reading stays the
   `AxisRole`+feet model (its interpretation); the momentum body reads the
   same contacts through tangents.

2. **`SurfaceChain` — the first richer primitive** (engine_core `world.rs`,
   beside `Block`):

   ```rust
   pub struct SurfaceChain {
       pub name: String,
       pub points: Vec<Vec2>,     // polyline; winding defines the solid side
       pub closed: bool,          // loops close the chain
       pub kind: SurfaceKind,     // Ground for slice 1; semantics grow later
       pub velocity: Vec2,        // per-frame frame motion, like Block.velocity
       pub source: SpatialSource, // AJ9 provenance
   }
   ```

   `World` gains `chains: Vec<SurfaceChain>`; rooms with zero chains take the
   existing borrow fast path (the `world_with_sandbox_solids` pattern
   already does this for overlays). One-sided by winding; normals derived,
   never authored (a validator checks joins + winding — the manifesto's
   "inverted normals / discontinuous joins masquerade as physics bugs").

3. **The surface-follower solver** — the ONE new mover, for momentum bodies
   only (slice 1). State machine per body:
   `Airborne | Riding { chain, segment, s, v_t }` (arc-length parameterized —
   while riding, integration is 1-D along the chain: elegant, deterministic,
   headless-testable). Collision proxy while riding is a **circle** (radius =
   `kin.size.min_element()/2`) — a circle rolls cleanly through chain joints
   and gives an unambiguous tangent contact; the body's `kin.size` AABB stays
   authoritative for EVERYTHING else (hurtboxes, triggers, portals, camera,
   `CenteredAabb`), which is what keeps the whole downstream engine
   unchanged. Physics: gravity projects onto the tangent (slope accel =
   `g·t̂`), stick condition = classic speed-vs-normal-load test
   (`v_t² / r ≥ stick_factor · max(0, g·(-n̂))` on curvature, plus a min-speed
   threshold on inverted surfaces); unstick → ballistic `Airborne` with the
   tangent velocity carried out; landing re-sticks by swept circle-vs-chain
   with the SAME no-artificial-snap guard discipline as
   `is_contact_range_snap`. Moving chains: contact carries
   `surface_velocity`; the carry falls out of the contact frame instead of a
   special case. **M10 (no pushout) binds fully**: swept TOI + slide, never
   positional ejection.

**Coexistence rules (slice 1, deliberate):** chains collide ONLY with
momentum bodies; axis-swept bodies ignore them (the knight walks the AABB
floor of the same room). Extending chain contact to axis-swept bodies —
Celeste-lite ramps for knight-likes — is a natural later pressure that rides
the SAME `Contact` vocabulary; it is explicitly out of scope until the
follower solver has proven the model. Angled portals and portal-on-moving-
platform are the same one-pressure family (frames + normals + relative
motion); they queue behind this arc on the `PortalFrame` FIXME seam, not
inside it.

**Compile-time note for executors:** all three layers live in
`ambition_engine_core` — the Tier-1 crate everything rebuilds on. Iterate
with `cargo test -p ambition_engine_core` (fast, doesn't rebuild the world);
touch gameplay_core integration only when the kernel suite is green.

### AJ11. Movement identity is body DATA: the `MotionModel` policy

The manifesto's stable principle — *movement identity travels with the
possessed body; capabilities belong to bodies* — is already the codebase's
own law (M1/M2, R2.5: "whatever character is chosen behaves like the
character"). The mechanism follows the R1.2b `Perception` precedent exactly:
a typed per-body policy, default = today:

```rust
#[derive(Component, Default)]
pub enum MotionModel {
    #[default]
    AxisSwept,                     // today's path, byte-identical, most bodies
    SurfaceMomentum(MomentumParams), // the follower solver drives this body
}

pub struct MomentumParams {        // RON-authorable on the archetype row
    pub ground_accel: f32, pub brake: f32, pub friction: f32,
    pub slope_factor: f32, pub top_speed: f32,
    pub air_accel: f32, pub jump_speed: f32,
    pub stick_factor: f32, pub min_stick_speed: f32,
}
```

- A body WITHOUT the component is `AxisSwept` — zero migration, zero cost.
- The integrator dispatches on the policy inside the ONE shared
  `integrate_actor_body` path (a policy field like `gravity_dir`, NOT a
  parallel system — the R1 lesson).
- The controller seam is unchanged: `ControlFrame` in, body enforces. The
  momentum body INTERPRETS the same frame (x-axis = accelerate along its
  tangent, jump = leave the surface along the normal, per Jon's
  frame-of-reference notes) — the two-port model absorbs the new identity
  without a new port.
- Possession/mounting compose for free: possess Sanic → you move like Sanic
  (brain transfer, body identity stays); a momentum-body MOUNT would give its
  rider momentum locomotion through the existing grant. No new seams.
- `MomentumParams` enters through the archetype RON row (the character
  catalog install seam), so a second game's momentum character is pure data.

### AJ12. The mounted giant: gnuton (rider, boss brain) drives the gnu (mount, limbs)

The multi-limb draft asked "should gnuton's hands be their own actor?" and
answered "coordinated limbs — own body, shared brain," flagging ONE genuinely
new mechanism: *a coordinator writing other entities' `ActorControl`*. **That
mechanism now exists**: `steer_mount_from_rider` (`mount/mod.rs:275`, C1) is
exactly it, 1→1. Jon's reframe completes the picture and dissolves the
draft's one open modeling question (where the coordinator lives):

```text
gnuton  = the RIDER  — small actor, CanPilot(["gnu"]), Brain::StateMachine(BossPattern),
                       BossConfig + BossEncounter + the boss HP pool + the hurtbox
giant gnu = the MOUNT — Mountable { class: "gnu" }, its own big BodyHealth,
                       a real mover (StationaryGiant dies), and the LIMB RIG:
                       two hand bodies linked to it, driven through the grant
```

- **The coordinator is the rider's brain, through `ControlGrant`.** The draft's
  head-core-as-coordinator is superseded: the thing that "also happens to be a
  visible body" is gnuton himself, riding. The gnu grants Total control;
  gnuton's scripted pattern steers the gnu's body AND choreographs its limbs.
  This is Jon's sentence made structural: *the gnu has the crazy multi-limb
  ability set, which the gnu allows gnuton to drive.*
- **The limb rig is a MOUNT capability, not boss machinery** — which resolves
  the draft's "hold off generalizing until a second multi-limb boss lands"
  tension the right way: the general thing is "a body with driven limbs whose
  pilot fans intent out to them." A mech with arms is the same component set.
  Engine ships the rig; the gnu is content.
- **Fan-out generalizes `steer_mount_from_rider` from 1→1 to 1→N.** The pilot
  brain writes a per-limb intent table; a fan-out system copies each limb's
  intent into that limb's `ActorControl` (limbs sorted by stable id —
  query-order determinism rule). Sketch (executor refines):

  ```rust
  pub struct LimbRig { pub limbs: Vec<Entity> }          // on the mount; spawn-ordered
  pub struct Limb { pub of: Entity, pub slot: LimbSlot } // on each limb body
  pub enum LimbSlot { HandL, HandR /* grows */ }
  #[derive(Component, Default)]
  pub struct LimbIntents(pub BTreeMap<LimbSlot, ActorControlFrame>); // written by the pilot's brain tick
  // fan_out_limb_intents: after tick_boss_brains/steer_mount_from_rider,
  // before integrate_sim_bodies — copies intents onto limb ActorControl.
  ```

  Limb bodies follow the draft exactly: gravity-free `flight_direct_velocity`
  movers with `ActorControl` + `ActorMoveset`, **no `Brain`, no `BossConfig`,
  no `BodyHealth`** (damageable limbs stay the reserved knob). The downstream
  sim is already N-body-safe (draft's research finding, still true:
  `FollowOwner` hitboxes re-resolve per tick; damage attributes via
  `Hitbox.owner`; `MovePlayback` per-entity singletons are WHY two hands
  attacking at once = two entities).
- **Two HP pools, ADR 0020 semantics, and the fight gets BETTER:** the
  encounter keys on the RIDER (gnuton is the boss; today's head-only hurtbox
  becomes gnuton's own hurtbox — the `scholar` anchor in the rig is literally
  the rider figure). The gnu has its own big pool and `body_damage > 0` at
  last (the body stops being non-interactive scenery). Kill the gnu first →
  `MountDeathImpact::Dismount` → gnuton, tiny and furious, fights on foot (a
  free phase with real pathos). Kill gnuton on the gnu's shoulders → the gnu,
  brainless of purpose, is a normal `Mountable` actor again (wanders,
  possibly tameable later — M5 makes it PLAYER-pilotable: possess/board the
  giant and drive the limbs yourself, the single most expressive payoff in
  the plan).
- **Everything bespoke about gnuton dies:** per-frame `left_hand`/`right_hand`
  hitbox tables in the sprite RON, `HAND_SLAM`/`HAND_SWEEP` StrikeRects,
  `sync_boss_split_overlay` + `BossOverlayLayer` + split z-consts,
  `StationaryGiant`, `body_damage: 0`. Hands strike through real
  `MoveSpec`s — which also makes gnuton the first boss fully on the moveset
  runtime, advancing the 07-04 A1 tail (BossAnim→CharacterAnim) instead of
  fighting it.
- **No feel to preserve** (draft's own verdict): gate = compiles + drives the
  real sim + the boss suites pass with retargeted assertions; the expression
  work (limb arcs) ships BLIND for Jon's pass.

## 4. THE ROADMAP EXTENSION — R7–R10

Ordering logic: R8 unblocks R9 (Jon's expedite) and is fable-led kernel work
that touches Tier 1 only — it can start immediately and conflicts with
nothing in R4. R7 replaces R4b in the carve stream and is opus-safe,
long-running, parallel. R10 is opus-executable content+mount work gated only
on the mount cutover (landed) and R10.1's mechanism (specced above). Nothing
here blocks R4c–R4g, R5, or R6.

### R7 — the spatial IR + the backend split (subsumes R4b) [opus, ~3–4 sessions]

- **R7.1 [opus]** The ~13 `rooms` upward-dep inversions, exactly as scouted in
  the 07-04 doc's R4b starting map (each a compiling, committable step).
  Unchanged by this review.
- **R7.2 [opus]** Name the IR in place (pre-carve, so the carve is a move not
  a redesign): relocate `RuntimeEntityEmission` (→ `RoomEmission`) + the fold
  (`compose_runtime_area`) out of `ldtk_world::conversion` into
  `world::rooms` (IR-side); leave `LdtkEntityCtx` + registry impl behind.
  Add `SpatialSource` provenance to `Block` + `Authored<T>` + chains
  (default `Unknown`, LDtk emitters stamp `Ldtk{iid}`); replace render's
  `"ldtk "` name-sniff with it. Make `RoomSpec` serde-round-trippable and add
  the `ron-room` manifest format (importer registry, two entries — see AJ9).
  Gate: full workspace + a new round-trip test (RoomSpec → RON → RoomSpec).
- **R7.3 [opus]** The carve, to the TWO-crate shape: `ambition_world` (IR,
  rooms, staging, platforms, physics adapter, gravity zones, manifest core,
  validators) + `ambition_ldtk_map` (parser, converter registry impl, IntGrid,
  hot-reload, `bevy_ecs_ldtk` spine, LDtk manifest rows). Repoint the
  139-inbound spine (D2 facade-then-delete template); content/app take the
  `ambition_ldtk_map` dep; `ambition_world` must not. Record compile-time
  before/after (the carve's purchase).
- **R7.4 [opus]** The leakage ratchet (AJ9 list): encounter loading consumes
  emissions not `LdtkProject`; menu-map/session/settings inversions;
  schedule-set rename. Exit: `rg -i 'ldtk'` over engine crates → only
  `ambition_ldtk_map` + install sites. Then **[opus]** write ADR 0021 citing
  `spatial-model.md` + this doc.

### R8 — the contact/surface kernel [★fable core, ~2 sessions]

- **R8.1 [★fable]** `Contact` vocabulary in `collision_semantics.rs`; both
  existing sweeps populate it from the parry normals they already compute;
  ZERO resolution change (byte-identical, pinned by replay fixtures + C4
  rigs). `apply_side_contact` becomes the first reader.
- **R8.2 [★fable]** `SurfaceChain` in engine_core `World` + winding/join
  validator + the debug overlay (normals, tangents, joins, support state —
  draw-blind rule: the overlay ships with the primitive, not after).
- **R8.3 [★fable]** The surface-follower solver (AJ10 layer 3): circle-proxy
  swept contact vs chains+blocks, arc-length riding, stick/unstick, gravity
  tangent projection, moving-chain frame velocity, no-pushout discipline.
  Headless suite in-crate: ramp accelerate/decelerate symmetry, valley
  oscillation energy decay, loop completion above threshold speed / fall
  below it, launch-angle parity off a ramp, and the C4 rotation rig (whole
  room + chains + gravity rotated 90° ⇒ identical trajectories in the rotated
  frame — the strongest test we own).
- **R8.4 [opus]** AABB-protection net: zero-chain rooms borrow the fast path
  (assert no chain code executes); perf pin on a representative room; replay
  fixtures + full boss/duel suites green; grep-ratchet that no existing
  resolver imports the follower.

### R9 — momentum locomotion + the Sanic sandbox [mixed, ~2 sessions]

- **R9.1 [★fable]** `MotionModel` policy component + `MomentumParams` (AJ11);
  dispatch inside the shared integrate path; `ControlFrame` interpretation
  for the momentum body; archetype-RON plumbing (catalog row →
  `MotionModel::SurfaceMomentum` at spawn). Possession test: possess Sanic,
  assert the controlled body still rides (movement identity travels).
- **R9.2 [opus]** Sanic the character: catalog row + archetype (momentum
  params authored in RON), sprite via the existing Python generator toolkit
  (blue-hedgehog PARODY — original silhouette, not a copy; draw blind, ship
  the sheet + Idle row per the sprite invariants), playable via
  `AMBITION_START_CHARACTER=sanic`.
- **R9.3 [opus]** The sandbox room, BOTH authoring paths (each proves an AJ9
  claim): (a) an LDtk `SurfaceChain` entity (point-array field → converter →
  chains; `ambition_ldtk_tools` gains `surface add/validate` subcommands —
  never hand-edit) authoring the slopes/valley; (b) a `ron-room` native-IR
  room (or generated by a small script) authoring the LOOP — hard-coded demo
  geometry in content/app, NEVER in engine core. One knight NPC placed in the
  same room (coexistence).
- **R9.4 [opus]** Demo proof: scripted reachability tests (loop at speed,
  fail-below-threshold, slope round-trip), the coexistence test (knight and
  Sanic fight in the room — combat stays AABB/CombatVolume, unaffected), and
  the debug-overlay screenshot artifact for Jon's visual pass (BLIND-marked).
- Roadmap bookkeeping: Sonic's matrix row upgrades on R9 exit; the full
  "Sanic game" (enemies, zones, rings-analog) stays post-1.0 unless Jon
  promotes it (Q13, §7).

### R10 — the mounted giant [mostly opus, ~2–3 sessions; one ★fable slice]

- **R10.1 [★fable]** The limb rig + fan-out (AJ12 sketch): `LimbRig`/`Limb`/
  `LimbIntents` + `fan_out_limb_intents` scheduled between the brain tick and
  `integrate_sim_bodies` (beside `steer_mount_from_rider`); limb spawn recipe
  off `boss_actor_cluster` (`spawn_actors.rs:542`) — gravity-free
  direct-velocity movers, no Brain/BossConfig/BodyHealth; determinism by
  stable limb order. Headless test: a scripted pilot writes two diverging
  limb intents; both limbs integrate + strike via FollowOwner hitboxes.
  (Specced enough that opus MAY attempt it; first sign of schedule-order or
  intent-shape drift → stop and queue for fable.)
- **R10.2 [opus]** Sprite split in the Python generator: `giant_gnu`
  (body+head) sheets, `gnu_hands` sheets, `gnu_ton` scholar-rider sheet (the
  existing `scholar` per-frame anchor IS the rider figure and becomes
  `rider_offset`); actor RONs; regen_sprites.sh path; parity baselines
  re-pinned. Delete the per-frame hand hit-geometry from the sheet RON.
- **R10.3 [opus]** Archetype split + encounter re-key: `giant_gnu` mount row
  (`mount_class: "gnu"`, big HP, real mover — retire `StationaryGiant`,
  `body_damage: 0` dies) + `gnu_ton` rider row (`pilotable_mount_classes:
  ["gnu"]`, boss identity, the scripted `BossPattern`, encounter HP =
  rider HP). Mount-death → dismount phase via an `External` gate trigger
  ("mount_died") into the existing phase vocabulary; `MountDeathImpact::
  Dismount` (no splash — gnuton survives to rage on foot).
- **R10.4 [opus]** Choreography port: scripted pattern steps → `LimbIntents`
  + limb `MoveSpec`s (`hand_slam`/`hand_sweep` as FollowOwner moveset moves
  on the hand bodies; `head_descent` = a gnu body move; `converging_shockwave`
  = both hands, one intent each; `apple_rain` stays a rider Special). Delete
  the `HAND_SLAM`/`HAND_SWEEP` StrikeRect tables + `sync_boss_split_overlay`
  + `BossOverlayLayer` + split z-consts. Boss suites retargeted (assert limb
  strikes land, two HP pools, dismount phase); expression arcs ship BLIND.
- **R10.5 [opus]** Authoring: `BossSpawn` gains the same `mounted_on`
  EntityRef the EnemySpawn path has (converter + `ambition_ldtk_tools`
  `mount split` extension); `gnu_ton_arena` reauthored as the linked pair
  (mount entity + rider BossSpawn); roundtrip + validate gate.
- **R10.6 [★fable, after M5]** The payoff: player boards/possesses into the
  giant — player-piloting (M5) + a controller→limb verb map (limb strikes on
  attack verbs through the SAME directional-verb resolution the moveset
  already has). Reserved until M5 lands; design note only.

## 5. THE FABLE QUEUE (Jon: schedule these with fable, soonest first)

1. **R8.1–R8.3** — the contact vocabulary, the chain primitive, the follower
   solver. The kernel of everything; wrong here is expensive. One focused
   fable session gets through R8.1+R8.2 and most of R8.3's suite.
2. **R9.1** — the `MotionModel` seam (half a session; rides R8.3 directly).
3. **R10.1** — the limb fan-out (opus MAY attempt from the spec; fable on
   first drift).
4. **Later, same family:** angled portals / portal-on-moving-platform (the
   `PortalFrame` FIXME arc) and axis-swept-bodies-on-chains (knight ramps) —
   both deliberately parked behind R8's proof.

Everything else in R7/R9/R10 is opus-executable from this doc.

## 6. Interaction with the R4–R6 decomposition (the "don't interfere, help" check)

- **R8 lives entirely in Tier 1** (`engine_core` + a `platformer_primitives`
  touch) — BELOW the gameplay_core carve; zero file overlap with R4 slices.
  It helps R7/R4: `ambition_world` is born speaking `Contact`/`SurfaceChain`
  instead of being carved and then re-plumbed.
- **R7 IS R4b done right** — same dep-inversion prep, same 139-inbound
  repoint, better target shape. The 07-04 doc's R4b section now carries a
  pointer here; do not execute R4b in the one-crate shape.
- **R9 adds** a policy + content; it deletes nothing and touches the shared
  integrate path in one dispatch point (the Perception-pattern precedent).
- **R10 subtracts** boss special-cases (split overlay, per-frame geometry,
  stationary movement) and pushes the last boss onto the moveset runtime —
  it ACCELERATES the 07-04 A1 tail and the R4e sprite-metadata carve
  (gnuton's sheets go through the ONE pipeline like everyone else's).
- **Compile discipline:** R8/R9 kernel iteration stays in-crate
  (`cargo test -p ambition_engine_core`); R7's carve is measured
  before/after like every R4 slice.

## 7. JON'S OPEN DECISIONS (defaults chosen; nothing blocks)

- **Q13 — how far does Sanic go after R9?** Default: the sandbox proof only;
  a momentum DEMO GAME (zones, enemies, rings-analog) becomes a demo-matrix
  candidate beside SMB1/MoneySeize (Q12) rather than jumping the queue.
- **Q14 — mount class name for the gnu** (`"gnu"` chosen; `"giant"` if you
  want mechs/colossi to share the class) and whether the dismounted-gnuton
  on-foot phase gets its own mini phase-pattern (default: yes, one short
  scripted phase — it's one RON block).
- **Q15 — knight-likes on chains (Celeste ramps)**: in 1.0 scope or not?
  Default: decide AFTER R8/R9 prove the contact model (the honest answer to
  the old Q6 depends on evidence we're about to buy).

---

# EXECUTION LOG (live — newest last)

*Executor entries append here, 07-04 conventions (R-numbered, gates named,
BLIND marked, wall-clock per phase for multi-phase runs).*

## THE FABLE QUEUE RUN (executor: fable, 2026-07-05, one session)

Jon: "execute the hardest unblocked items." All five queue slices + R8.4
landed, each committed + gated. Every slice is HEADLESS-verified; nothing here
touches feel (no BLIND bits — no production body carries the new policies yet).

### R8.1 — the contact vocabulary ✅ (`9f13a7b8`, byte-identical)
`Contact { point, normal, toi, surface_velocity, source }` + `ContactSource
{ Block, Chain }` in `collision_semantics.rs`, one winding rule everywhere
(`normal` = surface-outward, `tangent = (-n.y, n.x)`; parry's `normal1` is the
moving shape's outward normal — negated at the boundary). BOTH sweeps
populate; resolution untouched. The elegant discovery: the player path needed
ZERO public signature changes — contacts ride `FrameEvents` (already a
Vec-carrying struct); the kinematic path gained `step_kinematic_observed(...,
Option<&mut Vec<Contact>>)` with the plain fn delegating None. Landing = feet
contact, wall = side contact, grounded frame = a REST contact carrying the
support's `surface_velocity` (moving-platform carry made visible). Tests: C4
landing normals under all four cardinal gravities, platform-velocity rest
contact, player feet/wall contacts via the scratch API. Gate: engine_core 216,
primitives 48, gameplay_core --lib 1129.

### R8.2 — `SurfaceChain` ✅ (`8aff61a4`)
The first richer-than-AABB primitive, in engine_core beside `Block`:
polyline + `closed` + `kind` + per-frame `velocity`; normals DERIVED by the
shared winding rule, never authored. `World.chains` (+ `with_chains`); every
AABB-only room authors zero chains. Geometry kit: arc-length `frame_at`
(wraps/clamps), `project` (arc + signed rideable side), shoelace
`signed_area` (negative = interior-rideable loop), and the pragmatic
`validate()` (min points, degenerate joins, duplicated closing vertex, O(n²)
self-intersection) — bad authored geometry can't masquerade as physics bugs.
20 `World` literal sites gained the field (tests only). **Deviation from the
plan doc:** the debug OVERLAY (gizmos) moved out of this slice into the R9.3
sandbox slice — one gameplay-side touch instead of two; the validator landed
here as planned. Gate: engine_core 220, gameplay_core --lib 1129.

### R8.3 — the surface-follower solver ✅ (`d825f647`) — the heart
`engine_core::surface`: circle-proxy body, `Airborne` (ballistic + swept
circle vs chains AND solid blocks, parry casts, TOI, M10 no-pushout) or
`Riding { chain, s, v_t }` — 1-D arc-length integration while attached.
Rules: gravity projects onto the tangent (MIDPOINT-of-step force evaluation —
found and killed a first-order energy pump at joint crossings); input
accelerates to `top_speed`, slopes may exceed it; straight-run stick rule
(low-press surfaces shed a slow body); convex joints launch when centripetal
demand `v_t²θ/r` beats `stick_factor × press` — convexity computed in
AUTHORED order (found and fixed: traversal-order cross flips sign moving
backward; a crest must be a crest both ways); concave (loop-interior) joins
always follow; open ends launch along the end tangent; chains one-sided;
jump = +normal, tangent momentum kept. All feel in RON-authorable
`MomentumParams`. 16 tests incl. loop-completes-above-threshold /
halfpipe-oscillates-below (initially asserted "sheds" — the solver taught the
test the right physics), 500px/frame no-tunnel, and **the C4 rig: the whole
scenario rotated 90° (points + gravity) matches to <0.5px over 600 frames**
of riding, joints, launch, jump, and ballistic fall. Gate: engine_core 231.

### R8.4 — AABB protection net ✅ (`30010fcf`)
The reciprocal pin: an axis-swept kinematic body falls straight through a
chain-only world (zero chain code on the AABB path). Chain-side coexistence
already pinned by R8.3's suite. Full-workspace behavioral gate recorded below.

### R9.1 — `MotionModel` ✅ (`7041d1d0`)
The AJ11 policy, exactly the `Perception` shape: absent/`AxisSwept` = today's
path; `SurfaceMomentum(MomentumMotion { params, state })` dispatches to the
follower INSIDE the one `integrate_actor_body` (policy branch on body data;
boss call site passes None). Reads the same brain-produced
`ActorControlFrame` every controller writes — possession-invariance by
construction (brain transfer moves the brain; the body's motion identity
stays). Sets `on_ground` = riding; `surface_normal` follows the ridden chain
(§B2 — footprint/sprite tilt with the slope); universal CenteredAabb tail.
Tests: fall→land→run-the-flat→climb-the-ramp with the frame tilting; jump
launches along +normal with facing write-back. Gate: gameplay_core --lib 1131.
The archetype-RON row (`MomentumParams` → catalog) + the possessed-Sanic
end-to-end ride with R9.2/R9.3.

### R10.1 — the limb rig fan-out ✅ (`c9b9dd02`)
`LimbRig` (spawn-order limbs — stable-id determinism) / `Limb { of, slot }` /
`LimbIntents` (BTreeMap<LimbSlot, ActorControlFrame>) +
`fan_out_limb_intents` — `steer_mount_from_rider` generalized 1→N. Limbs =
ordinary actor bodies (no Brain/BossConfig/BodyHealth); absent slots
explicitly neutralized (no stale-intent drift). Placed in
`features/ecs/actors/limbs.rs` (a mount-level capability, but the mount
module has an in-flight concurrent slice — no shared files touched). Schedule
REGISTRATION deliberately deferred to the first production rig
(R10.3/R10.4): contract documented in the module head (after host brain tick
+ mount steer, before `integrate_sim_bodies`). Test: diverging hand intents
land on the right limbs, strike edges don't bleed, dropped slots neutralize.
Gate: gameplay_core --lib 1132.

### FULL GATE — `cargo test --workspace --all-targets --features rl_sim --no-fail-fast` ✅
73 test binaries green (engine_core 231, primitives 49, gameplay_core --lib
1132, characters 253, app --lib 140, all boss/duel/possession suites). The
ONLY failure in the workspace is the DOCUMENTED pre-existing RED
`unified_melee::a_hostile_actor_enters_the_same_body_melee_lifecycle`
(feel-reserved moveset-cadence gap — confirmed identical before this run;
untouched). Note: the working tree also carries a concurrent agent's
uncommitted mount-cutover cleanup; this run shares no files with it.

### Session table (wall-clock, single fable session 2026-07-05)

| Slice | Commit | Est (doc) | Actual |
|---|---|---|---|
| kernel read-in | — | — | ~14 min |
| R8.1 contacts | `9f13a7b8` | (R8.1–8.3 ≈ 1 session) | ~15 min |
| R8.2 SurfaceChain | `8aff61a4` | " | ~5 min |
| R8.3 follower solver | `d825f647` | " | ~10 min |
| R8.4 protection net | `30010fcf` | — | ~1 min |
| R9.1 MotionModel | `7041d1d0` | ~½ session | ~7 min |
| R10.1 limb fan-out | `c9b9dd02` | opus-attemptable | ~3 min |
| **Total** | 6 commits | ~2 sessions | **~55 min** |

**REMAINING (all [opus]-executable from this doc):** R9.2 Sanic character
(catalog row + `momentum` archetype field → `MotionModel` insert at spawn +
sprite), R9.3 sandbox room (LDtk chain entity + `ron-room` loop + debug
overlay gizmos — the overlay deferred from R8.2 lands here), R9.4 proof
tests, R10.2–R10.5 (sprite split / archetype split / choreography port /
authoring — R10.1's fan-out registration rides R10.3), R7 (the world carve),
and the R10.6/M5 player-pilots-the-giant payoff after M5.
