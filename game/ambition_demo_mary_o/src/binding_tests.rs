//! Mary-O binds every reference she declares — checked through the engine's
//! sweep, from outside the engine.
//!
//! The reason to check here is unchanged but it is a different reason — what a level FILE
//! declares and what the demo REGISTERS are two halves that no single crate validates, and
//! resolving the world IR is how you ask both at once.

use bevy::prelude::App;

use ambition_platformer2d::platformer::binding::BindingLedger;
use ambition_platformer2d::platformer::world_item_art::{WorldItemArtManifest, WorldItemSpriteRef};

use crate::ai_slop::AI_SLOP_DISPLAY_NAME;
use crate::powerups::{CINDER_BEACON_SPRITE, STAR_WAND_SPRITE};
use crate::snake::SNAKE_DISPLAY_NAME;

/// The prepared cast Mary-O actually registers — the SAME registrations the
/// app installs, which is the point: a cast assembled specially for this test
/// would prove nothing about the game. Hand the assertions to `check`, because
/// the registry is read in place rather than cloned out.
fn with_mary_o_prepared_cast(
    check: impl FnOnce(&ambition_platformer2d::characters::prepared::PreparedCharacterRegistry),
) {
    let mut app = App::new();
    crate::install_mary_o_content(&mut app);
    ambition_platformer2d::platformer::app_finalization::finalize(&mut app);
    check(
        app.world()
            .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>(
            )
            .expect("installing this demo's content registers its characters"),
    );
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
/// That id was registered correctly all along; the PNG it named did not exist. Nothing here
/// opens a file, so nothing here could have caught it —
/// `render::item_visuals::report_unloadable_item_art` is what does, at runtime, by checking the
/// load rather than the reference.
///
/// See [`every_authored_enemy_is_named_something_that_has_a_sheet`].
///
/// Their `mary_o_snake` / `mary_o_ai_slop` rows are deleted, so this is where those facts are
/// pinned.
#[test]
fn the_two_enemies_author_the_bodies_their_roster_rows_used_to() {
    use ambition_platformer2d::characters::brain::{CharacterBrainTemplate, MoveStyleSpec};
    use ambition_platformer2d::characters::prepared::PreparedCharacterRegistry;

    let mut app = bevy::prelude::App::new();
    crate::install_mary_o_content(&mut app);
    ambition_platformer2d::platformer::app_finalization::finalize(&mut app);
    let registry = app
        .world()
        .get_resource::<PreparedCharacterRegistry>()
        .expect("installing this demo's content registers its characters");

    for (id, run_speed) in [
        (crate::snake::SNAKE_SHEET_TARGET, 46.0),
        (crate::ai_slop::AI_SLOP_SHEET_TARGET, 42.0),
    ] {
        let definition = registry
            .get(id)
            .unwrap_or_else(|| panic!("`{id}` is not registered, so nothing can build one"));
        assert_eq!(definition.vitals.max_health, Some(1), "{id}");
        let locomotion = definition.locomotion.unwrap_or_else(|| {
            panic!(
                "`{id}` states no locomotion, so it cannot be built \
                                      character-first and its deleted row is a regression"
            )
        });
        assert_eq!(locomotion.run_speed, run_speed, "{id}");
        assert!(matches!(locomotion.move_style, MoveStyleSpec::Walk), "{id}");
        assert!(
            definition.contact_damage.is_some(),
            "`{id}` stopped hurting on touch, which is its ONLY offense"
        );
        let profile = definition
            .autonomous_profile
            .unwrap_or_else(|| panic!("`{id}` states no policy"));
        assert_eq!(profile.template, CharacterBrainTemplate::Wanderer, "{id}");
    }
}

#[test]
fn mary_o_binds_every_ref_it_declares() {
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

/// An authored enemy must be named something Mary-O publishes a sheet under.
///
/// Every enemy in both levels drew that way between `07f0fc7cc` and this test, because the LDtk
/// `EnemySpawn` entities carry no name and the converter falls back to the LDtk identifier —
/// `"EnemySpawn"`, which resolves nothing.
///
/// The two sides this ties together are the level and the demo's own
/// `publish_under` calls: `ai_slop.rs`, `snake.rs` and `plane.rs` publish their
/// sheets under these exact strings, so a rename on either side fails here rather than at runtime.
#[test]
fn every_authored_enemy_is_named_something_that_has_a_sheet() {
    let published = [
        SNAKE_DISPLAY_NAME,
        AI_SLOP_DISPLAY_NAME,
        crate::plane::PAPER_PLANE_DISPLAY_NAME,
        crate::plane::CARTESIAN_PLANE_DISPLAY_NAME,
    ];
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
    assert!(
        !crate::level_1_1().enemy_spawns.is_empty(),
        "1-1 authors enemies; if it stops, this test is checking nothing"
    );
}

/// The snake-plane swarms are registered characters, and they FLY.
///
/// the STANDALONE road is what this defends — the reason the pair's
/// roster rows survived two migrations. Mary-O is their ONE provider now
/// she registers
/// `npc_snakes_on_a_*` in every composition, so a standalone 1-2 builds the
/// same bodies the hosted app does and there is no fallback left to diverge.
///
/// The row-vs-character fork had the hosted build ambling at the default half speed while the
/// standalone row flew at full — the fork hid a real divergence, and this is the number the
/// unification chose.
#[test]
fn both_snake_plane_swarms_assemble_as_flyers() {
    use ambition_platformer2d::characters::brain::{CharacterBrainTemplate, MoveStyleSpec};

    with_mary_o_prepared_cast(|registry| {
        for (id, run_speed, max_health) in [
            (crate::plane::PAPER_PLANE_CHARACTER_ID, 58.0, 1),
            (crate::plane::CARTESIAN_PLANE_CHARACTER_ID, 38.0, 2),
        ] {
            let definition = registry
                .get(id)
                .unwrap_or_else(|| panic!("`{id}` is not registered, so nothing can build one"));
            assert_eq!(definition.vitals.max_health, Some(max_health), "{id}");
            let locomotion = definition
                .locomotion
                .as_ref()
                .unwrap_or_else(|| panic!("`{id}` states no locomotion"));
            assert_eq!(locomotion.run_speed, run_speed, "{id}");
            assert_eq!(
                locomotion.baseline_free_flight,
                Some(true),
                "`{id}` did not register as a DECIDED flyer, so gravity applies \
                 and the swarm falls out of the sky. ⚠ `None` and `Some(false)` \
                 are different failures: `Some(false)` is a body that chose to \
                 be grounded, `None` is a body that never said"
            );
            assert!(
                matches!(locomotion.move_style, MoveStyleSpec::Float),
                "`{id}` registered with a grounded move style"
            );
            let profile = definition
                .autonomous_profile
                .as_ref()
                .unwrap_or_else(|| panic!("`{id}` states no policy"));
            assert_eq!(profile.template, CharacterBrainTemplate::Aerial, "{id}");
            assert_eq!(
                profile.patrol_effort, 1.0,
                "`{id}` lost the full-pace patrol its deleted row authored — \
                 the swarm now ambles at half speed"
            );
            assert_eq!(
                (profile.aggro_radius, profile.attack_range),
                (0.0, 0.0),
                "`{id}`'s zeros are what make it ROAM and never DIVE; a nonzero \
                 aggro radius turns it into a homing dive-bomber"
            );
        }
    });
}
