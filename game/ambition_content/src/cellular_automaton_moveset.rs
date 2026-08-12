//! **The Perfect Cellular Automaton's signature move**, authored as data.
//!
//! ⭐ "Cellular Pulse" was the first real moveset consumer in the repository and
//! it lived on an ENEMY ARCHETYPE ROW — `character_archetypes.ron`'s
//! `signature_move` field, which existed for exactly one creature. It says the
//! same thing here that it said there: a 0.40s telegraph, a 0.14s active window
//! with one forward volume, then recovery, on the owner's proper-time clock,
//! triggered by the `special` verb through the shared moveset runtime.
//!
//! ⚠ **the numbers are the row's verbatim.** A migration that retuned on the way
//! would be a retune wearing a migration's commit.

use ambition_platformer2d::entity_catalog::{
    ClipBinding, HitVolume, MoveEvent, MoveEventKind, MoveSpec, MoveWindow, MovesetContract,
    VolumeShape, WindowTag,
};

/// See the module doc. One move, one verb.
pub fn cellular_pulse_moveset() -> MovesetContract {
    let window = |start_s: f32, end_s: f32, tag: WindowTag, volumes: Vec<HitVolume>| MoveWindow {
        start_s,
        end_s,
        tag,
        volumes,
        motion_scale: 1.0,
        sustain_effect: None,
    };
    MovesetContract {
        verbs: [("special".to_string(), "cellular_pulse".to_string())]
            .into_iter()
            .collect(),
        moves: vec![MoveSpec {
            id: "cellular_pulse".to_string(),
            clip: ClipBinding {
                clip: "special".to_string(),
                fallbacks: vec!["idle".to_string()],
            },
            duration_s: 0.85,
            windows: vec![
                // The tell. Long enough to be READ, which is what makes the
                // punish fair and the move boss-grade rather than merely strong.
                window(0.0, 0.40, WindowTag::Startup, Vec::new()),
                window(
                    0.40,
                    0.54,
                    WindowTag::Active,
                    vec![HitVolume {
                        shape: VolumeShape::Rect {
                            offset: (30.0, 0.0),
                            half_extents: (34.0, 28.0),
                        },
                        damage: 3,
                        knockback: 140.0,
                        // Flat, exactly as the row authored it — the stage's
                        // ruleset decides whether knockback grows with percent.
                        knockback_growth: 0.0,
                        launch_dir: None,
                        on_hit: None,
                        vfx: None,
                        hit_sfx: None,
                    }],
                ),
                window(0.54, 0.85, WindowTag::Recovery, Vec::new()),
            ],
            events: vec![MoveEvent {
                at_s: 0.40,
                kind: MoveEventKind::Sfx {
                    cue: "pca.cellular_pulse".to_string(),
                },
            }],
            gates: Default::default(),
            start_impulse: None,
            smash_charge_mult: 1.0,
            landing_lag_s: None,
            autocancel_after_s: None,
        }],
    }
}
