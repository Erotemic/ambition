# Restructuring blueprint — actionable distillation

*Author: Claude Opus 4.8 (1M) · 2026-06-25 · status: PROPOSAL (one decision resolved; see RoomGeometry)*

This distils an externally-generated "restructuring blueprint v5" (static
inspection only, no `cargo`) down to the parts worth doing, **reordered to put
concrete wins first**, and filtered through standing constraints:

- **No backwards-compat tax.** Nothing depends on this repo, there is no release.
  Prefer single-commit replacement over bridge/alias/compat ceremony.
- **Narrow types over wide generic surfaces.** Add a seam (message, trait, knob)
  when a second use case lands — not preemptively.
- **Relativity principle.** No player-centrism; mechanics frame-agnostic and
  shared by every actor.
- **Elegance over hacks.** Generalise the elegant pattern already in the code;
  delete the leak. Correctness is emergent from the right shape.

Counts below were re-verified against live `main` on 2026-06-25, so this is a
description of the repo as it stands, not a stale snapshot.

---

## Resolved decision: `GameWorld` → `RoomGeometry`, read through a collision view

The blueprint posed an open fork: is `GameWorld` an *authoritative mutable world*
or a *derived cache*? That fork was a false dichotomy built on a bad name. There
is no cache anywhere.

**What the type actually is.** `ae::World` is
`{ name, size, spawn, blocks, water_regions, climbable_regions }` — purely the
**static spatial geometry of one room**: bounds, spawn, collision blocks, water,
ladders. No entities, no actors, no items, no dynamic state. `GameWorld` is just
the Bevy-resource wrapper around it. The `Game` prefix carries no meaning; it
exists only to avoid clashing with `bevy::ecs::World`. The name is named for what
it *isn't*.

**How it behaves today (already a clean split, just unnamed):**

- `GameWorld` is **authored, write-once-per-room.** Every production write is
  wholesale replacement at a room boundary — `world.0 = spec.world.clone()` in
  `room_flow.rs`, `session/reset/mod.rs`, `dev_runtime.rs`, plus the initial
  insert. Nothing in production mutates it incrementally mid-room. (The
  `gnu_ton.rs` `.0 = World::new(...)` writes are test scaffolding simulating room
  changes, not a content hack.)
- The **mid-room dynamics are a derived view, not mutation.** Moving platforms,
  ECS solids, and portal carves fold into a *fresh* `ae::World` each frame via
  `combat::world_overlay::world_with_sandbox_solids`, with a `Cow::Borrowed` fast
  path so the no-dynamics case never clones. Portal core owns carve geometry and
  is forbidden from naming the host overlay (`FeatureEcsWorldOverlay`); Ambition
  owns how a carve alters collision.

**Decision:**

1. Rename the resource `GameWorld` → **`RoomGeometry`** (authored, swapped at
   room/reset/hot-reload boundaries). The engine `ae::World` may keep `World`
   (physics-engine idiomatic; `ae::` disambiguates) or later become `Terrain` —
   lower priority than the resource wrapper.
2. The per-frame composite is a **collision view**, not a cache: a computed
   `RoomGeometry + overlay` value, transient (the `Cow` path may not even
   materialise it). `FeatureEcsWorldOverlay` is the retained per-frame *gather*
   of dynamic contributions (platforms, ECS solids, carves) — it's the overlay
   layer the view composites over.
3. **The 25 raw `Res<GameWorld>` readers are the bug.** They read bare geometry
   when they should read the collision view. Promote the composite to the single
   collision read-API and route readers through it. This is the *same seam* the
   collision-semantics dedup needs (item 2 below) — treat them as one frontier.

Why this is the elegant answer and not the mutable pole: an authored
`RoomGeometry` + derived collision view is replay/RL-friendly (a frame's
collision truth is a pure function of room id + overlay state — snapshot/rewind is
free) and naturally supports per-player world variants later, without a mutable
monolith that tempts content to reach in and mutate the base.

---

## The plan, ordered by value

### 1. Delete the compatibility shims (one canonical import per concept)

`ambition_gameplay_core/src/lib.rs` re-exports already-extracted crates under
historical paths, creating multiple valid import paths for one concept — directly
against the agent-navigability goal. Live call-site pressure (excluding
gameplay_core itself):

