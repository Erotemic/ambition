---
status: current
related_docs:
  - docs/systems/boss-behavior-profiles.md
  - docs/systems/persistence-settings-and-progression.md
---

# Boss encounter architecture

An encounter is a generic lifecycle with provider-authored participants,
objectives, rewards, and presentation. Boss profiles supply the behavior of
individual combatants; they do not own encounter completion.

## Lifecycle authority

`crates/ambition_encounter/src/lifecycle.rs` owns `EncounterLifecycle` and its
state machine:

```text
Inactive -> Starting { remaining } -> Active -> Completed | Failed
              (optional intro; zero duration can enter Active directly)
```

An `EncounterCommand` requests transitions. The lifecycle reducer owns phase
changes; consumers read `phase()` rather than writing it. Wave progress and
other encounter-specific facts live in their own policy/state components.

The provider's `boss_encounters/<id>.ron` describes an encounter's numeric and
music configuration, while `boss_profiles.ron` describes the boss's behavior.
`crates/ambition_boss_encounter/src/profile.rs` assembles the installed pair.
Placement, room construction, activation triggers, and any scripted beats
supply the surrounding experience.

## Ownership and effects

- Encounter: activation, active participants, phases, gates, objectives,
  completion, and reward authorization.
- Body, brain, and combat: individual action, motion, health, strikes, and
  accepted damage.
- Progression/save: durable completion and rewards.
- Presentation: music, camera, UI, dialogue, and VFX reacting to encounter
  facts. Presentation does not decide the outcome.

Room departure, encounter reset, and snapshot restoration must not leave
stale participants or duplicate a one-shot reward. Prefer canonical room and
entity lifecycle facilities rather than a special cleanup list for one boss.

## Adding or changing an encounter

Start with a nearby `boss_encounters/<id>.ron` and corresponding
`boss_profiles.ron` row. Trace the activation/placement in the relevant room.
Check that entering, defeating, resetting, and revisiting the encounter
produce the intended state. When the change affects rewards or rollback,
exercise that transition specifically. Broader tests are appropriate only
when modifying the shared encounter lifecycle.

See [boss behavior profiles](boss-behavior-profiles.md).
