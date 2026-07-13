# `ambition_actors` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_actors** — Ambition's gameplay-systems ("machinery") layer.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`abilities`](src/abilities/mod.rs) | Ambition's player ability / weapon kit. |
| [`ability_cooldown`](src/ability_cooldown.rs) | Shared cooldown for the movement abilities (Blink, Grapple) so they read as deliberate verbs instead of spammable teleports. |
| [`actor`](src/actor.rs) | The neutral **actor vocabulary** home for shared sim-state — the components every actor carries, the player included. |
| [`affordances`](src/affordances/mod.rs) | Player affordances: "what would each button do right now?" |
| [`assets`](src/assets/mod.rs) | Asset registries and load-time wiring. |
| [`audio`](src/audio/mod.rs) | Audio runtime for the Ambition sandbox. |
| [`avatar`](src/avatar/mod.rs) | **The HOME AVATAR** — the body slot 0 owns and returns to, and the policy that belongs to the local human rather than to any body. |
| [`body_mode`](src/body_mode/mod.rs) | Sandbox-side body-mode driver: facade re-exporting [`update_body_mode`]. |
| [`boss_encounter`](src/boss_encounter/mod.rs) | Sandbox-side coordinator for boss fights (distinct from the generic `crate::encounter` enemy-wave system). |
| [`character_sprites`](src/character_sprites/mod.rs) | Spritesheet metadata, atlas/animation logic, and loading for every animated character (player robot, goblins, sandbag, boss, NPCs). |
| [`config`](src/config.rs) | The render-only `rgba` color helper. |
| [`control`](src/control/mod.rs) | **The local control seam** — device frame → slot → the body carrying that slot's player brain. |
| [`cutscene`](src/cutscene.rs) | Cutscene playback runtime (the systems that drive the scripts). |
| [`cutscene_trigger`](src/cutscene_trigger.rs) | The cutscene TRIGGER channel — a presentation-neutral request queue. |
| [`debug_label`](src/debug_label.rs) | Compatibility facade for room debug labels. |
| [`dev`](src/dev.rs) | Sim-side developer tooling that still samples actor-domain state. |
| [`dialog`](src/dialog.rs) | Sim-side dialogue glue. |
| [`encounter`](src/encounter/mod.rs) | Generic, reusable enemy-WAVE / arena-lockdown system (data-driven, not scripted) — distinct from `crate::boss_encounter`, which is one specific scripted boss fight with hand-authored phases. |
| [`enemy_projectile`](src/enemy_projectile/mod.rs) | Enemy-fired projectile glue (pirate volleys etc). |
| [`features`](src/features/mod.rs) | The enemy / NPC / boss ECS ACTOR SIMULATION — NOT a feature-toggle layer. |
| [`gravity`](src/gravity/mod.rs) | Gravity-zone mechanic. |
| [`host`](src/host/mod.rs) | Host vocabulary that machinery reads: windowing/display-mode types consumed by the settings model and menu IR. |
| [`items`](src/items/mod.rs) | Actor-sim item adapters. |
| [`menu`](src/menu/mod.rs) | Unified menu content for the sandbox. |
| [`music`](src/music/mod.rs) | Sandbox music adapters over the `ambition_audio` music core. |
| [`persistence`](src/persistence/mod.rs) | Compatibility adapter for persistence paths that still sit inside the gameplay-core UI surface. |
| [`physics`](src/physics.rs) | Shared world physics facade. |
| [`platformer_runtime`](src/platformer_runtime/mod.rs) | Proto-runtime facade for reusable platformer systems. |
| [`projectile`](src/projectile/mod.rs) | Sandbox PLAYER-faction projectile glue. |
| [`quest`](src/quest/mod.rs) | Gameplay-core adapter for the generic quest runtime. |
| [`schedule`](src/schedule/mod.rs) | Schedule + input-frame vocabulary shared by the machinery lib, the content crate, and the app crate. |
| [`session`](src/session/mod.rs) | Sandbox SESSION lifecycle: startup setup ([`setup`]), full reset/respawn ([`reset`]), RON data manifests ([`data`]), and setup glue. |
| [`shrine`](src/shrine.rs) | Healing / save-point shrine. |
| [`time`](src/time/mod.rs) | Time domain plumbing: clocks (ADR 0010/0011), time-control authority, per-entity proper-time scale, and game-feel tuning. |
| [`world`](src/world/mod.rs) | World / level authoring runtime: room graph + spawning, the code-first room builder, the LDtk hot-reloadable project loader, the Avian2D physics adapter, and LDtk-authored moving platforms. |

_35 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->


## Notes

**Read this first if you are about to change something here.** This crate is the
residual `ambition_actors` the decomposition left: 63.5k total src lines
(units: TOTAL, incl. tests). The 2026-07-10 ledger ruling
([`docs/planning/engine/decomposition.md`](../../docs/planning/engine/decomposition.md))
says no further crate split is owed — navigability below the crate line is won by
the D-B standard (every module ≤ ~1.5k lines with a `//!` header stating its ONE
concern, plus this map), not by more crates.

### The three modules whose names mislead

| Name | What it actually is |
|---|---|
| `features` | **the enemy / NPC / boss ECS ACTOR SIMULATION.** Not a feature-toggle layer. "Features" here means in-world entities. The `features/` rename rides the S5/S6 player fold (refactor-chain R6). |
| `actor` (singular) | the neutral **body VOCABULARY** — components every actor carries, the player included. Not systems. New shared body state lands here, never on a `Player*` component. |
| `player` | what is LEFT of player-centrism after R6a–R6c took out the body vocabulary and the control seam: home-avatar POLICY (respawn safety, blink camera, identity bundle, starting character) plus the body mechanics still to be re-homed. The directory finishes dissolving in the rest of R6. |
| `control` | **the local control seam** — device frame → slot → the body carrying that slot's player brain. Not player-centrism: it is the wire between a human and a body. Downstream of it, nothing holds `Res<ControlFrame>`. |

### Authoritative state — who mutates what

- **A body's motion** is `BodyKinematics` + the 18 movement clusters, and exactly
  one kernel entry writes them: `ae::step_motion`. The player
  tick and `update_ecs_actors` are two Bevy systems calling the SAME body tick.
  Do not add a third.
- **A body's melee** is `BodyMelee` / `MeleeSwing`, spawned through
  `combat::hitbox::spawn_melee_strike` — ONE seam for the player and every actor.
- **Who the human is driving** is `ControlledSubject` (the entity carrying
  `Brain::Player(slot)`), never a possession flag and never `PrimaryPlayer`.
  `PrimaryPlayer` means *slot 0's own body*, which is a different question; every
  surviving `PrimaryPlayerOnly` filter in this crate carries a comment saying why
  it is asking that one.
- **The collision world** is `ambition_world::collision::CollisionWorld`, not
  `Res<RoomGeometry>`. A sweep or raycast that reads the bare geometry misses
  moving platforms, ECS solids, and portal carves.

### The two lints that will fail you

- `ambition_runtime/tests/determinism_lints.rs` (ADR 0023) — no ambient RNG, no
  wall clock, no `std` hash-container iteration, no `Entity` as an ordering key.
- `ambition_runtime/tests/control_frame_lint.rs` — only the input layer may hold
  the global `ControlFrame`. A body system that reads it is silently slot-0-only.
  Its allowlist doubles as the netcode N1 checklist.

### Maintaining this file

The table above is generated from each module's own `//!` header:
`python scripts/modules_md.py --write`. `python scripts/modules_md.py` checks for
drift and exits non-zero. Everything under `## Notes` is hand-written and survives
regeneration.
