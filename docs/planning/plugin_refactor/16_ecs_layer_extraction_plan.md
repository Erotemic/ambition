# Stage 16: ECS-layer extraction (the gate above engine_core)

After `ambition_engine_core` (foundation) was extracted, the next layer blocking all
further crate work is `crate::physics` + `crate::player` + `crate::features`. Analysis
(2026-06-06) shows it **trisects**, and only a small generic third should extract now.

## Map / classification
- **`crate::physics` (physics.rs, 538 LOC) — 100% GENERIC.** It's the gravity runtime,
  misnamed: `GravityField/BaseGravity/GravityZone/GravityZones/GravityCtx` + systems
  (`collect/oscillate/tick_temporary/resolve_active`) + pure fns (`gravity_dir_at`,
  `gravity_upright_angle`, `gravity_aware_flip_x`, …). Non-engine_core couplings: only
  `crate::WorldTime` (3 systems) and `crate::player::{BodyKinematics,PlayerEntity,PrimaryPlayer}`
  (1 system, `resolve_active_gravity`). Overlaps the Stage-N `crate::mechanics::gravity` seed —
  MUST be reconciled, not duplicated.
- **`crate::player` (~4k LOC) — mixed by file.** `movement_components.rs`/`components.rs` are
  generic body components (only dep: `ControlFrame`). `systems.rs`/`bundles.rs`/`affordances/*`
  are glue/content (brain/input/features). `player::BodyKinematics` == `features::BodyKinematics`
  == `engine_core::BodyKinematics` (unification done; no "3 types" problem remains).
- **`crate::features` / content/features/ecs (~21.5k LOC) — overwhelmingly CONTENT + glue.**
  Heavy deps on content/brain/presentation/combat_slots/boss_encounter/audio/persistence.
  A thin generic combat kit exists (hitbox/damage/held_items/target_volumes/overlay/mount) but
  is interwoven with named bosses/enemies → DEFER (do NOT extract this stage).

## Decision: grow `ambition_platformer_runtime` (no new crates this stage)
Fewer stable crates rebuild less; one navigable runtime crate beats 4 thin ones around a
still-shifting vocabulary (post-Task-K lesson). Keep the public surface NARROW (components +
systems + pure fns), ECS-native (no parallel-engine god object — the old `ambition_engine`
failure). Target modules added: `world_query`, `body`, `orientation`, `gravity`. New dep:
`ambition_engine_core`. `crate::physics` + sandbox `platformer_runtime::{collision,orientation}`
become facade re-exports (0-churn, the engine_core pattern).

## Decoupling needed (established inversion patterns)
1. `crate::WorldTime` → a neutral runtime `SimDt`/`RuntimeTime` resource the sandbox mirrors from
   `WorldTime.sim_dt()` each frame (must mirror sim_dt exactly — pause/bullet-time feel).
2. `resolve_active_gravity`'s player query → a neutral `PrimaryBody` marker (sandbox adds it to
   the player); runtime queries `(&BodyKinematics, With<PrimaryBody>)`.
3. `orientation` → once gravity is in-crate, `GravityCtx` is in-crate; collapse the dual
   BodyKinematics arms to one `With<BodyKinematics>` query.

## Staged steps (each: git mv + facade; gate on `--lib` + replay_fixture_regression + scripted_gameplay)
- **S0** Add `ambition_engine_core` dep to the runtime crate; add neutral `SimDt`/`RuntimeTime`
  resource + sandbox mirror system. (no moves)
- **S1** Move `SolidWorldQuery`/`raycast_solids`/`ray_aabb` → `runtime::world_query`; keep the
  `impl … for engine_core::World` adapter sandbox-side. (backlog M2)
- **S2** Move `platformer_runtime/body.rs` → `runtime::body`.
- **S3** Decouple gravity IN PLACE: `WorldTime`→`SimDt`, player query→`PrimaryBody`; reconcile
  the `crate::mechanics::gravity` seed (single home).
- **S4** `git mv` gravity → `runtime::gravity`; `crate::physics` becomes a facade.
- **S5** Move orientation → `runtime::orientation`, collapsed to one BodyKinematics query. (backlog M3)
- **S6** Ratchet `architecture_boundaries` + `runtime_extraction_backlog.md`; sandbox
  `platformer_runtime/` is then only facades/adapters.

End state: generic ECS runtime (body/world_query/gravity/orientation) is in the crate; **portal
and gravity mechanic crates are unblocked**. Combat kit + features/ecs stay (content) for a later stage.

## Risks
- S5 query-conflict if any system holds `&mut BodyKinematics` over an aliasing set — keep disjoint
  `With<PlayerEntity>`/`With<EnemyConfig>`/`With<BossConfig>` filters; never re-split the component.
- Gravity-seed (`crate::mechanics::gravity`) reconciliation — audit before S4 to avoid duplicate state.
- `SimDt` must mirror `WorldTime.sim_dt()` exactly (gravity columns / temporary zones / roll feel) —
  gated by scripted_gameplay + gravity_room_reachability.
