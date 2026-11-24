//! The seat consequences of a portal transit, measured through
//! `PortalSchedulePlugin` where a game gets them, and the input guards on the
//! seat frame.

use bevy::prelude::*;

use ambition_characters::control::{DrivingParticipant, PlayerSlot, SeatRawFrames, SlotControls};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_portal2d::{
    BodyTeleported, PlacedPortal, PortalChannel, PortalConvention,
    PortalEmission, PortalGunColor, PortalInputWarp, PortalTuning,
};

use super::warp_portal_input;
use ambition_characters::actor::control::ActorControlFrame as ActorFrame;
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core::LocalAxes;

/// Hold the seat's stick at `axis_x` and, standing in for the translator
/// (`tick_controlled_brains`), write the driven body's control from it as
/// that translation does every tick: the guard reads the stick for its hold
/// and rewrites the resolved control.
fn hold_x(app: &mut App, body: Entity, axis_x: f32) {
    let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
    let mut frame = raw.get(PlayerSlot::PRIMARY);
    frame.axis_x = axis_x;
    raw.set(PlayerSlot::PRIMARY, frame);
    translate(app, body, Vec2::new(axis_x, 0.0));
}

/// Write `local` as the body's resolved locomotion and movement-aimed attack.
fn translate(app: &mut App, body: Entity, local: Vec2) {
    let mut control = ActorFrame::neutral();
    control.locomotion = LocalAxes::from_vec(local);
    control.attack_axis = LocalAxes::from_vec(local);
    control.facing = if local.x.abs() > 0.01 { local.x.signum() } else { 0.0 };
    app.world_mut().entity_mut(body).insert(ActorControl(control));
}

fn locomotion(app: &App, body: Entity) -> Vec2 {
    app.world().get::<ActorControl>(body).map_or(Vec2::ZERO, |c| c.0.locomotion.vec())
}

fn seat_axis_x(app: &App, body: Entity) -> f32 {
    locomotion(app, body).x
}

#[derive(Clone, Copy, Debug)]
enum Body {
    HomeWithoutTheSeat,
    PossessedWithTheSeat,
}

