use super::*;

/// Load file-backed cue sources and install the generic cue catalog.
pub fn load_music_cues(mut commands: Commands, asset_server: Res<AssetServer>) {
    let catalog = MusicCueCatalog::builtin();
    let mut sources = HashMap::new();
    for cue in catalog.cues.values() {
        for section in &cue.sections {
            for source in &section.sources {
                let rel = format!("{}/{}", cue.asset_root.trim_end_matches('/'), source.path);
                sources.insert(
                    MusicSourceKey::new(&cue.id, &section.id, &source.layer_id),
                    asset_server.load(rel),
                );
            }
        }
        info!(
            target: MUSIC_LOG_TARGET,
            "loaded music cue id={} sections={} layers={}",
            cue.id,
            cue.sections.len(),
            cue.layers.len(),
        );
    }

    commands.insert_resource(catalog);
    commands.insert_resource(LoadedMusicCueAssets { sources });
    commands.insert_resource(MusicDirectorState::default());
}

/// Unified music director.
///
/// Handles both simple track selection and adaptive cue state transitions. The
/// simple track backend still reuses the existing `AudioLibrary` / `MusicChannel`
/// sources; adaptive cues use the generic layer-bank scheduler below.
pub fn drive_music_director(
    time: Res<Time>,
    catalog: Option<Res<MusicCueCatalog>>,
    assets: Option<Res<LoadedMusicCueAssets>>,
    director: Option<ResMut<MusicDirectorState>>,
    encounters: Res<EncounterRegistry>,
    mut encounter_music: ResMut<EncounterMusicRequest>,
    room_music: Res<RoomMusicRequest>,
    layer_channels: MusicLayerChannels,
    base_music_channel: Res<AudioChannel<MusicChannel>>,
    library: Res<AudioLibrary>,
    mut music_state: ResMut<MusicPlaybackState>,
    radio: Option<Res<RadioStationState>>,
    sandbox_data: Res<SandboxDataSpec>,
    settings: Res<UserSettings>,
) {
    let Some(catalog) = catalog else {
        return;
    };
    let Some(assets) = assets else {
        return;
    };
    let Some(mut director) = director else {
        return;
    };

    let dt = time.delta_secs();
    director.seconds_in_mode += dt;
    if director.mode == MusicDirectorMode::AdaptiveLoop {
        director.seconds_in_loop += dt;
    }

    let adaptive = resolve_adaptive_directive(&catalog, &encounters, &director);
    match adaptive {
        Some(AdaptiveCueDirective::Play { cue_id, state_id }) => {
            if let (Some(cue), Some(target_state)) = (
                catalog.cue(&cue_id),
                catalog.cue(&cue_id).and_then(|cue| cue.state(&state_id)),
            ) {
                drive_adaptive_cue_state(
                    &mut director,
                    cue,
                    target_state,
                    &assets,
                    &layer_channels,
                    &base_music_channel,
                    &settings,
                    dt,
                );
            } else {
                warn!(
                    target: MUSIC_LOG_TARGET,
                    "adaptive directive references missing cue/state cue={} state={}",
                    cue_id,
                    state_id,
                );
            }
        }
        Some(AdaptiveCueDirective::StopNow) => {
            if director.active_cue_id.is_some() {
                shutdown_adaptive_cue(
                    &mut director,
                    &layer_channels,
                    &library,
                    &mut music_state,
                    &base_music_channel,
                    &room_music,
                    radio.as_deref(),
                    &sandbox_data,
                    &mut encounter_music,
                );
            }
        }
        None => {
            if director.active_cue_id.is_some()
                && director.mode != MusicDirectorMode::AdaptiveFinished
                && director.mode != MusicDirectorMode::Idle
            {
                // Leaving the room or losing the cue owner without a clear should
                // not leave the adaptive channels running.
                shutdown_adaptive_cue(
                    &mut director,
                    &layer_channels,
                    &library,
                    &mut music_state,
                    &base_music_channel,
                    &room_music,
                    radio.as_deref(),
                    &sandbox_data,
                    &mut encounter_music,
                );
            } else {
                apply_simple_music_intent(
                    &mut director,
                    &library,
                    &mut music_state,
                    &base_music_channel,
                    &room_music,
                    radio.as_deref(),
                    &sandbox_data,
                    &mut encounter_music,
                );
            }
        }
    }

    if let Some(cue_id) = director.active_cue_id.clone() {
        if let Some(cue) = catalog.cue(&cue_id) {
            update_gain_smoothing(&mut director, &layer_channels, dt);
            drive_outro_tail(
                &mut director,
                cue,
                &layer_channels,
                &library,
                &mut music_state,
                &base_music_channel,
                &room_music,
                radio.as_deref(),
                &sandbox_data,
                &mut encounter_music,
            );
            log_periodic_state(&mut director, cue, dt);
        }
    }
}

