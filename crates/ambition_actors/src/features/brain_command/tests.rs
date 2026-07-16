//! Behaviour tests for the runtime brain-switch authority + directive routing.
//!
//! These pin the campaign's runtime-switching requirements: `UseBrainPreset`
//! replaces the live brain, `RestoreDefault` rebuilds a fresh default, the same
//! command replays deterministically, and a gameplay ACTION request is a
//! different channel from a pure ANIMATION directive.

use super::*;
use ambition_characters::actor::character_catalog::{
    parse_catalog, BrainBinding, BrainPresetId, BrainSelection, CharacterCatalog,
};
use ambition_characters::actor::ActorPose;
use ambition_characters::brain::Brain;
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
    },
    action_set_presets: { "peaceful": (move_style: Walk) },
    characters: {
        "npc_puppy_slug": (
            display_name: "Puppy Slug", spritesheet: "x.png", manifest: "x_spritesheet.ron",
            tier: MainHall, body_kind: Crawler, composition: None,
            default_brain: "wanderer_puppy_slug", default_action_set: "peaceful", tags: [],
        ),
    },
)"#;

fn catalog() -> CharacterCatalog {
    CharacterCatalog::from_data(parse_catalog(CATALOG))
}

/// Minimal App with the directive channels + the (route → apply) chain on Update.
fn app() -> App {
    let mut app = App::new();
    app.add_message::<BrainCommand>();
    app.add_message::<ActorDirectiveRequest>();
    app.add_message::<ActorActionRequest>();
    app.add_message::<ActorAnimationDirective>();
    app.add_message::<DispositionDirective>();
    app.insert_resource(catalog());
    app.add_systems(
        Update,
        (route_actor_directives, apply_brain_commands).chain(),
    );
    app
}

/// A puppy-slug NPC entity carrying its default (wanderer) brain + binding.
fn spawn_puppy(app: &mut App, sim: &str) -> Entity {
    let binding = BrainBinding::new(
        BrainPresetId::new("wanderer_puppy_slug"),
        BrainSelection::Default,
    );
    let brain = catalog()
        .build_default_brain("npc_puppy_slug", 100.0)
        .expect("puppy default brain resolves");
    app.world_mut()
        .spawn((
            SimId::placement(sim),
            brain,
            binding,
            ActorPose::from_parts(ae::Vec2::new(100.0, 0.0), ae::Vec2::new(8.0, 8.0), 1.0),
        ))
        .id()
}

fn request(app: &mut App, target: &str, directive: ActorDirective) {
    app.world_mut()
        .resource_mut::<Messages<ActorDirectiveRequest>>()
        .write(ActorDirectiveRequest {
            target: SimId::placement(target),
            directive,
        });
}

/// #5 — `UseBrainPreset` replaces the active brain with the requested preset and
/// records the override in the binding.
#[test]
fn use_preset_replaces_the_live_brain() {
    let mut app = app();
    let e = spawn_puppy(&mut app, "puppy");
    assert_eq!(app.world().get::<Brain>(e).unwrap().label(), "wanderer");

    request(
        &mut app,
        "puppy",
        ActorDirective::UseBrainPreset(BrainPresetId::new("melee_brute_striker")),
    );
    app.update();

    assert_eq!(
        app.world().get::<Brain>(e).unwrap().label(),
        "melee_brute",
        "UseBrainPreset must swap the live brain to the requested preset"
    );
    assert_eq!(
        app.world().get::<BrainBinding>(e).unwrap().selection,
        BrainSelection::Override(BrainPresetId::new("melee_brute_striker")),
        "the binding records the override selection"
    );
}

