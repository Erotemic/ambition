use super::*;
use ambition_platformer2d_core::BodyKinematics;

/// The shared teleport rule (player blink and any actor body): full distance
/// over open space, stopped a body-half short of a wall, never embedding.
#[test]
fn blink_target_travels_full_distance_then_clamps_at_a_wall() {
    let half = ae::Vec2::new(12.0, 20.0);
    // Open world (no blocks): blink the full distance to the right.
    let empty = ae::World::new("t", ae::Vec2::new(2000.0, 600.0), ae::Vec2::ZERO, vec![]);
    let from = ae::Vec2::new(0.0, 0.0);
    let open = blink_target(&empty, from, ae::Vec2::new(1.0, 0.0), 150.0, half);
    assert!(
        (open.x - 150.0).abs() < 1e-3,
        "open blink travels full distance: {open:?}"
    );

    // A wall whose left face is at x=100 (Block::solid takes the MIN corner):
    // the body stops a half-width (+margin) short of it, never crossing in.
    let walled = ae::World::new(
        "t",
        ae::Vec2::new(2000.0, 600.0),
        ae::Vec2::ZERO,
        vec![ae::Block::solid(
            "wall",
            ae::Vec2::new(100.0, -300.0),
            ae::Vec2::new(120.0, 600.0),
        )],
    );
    let clamped = blink_target(&walled, from, ae::Vec2::new(1.0, 0.0), 150.0, half);
    assert!(
        clamped.x + half.x <= 100.0 + 1e-3,
        "clamped blink must not cross the wall's left face at x=100: right edge={}",
        clamped.x + half.x
    );
    assert!(
        clamped.x > 0.0,
        "but it should still carry toward the wall: {clamped:?}"
    );
}

fn test_app() -> App {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_vfx::vfx::VfxInRoom>();
    app.add_message::<ambition_combat::events::HitEvent>();
    app.add_systems(Update, blink_system);
    app
}

fn spawn_player_holding(app: &mut App, id: &str, facing: f32) -> Entity {
    crate::test_support::spawn_primary_player_holding_at(
        app,
        id,
        ae::Vec2::new(300.0, 300.0),
        facing,
    )
}

fn player_pos(app: &App, player: Entity) -> ae::Vec2 {
    app.world().get::<BodyKinematics>(player).unwrap().pos
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
fn blink_emits_a_player_side_shockwave_at_arrival() {
    let mut app = test_app();
    app.init_resource::<CapturedHits>();
    app.add_systems(bevy::prelude::Update, capture_hits.after(blink_system));
    let player = spawn_player_holding(&mut app, BLINK_ID, 1.0);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    let hits = &app.world().resource::<CapturedHits>().0;
    assert_eq!(hits.len(), 1, "one shockwave on arrival");
    // Centered at the arrival point (300 + BLINK_DISTANCE along facing).
    let center_x = (hits[0].volume.bounds().min.x + hits[0].volume.bounds().max.x) * 0.5;
    assert!(
        (center_x - (300.0 + BLINK_DISTANCE)).abs() < 1.0,
        "shockwave is at the arrival point",
    );
    assert_eq!(hits[0].damage, BLINK_SHOCKWAVE_DAMAGE);
    assert!(
        matches!(hits[0].source, ambition_combat::events::HitSource::Melee),
        "player-side source so it spares the player",
    );
}

#[test]
fn attack_blinks_the_player_forward_along_facing() {
    // No RoomGeometry inserted → the no-clamp branch teleports the full distance.
    let mut app = test_app();
    let player = spawn_player_holding(&mut app, BLINK_ID, 1.0);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    assert_eq!(
        player_pos(&app, player),
        ae::Vec2::new(300.0 + BLINK_DISTANCE, 300.0),
        "blink carried the player one BLINK_DISTANCE along facing",
    );
}

#[test]
fn downward_blink_does_not_embed_in_the_floor() {
    // A vertical blink must pull back by the body's half-height, not
    // half-width, or the 40-tall body embeds in the floor and trips the
    // inside-solid OOB detector.
    let mut app = test_app();
    let player = spawn_player_holding(&mut app, BLINK_ID, 1.0); // (300,300), 24x40
                                                                // Solid floor whose top edge is at y=350, just below the player.
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "test",
            ae::Vec2::new(600.0, 600.0),
            ae::Vec2::new(300.0, 300.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 350.0),
                ae::Vec2::new(600.0, 250.0),
            )],
        )),
    );
    {
        let mut control = app.world_mut().get_mut::<ActorControl>(player).unwrap();
        control.0.melee_pressed = true;
        control.0.aim = ae::LocalAxes::new(0.0, 1.0); // brain-resolved local aim: down
    }
    app.update();
    let pos = player_pos(&app, player);
    let half_h = 20.0;
    assert!(
        pos.y + half_h <= 350.0 + 1e-3,
        "downward blink embedded the body in the floor: bottom={}, floor top=350",
        pos.y + half_h,
    );
    assert!(
        pos.y > 300.0,
        "the blink should still carry the player toward the floor (got y={})",
        pos.y,
    );
}