pub(super) fn resolve_adaptive_directive(
    catalog: &MusicCueCatalog,
    encounters: &EncounterRegistry,
    director: &MusicDirectorState,
) -> Option<AdaptiveCueDirective> {
    // First binding with a live directive wins. Iterating the catalog
    // (rather than hardcoding mob_lab) lets future encounter cues
    // (boss / mini-boss) drop in by adding a binding without touching
    // this resolver.
    for binding in &catalog.encounter_bindings {
        if let Some(directive) = resolve_directive_for_binding(binding, encounters, director) {
            return Some(directive);
        }
    }
    None
}

/// Resolve a single encounter binding's adaptive cue directive.
/// Returns:
/// - `Some(StopNow)` when the encounter no longer exists but the
///   binding's cue is still active.
/// - `Some(Play { cue_id, state_id })` when the encounter is in a
///   phase mapped to a cue state.
/// - `None` when the encounter is unknown / inactive AND its cue is
///   not playing — the binding doesn't claim audio this frame.
pub(super) fn resolve_directive_for_binding(
    binding: &EncounterMusicBinding,
    encounters: &EncounterRegistry,
    director: &MusicDirectorState,
) -> Option<AdaptiveCueDirective> {
    let cue_active = director.active_cue_id.as_deref() == Some(binding.cue_id.as_str());
    let Some(encounter) = encounters.get(&binding.encounter_id) else {
        if cue_active {
            return Some(AdaptiveCueDirective::StopNow);
        }
        return None;
    };

    match encounter.phase {
        EncounterPhase::Starting { .. } => Some(AdaptiveCueDirective::Play {
            cue_id: binding.cue_id.clone(),
            state_id: binding.starting_state.clone(),
        }),
        EncounterPhase::Active { wave_index, .. } => {
            let mut state_id = binding
                .wave_states
                .get(wave_index)
                .or_else(|| binding.wave_states.last())
                .cloned()
                .unwrap_or_else(|| binding.starting_state.clone());
            if wave_index == 1 && encounter.run.wave_elapsed >= LARGE_BRUTE_DELAY_SECONDS {
                if let Some(reinforced) = &binding.wave2_reinforced_state {
                    state_id = reinforced.clone();
                }
            }
            Some(AdaptiveCueDirective::Play {
                cue_id: binding.cue_id.clone(),
                state_id,
            })
        }
        EncounterPhase::Cleared => Some(AdaptiveCueDirective::Play {
            cue_id: binding.cue_id.clone(),
            state_id: binding.cleared_state.clone(),
        }),
        EncounterPhase::Inactive => {
            // The encounter often resets to Inactive immediately after clear;
            // if this adaptive cue is already active, continue into its outro
            // instead of hard-cutting to room music.
            if cue_active
                && director.mode != MusicDirectorMode::AdaptiveFinished
                && director.mode != MusicDirectorMode::Idle
            {
                Some(AdaptiveCueDirective::Play {
                    cue_id: binding.cue_id.clone(),
                    state_id: binding.cleared_state.clone(),
                })
            } else {
                None
            }
        }
        EncounterPhase::Failed => Some(AdaptiveCueDirective::StopNow),
    }
}

fn apply_simple_music_intent(
    director: &mut MusicDirectorState,
    library: &AudioLibrary,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    room_music: &RoomMusicRequest,
    radio: Option<&RadioStationState>,
    sandbox_data: &SandboxDataSpec,
    encounter_music: &mut EncounterMusicRequest,
) {
    let target = resolved_simple_track(library, room_music, radio, sandbox_data, encounter_music);
    let needs_switch = director.last_simple_track.as_deref() != Some(target.as_str())
        || music_state.active_track != target;
    if needs_switch && library.track(&target).is_some() {
        info!(target: MUSIC_LOG_TARGET, "simple_music target={}", target);
        switch_to_music_track(library, music_state, base_music_channel, &target);
        director.last_simple_track = Some(target.clone());
        director.mode = MusicDirectorMode::SimpleTrack;
    }
    encounter_music.last_applied = Some(target);
}