| shim | canonical | live hits |
| --- | --- | --- |
| `::kinematic` | `ambition_platformer_primitives::kinematic` | **0** |
| `::ui_nav` | `ambition_ui_nav` | 3 |
| `::interaction` | `ambition_interaction` | 6 |
| `::actor` | `ambition_characters::actor` | 16 |
| `::brain` | `ambition_characters::brain` | 37 |
| `::engine_core` | `ambition_engine_core` | 68 |
| `::input` | `ambition_input` | 70 |

**Do:** delete each shim and fix imports in one commit per shim — **no facade,
no allowlist, no deprecation window** (there are no external consumers). Start
with `kinematic` (free — already 0) and `ui_nav`/`interaction`. Add an
architecture-boundary test that fails on new internal use of these paths — keep
the *test* as a guardrail, not an alias.

**Validation:** `rg "ambition_gameplay_core::(input|engine_core|brain|actor|interaction|ui_nav|kinematic)" crates`
should drop to zero internal hits.

### 2. Collision/support-semantics dedup (+ RoomGeometry collision view)

The highest-value correctness work. Two implementations carry overlapping
gravity-relative support semantics that can agree at the design level while
drifting at the implementation level:

- `ambition_engine_core/src/movement/collision.rs` (707 lines) — controlled-body
  movement collision.
- `ambition_platformer_primitives/src/kinematic.rs` (1226 lines) — generic
  actor/NPC/enemy sweep.

This is the relativity principle as a correctness property: every actor —
player, NPC, enemy, projectile, remote/AI — should collide against one
composited truth. It's also the engine-for-other-games keystone.

**Do (parity-first, the proven-safe order for big mechanical ports):**

1. Build a shared fixture table: `BlockKind` × cardinal `gravity_dir` ×
   previous-feet coord × delta × drop-through → expected support/block/pass.
2. Run identical expectations against *both* current paths before changing
   anything.
3. Extract pure helpers (support-surface classification, gravity-axis role,
   support-face separation, one-way landing eligibility) into a shared semantics
   module; keep both sweeps but make them call it.
4. Land the `RoomGeometry` collision-view API here — both sweeps query the
   composited view, not bare geometry. This unifies item 1's decision with the
   dedup.

### 3. Drain simulation out of `ambition_app` into domain plugins

`ambition_app` should compose plugins and host platform/device/window concerns —
it should not *define what a domain transition means*. Today it still owns real
simulation in `app/sim_systems.rs`, `app/combat_schedule.rs`,
`app/progression_schedule.rs`. Clearest first movers (low coupling):

- `attack_advance_system` → combat runtime.
- `detect_room_transition_system` → world runtime (after the RoomGeometry
  write-map exists).
- `apply_player_hit_events` → combat/actor-health runtime (with source/cause
  attribution).

Keep platform/device/Android/mobile/window systems in the app. The blueprint's
`ambition_game` composition-root crate is a *direction*, not a prerequisite —
introduce it only when the app file genuinely reads as two jobs (host vs.
compose). Preserve ordering-sensitive comments **as tests** when moving systems
(projectile-spawn timing especially).

### 4. `ControlFrame` → actor-local intent

`ControlFrame` is a fine input-source snapshot; the problem is ~46 simulation and
presentation systems read the global `Res<ControlFrame>` directly, which hardcodes
one local input source and one primary controlled actor — the player-centrism the
relativity principle rejects. Keep `ControlFrame` as input-source data; move
*simulation* onto entity-local `ActorIntent`/`ActorInputFrame`.

**First converts (one at a time, behaviour-preserving):**

1. `heal_save_shrine_system` → actor-local interact/use intent (smallest).
2. `compute_player_intent` → `compute_controlled_actor_intent`; centralise ability
   use decisions there instead of each ability re-reading global input.
3. One ranged ability (`fire_shockwave_system`) as the pattern.
4. Carryable-item use/throw/fire (`throw_held_item_system`,
   `fire_held_ranged_system`) onto actor/item intent + holder relationship.
5. Portal input adapter last (after the core consumers move).

