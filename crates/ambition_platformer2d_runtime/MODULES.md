# `ambition_platformer2d_runtime` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_runtime** — Content-free platformer simulation assembly.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`causal`](src/causal.rs) | Bevy host adapter for `ambition_causal` recording and frame stamps. |
| [`checkpoint_horizon`](src/checkpoint_horizon.rs) | Host wiring for the reset/checkpoint horizon. |
| [`combat_schedule`](src/combat_schedule.rs) | Combat-phase schedule plugin. |
| [`content_identity`](src/content_identity.rs) | Immutable prepared-content identity shared by preparation, activation, snapshots, and transactional hot reload. |
| [`durable_save_horizon`](src/durable_save_horizon.rs) | Host installation of the durable save horizon. |
| [`external_effects`](src/external_effects.rs) | Defers presentation-only simulation effects until their producing frame is confirmed. |
| [`input_drive`](src/input_drive.rs) | Backend-neutral authored input delivery for simulation drivers. |
| [`input_stream`](src/input_stream.rs) | Input-stream capture (netcode N0.2) — the one place a session's input is recorded. |
| [`ldtk_world`](src/ldtk_world.rs) | Opt-in host composition for games that use an LDtk world. |
| [`mode_scope`](src/mode_scope.rs) | Scoped game-mode runtime for hosted demos/rulesets. |
| [`player_schedule`](src/player_schedule.rs) | Engine-generic per-frame player lifecycle and its host extension slots. |
| [`portal_schedule`](src/portal_schedule.rs) | Portal simulation assembly and schedule placement. |
| [`progression_schedule`](src/progression_schedule.rs) | Progression-phase schedule plugin. |
| [`projectile_schedule`](src/projectile_schedule.rs) | Projectile schedule seams owned by the runtime composition tier. |
| [`rollback`](src/rollback/mod.rs) | Backend-neutral rollback schema composition. |
| [`room_schedule`](src/room_schedule.rs) | Room-transition schedule anchors. |
| [`room_transition`](src/room_transition/mod.rs) | Engine-owned room-transition orchestration. |
| [`sandbox_reset`](src/sandbox_reset.rs) | The sandbox reset authority and its room-replay consumer. |
| [`session_world`](src/session_world.rs) | Prepared platformer definitions and canonical live session components. |
| [`sim_core_resources`](src/sim_core_resources.rs) | The engine-generic simulation messages + resource defaults (E5 step 6). |
| [`sim_identity`](src/sim_identity.rs) | Backend-neutral stable simulation identity maintenance. |

_21 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
