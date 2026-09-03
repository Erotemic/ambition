# Boss system

> **Architecture update (Jon Crall, 2026-07-10):** the optional encounter-wrapper
> direction was correct but incomplete. Boss fights, ordinary wave encounters,
> races, puzzles, escorts, and no-actor set pieces now converge on one generic
> encounter authority. Boss-capable actors retain only actor-local capabilities
> and phase/pattern state. The binding migration plan is
> [`../../systems/boss-encounter-architecture.md`](../../systems/boss-encounter-architecture.md); where this document
> describes boss-specific encounter entities, music, progress, or scripting, that
> machinery is migration input rather than the final authority. Actor-local boss
> behavior and fight-quality guidance remain valid.

> **RE-MEASURED against `ff0e83be5` (2026-09-03), two months on. ⛔ THE "ONE
> BOUNDED SLICE" OF RESIDUE IS TWO HALVES, AND ONE OF THEM WAS DECIDED AGAINST
> TWO DAYS AFTER THIS PAGE LISTED IT.**
>
> | residue | state at HEAD |
> |---|---|
> | `BossAnim`→`CharacterAnim` | ⛔ **not pending — rejected on purpose** |
> | `target_pos` retirement | ▢ still open (`brain/boss_pattern/mod.rs:974`) |
>
> `BossAnim` is alive at `ambition_sprite_sheet/src/boss.rs:28` (7 referencing
> files), and its own doc comment records the decision not to fold it:
> *"E6(b) policy: keep this boss-domain vocabulary for authored boss sheets
> instead of forcing non-GNU-ton rows through `CharacterAnim`. Boss rows name
> attack-geometry verbs (`floor_slam`, `side_sweep`, `spike_halo`, `dash_echo`)
> that are also keys into hurtbox/hitbox metadata; mapping them to character
> locomotion/melee rows would be an adapter, not canonicalization."*
>
> ⭐ **AND THE DATES SETTLE WHICH ONE IS CURRENT.** The fold was written into this
> page on **2026-07-05** (`c8de27d5a`); the keep-policy was written into the code
> on **2026-07-07** (`cdf21e0b1`). The decision is the LATER of the two, and this
> page was never told.
>
> ⚠ **The decision exists in exactly one place in the repository, and it is a
> code comment.** `BossAnim` appears in the live docs only here, in the line
> calling it residue; `E6(b)` appears nowhere in `docs/` at all. So the only
> record that this work was considered and declined is three lines above the
> enum — which is a good place for it to be true and a bad place for it to be
> the only copy. ⇒ Anyone picking up this "bounded slice" would have started the
> fold, found the comment, and had to reconstruct a two-month-old decision from
> it.

Bosses are not a special simulation path — they are actors (see
[`../../concepts/one-body-one-path.md`](../../concepts/one-body-one-path.md)) with **entity-local phase state** and an
**optional encounter wrapper**. The whole system is engine machinery; specific
bosses are content.

> **Status (2026-07-05, current):** the unification LANDED — boss HP/
> liveness/hit-flash live on the shared `BodyHealth`/`BodyCombat`
> (`BossEncounter` is encounter-state only); boss strikes run on the moveset
> runtime (`BossAttackState` is a projection of the live `MovePlayback`);
> gnuton is the ADR 0020 mounted pair with drivable limb actors and
> possession verbs. The remaining residue is ONE bounded slice — the boss
> animator fold in [`../tracks.md`](../tracks.md) Parallel maintenance
> (`BossAnim`→`CharacterAnim`, `target_pos` retirement; the old
> decomposition-ledger "E6" id was retired when that doc became doctrine).
> Fight QUALITY work is
> [`boss-design.md`](boss-design.md). Multi-limb history:
> `docs/archive/planning-superseded/multi-limb-bosses.md`.

---

## The thesis

> Spawn boss X (with tweaks Z) at position Y and it just works — no global encounter
> registration, correct for gauntlets and multiple bosses at once, with phases as a
> trigger-driven property of the entity (its own mechanism, parallel to hitstun).

## The rules

- **Per-entity keying, not archetype-string keying.** Live state (HP, current phase)
  is a component on the entity (`BodyHealth` for HP; the encounter's phase-state on its
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
- **Reactions are message-driven, per-entity.** `BossPhaseEvent` (its `PhaseChanged` variant)
  carries the entity; music / cutscene / reward subscribers never collide across
  simultaneous bosses.
- **⛔⛔ Spacing reasoning about BODIES uses body envelopes, never centres.** A
  `BossPatternContext` carries the target's body box beside its position, and
  `lateral_body_gap` is the separation between the two SURFACES. This is not
  pedantry: the contact-chase closure test used to be
  `centre_distance <= 4.0`, which a 208px-wide boss can only satisfy by standing
  with its centre inside its target's. It never engaged, and with
  `suppress_attacks_while_moving` it therefore never attacked — a defect whose
  severity scaled with body size, so the biggest, most memorable boss in the game
  was the one it silenced completely. Standoff RINGS (`too_close_distance`,
  `engage_distance`) remain distance policy and stay centre-based on purpose;
  only the predicate that claims *contact* was ever making a claim about bodies.

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
— see [`../../architecture/engine-architecture.md`](../../architecture/engine-architecture.md)).

## Pointers

`ambition_characters/src/boss_encounter.rs` (`ActorPhaseState`), HP on the body's
`BodyHealth`, the `BossPattern` brain, `ambition_platformer2d_actor_monolith/src/features/ecs/damage/boss_hit.rs`
(`apply_boss_hit` is the entry; it delegates HP and phase to
`apply_entity_boss_damage`, which now takes its shield through
`ambition_damage::WalletArmor`).
The blast radius of a registry change is ~15 files across machinery / characters / app /
content — run the boss lifecycle tests after.

## Status

The structural refactor (entity-local state, optional encounter, generic scripted
beats) has landed and is headless-green (the canary
`two_same_archetype_bosses_have_independent_encounter_state` guards the keying win).
What remains is **content** (authoring specific encounters, the cut-rope victory NPC)
and **in-game feel** (boss pacing, music / lock-wall timing) — verified against the
real sim and Jon's eye.
