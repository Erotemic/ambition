//! Boss systems: brain tick (intent), encounter-phase forwarding,
//! sandbox-aware integration, and contact-damage publication.
//!
//! Three Bevy systems live here, chained in the `WorldPrep` set:
//!
//! 1. [`sync_boss_encounter_phase`] — copy the active encounter
//!    phase from `BossEncounterRegistry` into each boss's
//!    `BossRuntime::encounter_phase` mirror. Runs first so the
//!    brain tick below sees the current phase this frame.
//! 2. [`tick_boss_brains_system`] — for every boss with a
//!    `Brain::StateMachine(BossPattern)`, build a
//!    [`BossPatternContext`], call [`tick_boss_pattern`], and
//!    write the resulting [`ActorControlFrame`] + [`BossAttackState`].
//!    `BossAttackState` is the single source of truth for boss
//!    attack state — debug overlay, damage application, and
//!    vulnerable-volume rendering all read from it via the pure
//!    helpers in `content/features/boss_attack_geometry`.
//! 3. [`update_ecs_bosses`] — **integration only**. Reads
//!    `ActorControl::0.desired_vel`, integrates the boss body via
//!    `BossRuntime::integrate_body`, syncs presentation mirrors,
//!    and publishes both strike and body-contact damage by calling
//!    `boss_attack_damage` against the boss's `BossAttackState` —
//!    no runtime attack-state fields are involved.

mod sync;
mod tick;

pub use sync::*;
pub use tick::*;

#[cfg(test)]
mod tests;
