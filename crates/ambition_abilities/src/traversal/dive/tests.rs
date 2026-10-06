use super::*;
use crate::test_support::spawn_primary_player_holding;
use ambition_platformer2d_core::BodyKinematics;

fn test_app() -> App {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_combat::events::HitEvent>();
    app.add_systems(Update, fire_dive_system);
    app
}

#[derive(bevy::prelude::Resource, Default)]
struct CapturedHits(Vec<ambition_combat::events::HitEvent>);

fn capture_hits(
    mut reader: bevy::prelude::MessageReader<ambition_combat::events::HitEvent>,
    mut out: bevy::prelude::ResMut<CapturedHits>,
) {
    out.0.extend(reader.read().cloned());
}

#[test]
fn dive_lunges_the_player_forward_and_cuts_a_corridor() {
    let mut app = test_app();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, capture_hits.after(fire_dive_system));
    let player = spawn_primary_player_holding(&mut app, DIVE_ID);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    // No world, no walls: full lunge along facing (+x).
    let pos = app.world().get::<BodyKinematics>(player).unwrap().pos;
    assert!(
        (pos.x - (100.0 + DIVE_LUNGE)).abs() < 0.01,
        "player lunged a full DIVE_LUNGE forward: {pos:?}"
    );
    let hits = &app.world().resource::<CapturedHits>().0;
    assert_eq!(hits.len(), 1, "one corridor hit emitted");
    assert_eq!(hits[0].damage, DIVE_DAMAGE);
    assert!(
        matches!(hits[0].source, ambition_combat::events::HitSource::Melee),
        "player-side source so it spares the player",
    );
    // The authored push is on the hit event at its authored magnitude. Check
    // the magnitude, not only that a knockback exists.
    let knockback = hits[0]
        .knockback
        .as_ref()
        .expect("an authored dive shove reaches the victim as a HitKnockback");
    assert_eq!(knockback.dir, 1.0, "shoved along the lunge");
    assert_eq!(
        knockback.magnitude,
        ambition_combat::events::HitKnockbackMagnitude::FeelScale(DIVE_KNOCKBACK),
        "the authored DIVE_KNOCKBACK arrives intact — a sign-only channel loses it"
    );
    // The corridor spans the dash: from start (100) to landing (240) along x.
    assert!(
        hits[0].volume.bounds().min.x <= 100.0
            && hits[0].volume.bounds().max.x >= 100.0 + DIVE_LUNGE,
        "corridor covers start..landing: {:?}",
        hits[0].volume
    );
}

#[test]
fn downward_dive_does_not_embed_in_the_floor() {
    // Like the blink: a vertical lunge clamps by half-height, not half-width,
    // or a downward dive embeds in the floor.
    let mut app = test_app();
    let player = spawn_primary_player_holding(&mut app, DIVE_ID); // (100,100), 24x40
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "test",
            ae::Vec2::new(600.0, 600.0),
            ae::Vec2::new(100.0, 100.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 200.0),
                ae::Vec2::new(600.0, 400.0),
            )],
        )),
    );
    {
        let mut control = app.world_mut().get_mut::<ActorControl>(player).unwrap();
        control.0.melee_pressed = true;
        control.0.aim = ae::LocalAxes::new(0.0, 1.0); // brain-resolved local aim: down
    }
    app.update();
    let pos = app.world().get::<BodyKinematics>(player).unwrap().pos;
    assert!(
        pos.y + 20.0 <= 200.0 + 1e-3,
        "downward dive embedded the body in the floor: bottom={}, floor top=200",
        pos.y + 20.0,
    );
    assert!(
        pos.y > 100.0,
        "the dive should still carry the player downward"
    );
}

#[test]
fn no_dive_without_attack_or_item() {
    let mut app = test_app();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, capture_hits.after(fire_dive_system));
    let player = spawn_primary_player_holding(&mut app, DIVE_ID);
    app.update(); // no attack pressed
    assert_eq!(app.world().resource::<CapturedHits>().0.len(), 0);
    assert_eq!(
        app.world().get::<BodyKinematics>(player).unwrap().pos.x,
        100.0,
        "no lunge without an attack press"
    );
}

#[test]
fn dive_costs_mana_and_is_blocked_when_empty() {
    let mut app = test_app();
    app.init_resource::<CapturedHits>();
    app.add_systems(Update, capture_hits.after(fire_dive_system));
    let player = spawn_primary_player_holding(&mut app, DIVE_ID);
    crate::test_support::set_mana(&mut app, player, 5.0);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    assert_eq!(
        app.world().resource::<CapturedHits>().0.len(),
        0,
        "no dive when mana < cost"
    );
    assert_eq!(
        app.world().get::<BodyKinematics>(player).unwrap().pos.x,
        100.0,
        "and no lunge either"
    );

    crate::test_support::set_mana(&mut app, player, 100.0);
    app.update();
    assert_eq!(
        app.world().resource::<CapturedHits>().0.len(),
        1,
        "fires once there's mana"
    );
}

