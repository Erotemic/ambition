//! A temporary playtest aid: a player's shot takes a boss to its next phase.
//!
//! Jon, 2026-10-24: to playtest each phase of a fight without grinding it
//! down, a projectile from a player-driven body deals at least what takes the
//! boss to its next phase boundary (death after the last phase). It is a
//! floor, not a cap: a hit that deals more still deals more, so a strong
//! enough hit carries the boss past a boundary as it always would. A boss
//! never protects itself from being one-shot or from phase skipping.
//!
//! Off unless a host inserts it on. Retire it when the planned playtest
//! mechanism lands.

use bevy::prelude::Resource;

use crate::ActorPhaseState;

/// Whether a player's projectile takes a boss at least to its next phase.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerShotsReachNextPhase(pub bool);

/// The damage that takes a boss at `current` of `max` health to the next
/// boundary its phase state names: the first health at or below the
/// boundary's fraction, which is where its `HpBelow` trigger fires.
pub fn damage_to_next_phase(phase: &ActorPhaseState, current: i32, max: i32) -> i32 {
    let boundary = phase.next_hp_boundary().unwrap_or(0.0);
    let target = (boundary * max as f32).floor() as i32;
    (current - target).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BossEncounterPhase, PhaseTrigger};

    fn two_phase_boss() -> ActorPhaseState {
        let mut phase = ActorPhaseState::new(vec![
            PhaseTrigger::hp_below(0.64, BossEncounterPhase::Phase1, BossEncounterPhase::Phase2, 0.0),
            PhaseTrigger::hp_below(0.28, BossEncounterPhase::Phase2, BossEncounterPhase::Enrage, 0.0),
        ]);
        let _ = phase.wake();
        phase
    }

    /// The shot lands exactly on the boundary, and the boundary is where the
    /// trigger fires: the phase changes on the next tick.
    #[test]
    fn a_shot_takes_the_boss_to_the_health_its_next_trigger_fires_at() {
        let mut phase = two_phase_boss();
        let damage = damage_to_next_phase(&phase, 42, 42);
        assert_eq!(damage, 42 - 26, "0.64 of 42 is 26.88, so the boundary is 26");
        let events = phase.tick(0.016, (42 - damage) as f32 / 42.0);
        assert!(!events.is_empty(), "the hit reached the boundary but no phase fired");
        assert_eq!(phase.phase, BossEncounterPhase::Phase2);
        assert_eq!(damage_to_next_phase(&phase, 26, 42), 26 - 11, "0.28 of 42 is 11.76");
    }

    /// After the last HP trigger the next boundary is death.
    #[test]
    fn in_the_last_phase_the_shot_is_lethal() {
        let mut phase = two_phase_boss();
        let _ = phase.tick(0.016, 0.5);
        let _ = phase.tick(0.016, 0.2);
        assert_eq!(phase.phase, BossEncounterPhase::Enrage);
        assert_eq!(damage_to_next_phase(&phase, 9, 42), 9);
    }
}
