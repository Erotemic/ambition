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
> | `target_pos` retirement | ▢ still open (`crates/ambition_characters/src/brain/boss_pattern/mod.rs:981`) |
>
> ✔ **RE-DERIVED 2026-09-17 and both rows hold.** `BossAnim` is still at
> `crates/ambition_sprite_sheet/src/boss.rs:28` with seven variants, and the
> reference count reproduces EXACTLY at 7 files, 5 of them production.
> ⚠ **The instrument matters and nearly cost a wrong correction:** a plain
> `grep -rl BossAnim` answers **27**, because `BossAnimDrivePhase` and its
> siblings contain the name. The 7 needs `grep -rlE '\bBossAnim\b'`. A number
> that TRIPLES under a looser pattern is the reading to distrust, not the page.
>
> `BossAnim`'s own doc comment records the decision not to fold it:
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
> ⚠ **The decision existed in exactly one place in the repository, and it was a
> code comment.** `BossAnim` appeared in the live docs only in the line calling
> it residue; `E6(b)` appears nowhere in `docs/` at all, re-checked 2026-09-17. A
> code comment is a good place for a decision to be TRUE and a bad place for it
> to be the only copy — anyone picking up this "bounded slice" would have started
> the fold, found the comment, and had to reconstruct a two-month-old decision
> from it. ⇒ **Closed by writing it down as a decision rather than as evidence:
> see "Decided" below.**

## Decided

- ⛔ **`BossAnim` does NOT fold into `CharacterAnim` (2026-07-07, `cdf21e0b1`).**
  Boss rows name attack-geometry verbs — `floor_slam`, `side_sweep`,
  `spike_halo`, `dash_echo` — that are also keys into hurtbox/hitbox metadata, so
  mapping them onto character locomotion/melee rows would be an ADAPTER rather
  than canonicalization. The enum stays boss-domain vocabulary for authored boss
  sheets. ⚠ This page listed the fold as pending residue on 2026-07-05, two days
  BEFORE the decision; the decision is the later of the two and is the one that
  stands. Do not reopen it as tidiness — reopen it only if a boss sheet ever
  wants character locomotion rows, which is a content fact, not a naming one.

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
> (`BossAnim`→`CharacterAnim` — ⛔ since DECIDED AGAINST, see "Decided" above —
> and `target_pos` retirement, which is the one that is actually still open; the
> old decomposition-ledger "E6" id was retired when that doc became doctrine).
> Fight QUALITY work is
> [`boss-design.md`](boss-design.md). Multi-limb history:
> docs/archive/planning-superseded/multi-limb-bosses.md (removed from the checkout 2026-09-05; still in git history).

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

## Boss boundary in the architecture reassessment

Boss pattern/content selection, shared actor materialization, accepted combat
reaction and encounter/reward lifecycle are distinct authorities. The existing
boss crates are evidence of some separation, not permission to move every boss
caller into one new capability. Preserve the current body construction road.

A2 in the [frontier](actor-monolith-work-frontier.md) first aligns projectile boss
admission and damage with published authored hurt geometry, including an explicit
empty set. That fix precedes removal of feature-family dispatch. Boss health,
invulnerability and reward policy remain with their existing semantic owners;
Q48 and boss replay/reward choices are not answered by the geometry repair.
