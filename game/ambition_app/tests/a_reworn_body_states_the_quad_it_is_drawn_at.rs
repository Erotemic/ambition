//! A body that changes form takes the form's own size (Q159, ruled
//! 2026-10-19), in the shipped game.
//!
//! The player wears the kernel guide, the companion dog, the robot and the
//! guide again. A form with no posed body is sized from its sheet at its
//! standing height: the box and the quad an NPC of that form is built with,
//! not the old box with the new art fitted to it (the dog was drawn at a third
//! of its built size, measured 2026-10-06). It is the same body: the same
//! entity, and the damage it took stays. The posed robot brings its own box,
//! as before. The renderer draws the quad the simulation states.

use ambition_platformer2d::characters::actor::{BodyHealth, CharacterForm};
use ambition_platformer2d::combat::components::ActorRenderSize;
use ambition_platformer2d::engine_core::{AbilityBase, BodyBaseSize, BodyKinematics};
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::platformer::lifecycle::PlayerVisual;
use ambition_platformer2d::sprite_sheet::character::{CharacterAnimator, SpritePosedBody};
use bevy::prelude::*;

/// The body an NPC of `character` is built with: (collision box, quad).
fn built(app: &App, character: &str) -> (Vec2, Vec2) {
    let sprite_content = app.world().resource::<ambition_platformer2d::sprite_sheet::SpriteContent>().clone();
    let world = app.world();
    let body = ambition_platformer2d::sprite_sheet::character::catalog_join::sprite_body_collision_for_character_id_from_data(&sprite_content,
        world.resource::<ambition_platformer2d::sprite_sheet::character::sheets::AuthoredSheets>(),
        world
            .resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>()
            .data(),
        character,
        Vec2::new(30.0, 48.0),
    )
    .unwrap_or_else(|| panic!("`{character}` has a sheet with a published body"));
    (body.collision, body.render_size)
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a - b).abs().max_element() < 0.5
}

#[test]
fn a_player_that_changes_form_takes_the_size_of_that_form() {
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
    let robot_box = app.world().get::<BodyBaseSize>(player).expect("a standing box").base_size;
    let robot_abilities = app.world().get::<AbilityBase>(player).expect("base abilities").abilities;
    // Damage the body, so a heal at the change of form shows.
    let max = {
        let mut health = app.world_mut().get_mut::<BodyHealth>(player).expect("the player has health");
        health.health.current = (health.health.max - 1).max(1);
        health.health.max
    };

    // (character, posed). The posed robot is the control of the other road.
    let wears = [
        ("npc_kernel_guide", false),
        ("npc_companion_dog", false),
        ("player_robot_v3", true),
        ("npc_kernel_guide", false),
    ];
    let mut boxes = Vec::new();
    for (character, posed) in wears {
        *app.world_mut().get_mut::<CharacterForm>(player).expect("the player wears a character") =
            CharacterForm::new(character);
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
        assert!(close(stated, drawn), "`{character}`: the body states the quad {stated:?} and is drawn at {drawn:?}");
        let standing = world.get::<BodyBaseSize>(player).expect("a standing box").base_size;
        let live = world.get::<BodyKinematics>(player).expect("kinematics").size();
        if posed {
            assert!(close(standing, robot_box), "back to the robot: its box is {standing:?}, the robot's is {robot_box:?}");
            assert_eq!(
                world.get::<AbilityBase>(player).expect("base abilities").abilities,
                robot_abilities,
                "back to the robot: its base abilities are not the ones it was built with"
            );
        } else {
            let (collision, quad) = built(&app, character);
            assert!(
                close(standing, collision) && close(live, collision) && close(stated, quad),
                "`{character}` is built in the box {collision:?} at the quad {quad:?}; the player that became it \
                 has the box {standing:?} (live {live:?}) at the quad {stated:?}"
            );
        }
        boxes.push(standing);
        let health = world.get::<BodyHealth>(player).expect("health").health;
        assert!(
            health.current < health.max || health.max < max,
            "`{character}`: the change of form healed the body ({}/{})",
            health.current,
            health.max
        );
    }
    assert!(!close(boxes[0], boxes[1]), "premise: the guide and the dog are two sizes: {boxes:?}");
    assert!(close(boxes[0], boxes[3]), "the same form gives the same box on a later change: {boxes:?}");
    let world = app.world_mut();
    assert_eq!(
        world.query_filtered::<Entity, With<PlayerVisual>>().iter(world).collect::<Vec<_>>(),
        vec![player],
        "a change of form built another body"
    );
}

/// What the simulation reads of a body's geometry and motion: its standing
/// box, its collision size, its resolved hurtboxes and its movement tuning.
fn sim_geometry(sim: &mut ambition_app::rl_sim::Platformer2dSimHarness, body: Entity) -> String {
    let world = sim.world();
    format!(
        "{:?} {:?} {:?} {:?} {:?}",
        world.get::<BodyBaseSize>(body).map(|size| size.base_size),
        world.get::<BodyKinematics>(body).map(|kin| kin.canonical_size),
        world.get::<ambition_platformer2d::combat::hurtbox_resolution::ResolvedHurtboxes>(body),
        world.get::<ambition_platformer2d::engine_core::AuthoredMovementTuning>(body).map(|tuning| tuning.0),
        world.get::<ambition_platformer2d::engine_core::ActiveMovementTuning>(body).map(|tuning| tuning.0),
    )
}

/// CHARACTER-FORM slice 3: a skin is presentation only. The player puts on the
/// dog's look (`WornSkin`) and its standing box, collision size, hurtboxes and
/// movement tuning stay as they were. The control is the dog's FORM
/// (`CharacterForm`) on the same body, which moves them.
#[test]
fn a_skin_change_moves_no_collider_hurtbox_or_movement_value() {
    use ambition_app::rl_sim::{AgentAction, AmbitionSim as _, Platformer2dSimHarness};
    use ambition_platformer2d::characters::actor::WornSkin;
    const LOOK: &str = "npc_companion_dog";
    let boot = || {
        let mut sim = Platformer2dSimHarness::new_with_options(crate::common::fixed_60hz_room_options("switch_lab"))
            .expect("the room boots");
        sim.step_n(AgentAction::default(), 10);
        let body = {
            let world = sim.world_mut();
            world
                .query_filtered::<Entity, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
                .single(world)
                .expect("a player")
        };
        (sim, body)
    };

    let (mut sim, body) = boot();
    let before = sim_geometry(&mut sim, body);
    sim.world_mut().entity_mut(body).insert(WornSkin(LOOK.into()));
    sim.step_n(AgentAction::default(), 30);
    let skinned = sim_geometry(&mut sim, body);
    let wears = sim.world().get::<CharacterForm>(body).map(|worn| worn.id().to_owned());
    let skin = sim.world().get::<WornSkin>(body).map(|skin| skin.id().to_owned());

    let (mut control, control_body) = boot();
    *control.world_mut().get_mut::<CharacterForm>(control_body).expect("the player wears a character") =
        CharacterForm::new(LOOK);
    control.step_n(AgentAction::default(), 30);
    let formed = sim_geometry(&mut control, control_body);

    println!("[skin] before {before}\n[skin] skinned {skinned}\n[skin] the dog's form {formed}");
    assert_eq!(skin.as_deref(), Some(LOOK), "premise: the body wears the skin");
    assert_ne!(wears.as_deref(), Some(LOOK), "premise: the skin did not change the worn character");
    assert_eq!(skinned, before, "a skin change moved the body's geometry or motion");
    assert_ne!(formed, before, "the control: the dog's form did not move the body's geometry, so the reading cannot tell a skin from a form");
}
