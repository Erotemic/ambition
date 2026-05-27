# TODO: Controllable-entity unification (Player / Enemy / NPC / Boss)

**Review date:** 2026-05-27. Reviewed against source archive `ambition-source-2026-05-26T222032-5-3e93516618a5`.

## Current status

The structural actor/brain migration is substantially landed.

Every controllable actor type can carry `Brain` + `ActionSet` + `ActorControl` sibling components. `Brain::Player` translates per-player input into `ActorControlFrame`; player control and simulation consume `ActorControl`; `emit_brain_action_messages` resolves concrete `ActorActionMessage`s from each actor's `ActionSet`.

Live consumers now include player melee-start gating, hostile enemy ranged projectiles, hostile enemy melee windup starts, GNU-ton apple rain, and Gradient Sentinel boss specials. Treat old pre-migration notes as historical, not current guidance.

## Historical decisions still worth preserving

The previous long execution plan was intentionally collapsed because the original chunks have landed. The durable decisions from that plan are still current guidance:

| Decision | Current form |
|---|---|
| Unified actor shape | `Brain`, `ActionSet`, and `ActorControl` are sibling components on controllable entities rather than fields in one monolithic actor struct. |
| Brain reuse model | Keep a small set of reusable brain templates; put per-entity variety in `ActionSet` specs. |
| Aggressiveness / hostility | Hostility remains policy state inside the brain/template, with `Brain::is_hostile()` / `StateMachineCfg::is_hostile()` as helpers. |
| Crate ownership | Brain backends and effect consumers live in `ambition_sandbox`; `ambition_engine` owns narrow data/control vocabulary such as `ActorControlFrame`. |
| Edge semantics | Action edges live on `ActorControlFrame`; brains write the edge for the tick they want the action. |
| Compatibility posture | Pre-release docs and saves may be broken by direct replacement when it keeps the architecture simpler. |

Design-risk notes that remain relevant: do not introduce a parallel “unbrained” path without profiling; avoid `Box<dyn Brain>` unless there is a concrete extension requirement; and decompose `ae::Player` with overlap-then-delete discipline rather than one broad rewrite.

## Active remaining work

1. **Player projectile path:** decide whether projectile charge / motion-input recognition remains a player-specific `PlayerInputFrame` system or moves behind an ActionSet/ActorActionMessage consumer.
2. **Pogo path:** decide whether pogo remains a raw player-specific verb or becomes an attack variant / hit-result reaction.
3. **Hit pipeline:** replace fragmented `DamageEvent` / hostile `Hitbox` / `PlayerDamageEvent` / boss outcome metadata with a canonical `HitSpec` -> `HitInstance` -> `HitResult` flow.
4. **`ae::Player` decomposition:** migrate reader/writer clusters out of the large `PlayerMovementAuthority.player` aggregate only when a focused cluster is ready.
5. **Possession / co-op proof:** swap `Brain::Player(slot)` onto a non-player actor and prove input, camera, affordances, and action effects all route correctly.
6. **Data cleanup:** push stable archetype and boss numeric knobs into data where appropriate, without moving runtime integration state prematurely.

## Current reading packet

- `docs/systems/brain-driver.md` — current overview.
- `docs/recipes/extending-brains-and-action-sets.md` — current extension recipe.
- `docs/systems/character-ai-refactor.md` — AI/refactor context.
- `docs/current/state.md` — active state summary.
- `docs/mechanics/expressibility-checklist.md` — compact capability/gap status.

## Validation anchors

```bash
cargo test -p ambition_engine actor_control
cargo test -p ambition_sandbox --lib brain::
cargo test -p ambition_sandbox --lib content::features::ecs::brain_effects
cargo test -p ambition_sandbox --lib player::systems
cargo run -p ambition_sandbox --bin headless -- --ticks 30
```
