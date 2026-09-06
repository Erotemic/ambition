//! A Smash move names effect art the old five-variant enum could not express.
//!
//! `ExplosionKind` had five variants, and they were the five rows of ONE
//! spritesheet — so a fighter's whole visual vocabulary was `classic_burst`,
//! `burst_round`, `shockwave`, `smoke_burst`, `starburst`, while 189 authored
//! effect rows shipped beside them. `Feel::Special` now claims `sonic_boom`, a
//! row of `generic_exotic_fx`, and this pins the entire chain that makes that
//! legal: the vocabulary accepts it, the art resolves, and the sound it makes is
//! the cue the bank packed for that same row — one name, three answers, no
//! translation table.
//!
//! this does NOT assert the sheet is decoded in this app, and the reason is a
//! finding rather than an omission: the Smash shell installs no
//! `PlatformerAssetsPlugin` at all (Mary-O, Sanic and Twintrack each do), so
//! `GameAssets` does not exist in this process and no sheet-driven visual —
//! fighters included — has art here. That the engine SHIPS the effect sheets is
//! pinned on a composed host that binds assets:
//! `ambition_app/tests/the_engine_ships_its_own_effects.rs`.

use ambition_platformer2d::entity_catalog::MoveEventKind;
use ambition_platformer2d::sprite_sheet::fx::{authored_effect, is_authored_effect};

#[test]
fn a_move_names_art_outside_the_old_five_and_the_whole_chain_resolves() {
    // 1. the vocabulary a roster validator asks. The old five said no to this.
    assert!(is_authored_effect("sonic_boom"));
    assert!(!is_authored_effect("kaboom"), "a typo still names nothing");

    // 2. the art, and 3. the sound — both addressed by the SAME authored name.
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

    // 4. a shipped fighter actually names it, and the engine's own presentation
    //    validator — handed the SHEETS as its oracle — accepts the whole table.
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
