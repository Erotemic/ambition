//! Mary-O binds every reference she declares — checked through the engine's
//! sweep, from outside the engine.
//!
//! ⚠ **this said Mary-O "has NO `.ldtk` file" and "stages her enemies as spawn
//! requests".** Both stopped being true in the 08-04 migration: she has
//! `assets/worlds/mary_o.ldtk`, and the engine's authored-enemy construction
//! builds them. The reason to check here is unchanged but it is a different
//! reason — what a level FILE declares and what the demo REGISTERS are two
//! halves that no single crate validates, and resolving the world IR is how you
//! ask both at once.

use bevy::prelude::App;

use ambition_platformer2d::actors::features::CharacterRoster;
use ambition_platformer2d::actors::world::rooms::RoomBindings;
use ambition_platformer2d::platformer::binding::BindingLedger;
use ambition_platformer2d::platformer::world_item_art::{WorldItemArtManifest, WorldItemSpriteRef};

use crate::ai_slop::{AI_SLOP_BRAIN_KEY, AI_SLOP_DISPLAY_NAME};
use crate::powerups::{CINDER_BEACON_SPRITE, STAR_WAND_SPRITE};
use crate::snake::{SNAKE_BRAIN_KEY, SNAKE_DISPLAY_NAME};

/// The roster Mary-O actually registers, from the SAME rows the app installs.
///
/// One fragment, not the two per-enemy helpers: assembly rejects a second
/// fragment from the same provider, and reading the shipped rows is the point —
/// a roster assembled specially for this test would prove nothing about the game.
fn mary_o_roster() -> CharacterRoster {
    use ambition_platformer2d::actors::features::{CharacterRosterAppExt, CharacterRosterFragment};

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
/// `CharacterRoster::spec_for_brain` has no failure mode — an unknown key
/// silently becomes the generic `combatant` fallback — so misspelling
/// `mary_o_snake` would have shipped correctly-named enemies with the wrong
/// body, speed, and health.
///
/// This does NOT cover the cinder beacon's actual bug, and an earlier version of
/// this comment claimed it did. That id was registered correctly all along; the
/// PNG it named did not exist. Nothing here opens a file, so nothing here could
/// have caught it — `render::item_visuals::report_unloadable_item_art` is what
/// does, at runtime, by checking the load rather than the reference.
///
/// ⛔ **this used to say the rooms are not swept because "Mary-O authors no
/// kinematic paths, ground items, or enemy spawns, so a room sweep would be an
/// assertion that cannot fail".** She authors 17 enemy spawns now — the LDtk
/// migration made that sentence false and nobody came back for it, so the one
/// place that would have noticed every enemy losing its art was excused from
/// looking. See [`every_authored_enemy_is_named_something_that_has_a_sheet`].
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
        (STAR_WAND_SPRITE, "star wand reward"),
        (CINDER_BEACON_SPRITE, "cinder beacon reward"),
    ] {
        ledger.resolve(&sprites, &WorldItemSpriteRef::new(sprite), reward);
    }
    let pickups = ledger.finish();
    assert!(
        pickups.is_empty(),
        "every pickup's art id resolves against the manifest Mary-O registers:\n{pickups}"
    );
}

/// **An authored enemy must be named something Mary-O publishes a sheet under.**
///
/// The render binder resolves art by NAME and installs nothing on a miss, so the
/// only symptom of a wrong name is a red placeholder rectangle in a running game
/// and one `warn!` nobody was reading. Every enemy in both levels drew that way
/// between `07f0fc7cc` and this test, because the LDtk `EnemySpawn` entities
/// carry no name and the converter falls back to the LDtk identifier —
/// `"EnemySpawn"`, which resolves nothing.
///
/// The two sides this ties together are the level and the demo's own
/// `publish_under` calls: `ai_slop.rs` and `snake.rs` publish their sheets under
/// these exact strings, so a rename on either side fails here rather than in
/// Jon's face.
#[test]
fn every_authored_enemy_is_named_something_that_has_a_sheet() {
    let published = [SNAKE_DISPLAY_NAME, AI_SLOP_DISPLAY_NAME];
    for (room_id, room) in [
        ("1-1", crate::level_1_1()),
        ("1-2", crate::level_1_2::level_1_2()),
    ] {
        let unnamed: Vec<&str> = room
            .enemy_spawns
            .iter()
            .map(|spawn| spawn.name.as_str())
            .filter(|name| !published.contains(name))
            .collect();
        assert!(
            unnamed.is_empty(),
            "level {room_id} authors enemies whose names resolve no sheet, so they \
             draw as red placeholder rectangles: {unnamed:?}. \
             Mary-O publishes sheets under {published:?}."
        );
    }
    // The poison for the assertion above: it would also pass against a level
    // with no enemies at all, which is exactly the excuse the stale comment on
    // `mary_o_binds_every_ref_it_declares` used to skip the sweep.
    assert!(
        !crate::level_1_1().enemy_spawns.is_empty(),
        "1-1 authors enemies; if it stops, this test is checking nothing"
    );
}
