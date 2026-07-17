//! Behaviour tests for the runtime brain-switch authority.
//!
//! These pin the campaign's runtime-switching requirements: `UsePreset` replaces
//! the live brain, `RestoreDefault` rebuilds a fresh default around the AUTHORED
//! home (not the current pose), the same command replays deterministically, a
//! command only touches its target, and temporary control (player / mount) is
//! never overwritten by an autonomous switch.

use super::*;
use ambition_characters::actor::character_catalog::{
    parse_catalog, AuthoredBrainContext, BrainBinding, BrainPresetId, BrainSelection,
    CharacterCatalog,
};
use ambition_characters::actor::ActorPose;
use ambition_characters::brain::{Brain, PlayerSlot, StateMachineCfg};
use ambition_engine_core as ae;
use ambition_platformer_primitives::sim_id::SimId;
use bevy::ecs::message::Messages;

const CATALOG: &str = r#"(
    brain_presets: {
        "stand_still": StandStill,
        "wanderer_puppy_slug": Wanderer(speed: 36.0, aggressiveness: 0.0),
        "melee_brute_striker": MeleeBrute(
            aggressiveness: 1.0, aggro_radius: 220.0, attack_range: 36.0, chase_speed: 110.0,
        ),
        "patrol_peaceful": Patrol(
            spawn_local_x: 0.0, radius: 64.0, speed: 28.0,
            aggressiveness: 0.0, aggro_radius: 80.0, attack_range: 0.0,
        ),
    },
    action_set_presets: { "peaceful": (move_style: Walk) },
    characters: {
        "npc_puppy_slug": (
            display_name: "Puppy Slug", spritesheet: "x.png", manifest: "x_spritesheet.ron",
            tier: MainHall, body_kind: Crawler, composition: None,
            default_brain: "wanderer_puppy_slug", default_action_set: "peaceful", tags: [],
        ),
        "npc_patroller": (
            display_name: "Patroller", spritesheet: "x.png", manifest: "x_spritesheet.ron",
            tier: MainHall, body_kind: Standard, composition: None,
            default_brain: "patrol_peaceful", default_action_set: "peaceful", tags: [],
        ),
    },
)"#;

fn catalog() -> CharacterCatalog {
    CharacterCatalog::from_data(parse_catalog(CATALOG))
}

/// Minimal App with the `BrainCommand` channel + its reducer on `Update`.
fn app() -> App {
    let mut app = App::new();
    app.add_message::<BrainCommand>();
    app.insert_resource(catalog());
    app.add_systems(Update, apply_brain_commands);
    app
}

fn send(app: &mut App, cmd: BrainCommand) {
    app.world_mut()
        .resource_mut::<Messages<BrainCommand>>()
        .write(cmd);
}

/// Spawn a catalog NPC carrying its default brain, binding, and authored context.
fn spawn_npc(app: &mut App, sim: &str, character_id: &str, anchor_x: f32) -> Entity {
    let cat = catalog();
    let (binding, brain) = ambition_characters::actor::character_catalog::resolve_initial_brain(
        &cat,
        character_id,
        None,
        &ambition_characters::actor::character_catalog::BrainBuildContext::at(anchor_x),
    )
    .expect("catalog default resolves");
    app.world_mut()
        .spawn((
            SimId::placement(sim),
            brain,
            binding,
            AuthoredBrainContext::from_placement(anchor_x, 0.0),
            ActorPose::from_parts(ae::Vec2::new(anchor_x, 0.0), ae::Vec2::new(8.0, 8.0), 1.0),
        ))
        .id()
}

/// #5 — `UsePreset` replaces the active brain with the requested preset and
/// records the override in the binding.
#[test]
fn use_preset_replaces_the_live_brain() {
    let mut app = app();
    let e = spawn_npc(&mut app, "puppy", "npc_puppy_slug", 100.0);
    assert_eq!(app.world().get::<Brain>(e).unwrap().label(), "wanderer");

    send(
        &mut app,
        BrainCommand::use_preset(SimId::placement("puppy"), "melee_brute_striker"),
    );
    app.update();

    assert_eq!(app.world().get::<Brain>(e).unwrap().label(), "melee_brute");
    assert_eq!(
        app.world().get::<BrainBinding>(e).unwrap().selection,
        BrainSelection::Override(BrainPresetId::new("melee_brute_striker")),
    );
}

/// #4 — `RestoreDefault` rebuilds a FRESH default brain and clears the override.
#[test]
fn restore_default_rebuilds_a_fresh_default_brain() {
    let mut app = app();
    let e = spawn_npc(&mut app, "puppy", "npc_puppy_slug", 100.0);

    send(
        &mut app,
        BrainCommand::use_preset(SimId::placement("puppy"), "stand_still"),
    );
    app.update();
    assert_eq!(app.world().get::<Brain>(e).unwrap().label(), "stand_still");

    send(
        &mut app,
        BrainCommand::restore_default(SimId::placement("puppy")),
    );
    app.update();

    assert_eq!(app.world().get::<Brain>(e).unwrap().label(), "wanderer");
    assert_eq!(
        app.world().get::<BrainBinding>(e).unwrap().selection,
        BrainSelection::Default,
    );
}

