# ambition_engine crate collapse (2026-05-28)

The standalone `ambition_engine` crate was deleted. Its modules now
live under `crates/ambition_actors/src/engine_core/` as an intra-
crate module of `ambition_actors`.

## Trigger

Concrete frictions had piled up:

- `sandbox` depended on `engine`, so an engine rebuild forced a
  sandbox rebuild anyway — the boundary did not buy meaningful
  compile-time isolation.
- Every sandbox source file carried `use ambition_engine as ae;`
  plus per-call `ae::Foo` derefs; engine modules carried trampoline
  re-exports through `engine/lib.rs`.
- The "engine should be backend-neutral" rationale had already been
  retracted in ADR 0002 ("engine must be Bevy-native"). The
  `Aabb = Aabb2d` re-export and `bevy_ecs`/`bevy_math` deps the
  engine already carried said the same thing.
- The cluster-component migration ([[ECS player migration]]) had
  already moved the player's source of truth into Bevy ECS
  components. The engine crate had shrunk to ~5k LoC of mechanics
  helpers — no longer a natural "engine vs game" split.
- No external crate ever consumed `ambition_engine`.

## What landed

Two final pre-collapse moves:

1. `actor.rs` (292 lines: `Actor`, `Health`, `KinematicPath`,
   `RespawnPolicy`, …) → `crates/ambition_actors/src/actor.rs`
   (commit `d30cb3d4`).
2. `combat.rs` (499 lines: damage volumes, hit semantics) →
   `crates/ambition_actors/src/combat.rs` (earlier in branch).

Then the big bang (commit `696a4835`):

- All remaining engine modules (`abilities`, `geometry`,
  `ledge_grab`, `movement` + `movement/`, `player_clusters`,
  `player_state`, `world`) → `crates/ambition_actors/src/engine_core/`.
  `engine/lib.rs` became `engine_core/mod.rs`.
- Inside the moved files, `crate::X` references were rewritten to
  `crate::engine_core::X` (sed). Because the files now live under
  `engine_core/`, "the crate root" refers to the sandbox, not the
  engine.
- Sandbox-wide, `use ambition_engine as ae;` → `use crate::engine_core as ae;`,
  and `ambition_engine::Foo` → `crate::engine_core::Foo` (sed, ~133
  files).
- `crates/ambition_actors/Cargo.toml` inherited the engine's
  `bevy_math`, `bevy_ecs`, `parry2d` deps and `insta` + `proptest`
  dev-deps.
- `crates/ambition_engine/` deleted.

Aftermath:

- 169 files changed (1055 deletions, 434 insertions; net -621
  lines).
- Workspace from 5 crates → 4.
- rl_smoke 42/42 ok.
- A handful of test-only sed misses cleaned up in `7d27c4c5`
  (tests outside `src/` that the sed pass didn't cover, and types
  that had moved with `actor.rs`/`projectile/` and so didn't live
  in the engine anymore).
- 12 unused `use crate::engine_core as ae;` lines cleaned by
  `cargo fix` (`0196d55f`).

## Gotchas worth remembering

- **`src/bin/*.rs` are separate compilation units.** Inside a bin,
  `crate::` resolves to the bin itself, not the lib. The headless
  bin had to use `ambition_actors::engine_core::*` while every
  other file uses `crate::engine_core::*`.
- **`use crate::ae;` was never valid.** A handful of test files
  used that form; they were broken before the big bang too, because
  the `use crate::engine_core as ae;` alias in `lib.rs` is private.
  Those tests need their own follow-up.
- **`QueryData` impls keep two lifetimes**, not one. The bevy_ecs
  macro generates `Item<'w, 's>`, not `Item<'w>` — see the cluster
  migration journal for the original gotcha.
- **The sed pattern `s|crate::|crate::engine_core::|g` was greedy
  by intent**, but `engine_core/mod.rs` itself did not need
  rewriting (its `pub mod abilities; pub use abilities::AbilitySet;`
  pattern accesses siblings without `crate::`). Verified — no
  manual fixup needed there.

## What did NOT change

- The cluster components (`PlayerKinematics`, `PlayerGroundState`,
  …) and the cluster-ref entry points
  (`update_player_*_with_clusters`) still drive the runtime. The
  big bang moved them, did not refactor them.
- `ae::Player` and the legacy `update_player_*_with_tuning` entry
  points still exist inside `engine_core/movement/`. `app/phases.rs`
  still calls them via the `clusters → to_player → call → write_from_player`
  round-trip in `player_control_phase_clusters` and
  `player_simulation_phase_clusters`. Deleting the legacy entry
  points and `ae::Player` is the natural next step ([[project_ecs_migration]]).

## Forward path

If a separate crate is ever needed for genuine reuse (a sibling
game, a published library, an embedded research target), the
`engine_core/` module is now a clean unit to lift back out. Today
keeping it inline is the path of least resistance.
