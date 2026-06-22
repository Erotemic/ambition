//! Entity-local boss phase mechanism — the Bevy glue (Stage R1).
//!
//! [`tick_boss_phases`] drives each boss's [`BossPhaseState`] from its own HP
//! (`BossStatus.health`) + intrinsic [`PhaseTrigger`]s. It is the trigger-driven
//! replacement for the global [`BossEncounterRegistry`]'s phase machine, built
//! as its OWN system parallel to the combat-feel hitstun code (Jon's decision
//! that phase transitions are a separate mechanism).
//!
//! **Not yet wired into the live schedule.** Through R2 the global registry
//! stays authoritative and `update_boss_encounters` mirrors its state onto
//! `BossStatus.encounter` every frame — which would clobber whatever this
//! system writes. R3 deletes that mirror + the live map and registers this as
//! the sole phase driver (and bridges its [`BossPhaseEvent`]s to the music /
//! banner / VFX consumers). It is compiled + unit-tested here as a real Bevy
//! system so the R3 flip is a one-line `add_systems`.
//!
//! See `docs/planning/boss-entity-local-refactor.md` (R1 / R3).

use bevy::prelude::*;

use crate::combat::boss_clusters::BossStatus;

/// Advance every boss's entity-local phase mechanism by the sim dt.
///
/// Uses [`WorldTime::sim_dt`](crate::WorldTime::sim_dt) so phase pacing freezes
/// in bullet-time / pause alongside the rest of the sim (ADR 0010), matching
/// the registry-driven `update_boss_encounters`.
pub fn tick_boss_phases(
    world_time: Res<crate::WorldTime>,
    mut bosses: Query<&mut BossStatus>,
) {
    let dt = world_time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for mut status in &mut bosses {
        let hp_fraction = status.health.ratio();
        if let Some(phase) = status.encounter.as_mut() {
            // R3: bridge the returned `BossPhaseEvent`s to music / banner / VFX.
            let _events = phase.tick(dt, hp_fraction);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::Health;
    use crate::boss_encounter::{BossEncounterPhase, BossPhaseState, PhaseTrigger};
    use crate::combat::boss_clusters::BossStatus;
    use crate::WorldTime;

    fn boss_status(phase: BossPhaseState) -> BossStatus {
        BossStatus {
            health: Health::new(100),
            alive: true,
            hit_flash: 0.0,
            encounter_phase: BossEncounterPhase::Dormant,
            sprite_metrics: None,
            encounter: Some(phase),
        }
    }

    /// End-to-end proof that `tick_boss_phases` is a legal Bevy system AND that
    /// it advances the entity-local mechanism: an HpBelow trigger fires from
    /// the entity's own HP fraction over a few sim ticks.
    #[test]
    fn system_advances_phase_from_entity_hp() {
        let mut app = App::new();
        app.insert_resource(WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        app.add_systems(Update, tick_boss_phases);

        let mut phase =
            BossPhaseState::new(vec![PhaseTrigger::hp_below(0.5, BossEncounterPhase::Phase1, BossEncounterPhase::Phase2, 0.0)]);
        phase.wake();
        let mut status = boss_status(phase);
        // Drop HP below the 0.5 threshold so the trigger has something to fire on.
        status.health.current = 40;
        let id = app.world_mut().spawn(status).id();

        app.update();

        let status = app.world().entity(id).get::<BossStatus>().unwrap();
        assert_eq!(
            status.encounter.as_ref().unwrap().phase,
            BossEncounterPhase::Phase2,
            "the system should drive the HpBelow transition from the entity's HP"
        );
    }

    /// A paused sim (sim_dt == 0) must not advance phases.
    #[test]
    fn system_is_inert_while_paused() {
        let mut app = App::new();
        app.insert_resource(WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 0.0,
        });
        app.add_systems(Update, tick_boss_phases);

        let mut phase = BossPhaseState::new(vec![PhaseTrigger::time_in_phase(
            0.1,
            BossEncounterPhase::Phase1,
            BossEncounterPhase::Phase2,
            0.0,
        )]);
        phase.wake();
        let id = app.world_mut().spawn(boss_status(phase)).id();

        for _ in 0..10 {
            app.update();
        }

        let status = app.world().entity(id).get::<BossStatus>().unwrap();
        assert_eq!(
            status.encounter.as_ref().unwrap().phase,
            BossEncounterPhase::Phase1,
            "phases must freeze while the sim is paused"
        );
    }
}
