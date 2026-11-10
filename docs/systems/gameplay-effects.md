# Gameplay effects and ECS messages

A simulation system uses typed Bevy messages to request effects owned by
another domain. The receiving domain decides how to apply the request.
This keeps combat, quests, switches, audio, and presentation from reaching
into one another's internal resources.

## Cross-domain requests

`crates/ambition_platformer2d_runtime/src/effect_bus.rs` handles four
focused requests:

| Message | Consumed by | Result |
|---|---|---|
| `SetFlagRequested` | `apply_flag_effects` | Changes the save flag through quest helpers and advances matching quest state |
| `QuestAdvanceRequested` | `apply_quest_effects` | Enqueues an authored quest event |
| `SwitchActivated` | `apply_switch_effects` | Queues the encounter activation and switch sound |
| `GameplaySfxRequested` | `apply_gameplay_sfx_effects` | Emits a sound message |

These messages do not replace the other domains' specific event types.
`ActorStimulus` feeds aggression, `ActorActionMessage` requests resolved
actor actions, `GameplayBannerRequested` requests UI text, and presentation
uses messages such as `SfxMessage` and `VfxMessage`.

## Combat events

`ambition_combat::events::HitEvent` is the common typed request for applying
combat harm to bodies and world features. It carries a `CombatVolume`, source,
attacker, room, target, damage, mode, knockback, exclusions, and SFX.

`HitSource` identifies the cause (`Melee`, `Projectile`, `Contact`, `Hazard`,
`LeftTheWorld`, or `Pogo`), not whether the attacker was a player or boss.
`HitTarget` distinguishes a resolved body victim from feature-oriented
resolution; that distinction prevents scanning and damaging the same body
twice. The body damage and player-damage consumers apply their respective
parts of the result. Defense and reaction decisions belong downstream, not
in a second role-specific event pipeline.

When adding a new attack, follow an existing producer and consumer for its
kind of strike. Prefer extending the shared hit vocabulary over inventing a
player-only or boss-only damage request.

## Scheduling and rollback

A message being emitted does not imply its effects have already been applied.
The runtime's `combat_schedule.rs` and `world_schedule.rs` establish reader
order. A producer needing same-tick visibility must be scheduled with the
relevant consumer; otherwise the result may be observed on a later tick.

Events and one-shot effects that cross rollback or room-reset boundaries need
explicit replay/cleanup semantics. `RoomReplayAdmitted` represents an
accepted replay, with consumers draining it at their assigned lifecycle seam.

Use an existing domain message when its semantics fit. Add a focused new
message when another domain genuinely owns the requested effect; don't add
an intermediary bus merely to pass the same payload onward.
