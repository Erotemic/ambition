//! The presentation an engine PROMISES, asked of a third party's app.
//!
//! None of them crashed. The backdrop was absent, or present and motionless; the quality budget
//! existed and never moved.
//!
//! `scripts/check_engine_systems_are_engine_installed.py` catches the SHAPE —
//! "no engine crate registers this" — and it is a text search over registration
//! sites. This file asks the other question, of the only composition that can
//! answer it honestly: a third party builds its app the documented way and the
//! pictures are there. Outlander is not a fixture of the engine's assumptions;
//! it is a real out-of-workspace crate whose only dependency is `ambition_platformer2d`.

#![cfg(feature = "visible")]

use bevy::prelude::*;

use outlander::build_windowed_app;

/// Drive the app until `ready` answers, or give up after `frames`.
fn settle_until(app: &mut App, frames: usize, ready: impl Fn(&App) -> bool) -> bool {
    for _ in 0..frames {
        app.update();
        if ready(app) {
            return true;
        }
    }
    false
}

fn parallax_layer_positions(app: &mut App) -> Vec<(f32, f32)> {
    let world = app.world_mut();
    let mut query = world
        .query_filtered::<&Transform, With<ambition_platformer2d::view::ParallaxLayerVisual>>();
    query
        .iter(world)
        .map(|transform| (transform.translation.x, transform.translation.y))
        .collect()
}

/// The backdrop exists, and it MOVES with the camera. (72h S12)
///
/// `sync_parallax_layers` was registered by the shipped app alone, so a
/// consumer's backdrop spawned at the world origin and stayed there — sliding
/// out of frame as the camera walked away, which is the one thing a parallax
/// layer exists not to do. The art is correct, in the wrong place, and only once
/// you walk: nothing about that reads as a missing system.
#[test]
fn the_backdrop_is_drawn_and_follows_the_camera() {
    let mut app = build_windowed_app(false);

    let spawned = settle_until(&mut app, 600, |app| {
        app.world()
            .try_query_filtered::<(), With<ambition_platformer2d::view::ParallaxLayerVisual>>()
            .map(|mut query| query.iter(app.world()).next().is_some())
            .unwrap_or(false)
    });
    assert!(
        spawned,
        "no parallax layer was ever spawned in a consumer's app, so the \
         assertion below would compare two empty lists — the engine draws this \
         room's backdrop or it does not, and this says it does not"
    );

    let before = parallax_layer_positions(&mut app);

    // Walk.
    //
    // The endpoints are the one pair of samples that cannot see a following backdrop working.
    //
    // Nothing guarantees the outlander's course does not double back either, so
    // this tracks the largest deviation across the walk. That holds whether the
    // route loops, reverses, or runs straight.
    let mut max_deviation = 0.0f32;
    for _ in 0..180 {
        outlander::drive_control_frame(
            &mut app,
            ambition_platformer2d::sim::ControlFrame {
                axis_x: 1.0,
                ..Default::default()
            },
        );
        app.update();
        let now = parallax_layer_positions(&mut app);
        if now.len() == before.len() {
            for (start, current) in before.iter().zip(&now) {
                // Layer positions are (x, y) here, unlike the Sanic twin's bare
                // x — take the distance so a purely vertical parallax counts too.
                let moved = ((current.0 - start.0).powi(2) + (current.1 - start.1).powi(2)).sqrt();
                max_deviation = max_deviation.max(moved);
            }
        }
    }

    let after = parallax_layer_positions(&mut app);
    assert_eq!(
        before.len(),
        after.len(),
        "the layer set changed while walking, so the comparison below is not \
         about motion"
    );
    assert!(
        max_deviation > 1.0,
        "over 180 frames of walking, no parallax layer ever moved more than \
         {max_deviation} px from where it started. The backdrop is pinned to the \
         world while the camera moves, and it looks like art that is simply \
         somewhere else. ⚠ before blaming a system: check that the SUBJECT moved \
         and that the camera left its clamp — the sibling of this assertion was \
         misread as four different bugs, every one of them innocent."
    );
}

// The quality-budget half of this file does not belong here, and finding
// that out was worth the attempt. `ResolvedVisualQuality` reads
// `UserSettings`, and this consumer is built `default-features = false` — it
// never asked for the `ambition_persistence` capability, so `ambition_platformer2d::
// persistence` does not exist for it and there is no settings resource to read.
// The quality resolve is inert in this composition BY CONSTRUCTION rather than
// by omission, which is slice H working as intended.
//
// The engine-side guard for that half is
// `scripts/check_engine_systems_are_engine_installed.py`: the sync is registered
// by `VisualQualityPlugin` for every composition, so a consumer that DOES take
// the persistence capability gets it. A consumer-level test belongs in a fixture
// that takes that capability, not in the one whose whole point is taking as
// little as possible.
