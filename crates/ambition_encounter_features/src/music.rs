//! Publish every active encounter's music source after lifecycle reduction.

use ambition_encounter::{
    EncounterLifecycle, EncounterMusicRequest, EncounterTrack, MusicClaimTier, MusicOccurrence,
};
use ambition_platformer2d_shared_tangle::{
    lifecycle::{LiveRooms, SessionWorldMut},
    sim_id::SimId,
};
use bevy::prelude::*;

pub fn publish_encounter_music(
    encounters: Query<(Entity, &EncounterLifecycle, &SimId, Option<&EncounterTrack>)>,
    rooms: LiveRooms,
    tick: Option<Res<ambition_time::SimTick>>,
    mut request: SessionWorldMut<EncounterMusicRequest>,
) {
    let occurrences = encounters
        .iter()
        .filter_map(|(entity, lifecycle, id, track)| {
            lifecycle.phase().in_flight().then(|| MusicOccurrence {
                room: rooms.of(entity),
                source_id: id.as_str().to_owned(),
                attempt: lifecycle.attempt(),
                track: track
                    .filter(|track| !track.0.is_empty())
                    .map(|track| track.0.clone()),
            })
        });
    request.publish_occurrences(
        "encounter",
        MusicClaimTier::Base,
        tick.as_deref().map_or(0, |tick| tick.0),
        occurrences,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_encounter::{EncounterCommandKind, EncounterParticipants};
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, session_world_component,
    };

    #[test]
    fn every_encounter_publishes_without_an_event_or_player_body() {
        let project = |reverse: bool| {
            let mut app = App::new();
            insert_session_world_component(app.world_mut(), EncounterMusicRequest::default());
            let mut ids = vec!["a", "b"];
            if reverse {
                ids.reverse();
            }
            for id in ids {
                let mut lifecycle = EncounterLifecycle::default();
                lifecycle.reduce(
                    0.0,
                    [&EncounterCommandKind::Start],
                    &EncounterParticipants::default(),
                    None,
                );
                app.world_mut().spawn((
                    lifecycle,
                    SimId::encounter(id),
                    EncounterTrack(format!("{id}_track")),
                ));
            }
            app.add_systems(Update, publish_encounter_music);
            app.update();
            let request = session_world_component::<EncounterMusicRequest>(app.world()).unwrap();
            assert_eq!(request.candidates().count(), 2);
            assert!(request.candidates().all(|candidate| candidate.attempt == 1));
            (
                request.desired_track(None).map(str::to_owned),
                request.checksum(),
            )
        };
        let forward = project(false);
        assert_eq!(forward.0.as_deref(), Some("a_track"));
        assert_eq!(forward, project(true));
    }
}
