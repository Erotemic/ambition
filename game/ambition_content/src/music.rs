//! Ambition's authored music-cue catalog + encounter bindings.
//!
//! Named content data: the adaptive cues that ship and the encounters they
//! bind to (e.g. the goblin-lab tune). The reusable music director in
//! `ambition_audio` plays whatever [`MusicCueCatalog`] the host inserts;
//! this module is the one Ambition installs. Gated behind the `audio`
//! feature.

use ambition_audio::music::{
    EncounterMusicBinding, MusicCueCatalog, MusicCueSpec, MusicLayerGainSpec, MusicLayerSourceSpec,
    MusicLayerSpec, MusicSectionSpec, MusicStateSpec,
};

/// Sandbox encounter id the goblin cue binds to.
pub const MOB_LAB_ENCOUNTER_ID: &str = "goblin_encounter";
/// Cue id for the generated first-goblin adaptive tune.
pub const FIRST_GOBLIN_CUE_ID: &str = "first_goblin_tune_v2";

/// Relative volume for adaptive cues after user music volume. Stacked
/// layers sum hotter than the single-channel room tracks, so keep the
/// per-cue default conservative.
const ADAPTIVE_MUSIC_RELATIVE_VOLUME: f32 = 1.0;

/// Ambition's authored music-cue catalog: the cues that ship and the
/// encounters that bind to them. The reusable director plays whatever
/// catalog the host inserts; THIS is the sandbox's.
pub fn ambition_music_cue_catalog() -> MusicCueCatalog {
    MusicCueCatalog::from_parts(
        vec![first_goblin_tune_v2_spec()],
        vec![EncounterMusicBinding {
            encounter_id: MOB_LAB_ENCOUNTER_ID.to_string(),
            cue_id: FIRST_GOBLIN_CUE_ID.to_string(),
            starting_state: "intro".to_string(),
            wave_states: vec![
                "wave1".to_string(),
                "wave2".to_string(),
                "wave3".to_string(),
            ],
            wave2_reinforced_state: Some("wave2_brute".to_string()),
            cleared_state: "outro".to_string(),
        }],
    )
}

pub fn first_goblin_tune_v2_spec() -> MusicCueSpec {
    let asset_root = "audio/music/generated/first_goblin_tune_v2".to_string();
    let layers = vec![
        // The current generated goblin cue intentionally plays mastered
        // per-section full mixes rather than raw per-stem files. Keep the cue
        // layer vocabulary to the actual playable layer so old runtime stem
        // balance overrides cannot accidentally overwrite the `full` layer's
        // slot gain for wave sections.
        MusicLayerSpec {
            id: "full".into(),
            slot: 0,
        },
    ];

    // `stem_sources` (per-stem layer set) was used here before the
    // 2026-05-08 rebalance. Re-add it only after the renderer applies its
    // mastering chain to per-stem outputs and stems are individually audible.
    // Until then, section boundaries can be abrupt: the runtime music director
    // owns the crossfade/overlap, while generated section files should avoid
    // baking long fade-outs that would double-count the transition and create
    // a perceived dip before wave1.

    fn full_source(section: &str) -> Vec<MusicLayerSourceSpec> {
        vec![MusicLayerSourceSpec {
            layer_id: "full".into(),
            path: format!("adaptive/{section}/{section}.full.ogg"),
        }]
    }

    fn gains(items: &[(&str, f32)]) -> Vec<MusicLayerGainSpec> {
        items
            .iter()
            .map(|(layer, gain)| MusicLayerGainSpec {
                layer_id: (*layer).to_string(),
                gain: *gain,
            })
            .collect()
    }

    // Full-mix sections should arrive from the renderer at roughly matched
    // perceived loudness. Keep runtime gains near unity so the music director
    // is not acting as a fake mastering stage: large runtime boosts magnify
    // SoundFont/reverb/codec noise floors and make section boundaries obvious.
    // If a section needs +10 dB here, fix the YAML/generator and rerender.
    //
    // Cost: wave2_brute still degenerates to wave2 while the cue uses one
    // mastered full mix per section. Reintroduce stem state gains only after
    // the renderer masters per-stem outputs at usable levels.
    let wave_state_gain = 1.0;
    let bridge_state_gain = 0.85;
    MusicCueSpec {
        id: FIRST_GOBLIN_CUE_ID.to_string(),
        asset_root,
        bpm: 132.0,
        beats_per_bar: 4.0,
        relative_volume: ADAPTIVE_MUSIC_RELATIVE_VOLUME,
        // Single mastered `full` layer per section: the renderer owns
        // loudness, so no per-stem runtime balance table.
        runtime_balance_overrides: Vec::new(),
        layers,
        sections: vec![
            MusicSectionSpec {
                id: "intro".into(),
                duration_beats: 16.0,
                looped: false,
                sources: full_source("intro"),
            },
            MusicSectionSpec {
                id: "wave1".into(),
                duration_beats: 32.0,
                looped: true,
                sources: full_source("wave1"),
            },
            MusicSectionSpec {
                id: "wave2".into(),
                duration_beats: 32.0,
                looped: true,
                sources: full_source("wave2"),
            },
            MusicSectionSpec {
                id: "wave3".into(),
                duration_beats: 32.0,
                looped: true,
                sources: full_source("wave3"),
            },
            MusicSectionSpec {
                id: "recap_loop".into(),
                duration_beats: 32.0,
                looped: true,
                sources: full_source("recap_loop"),
            },
            MusicSectionSpec {
                id: "outro".into(),
                duration_beats: 16.0,
                looped: false,
                sources: full_source("outro"),
            },
        ],
        states: vec![
            MusicStateSpec {
                id: "intro".into(),
                section_id: "intro".into(),
                gains: gains(&[("full", 1.0)]),
            },
            MusicStateSpec {
                id: "wave1".into(),
                section_id: "wave1".into(),
                gains: gains(&[("full", wave_state_gain)]),
            },
            MusicStateSpec {
                id: "wave2".into(),
                section_id: "wave2".into(),
                gains: gains(&[("full", wave_state_gain)]),
            },
            MusicStateSpec {
                // wave2_brute degenerates to wave2 with the full-mix
                // approach -- keep the state so existing encounter
                // wiring (`wave2_reinforced_state`) still resolves.
                id: "wave2_brute".into(),
                section_id: "wave2".into(),
                gains: gains(&[("full", wave_state_gain)]),
            },
            MusicStateSpec {
                id: "wave3".into(),
                section_id: "wave3".into(),
                gains: gains(&[("full", wave_state_gain)]),
            },
            MusicStateSpec {
                id: "cleared_bridge".into(),
                section_id: "recap_loop".into(),
                gains: gains(&[("full", bridge_state_gain)]),
            },
            MusicStateSpec {
                id: "outro".into(),
                section_id: "outro".into(),
                gains: gains(&[("full", 1.0)]),
            },
        ],
        outro_state: Some("outro".into()),
        post_clear_bridge_state: Some("cleared_bridge".into()),
    }
}
