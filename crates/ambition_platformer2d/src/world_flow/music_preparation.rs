//! Prepare music for incoming, live, and selected neighboring rooms.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;

use crate::audio::{
    catalog::PreparedMusicCatalog, cue_program::PreparedMusicCues, library::KiraAudioSource,
    music::AdaptiveMusicCatalogRegistry, selection::ActiveAudioSelection,
};
use crate::boss_encounter::{BossCatalog, BossSpecRoster};
use crate::platformer::lifecycle::SessionWorldRef;
use crate::world::rooms::RoomSpec;

#[derive(Default)]
pub(super) struct RoomMusicSources {
    pub opening: BTreeSet<String>,
    anticipated: BTreeSet<String>,
}

impl RoomMusicSources {
    fn track(catalog: &PreparedMusicCatalog, id: &str) -> Option<String> {
        catalog
            .music
            .as_ref()?
            .track(id)
            .map(|track| track.resolved_asset_path())
    }

    fn state_sources(catalog: &PreparedMusicCues, cue: &str, state: &str) -> Self {
        let mut sources = Self::default();
        if let Some((cue, state)) = catalog
            .cue(cue)
            .and_then(|cue| cue.state(state).map(|state| (cue, state)))
        {
            let section = cue.section(&state.section_id).expect("prepared section");
            sources
                .opening
                .extend(section.sources.iter().map(|source| source.path.clone()));
            if let Some(next) = state.next_state.as_deref().and_then(|id| cue.state(id)) {
                let section = cue
                    .section(&next.section_id)
                    .expect("prepared continuation");
                sources
                    .anticipated
                    .extend(section.sources.iter().map(|source| source.path.clone()));
            }
        }
        sources
    }

    fn anticipate(&mut self, other: Self) {
        self.anticipated.extend(other.opening);
        self.anticipated.extend(other.anticipated);
    }

    fn paths(&self) -> impl Iterator<Item = &str> {
        self.opening
            .iter()
            .chain(&self.anticipated)
            .map(String::as_str)
    }
}

pub(super) fn room_music_sources(
    catalog: &PreparedMusicCatalog,
    room: &RoomSpec,
    bosses: &BossCatalog,
    waves: Option<&crate::encounter::EncounterWaveBook>,
    phase: Option<&str>,
) -> RoomMusicSources {
    let cues = catalog.cues.as_ref();
    let binding = cues.and_then(|cues| cues.binding_for_room(&room.id));
    let mut sources = if let Some((cues, binding)) = cues.zip(binding) {
        let mut sources = RoomMusicSources::state_sources(
            cues,
            &binding.cue_id,
            binding.entry_state.as_deref().unwrap_or(&binding.state),
        );
        if binding.entry_state.is_some() {
            sources.anticipate(RoomMusicSources::state_sources(
                cues,
                &binding.cue_id,
                &binding.state,
            ));
        }
        sources
    } else {
        let track = cues
            .zip(phase)
            .and_then(|(cues, phase)| cues.room_phase_track(&room.id, phase))
            .or(room.metadata.music_track.as_deref())
            .and_then(|id| RoomMusicSources::track(catalog, id))
            .or_else(|| {
                catalog
                    .music
                    .as_ref()
                    .and_then(|music| RoomMusicSources::track(catalog, &music.default_track))
            });
        RoomMusicSources {
            opening: track.into_iter().collect(),
            ..Default::default()
        }
    };
    let mut fight = |adaptive: Option<(&str, &str)>, track: &str| {
        if let Some(path) = room
            .metadata
            .fight_music_track
            .as_deref()
            .and_then(|id| RoomMusicSources::track(catalog, id))
        {
            sources.anticipated.insert(path);
        } else if let Some((cues, (cue, state))) = cues.zip(adaptive) {
            sources.anticipate(RoomMusicSources::state_sources(cues, cue, state));
        } else if let Some((cues, binding, state)) =
            cues.zip(binding).and_then(|(cues, binding)| {
                binding
                    .fight_state
                    .as_deref()
                    .map(|state| (cues, binding, state))
            })
        {
            sources.anticipate(RoomMusicSources::state_sources(
                cues,
                &binding.cue_id,
                state,
            ));
        } else if let Some(path) = RoomMusicSources::track(catalog, track) {
            sources.anticipated.insert(path);
        }
    };
    for boss in &room.boss_spawns {
        let behavior = crate::boss_encounter::behavior::authored_boss_behavior(
            bosses,
            &boss.name,
            &boss.payload,
        );
        let seed = bosses
            .encounter(&behavior.id)
            .cloned()
            .unwrap_or_else(crate::boss_encounter::BossEncounterSpec::gradient_sentinel);
        let binding = cues.and_then(|cues| cues.binding_for_boss(&behavior.id));
        for (phase, track) in [("Intro", &seed.music_intro), ("Phase1", &seed.music_phase1)] {
            fight(
                binding.and_then(|binding| {
                    binding
                        .state_in(phase)
                        .map(|state| (binding.cue_id.as_str(), state))
                }),
                track,
            );
        }
    }
    if let Some(encounter) = crate::encounter_features::loading::prepare_room_encounter(room, waves)
    {
        let binding = cues.and_then(|cues| cues.binding_for_encounter(&encounter.id));
        fight(
            binding.map(|binding| (binding.cue_id.as_str(), binding.starting_state.as_str())),
            &encounter.music_track,
        );
        if let Some((binding, state)) =
            binding.and_then(|binding| binding.wave_states.first().map(|state| (binding, state)))
        {
            fight(Some((&binding.cue_id, state)), &encounter.music_track);
        }
    }
    sources
}