#[test]
fn blink_follows_facing_left() {
    let mut app = test_app();
    let player = spawn_player_holding(&mut app, BLINK_ID, -1.0);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    assert_eq!(
        player_pos(&app, player),
        ae::Vec2::new(300.0 - BLINK_DISTANCE, 300.0),
        "a left-facing blink goes left",
    );
}

/// Blink runs on whatever body is the `ControlledSubject`, even a possessed
/// actor that is not a `PlayerEntity`, and the home avatar (not the subject)
/// does not blink.
#[test]
fn blink_executes_on_the_controlled_actor_not_the_home_avatar() {
    use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
    let mut app = test_app();
    // Home avatar (a PlayerEntity): holds blink but is not the controlled
    // subject this frame. It must stay put.
    let home_spec = ambition_characters::brain::held_item_by_id(BLINK_ID).unwrap();
    let home = app
        .world_mut()
        .spawn((
            PlayerEntity,
            BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            ambition_platformer2d_core::movement::MotionModel::default(),
            HeldItem::new(home_spec),
            // Every body carries the per-tick resolved frame and full clusters
            // (ADR 0024), both in the ancillary bundle.
            ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                ae::BodyClusterScratch::new_with_abilities(
                    ae::Vec2::new(100.0, 100.0),
                    ae::AbilitySet::default(),
                ),
            ),
            {
                let mut c = ActorControl::default();
                c.0.melee_pressed = true; // even pressing attack, it must not blink
                c
            },
        ))
        .id();
    // A possessed actor (not a PlayerEntity) holding blink is the controlled
    // subject and presses attack. It must blink.
    let actor_spec = ambition_characters::brain::held_item_by_id(BLINK_ID).unwrap();
    let actor = app
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: ae::Vec2::new(500.0, 500.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            ambition_platformer2d_core::movement::MotionModel::default(),
            HeldItem::new(actor_spec),
            ambition_platformer2d_shared_tangle::body::AncillaryMovementBundle::from_scratch(
                ae::BodyClusterScratch::new_with_abilities(
                    ae::Vec2::new(500.0, 500.0),
                    ae::AbilitySet::default(),
                ),
            ),
            {
                let mut c = ActorControl::default();
                c.0.melee_pressed = true;
                c.0.facing = 1.0;
                c
            },
        ))
        .id();
    app.insert_resource(
        ambition_platformer2d_shared_tangle::markers::ControlledSubject(Some(actor)),
    );
    app.update();

    assert_eq!(
        player_pos(&app, home),
        ae::Vec2::new(100.0, 100.0),
        "the home avatar is NOT the controlled subject — it must not blink",
    );
    assert_eq!(
        player_pos(&app, actor),
        ae::Vec2::new(500.0 + BLINK_DISTANCE, 500.0),
        "the possessed actor (a non-PlayerEntity controlled body) blinks",
    );
}

