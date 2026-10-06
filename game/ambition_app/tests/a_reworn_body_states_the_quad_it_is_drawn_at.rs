//! A body that changes character states the quad its art is drawn at.
//!
//! The landmark query (a hand, a head) answers from a body's art at the scale
//! the body states (`ActorRenderSize`). A body built as a character states the
//! quad its seed resolved. A body that CHANGES to a character with a sheet and
//! no posed body stated nothing: the wear grant restated a quad only for a
//! posed body. Only a player re-wears in the shipped games, and the renderer
//! draws such a player with the frame fitted to the box the body keeps.
//!
//! So the quad stated at a re-wear is that fit, not the quad the character is
//! built with. This arm holds the two against each other in the shipped game:
//! what the simulation states, and what the renderer's animator draws.

use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::combat::components::ActorRenderSize;
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::platformer::lifecycle::PlayerVisual;
use ambition_platformer2d::sprite_sheet::character::{CharacterAnimator, SpritePosedBody};
use bevy::prelude::*;

/// The quad the catalog join resolves for a character that is BUILT: the quad
/// the seed states. A re-worn body is not drawn at it.
fn built_quad(app: &App, character: &str) -> Vec2 {
    let world = app.world();
    ambition_platformer2d::sprite_sheet::character::catalog_join::sprite_body_collision_for_character_id_from_data(
        world.resource::<ambition_platformer2d::sprite_sheet::character::sheets::AuthoredSheets>(),
        world
            .resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>()
            .data(),
        character,
        Vec2::new(30.0, 48.0),
    )
    .unwrap_or_else(|| panic!("`{character}` has a sheet with a published body"))
    .render_size
}

#[test]
fn the_player_states_the_quad_its_art_is_drawn_at_after_each_wear() {
    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
        |_| {},
    );
    for _ in 0..30 {
        app.update();
    }
    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_content::provider::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    let mut player = None;
    for _ in 0..1200 {
        app.update();
        let world = app.world_mut();
        let mut players = world.query_filtered::<Entity, With<PlayerVisual>>();
        player = players.iter(world).next();
        if player.is_some() {
            break;
        }
    }
    let player = player.expect("the Ambition route has a player");

    // (character, posed). The posed robot is the control of the other road.
    let wears = [
        ("npc_kernel_guide", false),
        ("npc_companion_dog", false),
        ("player_robot_v3", true),
        ("npc_kernel_guide", false),
    ];
    let mut stated_quads = Vec::new();
    for (character, posed) in wears {
        *app.world_mut().get_mut::<WornCharacter>(player).expect("the player wears a character") =
            WornCharacter::new(character);
        // The wear lands, the art binds, and the animator draws a frame.
        for _ in 0..60 {
            app.update();
        }
        let world = app.world();
        assert_eq!(
            world.get::<SpritePosedBody>(player).is_some(),
            posed,
            "premise: `{character}` is {} a posed body",
            if posed { "" } else { "not" }
        );
        let stated = world
            .get::<ActorRenderSize>(player)
            .unwrap_or_else(|| panic!("the player wears `{character}` and states no quad"))
            .0;
        let drawn = world
            .get::<CharacterAnimator>(player)
            .and_then(|animator| animator.render_basis)
            .unwrap_or_else(|| panic!("the player wears `{character}` and its animator has drawn no frame"))
            .render_size;
        assert!(
            (stated - drawn).abs().max_element() < 1.0e-3,
            "the player wears `{character}`: it states the quad {stated:?} and its art is drawn at {drawn:?}"
        );
        if !posed {
            // The premise that makes this arm about a RE-wear: the quad the
            // character is built with is another quad.
            let built = built_quad(&app, character);
            assert!(
                (stated - built).abs().max_element() > 5.0,
                "premise: `{character}` is built at {built:?}, and a player that wears it is drawn at {stated:?}"
            );
        }
        stated_quads.push(stated);
    }
    assert!(
        (stated_quads[0] - stated_quads[1]).abs().max_element() > 1.0,
        "premise: the guide and the dog are drawn at two quads: {stated_quads:?}"
    );
    assert_eq!(stated_quads[0], stated_quads[3], "the same character states the same quad on a later wear");
}