fn resolved_simple_track(
    library: &AudioLibrary,
    room_music: &RoomMusicRequest,
    radio: Option<&RadioStationState>,
    sandbox_data: &SandboxDataSpec,
    encounter_music: &EncounterMusicRequest,
) -> String {
    if let Some(track) = &encounter_music.desired_track {
        if library.track(track).is_some() {
            return track.clone();
        }
    }
    if let Some(track) = radio.and_then(|radio| radio.selected_track()) {
        if library.track(track).is_some() {
            return track.to_string();
        }
    }
    room_music
        .desired_track
        .as_ref()
        .filter(|track| library.track(track).is_some())
        .cloned()
        .unwrap_or_else(|| sandbox_data.audio.default_music_track.clone())
}

fn drive_adaptive_cue_state(
    director: &mut MusicDirectorState,
    cue: &MusicCueSpec,
    target_state: &MusicStateSpec,
    assets: &LoadedMusicCueAssets,
    channels: &MusicLayerChannels,
    base_music_channel: &AudioChannel<MusicChannel>,
    settings: &UserSettings,
    dt: f32,
) {
    if director.active_cue_id.as_deref() != Some(cue.id.as_str()) {
        base_music_channel.stop().fade_out(AudioTween::new(
            Duration::from_millis(650),
            AudioEasing::OutPowi(2),
        ));
        start_adaptive_state(
            director,
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
        queue_or_fire_outro(director, cue, target_state, assets, channels, settings, dt);
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
    cue: &MusicCueSpec,
    target_state: &MusicStateSpec,
    assets: &LoadedMusicCueAssets,
    channels: &MusicLayerChannels,
    settings: &UserSettings,
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
    cue: &MusicCueSpec,
    target_state: &MusicStateSpec,
    assets: &LoadedMusicCueAssets,
    channels: &MusicLayerChannels,
    settings: &UserSettings,
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

    let old_bank = director.active_bank;
    let new_bank = if director.active_cue_id.is_some() {
        old_bank.other()
    } else {
        MusicBank::A
    };

    info!(
        target: MUSIC_LOG_TARGET,
        "start_adaptive_state cue={} state={} section={} old_bank={} new_bank={} looped={} crossfade={:.2}s gains={}",
        cue.id,
        target_state.id,
        section.id,
        old_bank.label(),
        new_bank.label(),
        section.looped,
        crossfade_seconds,
        format_gains(gains_for_state(cue, target_state, settings)),
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
        if let Some(handle) = assets.get(&cue.id, &section.id, &source.layer_id) {
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

    set_bank_targets(
        director,
        new_bank,
        gains_for_state(cue, target_state, settings),
    );
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

fn drive_outro_tail(
    director: &mut MusicDirectorState,
    cue: &MusicCueSpec,
    channels: &MusicLayerChannels,
    library: &AudioLibrary,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    room_music: &RoomMusicRequest,
    radio: Option<&RadioStationState>,
    sandbox_data: &SandboxDataSpec,
    encounter_music: &mut EncounterMusicRequest,
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
        resume_simple_music(
            director,
            library,
            music_state,
            base_music_channel,
            room_music,
            radio,
            sandbox_data,
            encounter_music,
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

fn shutdown_adaptive_cue(
    director: &mut MusicDirectorState,
    channels: &MusicLayerChannels,
    library: &AudioLibrary,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    room_music: &RoomMusicRequest,
    radio: Option<&RadioStationState>,
    sandbox_data: &SandboxDataSpec,
    encounter_music: &mut EncounterMusicRequest,
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
    resume_simple_music(
        director,
        library,
        music_state,
        base_music_channel,
        room_music,
        radio,
        sandbox_data,
        encounter_music,
    );
}

fn resume_simple_music(
    director: &mut MusicDirectorState,
    library: &AudioLibrary,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    room_music: &RoomMusicRequest,
    radio: Option<&RadioStationState>,
    sandbox_data: &SandboxDataSpec,
    encounter_music: &mut EncounterMusicRequest,
) {
    let target = resolved_simple_track(library, room_music, radio, sandbox_data, encounter_music);
    if library.track(&target).is_some() {
        info!(target: MUSIC_LOG_TARGET, "resume_simple_music target={}", target);
        switch_to_music_track(library, music_state, base_music_channel, &target);
        director.last_simple_track = Some(target.clone());
        encounter_music.last_applied = Some(target);
        director.mode = MusicDirectorMode::SimpleTrack;
    }
}

fn is_outro_target(cue: &MusicCueSpec, state: &MusicStateSpec) -> bool {
    cue.outro_state.as_deref() == Some(state.id.as_str())
}

fn apply_first_goblin_runtime_balance_overrides(
    cue: &MusicCueSpec,
    state: &MusicStateSpec,
    gains: &mut LayerGains,
    master: f32,
) {
    if cue.id != FIRST_GOBLIN_CUE_ID {
        return;
    }

    let overrides: &[(&str, f32)] = match state.id.as_str() {
        // Intro: the cue authors `full = 1.0` against the pre-rendered
        // intro mix. The newest goblin v2 stems are mastered hotter
        // than the wave1 stems-mix, which made the encounter open
        // loud relative to wave1/wave2. Drop intro's full layer to
        // 0.40 so the perceived loudness lines up with the
        // post-intro stem blend. (Fine-tuning will need a real audio
        // mastering pass; this is the conservative single-knob fix.)
        "intro" => &[("full", 0.40)],
        // Outro: same logic as intro.
        "outro" => &[("full", 0.40)],
        // Cleared bridge: keep dialed-back, the encounter is over.
        "cleared_bridge" => &[
            ("strings", 0.40),
            ("winds", 0.36),
            ("mallets", 0.10),
            ("percussion", 0.06),
            ("brass", 0.10),
            ("choir_pad", 0.04),
        ],
        "wave1" => &[
            ("strings", 0.95),
            ("winds", 1.00),
            ("mallets", 0.18),
            ("percussion", 0.08),
            ("brass", 0.00),
            ("choir_pad", 0.00),
        ],
        // wave2: drums COME IN (intentional entrance). Percussion at
        // 0.42 gives a clear "rhythm has joined the mix" feel without
        // overpowering the strings/brass. wave1 (0.08) is the
        // pre-percussion baseline; wave2 is the first beat-driven
        // beat-of-the-encounter.
        "wave2" => &[
            ("strings", 0.95),
            ("winds", 1.00),
            ("mallets", 0.14),
            ("percussion", 0.42),
            ("brass", 0.44),
            ("choir_pad", 0.04),
        ],
        // wave2_brute: heavier mob variant. Drums + brass push beyond
        // standard wave2 to match the "brute encounter" intensity.
        "wave2_brute" => &[
            ("strings", 0.95),
            ("winds", 1.00),
            ("mallets", 0.12),
            ("percussion", 0.50),
            ("brass", 0.58),
            ("choir_pad", 0.06),
        ],
        // wave3: drums KICK UP FURTHER. Bumped from the prior 0.42 to
        // 0.58 so the wave3 transition is unambiguously a step up from
        // wave2's 0.42 -- enough headroom that the audio file change
        // at the section crossfade reads as "drums got louder + the
        // rhythm shifted" rather than "the track switched".
        "wave3" => &[
            ("strings", 0.90),
            ("winds", 1.00),
            ("mallets", 0.08),
            ("percussion", 0.58),
            ("brass", 0.62),
            ("choir_pad", 0.06),
        ],
        "recap_loop" => &[
            ("strings", 0.90),
            ("winds", 0.90),
            ("mallets", 0.10),
            ("percussion", 0.12),
            ("brass", 0.16),
            ("choir_pad", 0.02),
        ],
        _ => return,
    };

    for (layer_id, gain) in overrides {
        if let Some(layer) = cue.layer(layer_id) {
            let slot = layer.slot.min(MAX_LAYERS - 1);
            gains[slot] = gain.max(0.0) * master;
        }
    }
}

fn gains_for_state(
    cue: &MusicCueSpec,
    state: &MusicStateSpec,
    settings: &UserSettings,
) -> LayerGains {
    let mut gains = [0.0; MAX_LAYERS];
    let master = settings.audio.effective_music() * cue.relative_volume;
    for layer_gain in &state.gains {
        if let Some(layer) = cue.layer(&layer_gain.layer_id) {
            let slot = layer.slot.min(MAX_LAYERS - 1);
            gains[slot] = layer_gain.gain.max(0.0) * master;
        }
    }
    apply_first_goblin_runtime_balance_overrides(cue, state, &mut gains, master);
    gains
}

fn set_bank_targets(director: &mut MusicDirectorState, bank: MusicBank, gains: LayerGains) {
    director.target_gains[bank.index()] = gains;
}

fn zero_all_targets(director: &mut MusicDirectorState) {
    director.target_gains = [[0.0; MAX_LAYERS]; 2];
}

fn zero_all_current_and_targets(director: &mut MusicDirectorState) {
    director.current_gains = [[0.0; MAX_LAYERS]; 2];
    director.target_gains = [[0.0; MAX_LAYERS]; 2];
}

fn update_gain_smoothing(
    director: &mut MusicDirectorState,
    channels: &MusicLayerChannels,
    dt: f32,
) {
    let alpha = if STEM_GAIN_BLEND_SECONDS <= 0.0 {
        1.0
    } else {
        1.0 - (-dt / STEM_GAIN_BLEND_SECONDS).exp()
    };
    for bank in [MusicBank::A, MusicBank::B] {
        for slot in 0..MAX_LAYERS {
            let current = director.current_gains[bank.index()][slot];
            let target = director.target_gains[bank.index()][slot];
            let next = current + (target - current) * alpha;
            director.current_gains[bank.index()][slot] =
                if next.abs() < 0.0005 { 0.0 } else { next };
            channels.set_layer_volume(bank, slot, director.current_gains[bank.index()][slot]);
        }
    }

    if let Some(fading_bank) = director.fading_bank {
        director.fade_stop_seconds -= dt;
        if director.fade_stop_seconds <= 0.0 {
            channels.stop_bank(fading_bank, 120);
            director.current_gains[fading_bank.index()] = [0.0; MAX_LAYERS];
            director.target_gains[fading_bank.index()] = [0.0; MAX_LAYERS];
            director.fading_bank = None;
        }
    }
}

fn seconds_until_next_bar(cue: &MusicCueSpec, seconds_in_loop: f32) -> f32 {
    let bar = cue.seconds_per_bar().max(0.001);
    let rem = seconds_in_loop.rem_euclid(bar);
    if rem <= 0.001 {
        0.0
    } else {
        bar - rem
    }
}

fn seconds_until_next_phrase_marker(
    cue: &MusicCueSpec,
    seconds_in_loop: f32,
    bars_per_phrase: f32,
) -> f32 {
    let phrase = (cue.seconds_per_bar() * bars_per_phrase.max(1.0)).max(0.001);
    let rem = seconds_in_loop.rem_euclid(phrase);
    if rem <= 0.001 {
        0.0
    } else {
        phrase - rem
    }
}

fn log_periodic_state(director: &mut MusicDirectorState, cue: &MusicCueSpec, dt: f32) {
    director.debug_log_timer -= dt;
    if director.debug_log_timer > 0.0 {
        return;
    }
    director.debug_log_timer = DEBUG_LOG_PERIOD_SECONDS;
    debug!(
        target: MUSIC_LOG_TARGET,
        "music_director mode={:?} cue={:?} state={:?} section={:?} t_mode={:.3} t_loop={:.3} bar_beat={} active_bank={} gains_a={} gains_b={}",
        director.mode,
        director.active_cue_id,
        director.current_state_id,
        director.current_section_id,
        director.seconds_in_mode,
        director.seconds_in_loop,
        format_bar_beat(cue, director.seconds_in_loop),
        director.active_bank.label(),
        format_gains(director.current_gains[MusicBank::A.index()]),
        format_gains(director.current_gains[MusicBank::B.index()]),
    );
}

fn format_bar_beat(cue: &MusicCueSpec, seconds: f32) -> String {
    let beat = seconds / cue.seconds_per_beat();
    let beats_per_bar = cue.beats_per_bar.max(1.0);
    let bar = (beat / beats_per_bar).floor() as i32 + 1;
    let beat_in_bar = beat.rem_euclid(beats_per_bar) + 1.0;
    format!("{}.{}", bar, beat_in_bar.floor() as i32)
}

fn format_gains(gains: LayerGains) -> String {
    gains
        .iter()
        .map(|g| format!("{g:.2}"))
        .collect::<Vec<_>>()
        .join(",")
}
