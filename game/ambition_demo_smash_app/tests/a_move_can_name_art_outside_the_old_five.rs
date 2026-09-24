//! A Smash move names effect art the old five-variant enum could not express.
//!
//! `ExplosionKind` had five variants, the five rows of one spritesheet.
//! `Feel::Special` now claims `sonic_boom`, a row of `generic_exotic_fx`. This
//! pins the chain: the vocabulary accepts it, the art resolves, and its sound
//! is the cue the bank packed for the same row. One name, three answers, no
//! translation table.
//!
//! It does not assert that the sheet is decoded in this app: the headless
//! build installs no `PlatformerAssetsPlugin`, so `GameAssets` does not exist.
//! `ambition_app/tests/the_engine_ships_its_own_effects.rs` pins that the
//! engine ships the effect sheets.

use ambition_platformer2d::entity_catalog::MoveEventKind;
use ambition_platformer2d::sprite_sheet::fx::{authored_effect, is_authored_effect};

#[test]
fn a_move_names_art_outside_the_old_five_and_the_whole_chain_resolves() {
    // 1. The vocabulary a roster validator asks. The old five said no.
    assert!(is_authored_effect("sonic_boom"));
    assert!(!is_authored_effect("kaboom"), "a typo still names nothing");

    // 2. The art, and 3. the sound, both addressed by the same name.
    let effect = authored_effect("sonic_boom").expect("generic_exotic_fx ships it");
    assert_eq!(effect.sheet, "generic_exotic_fx");
    assert_eq!(effect.cue, "vfx.generic_exotic.sonic_boom");
    assert_eq!(
        ambition_platformer2d::render::fx::effect_cue(ambition_platformer2d::vfx::FxId::new(
            "sonic_boom"
        )),
        Some(ambition_platformer2d::sfx::SfxId::new(
            "vfx.generic_exotic.sonic_boom"
        )),
    );

    // 4. A shipped fighter names it, and the engine's presentation
    //    validator, given the sheets as its oracle, accepts the whole table.
    let set = ambition_demo_smash::george_booul_moveset::george_booul_moveset();
    assert!(
        set.moves
            .iter()
            .any(|m| m.events.iter().any(
                |e| matches!(&e.kind, MoveEventKind::Vfx { effect, .. } if effect == "sonic_boom")
            )),
        "George's special claims the Feel::Special look, which is `sonic_boom`"
    );
    for m in &set.moves {
        let problems = m.presentation_problems(is_authored_effect);
        assert!(problems.is_empty(), "{problems:?}");
    }
}
