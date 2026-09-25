//! These drive the REAL adapters + the portal-owned marker components.
use bevy::prelude::*;

use ambition_characters::control::{DrivingParticipant, PlayerSlot, SeatRawFrames, SlotControls};
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

/// Seat-local input surface read and written by the portal warp.
fn hold_x(app: &mut App, axis_x: f32) {
    let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
    let mut frame = raw.get(PlayerSlot::PRIMARY);
    frame.axis_x = axis_x;
    raw.set(PlayerSlot::PRIMARY, frame);
}

fn seat_axis_x(app: &App) -> f32 {
    app.world()
        .resource::<SeatRawFrames>()
        .get(PlayerSlot::PRIMARY)
        .axis_x
}
use ambition_portal2d::{
    PortalChannel, PortalEmission, PortalGunColor, PortalInputWarp, PortalTransit, PortalTuning,
};

use super::{warp_portal_input, withhold_wall_verbs_during_transit};

const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);

#[test]
fn portal_input_warp_transforms_held_input_then_clears() {
    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    app.init_resource::<PortalTuning>();
    // The content adapter brackets the core warp: mirror ControlFrame -> intent
    // before the warp, and the warped intent -> ControlFrame after, so this
    // exercises the full content+core chain on the ControlFrame surface exactly
    // as the game does.
    app.add_systems(Update, warp_portal_input);
    // A 180° warp (a same-wall pair). Player holds RIGHT (anchor right).
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            DrivingParticipant(PlayerSlot::PRIMARY),
            PortalInputWarp {
                n_in: Vec2::new(-1.0, 0.0),
                n_out: Vec2::new(-1.0, 0.0),
                anchor: Vec2::new(1.0, 0.0),
            },
        ))
        .id();

    // Still holding right → input is warped to LEFT (keeps you moving out).
    hold_x(&mut app, 1.0);
    app.update();
    assert!(
        seat_axis_x(&app) < -0.5,
        "held right is warped to left while the warp is active"
    );
    assert!(
        app.world().get::<PortalInputWarp>(player).is_some(),
        "warp persists while held"
    );

    // Release movement → warp drops, input passes through untouched next frame.
    hold_x(&mut app, 0.0);
    app.update();
    assert!(
        app.world().get::<PortalInputWarp>(player).is_none(),
        "release drops the warp"
    );

    // Re-arm, then press a clearly different direction (left) → warp drops.
    app.world_mut().entity_mut(player).insert(PortalInputWarp {
        n_in: Vec2::new(-1.0, 0.0),
        n_out: Vec2::new(-1.0, 0.0),
        anchor: Vec2::new(1.0, 0.0),
    });
    hold_x(&mut app, -1.0);
    app.update();
    assert!(
        app.world().get::<PortalInputWarp>(player).is_none(),
        "a clearly different direction drops the warp"
    );
}

fn abilities_app() -> App {
    let mut app = App::new();
    app.init_resource::<PortalTuning>();
    app.add_systems(
        Update,
        (
            withhold_wall_verbs_during_transit,
            ambition_platformer2d_core::project_body_abilities,
        )
            .chain(),
    );
    app
}

fn effective(app: &App, body: Entity) -> ambition_platformer2d_core::AbilitySet {
    app.world()
        .get::<ambition_platformer2d_core::BodyAbilities>(body)
        .unwrap()
        .abilities
}

/// The aperture-edge hazard is a property of TRANSITING, not of being the
/// primary player: a plain actor (no player markers) mid-transit loses its wall
/// verbs, and gets back exactly what its base grants when the latch goes.
#[test]
fn wall_verbs_are_withheld_for_any_transiting_body_and_come_back_from_its_base() {
    use ambition_platformer2d_core::{AbilityBase, AbilitySet, BodyAbilities};
    let mut app = abilities_app();
    let authored = AbilitySet {
        ledge_grab: true,
        wall_jump: true,
        ..AbilitySet::NONE
    };
    let actor = app
        .world_mut()
        .spawn((
            BodyAbilities::new(authored),
            AbilityBase::new(authored),
            // A body with verbs is an integrated body (ADR 0024 §1).
            ambition_platformer2d_core::movement::MotionModel::default(),
            PortalTransit {
                straddling: BLUE,
                crossed: false,
            },
        ))
        .id();

    app.update();
    let a = effective(&app, actor);
    assert!(
        !a.ledge_grab && !a.wall_jump,
        "a transiting ACTOR has its wall verbs withheld too"
    );

    app.world_mut().entity_mut(actor).remove::<PortalTransit>();
    app.update();
    assert_eq!(
        effective(&app, actor),
        authored,
        "transit end gives back exactly the base: its wall verbs, and nothing it never had"
    );
}

