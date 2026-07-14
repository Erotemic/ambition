use super::gains::{
    gains_for_state, set_bank_targets, zero_all_current_and_targets, zero_all_targets,
};
use super::logging::format_gains;
use super::simple::resume_simple_music;
use super::timing::{seconds_until_next_bar, seconds_until_next_phrase_marker};
use super::*;

/// Decide whether `drive_adaptive_cue_state` should stop the base
/// (simple-track) channel and (re)start the adaptive cue from its
/// intro.
///
/// Three conditions trigger a restart, all preserving the invariant
/// that **simple base track and adaptive layers cannot remain
/// audible at the same time**:
///
/// 1. A different cue is taking over (the obvious case).
/// 2. The director's mode says a simple base track is playing
///    (`SimpleTrack`, `Idle`, `AdaptiveFinished`). Defensive: the
///    primary `resume_simple_music(set_mode = false)` fix prevents
///    this state from coexisting with `Some(active_cue_id)`, but
///    if anything leaves the director in that shape we still need
///    to stop the base channel before the adaptive layers ramp up.
/// 3. The cue is in `AdaptiveOutro` and the new directive points
///    back to a non-outro state — i.e. the encounter restarted
///    during the outro tail. `drive_outro_tail` had already started
///    the base lofi channel for the overlap; we must stop it
///    before the adaptive layers come back.
///
/// Captured as a free function so the decision can be unit-tested
/// without spinning up Bevy resources (audio channels, asset
/// servers, etc.).
pub(in crate::music) fn should_restart_adaptive(
    director_active_cue: Option<&str>,
    director_mode: MusicDirectorMode,
    cue_id: &str,
    target_state_is_outro: bool,
) -> bool {
    let same_cue = director_active_cue == Some(cue_id);
    let mode_lost_adaptive = matches!(
        director_mode,
        MusicDirectorMode::SimpleTrack
            | MusicDirectorMode::Idle
            | MusicDirectorMode::AdaptiveFinished
    );
    let outro_to_active =
        same_cue && director_mode == MusicDirectorMode::AdaptiveOutro && !target_state_is_outro;
    !same_cue || mode_lost_adaptive || outro_to_active
}

/// Decide whether a newly started adaptive bank should begin at its
/// target gain instead of ramping up from silence.
///
/// Intro-to-loop full-mix handoffs need to feel like one continuous
/// cue. If the loop bank starts silent and ramps up while the intro
/// bank fades down, players hear a brief hole and become aware that
/// the engine switched files. Loop-to-loop transitions still use the
/// normal gain smoothing so wave changes can blend rather than pop.
pub(super) fn should_start_new_bank_at_target_gain(
    previous_mode: MusicDirectorMode,
    target_section_looped: bool,
) -> bool {
    previous_mode == MusicDirectorMode::AdaptiveIntro && target_section_looped
}

