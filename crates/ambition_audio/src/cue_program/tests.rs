use super::*;

fn sections(cue: &mut MusicCueSpec) -> &mut Vec<MusicSectionSpec> {
    let MusicCueMedia::Inline { sections, .. } = &mut cue.media else {
        panic!("inline fixture")
    };
    sections
}
fn bpm(cue: &mut MusicCueSpec) -> &mut f32 {
    let MusicCueMedia::Inline { bpm, .. } = &mut cue.media else {
        panic!("inline fixture")
    };
    bpm
}
fn meter(cue: &mut MusicCueSpec) -> &mut f32 {
    let MusicCueMedia::Inline { beats_per_bar, .. } = &mut cue.media else {
        panic!("inline fixture")
    };
    beats_per_bar
}

fn authored() -> AuthoredMusicCues {
    AuthoredMusicCues {
        cues: vec![MusicCueSpec {
            id: "cue".into(),
            playback: Default::default(),
            media: MusicCueMedia::Inline {
                asset_root: "content://library/music".into(),
                bpm: 120.0,
                beats_per_bar: 4.0,
                sections: vec![MusicSectionSpec {
                    id: "loop".into(),
                    duration_beats: 8.0,
                    looped: true,
                    sources: vec![MusicLayerSourceSpec {
                        layer_id: "bed".into(),
                        path: "bed.ogg".into(),
                    }],
                }],
            },
            relative_volume: 0.5,
            layers: vec![
                MusicLayerSpec {
                    id: "bed".into(),
                    slot: 2,
                },
                MusicLayerSpec {
                    id: "accent".into(),
                    slot: 5,
                },
            ],
            states: vec![MusicStateSpec {
                id: "song".into(),
                section_id: "loop".into(),
                next_state: None,
                gains: vec![MusicLayerGainSpec {
                    layer_id: "bed".into(),
                    gain: 0.8,
                }],
            }],
            outro_state: None,
            post_clear_bridge_state: None,
            runtime_balance_overrides: vec![],
        }],
        room_bindings: vec![RoomMusicBinding {
            room_id: "room".into(),
            cue_id: "cue".into(),
            state: "song".into(),
            entry_state: None,
            fight_state: None,
        }],
        ..Default::default()
    }
}

#[test]
fn preparation_resolves_paths_slots_and_total_gains() {
    let mut source = authored();
    source.cues[0]
        .runtime_balance_overrides
        .push(MusicStateBalanceOverride {
            state_id: "song".into(),
            layer_gains: vec![("bed".into(), 0.6)],
        });
    let prepared =
        PreparedMusicCues::prepare(source, &crate::music_media::MusicMediaCatalog::default())
            .unwrap();
    let cue = prepared.cue("cue").unwrap();
    let section = cue.section("loop").unwrap();
    assert_eq!(section.musical_duration_seconds, Some(4.0));
    assert_eq!(section.file_duration_seconds, None);
    assert_eq!(section.sources[0].slot, 2);
    assert_eq!(section.sources[0].path, "content://library/music/bed.ogg");
    assert_eq!(
        cue.state("song").unwrap().gains,
        [0.0, 0.0, 0.3, 0.0, 0.0, 0.0]
    );
    assert_eq!(prepared.binding_for_room("room").unwrap().cue_id, cue.id);

    let mut source = authored();
    sections(&mut source.cues[0])[0].sources[0].path = "game://music/bed.ogg".into();
    let prepared =
        PreparedMusicCues::prepare(source, &crate::music_media::MusicMediaCatalog::default())
            .unwrap();
    assert_eq!(
        prepared.cue("cue").unwrap().sections[0].sources[0].path,
        "game://music/bed.ogg"
    );
}

#[test]
fn a_mastered_full_mix_keeps_its_state_gain() {
    let mut source = authored();
    let cue = &mut source.cues[0];
    cue.layers = vec![MusicLayerSpec {
        id: "full".into(),
        slot: 3,
    }];
    sections(cue)[0].sources[0].layer_id = "full".into();
    cue.states[0].gains[0].layer_id = "full".into();
    cue.runtime_balance_overrides = vec![MusicStateBalanceOverride {
        state_id: "song".into(),
        layer_gains: vec![("full".into(), 10.0)],
    }];
    let prepared =
        PreparedMusicCues::prepare(source, &crate::music_media::MusicMediaCatalog::default())
            .unwrap();
    assert_eq!(prepared.cue("cue").unwrap().states[0].gains[3], 0.4);
}

