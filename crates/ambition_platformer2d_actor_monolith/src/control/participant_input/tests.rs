use super::*;
use crate::control::{blank_scripted_control_frames, publish_seat_controls_when_nobody_else_does};
use ambition_characters::control::{PlayerSlot, SeatRawFrames};
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;

/// End-to-end: spawn a player entity with the brain components,
/// populate ControlFrame into the primary slot, run tick_controlled_brains,
/// and assert ActorControl reflects that slot input.
/// Pins the universal-brain seam on the player side.
#[test]
fn player_brain_seam_translates_control_frame_to_actor_control() {
    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    // The seat's frame-mode policy, beside the seat table it interprets.
    app.init_resource::<ambition_characters::control::SeatControlFrameModes>();
    //  and the table it is committed FROM. These are one model:
    // `BrainPlugin` installs both, and a hand-built fixture that takes
    // only the destination is describing a composition that cannot exist.
    app.init_resource::<SeatRawFrames>();
    let mut player = crate::avatar::home_body_scratch(
        ae::Vec2::new(100.0, 100.0),
        ae::AbilitySet::sandbox_all(),
    );
    ae::refresh_movement_resources_clusters(
        &player.abilities,
        &mut player.dash,
        &mut player.jump,
        &mut player.dodge,
        ae::DEFAULT_TUNING.air_jumps,
        ae::RecoveryRefresh::Answered,
    );
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        player,
        ambition_characters::actor::Health::new(10),
    );
    app.world_mut().spawn(bundle);
    app.add_systems(
        Update,
        (
            publish_seat_controls_when_nobody_else_does,
            tick_controlled_brains,
        )
            .chain(),
    );

    // Stamp the control frame with a known input.
    {
        let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
        raw.shape(PlayerSlot::PRIMARY, |cf| {
            cf.axis_x = 1.0;
            cf.jump_pressed = true;
            cf.attack_pressed = true;
            cf.shield_held = true;
        });
    }
    app.update();

    let mut q = app
        .world_mut()
        .query_filtered::<&ActorControl, With<PlayerEntity>>();
    let control = q
        .iter(app.world())
        .next()
        .expect("player entity should have ActorControl");
    // axis_x → desired_vel.x, jump_pressed → jump_pressed, etc.
    assert_eq!(control.0.locomotion.x, 1.0);
    assert!(control.0.jump_pressed);
    assert!(control.0.melee_pressed);
    assert!(control.0.shield_held);
    assert_eq!(control.0.facing, 1.0);
}

/// Build the target through the actor seed, then give it a driving seat.
/// Participant input must reach this body through its own motion model.
#[test]
fn a_possessed_actor_is_driven_by_the_controlled_brain_producer() {
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};

    const POSSESSED_TOP_SPEED: f32 = 137.0;

    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    // The seat's frame-mode policy, beside the seat table it interprets.
    app.init_resource::<ambition_characters::control::SeatControlFrameModes>();
    //  and the table it is committed FROM. These are one model:
    // `BrainPlugin` installs both, and a hand-built fixture that takes
    // only the destination is describing a composition that cannot exist.
    app.init_resource::<SeatRawFrames>();
    app.add_systems(
        Update,
        (
            publish_seat_controls_when_nobody_else_does,
            tick_controlled_brains,
        )
            .chain(),
    );

    let pos = ae::Vec2::new(64.0, 32.0);
    let size = ae::Vec2::new(44.0, 78.0);
    let mut seed = ambition_body_seed::ActorClusterSeed::new(&ambition_sprite_sheet::SpriteContent::default(),
        "possessed",
        "possessed",
        ae::Aabb::new(pos, size * 0.5),
        ambition_entity_catalog::placements::CharacterBrain::Custom("pirate_raider".into()),
        &[],
    );
    seed.kin.size = size;
    seed.kin.pos = pos;
    seed.health.reset();
    let mut params = ae::DEFAULT_TUNING.axis_swept_params();
    params.locomotion.max_run_speed = POSSESSED_TOP_SPEED;
    let body = app
        .world_mut()
        .spawn((
            seed.into_components(),
            // What the production spawn sites add beside the cluster: an intent
            // frame and a seat. Possession moves only the second.
            ambition_characters::control::ActorControl::default(),
            DrivingParticipant(PlayerSlot::PRIMARY),
        ))
        .id();
    app.world_mut()
        .entity_mut(body)
        .insert(ambition_platformer2d_core::movement::MotionModel::axis_swept(params));

    {
        let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
        raw.shape(PlayerSlot::PRIMARY, |cf| {
            cf.axis_x = 1.0;
            cf.jump_pressed = true;
        });
    }
    app.update();

    let control = app
        .world()
        .entity(body)
        .get::<ActorControl>()
        .expect("an actor cluster carries ActorControl")
        .0;
    assert_eq!(
        control.locomotion.x, 1.0,
        "the possessed body did not receive the participant's stick — if this is \
         zero, the controlled producer's query does not match a production actor \
         body and possession moves nothing"
    );
    assert!(
        control.jump_pressed,
        "the possessed body did not receive the participant's jump edge"
    );
    assert_eq!(
        control.velocity_target,
        ae::WorldVec2::new(POSSESSED_TOP_SPEED, 0.0),
        "the direct velocity command must be scaled by THIS body's own movement \
         policy, not by another body's capability and not by actor configuration"
    );
}