**Validation:** remaining direct `Res<ControlFrame>` uses should cluster in
input-source *writers*, tests, and presentation — not ability/item/combat sim.

### 5. Classify the `OnceLock` global registries

Eight `OnceLock`s (boss profiles/specs, enemy roster, encounter waves, sheet
indices). They are not automatically wrong. **Classify each** as: content
registry, immutable asset-metadata cache, or test-override seam. Promote content
registries (`ENEMY_ROSTER_OVERRIDE`, `BOSS_PROFILE_OVERRIDE`,
`BOSS_ENCOUNTER_SPEC_OVERRIDE`, `ENCOUNTER_WAVE_BOOK`) toward resources/contexts;
keep pure immutable sheet/index caches but *name and document them as asset
caches*. Low urgency relative to 1–4.

---

## Do opportunistically, NOT as a scheduled wave

These are the right *direction* but wrong as big up-front pushes — they'd be the
wide tech-debt surface that's explicitly not the goal.

- **`Player*` → actor/participant/viewpoint rename.** The conceptual split is
  correct (actor body vs. control authority vs. participant vs. presentation
  focus), and it *is* the relativity principle. But there are hundreds of sites;
  renaming all of them now — justified largely by multiplayer that isn't designed
  — is speculative churn. **Rename role-by-role, in files you're already editing
  for items 1–4. No `legacy`/alias module that doubles every name.** The renames
  that buy clarity *today* land for free as a byproduct; the ones justified only
  by "future replication IDs" wait.
- **fact/request/event message vocabulary.** Defining
  `StartCutsceneRequest`/`CutsceneStarted`/etc. for domains with one producer and
  one consumer is premature indirection (and we've been bitten by query-order
  determinism). Add the message seam when the *second* consumer appears.
- **Doc-consistency annotations.** Nine docs under `docs/planning`/`docs/systems`
  still say "COMPLETE" while bridge vocabulary survives (verified). Worth fixing,
  but it's annotation work that should not *gate* the engineering — do it
  alongside the code it describes.

---

## Deliberately deferred / avoid

- **Bridge/alias/compat scaffolding.** No external consumers, no release → no
  compat tax. Delete-and-fix beats two-step migration here every time.
- **The mutable-authority world pole.** Avoid. The authored-`RoomGeometry` +
  derived-collision-view model is the one to formalise. The only thing that could
  pull toward mutability is *persistent* mid-room geometry change — see open
  questions.
- **Crate-splitting `ambition_content`.** Do module families
  (`authored`/`install`/`adapters`/`presentation_bindings`) first; split into
  crates only when a boundary proves itself.
- **Choosing netcode.** Prepare seams (actor/source/cause attribution on
  projectiles/damage/SFX/effects; item instance identity + holder) compactly so
  causality exists for future replay/multiplayer — but do not pick or build a
  netcode implementation.

---

## Open questions

1. **Falling sand is the forcing function for the world model.** If settled sand
   becomes *durable* collision (`falling_sand.rs`), the pure derived-view model
   needs a **durable overlay tier** (a persistent block list that survives frames
   but still isn't the authored base) rather than a mutable authoritative world.
   Verify how settled sand reaches collision before committing the "no persistent
   mutation" stance. This is the one fact that confirms or complicates the
   RoomGeometry decision.
2. **`ae::World` → `Terrain`?** Optional, lower priority than the resource rename.
3. **Future quest/progression model.** Current quest runtime is replaceable
   scaffolding; preserve facts/save boundaries, don't design around today's quest
   code.

---

## Guiding contracts for patches

```text
RoomGeometry is authored, swapped at room boundaries; collision is read through
  the composited view, never the bare geometry.
Simulation is modelled around actors and actor-local intent, not a global input
  frame or a primary player.
Carryable items stay one lifecycle across held/world/thrown/recovered.
Attach source/cause attribution to projectiles/damage/effects/SFX/facts compactly.
Reusable mechanics are Bevy plugins with owned resources/messages/local sets;
  the app composes and hosts, it does not define domain meaning.
Canonical import path per concept; bridge vocabulary, if unavoidable, is named
  legacy/adapter/compat and is temporary.
Delete, don't bridge. Rename in place, don't alias. Add seams when the second
  use case lands.
```
