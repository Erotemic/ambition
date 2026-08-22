//! A third party's character is DRAWN, from art the third party owns.
//!
//! Every other link was proved separately and none of them draws anything: the
//! catalog can name `game://sprites/outlander.png`, the provider can describe
//! the sheet, the collision box comes from that sheet, the asset source reads
//! the file, and `build.rs` makes the file real. A chain of five green tests
//! still permits "and the character renders as a placeholder rectangle", which
//! is precisely what it did before U1 and precisely what a stranger evaluating
//! this engine would see.
//!
//! So this builds the SAME app `src/bin/visible.rs` runs — one function, no
//! test-only composition — with the full render graph against no wgpu backend
//! (`without_gpu`, the standard Bevy recipe the in-repo demos use in
//! `ov1_draws_the_world`). It observes ENTITIES and ASSET PATHS, not pixels; a
//! real window changes one argument.
//!
//! The assertion that matters is the last one: the texture the engine chose for
//! this character resolves through the `game://` source, which is the
//! consumer's own tree. A sprite drawn from `sprites/mary_o_spritesheet.png`
//! would pass every other test in this crate.

#![cfg(feature = "visible")]

use bevy::prelude::*;

use outlander::build_windowed_app;

fn drawn() -> App {
    build_windowed_app(false)
}

/// Long enough for shell routing, session activation, asset binding and the
/// character decode — the same settle the acceptance walk uses, for the same
/// reason: this fixture drives a real host lifecycle, not a fixture of one.
fn settle(app: &mut App) {
    for _ in 0..600 {
        app.update();
        if app
            .world()
            .get_resource::<ambition_platformer2d::view::GameAssets>()
            .map(|assets| assets.characters.sheet_state("outlander_wanderer").is_ready())
            .unwrap_or(false)
        {
            return;
        }
    }
}

#[test]
fn the_consumers_character_draws_from_the_consumers_own_art() {
    let mut app = drawn();
    settle(&mut app);

    let assets = app
        .world()
        .get_resource::<ambition_platformer2d::view::GameAssets>()
        .expect("the umbrella asset install put `GameAssets` in the world");
    let state = assets.characters.sheet_state("outlander_wanderer");
    let ready = match state {
        ambition_platformer2d::character::CharacterSheetState::Ready(asset) => asset,
        ambition_platformer2d::character::CharacterSheetState::Declared { .. } => {
            // The engine records WHY a decode failed. Reporting it beats
            // "it did not work", which is the diagnostic quality this fixture
            // exists to hold the engine to in the first place.
            let states = app
                .world()
                .get_resource::<ambition_platformer2d::character::CharacterLoadStates>();
            let outcome = states.and_then(|states| states.outcome("outlander_wanderer"));
            let failures: Vec<String> = states
                .map(|states| {
                    states
                        .failures()
                        .map(|(token, why)| format!("{token}: {why:?}"))
                        .collect()
                })
                .unwrap_or_default();
            panic!(
                "the consumer's character never decoded — it is declared and \
                 still waiting, which draws a placeholder rectangle. \
                 outcome={outcome:?} failures={failures:?}"
            )
        }
        ambition_platformer2d::character::CharacterSheetState::Unknown => panic!(
            "the engine does not know this character at all, so the catalog \
             fragment this crate registers is not reaching the decode"
        ),
    };

    // The frame the PROVIDER authored, not one the engine had baked. 32×48 is
    // this crate's sheet RON; every engine sheet is a different size, so this
    // cannot pass by resolving somebody else's art.
    assert_eq!(
        (ready.spec.frame_width, ready.spec.frame_height),
        (32, 48),
        "the decoded sheet is not the one this crate authored"
    );

    // ...and the texture came out of the consumer's own tree. This is the
    // assertion the other five tests cannot make: a character drawn from
    // `sprites/mary_o_spritesheet.png` satisfies all of them.
    let server = app.world().resource::<AssetServer>();
    let path = server
        .get_path(ready.texture.id())
        .expect("a decoded sheet's texture has an asset path");
    assert_eq!(
        path.source(),
        &bevy::asset::io::AssetSourceId::from("game"),
        "the character's texture resolves through `{}` rather than the \
         consumer's own `game://` source — its art is somewhere else's",
        path.source()
    );
    assert!(
        path.path().ends_with("outlander.png"),
        "the character draws `{}`, not the art this crate generated",
        path.path().display()
    );
}

/// The world around it is drawn too — the OV1 claim, restated from outside the
/// workspace. A consumer whose character renders over an empty screen has an
/// engine that can draw ITS content and not the consumer's room.
#[test]
fn the_consumers_room_is_drawn_by_the_engine_and_not_by_the_consumer() {
    let mut app = drawn();
    settle(&mut app);

    let mut cameras = app.world_mut().query::<&Camera>();
    assert!(
        cameras.iter(app.world()).next().is_some(),
        "no camera exists, so nothing this consumer composed can be seen"
    );

    let mut visuals = app
        .world_mut()
        .query::<&ambition_platformer2d::view::RoomVisual>();
    let drawn_blocks = visuals.iter(app.world()).count();
    assert!(
        drawn_blocks > 0,
        "the consumer's authored room produced no `RoomVisual` entities, so the \
         engine drew none of the geometry the fixture built"
    );
}