#[test]
fn no_blink_without_attack_or_with_a_different_item() {
    // Holding blink but not attacking → stays put.
    let mut app = test_app();
    let player = spawn_player_holding(&mut app, BLINK_ID, 1.0);
    app.update();
    assert_eq!(player_pos(&app, player), ae::Vec2::new(300.0, 300.0));
    // Holding the bomb + attacking → blink_system ignores it.
    let mut app2 = test_app();
    let player2 = spawn_player_holding(&mut app2, "bomb", 1.0);
    app2.world_mut()
        .get_mut::<ActorControl>(player2)
        .unwrap()
        .0
        .melee_pressed = true;
    app2.update();
    assert_eq!(player_pos(&app2, player2), ae::Vec2::new(300.0, 300.0));
}

/// A second driven body blinks too. With `ControlledSubject` alone, a couch's
/// second seat could not blink, and with nobody possessed neither body could.
#[test]
fn two_driven_bodies_each_blink_from_their_own_position() {
    use crate::test_support::spawn_seated_body_holding;
    let mut app = test_app();
    app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));
    let a = spawn_seated_body_holding(&mut app, BLINK_ID, 0, "seat_a", ae::Vec2::new(300.0, 300.0));
    let b = spawn_seated_body_holding(&mut app, BLINK_ID, 1, "seat_b", ae::Vec2::new(900.0, 300.0));
    for body in [a, b] {
        app.world_mut()
            .get_mut::<ActorControl>(body)
            .unwrap()
            .0
            .melee_pressed = true;
    }
    app.update();
    assert_eq!(
        player_pos(&app, a),
        ae::Vec2::new(300.0 + BLINK_DISTANCE, 300.0),
        "seat a did not blink"
    );
    assert_eq!(
        player_pos(&app, b),
        ae::Vec2::new(900.0 + BLINK_DISTANCE, 300.0),
        "seat b did not blink"
    );
}

/// OW1 cut 7o: a body blinks against the walls of its own live room. With two
/// rooms live, the blink once read the sole live room, found none, and went
/// the full distance through the wall.
#[test]
fn a_blink_stops_at_a_wall_of_its_own_live_room() {
    let mut app = test_app();
    let second = crate::test_support::two_live_rooms_with_a_wall_in_the_second(&mut app);
    let player = spawn_player_holding(&mut app, BLINK_ID, 1.0);
    app.world_mut()
        .entity_mut(player)
        .insert(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(second));
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    let pos = player_pos(&app, player);
    assert!(pos.x > 300.0, "the body did not blink: {pos:?}");
    assert!(
        pos.x + 12.0 <= 380.0,
        "the body blinked to {pos:?}, through #1's wall at x = 380"
    );
}

/// Gravity toward world `+x`. The 24x40 body lies along its floor: its box is
/// 40 on x and 24 on y. Its side axis is world `-y`, so a body that faces `+1`
/// blinks toward world `-y`.
const SIDEWAYS: ae::Vec2 = ae::Vec2::new(1.0, 0.0);
/// The half of the 24x40 body, turned to [`SIDEWAYS`].
const TURNED_HALF: ae::Vec2 = ae::Vec2::new(20.0, 12.0);

