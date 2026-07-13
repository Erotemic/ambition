//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::actor::BodyBaseSize;
use crate::actor::BodyKinematics;
use crate::actor::PrimaryPlayer;
use crate::features::ActorFaction;
use ambition_characters::brain::{PlayerSlot, StateMachineCfg};

fn vec2(x: f32, y: f32) -> ambition_engine_core::Vec2 {
    ambition_engine_core::Vec2::new(x, y)
}

/// App with the trigger + 1s/frame real time, so 2 held frames clear the 2s hold.
fn trigger_app() -> App {
    let mut app = App::new();
    app.insert_resource(ambition_input::ControlFrame::default());
    app.insert_resource(ambition_time::WorldTime {
        raw_dt: 1.0,
        scaled_dt: 1.0,
    });
    app.init_resource::<PossessionState>();
    app.add_systems(
        Update,
        (possession_trigger_system, release_possession_if_target_lost).chain(),
    );
    app
}

fn spawn_home(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            Brain::Player(PlayerSlot::PRIMARY),
            ActorControl::default(),
            BodyKinematics {
                pos: vec2(0.0, 0.0),
                vel: vec2(0.0, 0.0),
                size: vec2(24.0, 40.0),
                facing: 1.0,
            },
            // The vacate-exit is a discrete transit through the home body's
            // full clusters + policy (ADR 0024 authority) — spawn the real set.
            crate::features::MotionModel::default(),
            crate::actor::AncillaryMovementBundle::from_scratch(
                ambition_engine_core::BodyClusterScratch::new_with_abilities(
                    vec2(0.0, 0.0),
                    ambition_engine_core::AbilitySet::default(),
                ),
            ),
        ))
        .id()
}

fn spawn_candidate(app: &mut App, pos: ambition_engine_core::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            CenteredAabb::new(pos, vec2(12.0, 16.0)),
            Brain::StateMachine(StateMachineCfg::StandStill),
            ActorControl::default(),
            ActorFaction::Enemy,
        ))
        .id()
}

fn brain_slot(app: &App, e: Entity) -> Option<PlayerSlot> {
    app.world().get::<Brain>(e).and_then(|b| b.player_slot())
}

fn faction_of(app: &App, e: Entity) -> ActorFaction {
    *app.world().get::<ActorFaction>(e).unwrap()
}

fn hold_down_interact(app: &mut App, held: bool) {
    let mut control = app
        .world_mut()
        .resource_mut::<ambition_input::ControlFrame>();
    control.axis_y = if held { 1.0 } else { 0.0 };
    control.interact_held = held;
}

#[test]
fn possession_transfers_the_player_brain_and_release_restores_it() {
    let mut app = trigger_app();
    let home = spawn_home(&mut app);
    let actor = spawn_candidate(&mut app, vec2(80.0, 0.0)); // in range

    // Before possession: home carries the player brain; the actor its own.
    assert_eq!(brain_slot(&app, home), Some(PlayerSlot::PRIMARY));
    assert_eq!(brain_slot(&app, actor), None);

    // Hold Down+Interact: 1s, then 2s → crosses the threshold → possess.
    hold_down_interact(&mut app, true);
    app.update(); // hold_timer = 1.0
    assert_eq!(brain_slot(&app, actor), None, "not possessed mid-hold");
    app.update(); // hold_timer = 2.0 ≥ threshold → transfer

    // After possession: the ACTOR carries the player brain; the home avatar
    // no longer does; the actor is player-aligned; its old brain is stashed.
    assert_eq!(brain_slot(&app, actor), Some(PlayerSlot::PRIMARY));
    assert_eq!(
        brain_slot(&app, home),
        None,
        "home avatar's player brain is vacated"
    );
    assert!(app.world().get::<Brain>(home).is_none());
    // Effective allegiance: the target's AUTHORED faction is NOT mutated by
    // possession (it stays Enemy). Combat treats it as Player because it
    // carries `Brain::Player` — verified by the targeting/damage tests — so
    // there is no flip to bookkeep and no restore on release.
    assert_eq!(
        faction_of(&app, actor),
        ActorFaction::Enemy,
        "possession must NOT overwrite the authored faction"
    );
    assert_eq!(
        app.world().resource::<PossessionState>().possessed,
        Some(actor)
    );
    // The REPORTED BUG's root cause is gone: the vacated home avatar has a
    // neutral `ActorControl` and no brain to repopulate it, so it emits no
    // melee/attack this frame or any frame while possessed — attack authority
    // can only originate from the body carrying `Brain::Player`.
    assert_eq!(
        app.world().get::<ActorControl>(home).map(|c| c.0),
        Some(ambition_characters::actor::control::ActorControlFrame::neutral()),
        "vacated home avatar's control frame is cleared — no attack authority"
    );

    // Release: a fresh Down+Interact press hands control back.
    hold_down_interact(&mut app, false);
    app.update();
    hold_down_interact(&mut app, true);
    app.update();

    assert_eq!(
        brain_slot(&app, home),
        Some(PlayerSlot::PRIMARY),
        "release restores the home avatar's player brain"
    );
    assert_eq!(
        brain_slot(&app, actor),
        None,
        "release restores the actor's autonomous brain"
    );
    assert_eq!(
        faction_of(&app, actor),
        ActorFaction::Enemy,
        "authored faction unchanged across the whole possess/release cycle"
    );
    assert!(app
        .world()
        .resource::<PossessionState>()
        .possessed
        .is_none());
    // Vacate exit: the home avatar stepped out where the actor stood.
    let home_pos = app
        .world_mut()
        .query_filtered::<&BodyKinematics, With<PlayerEntity>>()
        .single(app.world())
        .unwrap()
        .pos;
    assert_eq!(home_pos, vec2(80.0, 0.0));
}

