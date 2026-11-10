---
status: current
related_docs:
  - docs/systems/boss-encounter-architecture.md
  - docs/systems/actors-brains-and-character-content.md
---

# Boss behavior profiles

A boss combines an authored encounter specification with reusable body,
combat, brain, and presentation behavior. Named bosses belong to the game
provider; shared mechanics belong to the engine.

## Where to author a boss

In Ambition, these sources have different jobs:

- `game/ambition_content/assets/data/boss_encounters/<id>.ron` defines the
  encounter's ID, display name, health thresholds, intro/transition timing,
  and phase music (`BossEncounterSpec`).
- `game/ambition_content/assets/data/boss_profiles.ron` defines behavior,
  movement, attacks, strike geometry, rewards, and optional limb routes
  (`BossBehaviorProfile`).
- `game/ambition_content/assets/data/boss_seeds.ron` and the character/art
  sources provide other prepared boss and presentation data.

The schemas live in `crates/ambition_boss_encounter/src/pattern/`.
`BossProfile::from_id` in `crates/ambition_boss_encounter/src/profile.rs`
combines the encounter spec and behavior profile from the installed catalog.
Start with a nearby existing boss and change its authored data rather than
inventing a new construction path.

## Runtime responsibilities

The encounter owns activation, phases, and completion. The boss body owns
its physical state, damage, and action execution. The brain/pattern chooses
moves; shared movement, combat, and projectile systems execute them.
Presentation reads the resulting facts.

`BossBehaviorProfile` includes both flat attack tuning and an
`attack_pattern`. `Cycle` uses the flat attack timings; `Scripted` uses the
phase-keyed timeline. A move may supply custom `strike_geometry` and
`limb_routing` without creating a separate boss-specific collision engine.
For a new mechanic, extend the appropriate shared action or encounter
vocabulary and express the selection in authored data.

## Missing content and validation

The current `BossProfile::from_id` requires an encounter spec. The behavior
lookup can use a **generic fallback** when a named behavior profile is absent.
That is useful for prototyping but is not proof that the intended boss was
fully authored. Inspect the resolved profile when adding or renaming a boss.

Validate the pack and then play the affected encounter. Check the phase
transition, attack behavior, defeat, reset, and reward that your change
actually touches; a broad boss-test run is not a prerequisite for editing
one profile.

Related: [boss encounter architecture](boss-encounter-architecture.md),
[adding a character](../recipes/adding-a-character.md).
