use super::*;

/// A different cue or a new active request after an ending needs a fresh start.
pub(in crate::music) fn should_restart_adaptive(
    director_active_cue: Option<&str>,
    director_mode: MusicDirectorMode,
    cue_id: &str,
    target_state_is_outro: bool,
) -> bool {
    let same_cue = director_active_cue == Some(cue_id);
    let mode_lost_adaptive = matches!(
        director_mode,
        MusicDirectorMode::SimpleTrack | MusicDirectorMode::Idle
    );
    let outro_to_active =
        same_cue && director_mode == MusicDirectorMode::AdaptiveOutro && !target_state_is_outro;
    !same_cue || mode_lost_adaptive || outro_to_active
}

pub(super) fn drive_adaptive_cue_state(
    director: &mut MusicDirectorState,
    selection: &MusicSelection,
    cue: &PreparedMusicCue,
    target_state: &PreparedMusicState,
    assets: &mut LoadedMusicCueAssets,
    server: &AssetServer,
    decoded: &Assets<KiraAudioSource>,
    channels: &mut MusicPlaybackCommands,
) {
    let provider_id = selection.provider.as_str();
    let generation = selection.generation;
    let same_request = director.active_cue_id.as_deref() == Some(cue.id.as_str())
        && director.requested_state_id.as_deref() == Some(target_state.id.as_str());
    if !same_request {
        director.continuation_state_id = None;
        director.pending_state = None;
    }
    director.requested_state_id = Some(target_state.id.clone());
    // A held game request must not rewind the continuation of a completed one-shot.
    let target_state = director
        .continuation_state_id
        .as_deref()
        .and_then(|id| cue.state(id))
        .unwrap_or(target_state);
    let completed = same_request
        && director.mode == MusicDirectorMode::AdaptiveIntro
        && director.current_state_id.as_deref() == Some(target_state.id.as_str())
        && cue
            .section(&target_state.section_id)
            .is_some_and(|section| {
                director.transport_completed
                    || director.position_seconds.is_some_and(|position| {
                        section
                            .musical_duration_seconds
                            .is_some_and(|end| position >= end)
                    })
            });
    let target_state = if completed {
        target_state
            .next_state
            .as_deref()
            .and_then(|id| cue.state(id))
            .map(|next| {
                director.continuation_state_id = Some(next.id.clone());
                next
            })
            .unwrap_or(target_state)
    } else {
        target_state
    };
    assets.ensure_state_loaded(provider_id, generation, cue, target_state, server);
    let ready = assets.section_ready(
        provider_id,
        generation,
        cue,
        &target_state.section_id,
        decoded,
    );
    if should_restart_adaptive(
        director.active_cue_id.as_deref(),
        director.mode,
        cue.id.as_str(),
        is_outro_target(cue, target_state),
    ) {
        if !ready {
            return;
        }
        start_adaptive_state(
            director,
            selection,
            cue,
            target_state,
            assets,
            channels,
            selection.crossfade_s,
        );
        return;
    }

    let current_state_matches =
        director.current_state_id.as_deref() == Some(target_state.id.as_str());
    if current_state_matches {
        director.active_phase_policy = selection.phase_policy;
        channels.set_section_gains(
            director.entry.expect("a selected music entry"),
            director.goal_revision,
            provider_id,
            &cue.id,
            &target_state.section_id,
            &target_state.gains,
            Some((&target_state.id, selection.phase_policy)),
        );
        return;
    }

    let target_section = match cue.section(&target_state.section_id) {
        Some(section) => section,
        None => {
            warn!(
                target: MUSIC_LOG_TARGET,
                "music state references missing section cue={} state={} section={}",
                cue.id,
                target_state.id,
                target_state.section_id,
            );
            return;
        }
    };

    if let Some(current_section_id) = director.current_section_id.as_deref() {
        if current_section_id == target_section.id {
            director.active_phase_policy = selection.phase_policy;
            channels.set_section_gains(
                director.entry.expect("a selected music entry"),
                director.goal_revision,
                provider_id,
                &cue.id,
                &target_state.section_id,
                &target_state.gains,
                Some((&target_state.id, selection.phase_policy)),
            );
            director.current_state_id = Some(target_state.id.clone());
            return;
        }
    }

    if director.transport_completed
        && director.continuation_state_id.is_some()
        && cue.playback.exhausted == crate::music_policy::MusicExhaustionPolicy::Finish
    {
        return;
    }
    let outro = is_outro_target(cue, target_state);
    if director.mode == MusicDirectorMode::AdaptiveIntro
        && director.active_phase_policy == MusicPhasePolicy::FollowPhase
    {
        if ready {
            start_adaptive_state(
                director,
                selection,
                cue,
                target_state,
                assets,
                channels,
                selection.crossfade_s,
            );
        }
        return;
    }
    if director.mode == MusicDirectorMode::AdaptiveIntro && !outro {
        let intro_done = director
            .current_section_id
            .as_deref()
            .and_then(|id| cue.section(id))
            .is_some_and(|section| {
                director.transport_completed
                    || director.position_seconds.is_some_and(|position| {
                        section
                            .musical_duration_seconds
                            .is_some_and(|end| position >= end)
                    })
            });
        if intro_done && ready {
            start_adaptive_state(
                director,
                selection,
                cue,
                target_state,
                assets,
                channels,
                cue.playback.continuation_crossfade_seconds,
            );
        }
        return;
    }
    if outro {
        if let Some(bridge) = cue
            .post_clear_bridge_state
            .as_deref()
            .and_then(|id| cue.state(id))
        {
            channels.set_section_gains(
                director.entry.expect("a selected music entry"),
                director.goal_revision,
                provider_id,
                &cue.id,
                director
                    .current_section_id
                    .as_deref()
                    .unwrap_or(&bridge.section_id),
                &bridge.gains,
                None,
            );
        }
    }
    let fade = if outro {
        cue.playback.ending_crossfade_seconds
    } else {
        cue.playback.section_crossfade_seconds
    };
    if director.transport_completed {
        if ready {
            start_adaptive_state(
                director,
                selection,
                cue,
                target_state,
                assets,
                channels,
                fade,
            );
        }
        return;
    }
    let Some(current) = channels.current().filter(|group| {
        Some(group.entry) == director.entry
            && group.phase == MusicGroupPhase::Running
            && matches!(&group.product, MusicProduct::Section { cue: c, section: s, .. }
                if c == &cue.id && Some(s) == director.current_section_id.as_ref())
    }) else {
        return;
    };
    let Some(position) = current.position_seconds else {
        return;
    };
    let Some(section) = director
        .current_section_id
        .as_deref()
        .and_then(|id| cue.section(id))
    else {
        return;
    };
    let exit = if outro {
        &cue.playback.ending_exit
    } else {
        &cue.playback.section_exit
    };
    let asset_seconds = current.file_duration_seconds();
    let mut pending = director
        .pending_state
        .take()
        .filter(|pending| {
            pending.state_id == target_state.id
                && pending.occurrence == current.occurrence
                && pending.goal_revision == director.goal_revision
        })
        .or_else(|| {
            let delay = section.delay_to_exit(exit, position, asset_seconds)?;
            Some(MusicTransitionPlan {
                exit: exit.clone(),
                fade_seconds: fade,
                state_id: target_state.id.clone(),
                occurrence: current.occurrence,
                goal_revision: director.goal_revision,
                exit_seconds: position + delay,
                observed_seconds: position,
            })
        });
    let Some(plan) = pending.as_mut() else { return };
    // The file cursor can wrap before the selected marker in the next pass.
    if section.looped && position < plan.observed_seconds {
        if let Some(period) = asset_seconds.or(section.musical_duration_seconds) {
            plan.exit_seconds -= period;
        }
    }
    let reached = position >= plan.exit_seconds;
    plan.observed_seconds = position;
    if reached {
        // If the target is not decoded at this exit, select a new exit.
        if ready {
            start_adaptive_state(
                director,
                selection,
                cue,
                target_state,
                assets,
                channels,
                fade,
            );
        }
    } else {
        director.pending_state = pending;
    }
}

