# `ambition_combat` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_combat** — Combat helpers and reusable damage volumes.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`actor_tuning`](src/actor_tuning.rs) | PER-ACTOR RUNTIME TUNING, THE BRAIN-CONSTRUCTION INPUTS, AND THE CONFIG THAT CARRIES THEM. |
| [`attack_support`](src/attack_support.rs) | Attack-phase support: brain-output → engine-input translation, the shared post-hit stagger gates, the moveset down-air's world-orb pogo, and the debug-overlay hitbox source. |
| [`authored_volumes`](src/authored_volumes.rs) | App-local authored attack-volume resolution. |
| [`banner`](src/banner.rs) | Gameplay banner ticking and deferred-request application. |
| [`body_geometry`](src/body_geometry/mod.rs) | ACTOR-NEUTRAL COMBAT GEOMETRY: how a body's collision box and its damageable hurtbox are derived from its pose. |
| [`brain`](src/brain/mod.rs) | THINKING THAT IS NOT THE FLOOR'S BUSINESS. |
| [`breakables`](src/breakables.rs) | Per-frame tick for breakable feature entities: respawn countdown and the stand-to-break collapse trigger. |
| [`capture`](src/capture/mod.rs) | Capture is a persistent relationship between two bodies, separate from hit resolution and [`MovePlayback`](crate::moveset::MovePlayback). |
| [`causal`](src/causal.rs) | Causal facts derived from stock-lifecycle messages. |
| [`clank`](src/clank.rs) | TWO ATTACKS MEETING — hitbox-vs-hitbox arbitration, before either reaches a victim. |
| [`components`](src/components/mod.rs) | ECS-native feature components. |
| [`crowd`](src/crowd.rs) | Crowding classification used by fighter spacing logic. |
| [`death_rules`](src/death_rules.rs) | Game-scoped consequences of participant death (ADR 0033). |
| [`events`](src/events.rs) | Combat message/event vocabulary and small shared value types. |
| [`falling_chest`](src/falling_chest.rs) | Falling-chest physics for ECS reward chests. |
| [`feel`](src/feel.rs) | Live gameplay-feel tuning owned by the combat domain. |
| [`footstool`](src/footstool.rs) | Footstool interaction: jumping off another body. |
| [`hazard_runtime`](src/hazard_runtime.rs) | `HazardRuntime`: the per-hazard runtime blob (id/name/pos/size, its `DamageVolume`, optional patrol `PathMotion`, and resolve `HitMode`) carried by LDtk-entity hazards. |
| [`hazards`](src/hazards.rs) | Hazard tick: patrol motion and contact damage. |
| [`held_items`](src/held_items.rs) | ECS-owned held item capability for actors. |
| [`hit_camera_shake`](src/hit_camera_shake.rs) | Camera-shake intents derived from landed hits. |
| [`hit_reaction`](src/hit_reaction.rs) | Body-generic hit reaction: knockback, directional influence, and reaction timers. |
| [`hitbox`](src/hitbox/mod.rs) | Hitbox-entity lifecycle: spawn, resolve overlaps, then despawn. |
| [`impact_hitstop`](src/impact_hitstop/mod.rs) | The match-level impact freeze: an absolute expiry tick, held in rollback state, that stops the sim clock for a connect nobody is playing. |
| [`ledge_trump`](src/ledge_trump.rs) | Ledge trumping enforces one hanging body per edge. |
| [`moveset`](src/moveset/mod.rs) | Data-driven move playback — the runtime half of the Smash model. |
| [`on_hit`](src/on_hit.rs) | On-hit techniques — conditional effects driven by resolved strike facts. |
| [`path_motion`](src/path_motion.rs) | `PathMotion`: waypoint-following used by moving hazards/platforms. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_combat`. |
| [`rules`](src/rules.rs) | The combat rules a match plays under — resolved, not borrowed. |
| [`snapshot_impls`](src/snapshot_impls.rs) | Rollback wire-format implementations for combat-owned types. |
| [`stale`](src/stale.rs) | Move staling — the history that makes a repeated answer worth less. |
| [`stocks`](src/stocks.rs) | Ruleset-owned lives/stocks accounting. |
| [`strike`](src/strike.rs) | Authoritative live strike volume and lifecycle state. |
| [`targeting`](src/targeting.rs) | Per-frame combat relationship and `ActorTarget` selection. |
| [`util`](src/util.rs) | Small feature-side helpers that do not own a subsystem. |
| [`variation`](src/variation.rs) | Stable per-actor variation helpers for ECS feature actors. |
| [`vitality`](src/vitality.rs) | A move that pays or repays its own mover's health. |
| [`worn_kit`](src/worn_kit.rs) | The kit a body wears: what a character id resolves to when a body puts it on. |

_39 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