#[test]
fn duplicate_definitions_cannot_alias_a_playback_slot_or_hide_an_invalid_cue() {
    let edits: &[fn(&mut AuthoredMusicCues)] = &[
        |source| {
            let mut duplicate = source.cues[0].clone();
            *bpm(&mut duplicate) = 0.0;
            source.cues.insert(0, duplicate);
        },
        |source| {
            let duplicate = sections(&mut source.cues[0])[0].clone();
            sections(&mut source.cues[0]).push(duplicate);
        },
        |source| {
            let duplicate = source.cues[0].states[0].clone();
            source.cues[0].states.push(duplicate);
        },
        |source| {
            let duplicate = source.cues[0].layers[0].clone();
            source.cues[0].layers.push(duplicate);
        },
        |source| source.cues[0].layers[1].slot = 2,
        |source| source.cues[0].layers[1].slot = MAX_MUSIC_LAYERS,
        |source| {
            let section = &mut sections(&mut source.cues[0])[0];
            section.sources.push(section.sources[0].clone());
        },
        |source| {
            let state = &mut source.cues[0].states[0];
            state.gains.push(state.gains[0].clone());
        },
        |source| source.room_bindings.push(source.room_bindings[0].clone()),
    ];
    for edit in edits {
        let mut source = authored();
        edit(&mut source);
        assert!(PreparedMusicCues::prepare(
            source,
            &crate::music_media::MusicMediaCatalog::default()
        )
        .is_err());
    }
}

#[test]
fn an_unexecutable_graph_is_refused_before_registration() {
    let edits: &[fn(&mut MusicCueSpec)] = &[
        |cue| *bpm(cue) = f32::NAN,
        |cue| *bpm(cue) = 0.0,
        |cue| *meter(cue) = f32::INFINITY,
        |cue| cue.relative_volume = -1.0,
        |cue| {
            cue.relative_volume = f32::MAX;
            cue.states[0].gains[0].gain = f32::MAX;
        },
        |cue| sections(cue)[0].duration_beats = f32::INFINITY,
        |cue| sections(cue)[0].duration_beats = 0.0,
        |cue| sections(cue)[0].sources.clear(),
        |cue| cue.states.clear(),
        |cue| sections(cue)[0].sources[0].path.clear(),
        |cue| cue.states[0].gains[0].gain = f32::NAN,
        |cue| {
            cue.states[0].gains.push(MusicLayerGainSpec {
                layer_id: "accent".into(),
                gain: 0.2,
            })
        },
        |cue| cue.states[0].next_state = Some("song".into()),
        |cue| {
            cue.runtime_balance_overrides
                .push(MusicStateBalanceOverride {
                    state_id: "missing".into(),
                    layer_gains: vec![],
                })
        },
        |cue| {
            cue.runtime_balance_overrides
                .push(MusicStateBalanceOverride {
                    state_id: "song".into(),
                    layer_gains: vec![("missing".into(), 1.0)],
                })
        },
        |cue| {
            cue.runtime_balance_overrides
                .push(MusicStateBalanceOverride {
                    state_id: "song".into(),
                    layer_gains: vec![("bed".into(), -1.0)],
                })
        },
    ];
    for edit in edits {
        let mut source = authored();
        edit(&mut source.cues[0]);
        assert!(PreparedMusicCues::prepare(
            source,
            &crate::music_media::MusicMediaCatalog::default()
        )
        .is_err());
    }
}

