//! Where the HUD actually lands, at a real display size.
//!
//! This exists because two placement bugs shipped that no other test could see.
//! The declaration was right, the slot count was right, the text was right — and
//! on screen the readouts sat in a corner by accident and the card started at
//! the middle of the display and ran off the right-hand side. Placement is
//! resolved from the live window, so it is invisible to anything that does not
//! build one.
#![cfg(feature = "visible")]

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::presentation::gameplay_presentation::ResolvedGameplayPresentation;
use ambition_platformer2d::presentation::DeclaredHudRoot;
use ambition_demo_mary_o_app::{build_windowed_demo_app, RenderMode};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window, WindowResolution};

/// An ordinary widescreen monitor.
const DISPLAY: ae::Vec2 = ae::Vec2::new(1920.0, 1080.0);

fn windowed_app() -> App {
    let mut app = build_windowed_demo_app(RenderMode::Headless);
    let mut resolution = WindowResolution::new(DISPLAY.x as u32, DISPLAY.y as u32);
    resolution.set_scale_factor(1.0);
    resolution.set(DISPLAY.x, DISPLAY.y);
    app.world_mut().spawn((
        Window {
            resolution,
            ..default()
        },
        PrimaryWindow,
    ));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    for _ in 0..12 {
        app.update();
    }
    app
}

fn nodes(app: &mut App) -> Vec<(Node, String)> {
    let mut query = app
        .world_mut()
        .query_filtered::<(&Node, &Text), With<DeclaredHudRoot>>();
    query
        .iter(app.world())
        .map(|(node, text)| (node.clone(), text.0.clone()))
        .collect()
}

/// The readouts live in reserved surround, NOT over the level; the card is
/// centred on the gameplay rect rather than starting at the middle of it.
#[test]
fn the_readouts_sit_in_the_surround_and_the_card_is_actually_centred() {
    let mut app = windowed_app();
    let gameplay = app
        .world()
        .resource::<ResolvedGameplayPresentation>()
        .gameplay_rect;
    assert!(
        gameplay.min.x > 0.0,
        "this profile must pillarbox at {DISPLAY:?} or the test proves nothing"
    );

    for (node, text) in nodes(&mut app) {
        let Val::Px(left) = node.left else {
            panic!(
                "HUD nodes are placed in pixels; got {:?} for {text:?}",
                node.left
            );
        };

        if let Val::Px(width) = node.width {
            assert_eq!(
                left, gameplay.min.x,
                "a centred card starts at the gameplay rect, not at its middle"
            );
            assert_eq!(
                width,
                gameplay.width(),
                "and spans it, so the text centres on the level"
            );
        } else {
            // The READOUTS: genuinely PLACED in a reserved region, not merely
            // sitting somewhere that looks fine.
            //
            // Compared against `OVERLAY_ANCHOR` by name, because on a widely pillarboxed
            // display the overlay corner also lands inside the surround — so "is it clear of
            // the gameplay rect" cannot tell "placed where it asked" from "never moved at all".
            assert!(
                left + 120.0 <= gameplay.min.x,
                "readout {text:?} at x={left} must sit clear of the gameplay \
                 rect starting at {}",
                gameplay.min.x
            );
            let Val::Px(top) = node.top else {
                panic!("HUD nodes are placed in pixels; got {:?}", node.top);
            };
            let overlay = ambition_platformer2d::render::hud::OVERLAY_ANCHOR;
            assert!(
                (left - overlay.x).abs() > 0.5 || (top - overlay.y).abs() > 0.5,
                "readout {text:?} is still at the OVERLAY anchor {overlay:?} — it \
                 declared a region this display does not reserve and nothing \
                 fell it back to one that is, so it landed there by luck"
            );
        }
    }
}

/// The intro card is TRANSIENT. A card that never retires is just a permanent
/// banner across the middle of the level.
#[test]
fn the_intro_card_retires_on_its_own() {
    let mut app = windowed_app();
    assert!(
        nodes(&mut app).iter().any(|(_, t)| t.contains("WORLD 1-1")),
        "the level opens on its title card"
    );
    for _ in 0..200 {
        app.update();
    }
    assert!(
        nodes(&mut app)
            .iter()
            .all(|(_, t)| !t.contains("WORLD 1-1")),
        "and it is gone a few seconds later, without a hide path"
    );
}
