//! One scoped music goal from local listeners and published source occurrences.

use ambition_audio::{
    library::{AudioLibrary, RadioStationState},
    music::*,
    selection::ActiveAudioSelection,
};
use ambition_characters::control::DrivingParticipant;
use ambition_encounter::{
    Encounter, EncounterLifecycle, EncounterMusicRequest, EncounterPhase, EncounterRelease,
    EncounterReleaseReason, EncounterWaves, MusicClaimTier,
};
use ambition_input::participant::InputParticipant;
use ambition_platformer2d_shared_tangle::{
    lifecycle::{LiveRoomInstance, SessionWorldRef},
    sim_id::SimId,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use std::collections::{BTreeSet, HashMap};

pub(super) const LARGE_BRUTE_DELAY_SECONDS: f32 = 3.5;
const DAY_PHASE_CROSSFADE_SECONDS: f32 = 6.0;

pub(super) fn the_room_the_music_plays_for(
    primary: Option<LiveRoomInstance>,
    held: impl IntoIterator<Item = LiveRoomInstance>,
    priority_of: impl Fn(Option<LiveRoomInstance>) -> u8,
) -> Option<LiveRoomInstance> {
    let mut rooms: Vec<_> = held.into_iter().map(Some).collect();
    if primary.is_some() || rooms.is_empty() {
        rooms.push(primary);
    }
    rooms.sort();
    rooms.dedup();
    rooms
        .into_iter()
        .max_by_key(|room| {
            (
                priority_of(*room),
                *room == primary,
                std::cmp::Reverse(*room),
            )
        })
        .flatten()
}

fn local_rooms(
    rooms: &ambition_platformer2d_world::rooms::LiveRoomSpecs,
    bodies: &Query<(Entity, &DrivingParticipant)>,
    locals: &Query<&InputParticipant>,
) -> Vec<LiveRoomInstance> {
    let local: BTreeSet<_> = locals.iter().map(|seat| seat.id.slot()).collect();
    bodies
        .iter()
        .filter(|(_, driver)| local.is_empty() || local.contains(&driver.0.0))
        .filter_map(|(body, _)| rooms.live().of(body))
        .collect()
}

pub fn release_narrative_music_on_room_change(
    rooms: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    primary: ambition_platformer2d_shared_tangle::body::PrimaryLiveRoom,
    participants: Query<(Entity, &DrivingParticipant)>,
    locals: Query<&InputParticipant>,
    encounter_music: Option<SessionWorldRef<EncounterMusicRequest>>,
    mut last: Local<Option<ambition_platformer2d_world::rooms::LiveRoomDefinition>>,
    narrative: Option<ResMut<ambition_conversation::NarrativeMusicRequest>>,
) {
    let Some(rooms) = rooms else { return };
    let listening = local_rooms(&rooms, &participants, &locals);
    let primary = primary.get().filter(|room| listening.contains(room) || locals.is_empty());
    let heard = the_room_the_music_plays_for(
        primary,
        listening,
        |room| {
            encounter_music
                .as_ref()
                .map_or(0, |source| source.priority_of(room))
        },
    );
    let Some(definition) = rooms.definition_named(heard) else {
        return;
    };
    if last
        .replace(definition)
        .is_some_and(|previous| previous != definition)
    {
        if let Some(mut narrative) = narrative {
            narrative.clear();
        }
    }
}

#[derive(SystemParam)]
pub struct MusicFacts<'w, 's> {
    rooms: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs<'w, 's>>,
    primary: ambition_platformer2d_shared_tangle::body::PrimaryLiveRoom<'w, 's>,
    session: ambition_platformer2d_shared_tangle::lifecycle::LiveSessionScope<'w, 's>,
    participants: Query<'w, 's, (Entity, &'static DrivingParticipant)>,
    locals: Query<'w, 's, &'static InputParticipant>,
    encounters: Query<
        'w,
        's,
        (
            Entity,
            &'static Encounter,
            &'static EncounterLifecycle,
            Option<&'static EncounterWaves>,
            &'static SimId,
        ),
    >,
    bosses: Query<
        'w,
        's,
        (
            Entity,
            &'static ambition_boss_encounter::BossConfig,
            &'static ambition_boss_encounter::BossEncounter,
            &'static SimId,
        ),
    >,
    sources: Option<SessionWorldRef<'w, 's, EncounterMusicRequest>>,
    narrative: Option<Res<'w, ambition_conversation::NarrativeMusicRequest>>,
    radio: Option<Res<'w, RadioStationState>>,
    day: Option<Res<'w, ambition_time::DayClock>>,
    cycle: Option<Res<'w, ambition_time::DayCycle>>,
    releases: MessageReader<'w, 's, EncounterRelease>,
    boundary: Option<Res<'w, ambition_platformer2d_core::ConfirmedFrameBoundary>>,
    director: Option<Res<'w, MusicDirectorState>>,
}

fn claim(
    context: ambition_sfx::AudioContextOwner,
    session: Option<u64>,
    room: Option<LiveRoomInstance>,
    source: impl Into<String>,
    attempt: u64,
) -> MusicClaim {
    MusicClaim {
        context,
        session,
        scope: room.map(LiveRoomInstance::ordinal),
        source: source.into(),
        attempt,
    }
}

fn track_choice(
    id: &str,
    library: Option<&AudioLibrary>,
    authority: &ambition_audio::selection::MusicAuthority,
) -> Option<(MusicChoice, u64)> {
    let track = library?.track(id).filter(|_| authority.allows(id))?;
    Some((MusicChoice::Track(id.into()), track.generation))
}

/// Recency resolves a tie inside a room. Primary locality resolves ties across rooms.
fn choose(
    candidates: Vec<(MusicSelection, u64)>,
    primary: Option<LiveRoomInstance>,
) -> Option<MusicSelection> {
    let mut per_room: HashMap<Option<u32>, (MusicSelection, u64)> = HashMap::new();
    for (candidate, began) in candidates {
        let replace = per_room
            .get(&candidate.claim.scope)
            .is_none_or(|(old, old_began)| {
                candidate
                    .priority
                    .cmp(&old.priority)
                    .then(began.cmp(old_began))
                    .then_with(|| old.claim.source.cmp(&candidate.claim.source))
                    .is_gt()
            });
        if replace {
            per_room.insert(candidate.claim.scope, (candidate, began));
        }
    }
    per_room
        .into_values()
        .map(|(selection, _)| selection)
        .max_by(|a, b| {
            (
                a.priority,
                a.claim.scope == primary.map(LiveRoomInstance::ordinal),
                std::cmp::Reverse(a.claim.scope),
            )
                .cmp(&(
                    b.priority,
                    b.claim.scope == primary.map(LiveRoomInstance::ordinal),
                    std::cmp::Reverse(b.claim.scope),
                ))
        })
}

pub fn compute_music_intent(
    mut facts: MusicFacts,
    catalogs: Res<AdaptiveMusicCatalogRegistry>,
    selection: Res<ActiveAudioSelection>,
    library: Option<Res<AudioLibrary>>,
    mut intent: ResMut<MusicIntent>,
    mut withdrawal: Local<Option<(ambition_audio::music::playback::MusicEntry, i32)>>,
) {
    intent.outgoing = None;
    let (Some(context), Some(provider)) = (selection.owner(), selection.provider_id()) else {
        return;
    };
    let library = library.as_deref();
    intent.context = Some(context);
    let catalog = catalogs.catalog_for(provider);
    let session = facts.session.get().map(|scope| scope.0);
    if selection.is_changed()
        || catalogs.is_changed()
        || intent.provider_id.as_deref() != Some(provider)
    {
        intent.provider_id = Some(provider.into());
        let mut authority = selection.music_authority();
        if let Some(catalog) = catalog {
            authority.authorize_cues(catalog.cue_ids().map(str::to_owned));
        }
        intent.authority = authority;
    }
    if let Some(admitted) = facts
        .director
        .as_deref()
        .and_then(MusicDirectorState::admitted)
    {
        let released = facts
            .releases
            .read()
            .filter(|released| {
                admitted
                    .selection
                    .release_owner
                    .as_ref()
                    .is_some_and(|owner| {
                        owner.context == context
                            && owner.scope == released.room.map(LiveRoomInstance::ordinal)
                            && owner.source == released.source.as_str()
                            && owner.attempt == released.attempt
                            && owner.session == Some(released.session.0)
                    })
            })
            .map(|released| match released.reason {
                EncounterReleaseReason::Completed => MusicReleaseReason::Completed,
                EncounterReleaseReason::Failed => MusicReleaseReason::Failed,
                EncounterReleaseReason::Reset => MusicReleaseReason::Reset,
            })
            .min_by_key(|reason| match reason {
                MusicReleaseReason::Completed | MusicReleaseReason::Failed => 0,
                _ => 1,
            });
        let present = facts.sources.as_ref().is_none_or(|sources| {
            sources.candidates().any(|candidate| {
                admitted.selection.claim.source
                    == format!(
                        "{}:{}",
                        candidate.source.kind(),
                        candidate.source.instance_id()
                    )
                    && admitted.selection.claim.scope
                        == candidate.room.map(LiveRoomInstance::ordinal)
                    && admitted.selection.claim.attempt == candidate.attempt
            })
        });
        let reason = if let Some(reason) = released {
            Some(reason)
        } else if admitted.selection.release_owner.is_some() && !present {
            match facts.boundary.as_deref() {
                Some(boundary) => {
                    let at = withdrawal.get_or_insert((admitted.entry, boundary.current));
                    if at.0 != admitted.entry {
                        *at = (admitted.entry, boundary.current);
                    }
                    Some(if boundary.confirmed < at.1 {
                        MusicReleaseReason::AwaitingSettlement
                    } else {
                        MusicReleaseReason::Withdrawn
                    })
                }
                None => Some(MusicReleaseReason::Withdrawn),
            }
        } else {
            *withdrawal = None;
            None
        };
        intent.outgoing = reason.map(|reason| MusicDisposition {
            entry: admitted.entry,
            claim: admitted.selection.claim.clone(),
            reason,
        });
    } else {
        facts.releases.clear();
        *withdrawal = None;
    }
    if intent.authority.is_deliberate_silence() {
        intent.publish(MusicGoal::Silence {
            context: Some(context),
            reason: MusicSilenceReason::ProviderHasNoMusic,
        });
        return;
    }
    let Some(rooms) = facts.rooms.as_ref() else {
        if !matches!(&intent.goal, Some(MusicGoal::Play(selected)) if selected.claim.context == context && selected.claim.session == session)
        {
            if let Some((choice, generation)) = selection
                .music()
                .and_then(|music| track_choice(&music.default_track, library, &intent.authority))
            {
                intent.publish(MusicGoal::Play(MusicSelection {
                    provider: provider.into(),
                    generation,
                    claim: claim(context, session, None, "default", 0),
                    release_owner: None,
                    choice,
                    continuity: MusicContinuity::SharedProgram,
                    entry_policy: MusicEntryPolicy::ContinueCompatible,
                    priority: 0,
                    crossfade_s: 0.22,
                    completed_state: None,
                }));
            }
        }
        return;
    };
    let primary = facts.primary.get();
    let mut listening = local_rooms(rooms, &facts.participants, &facts.locals);
    let primary = primary.filter(|room| listening.contains(room) || facts.locals.is_empty());
    if let Some(primary) = primary {
        listening.push(primary);
    }
    listening.sort();
    listening.dedup();
    let listening: Vec<_> = if listening.is_empty() {
        vec![None]
    } else {
        listening.into_iter().map(Some).collect()
    };
    let heard =
        the_room_the_music_plays_for(primary, listening.iter().flatten().copied(), |room| {
            facts
                .sources
                .as_ref()
                .map_or(0, |sources| sources.priority_of(room))
        });
    let encounters: HashMap<_, _> = facts
        .encounters
        .iter()
        .map(|row| ((rooms.live().of(row.0), row.4.as_str()), row))
        .collect();
    let lifecycles: HashMap<_, _> = facts
        .encounters
        .iter()
        .map(|row| ((rooms.live().of(row.0), row.1.id.as_str()), row))
        .collect();
    let bosses: HashMap<_, _> = facts
        .bosses
        .iter()
        .map(|row| ((rooms.live().of(row.0), row.3.as_str()), row))
        .collect();
    let mut candidates = Vec::new();
    let mut projected_room = false;
    for room in listening {
        let Some(definition) = rooms.definition_named(room) else {
            continue;
        };
        projected_room = true;
        let spec = rooms.rooms().spec(definition);
        let make = |source: String,
                    attempt,
                    choice,
                    generation,
                    priority,
                    continuity,
                    completed_state,
                    release_owner,
                    crossfade_s| MusicSelection {
            provider: provider.into(),
            generation,
            claim: claim(context, session, room, source, attempt),
            release_owner,
            entry_policy: MusicEntryPolicy::ContinueCompatible,
            choice,
            priority,
            continuity,
            completed_state,
            crossfade_s,
        };
        if let Some(track) = facts
            .radio
            .as_deref()
            .and_then(RadioStationState::selected_track)
            .and_then(|id| track_choice(id, library, &intent.authority))
        {
            candidates.push((
                make(
                    "radio".into(),
                    0,
                    track.0,
                    track.1,
                    400,
                    MusicContinuity::SharedProgram,
                    None,
                    None,
                    0.22,
                ),
                0,
            ));
        }
        for source in facts
            .sources
            .as_ref()
            .into_iter()
            .flat_map(|sources| sources.candidates())
            .filter(|source| source.room == room)
        {
            let mut lifecycle_owner = None;
            let mut completed = None;
            let mut adaptive = None;
            match source.source.kind() {
                "encounter" => {
                    let Some((_, encounter, lifecycle, waves, id)) =
                        encounters.get(&(room, source.source.instance_id()))
                    else {
                        continue;
                    };
                    if lifecycle.attempt() != source.attempt || !lifecycle.phase().in_flight() {
                        continue;
                    }
                    lifecycle_owner =
                        Some(claim(context, session, room, id.as_str(), source.attempt));
                    if let (Some(waves), Some(binding)) = (
                        waves,
                        catalog.and_then(|c| c.binding_for_encounter(&encounter.id)),
                    ) {
                        adaptive =
                            encounter_state(binding, lifecycle.phase(), waves).map(|state| {
                                MusicChoice::Cue {
                                    id: binding.cue_id.clone(),
                                    state,
                                    entry: None,
                                }
                            });
                        completed = Some(binding.cleared_state.clone());
                    }
                }
                ambition_boss_encounter::BOSS_MUSIC_SOURCE_KIND => {
                    let Some((_, config, status, _)) =
                        bosses.get(&(room, source.source.instance_id()))
                    else {
                        continue;
                    };
                    let Some((_, _, lifecycle, _, id)) =
                        lifecycles.get(&(room, config.id.as_str()))
                    else {
                        continue;
                    };
                    if lifecycle.attempt() != source.attempt || !lifecycle.phase().in_flight() {
                        continue;
                    }
                    lifecycle_owner =
                        Some(claim(context, session, room, id.as_str(), source.attempt));
                    if let Some(binding) =
                        catalog.and_then(|c| c.binding_for_boss(&config.behavior.id))
                    {
                        adaptive = binding
                            .state_in(boss_phase_key(status.encounter_phase()))
                            .map(|state| MusicChoice::Cue {
                                id: binding.cue_id.clone(),
                                state: state.into(),
                                entry: None,
                            });
                        completed = binding.state_in("Death").map(str::to_owned);
                    }
                }
                _ => {}
            }
            let choice = spec
                .metadata
                .fight_music_track
                .as_deref()
                .and_then(|id| track_choice(id, library, &intent.authority))
                .or_else(|| adaptive.map(|choice| (choice, catalogs.generation(provider))))
                .or_else(|| {
                    catalog
                        .and_then(|c| c.binding_for_room(&spec.id))
                        .and_then(|binding| {
                            binding.fight_state.as_ref().map(|state| {
                                (
                                    MusicChoice::Cue {
                                        id: binding.cue_id.clone(),
                                        state: state.clone(),
                                        entry: None,
                                    },
                                    catalogs.generation(provider),
                                )
                            })
                        })
                })
                .or_else(|| {
                    source
                        .track
                        .and_then(|id| track_choice(id, library, &intent.authority))
                });
            let Some((choice, generation)) = choice else {
                continue;
            };
            if matches!(choice, MusicChoice::Track(_)) {
                completed = None;
            }
            let priority = match source.tier {
                MusicClaimTier::Base => 200,
                MusicClaimTier::Focused => 300,
            };
            let mut fight = make(
                format!("{}:{}", source.source.kind(), source.source.instance_id()),
                source.attempt,
                choice,
                generation,
                priority,
                if source.source.kind() == ambition_boss_encounter::BOSS_MUSIC_SOURCE_KIND {
                    MusicContinuity::FreshOwner
                } else {
                    MusicContinuity::SharedProgram
                },
                completed,
                lifecycle_owner,
                0.65,
            );
            fight.entry_policy = MusicEntryPolicy::FreshOccurrence;
            candidates.push((fight, source.began));
        }
        if room == heard {
            if let Some((choice, generation)) = facts
                .narrative
                .as_deref()
                .and_then(|n| n.track())
                .and_then(|id| track_choice(id, library, &intent.authority))
            {
                candidates.push((
                    make(
                        "narrative".into(),
                        0,
                        choice,
                        generation,
                        100,
                        MusicContinuity::SharedProgram,
                        None,
                        None,
                        0.22,
                    ),
                    0,
                ));
            }
        }
        let phase = facts
            .day
            .as_deref()
            .zip(facts.cycle.as_deref())
            .filter(|(day, _)| day.day > 0)
            .map(|(day, cycle)| day.phase(cycle).key());
        let phase_track = catalog
            .zip(phase)
            .and_then(|(catalog, phase)| catalog.room_phase_track(&spec.id, phase));
        let room_choice = catalog
            .and_then(|c| c.binding_for_room(&spec.id))
            .map(|binding| {
                (
                    MusicChoice::Cue {
                        id: binding.cue_id.clone(),
                        state: binding.state.clone(),
                        entry: binding.entry_state.clone(),
                    },
                    catalogs.generation(provider),
                )
            })
            .or_else(|| {
                phase_track
                    .or(spec.metadata.music_track.as_deref())
                    .and_then(|id| track_choice(id, library, &intent.authority))
            });
        if let Some((choice, generation)) = room_choice {
            let previous = match intent.goal.as_ref() {
                Some(MusicGoal::Play(previous)) => Some(previous),
                _ => None,
            };
            let fade = previous
                .filter(|previous| previous.claim.scope == room.map(LiveRoomInstance::ordinal))
                .map_or(0.22, |previous| {
                    if previous.choice == choice {
                        previous.crossfade_s
                    } else if phase_track.is_some() {
                        DAY_PHASE_CROSSFADE_SECONDS
                    } else {
                        0.22
                    }
                });
            candidates.push((
                make(
                    format!("room:{}", spec.id),
                    0,
                    choice,
                    generation,
                    10,
                    MusicContinuity::SharedProgram,
                    None,
                    None,
                    fade,
                ),
                0,
            ));
        }
        if let Some((choice, generation)) = selection
            .music()
            .and_then(|music| track_choice(&music.default_track, library, &intent.authority))
        {
            candidates.push((
                make(
                    "default".into(),
                    0,
                    choice,
                    generation,
                    0,
                    MusicContinuity::SharedProgram,
                    None,
                    None,
                    0.22,
                ),
                0,
            ));
        }
    }
    if let Some(winner) = choose(candidates, primary) {
        intent.publish(MusicGoal::Play(winner));
    } else if projected_room
        && (library.is_some()
            || selection
                .music()
                .is_none_or(|music| music.tracks.is_empty()))
    {
        intent.publish(MusicGoal::Silence {
            context: Some(context),
            reason: MusicSilenceReason::NoEligibleChoice,
        });
    }
}

fn encounter_state(
    binding: &EncounterMusicBinding,
    phase: EncounterPhase,
    waves: &EncounterWaves,
) -> Option<String> {
    match phase {
        EncounterPhase::Starting { .. } => Some(binding.starting_state.clone()),
        EncounterPhase::Active => {
            let wave = waves.run.wave_index.unwrap_or(0);
            if wave == 1 && waves.run.wave_elapsed >= LARGE_BRUTE_DELAY_SECONDS {
                if let Some(state) = &binding.wave2_reinforced_state {
                    return Some(state.clone());
                }
            }
            binding
                .wave_states
                .get(wave)
                .or_else(|| binding.wave_states.last())
                .cloned()
                .or_else(|| Some(binding.starting_state.clone()))
        }
        _ => None,
    }
}

pub(super) fn boss_phase_key(phase: ambition_boss_encounter::BossEncounterPhase) -> &'static str {
    use ambition_boss_encounter::BossEncounterPhase as P;
    match phase {
        P::Dormant => "Dormant",
        P::Intro => "Intro",
        P::Phase1 => "Phase1",
        P::Transition => "Transition",
        P::Phase2 => "Phase2",
        P::Stagger => "Stagger",
        P::Enrage => "Enrage",
        P::Death => "Death",
    }
}
