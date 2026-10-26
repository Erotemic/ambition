# `ambition_platformer2d_runtime` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_runtime** — Content-free platformer simulation assembly.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`authored_verdict_timeline`](src/authored_verdict_timeline.rs) | Reconcile the authored-verdict ring with the host's rollback timeline. |
| [`causal`](src/causal.rs) | Bevy host adapter for `ambition_causal` recording and frame stamps. |
| [`checkpoint_horizon`](src/checkpoint_horizon.rs) | Host wiring for the reset/checkpoint horizon. |
| [`combat_rules`](src/combat_rules.rs) | Project match rules into the live combat tuning. |
| [`combat_schedule`](src/combat_schedule.rs) | Combat-phase schedule plugin. |
| [`content_identity`](src/content_identity.rs) | Immutable prepared-content identity shared by preparation, activation, snapshots, and transactional hot reload. |
| [`cutscene`](src/cutscene.rs) | Cutscene playback runtime (the systems that drive the scripts). |
| [`durable_save_horizon`](src/durable_save_horizon.rs) | Host installation of the durable save horizon. |
| [`effect_bus`](src/effect_bus.rs) | Apply typed flag, quest, switch, and audio effects. |
| [`extension_composition`](src/extension_composition.rs) | The extension host in the platformer composition (fast-iteration I4). |
| [`external_effects`](src/external_effects.rs) | Defers presentation-only simulation effects until their producing frame is confirmed. |
| [`gravity`](src/gravity/mod.rs) | Gravity-zone mechanic. |
| [`input`](src/input.rs) | Device input, participant seating, context claims, and frame publication adapters. |
| [`input_drive`](src/input_drive.rs) | Backend-neutral authored input delivery for simulation drivers. |
| [`input_stream`](src/input_stream.rs) | Input-stream capture (netcode N0.2) — the one place a session's input is recorded. |
| [`ldtk_world`](src/ldtk_world.rs) | Opt-in host composition for games that use an LDtk world. |
| [`mode_scope`](src/mode_scope.rs) | Scoped game-mode runtime for hosted demos/rulesets. |
| [`module_save_road`](src/module_save_road.rs) | The session durable save road of the extension host ([`SaveRoad::SessionDurable`]): a module's session record whose schema says `SaveEligibility::Durable` is kept in the durable world save, and comes back when that save loads. |
| [`player_schedule`](src/player_schedule.rs) | Engine-generic per-frame player lifecycle and its host extension slots. |
| [`portal_body`](src/portal_body.rs) | What a portal transit means for the body it moved. |
| [`portal_schedule`](src/portal_schedule.rs) | Portal simulation assembly and schedule placement. |
| [`portal_seat`](src/portal_seat.rs) | What a portal transit means for the body a seat drives. |
| [`posed_limbs`](src/posed_limbs.rs) | A limb its owner poses meets what is there (`docs/planning/engine/portals-a-body-in-two-places.md`, G6). |
| [`profile`](src/profile.rs) | Named supported engine profiles: capability contracts, not plugin lists. |
| [`progression_schedule`](src/progression_schedule.rs) | Progression-phase schedule plugin. |
| [`projectile_schedule`](src/projectile_schedule.rs) | Projectile schedule seams owned by the runtime composition tier. |
| [`quest`](src/quest.rs) | Gameplay-core adapter for the generic quest runtime. |
| [`rollback`](src/rollback/mod.rs) | Backend-neutral rollback schema composition. |
| [`room_departure`](src/room_departure.rs) | Leaving a room because the level in it is done: the one road for "go on". |
| [`room_schedule`](src/room_schedule.rs) | Room-transition schedule anchors. |
| [`room_transition`](src/room_transition/mod.rs) | Engine-owned room-transition orchestration. |
| [`runtime_census`](src/runtime_census.rs) | `[census] rooms` — WHICH ROOM IS LIVE, AND WHAT THE CROSSING IS DOING. |
| [`sandbox_reset`](src/sandbox_reset.rs) | The sandbox reset authority and its room-replay consumer. |
| [`schedule`](src/schedule.rs) | Runtime phase order and shared simulation gates. |
| [`session_world`](src/session_world.rs) | Prepared platformer definitions and canonical live session components. |
| [`sim_clock`](src/sim_clock/mod.rs) | Time domain plumbing: clocks (ADR 0010/0011), time-control authority, per-entity proper-time scale, and game-feel tuning. |
| [`sim_core_resources`](src/sim_core_resources.rs) | The engine-generic simulation messages + resource defaults (E5 step 6). |
| [`sim_identity`](src/sim_identity.rs) | Backend-neutral stable simulation identity maintenance. |
| [`stocks_match`](src/stocks_match.rs) | Resolve match outcomes and sudden-death entry. |
| [`verdict_census`](src/verdict_census.rs) | Diagnostic summary of the existing authored-verdict ring. |
| [`world_facts`](src/world_facts.rs) | Authored-logic domain for durable world flags. |
| [`world_gating`](src/world_gating.rs) | The two roads into `gate_solids`, registered in one place. |
| [`world_overlay`](src/world_overlay.rs) | Feature collision-world overlay rebuilt from ECS-owned world geometry. |
| [`world_schedule`](src/world_schedule.rs) | Cross-domain simulation assembly for world objects, matches, and gameplay effects. |

_44 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
