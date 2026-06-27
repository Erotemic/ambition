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
  is a component on the entity. Two identical bosses no longer share HP/phase — that
  whole defect class is gone by construction.
- **Phases are intrinsic-but-OPTIONAL data, not a mode.** A boss carries a list of
  health/time/gate-gated phase triggers — possibly empty. Empty list → a plain tough
  enemy, no phase-up. *Flipping a boss between "has phases" and "no phases" is editing
  its trigger DATA, never a code change.* Triggers: `HpBelow(frac)`, `TimeInPhase(s)`,
  `External(gate)`.
- **Phase transition is its own parallel mechanism** (not shared with hitstun /
  recoil). A trigger fires → a brief invulnerable "tell" beat (`transition_lock`) →
  the brain's exposed phase swaps.
- **The encounter is an OPTIONAL first-class entity.** Split: HP + phase state →
  the *boss* entity; thresholds-as-progress + music + lock-walls + HUD + scripted
  timeline → the *encounter* entity. No encounter entity = no HUD, no walls — just a
  tough enemy. "Cleared" is keyed by **encounter placement**, not archetype, so
  reusing a boss elsewhere isn't pre-cleared.
- **Reactions are message-driven, per-entity.** `BossPhaseChanged` / `BossDefeated`
  carry the entity; music / cutscene / reward subscribers never collide across
  simultaneous bosses.
- **Scripted encounters are data.** `EncounterScript { beats: [{ when: Trigger, then:
  [Effect] }] }` with a shared vocabulary (`Gate`, `MemberDied`, `CommandMoveTo`,
  `DropHazard`, `ForceKill`) — so a bespoke set-piece (cut-the-rope) is authored data,
  not new code.

## Engine vs content

The mechanism (phase triggers, encounter entity, the scripted-beat vocabulary, the
event channel) is **engine**. A boss's stats, phase thresholds, music, placement, and
signature effects are **content**. A second game gets the boss system for free and
installs its own bosses as data.

## Status

The structural refactor (entity-local state, optional encounter, generic scripted
beats) has landed. What remains is **content** (authoring specific encounters) and
**in-game feel** (boss pacing, music/lock-wall timing) — verified the usual way,
against the real sim and Jon's eye.