/// One body at a blue portal on a wall, whose orange partner sits on the same
/// wall above it, stepped through `PortalSchedulePlugin`.
fn same_wall_pair_app(who: Body) -> (App, Entity) {
    use ambition_platformer2d_shared_tangle::schedule::{GameMode, SimScheduleExt};
    let mut app = App::new();
    app.set_sim_schedule(Update);
    app.init_resource::<ambition_platformer2d_shared_tangle::time::SimDt>();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    app.insert_resource(State::new(GameMode::Playing));
    // The reflection convention supplies the mirror of a same-wall
    // turn-around as a facing flip rather than a roll.
    app.insert_resource(PortalTuning {
        convention: PortalConvention::Reflection,
        ..Default::default()
    });
    app.add_plugins(crate::PortalSchedulePlugin);
    for (color, y) in [(PortalGunColor::BLUE, 200.0), (PortalGunColor::ORANGE, 600.0)] {
        app.world_mut().spawn(PlacedPortal::fixed(
            PortalChannel::Gun(color),
            Vec2::new(20.0, y),
            Vec2::new(1.0, 0.0),
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap());
    }
    let mut body = app.world_mut().spawn(BodyKinematics { material: Default::default(),
        pos: Vec2::new(20.0, 200.0),
        vel: Vec2::new(-100.0, 0.0),
        canonical_size: Vec2::new(24.0, 40.0),
        facing: -1.0,
    });
    match who {
        Body::HomeWithoutTheSeat => body.insert((PlayerEntity, PrimaryPlayer)),
        Body::PossessedWithTheSeat => body.insert(DrivingParticipant(PlayerSlot::PRIMARY)),
    };
    let body = body.id();
    let kin = *app.world().get::<BodyKinematics>(body).unwrap();
    let mut sweep = ambition_platformer2d_core::SweepSample::at_rest(kin, Vec2::Y);
    sweep.prev = kin.pos + Vec2::X * 10.0;
    sweep.vel = kin.vel;
    app.world_mut().entity_mut(body).insert(sweep);
    (app, body)
}

/// Step until the body arrives at the orange portal, and return it then: a
/// consequence that ran before the transit would show one tick late.
fn step_to_arrival(app: &mut App, body: Entity) -> BodyKinematics {
    for _ in 0..4 {
        app.update();
        let kin = *app.world().get::<BodyKinematics>(body).unwrap();
        if kin.pos.y > 400.0 {
            return kin;
        }
    }
    *app.world().get::<BodyKinematics>(body).unwrap()
}

/// Every body transits the same way, and only the body a seat DRIVES turns
/// around through a same-wall pair: its facing follows its seat's input, and a
/// brain decides the facing of any other body.
///
/// Possession is the case that tells the two apart. The home body keeps its
/// player-population markers but not the seat; the possessed actor has the
/// seat and none of the markers. Both carry their momentum out of the exit.
#[test]
fn every_body_keeps_its_facing_and_carries_its_physical_parity() {
    for who in [Body::PossessedWithTheSeat, Body::HomeWithoutTheSeat] {
        let (mut app, body) = same_wall_pair_app(who);
        let kin = step_to_arrival(&mut app, body);
        assert!(kin.pos.y > 400.0, "{who:?} must transit to the orange portal, pos={:?}", kin.pos);
        assert!(kin.vel.x > 0.0, "{who:?} must carry its momentum out of the exit, vel={:?}", kin.vel);
        assert_eq!(kin.facing, -1.0, "{who:?}: facing selects the authored pose");
        assert!(kin.material.reflected(), "{who:?}: the physical map carries parity");
    }
}

/// The body a seat drives is marked on the tick it arrives: the trace and the
/// trail are told the snap was intentional, and its seat's input is guarded as
/// it emerges. A body no seat drives gets none of them.
#[test]
fn only_the_driven_body_is_marked_and_guarded_on_arrival() {
    #[derive(Resource, Default)]
    struct Heard(Vec<Entity>, Vec<Entity>);
    fn hear(
        mut heard: ResMut<Heard>,
        mut teleported: MessageReader<BodyTeleported>,
        mut breaks: MessageReader<ambition_platformer2d_actor_monolith::avatar::trail::TrailContinuityBreak>,
    ) {
        heard.0.extend(teleported.read().map(|m| m.body));
        heard.1.extend(breaks.read().map(|m| m.body));
    }
    for (who, marked) in [(Body::PossessedWithTheSeat, true), (Body::HomeWithoutTheSeat, false)] {
        let (mut app, body) = same_wall_pair_app(who);
        app.init_resource::<Heard>();
        app.add_systems(Update, hear.after(ambition_portal2d::PortalSet::Transited));
        let kin = step_to_arrival(&mut app, body);
        assert!(kin.pos.y > 400.0, "{who:?} must transit, pos={:?}", kin.pos);
        let heard = app.world().resource::<Heard>();
        assert_eq!(heard.0.contains(&body), marked, "{who:?}: BodyTeleported");
        assert_eq!(heard.1.contains(&body), marked, "{who:?}: the trail seam");
        assert_eq!(
            app.world().get::<PortalEmission>(body).is_some(),
            marked,
            "{who:?}: the emergence guard"
        );
        // The next tick, a hold back into the exit wall (the exit faces +x) is
        // stripped from the driven body's movement while the guard is fresh.
        hold_x(&mut app, body, -1.0);
        app.update();
        let kept = seat_axis_x(&app, body);
        assert_eq!(kept.abs() < 0.01, marked, "{who:?}: the hold into the exit wall, {kept}");
    }
}

#[test]
fn portal_input_warp_transforms_held_input_then_clears() {
    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    app.init_resource::<PortalTuning>();
    app.add_systems(Update, warp_portal_input);
    // A 180° warp (a same-wall pair). Player holds RIGHT (anchor right).
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            DrivingParticipant(PlayerSlot::PRIMARY),
            PortalInputWarp {
                map: Mat2::from_diagonal(Vec2::new(-1.0, 1.0)),
                anchor: Vec2::new(1.0, 0.0),
            },
        ))
        .id();

    // Still holding right → movement is warped to LEFT (keeps you moving out).
    hold_x(&mut app, player, 1.0);
    app.update();
    assert!(
        seat_axis_x(&app, player) < -0.5,
        "held right is warped to left while the warp is active"
    );
    assert!(
        app.world().get::<PortalInputWarp>(player).is_some(),
        "warp persists while held"
    );

    // Release movement → warp drops, input passes through untouched next frame.
    hold_x(&mut app, player, 0.0);
    app.update();
    assert!(
        app.world().get::<PortalInputWarp>(player).is_none(),
        "release drops the warp"
    );

    // Re-arm, then press a clearly different direction (left) → warp drops.
    app.world_mut().entity_mut(player).insert(PortalInputWarp {
        map: Mat2::from_diagonal(Vec2::new(-1.0, 1.0)),
        anchor: Vec2::new(1.0, 0.0),
    });
    hold_x(&mut app, player, -1.0);
    app.update();
    assert!(
        app.world().get::<PortalInputWarp>(player).is_none(),
        "a clearly different direction drops the warp"
    );
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
    let possessed = app
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
    hold_x(&mut app, possessed, 1.0);
    app.update();
    assert!(
        seat_axis_x(&app, possessed).abs() < 0.01,
        "the POSSESSED body's emergence guard shapes its movement"
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
    hold_x(&mut app, player, 1.0);
    app.update();
    assert!(
        seat_axis_x(&app, player).abs() < 0.01,
        "input pushing back into the exit wall is stripped during emergence"
    );
    // ...from movement only: the attack the same stick aimed keeps its side.
    assert_eq!(
        app.world().get::<ActorControl>(player).unwrap().0.attack_axis.vec(),
        Vec2::X,
        "the emergence guard is a movement constraint; it re-aimed an attack"
    );
    // Holding LEFT (the emergence direction) passes through untouched.
    hold_x(&mut app, player, -1.0);
    app.update();
    assert!(
        seat_axis_x(&app, player) < -0.5,
        "input in the emergence direction is preserved"
    );
}

