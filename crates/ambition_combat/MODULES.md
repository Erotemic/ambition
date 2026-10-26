# `ambition_combat` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_combat** — Combat helpers and reusable damage volumes.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`action_emission`](src/action_emission.rs) | The brain's action stream, routed around the moveset. |
| [`actor_tuning`](src/actor_tuning.rs) | PER-ACTOR RUNTIME TUNING, THE BRAIN-CONSTRUCTION INPUTS, AND THE CONFIG THAT CARRIES THEM. |
| [`attack_support`](src/attack_support.rs) | Attack-phase support: brain-output → engine-input translation, the shared post-hit stagger gates, the moveset down-air's world-orb pogo, and the debug-overlay hitbox source. |
| [`authored_volumes`](src/authored_volumes.rs) | App-local authored attack-volume resolution. |
| [`banner`](src/banner.rs) | Gameplay banner ticking and deferred-request application. |
| [`body_geometry`](src/body_geometry/mod.rs) | ACTOR-NEUTRAL COMBAT GEOMETRY: how a body's collision box and its damageable hurtbox are derived from its pose. |
| [`body_landmarks`](src/body_landmarks.rs) | The one landmark query: where a body's hand, head or foot is. |
| [`body_rig`](src/body_rig.rs) | A body's semantic rig on the body, and its pose resolved from simulation clocks every tick. |
| [`brain`](src/brain/mod.rs) | THINKING THAT IS NOT THE FLOOR'S BUSINESS. |
| [`capture`](src/capture/mod.rs) | Capture is a persistent relationship between two bodies, separate from hit resolution and [`MovePlayback`](crate::moveset::MovePlayback). |
| [`causal`](src/causal.rs) | Causal facts derived from stock-lifecycle messages. |
| [`clank`](src/clank.rs) | TWO ATTACKS MEETING — hitbox-vs-hitbox arbitration, before either reaches a victim. |
| [`components`](src/components/mod.rs) | ECS-native feature components. |
| [`crowd`](src/crowd.rs) | Crowding classification used by fighter spacing logic. |
| [`death_rules`](src/death_rules.rs) | Game-scoped consequences of participant death (ADR 0033). |
| [`dormancy_rule`](src/dormancy_rule.rs) | How near an observer must be for a hostile's brain to keep thinking: one game's rule for its own rooms (`room_rules`, field `hostile_wake_radius`). |
| [`events`](src/events.rs) | Combat message/event vocabulary and small shared value types. |
| [`extension`](src/extension.rs) | The combat domain's extension request adapters. |
| [`fade`](src/fade.rs) | A body that FADES between its attacks ([`MeleeTactics::fades`]): a spirit that is faint and intangible while it circles you, and solid while it strikes (Jon, 2026-10-22, the spectral wolf: "fades between attacks"). |
| [`feel`](src/feel.rs) | Live gameplay-feel tuning owned by the combat domain. |
| [`finish_zoom`](src/finish_zoom.rs) | The finishing blow pulls the camera in. |
| [`footstool`](src/footstool.rs) | Footstool interaction: jumping off another body. |
| [`governing_rules`](src/governing_rules.rs) | The rules that govern each live room: the reading of the declarations in [`crate::scoped_rules`], for the room a system or an entity is in. |
| [`guard`](src/guard.rs) | Guards at play: where each one is this tick, the strikes they turn aside, and what a turned strike does to the two bodies. |
| [`hand`](src/hand.rs) | What a body can do right now, kept equal to what its durable facts say. |
| [`hazard_runtime`](src/hazard_runtime.rs) | `HazardRuntime`: the per-hazard runtime blob (id/name/pos/size, its `DamageVolume`, optional patrol `PathMotion`, and resolve `HitMode`) carried by LDtk-entity hazards. |
| [`hazards`](src/hazards.rs) | Hazard tick: patrol motion and contact damage. |
| [`held_items`](src/held_items.rs) | ECS-owned held item capability for actors. |
| [`hit_camera_shake`](src/hit_camera_shake.rs) | Camera-shake intents derived from landed hits. |
| [`hit_reaction`](src/hit_reaction.rs) | Body-generic hit reaction: knockback, directional influence, and reaction timers. |
| [`hitbox`](src/hitbox/mod.rs) | Hitbox-entity lifecycle: spawn, resolve overlaps, then despawn. |
| [`hurtbox_resolution`](src/hurtbox_resolution.rs) | Hurtboxes resolved from simulation state and clocks, independent of rendering. |
| [`impact_hitstop`](src/impact_hitstop/mod.rs) | The match-level impact freeze: an absolute expiry tick, held in rollback state, that stops the sim clock for a connect nobody is playing. |
| [`landing_lag`](src/landing_lag.rs) | How much of a move's landing lag a game charges: a rule that each game states for its own rooms. |
| [`ledge_trump`](src/ledge_trump.rs) | Ledge trumping enforces one hanging body per edge. |
| [`moveset`](src/moveset/mod.rs) | Data-driven move playback — the runtime half of the Smash model. |
| [`on_hit`](src/on_hit.rs) | On-hit techniques — conditional effects driven by resolved strike facts. |
| [`path_motion`](src/path_motion.rs) | `PathMotion`: waypoint-following used by moving hazards/platforms. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_combat`. |
| [`room_rules`](src/room_rules.rs) | The `room_rules` authored-content schema: the per-game rules of a game's own rooms, as data in its pack. |
| [`rules`](src/rules.rs) | The combat rules a match plays under — resolved, not borrowed. |
| [`scoped_rules`](src/scoped_rules.rs) | Rules that one game states for the rooms it governs (ADR 0033). |
| [`shot_birth`](src/shot_birth.rs) | Where a shot is born, and so where its charge is drawn. |
| [`snapshot_impls`](src/snapshot_impls.rs) | Rollback wire-format implementations for combat-owned types. |
| [`stale`](src/stale.rs) | Move staling — the history that makes a repeated answer worth less. |
| [`stocks`](src/stocks.rs) | Ruleset-owned lives/stocks accounting. |
| [`strike`](src/strike.rs) | Authoritative live strike volume and lifecycle state. |
| [`strike_weight`](src/strike_weight.rs) | Which hits land HEAVY: a rule that each game states for its own rooms. |
| [`targeting`](src/targeting.rs) | Per-frame combat relationship and `ActorTarget` selection. |
| [`technique`](src/technique.rs) | What this composition installed a technique handler for. |
| [`util`](src/util.rs) | Small feature-side helpers that do not own a subsystem. |
| [`variation`](src/variation.rs) | Stable per-actor variation helpers for ECS feature actors. |
| [`vitality`](src/vitality.rs) | A move that pays or repays its own mover's health. |
| [`worn_kit`](src/worn_kit.rs) | The kit a body wears: what a character id resolves to when a body puts it on. |

_54 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