#[test]
fn dive_dir_snaps_to_the_dominant_axis() {
    // Engine y grows downward, so "up" is -y.
    assert_eq!(
        dive_dir(ae::Vec2::new(0.0, -1.0), 1.0),
        ae::Vec2::new(0.0, -1.0)
    );
    assert_eq!(
        dive_dir(ae::Vec2::new(1.0, 0.0), 1.0),
        ae::Vec2::new(1.0, 0.0)
    );
    // Null aim falls back to facing.
    assert_eq!(dive_dir(ae::Vec2::ZERO, -1.0), ae::Vec2::new(-1.0, 0.0));
    // Dominant axis wins on a diagonal.
    assert_eq!(
        dive_dir(ae::Vec2::new(0.3, -0.9), 1.0),
        ae::Vec2::new(0.0, -1.0)
    );
}

#[test]
fn dive_corridor_is_a_thin_rectangle_spanning_the_dash() {
    // A horizontal dash: long along x, thin along y.
    let c = dive_corridor(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(240.0, 100.0));
    assert!(c.min.x <= 100.0 && c.max.x >= 240.0, "spans the dash on x");
    let half_y = (c.max.y - c.min.y) * 0.5;
    let half_x = (c.max.x - c.min.x) * 0.5;
    assert!(
        half_x > half_y,
        "horizontal corridor is long along x: {c:?}"
    );
}

/// A second driven body dives too (see the blink's version of this test).
#[test]
fn two_driven_bodies_each_dive_from_their_own_position() {
    use crate::test_support::spawn_seated_body_holding;
    let mut app = test_app();
    app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));
    let a = spawn_seated_body_holding(
        &mut app,
        DIVE_ID,
        0,
        "seat_a",
        ambition_platformer2d_core::Vec2::new(100.0, 100.0),
    );
    let b = spawn_seated_body_holding(
        &mut app,
        DIVE_ID,
        1,
        "seat_b",
        ambition_platformer2d_core::Vec2::new(900.0, 100.0),
    );
    for body in [a, b] {
        app.world_mut()
            .get_mut::<ActorControl>(body)
            .unwrap()
            .0
            .melee_pressed = true;
    }
    app.update();
    for (body, start, who) in [(a, 100.0, "a"), (b, 900.0, "b")] {
        let pos = app.world().get::<BodyKinematics>(body).unwrap().pos;
        assert!(
            (pos.x - (start + DIVE_LUNGE)).abs() < 0.01,
            "seat {who} did not lunge from its own position: {pos:?}"
        );
    }
}

/// OW1 cut 7o: a body lunges against the walls of its own live room. With two
/// rooms live, the dive once read the sole live room, found none, and lunged
/// the full distance through the wall.
#[test]
fn a_dive_stops_at_a_wall_of_its_own_live_room() {
    let mut app = test_app();
    let second = crate::test_support::two_live_rooms_with_a_wall_in_the_second(&mut app);
    // The fixture with mana; then 80 px from the wall.
    let player = spawn_primary_player_holding(&mut app, DIVE_ID);
    app.world_mut().get_mut::<BodyKinematics>(player).unwrap().pos = ae::Vec2::new(300.0, 300.0);
    app.world_mut()
        .entity_mut(player)
        .insert(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(second));
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    let pos = app.world().get::<BodyKinematics>(player).unwrap().pos;
    assert!(pos.x > 300.0, "the body did not lunge: {pos:?}");
    assert!(
        pos.x + 12.0 <= 380.0,
        "the body lunged to {pos:?}, through #1's wall at x = 380"
    );
}

/// Gravity toward world `+x`. The 24x40 body lies along its floor: its box is
/// 40 on x and 24 on y. Its side axis is world `-y`, so a body that faces `+1`
/// dives toward world `-y`.
const SIDEWAYS: ae::Vec2 = ae::Vec2::new(1.0, 0.0);
/// The half of the 24x40 body, turned to [`SIDEWAYS`].
const TURNED_HALF: ae::Vec2 = ae::Vec2::new(20.0, 12.0);

