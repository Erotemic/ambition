# Gameplay effects, damage messages, and ECS messages

Cross-system gameplay side effects travel as typed Bevy messages. Each message
has a focused consumer. Do not add a custom bridge resource, a mixed-purpose
effect enum, or a parallel side-effect vector.

## Progression, save and audio effects

Four focused messages carry effects that cross into save, quest, encounter or
standalone audio routing. Each has one consumer system in
`crates/ambition_platformer2d_actor_monolith/src/features/ecs/effect_bus.rs`:

| Message | Owner crate | Consumer |
|---|---|---|
| `SetFlagRequested { id, on }` | `ambition_combat::events` | `apply_flag_effects` (save flag plus a same-frame `QuestAdvanceEvent::FlagSet`) |
| `QuestAdvanceRequested(QuestAdvanceEvent)` | `ambition_persistence::quest` | `apply_quest_effects` |
| `SwitchActivated { activation, pos }` | `ambition_encounter::switches` | `apply_switch_effects` (encounter queue plus click SFX) |
| `GameplaySfxRequested { id, pos }` | `ambition_combat::events` | `apply_gameplay_sfx_effects` |

NPC strike and aggression flow through `ActorStimulus`
(`ambition_combat::events`) to `apply_actor_stimuli`
(`.../features/ecs/aggression.rs`).

## Domain-specific messages

Use a domain-specific message when the consumer is known and the payload is
specific:

- `HitEvent` (`ambition_combat::events`) for **all** combat damage, incoming and
  outgoing.
- `ActorActionMessage` (`ambition_characters::brain`) for resolved brain/action
  requests that spawn or start concrete effects.
- `GameplayBannerRequested` for HUD banner text.
- `RoomReplayAdmitted` (`ambition_combat::events`) for a same-room reset. Its
  readers drain it unconditionally, so a reset is never observed while no body
  exists and re-read later against a different world.
- Presentation messages such as `SfxMessage`, `VfxMessage` and
  `DebrisBurstMessage` for facts that already have a presentation type.

## Damage: one `HitEvent`

`HitEvent` carries `volume` (a `CombatVolume`), `damage`, `source: HitSource`,
`attacker: Option<Entity>`, `room`, `target: HitTarget`, `mode: HitMode`,
`knockback: Option<HitKnockback>`, `ignored_targets` and `strike_sfx`.

`HitSource` names a KIND of harm, not a role: `Melee`, `Projectile`, `Contact`,
`Hazard`, `LeftTheWorld`, `Pogo`. Attacker identity comes from
`HitEvent::attacker`; victim routing comes from `HitTarget`. Do not add
role-shaped variants (player slash, enemy projectile, boss attack).

`apply_feature_hit_events` (`.../features/ecs/damage/mod.rs`) applies hits to
actors, bosses and breakables; the player-damage reader applies victim-side hits
to players. Hostile `Hitbox` entities emit `HitEvent`s on overlap during active
windows.

`HitEvent` does not model defense, armor, poise, stagger or reaction state. Those
are resolved downstream. Extend `HitEvent` handling for new combat behaviour; do
not add a parallel damage event.

## Scheduling contract

1. Simulation systems emit typed messages.
2. Brain/action consumers resolve `ActorActionMessage` into hitboxes,
   projectiles, boss specials and related effects.
3. Damage systems resolve `HitEvent`s and hostile `Hitbox` overlaps against ECS
   feature components.
4. The focused effect readers apply save, quest, switch and SFX side effects;
   the `ActorStimulus` readers apply NPC strike and aggression.
5. Progression systems observe the updated state in the same frame.

Do not make a producer reach into save, quest, boss, switch or audio resources
unless the behaviour is local to that producer.

## Adding gameplay behaviour

Prefer, in order:

1. an existing domain-specific message;
2. a new focused typed message with its own consumer system, for cross-domain
   progression, save or audio routing;
3. for combat, `HitEvent`.