#[test]
fn exactly_one_body_carries_the_player_brain_before_and_after() {
    let mut app = trigger_app();
    app.init_resource::<ControlledSubject>();
    app.add_systems(Update, resolve_controlled_subject);
    let home = spawn_home(&mut app);
    let actor = spawn_candidate(&mut app, vec2(80.0, 0.0));
    app.update();
    assert_eq!(app.world().resource::<ControlledSubject>().0, Some(home));

    hold_down_interact(&mut app, true);
    app.update(); // hold_timer = 1.0
    app.update(); // hold_timer = 2.0 → brain transfer commands queued
    app.update(); // transfer applied; resolver re-derives the subject
    assert_eq!(
        app.world().resource::<ControlledSubject>().0,
        Some(actor),
        "controlled subject follows the player brain onto the possessed actor"
    );
}

#[test]
fn a_brief_tap_does_not_possess() {
    let mut app = trigger_app();
    let _home = spawn_home(&mut app);
    let actor = spawn_candidate(&mut app, vec2(80.0, 0.0));
    hold_down_interact(&mut app, true);
    app.update();
    hold_down_interact(&mut app, false);
    app.update();
    assert_eq!(brain_slot(&app, actor), None, "a brief tap doesn't possess");
}

#[test]
fn out_of_range_actors_are_not_possessed() {
    let mut app = trigger_app();
    let _home = spawn_home(&mut app);
    let actor = spawn_candidate(&mut app, vec2(900.0, 0.0)); // far out of range
    hold_down_interact(&mut app, true);
    app.update();
    app.update();
    app.update();
    assert_eq!(
        brain_slot(&app, actor),
        None,
        "nothing in range → no transfer"
    );
}

/// The mandate's headline invariant: while controlling a possessed target,
/// pressing attack emits `ActorActionMessage` for the TARGET, and the vacated
/// home avatar emits nothing. The collapse of the bug: attack authority
/// follows the body carrying `Brain::Player`, resolved by the SAME
/// `emit_brain_action_messages` stream for every body.
#[test]
fn attack_while_controlling_target_emits_only_for_the_target() {
    use ambition_characters::actor::ActorPose;
    use ambition_characters::brain::{
        emit_brain_action_messages, ActionSet, ActorActionMessage, MeleeActionSpec, SwipeSpec,
    };

    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    let kit = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    // Vacated home avatar: neutral control (its brain was transferred away),
    // but it still owns a melee ActionSet + a pose.
    let home = app
        .world_mut()
        .spawn((ActorControl::default(), kit.clone(), ActorPose::default()))
        .id();
    // Possessed target: its `Brain::Player` produced a melee-pressed frame.
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.facing = 1.0;
    let target = app
        .world_mut()
        .spawn((ActorControl(frame), kit, ActorPose::default()))
        .id();

    app.add_systems(Update, emit_brain_action_messages);
    app.update();

    let msgs: Vec<_> = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
        .drain()
        .collect();
    let melee: Vec<_> = msgs.iter().filter(|m| m.is_melee()).collect();
    assert_eq!(melee.len(), 1, "exactly one melee action this frame");
    assert_eq!(
        melee[0].actor, target,
        "the attack originates from the possessed target"
    );
    assert!(
        melee.iter().all(|m| m.actor != home),
        "the vacated home avatar emits no attack"
    );
}

#[test]
fn losing_the_target_hands_control_back_to_home() {
    let mut app = trigger_app();
    let home = spawn_home(&mut app);
    let actor = spawn_candidate(&mut app, vec2(80.0, 0.0));
    hold_down_interact(&mut app, true);
    app.update();
    app.update();
    assert_eq!(brain_slot(&app, actor), Some(PlayerSlot::PRIMARY));
    // The possessed actor despawns (died / left the room).
    app.world_mut().entity_mut(actor).despawn();
    app.update();
    assert_eq!(
        brain_slot(&app, home),
        Some(PlayerSlot::PRIMARY),
        "the home avatar reclaims control when the possessed body is lost"
    );
    assert!(app
        .world()
        .resource::<PossessionState>()
        .possessed
        .is_none());
}
