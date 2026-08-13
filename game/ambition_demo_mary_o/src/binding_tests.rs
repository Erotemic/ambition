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
            // ⭐ **the SHIPPED string, not a copy of it.** This rebuilt the
            // same braces from the same constants — under a doc comment saying a
            // specially-assembled roster would prove nothing — and had already
            // fallen behind by one archetype.
            &crate::mary_o_roster_ron(),
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
/// **THE TWO ENEMIES AUTHOR THE BODIES THEIR ROSTER ROWS USED TO.**
///
/// Solid Snake and AI Slop are characters (D73 group A): 1 HP each, a slow
/// forward walk, contact damage that is their only offense, and a Wanderer
/// policy that notices nobody. Their `mary_o_snake` / `mary_o_ai_slop` rows are
/// deleted, so this is where those facts are pinned.
///
/// ⛔ pinned HERE rather than left in the roster sweep next door, because
/// `spec_for_brain` answers a deleted key with the `combatant` fallback — six
/// assertions about a row that no longer exists would have gone on passing about
/// the wrong creature.
#[test]
fn the_two_enemies_author_the_bodies_their_roster_rows_used_to() {
    use ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry;
    use ambition_platformer2d::characters::brain::{CharacterBrainTemplate, MoveStyleSpec};

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
    let roster = mary_o_roster();
    let bindings = RoomBindings::default().with_characters(roster.brain_keys());

    // ⭐ **THE TWO ENEMY KEYS LEFT THIS SWEEP ON 2026-08-11, and the reason is
    // that the question changed.** `mary_o_snake` and `mary_o_ai_slop` had to
    // resolve against the roster because the roster was where a snake's body
    // came from — health, speed, contact damage — and `spec_for_brain` answers
    // an unknown key with the generic `combatant` fallback, so a misspelling
    // shipped a correctly-named enemy with the wrong body.
    //
    // Both are CHARACTERS now: they author their own bodies, their twenty
    // placements name them by `character_id`, and their roster rows are deleted.
    // The keys still travel on the placements, but as TAGS — `is_snake_brain`
    // selects the shell behaviour — and a tag that matches nothing simply does
    // not tag, which is visible rather than silently wrong.
    //
    // ⇒ the body question is asked by
    // [`the_two_enemies_author_the_bodies_their_roster_rows_used_to`] instead,
    // beside the definitions that answer it.
    let staged = bindings.sweep_characters(roster.brain_keys().iter().map(|key| {
        (
            key.as_str(),
            "mary_o roster row staged by this demo".to_string(),
        )
    }));
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
/// `publish_under` calls: `ai_slop.rs`, `snake.rs` and `plane.rs` publish their
/// sheets under these exact strings, so a rename on either side fails here
/// rather than in Jon's face.
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
    // The poison for the assertion above: it would also pass against a level
    // with no enemies at all, which is exactly the excuse the stale comment on
    // `mary_o_binds_every_ref_it_declares` used to skip the sweep.
    assert!(
        !crate::level_1_1().enemy_spawns.is_empty(),
        "1-1 authors enemies; if it stops, this test is checking nothing"
    );
}

/// **The snake-plane swarms assemble, and they FLY.**
///
/// ⚠ **this is the STANDALONE road, and it is the reason these two rows
/// survived the migration** (2026-08-11). The swarms are Ambition's characters,
/// registered by that provider — so in a hosted build the placements build them
/// character-first and this roster is never consulted. In the standalone demo
/// there is no content provider, nothing registers them, and these rows are what
/// makes them exist at all.
///
/// ⛔ **`is_aerial` is asserted separately from `move_style`, and that is the
/// point of the test.** `Float` says how it MOVES; `is_aerial` says gravity does
/// not apply. Both catalog rows landed as `body_kind: Standard` an hour after
/// they were written — a snake riding a paper airplane falling out of the sky —
/// and migrating these two prematurely reproduced it exactly.
#[test]
fn both_snake_plane_swarms_assemble_as_flyers() {
    let roster = mary_o_roster();
    for key in [
        crate::plane::PAPER_PLANE_BRAIN_KEY,
        crate::plane::CARTESIAN_PLANE_BRAIN_KEY,
    ] {
        assert!(
            roster.has_brain_key(key),
            "`{key}` is not in Mary-O's assembled roster, so a STANDALONE spawn \
             naming it falls back to a generic stand-still body — which is the \
             silent failure `spec_for_brain` is documented to produce"
        );
        let spec = roster
            .archetype_for(key)
            .unwrap_or_else(|| panic!("`{key}` is in the roster but has no spec"));
        assert_eq!(
            spec.is_aerial,
            Some(true),
            "`{key}` did not assemble as a DECIDED flyer, so gravity applies and \
             the swarm falls out of the sky. ⚠ `None` and `Some(false)` are \
             different failures: `Some(false)` is a row that chose to be \
             grounded, `None` is a row that never said"
        );
        assert_eq!(
            spec.move_style,
            ambition_platformer2d::characters::brain::MoveStyleSpec::Float,
            "`{key}` assembled with a grounded move style"
        );
    }
}