/// A scripted sequence now silences a possessed body too.
///
/// A death beat or a flagpole slide driving a possessed body would blank nothing.
///
/// One producer in one phase is what fixes it, and the guard is the ORDER: the
/// two systems run in their real schedule relation, so a future move of either
/// one back across the other turns this red.
#[test]
fn a_scripted_sequence_silences_a_possessed_body() {
    use ambition_characters::control::{ControlHold, ControlHolds};
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};

    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    // The seat's frame-mode policy, beside the seat table it interprets.
    app.init_resource::<ambition_characters::control::SeatControlFrameModes>();
    //  and the table it is committed FROM. These are one model:
    // `BrainPlugin` installs both, and a hand-built fixture that takes
    // only the destination is describing a composition that cannot exist.
    app.init_resource::<SeatRawFrames>();
    app.add_systems(
        Update,
        (
            publish_seat_controls_when_nobody_else_does,
            tick_controlled_brains,
            blank_scripted_control_frames,
        )
            .chain(),
    );

    let pos = ae::Vec2::new(0.0, 0.0);
    let size = ae::Vec2::new(44.0, 78.0);
    let mut seed = ambition_body_seed::ActorClusterSeed::new(&ambition_sprite_sheet::SpriteContent::default(),
        "scripted",
        "scripted",
        ae::Aabb::new(pos, size * 0.5),
        ambition_entity_catalog::placements::CharacterBrain::Custom("pirate_raider".into()),
        &[],
    );
    seed.kin.size = size;
    seed.kin.pos = pos;
    seed.health.reset();
    let body = app
        .world_mut()
        .spawn((
            seed.into_components(),
            ambition_characters::control::ActorControl::default(),
            DrivingParticipant(PlayerSlot::PRIMARY),
            ControlHolds::only(ControlHold::Sequence),
        ))
        .id();

    {
        let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
        raw.shape(PlayerSlot::PRIMARY, |cf| {
            cf.axis_x = 1.0;
            cf.jump_pressed = true;
        });
    }
    app.update();

    let control = app.world().entity(body).get::<ActorControl>().unwrap().0;
    assert_eq!(
        control.locomotion.x, 0.0,
        "a scripted sequence must silence the body it is driving, whichever \
         population that body belongs to"
    );
    assert!(!control.jump_pressed, "the jump edge survived the blanking");
}

/// ⭐⭐ EVERY DRIVEN BODY REGENERATES, not just the possessed subject.
///
/// ⛔⛔ THIS REFILLED ONE `ControlledSubject`. Seat one spent mana on a gauntlet
/// it could never get back — a slow leak rather than a dead verb, which is why
/// it outlived the verbs' own fix.
/// A body driven by `slot` through the controlled brain producer, with the
/// seat table and its frame modes. Returns the app and the body.
fn driven_body_app(slot: ambition_characters::control::PlayerSlot) -> (App, Entity) {
    use ambition_characters::control::DrivingParticipant;
    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    app.init_resource::<ambition_characters::control::SeatControlFrameModes>();
    app.add_systems(Update, (publish_seat_controls_when_nobody_else_does, tick_controlled_brains).chain());
    let pos = ae::Vec2::new(64.0, 32.0);
    let size = ae::Vec2::new(44.0, 78.0);
    let mut seed = ambition_body_seed::ActorClusterSeed::new(&ambition_sprite_sheet::SpriteContent::default(),
        "driven",
        "driven",
        ae::Aabb::new(pos, size * 0.5),
        ambition_entity_catalog::placements::CharacterBrain::Custom("pirate_raider".into()),
        &[],
    );
    seed.kin.size = size;
    seed.kin.pos = pos;
    let body = app
        .world_mut()
        .spawn((seed.into_components(), ActorControl::default(), DrivingParticipant(slot)))
        .id();
    (app, body)
}