pub(super) fn drive_adaptive_cue_state(
    director: &mut MusicDirectorState,
    provider_id: &str,
    cue: &MusicCueSpec,
    target_state: &MusicStateSpec,
    assets: &LoadedMusicCueAssets,
    channels: &MusicLayerChannels,
    base_music_channel: &AudioChannel<MusicChannel>,
    settings: &MusicMix,
    dt: f32,
) {
    if should_restart_adaptive(
        director.active_cue_id.as_deref(),
        director.mode,
        cue.id.as_str(),
        is_outro_target(cue, target_state),
    ) {
        base_music_channel.stop().fade_out(AudioTween::new(
            Duration::from_millis(650),
            AudioEasing::OutPowi(2),
        ));
        start_adaptive_state(
            director,
            provider_id,
            cue,
            target_state,
            assets,
            channels,
            settings,
            INTRO_TO_LOOP_CROSSFADE_SECONDS,
        );
        return;
    }

    let current_state_matches =
        director.current_state_id.as_deref() == Some(target_state.id.as_str());
    if current_state_matches {
        let active_bank = director.active_bank;
        set_bank_targets(
            director,
            active_bank,
            gains_for_state(cue, target_state, settings),
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
            let active_bank = director.active_bank;
            set_bank_targets(
                director,
                active_bank,
                gains_for_state(cue, target_state, settings),
            );
            director.current_state_id = Some(target_state.id.clone());
            return;
        }
    }

    if is_outro_target(cue, target_state) && director.mode != MusicDirectorMode::AdaptiveOutro {
        queue_or_fire_outro(
            director,
            provider_id,
            cue,
            target_state,
            assets,
            channels,
            settings,
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
            .map(|section| director.seconds_in_mode >= section.duration_seconds(cue))
            .unwrap_or(true);
        if !intro_done {
            return;
        }
    }

    if let Some(mut pending) = director.pending_state.clone() {
        pending.state_id = target_state.id.clone();
        pending.delay_seconds -= dt;
        if pending.delay_seconds <= 0.0 {
            director.pending_state = None;
            // intro→loop transitions get a tighter crossfade than
            // loop↔loop section swaps. The intro's last bar already
            // signals the change melodically; a longer fade just
            // smears the downbeat of the loop. The longer
            // LOOP_SECTION_CROSSFADE_SECONDS still applies when
            // moving between loop sections (wave1↔wave2 etc.), where
            // we want the overlap to mask the section boundary.
            let crossfade = if director.mode == MusicDirectorMode::AdaptiveIntro {
                INTRO_TO_LOOP_CROSSFADE_SECONDS
            } else {
                LOOP_SECTION_CROSSFADE_SECONDS
            };
            start_adaptive_state(
                director,
                provider_id,
                cue,
                target_state,
                assets,
                channels,
                settings,
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
    cue: &MusicCueSpec,
    target_state: &MusicStateSpec,
    assets: &LoadedMusicCueAssets,
    channels: &MusicLayerChannels,
    settings: &MusicMix,
    dt: f32,
) {
    if director.pending_state.is_none() {
        let delay = seconds_until_next_phrase_marker(cue, director.seconds_in_loop, 2.0)
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
            let active_bank = director.active_bank;
            set_bank_targets(
                director,
                active_bank,
                gains_for_state(cue, bridge, settings),
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
                cue,
                target_state,
                assets,
                channels,
                settings,
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
    cue: &MusicCueSpec,
    target_state: &MusicStateSpec,
    assets: &LoadedMusicCueAssets,
    channels: &MusicLayerChannels,
    settings: &MusicMix,
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

    let previous_mode = director.mode;
    let old_bank = director.active_bank;
    let new_bank = if director.active_cue_id.is_some() {
        old_bank.other()
    } else {
        MusicBank::A
    };
    let target_gains = gains_for_state(cue, target_state, settings);
    let start_new_bank_at_target =
        should_start_new_bank_at_target_gain(previous_mode, section.looped);

    info!(
        target: MUSIC_LOG_TARGET,
        "start_adaptive_state cue={} state={} section={} old_bank={} new_bank={} looped={} crossfade={:.2}s gains={} gain_start={}",
        cue.id,
        target_state.id,
        section.id,
        old_bank.label(),
        new_bank.label(),
        section.looped,
        crossfade_seconds,
        format_gains(target_gains),
        if start_new_bank_at_target { "target" } else { "smooth" },
    );

    channels.stop_bank(new_bank, 80);
    channels.set_bank_silent(new_bank);
    director.current_gains[new_bank.index()] = [0.0; MAX_LAYERS];
    director.target_gains[new_bank.index()] = [0.0; MAX_LAYERS];

    let mut started = 0usize;
    for source in &section.sources {
        let slot = cue
            .layer(&source.layer_id)
            .map(|layer| layer.slot.min(MAX_LAYERS - 1))
            .unwrap_or(0);
        if let Some(handle) = assets.get(
            provider_id,
            &cue.id,
            &section.id,
            &source.layer_id,
        ) {
            channels.play_layer(new_bank, slot, handle, section.looped, LAYER_START_FADE_MS);
            started += 1;
        } else {
            warn!(
                target: MUSIC_LOG_TARGET,
                "missing music source cue={} section={} layer={}",
                cue.id,
                section.id,
                source.layer_id,
            );
        }
    }

    if director.active_cue_id.is_some() && new_bank != old_bank {
        set_bank_targets(director, old_bank, [0.0; MAX_LAYERS]);
        director.fading_bank = Some(old_bank);
        director.fade_stop_seconds = crossfade_seconds + 0.35;
    } else {
        channels.stop_bank(old_bank.other(), 80);
        director.fading_bank = None;
        director.fade_stop_seconds = 0.0;
    }

    if start_new_bank_at_target {
        director.current_gains[new_bank.index()] = target_gains;
        director.target_gains[new_bank.index()] = target_gains;
        for (slot, &gain) in target_gains.iter().enumerate().take(MAX_LAYERS) {
            channels.set_layer_volume(new_bank, slot, gain);
        }
    } else {
        set_bank_targets(director, new_bank, target_gains);
    }
    director.active_cue_id = Some(cue.id.clone());
    director.current_state_id = Some(target_state.id.clone());
    director.current_section_id = Some(section.id.clone());
    director.active_bank = new_bank;
    director.seconds_in_mode = 0.0;
    director.seconds_in_loop = 0.0;
    director.pending_state = None;
    director.default_resume_started = false;
    director.mode = if is_outro_target(cue, target_state) {
        MusicDirectorMode::AdaptiveOutro
    } else if section.looped {
        MusicDirectorMode::AdaptiveLoop
    } else {
        MusicDirectorMode::AdaptiveIntro
    };

    info!(
        target: MUSIC_LOG_TARGET,
        "started_music_sources cue={} state={} section={} bank={} source_count={} volume_blend={:.2}s",
        cue.id,
        target_state.id,
        section.id,
        new_bank.label(),
        started,
        STEM_GAIN_BLEND_SECONDS,
    );
}

pub(super) fn drive_outro_tail(
    director: &mut MusicDirectorState,
    cue: &MusicCueSpec,
    channels: &MusicLayerChannels,
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    output: crate::output::AudioOutputMode,
    simple_track_candidates: &[String],
) {
    if director.mode != MusicDirectorMode::AdaptiveOutro {
        return;
    }
    let duration = director
        .current_section_id
        .as_deref()
        .and_then(|id| cue.section(id))
        .map(|section| section.duration_seconds(cue))
        .unwrap_or(0.0);
    if !director.default_resume_started
        && director.seconds_in_mode >= (duration - DEFAULT_RETURN_OVERLAP_SECONDS).max(0.0)
    {
        // Overlap: start the base lofi track UNDER the still-tailing
        // adaptive outro. Mode stays AdaptiveOutro until the outro
        // duration completes (block below); only then do we
        // transition to AdaptiveFinished + clear the adaptive cue
        // identity. Setting `mode = SimpleTrack` here would break
        // the same-cue restart invariant — see the
        // `resume_simple_music` doc.
        resume_simple_music(
            director,
            library,
            asset_server,
            music_state,
            base_music_channel,
            output,
            simple_track_candidates,
            false,
        );
        director.default_resume_started = true;
    }
    if director.seconds_in_mode >= duration {
        info!(
            target: MUSIC_LOG_TARGET,
            "finish_adaptive_outro cue={} t={:.3}",
            cue.id,
            director.seconds_in_mode,
        );
        director.mode = MusicDirectorMode::AdaptiveFinished;
        director.active_cue_id = None;
        director.current_state_id = None;
        director.current_section_id = None;
        channels.stop_all(900);
        zero_all_targets(director);
    }
}

pub(super) fn shutdown_adaptive_cue(
    director: &mut MusicDirectorState,
    channels: &MusicLayerChannels,
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    output: crate::output::AudioOutputMode,
    simple_track_candidates: &[String],
) {
    info!(
        target: MUSIC_LOG_TARGET,
        "shutdown_adaptive_cue cue={:?} mode={:?} state={:?} section={:?}",
        director.active_cue_id,
        director.mode,
        director.current_state_id,
        director.current_section_id,
    );
    channels.stop_all(650);
    director.active_cue_id = None;
    director.current_state_id = None;
    director.current_section_id = None;
    director.mode = MusicDirectorMode::Idle;
    director.pending_state = None;
    zero_all_current_and_targets(director);
    // Adaptive cue identity is fully cleared here, so it's safe for
    // resume_simple_music to flip the mode to SimpleTrack.
    resume_simple_music(
        director,
        library,
        asset_server,
        music_state,
        base_music_channel,
        output,
        simple_track_candidates,
        true,
    );
}

fn is_outro_target(cue: &MusicCueSpec, state: &MusicStateSpec) -> bool {
    cue.outro_state.as_deref() == Some(state.id.as_str())
}