/// IN2: the emergence guard strips the push into the exit wall in the body's
/// RESOLVED frame. Under down `(1,0)` a flyer's local `(0,-1)` is world
/// `(-1,0)`, straight into an exit facing `+x`, and local `(-1,0)` is world
/// `(0,1)`, along the wall. The old raw-axis guard kept the first and removed
/// the second. A walker cannot travel along its local Y, so the guard leaves
/// that axis alone rather than invent a crouch or a jump.
#[test]
fn the_emergence_guard_reads_the_bodys_resolved_frame() {
    let after = |local: Vec2, flying: bool| {
        let mut app = App::new();
        app.init_resource::<SeatRawFrames>();
        app.init_resource::<SlotControls>();
        app.init_resource::<PortalTuning>();
        app.add_systems(Update, warp_portal_input);
        let mut frame = ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default();
        frame.publish_resolved_frame(ambition_platformer2d_core::MotionFrame::from_direction(Vec2::X, 1.0));
        let body = app
            .world_mut()
            .spawn((
                DrivingParticipant(PlayerSlot::PRIMARY),
                frame,
                ambition_platformer2d_core::BodyFlightState { fly_enabled: flying, ..Default::default() },
                PortalEmission { exit_normal: Vec2::X, timer: 1.0 },
            ))
            .id();
        translate(&mut app, body, local);
        app.update();
        locomotion(&app, body)
    };
    let inward = Vec2::new(0.0, -1.0);
    let tangent = Vec2::new(-1.0, 0.0);
    assert!(after(inward, true).length() < 0.01, "a flyer's push into the exit wall survived");
    assert_eq!(after(tangent, true), tangent, "a flyer's travel along the wall was removed");
    assert_eq!(after(inward, false), inward, "a walker's local Y is not travel; the guard changed it");
}