/// A `RestoreDefault` rebuilds a patrol brain around its AUTHORED home, not the
/// actor's current pose — the fix for a patroller re-centering wherever it walked.
#[test]
fn restore_default_uses_the_authored_home_not_the_current_pose() {
    let mut app = app();
    let e = spawn_npc(&mut app, "wanderer", "npc_patroller", 100.0);
    // The patroller wandered far from home.
    app.world_mut().get_mut::<ActorPose>(e).unwrap().center.x = 900.0;

    send(
        &mut app,
        BrainCommand::restore_default(SimId::placement("wanderer")),
    );
    app.update();

    match app.world().get::<Brain>(e).unwrap() {
        Brain::StateMachine(StateMachineCfg::Patrol { cfg, .. }) => {
            assert_eq!(
                cfg.lane.center_x, 100.0,
                "the rebuilt patrol lane centers on the AUTHORED anchor, not the current pose"
            );
        }
        other => panic!("expected a Patrol brain, got {other:?}"),
    }
}

/// #14 — the same command replays deterministically.
#[test]
fn a_brain_switch_replays_deterministically() {
    let switch = || {
        let mut app = app();
        let e = spawn_npc(&mut app, "puppy", "npc_puppy_slug", 100.0);
        send(
            &mut app,
            BrainCommand::use_preset(SimId::placement("puppy"), "melee_brute_striker"),
        );
        app.update();
        (
            app.world().get::<Brain>(e).unwrap().label().to_string(),
            app.world()
                .get::<BrainBinding>(e)
                .unwrap()
                .selection
                .clone(),
        )
    };
    assert_eq!(switch(), switch());
}

/// A command targeting a different SimId leaves this actor untouched.
#[test]
fn a_command_only_touches_its_target() {
    let mut app = app();
    let e = spawn_npc(&mut app, "puppy", "npc_puppy_slug", 100.0);
    send(
        &mut app,
        BrainCommand::use_preset(SimId::placement("someone_else"), "stand_still"),
    );
    app.update();
    assert_eq!(app.world().get::<Brain>(e).unwrap().label(), "wanderer");
}

/// An unknown preset is rejected (binding + brain unchanged) — never a silent
/// fall back to the default or StandStill.
#[test]
fn an_unknown_preset_is_rejected() {
    let mut app = app();
    let e = spawn_npc(&mut app, "puppy", "npc_puppy_slug", 100.0);
    send(
        &mut app,
        BrainCommand::use_preset(SimId::placement("puppy"), "no_such_preset"),
    );
    app.update();
    assert_eq!(
        app.world().get::<Brain>(e).unwrap().label(),
        "wanderer",
        "an unknown preset leaves the live brain unchanged"
    );
    assert_eq!(
        app.world().get::<BrainBinding>(e).unwrap().selection,
        BrainSelection::Default,
        "an unknown preset leaves the binding unchanged"
    );
}

/// A player-controlled body is NOT switched by a brain command — its live brain
/// is player control, and the autonomous command must not overwrite it.
#[test]
fn a_player_controlled_body_is_not_switched() {
    let mut app = app();
    let binding = BrainBinding::new(
        BrainPresetId::new("wanderer_puppy_slug"),
        BrainSelection::Default,
    );
    let e = app
        .world_mut()
        .spawn((
            SimId::placement("possessed"),
            Brain::Player(PlayerSlot::PRIMARY),
            binding,
            AuthoredBrainContext::from_placement(100.0, 0.0),
            ActorPose::from_parts(ae::Vec2::new(100.0, 0.0), ae::Vec2::new(8.0, 8.0), 1.0),
        ))
        .id();

    send(
        &mut app,
        BrainCommand::use_preset(SimId::placement("possessed"), "stand_still"),
    );
    app.update();

    assert!(
        app.world().get::<Brain>(e).unwrap().is_player(),
        "a possessed body keeps player control; the autonomous switch is ignored"
    );
    assert_eq!(
        app.world().get::<BrainBinding>(e).unwrap().selection,
        BrainSelection::Default,
        "the binding is left untouched while under temporary control"
    );
}

/// A mounted body is NOT switched by a brain command either.
#[test]
fn a_mounted_body_is_not_switched() {
    let mut app = app();
    let e = spawn_npc(&mut app, "rider", "npc_puppy_slug", 100.0);
    app.world_mut()
        .entity_mut(e)
        .insert(crate::features::ecs::Mounted);

    send(
        &mut app,
        BrainCommand::use_preset(SimId::placement("rider"), "stand_still"),
    );
    app.update();

    assert_eq!(
        app.world().get::<Brain>(e).unwrap().label(),
        "wanderer",
        "a mounted body's autonomous brain is not switched while it rides"
    );
}