fn start_adaptive_state(
    director: &mut MusicDirectorState,
    selection: &MusicSelection,
    cue: &PreparedMusicCue,
    target_state: &PreparedMusicState,
    assets: &LoadedMusicCueAssets,
    channels: &mut MusicPlaybackCommands,
    crossfade_seconds: f32,
) {
    let Some(section) = cue.section(&target_state.section_id) else {
        warn!(
            target: MUSIC_LOG_TARGET,
            "cannot start missing music section cue={} state={} section={}",
            cue.id,
            target_state.id,
            target_state.section_id,
        );
        return;
    };

    channels.play_section(
        director.entry.expect("a selected music entry"),
        director.goal_revision,
        selection,
        cue,
        section,
        assets,
        target_state,
        crossfade_seconds,
    );
    director.active_cue_id = Some(cue.id.clone());
    director.active_phase_policy = selection.phase_policy;
    director.current_state_id = Some(target_state.id.clone());
    director.current_section_id = Some(section.id.clone());
    director.position_seconds = None;
    director.pending_state = None;
    director.transport_completed = false;
    director.mode = if is_outro_target(cue, target_state) {
        MusicDirectorMode::AdaptiveOutro
    } else if section.looped {
        MusicDirectorMode::AdaptiveLoop
    } else {
        MusicDirectorMode::AdaptiveIntro
    };
}

fn is_outro_target(cue: &PreparedMusicCue, state: &PreparedMusicState) -> bool {
    cue.outro_state.as_deref() == Some(state.id.as_str())
}