#[test]
fn day_phase_and_boss_phase_rows_have_one_meaning() {
    let mut source = authored();
    source.boss_bindings = vec![BossMusicBinding {
        boss_id: "boss".into(),
        cue_id: "cue".into(),
        phase_states: vec![BossPhaseState {
            policy: Default::default(),
            phase: "Phase1".into(),
            state: "song".into(),
        }],
    }];
    source.room_phase_tracks = vec![RoomPhaseTrack {
        room_id: "room".into(),
        phase: "night".into(),
        track: "quiet".into(),
    }];
    let prepared = PreparedMusicCues::prepare(
        source.clone(),
        &crate::music_media::MusicMediaCatalog::default(),
    )
    .unwrap();
    assert_eq!(prepared.room_phase_track("room", "night"), Some("quiet"));
    assert_eq!(
        prepared
            .binding_for_boss("boss")
            .unwrap()
            .state_in("Phase1"),
        Some("song")
    );
    let mut duplicate = source.clone();
    duplicate
        .room_phase_tracks
        .push(source.room_phase_tracks[0].clone());
    assert!(PreparedMusicCues::prepare(
        duplicate,
        &crate::music_media::MusicMediaCatalog::default()
    )
    .is_err());
    let duplicate = source.boss_bindings[0].phase_states[0].clone();
    source.boss_bindings[0].phase_states.push(duplicate);
    assert!(
        PreparedMusicCues::prepare(source, &crate::music_media::MusicMediaCatalog::default())
            .is_err()
    );
}

#[test]
fn whole_asset_recipes_need_no_score_clock_or_copied_duration() {
    let mut source = authored();
    source.cues[0].media = MusicCueMedia::Assets {
        sections: vec![MusicAssetSectionSpec {
            id: "loop".into(),
            looped: false,
            sources: vec![MusicLayerSourceSpec {
                layer_id: "bed".into(),
                path: "audio/one-shot.ogg".into(),
            }],
        }],
    };
    let prepared = PreparedMusicCues::prepare(source.clone(), &Default::default()).unwrap();
    let section = &prepared.cue("cue").unwrap().sections[0];
    assert_eq!(section.musical_duration_seconds, None);
    assert_eq!(section.clock, PreparedMusicClock::Unclocked);
    assert_eq!(
        section.delay_to_exit(
            &crate::music_policy::MusicExitRule::AssetEnd,
            1.0,
            Some(3.0)
        ),
        Some(2.0)
    );
    let inspection = prepared.inspect();
    assert_eq!(inspection["cues"][0]["sections"][0]["end"], "AssetEnd");
    assert_eq!(inspection["cues"][0]["sections"][0]["required_voices"], 1);
    source.cues[0].playback = crate::music_policy::MusicPlaybackPolicy::default().into();
    assert!(
        PreparedMusicCues::prepare(source, &Default::default())
            .unwrap_err()
            .iter()
            .any(|error| error.contains("section_exit"))
    );
}

#[test]
fn preparation_rejects_unsupported_timing_exits_and_invalid_transition_values() {
    use crate::music_policy::*;
    let policies = [
        MusicPlaybackPolicy {
            timing: MusicTimingRequirement::SampleAccurate,
            ..Default::default()
        },
        MusicPlaybackPolicy {
            timing: MusicTimingRequirement::PhasePreserving,
            ..Default::default()
        },
        MusicPlaybackPolicy {
            section_exit: MusicExitRule::Bars(0),
            ..Default::default()
        },
        MusicPlaybackPolicy {
            section_exit: MusicExitRule::Markers(vec!["missing".into()]),
            ..Default::default()
        },
        MusicPlaybackPolicy {
            return_overlap_seconds: f32::NAN,
            ..Default::default()
        },
        MusicPlaybackPolicy {
            ending_media_timeout_seconds: -1.0,
            ..Default::default()
        },
    ];
    for policy in policies {
        let mut source = authored();
        source.cues[0].playback = policy.into();
        assert!(PreparedMusicCues::prepare(source, &Default::default()).is_err());
    }
}

#[test]
fn a_partial_policy_keeps_the_whole_asset_recipes_exit_defaults() {
    let mut source = authored();
    source.cues[0].media = MusicCueMedia::Assets {
        sections: vec![MusicAssetSectionSpec {
            id: "loop".into(),
            looped: true,
            sources: vec![MusicLayerSourceSpec {
                layer_id: "bed".into(),
                path: "audio/loop.ogg".into(),
            }],
        }],
    };
    source.cues[0].playback.section_crossfade_seconds = Some(0.4);
    let program = PreparedMusicCues::prepare(source, &Default::default()).unwrap();
    let policy = &program.cue("cue").unwrap().playback;
    assert_eq!(policy.section_crossfade_seconds, 0.4);
    assert_eq!(
        policy.section_exit,
        crate::music_policy::MusicExitRule::Immediate
    );
    assert_eq!(
        policy.ending_exit,
        crate::music_policy::MusicExitRule::Immediate
    );
}