#[derive(Resource, Default)]
pub(super) struct RoomMusicPreparation {
    incoming: BTreeMap<crate::load::LoadId, RoomMusicSources>,
    rooms: BTreeMap<String, RoomMusicSources>,
    phase: Option<String>,
    sources: BTreeMap<String, Handle<KiraAudioSource>>,
}

impl RoomMusicPreparation {
    fn reconcile(&mut self, wanted: BTreeSet<String>, server: &AssetServer) {
        self.sources.retain(|path, _| wanted.contains(path));
        for path in wanted {
            self.sources
                .entry(path.clone())
                .or_insert_with(|| server.load(path));
        }
    }
}

pub(super) fn prepare_room_music(
    sessions: Option<Res<crate::provider::PreparedPlatformerSessions>>,
    rooms: Option<crate::world::rooms::LiveRoomSpecs>,
    frozen: Option<SessionWorldRef<PreparedMusicCatalog>>,
    mechanics: Option<Res<crate::construction::mechanics::SessionMechanics>>,
    bosses: Res<BossCatalog>,
    waves: Option<Res<crate::encounter::EncounterWaveBook>>,
    prefetch: Res<super::room_transition_assets::RoomPreparationPrefetchState>,
    transition: Option<Res<crate::runtime::room_transition::RoomTransitionLoadState>>,
    selection: Option<Res<ActiveAudioSelection>>,
    catalogs: Option<Res<AdaptiveMusicCatalogRegistry>>,
    day: Option<Res<crate::time::DayClock>>,
    cycle: Option<Res<crate::time::DayCycle>>,
    server: Option<Res<AssetServer>>,
    // A composition that installs no audio backend never initialises the
    // audio asset type, and the server panics on a load of an uninitialised
    // type. With no audio there is nothing to keep warm.
    audio_sources: Option<Res<Assets<KiraAudioSource>>>,
    mut preparation: ResMut<RoomMusicPreparation>,
    mut direct: Local<Option<PreparedMusicCatalog>>,
) {
    let Some(server) = server else { return };
    if audio_sources.is_none() {
        return;
    }
    let phase = day
        .as_deref()
        .zip(cycle.as_deref())
        .filter(|(day, _)| day.day > 0)
        .map(|(day, cycle)| day.phase(cycle).key());
    let phase_changed = preparation.phase.as_deref() != phase;
    if phase_changed || waves.as_ref().is_some_and(|waves| waves.is_changed()) {
        preparation.incoming.clear();
        preparation.rooms.clear();
        preparation.phase = phase.map(str::to_owned);
    }
    if sessions
        .as_ref()
        .is_some_and(|sessions| sessions.is_changed())
    {
        preparation.incoming.clear();
    }
    let mut pending = BTreeSet::new();
    let mut wanted = BTreeSet::new();
    if let Some(sessions) = sessions {
        for (transaction, prepared) in sessions.published() {
            let load = transaction.barrier.load_id.clone();
            pending.insert(load.clone());
            let sources = preparation.incoming.entry(load).or_insert_with(|| {
                room_music_sources(
                    &prepared.music,
                    prepared.content.source().room_set().activation_spec(),
                    &prepared.mechanical.bosses,
                    waves.as_deref(),
                    phase,
                )
            });
            wanted.extend(sources.paths().map(str::to_owned));
        }
    }
    preparation
        .incoming
        .retain(|load, _| pending.contains(load));
    let mut direct_changed = false;
    if let Some(selection) = selection
        .as_ref()
        .filter(|selection| selection.owner() == Some(crate::sfx::AudioContextOwner::Direct))
    {
        if direct.is_none()
            || selection.is_changed()
            || catalogs
                .as_ref()
                .is_some_and(|catalogs| catalogs.is_changed())
        {
            direct_changed = true;
            *direct = selection
                .provider_id()
                .map(|provider| PreparedMusicCatalog {
                    provider_id: provider.into(),
                    music: selection.music().cloned(),
                    cues: catalogs
                        .as_deref()
                        .and_then(|catalogs| catalogs.catalog_for(provider))
                        .cloned(),
                });
        }
    } else {
        direct_changed = direct.is_some();
        *direct = None;
    }
    if direct_changed
        || frozen
            .as_deref()
            .is_some_and(|catalog| catalog.is_changed())
        || rooms.as_ref().is_some_and(|rooms| rooms.set_is_changed())
        || mechanics
            .as_ref()
            .is_some_and(|mechanics| mechanics.is_changed())
        || (frozen.is_none() && bosses.is_changed())
    {
        preparation.rooms.clear();
    }
    let catalog = frozen
        .as_deref()
        .map(|catalog| &**catalog)
        .or(direct.as_ref());
    let bosses = if frozen.is_some() {
        mechanics.as_deref().map(|mechanics| &mechanics.bosses)
    } else {
        Some(&*bosses)
    };
    let mut resident = BTreeSet::new();
    if let Some((rooms, catalog, bosses)) = rooms
        .as_ref()
        .zip(catalog)
        .zip(bosses)
        .map(|((rooms, catalog), bosses)| (rooms, catalog, bosses))
    {
        let mut indices: BTreeSet<_> = rooms
            .live_definitions()
            .map(|definition| definition.index())
            .collect();
        for id in &prefetch.selected_targets {
            if let Some(index) = rooms.rooms().rooms.iter().position(|room| &room.id == id) {
                indices.insert(index);
            }
        }
        if let Some(active) = transition
            .as_deref()
            .and_then(|transition| transition.active.as_ref())
        {
            indices.insert(active.target_room);
        }
        for index in indices {
            if let Some(room) = rooms.rooms().rooms.get(index) {
                resident.insert(room.id.clone());
                let sources = preparation.rooms.entry(room.id.clone()).or_insert_with(|| {
                    room_music_sources(catalog, room, bosses, waves.as_deref(), phase)
                });
                wanted.extend(sources.paths().map(str::to_owned));
            }
        }
    }
    preparation.rooms.retain(|room, _| resident.contains(room));
    preparation.reconcile(wanted, &server);
}

#[cfg(test)]
mod tests;
