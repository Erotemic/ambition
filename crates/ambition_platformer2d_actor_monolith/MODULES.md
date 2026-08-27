# `ambition_platformer2d_actor_monolith` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_actor_monolith** — Gameplay-system assembly layer for platformer actors, world interaction, abilities, items, sessions, and related Bevy plugins.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`abilities`](src/abilities/mod.rs) | Ambition's player ability / weapon kit. |
| [`ability_cooldown`](src/ability_cooldown.rs) | Shared per-body cooldown for movement abilities such as Blink and Grapple. |
| [`action_scheme`](src/action_scheme.rs) | Materializing each body's [`ActorActionScheme`] — the OBSERVATION CACHE of its derived slot→action scheme. |
| [`actor`](src/actor.rs) | THE ONE THING THIS MODULE ACTUALLY OWNS. |
| [`assets`](src/assets/mod.rs) | Asset registries and load-time wiring. |
| [`audio`](src/audio/mod.rs) | Audio runtime for the Ambition game. |
| [`avatar`](src/avatar/mod.rs) | Home-avatar policy and integration that has not yet moved to its final owner. |
| [`body_custody`](src/body_custody.rs) | Projects body custody from authoritative roots and attachment relations. |
| [`body_mode`](src/body_mode/mod.rs) | Body-mode driver: facade re-exporting [`update_body_mode`]. |
| [`brain_tick`](src/brain_tick.rs) | THE BRAIN DISPATCH, and it lives here because this is the only crate that can see every destination. |
| [`causal`](src/causal.rs) | This crate's causal facts. |
| [`character_runtime`](src/character_runtime/mod.rs) | Engine-owned character loading and materialization. |
| [`character_sprites`](src/character_sprites/mod.rs) | Character sprite asset loading and actor/content joins. |
| [`checkpoint_horizon`](src/checkpoint_horizon.rs) | Actor-side contribution to the reset/checkpoint horizon. |
| [`config`](src/config.rs) | The render-only `rgba` color helper. |
| [`construction`](src/construction/mod.rs) | Actor construction planner for authored, provider-staged, and runtime-dynamic origins. |
| [`control`](src/control/mod.rs) | Local control seam from device input to the body driven by a participant. |
| [`cutscene`](src/cutscene.rs) | Cutscene playback runtime (the systems that drive the scripts). |
| [`dev`](src/dev.rs) | Sim-side developer tooling that still samples actor-domain state. |
| [`encounter`](src/encounter/mod.rs) | Generic, reusable enemy-WAVE / arena-lockdown system (data-driven, not scripted) — distinct from `ambition_boss_encounter`, which is one specific scripted boss fight with hand-authored phases. |
| [`features`](src/features/mod.rs) | The enemy / NPC / boss ECS ACTOR SIMULATION — NOT a feature-toggle layer. |
| [`gravity`](src/gravity/mod.rs) | Gravity-zone mechanic. |
| [`host`](src/host/mod.rs) | Host vocabulary that machinery reads: windowing/display-mode types consumed by the settings model and menu IR. |
| [`items`](src/items/mod.rs) | Actor-sim item adapters. |
| [`music`](src/music/mod.rs) | Ambition-game music adapters over the `ambition_audio` music core. |
| [`participant_seat`](src/participant_seat.rs) | Central conversion between [`ParticipantId`] and [`PlayerSlot`]. |
| [`platformer_runtime`](src/platformer_runtime/mod.rs) | Compatibility facade over extracted platformer-runtime surfaces plus monolith-owned orientation. |
| [`projectile`](src/projectile/mod.rs) | Controlled-body projectile integration around the reusable projectile model. |
| [`quest`](src/quest/mod.rs) | Gameplay-core adapter for the generic quest runtime. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by the actor runtime. |
| [`schedule`](src/schedule/mod.rs) | Schedule + input-frame vocabulary shared by the machinery lib, the content crate, and the app crate. |
| [`session`](src/session/mod.rs) | Ambition-game session lifecycle: startup setup ([`setup`]), full reset/respawn ([`reset`]), RON data manifests ([`data`]), and setup glue. |
| [`shrine`](src/shrine.rs) | Healing / save-point shrine. |
| [`snapshot_impls`](src/snapshot_impls.rs) | `SnapshotState` for this crate's own types — the rollback wire format. |
| [`time`](src/time/mod.rs) | Time domain plumbing: clocks (ADR 0010/0011), time-control authority, per-entity proper-time scale, and game-feel tuning. |
| [`world`](src/world/mod.rs) | World / level authoring runtime: room graph + spawning, the code-first room builder, the Avian2D physics adapter, and LDtk-authored moving platforms. |
| [`world_facts`](src/world_facts.rs) | Authored-logic domain for durable world flags. |

