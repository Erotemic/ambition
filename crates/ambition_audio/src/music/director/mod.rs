use super::playback::{MusicGroupPhase, MusicProduct};
use super::state::MusicEnding;
use super::*;

mod adaptive;
mod gains;
mod loader;
mod logging;
#[cfg(test)]
mod playback_tests;
mod timing;
pub use loader::load_music_cues;

fn current_is_running(director: &MusicDirectorState, channels: &MusicPlaybackCommands) -> bool {
    channels.current().is_some_and(|group| {
        Some(group.entry) == director.entry
            && group.phase == MusicGroupPhase::Running
            && match &group.product {
                MusicProduct::Track { .. } => director.mode == MusicDirectorMode::SimpleTrack,
                MusicProduct::Section { cue, section, .. } => {
                    director.active_cue_id.as_ref() == Some(cue)
                        && director.current_section_id.as_ref() == Some(section)
                }
            }
    })
}

/// The goal selects music. An ending owns only its outgoing transport work.
pub fn drive_music_director(
    time: Res<Time<Real>>,
    catalogs: Res<AdaptiveMusicCatalogRegistry>,
    assets: Option<ResMut<LoadedMusicCueAssets>>,
    director: Option<ResMut<MusicDirectorState>>,
    intent: Res<MusicIntent>,
    mut channels: MusicPlaybackCommands,
    mut library: Option<ResMut<AudioLibrary>>,
    asset_server: Res<AssetServer>,
    decoded: Res<Assets<KiraAudioSource>>,
    mut music_state: Option<ResMut<MusicPlaybackState>>,
) {
    let (Some(mut director), Some(mut assets)) = (director, assets) else {
        return;
    };
    let mut retained = Vec::with_capacity(2);
    if let Some(selection) = &director.selection {
        retained.push((selection.provider.as_str(), selection.generation));
    }
    if let Some(ending) = &director.ending {
        retained.push((
            ending.admitted.selection.provider.as_str(),
            ending.admitted.selection.generation,
        ));
    }
    assets.retire_generations(&catalogs, &retained);
    let running = current_is_running(&director, &channels);
    let dt = if running { time.delta_secs() } else { 0.0 };
    director.seconds_in_mode += dt;
    if director.mode == MusicDirectorMode::AdaptiveLoop {
        director.seconds_in_loop += dt;
    }
    let Some(goal) = intent.goal.as_ref() else {
        return;
    };
    director.goal_revision = intent.revision;
    let MusicGoal::Play(winner) = goal else {
        if director.entry.is_some() || director.ending.is_some() {
            silence_music_backend(&mut channels, &mut director, music_state.as_deref_mut());
        }
        return;
    };
    let authorized = match &winner.choice {
        MusicChoice::Track(id) => intent.authority.allows(id),
        MusicChoice::Cue { id, .. } => intent.authority.allows_cue(id),
    };
    if intent.context != Some(winner.claim.context)
        || !authorized
        || intent.provider_id.as_deref() != Some(winner.provider.as_str())
    {
        silence_music_backend(&mut channels, &mut director, music_state.as_deref_mut());
        return;
    }

    if let Some(ending) = director.ending.as_ref() {
        let old = &ending.admitted.selection;
        if winner.claim.context != old.claim.context
            || winner.claim.session != old.claim.session
            || winner.claim.scope != old.claim.scope
            || winner.priority >= old.priority
        {
            director.last_ending_result =
                Some((ending.admitted.entry, MusicEndingResult::Interrupted));
            director.ending = None;
        }
    }
    if director.ending.is_none() {
        if let Some(outgoing) = intent.outgoing.as_ref() {
            if let Some(admitted) = director.admitted.clone().filter(|old| {
                old.entry == outgoing.entry
                    && old.selection.claim == outgoing.claim
                    && Some(old.entry) == director.entry
                    && channels.current().is_some_and(|group| {
                        group.entry == old.entry && group.phase == MusicGroupPhase::Running
                    })
                    && winner.claim != old.selection.claim
                    && winner.claim.context == old.selection.claim.context
                    && winner.claim.session == old.selection.claim.session
                    && winner.claim.scope == old.selection.claim.scope
                    && winner.priority < old.selection.priority
            }) {
                match outgoing.reason {
                    MusicReleaseReason::AwaitingSettlement => return,
                    MusicReleaseReason::Completed
                        if admitted.selection.completed_state.is_some() =>
                    {
                        director.ending = director.prepared_cue.clone().map(|cue| MusicEnding {
                            admitted,
                            cue,
                            started: false,
                            elapsed: 0.0,
                            duration: 0.0,
                            waiting_seconds: 0.0,
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    let transport_completed = director.transport_completed;
    let ending_failure = director.ending.as_mut().and_then(|ending| {
        if ending.started {
            return None;
        }
        let section = ending
            .admitted
            .selection
            .completed_state
            .as_deref()
            .and_then(|state| ending.cue.state(state))
            .map(|state| state.section_id.as_str())?;
        let ready = assets.section_ready(
            &ending.admitted.selection.provider,
            ending.admitted.selection.generation,
            &ending.cue,
            section,
            &decoded,
        );
        if !ready {
            ending.waiting_seconds += time.delta_secs();
        }
        if ending.waiting_seconds >= 5.0 {
            return Some(MusicEndingResult::MediaUnavailable);
        }
        if matches!(
            channels.outcome(),
            super::playback::MusicRequestOutcome::Failed(_)
        ) || (channels.current().is_none()
            && !channels.pending_for(ending.admitted.entry)
            && !transport_completed)
        {
            return Some(MusicEndingResult::BackendFailure);
        }
        None
    });
    if let Some(reason) = ending_failure {
        let ending = director.ending.take().expect("an ending was checked");
        director.last_ending_result = Some((ending.admitted.entry, reason));
    }
    if let Some(ending) = director
        .ending
        .as_ref()
        .filter(|ending| !ending.started && director.transport_completed)
    {
        director.pending_state = Some(PendingMusicStateTransition {
            state_id: ending
                .admitted
                .selection
                .completed_state
                .clone()
                .expect("ending state"),
            delay_seconds: 0.0,
        });
    }
    let ending_target = director
        .ending
        .as_ref()
        .filter(|ending| {
            !ending.started
                || ending.elapsed < (ending.duration - DEFAULT_RETURN_OVERLAP_SECONDS).max(0.0)
        })
        .map(|ending| {
            let mut selected = ending.admitted.selection.clone();
            if let MusicChoice::Cue { state, entry, .. } = &mut selected.choice {
                *state = selected.completed_state.clone().expect("ending state");
                *entry = None;
            }
            selected
        });
    if let Some(target) = ending_target {
        drive_selection(
            &target,
            &mut director,
            &catalogs,
            &mut assets,
            &mut channels,
            library.as_deref_mut(),
            &asset_server,
            &decoded,
            music_state.as_deref_mut(),
            dt,
        );
    } else {
        let mut successor = winner.clone();
        if director.ending.is_some() {
            successor.crossfade_s = DEFAULT_RETURN_OVERLAP_SECONDS;
        }
        drive_selection(
            &successor,
            &mut director,
            &catalogs,
            &mut assets,
            &mut channels,
            library.as_deref_mut(),
            &asset_server,
            &decoded,
            music_state.as_deref_mut(),
            dt,
        );
    }
    // An old ending can finish after its successor starts. It never clears that successor.
    let ending_progress = director.ending.as_ref().and_then(|ending| {
        let MusicChoice::Cue { id, .. } = &ending.admitted.selection.choice else { return None };
        let state = ending.admitted.selection.completed_state.as_deref()?;
        let cue = &ending.cue;
        if &cue.id != id { return None; }
        let section = cue.section(&cue.state(state)?.section_id)?;
        let sounding = channels.current().is_some_and(|group|
            group.entry == ending.admitted.entry && group.phase == MusicGroupPhase::Running
                && matches!(&group.product, MusicProduct::Section { section: sounding, .. } if sounding == &section.id));
        Some((sounding, section.duration_seconds(cue)))
    });
    if let Some((sounding, duration)) = ending_progress {
        if let Some(ending) = &mut director.ending {
            if sounding {
                ending.started = true;
                ending.duration = duration;
            }
            if ending.started {
                ending.elapsed += time.delta_secs();
            }
            if ending.started && ending.elapsed >= ending.duration {
                director.last_ending_result =
                    Some((ending.admitted.entry, MusicEndingResult::Finished));
                director.ending = None;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn drive_selection(
    target: &MusicSelection,
    director: &mut MusicDirectorState,
    catalogs: &AdaptiveMusicCatalogRegistry,
    assets: &mut LoadedMusicCueAssets,
    channels: &mut MusicPlaybackCommands,
    library: Option<&mut AudioLibrary>,
    server: &AssetServer,
    decoded: &Assets<KiraAudioSource>,
    mut music_state: Option<&mut MusicPlaybackState>,
    dt: f32,
) {
    let fresh = director
        .selection
        .as_ref()
        .is_none_or(|old| !old.can_continue(target));
    let changed_target = director
        .selection
        .as_ref()
        .is_none_or(|old| old.choice != target.choice);
    let changed_product = fresh
        || (changed_target
            && match &target.choice {
                MusicChoice::Track(_) => false,
                MusicChoice::Cue { state, .. } => director
                    .prepared_cue
                    .as_ref()
                    .and_then(|cue| cue.state(state))
                    .is_some_and(|state| {
                        director.current_section_id.as_ref() != Some(&state.section_id)
                    }),
            });
    if changed_product {
        channels.defer();
        if director.admitted.is_none() && !director.transport_completed {
            director.clear_active_cue();
            director.mode = MusicDirectorMode::Idle;
        }
    }
    match &target.choice {
        MusicChoice::Track(id) => {
            let Some(library) = library else { return };
            if library
                .track(id)
                .is_none_or(|track| track.generation != target.generation)
            {
                return;
            }
            let Some((source, one_shot)) = library.resolve_track(id, server) else {
                return;
            };
            if !decoded.contains(source.id()) {
                return;
            }
            if fresh {
                director.clear_active_cue();
                director.entry = Some(channels.new_entry());
            }
            let entry = director.entry.expect("a selected music entry");
            channels.play_track(
                entry,
                director.goal_revision,
                &target.provider,
                target.generation,
                id,
                source,
                one_shot,
                target.crossfade_s,
            );
            if fresh {
                if let Some(state) = music_state.as_deref_mut() {
                    state.begin_track(id.clone());
                }
            }
            director.mode = MusicDirectorMode::SimpleTrack;
        }
        MusicChoice::Cue { id, state, entry } => {
            let cue = if !fresh {
                director.prepared_cue.clone()
            } else {
                None
            }
            .or_else(|| {
                (catalogs.generation(&target.provider) == target.generation)
                    .then(|| catalogs.catalog_for(&target.provider))
                    .flatten()
                    .and_then(|catalog| catalog.cue(id))
                    .map(|cue| std::sync::Arc::new(cue.clone()))
            });
            let Some(cue) = cue else { return };
            assets.ensure_cue_loaded(&target.provider, target.generation, &cue, server);
            let requested = if fresh {
                entry.as_deref().unwrap_or(state)
            } else {
                state
            };
            let Some(target_state) = cue.state(requested) else {
                return;
            };
            if fresh {
                if !assets.section_ready(
                    &target.provider,
                    target.generation,
                    &cue,
                    &target_state.section_id,
                    decoded,
                ) {
                    return;
                }
                director.clear_active_cue();
                director.mode = MusicDirectorMode::Idle;
                director.entry = Some(channels.new_entry());
                if let Some(state) = music_state.as_deref_mut() {
                    state.silence();
                }
            }
            director.prepared_cue = Some(cue.clone());
            adaptive::drive_adaptive_cue_state(
                director,
                &target.provider,
                target.generation,
                &cue,
                target_state,
                assets,
                decoded,
                channels,
                dt,
                target.crossfade_s,
            );
            logging::log_periodic_state(director, &cue, dt);
        }
    }
    director.selection = Some(target.clone());
}