/// A player at (300, 300) that holds the blink, in sideways gravity, in a
/// live room with `blocks`.
fn sideways_player(app: &mut App, blocks: Vec<ae::Block>) -> Entity {
    let player = spawn_player_holding(app, BLINK_ID, 1.0);
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "test",
            ae::Vec2::new(600.0, 600.0),
            ae::Vec2::new(300.0, 300.0),
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
/// the body is after the blink.
fn blink_with_aim(app: &mut App, player: Entity, aim: Option<ae::LocalAxes>) -> ae::Vec2 {
    {
        let mut control = app.world_mut().get_mut::<ActorControl>(player).unwrap();
        control.0.melee_pressed = true;
        if let Some(aim) = aim {
            control.0.aim = aim;
        }
    }
    app.update();
    player_pos(app, player)
}

/// The premise of the sideways arms: which world axis each aim is.
#[test]
fn in_sideways_gravity_a_blink_goes_along_the_axes_of_the_body() {
    let mut app = test_app();
    let player = sideways_player(&mut app, Vec::new());
    assert_eq!(
        blink_with_aim(&mut app, player, None),
        ae::Vec2::new(300.0, 300.0 - BLINK_DISTANCE),
        "a body that faces +1 in gravity toward +x blinks toward world -y"
    );
    let mut app = test_app();
    let player = sideways_player(&mut app, Vec::new());
    assert_eq!(
        blink_with_aim(&mut app, player, Some(ae::LocalAxes::new(0.0, 1.0))),
        ae::Vec2::new(300.0 + BLINK_DISTANCE, 300.0),
        "a blink aimed down goes toward world +x"
    );
}

/// The body is 20 deep toward its floor, not 12: the pull-back from the floor
/// must use the half of the box the body has.
#[test]
fn in_sideways_gravity_a_blink_toward_the_floor_does_not_embed() {
    let mut app = test_app();
    // The floor: its face is at x = 350.
    let floor = ae::Block::solid("floor", ae::Vec2::new(350.0, 0.0), ae::Vec2::new(250.0, 600.0));
    let player = sideways_player(&mut app, vec![floor]);
    let pos = blink_with_aim(&mut app, player, Some(ae::LocalAxes::new(0.0, 1.0)));
    assert!(pos.x > 300.0, "the blink must carry the body toward the floor: {pos:?}");
    assert!(
        pos.x + TURNED_HALF.x <= 350.0 + 1e-3,
        "the blink put the body in the floor: its box ends at x = {}, the floor is at x = 350",
        pos.x + TURNED_HALF.x
    );
}

/// The body is 12 deep along its floor, not 20: a blink at a wall stops the
/// margin short of it, not the margin and 8 more.
#[test]
fn in_sideways_gravity_a_blink_along_the_floor_stops_at_the_wall() {
    let mut app = test_app();
    // A wall ahead of the body (world -y): its face is at y = 200.
    let wall = ae::Block::solid("wall", ae::Vec2::new(0.0, 100.0), ae::Vec2::new(600.0, 100.0));
    let player = sideways_player(&mut app, vec![wall]);
    let pos = blink_with_aim(&mut app, player, None);
    let gap = (pos.y - TURNED_HALF.y) - 200.0;
    assert!(gap >= -1e-3, "the blink put the body in the wall: gap {gap}");
    assert!(
        gap <= 2.0 + 1e-3,
        "the blink stopped {gap} short of the wall; the margin is 2"
    );
}

/// The centre ray misses a solid that the box of the body would clip at the
/// arrival: the safety net must ask the box the body has.
#[test]
fn in_sideways_gravity_a_blink_refuses_an_arrival_its_own_box_would_clip() {
    let mut app = test_app();
    // Beside the arrival at (300, 150): from x = 314, past the level half
    // (12) and inside the turned half (20).
    let corner = ae::Block::solid("corner", ae::Vec2::new(314.0, 140.0), ae::Vec2::new(16.0, 20.0));
    let player = sideways_player(&mut app, vec![corner]);
    let pos = blink_with_aim(&mut app, player, None);
    assert_eq!(
        pos,
        ae::Vec2::new(300.0, 300.0),
        "the arrival clips a solid, and the body must stay where it is"
    );
}

/// The control of the arm above: a solid past the box of the body does not
/// stop the blink.
#[test]
fn in_sideways_gravity_a_blink_arrives_beside_a_solid_its_box_does_not_touch() {
    let mut app = test_app();
    // From x = 322: past the turned half (20).
    let corner = ae::Block::solid("corner", ae::Vec2::new(322.0, 140.0), ae::Vec2::new(16.0, 20.0));
    let player = sideways_player(&mut app, vec![corner]);
    assert_eq!(
        blink_with_aim(&mut app, player, None),
        ae::Vec2::new(300.0, 300.0 - BLINK_DISTANCE)
    );
}