_37 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->


## Notes

**Read this first if you are about to change something here.** This crate is an
active decomposition target. At the 2026-08-07 planning baseline it contains
110,911 Rust lines under `src/`; with Cargo incremental compilation disabled,
that size is also an edit-loop cost. The current strategy is to subtract coherent
domains little by little, guided by dependency/authority evidence, until the
residue is honestly the actor/body/control simulation domain. See
[`docs/planning/engine/actor-monolith-decomposition.md`](../../docs/planning/engine/actor-monolith-decomposition.md).

The generated module map and `.agent` inventories are navigation evidence for
choosing slices; they are not a reason to preserve today's directory boundaries
as crates.

### Historical names to interpret during the carve

| Name | What it currently means / where it is headed |
|---|---|
| `features` | **the enemy / NPC / boss ECS ACTOR SIMULATION**, not a feature-toggle layer. Peel optional domains first; rename the residue only when its ownership is honest. |
| `actor` (singular) | neutral body vocabulary, not the whole actor runtime. Shared body state belongs here or in a lower canonical owner rather than on participant-specific types. |
| `avatar` | historical slot-0/home-body and protagonist integration. Do not extract an `avatar` crate; dissolve its mechanics, lifecycle policy, preparation, and presentation responsibilities into their actual owners. |
| `control` | local participant/input-source → slot/channel → controlled-body seam. Downstream body simulation should not depend on a privileged participant identity. |

### Authoritative state — who mutates what

- **A body's motion** is `BodyKinematics` + the 18 movement clusters, and exactly
  one kernel entry writes them: `ae::step_motion`. The player
  tick and `update_ecs_actors` are two Bevy systems calling the SAME body tick.
  Do not add a third.
- **A body's melee** is a `"attack"`-verb moveset move (`combat::moveset`) for
  every body — triggered by `trigger_moveset_moves`, struck by
  `advance_move_playback`, and projected back into `BodyMelee` / `MeleeSwing` as
  the anim/HUD read-model. ONE melee path; no player/actor driver split.
- **Who the human is driving** is `ControlledSubject` (the entity carrying
  `Brain::Player(slot)`), never a possession flag and never `PrimaryPlayer`.
  `PrimaryPlayer` means *slot 0's own body*, which is a different question; every
  surviving `PrimaryPlayerOnly` filter in this crate carries a comment saying why
  it is asking that one.
- **The collision world** is `ambition_platformer2d_world::collision::CollisionWorld`, not
  a bare `SessionWorldRef<RoomGeometry>`. A sweep or raycast that reads only
  authored geometry misses
  moving platforms, ECS solids, and portal carves.

### The two lints that will fail you

- `ambition_platformer2d_runtime/tests/determinism_lints.rs` (ADR 0023) — no ambient RNG, no
  wall clock, no `std` hash-container iteration, no `Entity` as an ordering key.
- `ambition_platformer2d_runtime/tests/control_frame_lint.rs` — only the input layer may hold
  the global `ControlFrame`. A body system that reads it is silently slot-0-only.
  Its allowlist doubles as the netcode N1 checklist.

### Maintaining this file

The table above is generated from each module's own `//!` header:
`python scripts/modules_md.py --write`. `python scripts/modules_md.py` checks for
drift and exits non-zero. Everything under `## Notes` is hand-written and survives
regeneration.