fn hold_stick(app: &mut App, slot: ambition_characters::control::PlayerSlot, x: f32) {
    app.world_mut().resource_mut::<SeatRawFrames>().shape(slot, |cf| cf.axis_x = x);
}

fn turn_gravity(app: &mut App, body: Entity, down: ae::Vec2) {
    let mut frame = app
        .world_mut()
        .get_mut::<ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>(body)
        .expect("a driven body has a resolved frame");
    frame.publish_resolved_frame(ae::MotionFrame::from_direction(down, 0.0));
}

/// Review b2c36ae4, finding 6. Seat one holds right and the body's hold is
/// seat one's. Possession moves the body to seat two while gravity turns,
/// and seat two holds right too: that is seat two's own press, read in the
/// body's frame now, not seat one's hold carried into the new frame. A body
/// that no seat drives keeps no hold. Measured before: the hold named no seat,
/// so seat two inherited it, and an undriven body kept it. Poison: keep a
/// hold across seats.
#[test]
fn a_held_stick_does_not_survive_a_change_of_seat() {
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};
    let (mut app, body) = driven_body_app(PlayerSlot::PRIMARY);
    hold_stick(&mut app, PlayerSlot::PRIMARY, 1.0);
    app.update();
    let first = *app.world().get::<HeldControlFrame>(body).expect("premise: seat one's press is held");
    assert_eq!(first.slot, PlayerSlot::PRIMARY);

    let turned = ae::Vec2::new(1.0, 0.0);
    assert_ne!(first.down, turned, "premise: gravity turns");
    turn_gravity(&mut app, body, turned);
    app.world_mut().entity_mut(body).insert(DrivingParticipant(PlayerSlot(1)));
    hold_stick(&mut app, PlayerSlot::PRIMARY, 0.0);
    hold_stick(&mut app, PlayerSlot(1), 1.0);
    app.update();
    let second = *app.world().get::<HeldControlFrame>(body).expect("seat two's press is held");
    assert_eq!(
        (second.slot, second.down),
        (PlayerSlot(1), turned),
        "seat two drives the body under seat one's hold, in the frame seat one pressed in"
    );

    app.world_mut().entity_mut(body).remove::<DrivingParticipant>();
    app.update();
    assert!(app.world().get::<HeldControlFrame>(body).is_none(), "a body no seat drives keeps a hold");
}

/// A seat that two bodies claim drives neither, and the body that held its
/// stick gives the hold up with the authority. Poison: keep the hold on a
/// refused body.
#[test]
fn a_refused_seat_leaves_no_hold() {
    use ambition_characters::control::DrivingParticipant;
    let (mut app, body) = driven_body_app(PlayerSlot::PRIMARY);
    hold_stick(&mut app, PlayerSlot::PRIMARY, 1.0);
    app.update();
    assert!(app.world().get::<HeldControlFrame>(body).is_some(), "premise: the press is held");
    app.world_mut().spawn(DrivingParticipant(PlayerSlot::PRIMARY));
    app.update();
    assert!(app.world().get::<HeldControlFrame>(body).is_none(), "a body the seat refuses keeps its hold");
}

/// A change of the frame mode is a new hold, read in the body's frame now.
#[test]
fn a_held_stick_does_not_survive_a_change_of_frame_mode() {
    use ambition_characters::control::PlayerSlot;
    let held = HeldControlFrame {
        down: ae::Vec2::new(0.0, 1.0),
        raw: ae::Vec2::new(1.0, 0.0),
        slot: PlayerSlot::PRIMARY,
        mode: ae::InputFrameMode::BodyRelativeStrict,
    };
    let turned = ae::Vec2::new(1.0, 0.0);
    let (kept, stick) = hold_stick_through_turns(
        Some(held),
        held.raw,
        turned,
        ae::InputFrameMode::BodyRelativeStrict,
        PlayerSlot::PRIMARY,
    );
    assert_eq!(kept, Some(held), "control: the same seat and mode keep the hold through the turn");
    assert_ne!(stick, held.raw, "control: the kept hold is read in the frame it was pressed in");
    let (next, stick) = hold_stick_through_turns(
        Some(held),
        held.raw,
        turned,
        ae::InputFrameMode::ScreenRelative,
        PlayerSlot::PRIMARY,
    );
    assert_eq!(next.map(|next| (next.down, next.mode)), Some((turned, ae::InputFrameMode::ScreenRelative)));
    assert_eq!(stick, held.raw, "a new hold is read as pressed");
}
