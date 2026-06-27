# Boss system

Bosses are not a special simulation path — they are actors (see
[`unified-actors.md`](unified-actors.md)) with **entity-local phase state** and an
**optional encounter wrapper**. The whole system is engine machinery; specific
bosses are content.

---

## The thesis

> Spawn boss X (with tweaks Z) at position Y and it just works — no global encounter
> registration, correct for gauntlets and multiple bosses at once, with phases as a
> trigger-driven property of the entity (its own mechanism, parallel to hitstun).

## The rules

- **Per-entity keying, not archetype-string keying.** Live state (HP, current phase)
  is a component on the entity (`BossStatus` for HP; the encounter's phase-state on its
  own entity), keyed by a unique **runtime id**, not the archetype `encounter_id`. This
  is the core correctness win: keying by archetype string made two identical bosses
  share HP/phase. (Watch the keying when you touch lifetimes — a pre-refactor bug set
  boss music keyed by archetype id, so a second identical boss cleared the first's
  music the same frame it woke. Everything keys by runtime id now.)
- **Phases are intrinsic-but-OPTIONAL data, not a mode.** A boss carries a list of
  phase triggers — possibly empty. Empty list → a plain tough enemy, no phase-up.
  *Flipping a boss between "has phases" and "no phases" is editing its trigger DATA,
  never a code change.* The phase vocabulary (Dormant → Intro → Phase1 → Transition →
  Phase2 → Stagger → Enrage → Death) is intrinsic, but **forced Intro invulnerability
  is now opt-in** (a `TimeInPhase` trigger), not imposed on every boss.
- **Triggers:** `HpBelow(frac)`, `TimeInPhase(s)`, `External(gate: String)`.
- **Phase transition is its own parallel mechanism** (not shared with hitstun /
  recoil). A trigger fires → a brief invulnerable "tell" beat (`transition_lock`) →
  the brain's exposed phase swaps. **Ordering gotcha:** a system that reads the
  entity's phase copy must be ordered **after** the mirror that writes it, or it sees a
  one-frame-stale phase. Wire new phase readers `.after` the mirror.
- **The encounter is an OPTIONAL first-class entity.** Split: HP + phase state →
  the *boss* entity; thresholds-as-progress + per-phase music + lock-walls + HUD +
  scripted timeline → the *encounter* entity. No encounter entity = no HUD, no walls —
  just a tough enemy. "Cleared" is keyed by **encounter placement**, not archetype, so
  reusing a boss elsewhere isn't pre-cleared.
- **Reactions are message-driven, per-entity.** `BossPhaseChanged` / `BossDefeated`
  carry the entity; music / cutscene / reward subscribers never collide across
  simultaneous bosses.

## Scripted encounters are data

A bespoke set-piece (cut-the-rope, escort, "stand under the thing") is authored data,
not new code: `EncounterScript { beats: [{ when: Trigger, then: [Effect] }] }` over a
shared vocabulary —

- **Triggers:** `RopeCut`, `MemberAtPosition`, `HazardImpact`, `MemberDied`,
  `AllMembersDead`, `Timer(s)`, `PlayerEntered`, `Gate(String)`.
- **Effects:** `CommandMoveTo`, `DropHazard`, `ForceKill`, `SetLockWalls`, `SetMusic`,
  `GrantReward`, `ReleasePayload`.

These resolve to reusable Bevy components an author can inspect: `CommandedMove {
target, speed, arrive_tolerance }`, `FallingHazard { anchor, size, gravity, terminal,
align_tolerance, target, impact_gate }`, `ReleaseOnDeath` + `PayloadReleased`. Add a
new beat/effect to this vocabulary, not a new bespoke system.

## Engine vs content

The mechanism (phase triggers, the optional encounter entity, the scripted-beat VM,
the event channel) is **engine**. A boss's stats, phase thresholds, music, placement,
and signature effects are **content**. A second game gets the boss system for free and
installs its own bosses as data (via the `BOSS_*` / `ENCOUNTER_WAVE_BOOK` install seams
— see [`architecture.md`](architecture.md)).

## Pointers

`ambition_characters/src/boss_encounter.rs` (`BossEncounterState`), the `BossStatus`
HP component, the `BossPattern` brain, `boss_encounter/damage.rs` (`record_boss_damage`).
The blast radius of a registry change is ~15 files across machinery / characters / app /
content — run the boss lifecycle tests after.

## Status

The structural refactor (entity-local state, optional encounter, generic scripted
beats) has landed and is headless-green (the canary
`two_same_archetype_bosses_have_independent_encounter_state` guards the keying win).
What remains is **content** (authoring specific encounters, the cut-rope victory NPC)
and **in-game feel** (boss pacing, music / lock-wall timing) — verified against the
real sim and Jon's eye.