/// #4 — `RestoreDefault` rebuilds a FRESH default brain and clears the override.
#[test]
fn restore_default_rebuilds_a_fresh_default_brain() {
    let mut app = app();
    let e = spawn_puppy(&mut app, "puppy");

    // Override first, then restore.
    request(
        &mut app,
        "puppy",
        ActorDirective::UseBrainPreset(BrainPresetId::new("stand_still")),
    );
    app.update();
    assert_eq!(app.world().get::<Brain>(e).unwrap().label(), "stand_still");

    request(&mut app, "puppy", ActorDirective::RestoreDefaultBrain);
    app.update();

    assert_eq!(
        app.world().get::<Brain>(e).unwrap().label(),
        "wanderer",
        "RestoreDefault rebuilds the character's default (wanderer) brain"
    );
    assert_eq!(
        app.world().get::<BrainBinding>(e).unwrap().selection,
        BrainSelection::Default,
        "the binding returns to Default"
    );
}

/// #14 — the same command replays deterministically: two independent runs of the
/// same switch produce the identical resulting brain + binding.
#[test]
fn a_brain_switch_replays_deterministically() {
    let switch = || {
        let mut app = app();
        let e = spawn_puppy(&mut app, "puppy");
        request(
            &mut app,
            "puppy",
            ActorDirective::UseBrainPreset(BrainPresetId::new("melee_brute_striker")),
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
    assert_eq!(
        switch(),
        switch(),
        "the same switch must reproduce the same state"
    );
}

/// A command targeting a different SimId leaves this actor untouched (routing is
/// by stable id, and only the matching entity is mutated).
#[test]
fn a_command_only_touches_its_target() {
    let mut app = app();
    let e = spawn_puppy(&mut app, "puppy");
    request(
        &mut app,
        "someone_else",
        ActorDirective::UseBrainPreset(BrainPresetId::new("stand_still")),
    );
    app.update();
    assert_eq!(
        app.world().get::<Brain>(e).unwrap().label(),
        "wanderer",
        "a command for another id must not change this actor"
    );
}

/// #15 — a jump ACTION request is a distinct channel from a jump ANIMATION
/// directive: the action lands on the gameplay channel, the animation on the
/// presentation channel, and neither leaks into the other.
#[test]
fn action_request_is_distinct_from_animation_directive() {
    let mut app = app();
    request(
        &mut app,
        "puppy",
        ActorDirective::RequestAction(ActorActionKind::Jump),
    );
    request(
        &mut app,
        "puppy",
        ActorDirective::PlayAnimation("jump".to_string()),
    );
    app.update();

    let actions: Vec<_> = app
        .world_mut()
        .resource_mut::<Messages<ActorActionRequest>>()
        .drain()
        .collect();
    let anims: Vec<_> = app
        .world_mut()
        .resource_mut::<Messages<ActorAnimationDirective>>()
        .drain()
        .collect();

    assert_eq!(
        actions,
        vec![ActorActionRequest {
            target: SimId::placement("puppy"),
            action: ActorActionKind::Jump,
        }],
        "a RequestAction(Jump) lands on the gameplay action channel"
    );
    assert_eq!(
        anims,
        vec![ActorAnimationDirective {
            target: SimId::placement("puppy"),
            clip: "jump".to_string(),
        }],
        "a PlayAnimation(\"jump\") lands on the presentation channel"
    );
}

/// A brain directive routes to a `BrainCommand`, never onto the action or
/// animation channels — the concerns stay separate.
#[test]
fn a_brain_directive_never_leaks_to_action_or_animation() {
    let mut app = app();
    spawn_puppy(&mut app, "puppy");
    request(
        &mut app,
        "puppy",
        ActorDirective::UseBrainPreset(BrainPresetId::new("stand_still")),
    );
    app.update();
    let actions = app
        .world_mut()
        .resource_mut::<Messages<ActorActionRequest>>()
        .drain()
        .count();
    let anims = app
        .world_mut()
        .resource_mut::<Messages<ActorAnimationDirective>>()
        .drain()
        .count();
    assert_eq!(
        actions, 0,
        "a brain directive must not emit an action request"
    );
    assert_eq!(
        anims, 0,
        "a brain directive must not emit an animation directive"
    );
}