/// ⭐ ONE HELD STICK THROUGH A TRANSIT AND A GRAVITY TURN, MAPPED ONCE (IN2).
/// Seat one holds right in ordinary gravity under the body-relative mode, and
/// on one tick the body arrives through a same-wall pair (a warp that sends
/// world right to world left) into gravity turned upside down. The translator
/// keeps the hold's world direction through the turn (`HeldControlFrame`) and
/// the warp maps that world direction by the accepted transit: travel and the
/// attack the movement stick aims go world left. Mapping twice (the turned
/// frame's reading of the raw stick, then the warp again) sends them world
/// right. Controls: ordinary gravity with the warp, and the turn alone.
#[test]
fn a_held_stick_through_a_transit_and_a_gravity_turn_is_mapped_once() {
    use ambition_platformer2d_actor_monolith::control::{publish_seat_controls_when_nobody_else_does, tick_controlled_brains};
    use ambition_platformer2d_core::{ControlFrameModes, InputFrameMode, MotionFrame};
    use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;

    let world_travel = |turn: bool, warp: bool| {
        let mut app = App::new();
        app.init_resource::<SeatRawFrames>();
        app.init_resource::<SlotControls>();
        app.init_resource::<ambition_characters::control::SeatControlFrameModes>();
        app.init_resource::<PortalTuning>();
        app.add_systems(
            Update,
            (publish_seat_controls_when_nobody_else_does, tick_controlled_brains, warp_portal_input).chain(),
        );
        let pos = Vec2::new(64.0, 32.0);
        let size = Vec2::new(44.0, 78.0);
        let mut seed = ambition_body_seed::ActorClusterSeed::new(
            &ambition_sprite_sheet::SpriteContent::default(),
            "driven",
            "driven",
            ambition_platformer2d_core::Aabb::new(pos, size * 0.5),
            ambition_entity_catalog::placements::CharacterBrain::Custom("pirate_raider".into()),
            &[],
        );
        seed.kin.canonical_size = size;
        seed.kin.pos = pos;
        let body = app
            .world_mut()
            .spawn((seed.into_components(), ActorControl::default(), DrivingParticipant(PlayerSlot::PRIMARY)))
            .id();
        app.world_mut().resource_mut::<SeatRawFrames>().shape(PlayerSlot::PRIMARY, |cf| {
            cf.axis_x = 1.0;
            cf.attack_pressed = true;
            cf.control_frame_modes =
                ControlFrameModes { movement: InputFrameMode::BodyRelativeStrict, ..ControlFrameModes::default() };
        });
        app.update();
        let pressed = app.world().get::<ResolvedMotionFrame>(body).unwrap().basis();
        let held = pressed.to_world(locomotion(&app, body));
        assert!(held.x > 0.9, "premise: the held stick goes world right in ordinary gravity, {held:?}");

        // The arrival tick.
        if turn {
            app.world_mut()
                .get_mut::<ResolvedMotionFrame>(body)
                .unwrap()
                .publish_resolved_frame(MotionFrame::from_direction(Vec2::new(0.0, -1.0), 0.0));
        }
        if warp {
            app.world_mut().entity_mut(body).insert(PortalInputWarp {
                map: Mat2::from_diagonal(Vec2::new(-1.0, 1.0)),
                anchor: Vec2::new(1.0, 0.0),
            });
        }
        app.update();
        let basis = app.world().get::<ResolvedMotionFrame>(body).unwrap().basis();
        let control = app.world().get::<ActorControl>(body).unwrap().0;
        assert!(!control.attack_axis_from_aim_stick, "premise: the movement stick aims the attack");
        (basis.to_world(control.locomotion.vec()), basis.to_world(control.attack_axis.vec()))
    };
    let left = |v: Vec2| v.x < -0.9 && v.y.abs() < 0.1;
    let right = |v: Vec2| v.x > 0.9 && v.y.abs() < 0.1;

    let (travel, aim) = world_travel(true, true);
    assert!(left(travel), "the transit's map, once, on the hold's world direction: travel {travel:?}");
    assert!(left(aim), "the movement-aimed attack follows the held stick: aim {aim:?}");
    let (travel, aim) = world_travel(false, true);
    assert!(left(travel) && left(aim), "control: the warp in ordinary gravity, travel {travel:?} aim {aim:?}");
    let (travel, aim) = world_travel(true, false);
    assert!(right(travel) && right(aim), "control: the turn alone keeps the world direction, travel {travel:?} aim {aim:?}");
}
