use super::gains::gains_for_state;
use super::timing::{seconds_until_next_bar, seconds_until_next_phrase_marker};
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

/// Has a section that plays one time, `duration` long, ended on this frame?
///
/// The director counts in frames, so it can not start the next section on
/// the sample. It starts it on the frame NEAREST the end: the first frame
/// that is within half a frame of it. The first frame after the end is half
/// a frame late on average, and that is silence.
pub(super) fn intro_has_ended(seconds_in_mode: f32, duration: f32, dt: f32) -> bool {
    seconds_in_mode >= duration - dt.max(0.0) * 0.5
}

pub(super) fn drive_adaptive_cue_state(
    director: &mut MusicDirectorState,
    provider_id: &str,
    generation: u64,
    cue: &MusicCueSpec,
    target_state: &MusicStateSpec,
    assets: &LoadedMusicCueAssets,
    decoded: &Assets<KiraAudioSource>,
    channels: &mut MusicPlaybackCommands,
    dt: f32,
    entry_crossfade_s: f32,
) {
    let same_request = director.active_cue_id.as_deref() == Some(cue.id.as_str())
        && director.requested_state_id.as_deref() == Some(target_state.id.as_str());
    if !same_request {
        director.continuation_state_id = None;
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
                    || intro_has_ended(director.seconds_in_mode, section.duration_seconds(cue), dt)
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
    // Start a section only when all its layers are decoded. Later sections
    // load in parallel and must not delay the opening or stop the current bank.
    if !assets.section_ready(
        provider_id,
        generation,
        cue,
        &target_state.section_id,
        decoded,
    ) {
        return;
    }
    if should_restart_adaptive(
        director.active_cue_id.as_deref(),
        director.mode,
        cue.id.as_str(),
        is_outro_target(cue, target_state),
    ) {
        start_adaptive_state(
            director,
            provider_id,
            generation,
            cue,
            target_state,
            assets,
            channels,
            entry_crossfade_s,
        );
        return;
    }

    let current_state_matches =
        director.current_state_id.as_deref() == Some(target_state.id.as_str());
    if current_state_matches {
        channels.set_section_gains(
            director.goal_revision,
            provider_id,
            &cue.id,
            &target_state.section_id,
            &gains_for_state(cue, target_state),
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
            channels.set_section_gains(
                director.goal_revision,
                provider_id,
                &cue.id,
                &target_state.section_id,
                &gains_for_state(cue, target_state),
            );
            director.current_state_id = Some(target_state.id.clone());
            return;
        }
    }

    if is_outro_target(cue, target_state) && director.mode != MusicDirectorMode::AdaptiveOutro {
        queue_or_fire_outro(
            director,
            provider_id,
            generation,
            cue,
            target_state,
            assets,
            channels,
            dt,
        );
        return;
    }

    if director.mode == MusicDirectorMode::AdaptiveIntro {
        let current_section = director
            .current_section_id
            .as_deref()
            .and_then(|id| cue.section(id));
        let intro_done = current_section
            .map(|section| {
                director.transport_completed
                    || intro_has_ended(director.seconds_in_mode, section.duration_seconds(cue), dt)
            })
            .unwrap_or(true);
        if !intro_done {
            return;
        }
        // The intro played to its end: the section the game asks for now
        // starts on this frame. It is not queued: a queue is a wait of
        // `MIN_TRANSITION_DELAY_SECONDS` with nothing playing, between the
        // intro's last sample and the loop's first.
        director.pending_state = None;
        start_adaptive_state(
            director,
            provider_id,
            generation,
            cue,
            target_state,
            assets,
            channels,
            INTRO_TO_LOOP_CROSSFADE_SECONDS,
        );
        return;
    }

    if let Some(mut pending) = director.pending_state.clone() {
        pending.state_id = target_state.id.clone();
        pending.delay_seconds -= dt;
        if pending.delay_seconds <= 0.0 {
            director.pending_state = None;
            // Intro-to-loop transitions use a shorter crossfade than
            // loop-to-loop section swaps: the intro's last bar already
            // signals the change, and a longer fade smears the loop's
            // downbeat. LOOP_SECTION_CROSSFADE_SECONDS still applies between
            // loop sections, where the overlap hides the boundary.
            let crossfade = if director.mode == MusicDirectorMode::AdaptiveIntro {
                INTRO_TO_LOOP_CROSSFADE_SECONDS
            } else {
                LOOP_SECTION_CROSSFADE_SECONDS
            };
            start_adaptive_state(
                director,
                provider_id,
                generation,
                cue,
                target_state,
                assets,
                channels,
                crossfade,
            );
        } else {
            director.pending_state = Some(pending);
        }
    } else {
        let delay = if director.mode == MusicDirectorMode::AdaptiveLoop {
            seconds_until_next_bar(cue, director.seconds_in_loop).max(MIN_TRANSITION_DELAY_SECONDS)
        } else {
            MIN_TRANSITION_DELAY_SECONDS
        };
        info!(
            target: MUSIC_LOG_TARGET,
            "queue_music_state cue={} state={} section={} delay={:.3}s current_section={:?}",
            cue.id,
            target_state.id,
            target_section.id,
            delay,
            director.current_section_id,
        );
        director.pending_state = Some(PendingMusicStateTransition {
            state_id: target_state.id.clone(),
            delay_seconds: delay,
        });
    }
}

fn queue_or_fire_outro(
    director: &mut MusicDirectorState,
    provider_id: &str,
    generation: u64,
    cue: &MusicCueSpec,
    target_state: &MusicStateSpec,
    assets: &LoadedMusicCueAssets,
    channels: &mut MusicPlaybackCommands,
    dt: f32,
) {
    if director.pending_state.is_none() {
        let pass_seconds = director
            .current_section_id
            .as_deref()
            .and_then(|id| cue.section(id))
            .filter(|section| section.looped)
            .map(|section| section.duration_seconds(cue));
        let delay =
            seconds_until_next_phrase_marker(cue, director.seconds_in_loop, 2.0, pass_seconds)
                .max(MIN_TRANSITION_DELAY_SECONDS);
        info!(
            target: MUSIC_LOG_TARGET,
            "queue_outro cue={} state={} delay={:.3}s loop_t={:.3}",
            cue.id,
            target_state.id,
            delay,
            director.seconds_in_loop,
        );
        director.pending_state = Some(PendingMusicStateTransition {
            state_id: target_state.id.clone(),
            delay_seconds: delay,
        });
    }

    if let Some(bridge_state_id) = cue.post_clear_bridge_state.as_deref() {
        if let Some(bridge) = cue.state(bridge_state_id) {
            channels.set_section_gains(
                director.goal_revision,
                provider_id,
                &cue.id,
                director
                    .current_section_id
                    .as_deref()
                    .unwrap_or(&bridge.section_id),
                &gains_for_state(cue, bridge),
            );
        }
    }

    if let Some(mut pending) = director.pending_state.clone() {
        pending.delay_seconds -= dt;
        if pending.delay_seconds <= 0.0 {
            director.pending_state = None;
            start_adaptive_state(
                director,
                provider_id,
                generation,
                cue,
                target_state,
                assets,
                channels,
                OUTRO_CROSSFADE_SECONDS,
            );
        } else {
            director.pending_state = Some(pending);
        }
    }
}

fn start_adaptive_state(
    director: &mut MusicDirectorState,
    provider_id: &str,
    generation: u64,
    cue: &MusicCueSpec,
    target_state: &MusicStateSpec,
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

    let gains = gains_for_state(cue, target_state);
    let fade = if director.mode == MusicDirectorMode::AdaptiveIntro && section.looped {
        0.0
    } else {
        crossfade_seconds
    };
    channels.play_section(
        director.entry.expect("a selected music entry"),
        director.goal_revision,
        provider_id,
        generation,
        cue,
        section,
        assets,
        gains,
        fade,
    );
    director.active_cue_id = Some(cue.id.clone());
    director.current_state_id = Some(target_state.id.clone());
    director.current_section_id = Some(section.id.clone());
    director.seconds_in_mode = 0.0;
    director.seconds_in_loop = 0.0;
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

fn is_outro_target(cue: &MusicCueSpec, state: &MusicStateSpec) -> bool {
    cue.outro_state.as_deref() == Some(state.id.as_str())
}
