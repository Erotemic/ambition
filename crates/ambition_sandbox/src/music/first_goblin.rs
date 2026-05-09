use super::*;

pub(super) fn first_goblin_tune_v2_spec() -> MusicCueSpec {
    let asset_root = "audio/music/generated/first_goblin_tune_v2".to_string();
    let layers = vec![
        MusicLayerSpec {
            id: "strings".into(),
            slot: 0,
        },
        MusicLayerSpec {
            id: "brass".into(),
            slot: 1,
        },
        MusicLayerSpec {
            id: "winds".into(),
            slot: 2,
        },
        MusicLayerSpec {
            id: "choir_pad".into(),
            slot: 3,
        },
        MusicLayerSpec {
            id: "mallets".into(),
            slot: 4,
        },
        MusicLayerSpec {
            id: "percussion".into(),
            slot: 5,
        },
        // The full intro/outro renders are intentionally mapped to slot 0;
        // they are exclusive one-shot layers, not simultaneous stems.
        MusicLayerSpec {
            id: "full".into(),
            slot: 0,
        },
    ];

    // `stem_sources` (per-stem layer set) was used here before the
    // 2026-05-08 rebalance. Re-add it when the renderer applies its
    // mastering chain to per-stem outputs and stems are individually
    // audible.

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

    // 2026-05-08 rebalance: the renderer's mastering chain
    // (compressor / reverb / limiter) only runs on the per-section
    // full-mix file, not on individual stems. The raw stems for
    // wave1/2/3 measure -50 to -inf LUFS — three of the six stems
    // are essentially silent — while the per-section full mixes sit
    // around -35 LUFS. To keep the cue audible at intro-level
    // loudness without pushing distortion, wave sections now play
    // the mastered full mix as a single layer with a fixed gain
    // boost (~ +14 dB → -21 LUFS, close to the lofi tracks at
    // -24 LUFS). The intro / outro / recap_loop full mixes are
    // already mastered and ride the same path.
    //
    // Cost: the wave2_brute state can no longer differ from wave2
    // (both share the wave2 section's single source). Acceptable
    // tradeoff until the renderer learns to master stems too —
    // until then the per-stem gains were applied on near-silence
    // anyway, so wave2_brute was inaudibly different from wave2.
    let wave_state_gain = 5.0;
    let bridge_state_gain = 2.4;
    MusicCueSpec {
        id: FIRST_GOBLIN_CUE_ID.to_string(),
        asset_root,
        bpm: 132.0,
        beats_per_bar: 4.0,
        relative_volume: ADAPTIVE_MUSIC_RELATIVE_VOLUME,
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
                // approach — keep the state so existing encounter
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
