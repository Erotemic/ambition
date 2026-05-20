# Sandbox runtime mirror should not own state the engine already owns

**Trap shape**: a Bevy ECS sandbox carries a per-feature runtime
(`BossRuntime`, `EnemyRuntime`, …) that owns mutable gameplay state
*and* a parallel engine state machine (`ae::BossEncounterState`) that
also owns the same state. The agent (or a past version of the
project) writes a sync system that mirrors deltas from runtime →
engine state to "tie them together."

Now both halves can write to the same logical state. Common bugs:

1. **Double-write** — runtime applies damage immediately; engine
   state applies it next tick when the gameplay-effect bus drains.
   Test cases pass because each half eventually agrees, but the gap
   shows up as one-frame off-by-one HP readings, banner mistiming,
   or VFX firing for hits the engine state never accepted.
2. **Invulnerable-phase corruption** — engine state rejects damage
   during invulnerable beats (Intro / Transition / Stagger), but the
   runtime applied it anyway. The mirror "fixes" it next frame, but
   for one tick the HUD shows wrong HP.
3. **Borrow tangles** — the sync system needs both halves mutable,
   so it clones the registry's lookup maps to avoid the borrow
   checker. The clone hides the real coupling.

**Resolution shape**: pick one authority, mirror the other. In
Ambition's case (OVERNIGHT-TODO #8, landed 2026-05-20):

- Engine `BossEncounterState` owns HP.
- `record_boss_damage` returns an outcome struct
  (`hp_remaining`, `killed`, `applied`) so the caller can mirror the
  new HP onto the runtime + drive VFX on the same tick.
- The sandbox runtime's `boss.health` becomes read-only; the
  encounter system's mirror is the only writer.
- The "gameplay effect" bus reader becomes a typed no-op seam (kept
  for future tracing hooks) instead of a damage-application path.

**Pre-flight check** before merging a "runtime mirror + engine
state" PR:

1. Does the engine type already own this state? If yes, the sandbox
   wrapper should be read-only.
2. Does the sync direction flow ONE WAY (engine → runtime)? If both
   halves mutate the field, you have a double-write.
3. Does the caller need to drive immediate VFX / banner / save
   transitions when the engine state changes? Return an outcome
   struct rather than reading both halves and comparing.

**Bench question for a future agent**: "I have a Bevy ECS runtime
that mirrors an engine state machine. Damage code applies it to
both. Some hits are dropped during invulnerable phases. Which side
should I make authoritative?"

The expected answer: engine state owns the gameplay invariant; the
sandbox runtime is a one-way mirror. Damage routes through a single
function that mutates engine state and returns an outcome the
caller uses for immediate VFX / banner.

**Reference**: see `crates/ambition_sandbox/src/boss_encounter/damage.rs`
for the `record_boss_damage` → `BossDamageOutcome` pattern and the
unit tests that pin its four outcome paths.