/// A crossing ends by WITHDRAWING its ceiling, not by restoring verbs, so a
/// verb another source withholds stays withheld. Restoring the four wall verbs
/// from the base handed back a verb the session mask had taken away.
#[test]
fn a_transit_ending_does_not_hand_back_a_verb_another_source_withholds() {
    use ambition_platformer2d_core::{
        AbilityBase, AbilityContribution, AbilityContributions, AbilitySet, BodyAbilities,
    };
    let mut app = abilities_app();
    let authored = AbilitySet {
        ledge_grab: true,
        wall_jump: true,
        ..AbilitySet::NONE
    };
    let mut contributions = AbilityContributions::default();
    contributions.set(
        "session.mask",
        AbilityContribution::Ceiling(AbilitySet {
            wall_jump: false,
            ..AbilitySet::ALL
        }),
    );
    let body = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyAbilities::new(authored),
            AbilityBase::new(authored),
            ambition_platformer2d_core::movement::MotionModel::default(),
            contributions,
            PortalTransit {
                straddling: BLUE,
                crossed: false,
            },
        ))
        .id();

    app.update();
    assert!(!effective(&app, body).ledge_grab, "the crossing withholds ledge-grab");

    app.world_mut().entity_mut(body).remove::<PortalTransit>();
    app.update();
    let a = effective(&app, body);
    assert!(a.ledge_grab, "the crossing's own withholding ended");
    assert!(!a.wall_jump, "the transit end handed back a verb the session mask withholds");
}

/// The emergence guard follows the DRIVEN body: possess an actor, send it through a portal, and
/// ITS `PortalEmission` shapes the local input stream
#[test]
fn emission_guard_follows_the_possessed_body() {
    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    app.init_resource::<PortalTuning>();
    app.add_systems(Update, warp_portal_input);
    // Home avatar has NO emission and no seat while possessed; the possessed
    // actor carries the seat and is the one emerging from a right-wall portal
    // (exit normal LEFT, into the room).
    app.world_mut().spawn((PlayerEntity, PrimaryPlayer));
    let _possessed = app
        .world_mut()
        .spawn((
            DrivingParticipant(PlayerSlot::PRIMARY),
            PortalEmission {
                exit_normal: Vec2::new(-1.0, 0.0),
                timer: 1.0,
            },
        ))
        .id();

    // Holding RIGHT (back into the wall) is stripped for the DRIVEN body.
    hold_x(&mut app, 1.0);
    app.update();
    assert!(
        seat_axis_x(&app).abs() < 0.01,
        "the POSSESSED body's emergence guard shapes the input stream"
    );
}

#[test]
fn emission_guard_strips_input_pushing_back_into_the_exit_wall() {
    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    app.init_resource::<PortalTuning>();
    app.add_systems(Update, warp_portal_input);
    // Emerging from a right-wall portal — exit_normal points LEFT (into room).
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            DrivingParticipant(PlayerSlot::PRIMARY),
            PortalEmission {
                exit_normal: Vec2::new(-1.0, 0.0),
                timer: 1.0,
            },
        ))
        .id();
    // Holding RIGHT (back into the wall) is stripped so physics carries you out.
    hold_x(&mut app, 1.0);
    app.update();
    assert!(
        seat_axis_x(&app).abs() < 0.01,
        "input pushing back into the exit wall is stripped during emergence"
    );
    // Holding LEFT (the emergence direction) passes through untouched.
    hold_x(&mut app, -1.0);
    app.update();
    assert!(
        seat_axis_x(&app) < -0.5,
        "input in the emergence direction is preserved"
    );
    let _ = player;
}
