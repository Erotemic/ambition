//! Music candidates from each boss's canonical encounter attempt.

use crate::{BossConfig, BossEncounter, BossEncounterPhase};
use ambition_encounter::{
    Encounter, EncounterLifecycle, EncounterMusicRequest, MusicClaimTier, MusicOccurrence,
};
use ambition_platformer2d_shared_tangle::{
    lifecycle::{LiveRooms, SessionWorldMut},
    sim_id::SimId,
};
use bevy::prelude::*;

pub const BOSS_MUSIC_SOURCE_KIND: &str = "boss_encounter";

pub fn publish_boss_music(
    bosses: Query<(Entity, &BossConfig, &BossEncounter, &SimId)>,
    encounters: Query<(Entity, &Encounter, &EncounterLifecycle)>,
    rooms: LiveRooms,
    tick: Option<Res<ambition_time::SimTick>>,
    mut request: SessionWorldMut<EncounterMusicRequest>,
) {
    let attempts: std::collections::BTreeMap<_, _> = encounters
        .iter()
        .filter(|(_, _, lifecycle)| lifecycle.phase().in_flight())
        .map(|(entity, encounter, lifecycle)| {
            (
                (rooms.of(entity), encounter.id.as_str()),
                lifecycle.attempt(),
            )
        })
        .collect();
    let occurrences = bosses.iter().filter_map(|(entity, config, status, id)| {
        let phase = status.encounter_phase();
        if matches!(
            phase,
            BossEncounterPhase::Dormant | BossEncounterPhase::Death
        ) {
            return None;
        }
        let room = rooms.of(entity);
        let attempt = *attempts.get(&(room, config.id.as_str()))?;
        let elapsed = status
            .encounter
            .as_ref()
            .map_or(0.0, |phase| phase.phase_elapsed);
        Some(MusicOccurrence {
            room,
            source_id: id.as_str().to_owned(),
            attempt,
            track: crate::systems::phase_music_track(&config.seed.encounter, phase, elapsed)
                .map(str::to_owned),
        })
    });
    request.publish_occurrences(
        BOSS_MUSIC_SOURCE_KIND,
        MusicClaimTier::Focused,
        tick.as_deref().map_or(0, |tick| tick.0),
        occurrences,
    );
}
