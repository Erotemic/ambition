//! Mary-O binds every reference she declares — checked through the engine's
//! sweep, from outside the engine.
//!
//! Mary-O is the interesting consumer for this because she has NO `.ldtk` file.
//! She builds `RoomSpec`s in Rust and stages her enemies as spawn requests, so
//! the cross-content validator that lives in `game/ambition_content` — which
//! reads LDtk JSON — could never have covered a single one of her references.
//! Everything asserted here comes from resolving the world IR instead, which is
//! what makes the capability the engine's rather than one game's.

use bevy::prelude::App;

use ambition::actors::features::CharacterRoster;
use ambition::actors::world::rooms::RoomBindings;
use ambition::platformer::binding::BindingLedger;
use ambition::platformer::world_item_art::{WorldItemArtManifest, WorldItemSpriteRef};

use crate::ai_slop::AI_SLOP_BRAIN_KEY;
use crate::powerups::{BLOSSOM_SPRITE, MILK_SPRITE};
use crate::snake::SNAKE_BRAIN_KEY;

/// The roster Mary-O actually registers, from the SAME rows the app installs.
///
/// One fragment, not the two per-enemy helpers: assembly rejects a second
/// fragment from the same provider, and reading the shipped rows is the point —
/// a roster assembled specially for this test would prove nothing about the game.
fn mary_o_roster() -> CharacterRoster {
    use ambition::actors::features::{CharacterRosterAppExt, CharacterRosterFragment};

    let mut app = App::new();
    app.register_character_roster_fragment(
        CharacterRosterFragment::from_ron(
            crate::provider::MARY_O_EXPERIENCE,
            None::<String>,
            &format!(
                "{{{}{}}}",
                crate::snake::SNAKE_ROSTER_ROWS,
                crate::ai_slop::AI_SLOP_ROSTER_ROWS
            ),
        )
        .expect("Mary-O enemy roster should be valid"),
    );
    app.finish();
    app.update();
    app.world()
        .get_resource::<CharacterRoster>()
        .expect("Mary-O's roster fragments assemble into a roster")
        .clone()
}

/// The art manifest Mary-O's experience plugin registers.
fn mary_o_world_item_art() -> WorldItemArtManifest {
    let mut app = App::new();
    app.add_plugins(crate::provider::MaryOExperiencePlugin);
    app.world_mut()
        .remove_resource::<WorldItemArtManifest>()
        .expect("Mary-O registers her pickup art")
}

/// Every reference Mary-O declares resolves against what Mary-O registers: the
/// brain keys her enemies stage under, and the art ids her pickups carry.
///
/// Neither could be checked any other way, and both have shipped broken.
/// `CharacterRoster::spec_for_brain` has no failure mode — an unknown key
/// silently becomes the generic `combatant` fallback — so misspelling
/// `mary_o_snake` would have shipped correctly-named enemies with the wrong
/// body, speed, and health. And the spark blossom really did go undrawn behind
/// an art id nothing bound.
///
/// The rooms are NOT swept here: Mary-O authors no kinematic paths, ground
/// items, or enemy spawns, so a room sweep would be an assertion that cannot
/// fail. `world::rooms::binding_tests` covers the sweep against a room that has
/// those families.
#[test]
fn mary_o_binds_every_ref_it_declares() {
    let roster = mary_o_roster();
    let bindings = RoomBindings::default().with_characters(roster.brain_keys());

    let staged = bindings.sweep_characters([
        (SNAKE_BRAIN_KEY, "mary_o snake content staging"),
        (AI_SLOP_BRAIN_KEY, "mary_o ai_slop content staging"),
    ]);
    assert!(
        staged.is_empty(),
        "every staged brain key resolves against the registered roster:\n{staged}"
    );

    let art = mary_o_world_item_art();
    let sprites = art.sprite_ids();
    let mut ledger = BindingLedger::new();
    for (sprite, reward) in [
        (MILK_SPRITE, "grow cap reward"),
        (BLOSSOM_SPRITE, "spark blossom reward"),
    ] {
        ledger.resolve(&sprites, &WorldItemSpriteRef::new(sprite), reward);
    }
    let pickups = ledger.finish();
    assert!(
        pickups.is_empty(),
        "every pickup's art id resolves against the manifest Mary-O registers:\n{pickups}"
    );
}
