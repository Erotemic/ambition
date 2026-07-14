# `ambition_runtime` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_runtime** — The platformer ENGINE face — [the sim assembly] (decomposition E5): [`PlatformerEnginePlugins`], a Bevy [`PluginGroup`] that assembles the **content-free simulation plugins** shared by every platformer built on this engine, plus the shared app-foundation helpers every entry point (visible, headless, RL, demo) composes with.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`combat_schedule`](src/combat_schedule.rs) | Combat-phase schedule plugin. |
| [`input_stream`](src/input_stream.rs) | **Input-stream capture** (netcode N0.2) — the one place a session's input is recorded. |
| [`mode_scope`](src/mode_scope.rs) | The demo-hosting seam (decomposition D-C, vision §5): **scoped game modes**. |
| [`player_schedule`](src/player_schedule.rs) | The per-frame PLAYER schedule wiring (E5 step 5) — the engine-generic player-frame lifecycle every platformer built on this engine runs, headless or windowed: time control → input → controlled-subject resolution → brains → body mode → possession → hit events → presentation write-back. |
| [`portal_schedule`](src/portal_schedule.rs) | Portal simulation assembly (E5 step 5, behind the `portal` feature): [`ambition_portal::PortalPlugin`] plus the schedule placement for portal's internal sets — each mapped to its sandbox phase, cross-set ordering edge, and gameplay run condition. |
| [`progression_schedule`](src/progression_schedule.rs) | Progression-phase schedule plugin. |
| [`projectile_schedule`](src/projectile_schedule.rs) | Projectile schedule seams owned by the runtime composition tier. |
| [`room_schedule`](src/room_schedule.rs) | The engine half of the room-transition phase (E5 step 5): detection emits `RoomTransitionRequested`; the feature-side `reset_ecs_room_features` system tears down per-room ECS state. |
| [`session_world`](src/session_world.rs) | Canonical live platformer-session world data. |
| [`sim_core_resources`](src/sim_core_resources.rs) | The engine-generic simulation messages + resource defaults (E5 step 6). |
| [`snapshot`](src/snapshot/mod.rs) | **N3.1's registration seam, and N0.4's desync canary.** |

_11 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
