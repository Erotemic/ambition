//! The seat consequences of a portal transit, measured through
//! `PortalSchedulePlugin` where a game gets them, and the input guards on the
//! seat frame.

use bevy::prelude::*;

use ambition_characters::control::{DrivingParticipant, PlayerSlot, SeatRawFrames, SlotControls};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_portal2d::{
    portal_half_extent, BodyTeleported, PlacedPortal, PortalChannel, PortalConvention,
    PortalEmission, PortalGunColor, PortalInputWarp, PortalTuning,
};

use super::warp_portal_input;

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
        reorient_facing: true,
        ..Default::default()
    });
    app.add_plugins(crate::PortalSchedulePlugin);
    for (color, y) in [(PortalGunColor::BLUE, 200.0), (PortalGunColor::ORANGE, 600.0)] {
        app.world_mut().spawn(PlacedPortal::fixed(
            PortalChannel::Gun(color),
            Vec2::new(20.0, y),
            Vec2::new(1.0, 0.0),
            portal_half_extent(Vec2::new(1.0, 0.0)),
        ));
    }
    let mut body = app.world_mut().spawn(BodyKinematics {
        pos: Vec2::new(20.0, 200.0),
        vel: Vec2::new(-100.0, 0.0),
        size: Vec2::new(24.0, 40.0),
        facing: -1.0,
    });
    match who {
        Body::HomeWithoutTheSeat => body.insert((PlayerEntity, PrimaryPlayer)),
        Body::PossessedWithTheSeat => body.insert(DrivingParticipant(PlayerSlot::PRIMARY)),
    };
    let body = body.id();
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
fn only_the_driven_body_turns_around_through_a_same_wall_pair() {
    for (who, facing) in [(Body::PossessedWithTheSeat, 1.0), (Body::HomeWithoutTheSeat, -1.0)] {
        let (mut app, body) = same_wall_pair_app(who);
        let kin = step_to_arrival(&mut app, body);
        assert!(kin.pos.y > 400.0, "{who:?} must transit to the orange portal, pos={:?}", kin.pos);
        assert!(kin.vel.x > 0.0, "{who:?} must carry its momentum out of the exit, vel={:?}", kin.vel);
        assert_eq!(kin.facing, facing, "{who:?}: only the body the seat drives turns around");
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
        // stripped from the seat's frame while the guard is fresh.
        hold_x(&mut app, -1.0);
        app.update();
        let kept = seat_axis_x(&app);
        assert_eq!(kept.abs() < 0.01, marked, "{who:?}: the seat's hold into the exit wall, {kept}");
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
