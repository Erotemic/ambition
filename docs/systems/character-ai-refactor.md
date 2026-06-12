# Character AI refactor

**Review date:** 2026-05-30. Reviewed against source archive `ambition-source-2026-05-30T104014-5-e721ea65c578`.

This is the companion doc for `crates/ambition_actor/src/actor/ai.rs` and the brain modules. It captures the current state of the shared character-AI vocabulary and the path forward for making actor policy, movement, and effects reusable across NPCs, enemies, bosses, and future player-controlled bodies.

## Current status

The old “brain seam pending” language is stale. The current code has a live universal-brain pipeline:

- every controllable actor type can carry `Brain` + `ActionSet` + `ActorControl` sibling components;
- player input is translated through `Brain::Player` into `ActorControlFrame`;
- player control/simulation phases consume `ActorControl`;
- `emit_brain_action_messages` writes concrete `ActorActionMessage`s from each actor's `ActionSet`;
- live consumers exist for player projectile ticks, enemy ranged projectiles, enemy melee windup starts, player melee starts, GNU-ton apple rain, and Gradient Sentinel boss specials.

The migration is still not “done forever.” Remaining direct paths are now narrow and should be treated as explicit exceptions rather than the main architecture:

- player projectile charge/motion-input logic now consumes `ActionRequest::PlayerProjectileTick`;
- pogo start is still a player-specific input path in the attack lifecycle, with target-surface policy centralized in `BlockKind::is_pogo_target()`;
- some boss and enemy runtime components still own timing/state that would be awkward to move without a focused reason;
- (Was: `ae::Player` aggregate inside `PlayerMovementAuthority`. As of 2026-05-28 the player ECS migration is complete — the player entity carries 18 cluster components, no monolithic aggregate.)

## Why this exists

The sandbox grew multiple nearly-parallel behavior loops: enemies, bosses, peaceful/hostile NPCs, and the player all needed to express “move this way, attack now, fire this projectile, trigger this special.” The universal-brain model keeps policy and capability separate:

```text
Brain = policy: when to move/attack/fire/special
ActionSet = capability: what concrete attack/projectile/special that actor owns
ActorControlFrame = abstract intent for movement/control
ActorActionMessage = concrete effect request after ActionSet resolution
```

This lets two actors share one brain template but look different because their `ActionSet`s differ.

## Engine AI vocabulary

`crates/ambition_actor/src/actor/ai.rs` remains the pure-data evaluator vocabulary:

- `CharacterAiSnapshot` — read-only view of actor/target state.
- `CharacterAiMode` — canonical coarse mode (`Idle`, `Patrol`, `Chase`, `Telegraph`, `Attack`, `Recover`, `Stunned`, `Dead`).
- `CharacterAiIntent` / `CharacterAiOutput` — coarse behavior output.
- `evaluate_character_ai` / `evaluate_character_ai_output` — deterministic, Bevy-free helpers with unit tests.

The sandbox brain system is now the higher-level runtime that maps actor snapshots and policy state into `ae::ActorControlFrame` and then `ActorActionMessage` effects.

## Current brain templates

`crates/ambition_actor/src/brain/state_machine.rs` currently exposes a small set of reusable templates rather than one bespoke brain per enemy:

| Template | Use |
|---|---|
| `StandStill` | Sandbags, idle actors, dialogue-only placeholders. |
| `Patrol` | Peaceful NPCs and simple route behavior. |
| `Wanderer` | Puppy-slug style movement. |
| `MeleeBrute` | Approach + melee + recover hostile actors. |
| `Skirmisher` | Ranged/strafe actors. |
| `Sniper` | Hold-position ranged actors. |
| `BossPattern` | Encounter-driven boss attack profiles and macro states. |
| `Smash` | Experimental Smash-style observation/action policy. |

Per-entity variety should still come from `ActionSet` and authored profiles, not from adding one template per creature.

## Remaining work

- **Data-table cleanup.** Archetype-specific speeds, aggro ranges, attack ranges, cooldown multipliers, and damage still live in sandbox mappings. Push durable tuning into tables/content where it is stable enough.
- **Runtime timer ownership.** Enemy melee active windows and several boss pattern states still live in feature runtime components. That is acceptable when the runtime owns integration state, but avoid adding new policy decisions there.
- **Pogo action ownership.** Decide whether pogo remains an intentionally player-specific attack lifecycle edge or becomes an ActionSet/HitResult concept; do not duplicate target-surface policy.
- **Hit pipeline.** `HitEvent` is the current transport; future cleanup should add richer `HitResult` semantics instead of reviving the old split damage-event shapes.
- **Possession/multiplayer.** The brain/action decomposition makes this cheap in principle, but production routing and UI are still future work.

## Until then

When adding a new enemy, boss behavior, or actor-controlled mechanic:

- Reuse an existing brain template when possible.
- Put concrete attack/projectile/special identity into `ActionSet`.
- Add or extend a focused `ActorActionMessage` consumer for the real effect.
- Keep policy decisions out of feature runtime update loops unless the state truly belongs to the runtime.
- Add tests at the pure brain/action layer first, then add the Bevy integration test for the consumer.

## Validation anchors

```bash
cargo test -p ambition_sandbox --lib character_ai
cargo test -p ambition_sandbox --lib actor_control
cargo test -p ambition_sandbox --lib brain::
cargo test -p ambition_sandbox --lib content::features::ecs::brain_effects
cargo run -p ambition_sandbox --bin headless -- --ticks 30
```
