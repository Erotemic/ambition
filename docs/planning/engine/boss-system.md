# Boss system

**Scope:** the engine mechanism for bosses: per-entity phase state, the
optional encounter entity, scripted beats and the engine/content split.
**Encounter authority:** [boss encounter architecture](../../systems/boss-encounter-architecture.md).
Boss fights, wave encounters, races, puzzles, escorts and no-actor set pieces
share one generic encounter authority; a boss-capable actor keeps only
actor-local capabilities and phase and pattern state.
**Fight quality:** [boss design pipeline](boss-design.md).

## Current shape

Bosses are actors (see [one body, one path](../../concepts/one-body-one-path.md))
with entity-local phase state and an optional encounter wrapper.

- Boss HP, liveness and hit flash are on the shared `BodyHealth` /
  `BodyCombat`. `BossEncounter` is encounter state only.
- Boss strikes run on the moveset runtime. `BossAttackState` is a projection of
  the live `MovePlayback`.
- GNU-ton is the ADR 0020 mounted pair with drivable limb actors and possession
  verbs.
- Boss profiles, encounters, the seed library and validator bands are
  content-pack data (`boss_profiles`, `boss_encounter`, `boss_seed_library`,
  `boss_validator_bands` in `game/ambition_content/assets/pack.ron`).
- Every boss special is a procedural module in `game/ambition_content_modules`.
  A conducted boss (the Flying Spaghetti Monster) has its conductor in a module.

**Open:** retire `target_pos` in
`crates/ambition_characters/src/brain/boss_pattern/mod.rs`.

## Thesis

Spawn boss X (with tweaks Z) at position Y and it works: no global encounter
registration, correct for gauntlets and several bosses at once, with phases as
a trigger-driven property of the entity.

## Rules

- **Key live state per entity.** HP and phase are components on the entity,
  keyed by runtime id, never by the archetype `encounter_id`. Two identical
  bosses must not share HP, phase or music. Canary:
  `two_same_archetype_bosses_have_independent_encounter_state`.
- **Phases are optional data.** A boss carries a possibly empty list of phase
  triggers; empty means a plain tough enemy. Adding or removing phases is a data
  edit. The phase vocabulary is Dormant, Intro, Phase1, Transition, Phase2,
  Stagger, Enrage, Death. Intro invulnerability is opt-in (a `TimeInPhase`
  trigger).
- **Triggers:** `HpBelow(frac)`, `TimeInPhase(s)`, `External(gate: String)`.
- **Phase transition is its own mechanism**, separate from hitstun: a trigger
  fires, a short invulnerable tell (`transition_lock`) runs, then the brain's
  phase swaps. Order every reader of the entity's phase copy after the mirror
  that writes it.
- **The encounter is an optional entity.** HP and phase belong to the boss.
  Thresholds as progress, per-phase music, lock walls, HUD and the scripted
  timeline belong to the encounter. "Cleared" is keyed by encounter placement,
  not archetype.
- **Reactions are per-entity messages.** `BossPhaseEvent` carries the entity, so
  music, cutscene and reward subscribers never collide.
- **Spacing about bodies uses body envelopes, not centres.**
  `BossPatternContext` carries the target's body box, and `lateral_body_gap` is
  the gap between surfaces. A contact predicate on centre distance never engages
  a wide boss. Standoff rings (`too_close_distance`, `engage_distance`) stay
  centre-based on purpose.
- **A replay that un-defeats a boss un-grants its consequences** for every
  family (Q51/Q56, `retract_boss_defeats_on_replay`).

## Scripted encounters are data

A set piece is authored data:
`EncounterScript { beats: [{ when: Trigger, then: [Effect] }] }`.

- **Triggers:** `RopeCut`, `MemberAtPosition`, `HazardImpact`, `MemberDied`,
  `AllMembersDead`, `Timer(s)`, `PlayerEntered`, `Gate(String)`.
- **Effects:** `CommandMoveTo`, `DropHazard`, `ForceKill`, `SetLockWalls`,
  `SetMusic`, `GrantReward`, `ReleasePayload`.

They resolve to inspectable components: `CommandedMove`, `FallingHazard`,
`ReleaseOnDeath` + `PayloadReleased`. Add a beat or effect to this vocabulary,
not a bespoke system.

## Engine and content

The mechanism (phase triggers, the optional encounter entity, the scripted-beat
interpreter, the event channel) is engine. A boss's stats, thresholds, music,
placement and signature specials are content, installed as pack data and
modules.

`BossAnim` stays separate from `CharacterAnim`: boss rows name attack-geometry
verbs (`floor_slam`, `side_sweep`, `spike_halo`, `dash_echo`) that are also keys
into hurtbox and hitbox metadata. Reopen only if a boss sheet needs character
locomotion rows.

## Pointers

- `ambition_characters::boss_encounter::ActorPhaseState`.
- `ambition_boss_encounter::pattern` (ticker, control flow, seeds, validator).
- `features/ecs/damage/boss_hit.rs` in the actor monolith: `apply_boss_hit` is
  the entry; it delegates HP and phase to `apply_entity_boss_damage`, which takes
  its shield through `ambition_damage::WalletArmor`.

Run the boss lifecycle tests after a registry change.

## Architecture boundary

Boss pattern selection, shared actor materialization, accepted combat reaction
and encounter and reward lifecycle are distinct authorities. Boss damage reads
the published `DamageableVolumes`; a boss has no fallback hull. Boss support as
an independent capability is engineering work (Q48): extract it when the seams
are mature, and do not force it.