/// A player at (100, 100) that holds the dive, in sideways gravity, in a live
/// room with `blocks`.
fn sideways_player(app: &mut App, blocks: Vec<ae::Block>) -> Entity {
    let player = spawn_primary_player_holding(app, DIVE_ID);
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "test",
            ae::Vec2::new(600.0, 600.0),
            ae::Vec2::new(100.0, 100.0),
            blocks,
        )),
    );
    app.world_mut()
        .get_mut::<ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>(player)
        .unwrap()
        .publish_resolved_frame(ae::MotionFrame::from_direction(SIDEWAYS, 900.0));
    player
}

/// Press Attack with the local aim `aim` (none: the facing), and answer where
/// the body is after the dive.
fn dive_with_aim(app: &mut App, player: Entity, aim: Option<ae::LocalAxes>) -> ae::Vec2 {
    {
        let mut control = app.world_mut().get_mut::<ActorControl>(player).unwrap();
        control.0.melee_pressed = true;
        if let Some(aim) = aim {
            control.0.aim = aim;
        }
    }
    app.update();
    app.world().get::<BodyKinematics>(player).unwrap().pos
}

/// The premise of the sideways arms: which world axis each aim is.
#[test]
fn in_sideways_gravity_a_dive_goes_along_the_axes_of_the_body() {
    let mut app = test_app();
    let player = sideways_player(&mut app, Vec::new());
    assert_eq!(
        dive_with_aim(&mut app, player, None),
        ae::Vec2::new(100.0, 100.0 - DIVE_LUNGE),
        "a body that faces +1 in gravity toward +x dives toward world -y"
    );
    let mut app = test_app();
    let player = sideways_player(&mut app, Vec::new());
    assert_eq!(
        dive_with_aim(&mut app, player, Some(ae::LocalAxes::new(0.0, 1.0))),
        ae::Vec2::new(100.0 + DIVE_LUNGE, 100.0),
        "a dive aimed down goes toward world +x"
    );
}

/// The body is 20 deep toward its floor, not 12.
#[test]
fn in_sideways_gravity_a_dive_toward_the_floor_does_not_embed() {
    let mut app = test_app();
    // The floor: its face is at x = 200.
    let floor = ae::Block::solid("floor", ae::Vec2::new(200.0, -300.0), ae::Vec2::new(400.0, 900.0));
    let player = sideways_player(&mut app, vec![floor]);
    let pos = dive_with_aim(&mut app, player, Some(ae::LocalAxes::new(0.0, 1.0)));
    assert!(pos.x > 100.0, "the dive must carry the body toward the floor: {pos:?}");
    assert!(
        pos.x + TURNED_HALF.x <= 200.0 + 1e-3,
        "the dive put the body in the floor: its box ends at x = {}, the floor is at x = 200",
        pos.x + TURNED_HALF.x
    );
}

/// The body is 12 deep along its floor, not 20.
#[test]
fn in_sideways_gravity_a_dive_along_the_floor_stops_at_the_wall() {
    let mut app = test_app();
    // A wall ahead of the body (world -y): its face is at y = 0.
    let wall = ae::Block::solid("wall", ae::Vec2::new(-300.0, -100.0), ae::Vec2::new(900.0, 100.0));
    let player = sideways_player(&mut app, vec![wall]);
    let pos = dive_with_aim(&mut app, player, None);
    let gap = pos.y - TURNED_HALF.y;
    assert!(gap >= -1e-3, "the dive put the body in the wall: gap {gap}");
    assert!(gap <= 2.0 + 1e-3, "the dive stopped {gap} short of the wall; the margin is 2");
}

/// The centre ray misses a solid that the box of the body would clip at the
/// arrival: the safety net must ask the box the body has.
#[test]
fn in_sideways_gravity_a_dive_refuses_an_arrival_its_own_box_would_clip() {
    let mut app = test_app();
    // Beside the arrival at (100, -40): from x = 114, past the level half
    // (12) and inside the turned half (20).
    let corner = ae::Block::solid("corner", ae::Vec2::new(114.0, -50.0), ae::Vec2::new(16.0, 20.0));
    let player = sideways_player(&mut app, vec![corner]);
    assert_eq!(
        dive_with_aim(&mut app, player, None),
        ae::Vec2::new(100.0, 100.0),
        "the arrival clips a solid, and the body must stay where it is"
    );
}

/// The control of the arm above: a solid past the box of the body does not
/// stop the dive.
#[test]
fn in_sideways_gravity_a_dive_arrives_beside_a_solid_its_box_does_not_touch() {
    let mut app = test_app();
    // From x = 122: past the turned half (20).
    let corner = ae::Block::solid("corner", ae::Vec2::new(122.0, -50.0), ae::Vec2::new(16.0, 20.0));
    let player = sideways_player(&mut app, vec![corner]);
    assert_eq!(dive_with_aim(&mut app, player, None), ae::Vec2::new(100.0, 100.0 - DIVE_LUNGE));
}
